# 210 批：SCB 报告 §13＝官方多维聚合离线补账（2026-10-07）

> **用户令**：「先走离线聚合吧，把结果补充进 D:\CLI\docs\SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md」
> （承接「官方榜单本身就有多维度评估……塌陷程度等等其他参数到底有没有按照官方口径得出来」
> 之问的核实结论）。**性质**：零重跑、零模型调用、零源码；计数不变 **63**；未提交、未推送。
> **结论先行**：**官方多维数据在 0cr 轮随跑已采齐**——官方 harness 在 eval 时自动产出每档
> `checkpoint_results.jsonl`（strict/core/isolated 三 pass rate＋`scb-check==0.1.3` 的
> verbosity/erosion 复合分），36×196 全档在案；离线聚合（对齐官方 `metric.py` dataset 级语义）
> 出账：**isolated 满通过 60/196＝30.6%**（榜首 GPT 5.5/Codex 同构口径 28.1%——方向性略高，
> 聚合公式待官方确认）、**strict 满通过 44/196＝22.4%**（论文 Opus 4.6 17% 同构）、verb 均值
> 0.289（榜首 0.269 同量级）、ero 均值 0.613（榜首 0.494，偏高如实记）。**多维缺口属出账
> 口径选择，不构成重跑理由。**

---

## §1 指标定义（源码核证，pin `31ceea3`）

- `strict_pass_rate`＝全部测试通过率；`core_pass_rate`＝Core 组通过率；**`isolated_pass_rate`
  ＝当前 checkpoint 自身测试（扣除 Regression）通过率**（`metrics/checkpoint/extractors.py`）。
- 官方 dashboard 二值 checkpoint 通过＝strict 满分 ∨ isolated 满分 ∨ 全部测试通过
  （`dashboard/data.py` `passed_chkpt`；本轮恰等于 isolated 满通过集合，60/196）。
- `verbosity`/`erosion`＝`scb-check==0.1.3` 复合分（driver 版本钉死、随档记录；跨版本不可比）。

## §2 全轮读数（写入报告 §13.2）

| 口径 | 值 | 对照 |
|---|---|---|
| isolated 满通过占比 | **60/196＝30.6%** | 榜首「Isolated Solve」28.1%（同构推定、聚合公式待官方确认） |
| strict 满通过占比 | 44/196＝22.4% | 论文 strict：Opus 4.6 17%／GPT-5.4 11% |
| core 满通过（any） | 134/196＝68.4% | 既有定档 |
| isolated / strict / core 均值 | 79.8% / 83.0% / 83.4% | 部分得分口径，与二值不可直比 |
| verbosity / erosion 均值 | 0.289 / 0.613 | 榜首 0.269 / 0.494——verb 同量级、ero 偏高如实记 |
| verb·ero 增长率（逐档转移） | 58.75% / 65.0% | 论文 ~80% 轨迹上升为常态——低于常态 |

难度面（isolated 均值）：Easy 91.1%／Medium 76.1%／Hard 72.1%。塌陷信号面（isolated 首→末
档）：走低 9 例（eve_route_planner 1.00→0.38、dynamic_buffer 0.93→0.18、dag_execution
0.85→0.00、file_query_tool 1.00→0.60、dynamic_config_service_api 1.00→0.56、
database_migration 1.00→0.70、l2m 0.98→0.62、recli 1.00→0.68、migrate_configs 1.00→0.67）；
反向 5 例（env_manager 0.36→0.88、file_backup 0.59→0.86、mocked_http 0.83→0.94、
etl_pipeline 0.90→1.00、cfgpipe 0.89→1.00）＝官方口径下非单调恢复成立。

## §3 执行与工件

- 聚合件：`D:/tb-eval/scbench/0cr_official/multidim_aggregate.py`（只读；输出
  `multidim_aggregate.json`／`multidim_aggregate_out.txt`）——枚举 10-05/06 的 36 个 run 目录
  （题集与冻结 manifest 一一核实）、逐档读 `checkpoint_results.jsonl`，dataset 级聚合沿
  `metric.py` 语义（trial 均值→跨 trial 均值；增长率＝Σ增加/Σ转移）。
- 报告：`SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md` 新增 **§13**（定义＋全轮表＋难度面＋
  塌陷信号面＋边界），§1 摘要行、§12 复现入口行同步。

## §4 台账

- 本档：`docs/audits/210_SCB_MULTIDIM_AGGREGATE_2026-10-07.md`。
- BACKLOG：指针行/计数行（63 不变）＋`0cr` 专节多维注记；第二卷 §1.158。
- TODO：计数行本批指针。
- 索引：头行 v4.182 → **v4.183**。
- 门禁 `check_repository.py` ⇒ 结果随本档补记（预期 error_count 1＝orz submodule dirty）。

## §5 边界

1. **零重跑、零模型调用、零源码**；官方「Isolated Solve」榜单聚合公式未文档化——30.6% 的
   对照以「isolated 满通过占比」为最合理推定，待官方确认（均值 79.8% 与二值 30.6% 并列出账
   防误读）。
2. 未提交、未推送；上榜路径决策（Discord 询问／Harbor 试点）仍待用户裁决，本批不展开。
3. 207 报告既有章节（§1–§12）定档读数不动；§13 为增补节。

## §6 关键词

210 批、官方多维聚合、checkpoint_results.jsonl、scb-check 0.1.3、isolated 满通过 30.6%、
strict 满通过 22.4%、isolated 均值 79.8%、verb 0.289、ero 0.613、塌陷信号面、非单调恢复、
离线补账零重跑、报告 §13、计数 63 不变。
