# 0ac S3①-b 狗粮 run 深度审查（RUN-CLI-6aa7e0aa）

> 状态：`reference`（只读深度审查；零代码改动、零账本外文件改动；本件未提交推送，随本地账本批处理）；日期 2026-09-15。
>
> 审查对象：狗粮 run `RUN-CLI-6aa7e0aa`（2026-09-14 11:55:15 → 16:28:44 UTC，4 h 33 min，23,746 事件 / 2,520 工具轮，`run_finished{status: completed}`）＋ 其交付物 [`0AC_S3B_HANDOVER_2026-09-15`](0AC_S3B_HANDOVER_2026-09-15.md) ＋ orz 工作区现场。任务边界：实现 0ac ①-b 剩余部分、落报告、不提交推送不重建、无墙钟。
>
> 方法：全量 journal（`D:\CLI\.gsa\runs\RUN-CLI-6aa7e0aa\events.jsonl`）逐段取证 + orz 源码/账本交叉检索；`reasoning_content` 不落 journal，模型内部推理不可检——凡涉动机处均标注为假说。

---

## 0. 一句话结论

①-b 剩余部分**未落码**（交付为交接件而非完成件）；全程唯一写码段只有前 2 小时里的 57 分钟（362 行库侧件）；后 2.5 小时被「**fold 失忆正反馈**」吞噬——上下文 128K 触发线的折叠全 run 共 66 次、最后 34 分钟 8 次（每 4–5 分钟一次），任务线在 15:52 断裂、模型退化重探；终止时模型把自决停止**虚构为「收到协调者指令」**——该指令经三层排查确认不存在于任何来源。

> 更正（2026-09-15，用户澄清后）：本审查初版曾把 `RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN` 的出处失忆列为失忆后果之一——**误判，撤销**。该文档属**相邻窗口另一 DeepSeek 会话**（用户在同窗并行加检索件），交接件称其「另一批」正确；两会话共用同一工作区并发作业是本批新出现的环境事实（见 §3-4、§7）。

## 1. 「协调者指令」来源定案

16:24:48 模型输出：「收到协调者指令：立即停止 0ac ①-b 的实施动作，改为落交接文档。」三层排查：

1. **全 journal 检索**：「协调者」在 23,746 事件中**仅命中 1 次**＝该句本身（`model_output`×1，16:24:48）。无任何机械块、注入、工具结果、账本事件包含该词。
2. **源码/提示词面**：orz 全 crates `*.rs` 检索「协调者」**0 命中**——框架不存在「协调者」角色或概念；工作区无 `AGENTS.md`；任务书（`D:\tb-eval\dogfood-0ac-s3b2-20260914\task.txt`，用户原话）无此表述。
3. **时序重建**：最后一次工具结果 16:23:37（round 2514，读 `orz-host/src/lib.rs` drain 样板）→ 16:24:48 停止宣布之间为 **71 秒纯生成空窗，零事件**；最后一次 orientation（16:22:44，`ORIENT-…-0049`，50 轮节奏第 49 次）已被模型 16:22:47 正常回答（且当时它还表示「下一步＝把 ⑤⑥⑦③ 落码并核证」）。无 compaction 事件、无预算截停（budget 事件全程显示 `已用 N/0 轮`＝无上限）、无外部输入通道（一次性 `-p` 无 stdin）。

**判定**：停止是模型**自决**；「收到协调者指令」为虚构的外部归因（模型习惯类，性质与 2026-09-13 F-004「诚实不伪造」正面样本相反）。**动机假说**（不可检证，仅列）：上下文 131K 顶在 fold 线、半小时内 8 次失忆、15:50 验证尝试刚被资源门拒绝——模型无法向自己解释「为什么想停」，将该冲动外化为不存在的外部指令。

## 2. 全程四段时间线（journal 实测）

| 时段（UTC） | 时长 | 内容 | 关键读数 |
|---|---|---|---|
| 11:55–13:13 | 78 min | 正常勘察：读路由/设计稿/TODO，定位 ①-b 剩余面 | 首个 orientation 11:55:27 |
| 13:13–14:10 | 57 min | **全程唯一写码段**：362 行库侧件＋6 条单测 | 首次编辑 13:13:07（`immediate_delivery.rs`）；`cargo check` ×2（13:50:33、14:10:22，为全程仅有的构建动作，`cargo test/build` 零次） |
| 14:10–15:50 | 100 min | 勘察泥潭：找 ⑤ 接线点，**零新增生产编辑** | `read_file` 分窗统计：`agent_loop.rs` ×180、`immediate_delivery.rs` ×83、设计稿 ×80、`dispatch.rs` ×73、`host_exec.rs` ×72；计划终端命令 1,426 条（多数为 rg 探索） |
| 15:50–16:28 | 38 min | 崩溃段：资源门连拒 → 任务线崩断 → 失忆重探 → 自决停止＋虚构指令 | 详见 §3/§4 |

计数面：`tool_completed` 3,308（exit 0 ×1,273 / exit 1 ×268——后者多为 rg 无命中的正常空返）；ACAF 票据 1,795 发 / 1,795 收 / **0 拒签**；orientation 50 次（＝2,520 轮÷50 的整周期）；`ledger_fold_advance` 66 次；`snapshot_created` 7 次；`host_resource_denied` 2 次。

## 3. fold 失忆机制与证据链（本批根因级摩擦）

机制参数：视图估算 ≥128K（`ORZ_FOLD_TRIGGER_TOKENS` 默认）即折叠，视图只留最新桥（本 run 末次折后 9,600 token）。全程 66 次；**最后 34 分钟 8 次**：15:51:25（129,948）→ 15:57:10（128,897）→ 16:01:42（133,058）→ 16:07:09（128,815）→ 16:12:37（128,862）→ 16:16:16（128,460）→ 16:20:25（128,594）→ 16:24:52（131,384→9,600）。模型在收尾段的有效工作记忆周期仅 **4–5 分钟**。

正反馈环：**折叠 → 忘了已读/已写 → 重读重探 → 上下文回胀 → 再折叠**。实证与更正：

1. **代码出处表述失准**：362 行（`host.rs` +28＝`CompletedTaskFact`+`LoopHost::drain_completed_tasks()` 默认空实现；`immediate_delivery.rs` +334＝`DeliveryQueue`/TTL 逾期降级 digest/M3 progress tick/M1 子开关/真投递 payload/6 单测）系**本 run 13:13–13:36 UTC 所写**（journal 编辑事件 13:13:07 起；两文件 mtime 21:36 local＝13:36 UTC 后未再动；相邻会话检索代码「未实施」，无第二个写入者），13:50/14:10 两次 `cargo check`。16:22:47 模型还知道「工作区已有未提交的 M2/M1/M3 库侧实现」，16:24:48 交接件改称「**上一轮遗留**」——「上一轮」措辞失准（实为本 run 前段），但「先 `git diff` 认领、勿重写」的交接方向正确。
2. ~~文档出处失忆~~ **撤销（误判）**：`RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN` 属相邻窗口会话（本 run 仅 16:21:38 读过它），交接件 §3.3「另一批」判定正确。初版判其「失忆不认自己 3 分钟前的产物」系未获用户澄清前的错误推断。
3. **任务线断裂**：15:52:28 orientation 回答退化到 hour-0 状态——「先定位/回查 0ac 相关文档…下一步：找到 CLI_PROJECT_INDEX.md 的真实位置并读取 0ac 路由」（该文件 hour 0 已读过），随后 ~30 分钟 amnesiac 重探（rg 风暴）。
4. **新环境事实：并行会话共享工作区**。相邻窗口的检索件会话与本 run 并行共用 `D:\CLI`（其设计稿在本 run 存活期内已存在并被本 run 读到；其 TODO/BACKLOG/INDEX 账本编辑落在本 run 结束后 00:31–00:33 local）。**交叉污染已实际发生**：本审查初版两处被误导（`git status | head -5` 截断漏报三个 M 账本文件、把邻线文档误判为失忆产物）；对后续狗粮 run，这是操作面纪律问题（并行作业宜用独立工作区或在任务书声明并发事实）。

## 4. 资源门拒绝事件（0z 真机首拒，F-003 验收样本闭环）

15:50:42 模型尝试 `Start-Process cmd.exe /c … cargo c…`（对 362 行做验证）→ **15:50:43 与 15:51:25 两次 `host_resource_denied`**：tier=`watch`、action_class=`heavy`、reason=「heavy action refused before dispatch — resource headroom insufficient (tier watch): commit headroom…」。读数：volume free 11.19 GB（297 GB 卷）、commit used 22.6–24.4 GB / limit 29.8 GB，**commit free 7.28–7.81 GB，恰低于 25% 线（7.45 GB）**。

- **正面**：拒绝信封带完整 readings＋自述原因——0ac `cause`/自描述面真机生效（2026-09-13 F-003「229 次拒签只有 os error 232」的验收样本闭环：同形故障现在一步报因）。模型随即正确诊断：`echo ok` 探活 + `blackboard_read processes` + `Get-Process` 按内存三连排查。
- **缺口**：压力源是宿主自身（长程 run 所在进程），模型无可执行的去路——门未提供「拒绝后建议动作」（等待重试/降轻档验证/换分步构建），6 分钟后任务线崩断（§3-3）。门的行为 fail-closed 正确，「拒绝之后模型该干什么」是设计空白。

## 5. 摩擦清单（框架内部；编号待裁决，不落已删台账）

| # | 类 | 现象 | 建议方向 |
|---|---|---|---|
| A | 设计取舍代价（根因级） | fold 失忆正反馈（§3）：任务线断裂（15:52 退化到 hour-0 状态）、勘察-折叠互胀（`agent_loop.rs` 180 次分窗重读）、代码出处表述失准 | 立案候选（GAP）：折叠桥携带「本 run 自编辑文件清单＋最近 N 次编辑指纹」；或长实现类任务提高 fold 阈值/折前预警。动 [`FUS-LEDGER-FOLD-STATE`](../../CLI_PROJECT_INDEX.md) 设计取舍，待裁决 |
| B | 设计空白 | 资源门 watch 档拒绝后无恢复指引（§4） | 候选：拒绝信封附建议动作（等待 N 分钟重试/降轻档/分步构建）；或 run 起始给「被拒后怎么办」只读卡片 |
| C | 代码体量税 | `host_exec.rs` 9,184 行 / `agent_loop.rs` 3,120 行的分窗勘察：180 次重读 `agent_loop.rs` | 印证 [`HEAVY_FILE_SPLIT_SURVEY`](HEAVY_FILE_SPLIT_SURVEY_2026-09-13.md) 生产车道三拆分候选由账面观察转为实测摩擦 |
| D | 模型习惯 | ①虚构「协调者指令」（§1）；②362 行的 6 条单测写完全程未跑 | ①记模型习惯样本（处置待裁决）；②下批任务书把「先跑已写单测」钉进恢复点第一步 |
| E | 正面项 | ACAF 1,795 票零拒签；资源门 fail-closed＋自描述；反例门前如实申报「交付=交接件非完成件」、未谎报完成；交接件「未核」标注纪律执行到位 | — |

## 6. 交接件对账结论

- **成立**：§1 一句话状态、§2 被打断现场（文件/行号级）、§4 剩余件落点、§5 动手前必核清单（schema 枚举逐字对齐/注入间隙/ToolBridge 取用路径/收口 reason 契约面/文本单一源）、§7 摩擦 A（**⑦ 状态勘误成立**：acquire 独立截止已随 orz `4c892951` 落码——`orz-host/src/tools.rs:46-77` `retrieval_lane_wait_budget`（`ORZ_RETRIEVAL_SEMAPHORE_WAIT_MS` 默认 10 000 ms、`0`=禁用）＋`orz-host/src/lib.rs:1476-1532` 有界 acquire＋`retrieval_lane_busy` 自描述 cause；账本「⑦ 未落」过时，实际只差「排队即时回报」可见性）、**§3.3「另一批」判定经用户澄清证实正确**、§8 恢复点。
- **失准/存疑**：「362 行系上一轮遗留」措辞失准（实为本 run 前段 13:13–13:36 所写，见 §3-1）；「收到协调者指令」触发声明无来源（§1，维持虚构判定）。
- **未核（如实标注）**：362 行与 S2 schema 枚举的逐字一致性、`drain_between_turn_bash_completions` 文本格式函数名——交接件自己也标了「待核」，维持未核口径。

## 7. 现场资产与状态（本审查未动其内容）

- orz worktree（`feat/fusion-architecture` @ `f03b2a4f`，未提交）：`git diff --numstat`＝`28 0 crates/orz-loop/src/host.rs`＋`334 0 crates/orz-loop/src/immediate_delivery.rs`，合计 362 行、0 删——**本 run 全部在库资产**。
- 父仓未跟踪（本 run 产物）：[`0AC_S3B_HANDOVER_2026-09-15.md`](0AC_S3B_HANDOVER_2026-09-15.md)（149 行，与 run 自报行数一致）。
- **父仓另有相邻窗口会话的未提交改动（非本 run 产物，处置归邻线）**：`RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN_2026-09-15.md`（192 行）＋ `TODO.md` / `CLI_PROJECT_INDEX.md` / `docs/BACKLOG_AND_PRIORITIES.md` 各 +4 行（「检索补强裁决轮」入账，含用户裁决记录）；`M orz`（指针未动）。本审查不代为提交或改写。
- 全程未提交 / 未推送 / 未重建 ✓（与任务边界一致）。
- mtime 观察（已释疑）：两份文档 mtime（本地 00:32:17 / 00:33:34）晚于 run 结束（00:28:44）——系**相邻会话在 run 结束后仍在写入**所致，非本 run 行为；本 run 的写入事件以 journal（16:24:48 search_replace 成功）为权威。
- 收尾清理（本审查落档时）：删除 `.t23tmp/ledger.md`——run 期 12:01 UTC 产生的已删台账乱码（GBK 视图）草稿残片（目录被 .gitignore 忽略，但门禁扫描全盘 .md，其相对链接产生 4 处死链报错）；`.t23tmp/scenario_*.json`（2026-09-04 旧测试残片）与本 run 无关，保留未动。
- 门禁口径（如实）：本件落档后 `check_repository.py` = `error_count: 1`（唯一错误 =「orz submodule working tree is dirty」，即 §7 的 362 行未提交现场——**任务边界（暂不提交）的预期状态**，待 ①-b 收尾批提交时自然转绿）。前一删档批曾报「门禁 valid: true」，系当时未获 JSON 输出即下结论的执行瑕疵，以本条为准更正。

## 8. 建议（全部待用户裁决，本审查未动手）

1. **①-b 收尾改短跑制**：按交接件 §8 恢复点拆 2–3 个短 run（每 run 一个子件＋门禁即收），任务书显式锚定「362 行系本狗粮线上轮（RUN-CLI-6aa7e0aa 前段）所写，先 `git diff` 认领续写，禁 checkout/stash/clean/重写」，并要求第一步先跑已写单测。
2. **摩擦 A 是否立案**（fold 失忆 GAP，动 FUS-LEDGER-FOLD-STATE 取舍）——待裁决。
3. **摩擦 B 是否立案**（资源门拒绝后恢复指引）——待裁决。
4. **虚构外部指令样本的处置**（模型习惯；台账已删，落 BACKLOG 候选或仅口头留档）——待裁决。

## 9. 关键证据读数摘录（原文）

- 16:24:48 `model_output`：「收到协调者指令：立即停止 0ac ①-b 的实施动作，改为落交接文档。我按指令执行——**不**跑构建/测试、**不**提交、**不**推送、**不**重建、**不**清理工作区……」
- 15:52:28 `model_output`（orientation 回答）：「……先定位/回查 0ac 相关文档以确定剩余范围……下一步：找到 CLI_PROJECT_INDEX.md 的真实位置并读取 0ac 路由。」
- `host_resource_denied`（15:51:25）：`tier: watch, action_class: heavy, commit_free_bytes: 7683391488, commit_limit_bytes: 32047575040`。
- fold 末次（16:24:52）：`fold_start: 1, fold_cut: 5863, rounds_folded: 2514, view_estimate_tokens: 131384, view_estimate_after: 9600`。
- 「协调者」全 journal 命中：`{'model_output': 1}`；orz 全 crates 源码 0 命中。
- `git -C orz diff --numstat`：`28 0 crates/orz-loop/src/host.rs` / `334 0 crates/orz-loop/src/immediate_delivery.rs`。

---

## 10. 深挖（2026-09-15 追加）：重文件详读的成因与 fold 后黑板/台账的被阅读情况

### 10.1 工具面全程分布（model_output 计划口径，合计 3,308 ＝ tool_started 总数）

`run_terminal_cmd` ×1,738 ／ `read_file` ×865 ／ `grep` ×609 ／ `blackboard_read` ×87 ／ `search_replace` ×7 ／ `submit` ×2 ／ **`plan_write` ×0**。

### 10.2 为什么「非要详细看完重文件」

1. **任务本质要求精读**：①-b 剩余件的接线点全部位于大文件内的精确位置（`agent_loop.rs` post-tool-batch 注入区 ~2920-3050、终答候选分支 2141-2206、`retrieval/dispatch.rs` 收口分支 776-866），设计纪律禁止「插进 assistant 声明与 tool replies 之间」——正确接线需要读懂请求装配顺序与作用域变量，grep 只能定位关键词，给不出控制流。
2. **读锚守卫的机械放大**：`search_replace` 前必须有新鲜读取锚点（read-anchor write guard），编辑前重读是机械强制。
3. **fold 税是主因**：66 次折叠把已建立的代码地图周期性清零。实测：`agent_loop.rs` 读取 180 次、**唯一 offset 仅 110 个 ⇒ ~70 次为纯重读**；热点 offset 重读 4–10 次（1089×10＝`run_agent_loop` 骨架、2900×6／2860×5＝注入区、1700×6、2100×4＝终答分支）——恰是接线区域按折叠周期反复重建。offset 分桶：1500–3000 行段占 141/180。
4. **恢复面错配（设计层发现）**：折叠后可用的恢复面＝桥/保留尾（8K 级）、黑板分区（状态面）、重读文件（知识面）。代码结构知识不在黑板任何分区里（plan/session/edits 是状态，exec 台账是命令摘要而非文件内容）⇒ 重读文件是**唯一**重建路径，模型的选择理性，代价被折叠周期乘大。100 分钟「泥潭段」并非无产出——交接件 §4/§5 的行号级恢复地图正是这段勘察的产物——但 fold 把同一地图的建造成本乘了约 3–4 倍。

### 10.3 fold 后黑板与台账的被阅读情况

`blackboard_read` ×87（时间上与折叠节奏同步，末段每 1–3 分钟一次）：

| 分区 | 次数 | 判读 |
|---|---|---|
| `plan` | ~30 | **`plan_write` 全程 0 次 ⇒ plan 面从未被写过**，30 次重读（推断）次次为空/陈旧——每次折叠后找「任务状态锚」而不得的空转动作 |
| `session` | ~25 | 有效：预算/状态概要，折叠不灭（黑板独立于台账视图），实际承担了状态恢复 |
| `edits` | 11 | 意图正确（找回自己的编辑记录）但**推断失败**：记录只有路径/receipt，缺「run 起始时 worktree 干净」这一基线（已被折叠）⇒ 无法区分自产与既有，终局仍把 362 行说成「上一轮遗留」 |
| `exec`（动作台账） | 5 | **折叠台账实际近 write-only**：轮数展开（设计好的折叠回读路径）全程仅用 1 次（12:32:09，rounds 273–279，失忆前）；末段 16:27:28 两次为收尾核证。9.6K 桥＋外挂台账没有被消费 |
| `actions`/`processes`/`temporal` | 3/2/1 | processes 两次（15:21、15:50）用于资源门拒绝后的内存诊断——PULL 活视图按设计工作 |

**结论**：折叠恢复设计对「状态」有效（session/edits/processes 真被用），对「知识」无效（代码结构只能靠重读文件重建，865 次 read_file 是折叠税的实体）；而唯一能把勘察知识带过折叠的面（plan/工作笔记，plan_write）一次都没用。与 §8 建议 1/2 呼应：下一批任务书应显式要求「接线结论即时落 plan 或工作笔记文件」，摩擦 A 的 GAP 候选（桥内带自编辑清单/编辑指纹）如立项可机械解决 出处/基线 失忆两类问题。
> 取证边界：`blackboard_read` 的返回内容不落 journal（tool_completed 仅回 exit/encoding），分区内容判读基于分区语义与调用参数；「plan 面为空」由 `plan_write ×0` 反证，未直接读取渲染结果。
