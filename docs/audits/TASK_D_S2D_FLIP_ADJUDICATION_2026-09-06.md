# 任务 D registry 翻转前两项裁决登记（2026-09-06）

> **登记日期**：2026-09-06
> **性质**：用户裁决登记（[S2c 复审处理 §3](TASK_D_S2C_REVIEW_HANDLING_2026-09-06.md)
> 翻转前裁决清单两项）——**裁决已定，实施待放行**（用户指示：先落文档、
> 不开工）。实施时按本文 §3 批次推进，涉及 ADR-0010 的部分按 §14.x 转录。
> **关联**：[S2a 盘点表](TASK_D_S2A_INVENTORY_2026-09-06.md) /
> [S2c 复审处理](TASK_D_S2C_REVIEW_HANDLING_2026-09-06.md) / ADR-0010
> §14.39（FUS-MECHANICAL-AUDIT-LAYER 写单面退役）/ [BACKLOG 00 S2d](../BACKLOG_AND_PRIORITIES.md)。

## 1. 裁决一：console_order_written / console_order_rejected 目标语义

**用户裁决（2026-09-06）**：当前仅保留「先读后写」（read-anchor 写前核证）
即可，写单摩擦彻底去掉，该退役的就退役。

**目标语义**：

1. **`console_order_written` 规则整体退役**。写单链
   （`blackboard_action_write` → written 背书链）随 FUS-MECHANICAL-AUDIT-LAYER
   （ADR-0010 §14.39）正式退役，生产零发射为既成事实；Python 法官与 Rust
   conformance 两侧同步退役该规则。**不转「written 不得再现」负检**——
   2026-08-16~24 时代历史 v0.2 期刊合法含 written 链，负检会误伤历史刊回放。
   生产侧零改动（不复活写单路径）。
2. **`console_order_rejected` 规则：退役摩擦、保留形状**。退役「须有在先
   同 run written + round/plan_epoch/run_id 戳一致」两个摩擦子规则（写单面
   退役后天然不可满足，且拒绝事件本就发生在任何 written 之前）；**保留三个
   形状子规则**：phase/step/code 三元组封闭（pre_issue 3 码 / issue 4 步 /
   policy 归一）、reason 非空、每 order 至多一条——三者由
   `console_exec.rs` 统一构造器机械固定，对真实期刊零摩擦，是仍在发射的
   拒绝事件仅存的结构化审计不变量。
3. **rejected 发射点保留**：拒绝审计面不变（五个发射点照常留痕）。
4. **先读后写不受影响**：read-anchor 写前核证（`content_anchor_mismatch`）
   是生产硬门、不依赖判官规则族，为唯一保留的写纪律。
5. **边界登记**：写单链退役后无机械核对兜底；未来若复活写单路径，重新立项
   （含规则重建与 ADR 转录）。

> **2026-09-06 批 1 三路复审勘误**（[复审处理](TASK_D_S2D_BATCH1_REVIEW_HANDLING_2026-09-06.md)）：
> ① 本节 1.2/1.3「仍在发射的拒绝事件」「五个发射点照常留痕」仅在**代码保留**
> 意义上成立——发射点运行时休眠（订单槽唯一生产写入面 `blackboard_action_write`
> 已随 §14.39 窄门拒发，rejected 与 written 同源休眠）；形状子规则保留语义
> 不变，ADR-0010 §14.57 项 2/4 已按此修正。② 复活立项须一并裁决休眠发射点的
> `content_anchor_mismatch` 码不在保留的 pre_issue 三码封闭集内（§14.57 项 4
> 已登记）。③ 批 1 实施 + 复审处理提交：orz `362b6071` + `141bd2cf`。

## 2. 裁决二：probe_accuracy 封存工具翻转语义

**用户裁决（2026-09-06）**：封掉的工具在当前流程中不会再被开放给模型
（除非未来工具面从根本上发生变动），工具栏刷新机制已全面退役；请求头探针
只要作用于当前可见工具即可。

**目标语义**：

1. **不变量收窄管辖范围**：「complete 集翻转 ⇔ 主车道 header change」
   交叉核对只作用于**当前可见（声明面）工具**；封存工具的探针翻转不属于
   该不变量管辖范围，不再产出违规。
2. **生产侧唯一改动**：`tool_probe` 的 `tool_availability_check` 记账口径
   收窄——封存工具不进 complete/incomplete 集（S2a 注记的翻转留痕点
   `orz-loop tool_probe.rs:983` 一带；实施时以现场代码回验记账形态为准）。
3. **判官两侧零改动**：Python 法官与 Rust conformance 的 probe_accuracy
   规则字面不动——生产停止记录封存翻转后，现有规则对真实刊零误报。
4. **合法翻转继续受查**：检索模式 A 浏览器能力降级等真实声明面翻转
   （header 随之变化）仍在不变量管辖内——「抓探针失真」的价值完整保留。
5. **边界登记**：未来工具面从根本上发生变动时，本裁决重新议定。

## 3. 实施批次规划

> 2026-09-06 用户先后放行两批裁决；**批 1、批 2 均已实施完毕，S2d 翻转前
> 裁决清单清空**——批 1（orz `362b6071`，见
> [批 1 实施审计](TASK_D_S2D_BATCH1_IMPL_AUDIT_2026-09-06.md)）、批 2
> （orz `00b9a440`，见
> [批 2 实施审计](TASK_D_S2D_BATCH2_IMPL_AUDIT_2026-09-06.md)）。

- **批 1（裁决一落地）——已完成 2026-09-06**：ADR-0010 §14.57 转录（写单
  链规则退役 + rejected 语义收窄）→ Python 法官同步（written 规则退役、
  rejected 去前置保留形状）→ Rust conformance 同步（families/families_s2c
  对应调整，31→30 族）→ 对拍复验（229 场景×30 族=6870 格 spec 表 +
  247 语料×30 族=7410 格 0 差；written 三场景转历史回放合法守卫）→
  BACKLOG/TODO 登记。
- **批 2（裁决二落地）——已完成 2026-09-06**：`tool_probe` 记账口径收窄
  （生产侧唯一改动，`narrow_to_declared` + run-start/逐轮两装配点接入，
  封存工具不进 complete/incomplete）→ ADR-0010 §14.58 转录 → 单测矩阵
  （纯函数三单测 + 两 e2e 改声明面载体 + 封存静默断言）→ probe_accuracy
  族对拍/fixture 复验零回归（判官两侧字面不动，250 语料×30 族 0 差）→
  登记。orz `00b9a440`，见
  [批 2 实施审计](TASK_D_S2D_BATCH2_IMPL_AUDIT_2026-09-06.md)。
- 两批完成后，S2d 翻转前裁决清单清空，可推进 S2d（30 族全量对拍矩阵落盘 +
  registry 翻转准备）→ S3（门禁改接）。

## 4. 裁决影响面小结

| 项 | Python 法官 | Rust conformance | 生产事件面 | schema |
|---|---|---|---|---|
| console_order_written 规则 | 退役 | 退役 | 零发射（不变） | 不动 |
| console_order_rejected 摩擦子规则 | 退役 | 退役 | 发射点保留 | 不动 |
| console_order_rejected 形状子规则 | 保留 | 保留 | 不变 | 不动 |
| probe_accuracy 规则 | 字面不动 | 字面不动 | 记账口径收窄（唯一代码改动） | 不动 |
| 先读后写（read-anchor） | 不涉及 | 不涉及 | 生产硬门保留 | 不动 |
