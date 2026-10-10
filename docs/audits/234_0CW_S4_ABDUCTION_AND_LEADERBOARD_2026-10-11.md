# 234 批：0cw S4 收取与对拍——四层对拍＋榜单核证＋V2 全轮报告交付（2026-10-11）

> **用户令**：「请开始进行S4吧」。
> **性质**：分析＋对拍＋报告批——零源码零跑批（复算件为评测侧本地件）；计数不变 **56**
> （0cw S4 达成、**闭合待用户裁决**）；未提交、未推送。

## §1 交付物

- **主交付**：[`SCB_V2_RESET_FORM_FULL_ROUND_REPORT_2026-10-11`](../SCB_V2_RESET_FORM_FULL_ROUND_REPORT_2026-10-11.md)
  ＝0cw 重置形态全轮报告（四层对拍＋难度面＋形态学＋公开面核证＋边界）。
- 复算件：`D:/tb-eval/scbench/s4_0cw_ab.py`＋`s4_ab_readings.json`（全部对拍数字机械可复算；
  原始面＝196 行逐档八指标 ledger）。

## §2 四层对拍要点（详见 V2 报告 §4）

| 层 | 读数 |
|---|---|
| ① 逐档 | solved 131/196（66.8%）vs 0cr 134（68.4%）＝−3；**strict 满通过 +17 档（31.1% vs 22.4%）、isolated 满通过 +22 档（41.8% vs 30.6%）＝满通过口径重置反超** |
| ② 逐题 | 双侧极端并存：meshctl **−0.958**（7/8→全零）vs eve_jump_planner **+0.833**（全零→满分）；|Δ|≥0.1 共 20 题——−3 档是剧烈方差净额 |
| ③ 相位 | **0cw 92.0%→74.5%（缓降）≈ 0cr 94.0%→77.7%，论文重置队列 64.6%→35.5%（坍缩）——「重置即坍缩」预期被同模型对照落空**；末档满分 20/36、末档零分仅 5/36 |
| ④ 轨迹级 | erosion 上升 86.1%（0cr 80.6%/论文 77%）、verbosity 83.3%（77.8%/75.5%）、erosion 均值 0.522（0cr 0.613）——「质量退化轴不随形态移动」成立 |
| 难度面 | Easy 88.1% 双方持平；Medium 重置 +7.1pp；**Hard 连续 +9.7pp——连续性收益全部集中在 Hard** |
| 全解集 | 15 vs 11（0cw 独有 6：eve_jump_planner/sith/l2m/migrate_configs/mvvault/file_query_tool；0cr 独有 2：env_manager/eve_route_planner） |

## §3 榜单与公开面核证

- **Hub**：38 job（36 题＋precheck＋1 零数据残件）经 `harbor hub job list` 全量核验全 finished；
  **36 题全 public**；构建失败零数据重复名残件（`b31a3480`）已从 Hub 删除（本地证据档保留）；
  recli/pwd_manager 断窗补传在案。
- **scbench.ai 榜单**：截至 10-11 **未自动聚合出 orz 条目**（仍论文 19 条）——208 批「事实入榜
  通道」的聚合环节未自动发生，如实登记；后续选项＝#40 回贴附 hub 链接／Discord 人工通道／
  观察聚合周期（随用户裁决）。
- 方向性注记（沿 0cr 报告 §6 谨慎口径）：同构口径（isolated 满通过占比）41.8% vs 榜单 top
  28.1%；会话形态已对齐（均重置），模型价位差与 harness 差并存、归因不断言。

## §4 台账

- 本档：`docs/audits/234_0CW_S4_ABDUCTION_AND_LEADERBOARD_2026-10-11.md`。
- BACKLOG：本批记录指针＋计数行＋`0cw` 专节 S4 达成注记；计数不变（56）。
- TODO：计数行指针＋P1 路由＋`P1-0cw` S4 勾选达成（**0cw 闭合待用户裁决**）。
- BACKLOG 第二卷：§1.179。
- 索引：头行 v4.206 → **v4.207**；§6 `0cw` S4 注记；§8 pending 桶。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑。

## §5 关联与关键词

[`V2 报告`](../SCB_V2_RESET_FORM_FULL_ROUND_REPORT_2026-10-11.md)／
[`V1 报告（0cr 基线）`](../SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md)／
[`233 批档`](233_0CW_S3_FULL_ROUND_COMPLETE_2026-10-10.md)／
[`226 批档`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md)（母批与对照表定义）。

关键词：234 批、0cw S4、四层对拍、相位曲线 92→74.5、重置即坍缩落空、strict +17 档、
isolated +22 档、meshctl −0.958、eve_jump_planner +0.833、Hard 集中、榜单未聚合、hub 38 job、
V2 报告、闭合待裁决、计数 56 不变、索引 v4.207。
