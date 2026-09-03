# TER T0.4 M0 放行签名（2026-09-03）

> 上级：TODO2 T0.4（M0 设计门放行门）；开放项
> [`BACKLOG2.md`](../BACKLOG2.md) TER-0；前置审计：TER_T0_1 /
> TER_T0_2 / TER_T0_3。
> 状态：T0.4 完成（签名记录已写回 TODO2.md）；M0 设计门全部闭合，
> 放行进入 M1（T1.1–T1.13）。

## 1. 放行齐备条件（逐项核对）

| 条件 | 证据 | 结果 |
|---|---|---|
| T0.1 默认值收敛核对表（覆盖 §3.1 全 6 行 + 单一生效源方案） | [`TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md`](TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md) | ✓ |
| T0.2 schema/verifier/fixtures 先行（枚举 53→54、payload v0.2、idle-kill 形态） | [`TER_T0_2_SCHEMA_VERIFIER_FIXTURES_CONTRACT_2026-09-03.md`](TER_T0_2_SCHEMA_VERIFIER_FIXTURES_CONTRACT_2026-09-03.md)；校验/夹具/测试全绿 **273 passed** | ✓ |
| T0.3 ADR-0010 §14.53 候选项 3 条落盘、索引无冲突 | [`TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md`](TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md)；CLI_PROJECT_INDEX v2.48 登记 AUTH-TOOL-EXECUTION-REFORM（`pending`） | ✓ |
| 生成器 v0.1 残留处理（方向 A）落地 | T0.3 审计 §4/§5；`tool_running` 移出 v0.1 `EVENT_TYPES`，v0.1/canonical_cli 重建与 git 索引 **0 差异** | ✓ |

## 2. 用户确认与签名

- 2026-09-03 用户指示：“请登记‘生成器 v0.2 表全面对齐’为专项吧；可进入
  T0.4。”——① 生成器专项登记于 BACKLOG2 TER-0.1（不阻塞本门/M1）；
  ② T0.4 放行门开放并签名。
- 签名结论：M0 设计门（T0.1–T0.4）闭合，无未闭合阻塞项；**放行进入
  M1**（TODO2 T1.1 起：`auto_background_on_timeout` struct 默认
  false→true，含“关闭态仍可配”用例）。
- 边界：本次只签 M0→M1 放行；M1 每步仍须“改代码 + 单测 + 验收”，M1
  终点 T1.13 另有全量验收放行门，不因本签名预支。

## 3. 遗留与后续

- 生成器 v0.2 表全面对齐：已单列 BACKLOG2 TER-0.1 专项，M1 期间可并行
  抽做；重跑生成器前先读 T0.3 审计 §4/§5（试运行会重建三棵夹具树；
  已跟踪文件还原用 git checkout-index）。
- M1 实施起点：T1.1（S5-2 默认开启，orz 主线 Linux 单测/构建闭环）。
