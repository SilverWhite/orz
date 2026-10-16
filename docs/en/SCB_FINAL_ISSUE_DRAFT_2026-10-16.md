# SlopCodeBench 收官报告 issue 草稿（**待用户批准后发送**；273 批线上产出）

> **状态**：**已发送**——2026-10-16T07:06:05Z（本地 15:06）由用户放行后经 `gh` CLI 发至
> `SprocketLab/slop-code-bench`，**issue #45**：https://github.com/SprocketLab/slop-code-bench/issues/45
> （OPEN、**无标签**、作者 SilverWhite、正文 6,653 字符＝本档 §English **逐字稿**，发送前机械复核
> `EXACT MATCH`）。形态＝**新开 issue**（非 #40 追评），引用 #40 与 #44 而不重复其正文。
> **触发**：用户 2026-10-16「我考虑可以去sc bench仓库提交最终版本的issue了，我们之前已经提交过#40和#44」。
> **发送前只读核（2026-10-16 实测）**：#40 OPEN／**仅我方两条评论、无维护者回复**；#44 OPEN／**0 评论**；
> 榜单 scbench.ai 当日 **24 runs／16 条可见**（harness 只有 Codex／Claude Code／Cursor CLI／Kimi CLI／
> OpenCode），**无 orz、无 DeepSeek 条目**；#42／#43 由维护者关闭，回帖称「runs are prohibitively
> expensive，we are trying to somewhat refresh the leaderboard」。
> **口径**：本件为**纯事实报告＋两条仍未获回复的复核请求＋入榜路径询问**；所有数字取自
> [`SCB_V4 四方报告`](../SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_2026-10-16.md) 与
> [`参考解缺陷档`](SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md)，不新增推断。
> **发送通道**：`gh` CLI 已就绪（账号 `SilverWhite`，`repo` scope）；发送动作与回执待用户放行后落账。
> **审阅版**：中文逐节对照见 §中文（**2026-10-16 用户手改稿已并入**）；**实际发送内容＝§English 逐字稿**。
> **§4 裁决**（2026-10-16 用户）：**补回事实句＋短询问**，措辞逐字采用用户给定版本（见 §4 末句）。

---

## 中文（审阅版；发送仍为下方 English 逐字稿）

**标题（中文对照）**：收官报告：官方形态下的四轮（末轮 36/36 净室）——外加 #44 的后续与两条仍未获复核的 oracle 请求

您好——这封是 #40 所述工作的收尾，同时承接 #44。单独开帖是因为 #40 已经很长；如果您更希望在原帖继续，也完全可以。我方不再新开轮次。

**1. 轮次弧线简述。** 四轮全部 k=1，harness `31ceea3`、题集 `38d627e`，全程同一模型（`deepseek-flash`，即 V4.1-Flash 后端）、同一价位。下表「端到端全解」＝某题每一档、全部测试都通过（严格口径，而非「全档 core 满」）。

| 轮次 | 会话形态 | 题数 | core 全绿档 | strict（全测试） | isolated | 端到端全解 |
|---|---|---|---|---|---|---|
| R1（10-05/06） | 连续会话——即 #40 所述那一轮 | 36/36 | 134/196（68.4%） | 44/196（22.4%） | 60/196（30.6%） | 1/36 |
| R2（10-08/10） | 每档重置，但我方 agent 自身的会话卷仍可读（适配器侧，188/189 档） | 36/36 | 131/196（66.8%） | 61/196（31.1%） | 82/196（41.8%） | 5/36 |
| R3（10-11/12） | 同上；发现 #44 的 `/tmp` 通道后停止 | 14/36 | 50/68（73.5%） | 29/68（42.6%） | 37/68（54.4%） | 3/14 |
| R4（10-12/16） | 净室——每档全新进程＋全新会话，无跨档指令通道 | 36/36 | 144/196（73.5%） | 66/196（33.7%） | 88/196（44.9%） | 4/36 |

四轮之中只有 R4 真正对齐参考形态，因此只有它适合作为「官方形态下的一次测量」来读。它仍不可与榜单对拍：harness 不同、模型处于 flash 价位、每个设定只跑了一次。

必须说明的一点是，orz 本身的存档机制更多的是服务于模型，保存了全部台账，以 .gz 形式打包存储，而在 R4 轮中存在模型在完全无任何提示与外部干涉的情况下自己找到了旧轮存档并解包使用的情况。

考虑到本测试的重点是测试模型在连续轮中的具体表现，并且我个人不喜好对测试进行专门处理，我最终并没有决定专门为 SlopCode Bench 制作专门的会清空存档的 orz 版本，而使用正常版本跑完了本测试。

**2. #44 的后续。** 我方适配器遗留在 `/tmp/orz-instruction-<N>.md` 的文件，被后续档在 14/14 题中读取到。我们已做双侧封堵——driver 读后即删、适配器上传前清旧、runner 逐档断言「档起点该路径下只有本档文件、档终点为空」。R4 的 36 题全部通过该断言，也正是封堵之后 R4 才成立。我们现在跑的检查如下，或许对其他集成者有用：*档起点时，交接路径下只有本档文件；档终点时，一个也没有。*

**3. #40 §② 中仍有两条复核请求未获回复。** 自那帖以来没有变化，R4 又在同一 pin 上添了一条 agent 侧读数：

- **`file_backup`**——参考解在 ck2 与 ck3 的 Core 通过率为 **0.0**（isolated 0.56／0.22），而 agent 运行在这两档通过 Core 组，并在 R4 中把该题 4/4 档全解。#27 曾把此题作为平台假象排除，因此两处读数相互矛盾。
- **`mvvault`**——ck5 参考解 strict 0.9946／core 0.875／isolated 0.9667（差一个测试），agent isolated 1.0000；R4 在全部 6 档得 1.000。

该节另两条（`env_manager` ck3、`test_translator`）已由 #27／PR30 记录，不属新发现。`test_translator` 有一条或仍有用：pin `38d627e` 仍复现该失败，说明 PR30 的 `typescript@5.3` 钉版似乎不在该 pin 上。

**4. 入榜问题，现在有了一次直接实验。** #40 问过 (a)：公开上传一轮 Harbor run 是否会形成可见条目。此后我们照做，而且做了两次，走官方转换 `scb_to_harbor.py`——R2 与 R4 均在 Harbor 上运行并以 `--public` 公开上传（36 题各一 job，另有重试记录；例如 job `a64d620b-ec9a-40ef-afa9-3c90858cd4f1`、任务集 `silverwhite/slopcodebench-friction`）。

截至今日榜单 24 runs、harness 只有 Codex／Claude Code／Cursor CLI／Kimi CLI／OpenCode，我方那轮没有条目出现；若系您侧人工维护则完全没问题，只想确知，若第三方 harness 需要某个我未找到的步骤请指个方向。

**5. 一条我明确不作结论的观察。** 净室轮的五相位曲线只是缓降，Core/Iso 比值全程接近 1.0、没有扩大；而论文的重置队列从 64.6% 掉到 35.5%、比值由 2.5× 扩到 5.4×。我们是不同的 harness、便宜得多的模型、每个设定单跑，所以这不是关于您方结果的证据。我提它只是因为：如果队列级坍缩是重置形态本身的性质，那么在我们这个层级上也应看到一些，而我们没有看到。但毕竟目前 LLM 进步与变化都极快，我方使用的还是新模型，很可能这一坍缩问题本身单纯就不存在于 DeepSeek V4.1 Flash 身上。

**6. 两条较小的东西，或许有用。**

- `dag_execution`：agent 自测 21 项全绿，而官方 Core 组全红——因为该档期待一个题面未提及的内建 `sleep()` 式任务。跨轮复现。
- `test_translator`：参考 oracle 在 ck7 没有产出 `strict_pass_rate`、在 ck8 完全没有 step result——与工具链问题相互独立。

**我不主张的事。** 我们未测人仓对照（RQ3）与 prompt 对照（RQ4），故不就此作任何主张。我不是这些题目及其测试的作者；以上全部是工具原始输出或机械复算的逐档读数。

**材料位置。** 轮次报告英文版：[https://github.com/SilverWhite/orz/blob/main/docs/en/SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_EN.md](https://github.com/SilverWhite/orz/blob/main/docs/en/SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_EN.md) ；数据集：[https://hub.harborframework.com/datasets/silverwhite/slopcodebench-friction](https://hub.harborframework.com/datasets/silverwhite/slopcodebench-friction) ；净室轮的一条公开 job：[https://hub.harborframework.com/jobs/a64d620b-ec9a-40ef-afa9-3c90858cd4f1](https://hub.harborframework.com/jobs/a64d620b-ec9a-40ef-afa9-3c90858cd4f1)

感谢这个 benchmark——据我所知它是唯一衡量「代码在反复变更下会变成什么样」的评测，而且我们真心实意地觉得 Harness 部分需要被额外测评和考虑，所以我们才在它上面花了四轮。

SilverWhite

---

## English (ready to file)

**Title**: Closing report: our four rounds on the official setup (final round 36/36 clean-room) — plus the #44 follow-up and two oracle re-checks still open

Hi — this closes out the work described in #40 and follows up on #44. I am filing it separately because #40 grew long; happy to move the discussion back there if you prefer. We have no further rounds planned.

**1. The arc, briefly.** Four rounds, all k=1, harness `31ceea3`, problems `38d627e`, one model throughout (`deepseek-flash`, the V4.1-Flash server), one price tier. "End-to-end full solve" below means every checkpoint of a problem passing every test — the strict sense, not "all checkpoints core-green".

| round | session form | problems | core-green checkpoints | strict (all tests) | isolated | end-to-end full solve |
|---|---|---|---|---|---|---|
| R1 (10-05/06) | continuous session — the run in #40 | 36/36 | 134/196 (68.4%) | 44/196 (22.4%) | 60/196 (30.6%) | 1/36 |
| R2 (10-08/10) | reset per checkpoint, but our own agent session volume stayed readable (adapter-side, 188/189 checkpoints) | 36/36 | 131/196 (66.8%) | 61/196 (31.1%) | 82/196 (41.8%) | 5/36 |
| R3 (10-11/12) | same, stopped once we found the `/tmp` channel in #44 | 14/36 | 50/68 (73.5%) | 29/68 (42.6%) | 37/68 (54.4%) | 3/14 |
| R4 (10-12/16) | clean room — fresh process + fresh session per checkpoint, no cross-checkpoint instruction channel | 36/36 | 144/196 (73.5%) | 66/196 (33.7%) | 88/196 (44.9%) | 4/36 |

R4 is the only one of the four that actually matches the reference setup, so it is the only one I would ask you to read as a measurement of the official form. It is still not comparable to the leaderboard: different harness, flash-tier model, single run per setting.

One thing I should disclose: orz's archive mechanism mostly exists to serve the model — it keeps the full ledger and stores it packed as `.gz` archives. In R4 there was a case where the model found a previous round's archive and unpacked it on its own, with no hint from us and no outside interference.

Since what this benchmark measures is how the model behaves across checkpoints, and since I personally dislike special-casing a benchmark, I decided **not** to build a special orz build that clears archives for SlopCodeBench, and ran the whole thing on the normal build.

**2. #44 follow-up.** The `/tmp/orz-instruction-<N>.md` files our adapter left behind were read by later checkpoints in 14/14 problems. We sealed it on both sides — the driver deletes each file after reading it, the adapter clears stale files before uploading, and the runner asserts per checkpoint that only the current checkpoint's file exists and that the directory is empty at the end. All 36 problems pass that assertion in R4, and sealing the channel is what made R4 possible. The check we now run, in case it is useful to other integrators: *at checkpoint start the only file under the handoff path is the current one; at checkpoint end, none.*

**3. Two oracle re-checks from #40 §② are still open.** Nothing has changed since that post, and R4 adds one more agent-side reading at the same pin:

- **`file_backup`** — reference solution Core pass rate **0.0 at ck2 and ck3** (isolated 0.56 / 0.22), while the agent run passes the Core group at both, and closes the problem 4/4 checkpoints in R4. #27 exonerated this problem as a platform artifact, so the two readings disagree.
- **`mvvault`** — ck5 reference strict 0.9946 / core 0.875 / isolated 0.9667 (short by one test), agent isolated 1.0000; R4 scores 1.000 over all 6 checkpoints.

The other two from that section (`env_manager` ck3, `test_translator`) are already in #27 / PR30 and are not new. One datum that may still be useful for `test_translator`: pin `38d627e` still reproduces the failure, so PR30's `typescript@5.3` pin does not appear to be present at this pin.

**4. The leaderboard question, now with a direct experiment.** #40 asked (a) whether publishing a public Harbor run would produce an entry. We have since done exactly that, twice, through the official conversion (`scb_to_harbor.py`) — R2 and R4 were run on Harbor and published with `--public` (all 36 problems, one job each, plus retry records; e.g. job `a64d620b-ec9a-40ef-afa9-3c90858cd4f1`, task set `silverwhite/slopcodebench-friction`).

As of today the leaderboard shows 24 runs, with the harness column covering only Codex / Claude Code / Cursor CLI / Kimi CLI / OpenCode, and no entry for our run has appeared. If that is simply because the leaderboard is maintained by hand on your side, that is completely fine — I would just like to know. If a third-party harness does need a step I have not found, a pointer would be appreciated.

**5. One observation I am explicitly not claiming as a finding.** In the clean-room round the five-phase curve declined only gently, and the Core/Iso ratio stayed near 1.0 with no widening — while the paper's reset-form cohort falls from 64.6% to 35.5% with the ratio widening from 2.5× to 5.4×. We are a different harness on a much cheaper model with one run per setting, so this is not evidence about your results. I mention it only because if the cohort collapse were a property of the reset form itself, I would have expected to see some of it at our level too, and we did not. That said, LLMs are advancing and changing very fast right now, and we are on a new model — it is entirely possible that this collapse simply does not exist for DeepSeek V4.1 Flash in the first place.

**6. Two smaller things, in case they are useful.**

- `dag_execution`: the agent's own 21 self-tests were green while the official Core group was red, because the checkpoint expects a built-in `sleep()`-style task that the statement does not mention. Reproduced across rounds.
- `test_translator`: the reference oracle produced no `strict_pass_rate` at ck7 and no step result at all at ck8 — separate from the toolchain issue.

**What I am not claiming.** We did not test the human-repository comparisons (RQ3) or the prompt ablations (RQ4), so we make no claims there. I am not the author of these problems or their tests; everything above is raw tooling output or mechanically recomputed per-checkpoint readings.

**Where everything is.** Round report, English: https://github.com/SilverWhite/orz/blob/main/docs/en/SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_EN.md · dataset: https://hub.harborframework.com/datasets/silverwhite/slopcodebench-friction · a public job from the clean-room round: https://hub.harborframework.com/jobs/a64d620b-ec9a-40ef-afa9-3c90858cd4f1

Thanks for the benchmark — it is the only one I know of that measures what happens to code under repeated change, and we genuinely think the harness side deserves to be measured and considered in its own right, which is why we spent four rounds on it.

SilverWhite

---

## 附件（随 issue 附上或改为仓内链接）

| # | 件 | 形态 |
|---|---|---|
| 1 | 收官报告英文版 | 仓内链接（上列，已 200 可达） |
| 2 | 逐档记录（r4 标准口径 + 四方聚合） | `D:/tb-eval/scbench/v4_four_way_readings.json`＋各轮 `*_checkpoint_ledger.jsonl` |
| 3 | 参考解缺陷逐档证据 | [`SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md`](SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md)（#40 附件 3 原文，未改） |

## 发送前核对清单（待用户放行）

- [x] **§4 结果句与询问段**＝**已补回**（用户 2026-10-16 给定措辞，逐字采用）；
- [x] 形态＝**新开 issue**（未采纳 #40 追评；`#44` 亦未单独追评）；
- [x] 附件形态＝**仓内链接**（无需打包；报告已公开可达，#40 的 52 文件包未复用）；
- [x] 发送：`gh issue create -R SprocketLab/slop-code-bench --title "<TITLE>" --body-file .tmp-273-issue-body.md` → **#45**（发后临时正文件已删）；
- [x] 发送后落账＝本档状态改「已发送」＋issue 号／URL／时间戳＋回读核验；索引 §6 条目 274 注记；第二卷 §1.221；TODO／BACKLOG 计数行；[`274 批档`](../audits/274_SCB_FINAL_REPORT_ISSUE_45_2026-10-16.md)。

## 发送回读核验（2026-10-16）

| 项 | 值 |
|---|---|
| issue | **#45**（https://github.com/SprocketLab/slop-code-bench/issues/45） |
| 状态／标签／作者 | OPEN／无标签／`SilverWhite` |
| 创建时刻 | 2026-10-16T07:06:05Z |
| 标题 | Closing report: our four rounds on the official setup (final round 36/36 clean-room) — plus the #44 follow-up and two oracle re-checks still open |
| 正文长度 | 6,653 字符（＝本档 §English 逐字稿，发送前逐字节比对通过） |
| 附件 | 无（正文内三条仓内／Harbor 链接；均发送前实测 200 可达） |

## 手改稿并入记录（2026-10-16）

- §1 新增两段（orz 存档机制披露＋不作专门清档版本的决定）——中文原文照录，英文逐字对应译出。
- §4 依手改稿删去「截至今日榜单仍无条目」与询问段（**已登记为待决项**，见核对清单第 1 条）。
- §5 末句增补「LLM 进步极快／可能本就不存在于 DeepSeek V4.1 Flash 身上」。
- 末段增补「Harness 部分需要被额外测评和考虑」。
- 材料位置三条改为可点击链接（去转义）。
- **用字修正两处**（手改稿笔误）：`不存在与` → `不存在于`；`真心实意的觉得` → `真心实意地觉得`。
- 表格表头行按手改稿粘贴时的折行还原为标准表头（内容未改）。
