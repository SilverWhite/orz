# 222 批：SCB 询问信上游先例核出、② 口径更正、附件 3 改写与重打包（2026-10-07）

> **用户令**：「我手动来发吧，正式发送之前我再给你复制粘贴内容，让你进行审查」＋「请给我一个
> 目标仓库链接，确保不会找错」＋「请修改我们新建中的口径吧，就按照你建议的改法即可」。
> **性质**：**上游先例核出＋信件口径更正＋附件 3 改写＋重打包批**——零源码、零跑批；
> **计数不变 54**；未提交、未推送；**信仍未发送**（改为用户手动发）。
> **结论先行**：发帖前例行回查目标仓库，核出**上游 issue #27 已记录本信准备报的 4 题中的 2 题**
> （env_manager ck3 同读数 184/187；test_translator 工具链未钉版本、已由 PR30 修复），并核出
> **issue #33 已问过同类进榜问题**——据此**撤回**「4 个 KNOWN_ISSUES 之外的新发现」定性，改为
> 「逐档证据＋两条请复核（file_backup／mvvault）」并互引 #33。

---

## §1 上游先例核出（本批，只读）

| 项 | 上游状态 | 对本信的影响 |
|---|---|---|
| **env_manager** ck3 | **issue #27 已记录**，读数同（fails three regression tests, **184/187**） | 由「新发现」降为**重复确认** |
| **test_translator** | **#27 已记录**（`npx ts-node` 未钉版本；八个 TypeScript 档全败），维护者回复**已由 PR30 钉 `typescript@5.3` 修复** | 由「新发现」降为**重复确认**＋一条新数据：**我方 pin `38d627e` 仍复现**（修复或未进该 pin） |
| dynamic_buffer ck2–4 | #27 已记录（48/50、102/104、122/172） | 本信原即列为 KNOWN_ISSUES，仅作对照 |
| **file_backup** | 未被列为缺陷；#27 审计将其**排除**为平台假象（APFS glob 大小写） | **请复核项①**：本 pin 观察 ck2／ck3 **Core 0.0**，与该排除结论不一致 |
| **mvvault** | **未见任何既有记录** | **请复核项②**：四题中唯一可能真属增量者 |
| 进榜途径 | **issue #33**「Custom Agent Evaluation and Leaderboard Submission」已问、**无人回复** | 诉求①**互引 #33**，不作新帖独占 |

**pin 差异如实记**：#27 的作者用 runner `06b5c06`／problems `ef6a9dd`，我方为 problems `38d627e`
——两方账目出现分歧不必然意味任何一方有错。

## §2 信件更正（CN 为权威，EN 同步重译）

1. **主题行改口**：`… + 4 reference-solution defects` → `… + 2 oracle-validation re-checks`。
2. **① 增互引**：CN／EN 各加一句，说明 issue #33 已问过同类问题，若在该帖回复同样有帮助。
3. **② 重写为三段**（撤回旧定性）：
   - 事实段＝官方 `--validate-with-oracle` 在 6 题 rc=4；四题逐档读数与 agent 同档反差明显；
   - **更正段**＝回查 issue 区后承认 env_manager 与 test_translator 已被 #27 记录、后者已由 PR30
     修复，属重复确认；并给出一条新数据（本 pin 仍复现 test_translator 失败）；
   - **收窄段**＝只请复核两点：file_backup（与 #27 排除结论不一致）与 mvvault（无既有记录）。
4. **附件清单第 3 条**改口径：由「4 缺陷明细」改为「oracle 验证失败逐档证据（含上游 #27 已记录项
   的更正说明）……请重点看 file_backup 与 mvvault 两条」。

## §3 附件 3 改写（`docs/en/SCB_REFERENCE_SOLUTION_DEFECTS_2026-10-07.md`）

- **标题改口**：`reference-solution defects beyond KNOWN_ISSUES (4 problems)` →
  `oracle-validation failures at pin 38d627e … (2 items already tracked upstream)`。
- **新增 §0 上游上下文**：逐项表（§1 同源）＋「Revised ask」＝只请复核 file_backup／mvvault；
  明示另外两节保留原因（本 pin 工具产物＋agent 对照仍可能有用）。
- **逐节 Upstream 注**：env_manager／test_translator 标为重复确认；file_backup 标注与 #27 排除
  结论冲突；mvvault 标注未见记录。
- **§5 收窄**：由「跑参考解看哪些测试失败」改为针对两条复核请求＋一条 PR30 是否进入 `38d627e`
  的询问。

## §4 重打包（旧 sha 作废）

- 因附件 3 变更重打包：`slopcodebench-orz-attachments-2026-10-07.zip`＝52 文件／**199,335 B**／
  sha256 **`e77151a90b38293835e338200621ea3a537e81f642bb32e648cddf2bda34cdb0`**
  （前版 198,029 B／`ca5a52e6…`）。
- 信件「打包件」行与发送前核对清单同步更新为新 sha。

## §5 粘贴件与目标仓库（防发错）

- **标题改定（同日两轮）**：第一轮＝用户令改为 `… (deepseek-4.1-flash + orz agent …)`（163 字符；
  去掉 `[SlopCodeBench]` 前缀——帖子本就在该仓、前缀冗余）；**第二轮（经主会话提示命名口径后用户
  再改）＝定稿**
  `External harness-native run (DeepSeek-V4.1-Flash / deepseek-flash + orz agent, k=1, 36 problems)
  — data contribution + leaderboard inclusion question + 2 oracle-validation re-checks`
  （**181 字符**；模型名用官方连字符写法、接口名取现行官方名，二者以 `/` 并列）。
  **命名留痕**：本次跑批实际调用的是 legacy 别名 `deepseek-v4-flash`，标题采用**现行**官方接口名
  `deepseek-flash`（208 批核证：同底模、同价位）；正文两处已写明 legacy 调用事实，标题不重复。
- **粘贴件**（从定稿自动抽取，避免与草稿漂移）：
  `0cr_official/issue_2026-10-07_title.txt`（标题）＋`issue_2026-10-07_body.md`（正文；EN 在前、
  CN 在后；7,288 字符；含 #27／#33，**无「4 缺陷」旧表述**）。
- **发帖口径改定（同日第三轮，用户裁决）＝只发英文**：用户令「我取消了中文版本……毕竟是英文
  开发者的仓库，中文版本我们留档就好了」——**帖子仅英文**；中文版**留在仓内作权威档**（
  `docs/en/SCB_OFFICIAL_INQUIRY_DRAFT_2026-10-07.md` 内 CN 段不动），粘贴件相应改为**仅英文**
  （`issue_2026-10-07_body.md` 重出为 4,971 字符）。**影响面＝零**：英文段本身自足，不含任何对
  中文段的指代。
- **粘贴件格式勘误（同日）**：投稿前复核用户回贴的正文，发现 `Related: I see issue #33 …` 与其后
  段落之间**缺空行**（Markdown 会并成同一段）。已把草稿与粘贴件统一为「前后各恰一空行」；用户
  尚未发帖，按刷新后的文件重贴即可。
- **附件上传核验（未果，如实记）**：用户回贴末附 GitHub user-attachments 链接
  `…/files/33158721/slopcodebench-orz-attachments-2026-10-07.zip`，主会话尝试下载核算 sha256
  **返回 404**——user-attachments 在帖子发布前不对未登录/他人可见，**故远端 hash 无法核**。
  本地工件现为**重打包版**（199,335 B／`e77151a9…`）；因文件名与旧版相同，已提示用户**删除后
  重新拖入**以排除拖到旧包（拖到旧包会导致包内附件 3 仍写「4 个 KNOWN_ISSUES 之外的新发现」，
  与正文更正自相矛盾）。
- **目标仓库唯一确认**：`SprocketLab/slop-code-bench`
  （https://github.com/SprocketLab/slop-code-bench ；237★；描述 "SlopCodeBench: Measuring Code
  Erosion Under Iterative Specification Refinement"；公开、issues 已开）。
- **近似名陷阱**（勿发）：`dburkhardt/slop-code-bench`、`Dongximing/slop-code-bench`（均 0★ 空仓）。

## §6 台账

- BACKLOG：计数行（**54 不变**）＋本批指针；第二卷 §1.168。
- TODO：头部计数行（54 不变）本批指针。
- 索引：头行 v4.194 → **v4.195**。

## §7 边界

1. **零源码、零跑批**；本批全部为上游只读回查＋文档改写＋重打包。
2. **信仍未发送**，且发送方式由「代理代发」改为**用户手动发**；发送动作与 issue 链接随后续批次落账。
3. 附件 3 保留两组「重复确认」段落并显式标注——**不删**，理由：第二 pin 得到同一读数本身是证据，
   且 agent 侧对照对维护者可能仍有用。
4. 未提交、未推送（本批尚未提交）。

## §8 关键词

222 批、上游先例核出、issue #27、issue #33、PR30、typescript@5.3、口径更正、撤回「新发现」、
file_backup／mvvault 两条复核、附件 3 改写、重打包 199,335 B、sha256 e77151a9、粘贴件、
目标仓库 SprocketLab/slop-code-bench、近似名陷阱、计数 54 不变、索引 v4.195、信仍未发送。
