# 依赖图主线设计（文件锚点链最小范围）

> 状态：`current-design`（2026-09-01 设计定稿 + S1 代码 + S1 全面审查
> 处理 + S2 测试完成；
> 实施为 `partial`——S3 重建 + S4 实机复验待放行，验证闭环后随既有纪律转
> `implemented`）；权威转录见 ADR-0010 §14.51；裁决来源：
> MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31 §8 第 6 项（P2-11 第 4 项，
> 用户确认"下一轮主线"）+ MECHANICAL-LAYER-MATH-CALCULUS D3（阶段 0 决策，
> 2026-08-30 用户确认）＋机械层设计 §1/§2.3/§5.5。

## 1. 目标与约束

目标：把「跨轮簿记」（改过什么、谁依赖谁）从模型面真正移走——机械层维护
**文件锚点链依赖图**作为重写系统内部状态（设计 §1 逻辑侧兄弟组件，本阶段
实现其最小范围），模型不再需要凭记忆回顾 read/write 关系。

最小范围（裁决 6 原文）：**文件锚点链**（read→write 锚点边 + 工具→实体
变更边；D3 命令/检索副作用不建图），**PULL 查询面**、**模型零改动**；顺带
闭合 F11 receipt↔事件链逐段同构核对。

硬约束：

- 模型零改动：无新注入、无新常驻 token、无建议、不干预模型怎么做；只新增
  一个 `blackboard_read` 分区（`section=deps`），模型可查可不查。
- 8 工具面冻结：不新增/删除/改名工具；只扩展 `blackboard_read` 的 section
  枚举与返回内容（与 PULL 自描述/temporal 同例）。
- 有界：读/写事实各设上限（64），超出按最旧淘汰；渲染 ≤8 KiB（与其他
  分区同构）；事实载荷受事件面 schema 约束。
- live-only：依赖图是 run 级内部状态，不进 epoch 快照、不持久化（同
  entities/temporal 纪律）。
- D3 边界：terminal.run（含 run_terminal_cmd/run_tests）与检索族
  （web_search/web_fetch/browser_read/retrieve_project_*/pdf_read 等）的
  副作用**不建图**；失败/拒绝（exit_code≠0、policy_denial、锚点拒单）也
  不建图（失败目标身份已由 F4/I2 事件面字段承担，不重复建图）。

## 2. 数据模型

```text
type Anchor   = { sha256: SHA256?, size: Int?, mtime: Int? }
                 -- 与 F1 §2.1 同口径：sha256 权威（≤16MB 计算），
                 -- 超限/非文件时 None；size/mtime 供快筛

type ReadFact  = { call_id: String, path: Path, anchor: Anchor?, seq: Int }
type WriteFact = { call_id: String, path: Path,
                   consumed_read: String?,   -- 锚点边来源（read call_id）
                   consumed_anchor: Anchor?, -- 写参数 expected_anchor
                   new_anchor: Anchor?,      -- 写后内容锚点
                   seq: Int }
```

事实存储为两个有界队列（`VecDeque`，超出容量 pop_front 淘汰最旧）：

- **ReadFact**：`read_file` 成功（exit_code==0）后登记——调用身份 + 目标
  文件（归一化路径）+ read 时刻内容锚点。
- **WriteFact**：`search_replace` 成功（exit_code==0）后登记——调用身份 +
  目标文件 + 写参数携带的期望锚点（`expected_anchor`，可能只有 size/mtime）
  + 写后内容锚点。

派生边（不重复存储，渲染/事件时由事实推导）：

- **read→write 锚点边**：WriteFact 携带 `consumed_read = Some(read.call_id)`
  ——该写消费了那次读的锚点（GetPut 前提）。匹配规则：同路径、最晚（seq
  最大）的 read 事实且锚点匹配；匹配 = sha256 相等（双方都有 sha256 时
  sha256 权威，R2 口径）或（**任一方**缺 sha256 时）size+mtime 相等
  （S1 审查处理 2026-09-01：与 verifier `_anchor_matches` / 写前核证
  `verify_content_anchor` 口径同构，实现即此语义）。写未携带
  `expected_anchor`、或图内无匹配 read（如 run 前已读/事实被淘汰/新建路径）
  → `consumed_read = None`（如实记录"无图内锚点边"）。
- **工具→实体变更边**：每条 WriteFact = 一次 `search_replace → file:<path>`
  变更边（工具调用 → 实体变更）。

## 3. 建图规则（S1 实施）

记录点：`run_host_tool_with_timeout` 通用执行路径（read_file/search_replace
唯一实际执行点——direct 模型调用与 console 发放都经此，单点覆盖全部路径），
成功分支内、ToolCompleted 事件落盘前：

1. 仅 `read_file` / `search_replace` 且 `exit_code == 0` 才建图；其余工具
   （含 D3 命令/检索族）直接跳过。
2. 目标路径从参数取（`target_file` / `file_path` / `path`，与
   `register_entities_for_tool` 同取法）；取不到（无目标文件工具形态）则
   不建图。
3. 锚点计算与实体登记同口径（`compute_anchor`：stat size/mtime + 仅文件且
   ≤16MB 计算 sha256；超限/目录 None）。
4. `read_file` → `record_read`；`search_replace` → `record_write`
   （内部完成锚点边匹配）。
5. 事实随 ToolCompleted 事件载荷写入 `dep_graph` 字段（§5），供 F11
   同构核对。

容量：`DEPS_READS_CAP = 64`、`DEPS_WRITES_CAP = 64`；`seq` 全局单调递增，
淘汰按入队序（最旧先出）。`revision` 每次可见内容变化 +1（PULL 自描述
增量头徽章；`#[serde(skip)]` 不进序列化面）。

## 4. PULL 查询面（section=deps）

`blackboard_read section=deps`：

- live-only：带 `epoch` → 显式文本错误（同 entities 纪律，`exit_code 0`，
  形状被 `is_blackboard_render_error` 识别→不挂增量头、不推进游标）；带
  `receipt_id` → 显式文本错误（receipt 点读仅 actions 语义）。
- 渲染：确定性文本（文件路径 BTreeMap 序；同一文件分组内 read 事实按
  seq、write 事实按 seq，先 read 后 write——S1 审查处理 2026-09-01
  措辞对齐实现），有界：

```text
== deps ==
file:src/a.py
  r1 read anchor=(sha256=ab12cd34 size=4096)
  w2 write used=r1 new=(sha256=ef56ab78 size=3900)
file:src/b.py
  w3 write (no anchor edge) new=(sha256=f0012ab3 size=120)
工具→实体变更:
  w2 (search_replace) → file:src/a.py
  w3 (search_replace) → file:src/b.py
deps total=reads:2 writes:2 truncated=false
```

规则：sha256 只渲染前 8 字符（防膨胀）；`truncated=true` 当事实队列已满
（len==cap，继续登记将淘汰最旧；满而未淘汰亦标记，保守口径）或渲染字节
超限；总字节 ≤8 KiB（与其他分区同构的 `enforce_bound` 截断；S1 审查处理
2026-09-01 修复字节超限时 truncated footer 的追加预算，footer 完整落盘
后总长仍 ≤8 KiB）。

## 5. 事件面与 F11 同构核对（顺带闭合）

F11（机械层设计 §5.4）：归约留痕 receipt 每段（arg_validation/gate/
execution/delivery）须对应事件链中的事件；verifier 交叉核对。V1c 已于
2026-08-31 实现事件链侧核对（`_verify_v02_receipt_event_isomorphism`，
verifier 245 passed）；本主线**顺带闭合**其剩余登记缺口 + 把依赖图事实
纳入同一同构核对面：

1. **schema 先行**：`runtime/tool-completed-event-payload-v0.1.schema.json`
   增可选 `dep_graph` 对象（`additionalProperties` 现有纪律下显式登记）：

```json
"dep_graph": {
  "type": "object",
  "required": ["kind", "path"],
  "properties": {
    "kind": { "enum": ["read", "write"] },
    "path": { "type": "string" },
    "consumed_read": { "type": ["string", "null"] },
    "consumed_anchor": { "$ref": "#/$defs/dep_anchor" },
    "anchor": { "$ref": "#/$defs/dep_anchor" },
    "new_anchor": { "$ref": "#/$defs/dep_anchor" }
  }
}
```

   `dep_anchor` = `{ sha256?, size?, mtime? }`（全可选、additionalProperties
   false）。fixtures 补正/反例。

2. **生产者**：read_file/search_replace 成功的事实随 ToolCompleted 写入
   `dep_graph`（§3 步骤 5）；失败/拒绝/D3 工具不写。

3. **verifier 新增 `_verify_v02_dep_graph_events`**：
   - 形状：kind∈{read,write}、path 非空、锚点值为 object/null（字段类型
     与 sha256 hex 由 schema 兜底校验，verifier 只做 object/null 与跨
     事件一致性核对——S1 审查处理 2026-09-01 措辞对齐分层实现）；read
     事实工具必须为 `read_file` 且 exit_code==0；write 事实工具必须为
     `search_replace` 且 exit_code==0。
   - 锚点边：write 的 `consumed_read` 非空时，必须在同一 run 事件链中存在
     同 call_id 的 read 事实（`dep_graph.kind=read`），且两事实 path 一致、
     锚点匹配（sha256 权威，缺失时 size+mtime）。
   - 不要求所有成功 read_file/search_replace 必带事实（向后兼容既有语料/
     归档 replay——事实为可选增强面），只对已登记事实做一致性核对。

4. **登记收口**：机械层设计 §5.4 状态行（未实现→已实现）与阶段 2 审查
   审计 F11 行（已挂账→已闭合）更新；本主线实现记录见
   `docs/audits/P2-11_DEPENDENCY_GRAPH_IMPL_AUDIT_2026-09-01.md`。

## 6. 实现接线

| 文件 | 改动 |
|---|---|
| `orz/crates/orz-loop/src/dep_graph.rs` | 新模块：`DepGraph`（ReadFact/WriteFact/容量/revision）、`compute_anchor`、`render_text` |
| `orz/crates/orz-loop/src/blackboard.rs` | `Blackboard` 增 `dep_graph` 字段；`partition_revisions()` 增 `("deps", …)` |
| `orz/crates/orz-loop/src/controller.rs` | `render_blackboard_section` 增 `deps` 分支（live-only 文本错误同 entities）；`blackboard_read` 工具定义 section 枚举 + 描述增 `deps` |
| `orz/crates/orz-loop/src/host_exec.rs` | 通用执行路径成功分支：`record_dep_graph_fact`（建图 + 返回事件载荷）；`blackboard_read` 分支接 `deps` 渲染 |
| `runtime/tool-completed-event-payload-v0.1.schema.json` | 增 `dep_graph` 可选属性 + `$defs.dep_anchor` |
| `assurance/run_event_journal_validation.py` | 增 `_verify_v02_dep_graph_events` 并注册 |
| fixtures / Python 测试 | 正/反例 + verifier 交叉核对测试 |

## 7. S2 测试计划

- `dep_graph.rs` 单元：read/write 登记、锚点边匹配（sha256 权威 / size+mtime
  快筛 / 无匹配）、容量淘汰、revision 单调、渲染确定性 + 字节上限、D3 过滤
  （terminal/检索不建图）。
- `host_exec.rs`/`blackboard.rs` 集成：真实工具链 `read_file → search_replace
  （带 expected_anchor）` 后 `blackboard_read section=deps` 渲染含锚点边；
  无 expected_anchor 写 → consumed_read=None；失败写不建图。
- Python：schema 正/反例 fixtures；`_verify_v02_dep_graph_events` 用例
  （合法 read/write、consumed_read 悬空、工具不匹配、锚点不匹配）。

## 8. 边界与登记

- 依赖图只覆盖文件锚点链最小范围；完整图（命令/检索副作用、跨文件依赖
  传播、LTL 时序验证）为后续路径，不在本批。
- GetPut 律"未给 anchor → 取依赖图中最近 read 的锚点"的**写前自动补锚**：
  本批**不改变**写路径行为（现状未带 expected_anchor 直接执行，S4 实证
  0 拒单、收益不足，裁决 4 维持现状）；依赖图只**记录**锚点链事实，供
  模型 PULL 与审计核对。
- 同轮并行批次边界：read 与 write 在同一轮并行执行时，若 write 事实先于
  read 事实落图，该 write 的锚点边记 `consumed_read=None`（如实记录；
  真实用法 read→write 跨轮，锚点边稳定；同轮 read+write 仅 pipe 归约
  路径可达，运行时未接线）。
- live-only：run 结束即随会话生命周期消失；不持久化（7 天 retention 的
  StoredConversation 侧车不含图，与 entities/temporal 同纪律）。
- 锚点计算成本（S1 审查处理 2026-09-01 登记）：每次成功
  read_file/search_replace 对 ≤16MB 文件全量重读计算 sha256，与实体
  登记/写前核证同口径但存在重复计算；维持同口径不动，后续可复用 host
  结果锚点（不在本批）。
- 序列化面（S1 审查处理 2026-09-01 登记）：`dep_graph` 字段 serde 注解
  与 entities 先例一致（`#[serde(default)]`）；EpochSnapshot 与
  StoredConversation 均不序列化整黑板，live-only 成立；若未来引入整黑板
  序列化路径，需将 `dep_graph` 改 `#[serde(skip)]`。
- 模型零改动确认：模型面仅新增一个可选的 PULL 分区；无注入、无提示词
  变更、无行为要求变化。
