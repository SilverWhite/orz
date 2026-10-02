# 171 批：recli 二跑真机核证——0cp S4 达成＋0cn S4 达成（2026-10-03）

> **用户令**：「我考虑再次跑一遍slopcode bench的recli作为验收测试，你觉得怎么样？」（主会话评估：合适，判据 2 看门狗按测试钉口径收取、不换题）→「那请启动recli吧」。
> **本批**＝0cp/0cn S4 真机核证承载轮（recli 二跑）；单题 k=1 串行，机制证据为主、分数仅评判参考（152 批评测哲学）；不外推、不横比榜单。
> **载体**＝0.8.11（源冻结 orz `6493fdae`；身份门 `c6a0812c…`；170 批进体、ACAF 容器内自 provision）。
> **题目同源性**：与首跑（159 批 run5）checkpoint 1/4/8 prompt 逐字节 diff 相等——同题同模板，前后对照成立。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| 发射前置 | 载体 manifest 0.8.11 ✓（`orz` sha256 `c6a0812c…` 与身份门一致）；Docker 29.6.2；C: 12G／D: 38G 空闲（159 批 §4 满盘教训检查项）；基镜像两件在位；`ORZ_DEEPSEEK_API_KEY` 注入；**ACP 冒烟全绿**（裸容器握手＋小任务 10.4s，provision rc=0，signer `7fee74b1…` 与 manifest 逐位一致，`smoke.txt` 落盘，journal 在册） |
| 跑批形态 | **8/8 checkpoint 单会话全程**（8 run 共享前缀 `72764d4e`＋ARC 归档，`--fake` 零出现）；发射 05:15、收尾约 06:2x，**总墙钟 ≈53 min**（均值 399.2±162.7 s/chkpt；首跑 580.7±425.5 s）；**185 模型轮／172 工具调用／172 权限请求（全 yolo 自动应答）**；上下文压缩 4 次；session_archive 1 次；harness exit 0 |
| 分数（参考面） | core 轨迹 **11/11→9/9→5/7→4/5→1/5→0/6→2/5→6/7**（首跑 11/11→9/9→7/7→4/5→1/5→0/6→2/5→6/7＝同款「扩展→侵蚀→谷底→反弹」形态）；官方 strict＝**2/8**（ckpt 1/2；首跑更正口径 3/8）；erosion 均分 0.4531（首跑 0.5289）；不外推 |
| **0cp S4（§5 七判据）** | **①✓ 动作样累计**（320 采样全动作源、网格退役面零残留；唯一 >60s 命令段内零采样；抽检窗内「工具窗内」样本 9 条全部深度 0.0s＝并行批完成边界合法动作样）**②看门狗＝真机未触发**（全程无卡死、0 时间样；按设计「测试钉＋真机可选」口径以 S2/S3 行为钉收取）**③✓ k=3**（2 次 fire 均 `×3`）**④✓ 附注直投**（37/37 fire＝37 条 journal `mechanical_audit_update kind=rli_notice`、轮末装配点同刻〔tool_result 后 5–14ms〕、`notice_delivered_total=37` 恰一致、fire 全集可由 journal 复算＝0co 盲区闭环）**⑤✓ 回归**（零动作周期零注入、五通道全程在算、注入面族完整）**⑥✓ D6 格式**（37 行全带参数含义＋趋势读数无成因；字节宽 186/213/228 ≤240）**⑦✓ D7 二合一**（spike 进入 13/13 全提醒；回归端零专用提醒、域机器不变） |
| **0cn S4** | **✓** `kind=budget`＝0＋`预算`字串＝0（首跑 136→**0**，9 卷 journal 覆盖式扫描）；轮次感改由 lif_domain 98＋rli_notice 37＋context_scale 8 行族承载；`墙钟约` 保留面＝会话末机械审查报告块 face，bench 形态两轮均不渲染（首跑同），保留性由 170 批字节判据（件内 1→1）覆盖，非回归 |
| 计数与闭合 | 两项 S4 **达成落账**；0cp/0cn 维持开放（**计数 59 不变**）；`pending → implemented` 闭合**待用户裁决**（59 → 57） |

## §1 发射前置与冒烟

- 检查单（159 批 §4 教训固化）：Docker 引擎 29.6.2 ✓；C: 12G／D: 38G 空闲 ✓（上批满盘事故根因面）；`slop-code:python3.12`（1.18GB）与 `ghcr.io/astral-sh/uv:python3.12-trixie-slim` 基镜像在位 ✓；`D:/tb-eval/.env` key 文件在位、发射时显式注入进程环境 ✓。
- ACP 冒烟（`smoke_orz_acp.py`，裸容器仿适配器供应链）：initialize/session/new/prompt 往返 **10.4s**、94 updates、`smoke.txt`＝`hello-from-orz` 实写、journal `RUN-cd9377ef-0`；provision rc=0，`binary_sha256=7fee74b1…` 与 orz-linux manifest `orz-signer` 逐位一致＝0.8.11 三件套进容器正确。
- 发射命令（等价首跑）：`uv run slop-code run --agent orz --model deepseek/deepseek-v4-flash --problem recli`（rig 目录内；seed 42 默认、pass_policy any、one_shot off）；输出目录 `outputs/deepseek-v4-flash/orz_just-solve_none_20261003T0514`；全程零重试零停摆（对照首跑 run1–run4 事故链）。

## §2 跑批终局读数

| ckpt | CORE | 全测试 | 首跑 CORE（对照） |
|---|---|---|---|
| 1 | **11/11** | 34/34 | 11/11 |
| 2 | **9/9** | 66/71 | 9/9 |
| 3 | 5/7 | 92/105 | 7/7 |
| 4 | 4/5 | 117/133 | 4/5 |
| 5 | 1/5 | 125/151 | 1/5 |
| 6 | 0/6 | 137/184 | 0/6 |
| 7 | 2/5 | 153/214 | 2/5 |
| 8 | **6/7** | 156/255 | 6/7 |

- 轨迹形态与首跑高度同构（ckpt 3 差一档：5/7 vs 7/7，其余七档逐位相同）——「扩展→侵蚀→谷底→回归修复反弹」退化曲线在 0.8.11 载体上复现，机械层行为面无漂移信号。
- 墙钟均值 399.2s/chkpt（首跑 580.7s）＝快 31%；压缩 4 次（首跑 10）；模型轮 185（首跑 155）。
- `section=rli` PULL 消费＝**0**（`blackboard_read` 共 8 次调用、无一次 rli 节）——与首跑 0、0bf S3 基线 0/3 同值；0cp 不以消费率为判据（0cp 改造的正是**免拉取的直投面**），仅作连续性观察记录。

## §3 0cp S4 判据逐条核收（设计稿 §5）

| # | 判据 | 口径 | 证据 |
|---|---|---|---|
| 1 | streak 只由动作样累计；长命令段内零中间采样 | 真机 | 侧车 `sample_points=320`、**零时间样零网格补点**（`grid` 引用 9 卷 journal＝0、侧车无 grid 字段＝D1 退役面零残留）；全部 320 样中无一条落入任何工具执行窗内部（抽检 128 条 trace 中 9 条「窗内」样本深度全部 0.0s＝同批并行工具完成边界，非段内采样）；唯一 >60s 命令（`run_terminal_cmd` 61s）执行段内**零采样**；2 次 streak fire 载荷均自证「连续 k 个**动作采样点** u≥θ」 |
| 2 | 看门狗 181s 恰一时间样 | **测试钉收取**（真机可选） | 全程无卡死发生（无 ≥181s 无动作＋无流式窗；TER 180s 后台化正常履职）→ 真机 0 时间样＝预期行为；S2/S3 行为钉覆盖：181.0 恰一样后休眠、动作样重武装、169 批 P1 时间基转轴＋接缝轴判别钉 |
| 3 | k=3 进体、触发按 k=3 复验 | 真机＋钉 | 2 次 `streak_crossed`：`err×3（u=0.75≥θ=0.74）`、`slow×3（u=1.01≥θ=0.59）`——恰 3 个连续动作样越线即 fire；常数进体由 170 批字节判据＋S2 钉覆盖 |
| 4 | 附注直投：fire 即模型面附注＋同刻 journal 行；delivered=装配数 | 真机 | **37/37 恰等**：37 次 fire＝37 条 `mechanical_audit_update kind=rli_notice` journal 行（0co「journal 对提醒面结构性盲区」闭环＝fire 全集可由 journal 复算）；同刻装配实证：seq 466 `write_control_review`→467 `tool_result`→**468/469 `rli_notice`（+5ms/+14ms）**→470 `face_fingerprint`；侧车 `notice_delivered_total=37` 且 16 条事件历史全部 `delivered=true` |
| 5 | 回归：零动作周期零注入；五通道终态对拍 | 真机＋钉 | 37 条 notice 全部绑定域事件（25）或 streak fire（2）或覆盖缺口评估（10），**零周期性/网格性注入**；五通道全程在算（adapt_trace θ/λ̂/ρ 连续更新 320 样）；逐位一致对拍属 S2 差分重放钉（同事件流），本轮活体无异常注入面 |
| 6 | D6 注解格式：参数含义＋趋势、无成因，≤240B | 真机 | 37 行全部带形制：`T̂(单轮工具周期会话自适应估计，秒)：8.00`、趋势 `↑/↓`、`〔…仅读数非阻断〕`尾注、coverage_gap 带 `g=未访域占比；r=驻留÷中位` 释义；字节宽 **min/med/max＝186/213/228**，全部 ≤240 ✓ |
| 7 | D7 二合一：spike 进入即提醒＋回归不提醒 | 真机 | `spike_entries=13` → `domain_spike_entry` notice **13/13（100%）**；回归端（`spike_returns=14`）**零专用提醒**（回程只在 `进N回N` 计数与稳定确认行体现）；域机器逻辑不变（normal↔low_progress 为主、pressure/stuck 未涉）；对照首跑「稳定确认 0 次、spike 全被滤＝提醒面在现实任务事实死亡」（167 批 D7 立项实证）——**提醒面复活** |

fire 构成：`domain_spike_entry` 13＋`migration_confirmed` 12＋`coverage_gap` 10＋`streak_crossed` 2＝**37**（8 个 checkpoint 均有分布，run-0 至 run-7 无空洞）。

## §4 0cn S4 核收

- **`kind=budget`＝0、`预算`字串＝0**：9 卷 journal（8 RUN＋1 ARC）覆盖式扫描全零；对照首跑 136 条（每工具批一刷新）——预算注入撤除真机生效。
- **轮次感承载面接管**：`lif_domain` 注入行 98 条（首跑 109，同量级）＋`rli_notice` 域/节律行 37＋`context_scale` 8——粗粒度时间元认知由 temporal/域行族维持，与 0cn S1 定稿「不设替代物、接受时间感回归」一致（模型轮 185 正常推进、轨迹形态正常，无轮次感丧失的行为信号）。
- **`墙钟约` 保留面**：本券属会话末机械审查报告块 face，bench 形态（容器内长会话、无正式收尾审查）两轮均不渲染——首跑快照 grep 同样为 0，非本轮回归；保留性由 170 批 S3 字节判据（件内 `墙钟约` 1→1）覆盖。

## §5 观察项（不占判据，留档）

1. **spike 密度**：13 次/≈3,200s ≈ 1/4.1min（首跑 13 次/5,166s ≈ 1/6.6min）——更密但仍稀疏，且驻留极短（多为 1–3 轮即回，`进N回N` 同步增长），**无突发聚集**——设计 §60「若 S4 读数显示突发聚集再以数据议最小间隔」条款**不触发**，不设阻尼常数的定稿维持。
2. **RLI 触发质量**：2 次 streak fire 均有真实 u/θ 读数（0.75≥0.74、1.01≥0.59）且伴随真实 err/slow 动作序列；对照首跑 3 次队列 StreakCrossed 全部 delivered=false（网格补点凑满、对质量无判别力）——D1+D3 的改造意图（动作样自带特征、k=3）真机成立。
3. **fire 与域事件伴生**：streak fire（r91）与 spike 进入（r91）同刻成对出现（seq 468/469 同批直投）——异常累积与域迁移的关联形态首次有真机样本。
4. **checkpoint 3 分差**：二跑 5/7 vs 首跑 7/7（唯 difference 档）；k=1 单题不追因，仅如实记。
5. **模型非确定性边界**：两轮轨迹同构但不逐位复现（165 批更正先例）——本档全部判据均为行为面机械核收（journal/侧车/字节/行宽），未做跨轮性能对比结论。

## §6 台账

- 本档：`docs/audits/171_0CP_S4_0CN_S4_RECLI_SECOND_RUN_2026-10-03.md`。
- TODO：`P1-0cp` S4 勾选＋`P1-0cn` S4 勾选；头部计数行指针改写（计数不变 59）。
- BACKLOG：`0cp`/`0cn` 节 S4 状态行＋计数行指针。
- BACKLOG 第二卷：§1.124 本批流水。
- 索引：头行 v4.141 → **v4.142**；`AUTH-RLI-ACTION-SAMPLING-NOTICE-PUSH` 条目 S4 状态更新。
- 机械门禁：落账后复跑 `check_repository.py`。
- 证据工件（仓外）：跑批日志 `D:/tb-eval/scbench/recli_run6_s4.log`；输出树 `outputs/deepseek-v4-flash/orz_just-solve_none_20261003T0514/`；收取脚本与原始读数 `D:/tb-eval/0cp_s4/`。

## §7 关键词

171 批、0cp S4、0cn S4、recli 二跑、SlopCodeBench、真机核证、七判据、动作采样、看门狗测试钉口径、
k=3、附注直投、delivered 恰等、0co 盲区闭环、D6 行宽、D7 spike 13/13、提醒面复活、预算注入归零、
墙钟报告块面、侵蚀曲线复现、同题同源、计数 59 不变、闭合待裁决。
