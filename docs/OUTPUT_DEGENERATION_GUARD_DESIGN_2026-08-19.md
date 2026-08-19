# 输出退化防护与工具结果可再读闭环设计（2026-08-19 设计定稿；S1/S2 已实施）

> 状态：`design-final`（2026-08-19 用户裁决：8K 全统一限值 + 桥 8K 不动 +
> 补读闭环硬约束 + 前两层补强；先设计、不动作）。
> 2026-08-19 S1 代码 + S2 测试实施闭合（用户放行；orz 提交 5e968ec +
> 审查处理 19b839f）；同日
> 全面审查处理（P1 点读判定改按订单动作名 + P3 指针预算/注释/可读性测试/
> 登记），见 §3.2/§3.3 修订与文末「审查处理登记」。
> 登记：ADR-0010 §14.33（v1.33，2026-08-19）/ BACKLOG 0d / TODO P0-0d /
> CLI_PROJECT_INDEX。
> 关联：FUS-LEDGER-FOLD-BRIDGE（§14.32 桥 8K）、BLACKBOARD_READ_CACHE_COST
> DESIGN（§4.5 方案 B 点读 8K）、FUS-STAGNATION（§4.5 停滞守卫）、
> ORZ-CACHE-CONTEXT-COST（§3.5 注入预算）、ADR-0007（重试/超时）。

## 1. 背景与证据

### 1.1 失败事实

- 2026-08-19 19:04 / 19:16 make-doom-for-mips 两次运行均在长时间运行后
  （35/56 请求）被**模型请求 hang/传输中断**终止：第一次
  `transport error: error decoding response body`（请求 hang 约 4 分钟），
  第二次 `AgentTimeoutError`（请求 hang 约 10 分钟）。
- 第一次失败前最后一次模型输出 **201,230 字符**（≈10 万 token）：
  开头 3.3K 字符为 blackboard_read 点读响应复述，其后 **2,316 次重复的
  「[elided middle/end — see archived log for full content]」省略标注**——
  模型退化复读（degenerate repetition），`finish_reason=length` 后流中断。
- **历史先例（非首次）**：2026-08-11 21:19 make-doom-for-mips 已有
  479,957 字符的超长 model_output（复述 read_file 工具结果 stubs-o32.h 等）——
  同一任务、同一类行为，早于折叠桥接等本轮全部改动。

### 1.2 排除项

- 网络/容器/沙箱：任务同款容器 30 分钟探针 60/60 次固定请求全 200（耗时
  0.7–1.5s），通道稳定；容器内 deepseek https 401（连接/TLS 正常）。
- 工具侧防护：bash 输出 20K 截断、点读 8K 截断代码正确（已核实）；
  ord-000013.log（9.6KB）完整无省略标记；点读/终端截断都不是"截断成省略
  标记"的来源。
- 停滞守卫（stagnation guard）：存在（连续重复阈值 10 / n-gram 阈值 10，
  完全满足触发条件），但**只在轮次正常完成后事后评估**——失败轮次直接走
  `run_failed`，守卫未被调用；且即便触发也无法阻止生成期已发生的超长流。

### 1.3 根因判定

**模型在长会话 + 大文件理解任务下对工具结果内容进行超长复述（退化复读），
叠加 `REQUEST_MAX_TOKENS=160_000` 的放大，超长流式传输导致连接中断。**

诱因链：
1. 工具结果截断后**无可再读闭环**：截断指针指向 journal/TraceStore（不在
   模型直接工具面），模型想恢复完整内容无路径，只能依赖记忆/复述。
2. 模型反复用终端打印大文件（nl/sed），上下文堆满高相似代码片段，诱导
   复读；20K 终端上限使打印 9–21K 段落几乎不触发截断提示（1/9 触发），
   模型从未被引导转向 read_file。
3. 160K max_tokens 允许单轮输出 10 万 token（正常轮次 p95 合计仅约 7K，
   max 23.7K——160K 严重过剩）。

## 2. 战略裁决（用户 2026-08-19）

1. **限值统一为 8K**：终端工具输出截断、点读窗口、桥保留尾预算全部 8K
   （桥 8K 为 S4 校准值不动；终端截断 20K → 8K；点读保持 8K 确认）——
   "只做一个限值"。
2. **补读闭环为硬约束**：任何截断末尾必须带明确的 read_file 指针
   （"完整内容见 \<路径\>，请使用 read_file，大文件用 offset/limit 分页"）；
   指针路径必须真实可读；桥截断保留尾部时工具结果自身的落盘指针行天然
   留存（sha256 存档指针降级为兜底）。
3. **前两层补强必做**：生成期实时复读检测（治本）+ `REQUEST_MAX_TOKENS`
   160K → 32K（止损）。
4. **桥 8K 不扩窗**：桥 8K 实测 3/3 折叠未截断；工具结果统一压到 8K 后桥
   内轮次更小、截断频率进一步下降；截断损失由可再读闭环恢复——扩窗
   （8K→12K）每窗重付 +~4K、命中 -0.3~0.5pp，收益递减。
5. **准确度判定**：8K 截断信息守恒（落盘完整、可补读），准确度差异被闭环
   吸收；高严谨性任务宁可多一次补读、不可缺失信息——"高触发 + 明确指针"
   对高严谨性任务更诚实。补读可靠性为设计硬约束，不是 8K 的固有代价。

## 3. 变更设计

### 3.1 统一限值（8K）

| 限值 | 现值 | 新值 | 位置 |
|---|---|---|---|
| 终端工具输出 | `DEFAULT_TOOL_OUTPUT_CHARS` 20,000 | **8,000** 字符 | orz-tools lib.rs |
| 点读窗口 | `RECEIPT_DETAIL_MAX_CHARS` 8,000 | 8,000（确认） | epoch.rs |
| 桥保留尾 | `fold_tail_tokens` 8,000 真实 token | 8,000（不动） | action_ledger.rs |
| 单轮输出上限 | `REQUEST_MAX_TOKENS` 160,000 | **32,000** | agent_loop.rs |

终端 8K 触发率（本次 make-doom 9 次输出样本）：78%（7/9 >8K）——"终端打印
大文件"几乎必触发截断 + read_file 引导；8K 以内正常结果完整可见。

### 3.2 工具结果可再读闭环（第三层核心）

**统一截断格式**（bash/终端工具结果超 8K 时）：
```text
[truncated: showing first/last X of Y]   ← 保留头尾，尾部优先（最新状态/错误最相关）
完整内容见 <落盘路径>，请使用 read_file 读取（大文件用 offset/limit 分页）
```
- 落盘路径：`.gsa/session/terminal/<order_id>.log`（bash 输出已落盘，现有
  `output_file` 字段；路径经 display 规范化，工作区相对路径）。
- **可读性已验证**：`.gsa` 为符号链接，read_file 的 gitignore 检查
  canonicalize 后路径在 git root 之外（`strip_prefix` 失败返回 not-ignored），
  直接放行；read_file 支持相对/绝对路径与 offset/limit 分页。
- 指针行机械附加（非模型生成），计入输出体积（同点读指针可见纪律）。
- **预算口径**（2026-08-19 全面审查处理 P3）：终端截断从字符预算中预留
  指针块（"\n\n" + 指针行），截断内容 + 指针块合计 ≤ 8K（与点读「整体
  ≤ 上限」对齐）；头部（exit 行/截断标注）与前后截断分隔符为既有渲染
  开销，不在本设计新增。未截断时不附加指针、预算全额用于内容。

**点读截断指针改向**：`完整内容见存档（epoch-N.json）/ TraceStore trace_id=…`
→ 优先指向落盘文件 `.gsa/session/terminal/<order_id>.log`（receipt 为
run_terminal 结果时）并附 read_file 指引；非终端 receipt（无落盘文件）保留
存档指针兜底。顺带闭合 F1（TraceStore 不在模型直接工具面）遗留。
**判定按发放时订单动作名**（`ActionResult.action`，2026-08-19 全面审查
处理 P1）：响应信封 `{"output": string}` 被 read_file/grep/run_tests/
search_replace/index 等 text-output 动作共用（同一
`text_output_response_schema`），不能作为终端判据；仅订单动作
`workspace.run_terminal` 且信封为 `{"output": string}` 才改向落盘指针，
其余（含旧 epoch 归档缺 action 字段）一律存档/TraceStore 兜底。

**桥截断保留工具结果自身指针**：桥内容级截断（`truncate_tool_reply_tail`）
从源原文截取时，若源 content 已含"完整内容见 \<路径\>"落盘指针行，截断形态
保留尾部即天然留存该行；仅当源 content 无落盘指针（未截断过的工具结果）
时，桥截断才生成 sha256 存档指针兜底。**桥 8K 信息损失因此可恢复。**

### 3.3 生成期实时复读检测（第一层，治本）

- 检测点：`transport::stream_once` 的 `on_chunk` 内容 delta 回调（生成期，
  非事后）。
- 检测算法（保守初值，S2 单测 + S4 观测校准）：
  - **连续相同块**：最近连续 N 个 content delta 完全相同（N=5）→ 触发；
  - **重复率**：累计输出 ≥ 1K token 且最近 1K token 内 3-gram 重复率
    > 60% → 触发（复用 stagnation 的 n-gram 思路，轻量实现）。
- 触发行为：主动中断流式请求（`GatewayError::StreamInterrupted { reason:
  "degeneration_detected" }`），journal 记录 `model_output`（incomplete=true +
  终止原因）后走 run_failed 同路径；**不重试**（已见输出，ADR-0007 纪律）。
- 防循环：同一 run 连续 `DEGENERATION_LIMIT`（默认 3）次退化中断 →
  `run_invalidated`（reason=degeneration），计入 stagnation 同类终止态。
  （2026-08-19 全面审查解释登记：一次 run_failed 即终止 run，「同一 run
  连续 3 次」字面不可达；实现为**会话级连续计数**——`DeepSeekTransport`
  实例级 `Arc<AtomicU32>`、任何成功请求重置，第 3 次起带
  `degeneration_limit_reached` 标记转 run_invalidated，防跨 run 循环，
  是防循环意图的可行实现。行为比字面更强，不改变终止语义。）
- 与现有停滞守卫关系：实时检测管生成期止损；停滞守卫保留（管跨轮重复），
  并补一条——失败轮次（run_failed 路径）也评估一次已输出文本的重复信号
  （审计留痕，不改变终止语义）。

### 3.4 单轮输出上限（第二层，止损）

- `REQUEST_MAX_TOKENS` 160,000 → 32,000（三 agent 统一，ADR-0010 §3.4.2
  "同一默认值注入"不变）。
- 依据：正常轮次 reasoning p95 5.2K + completion max 23.7K，合计 32K 覆盖
  正常分布 99%+；退化输出在 32K 截断（约 1–2 分钟流），消除 10 分钟 hang。
- 影响：`ModelConfig::max_tokens` 与请求级 min-cap 同步（transport.rs
  `deepseek_v4` 构造 + 测试断言更新）；请求头指纹含 max_tokens，部署后首次
  请求一次性指纹变化（既有纪律，登记）。

### 3.5 桥 8K 与统一 8K 的协同

- 工具结果统一 8K 截断 → 桥内消息体积更小 → 桥 8K（真实 token）在同等轮数
  下更易容纳 → 桥截断频率低于现状（现状 3/3 未截断）。
- 桥截断若发生：保留尾部（含工具结果落盘指针行）→ 模型 read_file 恢复 →
  连续性不损失。桥预算不因本设计调整。

## 4. 配置与参数

- `DEFAULT_TOOL_OUTPUT_CHARS`：20,000 → 8,000（终端工具输出上限）。
- `RECEIPT_DETAIL_MAX_CHARS`：8,000（不变，统一确认）。
- `fold_tail_tokens`：8,000（不变）。
- `REQUEST_MAX_TOKENS`：160,000 → 32,000。
- 实时检测：连续相同块 N=5、重复率 >60%（1K token 窗口）、
  `DEGENERATION_LIMIT`=3（初值，S4 校准）。

## 5. 实施路由

- **S1 代码**（orz）：
  - transport `on_chunk` 实时检测（degeneration guard）+
    `GatewayError::StreamInterrupted` 新 reason + 防循环计数；
  - `REQUEST_MAX_TOKENS` 32K（agent_loop.rs + transport.rs 构造 + 测试）；
  - 终端 8K（DEFAULT_TOOL_OUTPUT_CHARS）+ 截断格式统一（"完整内容见
    \<路径\>，请使用 read_file"）+ 落盘路径 display 规范化 + 指针块计入
    截断预算（全面审查处理 P3）；
  - 点读指针改向（epoch.rs：按 `ActionResult.action` 判定 run_terminal
    → 优先落盘文件 + read_file 指引，存档兜底；全面审查处理 P1）；
  - 桥截断保留工具结果自身落盘指针（action_ledger.rs truncate_tool_reply_tail
    / build_bridge：优先源 content 已有指针，sha256 兜底）；
  - stagnation 失败路径补评估（审计留痕）。
- **S2 测试**：实时检测触发（连续块/重复率/防循环上限）、32K 请求头断言、
  终端 8K 截断格式 + 指针行、点读指针改向、桥截断保留落盘指针、gitignore
  外路径可读（.gsa 符号链接）、回归全绿（fmt/clippy 基线）。审查处理补项：
  非终端 text-output receipt（`{"output": string}` + 非 run_terminal 动作）
  点读回退存档指针、截断预算预留指针块、.gsa 符号链接可读性专属测试。
- **S3 重建**：Linux musl（ORZ-BUILD-MOUNT-001 契约）。
- **S4 复验**：make-doom-for-mips 或同负载错题单题复验——无退化复读中断、
  无 400、命中率 ≥90% 保持、工具结果 read_file 补读路径真实可用。

## 6. 验收标准（DoD）

- S2 全绿、fmt 干净、clippy 与基线一致；
- S3 重建成功且时间戳更新；
- S4：无退化复读中断（或有触发但按设计终止、无 hang）、无 400、命中率
  ≥90%、补读路径可用（模型实际 read_file 落盘文件成功）；
- 计数：实施放行时入账 1 项（28 → 29），验证闭环后 29 → 28。

## 9. 审查处理登记（2026-08-19 全面审查）

- **P1（实现/符合性，已修复）**：点读指针改向原按响应信封
  `{"output": string}` 判定终端，但该信封被 read_file/grep/run_tests/
  search_replace/index 等 text-output 动作共用——非终端 receipt 超 8K 会
  给出不存在的 `.gsa/session/terminal/<order_id>.log` 死指针，违反「指针
  路径必须真实可读」与「非终端 receipt 保留存档兜底」。修复=发放时落盘
  订单动作名（`ActionResult.action`，serde 默认 None，旧归档安全回退），
  点读按动作 + 信封双判定；S2 补非终端 text-output 回归测试。
- **P3（已处理）**：①终端指针块计入截断预算（内容 + 指针 ≤ 8K，与点读
  口径对齐；头部/分隔符为既有开销）；②160K 陈旧注释清理（transport 测试、
  controller/agent_loop 文档、live 探针改 32K 与生产配置一致）；③新增
  .gsa 符号链接可读性专属测试（read_file gitignore 面）；④登记缺口同步
  （ADR-0010 §14.33 / BACKLOG 0d / TODO P0-0d / CLI_PROJECT_INDEX）。
- **解释登记**：「同一 run 连续 3 次退化中断」字面不可达（run_failed 即
  终止 run），实现为会话级连续计数（成功请求重置），见 §3.3。

## 7. 风险与回滚

- **实时检测误报**：正常长输出（模型生成大段代码/总结）可能触发重复率
  阈值 → 误中断。缓解：初值保守（1K token 窗口 + 60%），S4 观测；触发
  走 StreamInterrupted（可重试语义受"已见输出不重试"约束，误报代价=本轮
  输出丢失，下一轮继续）。
- **终端 8K 摩擦**：8–12K 正常结果（编译日志）截断需补读。缓解：8K 内
  完整（样本 2/9 落在 8–12K）；补读一次 read_file 成本可控；指针明确。
- **点读指针改向**：非终端 receipt 无落盘文件 → 存档指针兜底，不丢路径。
- **回滚**：各层独立可逆（限值参数、检测阈值、指针文案）；最坏代价=每窗
  一次全量视图或单轮输出截断，低频接受。

## 8. 关联

- 前置：LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN_2026-08-19（桥 8K）、
  BLACKBOARD_READ_CACHE_COST_DESIGN_2026-08-19（点读 8K 方案 B）。
- 不冲突：停滞守卫（跨轮）、压缩机制、折叠推进、缓存纪律。
- 登记：ADR-0010 §14.33 / BACKLOG 0d / TODO P0-0d / CLI_PROJECT_INDEX。
