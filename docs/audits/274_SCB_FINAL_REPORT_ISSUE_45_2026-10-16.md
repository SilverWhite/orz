# 274 批：SCBench 收官报告发帖上游——`SprocketLab/slop-code-bench#45`（2026-10-16）

> **用户令**：「我考虑可以去sc bench仓库提交最终版本的issue了，我们之前已经提交过#40和#44」
> →（中文审阅版过目、手改正文后）「还是补上事实句+短询问吧」。
> **性质**：**外发交付批**——新增仓内草稿档一件＋上游 issue 一件（外站）；零源码、零跑批、零重跑；
> **计数不变 61**。
> **编号注记**：273＝SCB V4 报告英文版落盘（前批）；本批 274。
> **结论先行**：收官报告以 **issue #45** 发出（https://github.com/SprocketLab/slop-code-bench/issues/45 ；
> OPEN、无标签、作者 `SilverWhite`、创建 2026-10-16T07:06:05Z）；正文 6,653 字符＝草稿 §English 逐字稿，
> 发送前逐字节比对 `EXACT MATCH`，发送后回读核验一致。

---

## §1 发送前只读核（2026-10-16 实测，决定本件写法）

| 项 | 实测值 | 对写法的影响 |
|---|---|---|
| #40 | OPEN；评论 2 条**均为我方**（`6040701347` 会话形态更正、`6041122591` v0.8.15 说明）；**无维护者回复** | 不重复其正文，只承接口径 |
| #44 | OPEN；**0 评论** | 以「后续」形式简述封堵结果，不重开论述 |
| scbench.ai 榜单 | **24 runs**（页面显示 16 条）；harness 过滤只有 Codex／Claude Code／Cursor CLI／Kimi CLI／OpenCode；**无 orz、无 DeepSeek** | §4 的事实句来源 |
| #42／#43 | 维护者已关闭；#43 回帖「these runs are prohibitively expensive. we are trying to somewhat refresh the leaderboard」 | 询问措辞收敛为「只想确知」，不催 |
| #33（Custom Agent Evaluation and Leaderboard Submission） | 仍 OPEN、无维护者回复 | 不重复挂在该帖下，改以 #40 承接 |

## §2 交付物与形态裁决

- **草稿档**：[`docs/en/SCB_FINAL_ISSUE_DRAFT_2026-10-16.md`](../en/SCB_FINAL_ISSUE_DRAFT_2026-10-16.md)——
  中文审阅版（用户要求「给我中文版本供我进行审阅」）＋英文逐字稿（**实际发送内容**）＋附件清单＋
  发送前核对清单＋发送回读核验＋手改稿并入记录。
- **形态**＝**新开 issue**（用户未采纳 #40 追评；#44 亦未单独追评）；**仅发英文**（英文开发者仓库，沿
  213／223 批口径，中文留档）；**附件**＝仓内／Harbor 链接（无需打包，#40 的 52 文件包未复用）。
- **正文六节**：① 四轮弧线表（含 orz 存档机制披露与「不作专门清档版本」的决定）② #44 封堵结果与逐档
  断言 ③ #40 §② 两条仍悬复核请求（file_backup ck2–ck3 参考解 Core 0.0；mvvault ck5 差一测试）＋
  test_translator 在 `38d627e` 仍复现 ④ 入榜路径实验（R2／R4 均公开 Harbor run）＋事实句与短询问
  ⑤ 一条**明示不作结论**的论文口径观察 ⑥ 两条小发现（dag_execution 未文档化 `sleep()`；
  test_translator ck7–ck8 结构缺失）。

## §3 用户手改并入（中文版与英文逐字稿同步）

1. §1 新增两段＝orz 存档机制披露（服务模型、全台账 `.gz` 打包；R4 存在模型无提示、无外部干涉下自行
   找到旧轮存档并解包使用）＋不作专门清档版本的决定（「不喜好对测试进行专门处理」，用正常版本跑完）。
2. §4 依用户裁决**补回事实句＋短询问**（逐字采用用户给定措辞）。
3. §5 末句增补「LLM 进步与变化极快／很可能这一坍缩本就不存在于 DeepSeek V4.1 Flash 身上」。
4. 末段增补「Harness 部分需要被额外测评和考虑」。
5. 材料位置三条改为可点击链接；用字修正两处（`不存在与`→`不存在于`、`真心实意的觉得`→`真心实意地觉得`）；
   表头折行还原（内容未改）。

## §4 发送与回读核验

- **命令**：`gh issue create -R SprocketLab/slop-code-bench --title "…" --body-file .tmp-273-issue-body.md`
  （`gh` 2.96.0，账号 `SilverWhite`，`repo` scope）。
- **机械预检**：正文从草稿 §English 段抽出后与临时正文件逐字节比对 → `EXACT MATCH`（6,652 字符；
  GitHub 记账 6,653 含尾换行）。
- **回读（API 实测）**：`number=45`、`state=open`、`user=SilverWhite`、`created_at=2026-10-16T07:06:05Z`、
  `labels=0`、`body_len=6653`。
- **清理**：临时正文件 `.tmp-273-issue-body.md` 已删（`tmp*` 前缀本就在门禁非仓区前缀内，删前未污染读数）。

## §5 边界

1. **零源码、零跑批、零重跑**；未触碰 `orz/` 子树；未改动任何既有报告与台账历史行（本条为新增批）。
2. **正文所有数字取自** [`V4 四方报告`](../SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_2026-10-16.md) 与
   [`参考解缺陷档`](../en/SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md)，**不新增推断**；论文口径观察已
   明示「不作结论」并给出三重不可比与「模型换代」两种保留。
3. **上游无承诺**：本件为事实报告＋复核请求＋路径询问；若长期无回复，可选 #40／#45 相关回贴或 Discord
   人工通道（随裁决，不占计数）。
4. 未提交、未推送（提交批另行放行）。

## §6 关键词

274 批、SCBench 收官报告、issue #45、SprocketLab/slop-code-bench、SCB_FINAL_ISSUE_DRAFT_2026-10-16、
中文审阅版、用户手改稿并入、§4 事实句＋短询问裁决、EXACT MATCH 6652、gh issue create、回读核验
2026-10-16T07:06:05Z、R4 净室 36/36、file_backup／mvvault 复核请求、入榜路径实验、仅发英文、
计数 61 不变、索引 v4.247、第二卷 §1.221。
