# Terminal-Bench 2.0 探索性跑分口径审计（2026-08-08）

## 1. 定位与结论

本轮 Terminal-Bench 2.0 运行处于 **pre-beta 工程诊断阶段**，主要目的为发现 agent、运行时、工具链、超时守卫、journal 与 harness 适配中的不足，不是正式 benchmark、排行榜提交或真人 beta 测试。

结论：现有 verifier PASS 可继续作为真实的任务结果和缺陷定位证据使用；当前样本与正式评测之间的偏差对这一目的影响有限。需要修正的是汇总名称和解释边界，而不是否定已经获得的工程信息。

## 2. 勘误后的结果台账

截至 2026-08-08 晚间，原先称作“hard 池累计 12 题 = 10/12”的台账实际由 **11 个 hard 任务 + 1 个 medium 任务**构成。`custom-memory-heap-crash` 的官方难度为 medium，不应计入 hard 分母。

| 口径 | 结果 | 当前用途 |
|---|---:|---|
| 12 项混合诊断台账，按每任务 best-observed | 10/12 | 探索性覆盖与“是否曾解出”记录 |
| 11 个 hard 唯一任务，按每任务 best-observed | 9/11 | hard 任务探索性覆盖，不是 pass@1 |
| 11 个 hard 唯一任务，取第一次获得 verifier 分数的尝试 | 7/11 | 辅助观察首次解题表现 |
| 全部有 verifier 分数的 hard 试次 | 9/17 | 辅助观察稳定性与重跑成本 |

持续未通过的两个 hard 任务为 `regex-chess` 和 `polyglot-rust-c`。`dna-assembly` 在 5 个有分数试次中仅 1 次通过，另有 1 个无分数启动/运行错误；其 16:02 PASS 同时带 `AgentTimeoutError`。因此它证明“最终容器状态曾达到 verifier 要求”，但不证明稳定通过或进程正常结束。

`write-compressor` 首次在旧二进制上超时为 0.0，20:55 重建守卫二进制后重跑为 1.0。该结果适合作为 stale-binary/挂死链路修复后的针对性回归证据；若未来计算正式 pass@1，不应以重跑成功覆盖首次失败。

## 3. 补样本 5 题的准确解释

`configure-git-webserver`、`sparql-university`、两项 FEAL 密码分析和 `write-compressor` 五个选定 hard 任务按 best-observed 为 5/5。

其中：

- 前四题在 19:32 批次使用 17:02 旧二进制并首次通过；这些 verifier 结果是真实的任务成功，但不验证新守卫二进制。
- `write-compressor` 在同一旧二进制批次超时失败；仅该题于 21:20 使用 20:55 新守卫二进制重跑并通过。
- 因而“选定任务集合 best-observed 5/5”成立；“新守卫二进制 5/5”不成立。新守卫二进制当晚直接任务证据只有 `write-compressor` 1/1。

## 4. 已确认可信的部分

1. Harbor 0.20.0 为实际执行 harness；trial lock 固定了 Terminal-Bench 2.0 task commit/digest、任务名、agent 配置、超时倍率和挂载配置。
2. 计为 PASS 的任务均有 `reward.txt = 1.0`；存在 CTRF 的 PASS 任务中，测试 summary 均为全通过、零失败。
3. 只读扫描现有 `events.jsonl`、`trajectory.json` 和 `orz.txt`，未发现访问 `/tests`、`solution.sh`、oracle、verifier 或 reward 文件的明显记录。该扫描只能排除显见的本地路径泄漏，不证明不存在模型训练污染或语义级捷径。
4. 现有运行已实际暴露并推动修复多项工程问题：Linux cfg 构建缺口、`--allow-write` 必要性、`PYTHONPATH` 启动条件、无 tracing 取证盲区、stale binary、工具执行无超时、总 wallclock、活动停滞看门狗和 journal 宿主卷落盘。

## 5. 当前阶段接受的偏差

以下偏差在“找不足”的 pre-beta 阶段可以接受，但必须随结果一起保留，不升级成正式分数：

- 任务为人工选择的小样本，而非随机抽样或完整 89 题；
- 重跑目的混合了环境排障、稳定性观察和修复回归；
- 部分批次使用 `agent_timeout_multiplier=2.0`，属于扩展预算；
- 多个源码/二进制阶段的结果被放在同一诊断台账中；
- adapter 评测上限为 999 tool rounds，而产品默认是 40；
- trial 未记录不可变的 orz 二进制 SHA、adapter SHA、具体模型 checkpoint/provider，token/cost 字段为空。

这些限制不妨碍使用结果发现薄弱任务和运行时故障，但会阻止与论文、排行榜或其他 agent 做强比较。

## 6. 正式评测前再启用的严格项

进入真人 beta 或需要对外比较前，再统一落实：

1. 冻结并记录 orz commit、二进制 SHA-256、adapter SHA-256、Harbor 版本、task digest、模型 provider/checkpoint 与完整参数。
2. 预先声明样本、超时、并发、重跑与基础设施失败处理规则；默认预算与 2× 预算分榜记录。
3. 正式 pass@1 不用后续成功覆盖首次失败；稳定性另报每任务多次试次分布。
4. 使用完整 89 题，或使用运行前固定且有代表性的样本；与论文比较时匹配其完整数据集和重复运行口径。
5. 报告分母、错误数、置信区间、token/cost 和所有排除项，不只报告 best-observed。

以上严格项当前不阻断继续用 TB/SWE-bench 做工程缺陷发现。

## 7. 证据来源与审计边界

本审计使用以下只读来源：

- 项目路由：`CLI_PROJECT_INDEX.md`
- TB 交接：`docs/TERMINAL_BENCH_2_EVAL_2026-08-08.md`
- 挂死守卫记录：`存档/docs/implementation-history/RUN_STALL_GUARDS_PLAN_2026-08-08.md`
- adapter：`D:\tb-eval\tb_agents\orz.py`
- 全部 trial/job：`D:\tb-eval\jobs\**\{lock.json,config.json,result.json,verifier\*,agent\*}`
- 任务元数据：`D:\tb-eval\terminal-bench-2\*\task.toml`
- 当前 Linux 二进制：`D:\tb-eval\orz-linux\orz`
- Terminal-Bench 论文：arXiv:2601.11868（完整 89 题、每个支持的 agent/model 组合至少五次运行的论文口径）

现有 Harbor JSON 形状不匹配 FEP `validate_fep_result.py` 的 `results[]` 或通用 artifact 表结构，因此本次采用逐 trial 的人工字段审计，没有把不匹配的 validator 输出当作证明。
