# SlopCodeBench 官方询问信（草稿，未发送；213 批落稿）

> **状态**：DRAFT——用户令「请先落一份草稿吧，暂时不发」。
> **形态**：双语（发送时英文在前、中文在后）；主通道＝GitHub issue（`SprocketLab/slop-code-bench`），
> 副通道＝Discord server 贴 issue 链接（`discord.gg/BrC4BA9sVj`；不直接 DM）。
> **事实锚**：0cr 官方轮＝2026-10-05 01:11（本地）D1 cfgpipe 起跑 → 10-07 凌晨 D6 收尾，
> 六日（D1–D6）压缩两日历日，逐题墙钟合计 ≈36.5h（批口径 ≈41h），账单 157.49 RMB；
> 载体 orz v0.8.14 单一快照（`b5cb57ca`）；模型 DeepSeek-V4.1-Flash（legacy 接口名
> `deepseek-v4-flash`）。

---

## English (send first)

**Subject: [SlopCodeBench] External harness-native run (orz agent, k=1, 36 problems) — data contribution + leaderboard inclusion question + 4 reference-solution defects**

Hello!

I'm a university student maintaining **orz**, a personal open-source AI coding agent harness (https://github.com/SilverWhite/orz). I recently integrated orz into your official harness via the documented Agent plugin mechanism (Agent ABC + register_agent) and completed a k=1 run — matching the paper's single-run methodology — over all 36 problems / 196 checkpoints, using the official default parameters (seed 42, pass_policy=any, one_shot off) on harness commit `31ceea3` / problems `38d627e`. Model: DeepSeek-V4.1-Flash (served under the legacy `deepseek-v4-flash` alias). The agent ran on a single frozen snapshot of orz **v0.8.14** throughout; total wall clock ≈41 hours (157.49 RMB), with full trajectories and per-checkpoint official scoring records retained.

Results (official metrics, produced by the harness itself):

- **60/196 checkpoints fully passed under the isolated metric (30.6%)**
- 44/196 fully passed under strict (22.4%)
- mean verbosity 0.289, mean erosion 0.613 (scb-check 0.1.3); increase rates across step transitions: 58.75% / 65.0%

Three things I'd like to ask:

① Through my own oversight I only converted the tasks (scb_to_harbor.py) without running a Harbor Run, and as a student my budget doesn't currently allow another full round. **Is there any path onto the official leaderboard other than a Harbor Run — e.g., accepting external harness-native runs, or third-party agent rows?** As a personal open-source project, external review and attention would mean a lot to me.

② During the oracle-verification step of the Harbor conversion, I identified — with AI-assisted review (GLM 5.3 Flash) — **4 reference-solution defects not covered by KNOWN_ISSUES**: env_manager, file_backup, mvvault, test_translator. Notably, the agent's own runs passed these checkpoints (e.g., env_manager 5/5), further indicating the reference solutions rather than the tests are at fault. For transparency: I'm not from a CS background and my manual review works at the natural-language level, so I've attached the mechanical evidence (oracle rc=4 exit codes and per-test comparisons) for your maintainers to verify directly.

③ Any feedback or suggestions on orz itself would be greatly appreciated.

If leaderboard inclusion isn't possible, no problem at all — please consider this a data contribution to SlopCodeBench. Everything is in the repo and the attachments. Thank you for reading!

SilverWhite

## 中文（发送时置后）

您好！

我是一名大学生，维护个人开源 AI coding agent harness「orz」（https://github.com/SilverWhite/orz）。近期我通过您方官方 Agent 插件机制（Agent ABC＋register_agent）将 orz 接入官方 harness（commit `31ceea3`，官方默认参数 seed 42／pass_policy=any／one_shot off），以 k=1（与论文 single-run 口径一致）完成了全目 36 题／196 checkpoint 的评测：模型为 DeepSeek-V4.1-Flash（经 legacy 接口名 `deepseek-v4-flash` 服务），载体单一快照 orz **v0.8.14** 全程不换，总耗时约 41 小时（157.49 RMB），完整轨迹与逐档官方判分记录全部留存。

成绩（官方 harness 自产的官方指标）：

- **isolated 满通过 60/196 checkpoints（30.6%）**
- strict 满通过 44/196（22.4%）
- verbosity 均值 0.289、erosion 均值 0.613（scb-check 0.1.3；逐档转移增长率 58.75%／65.0%）

三件事想向您方请教：

① 由于我的疏忽，事先只做了任务转换（scb_to_harbor.py）而没有跑 Harbor Run，且个人预算有限暂时无力承担新一轮 Harbor 跑批。**除 Harbor Run 之外，是否有其他途径进入官方榜单（例如收录外部 harness-native run，或第三方 agent 行）？** 作为一个个人开源项目，外部审查与关注对我非常重要。

② 在 Harbor 转换的 oracle 验证环节，经 AI 辅助复核（GLM 5.3 Flash）发现 **4 个 KNOWN_ISSUES 之外的参考解缺陷**：env_manager／file_backup／mvvault／test_translator。旁证是这些题的 agent 跑批反而全绿（如 env_manager 5/5），进一步指向参考解本身而非测试。需要说明：我本人非 CS 专业，人工复核以自然语言层面为主，所以缺陷的机械证据（oracle 验证 rc=4 退出码与逐测试对照）已随附件给出，方便您方直接转给题目维护者复核。

③ 若您方愿意对 orz 本身提出任何建议，我将不胜感激。

若无法进入榜单也完全没有关系，这份资料就作为 SlopCodeBench 的数据补充。全部内容见仓库与附件。感谢您的阅读！

SilverWhite

## 附件清单（发送前定稿）

1. 全轮报告（英文精简版待出；暂可附中文版 `SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md`）；
2. `multidim_aggregate.json`＋合并逐档记录（36×196 行 checkpoint_results 汇总件，待生成）；
3. 4 缺陷明细（题名＋oracle rc=4 证据＋逐测试对照＋agent 对照成绩）。

## 发送前核对清单

- [ ] 版本号／数据与账面最终对表（v0.8.14、30.6%／22.4%／0.289／0.613、157.49 RMB）；
- [ ] 附件包生成（上列三件）；
- [ ] issue 标签/版区选择＋Discord 频道选择；
- [ ] 发送后：issue 链接与发送事实落账。
