# 黑板读取缓存成本处理设计（2026-08-19 设计定稿；S1/S2 已实施，F1 处理=方案 B 定稿登记）

> 状态：`S1/S2 已实施（orz 工作树，未提交）`；`F1 处理=方案 B 按需点读（2026-08-19
> 用户定稿、未实施，见 §4.5）`——用户裁决：大机制不再更改，只补回应命中而未命中
> 的缓存成本；模型按需使用工具能力稳定且走 read 面，整段恒定优先。
> 登记：ADR-0010 §14.31（v1.31，2026-08-19 补充）/ BACKLOG 0c / TODO P0-0c /
> CLI_PROJECT_INDEX。
> 关联：FUS-REQUEST-CACHE / ORZ-CACHE-CONTEXT-COST / FUS-COMPACTION-REDESIGN
> （v1.29 机械压缩 / v1.30 HA 事实聚合）。

## 1. 背景与证据

2026-08-19 S4 换题复验（path-tracing，正式 1800s 预算、40 次模型请求、
29m51s、reward 0.0、无 400、无隐藏摘要请求）：

- provider 账单口径命中率 **85.43%**（02:00–03:00 时段：hit 1,873,792 /
  miss 319,486）；journal token 口径 **86.04%**（hit 1,832,064 / miss
  297,135）。
- miss 归因：**8/8 大尖峰（≥10K，合计约 190K = 总 miss 的 64%）全部紧跟
  `blackboard_read`**（actions ×7 / exec ×1）；单次分区结果 17–32K token
  作为全新工具结果注入，前缀缓存按消息位置匹配，新位置即使内容未变也整体
  重付。
- 折叠重建为次要项（4 次折叠，折叠后首请求约 10–32K）；计划/头部变化更小。
- 结论：**不是折叠频率、不是大机制问题**；是「应该命中而未命中」的重复大
  载荷注入。

## 2. 战略裁决（用户 2026-08-19）

- **大机制方向不再更改**：orz 的频繁折叠/压缩与 Codex 窗口手动频繁压缩的账单
  趋近 → 已可趋近成熟产品的缓存水平；折叠 + 机械压缩 + 黑板/外挂台账架构保留。
- 只处理应命中而未命中的部分：本设计（黑板分区读取载荷）+ 既有审计项
  （供应商 41 vs journal 40 的 +1 请求，另立跟踪）。

## 2.1 参考基准（2026-08-19 账单核对补充）

用户提供 Codex 窗口账单（`api_key_name=codex`，08-19 00:00–03:00）：

| 时段 | 请求数 | 命中率 | 单请求 miss 均值 | 成本 |
|---|---|---|---|---|
| 00:00–01:00 | 268 | 97.12% | 5,372 | 5.33 元 |
| 01:00–02:00 | 225 | 98.05% | 3,005 | 3.54 元 |
| 02:00–03:00 | 70 | 99.53% | 1,450 | 1.61 元 |
| 合计 | 563 | 97.91% | 3,937 | 10.48 元 |

对照 orz path-tracing 复验（02:02–02:32）：41 次请求、命中率 85.43%、单请求
miss 均值 7,428、成本 1.13 元（约 2.76 分/请求，Codex 窗口约 1.86 分/请求）。

**结论**：两者尚未趋近——Codex 窗口单请求 miss 仅 1.45–5.4K 且随会话递减；
orz 的差距全部集中在 `blackboard_read` 17–32K 尖峰。修掉后 orz 单请求 miss
可落入 Codex 区间（约 2–3K），命中率约 95%——即 Codex 水平（97–99.5%）的
下沿，是大机制不变前提下的合理目标。

## 3. 目标与边界

**目标**：`blackboard_read` 单次返回载荷从 17–32K token 降到 1–3K token；
预期总 miss 减少约 160–180K，命中率从 86% 升至约 95%。

**边界（不变项）**：
- 零模型调用、纯机械、有界；
- 工具契约：`blackboard_read` 仍为字符串返回，section/epoch/since 参数不变；
  **F1 处理（方案 B，§4.5）增量扩展可选 `receipt_id` 参数**——整段读行为
  不变，仅模型按需点读时使用；
- schema / 事件面无变化；
- 折叠阈值（128K）、压缩阈值（192K/200K）、五槽 17K、D1=(c) 事实聚合均不动；
- 不改黑板的单一写者、plan epoch 生命周期与归档语义。

## 4. 变更设计

### 4.1 actions 段渲染瘦身（epoch.rs `render_section` "actions"）

- results 板：**不再嵌入 response JSON / 完整 error 对象**；每条固定形态
  （约 ≤90 字符）：
  `<order_id> ok=<bool> step=<step|?> code=<code|?> trace_id=<trace_id>`
  （step/code 取信封既有字段，缺失回退 `?`——与 D1=(c)
  `failure_envelope_fields` 同口径）；
- `RESULTS_RENDER_CAP = 10` 与总数头行保留；
- registration 板 / order 板保留原样（模型下单需要契约与单槽状态，且体量
  小、稳定）。

### 4.2 exec 段渲染瘦身（epoch.rs `render_section` "exec"）

- 每条结果/错误行按字符截断到 200 字符（复用 summary.rs `truncate_chars`
  口径：199 字符 + 「…」）；
- `EXEC_RENDER_CAP = 50` 保留；
- 段总长上限 4K 字符（约 2K token），超限保留头行 + 计数行（与 actions 段
  同风格）。

### 4.3 （阶段 2，可选）`blackboard_read` `since` 扩展至 actions 段

- results 按 `since` 过滤（仅返回新增 receipt）；registration / order 仍全量；
- 工具描述注明可传 `since` 做增量读。

### 4.4 （阶段 2，可选）读取频率引导

- 动作栏写入前不重复读 actions：单槽满时写入会被机械拒绝并反馈原因
  （`console_order_rejected`），依赖拒绝反馈即可；提示词补一句纪律。

### 4.5 （F1 处理，2026-08-19 用户定稿）`blackboard_read` 按需点读（方案 B）

**背景**：S1/S2 实施后全面审查（2026-08-19）发现 F1（P1，设计层面）——设计
§4.1 假设「大载荷留在存档与 TraceStore，模型按需经 trace 回查」在 console
默认面机械上不成立：`assistant.trace` 不在 console 直接工具面（只能经订单
下发），其响应同样是结果栏 receipt、同样被瘦身隐藏；执行类订单
（run_script / run_tests / run_terminal）的成功 response 与失败信封
message/upstream 因此全部不可回查。用户 2026-08-19 裁决：模型按需使用工具
能力稳定且走 read 面，采用**方案 B 按需点读**，整段恒定优先。

**变更**：
- `blackboard_read` 新增**可选**参数 `receipt_id`（string，minLength 1）：
  结果栏单条 receipt 完整内容点读（含 response/error 全文，有界截断）；
  receipt_id = 结果行行首的 order_id（如 ORD-000012）。仅与
  `section=actions` 组合有效。
- 组合语义：与 `epoch` 组合 = 归档快照点读（cross-epoch）；与
  `since_timestamp` 同时给 = 忽略 since（点读按 id 寻址，时间过滤不适用，
  工具描述注明）；`section` 非 actions 却带 receipt_id = 显式报错（fail
  loud，同未知分区风格）；格式非法（非字符串/空串）= 显式报错（同非法
  epoch 处理）；合法但未找到（live 板仅保留最近 50 条）＝显式「not found」
  + 提示可试旧 epoch 归档。
- 渲染：无 receipt_id 时输出与 S1 逐字节一致（整段恒定）；点读返回固定
  形态行 + 有界完整内容——成功 `response=<JSON 原文>`，失败
  `error=<JSON 原文>`（含 message/upstream，此前模型从未见过这两项）。
- 上限：`RECEIPT_DETAIL_MAX_CHARS = 8_000`（约 4K token；用户 2026-08-19
  定档：「8K 够用，再多去原文档/存档查找」）；超限按字符截断 + 「…」+
  指针行「完整内容见存档（epoch-N.json）/ TraceStore trace_id=…」。
- 零模型调用、纯机械；黑板数据面/事件面/epoch 语义不动；仅工具定义（参数
  + 描述）增量扩展——整段读行为不变，header 指纹仅部署首轮一次性变化
  （v1.19 已接受的工具面变化纪律）。

**缓存特性**：整段 actions 字节恒定（最优前缀命中）；点读为模型主动的
单条读取，一次性 ≤8K 字符载荷，不再有每轮整段 17–32K 重付。

**实施路由**（B，待用户放行）：S1 代码——epoch.rs 点读分支 + 上限常量 +
截断指针；controller.rs 参数解析（非法值显式报错）与透传；blackboard_read
工具定义参数/描述更新。S2 测试——单测（完整 response/error、8K 上限截断、
未找到、非 actions 报错、跨 epoch 点读、**无 receipt_id 与 S1 输出逐字节
相等**）；工具级测试（真实工具链点读回达）。S3 重建；S4 复验（≥90%、
无 400）。计数不变（0c S4 前置修订）。

## 5. 预期效果与度量

- 单次 actions/exec 读取 17–32K → 1–3K；本轮重复读取 miss 约 190K →
  20–30K；
- 命中率（provider / journal 同口径）86% → 约 95%；
- 不改变请求次数 / 折叠次数；只改变单次工具结果载荷。

## 6. 实施路由（待用户放行）

> 2026-08-19 更新：S1（渲染瘦身）与 S2（测试）已实施完成（orz 工作树未提交、
> orz-loop 479 通过）；F1 处理=方案 B（§4.5）待放行后实施，路由见 §4.5。

- S1 代码：epoch.rs 两段渲染瘦身 + 单测（构造 50 条大 receipt / 超长 exec
  行，断言段上限、无 response JSON、截断语义）；
- S2 测试：`render_section` 单测 + `blackboard_read` 工具级断言；
- S3 重建：Linux musl（ORZ-BUILD-MOUNT-001 契约）；
- S4 复验：同题/换题 1800s 正式预算，provider / journal 命中率 ≥90%、无
  400、无隐藏请求。

## 7. 验收标准（DoD）

- 单测全绿（渲染尺寸 / 截断 / 字段缺失回退）、clippy 与基线一致；
- B 单测：点读完整返回 / 8K 上限截断 / 未找到 / 非 actions 报错 / 跨 epoch
  点读 / 无 receipt_id 与 S1 输出逐字节相等；工具级点读回达；
- S4 复验命中率 ≥90%（provider usage 口径）、无 400；
- 计数：本设计为 0c S4 前置，不新增未闭合计数（维持 29）。

## 8. 关联与审计项

- 供应商 41 vs journal 40（+1 请求，约 4.2 万 hit / 2.2 万 miss / 0.97 万
  output）登记为传输层计费审计项（非本设计范围，另立跟踪）。
