# 会话生命周期用户主权设计（0dd S1：run 末自动归档退役）

> **批次**：275（0dd S1 设计稿；零源码、零 API、零跑批；本批不重建载体）。**日期**：2026-10-16。**状态**：S1 定稿，D1–D8 自裁在案（266 批档 §3 批序授权）。
> **上游**：266 批档〔0dd 立项＋实证证据〕、0cy 设计稿（[`GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md`](GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md)，同域姊妹件）、滑块设计稿 §3.5.1〔增量归档时点 v7 定案＝本项退役对象的设计出处〕。
> **一句话**：交互（ACP）车道 **run 结束零归档动作**——删除 `handle_session_prompt` run 尾的 500K 里程碑增量归档 spawn 块（DP-11④ 的 ACP 半边退役）；对话生命周期控制（存档／新开对话／回档／删除）主权全归用户手动面；close 道、0ak 无头道、0cy 启动侧收卷三道**原样不动**；归档原语单一源不破。

---

## §0 事实与定位（为什么做）

1. **实证摩擦**（266 批 §2）：web 真人测试轮（0.9.0 载体、会话 2b1f85d2）——`RUN-2b1f85d2-0` 于 `run_finished{status:completed, intent:conclude}` 后 **80ms** 出现 `session_archive` 事件（记录 run `ARC-2b1f85d2-1`＝`run_preflight → session_archive → run_finished` 三事件；包 `.gsa/archives/2b1f85d2.json.gz`、**`incremental=true`**、`fatigue_pct=4`）；用户续聊被迫手动回档（回档成功即删包＝`acp_server.rs` restore 既定语义，非丢失）。
2. **用户裁决**（266 批 §1 逐字）：「存档这一类控制对话的行为本身就应该交给用户进行手动处理，不用自动存档，会话正常维持即可，用户需要存档/新开对话的话会自己处理的，自动归档太影响日常使用了」；「run 结束不得自动归档当前/存活会话；会话跨 run／跨日正常维持」。
3. **定位**：只裁「自动面与用户手动面的边界」；**不动** 0cy 核心语义（新对话新台账＋上一轮进存档，266 §5 明示不受挑战）、**不动**手动面三件（归档/回档/删除）、**不动**数据持久化面（侧车每个成功 prompt 恒落、journal 恒在）。

## §1 触发面勘定（裁决 D1）——归档动作四道清单

> **勘注（2026-10-16，279 批）**：本节「触发入口共四道」漏列**第五道＝on-demand 归档**（原文逐字保留于下）——`archive_session_on_demand`（orz `acp_server.rs:867`），经 orz-bin `main.rs` CLI `archive` 子命令与 orz-web `POST /api/archives/{s8}` 到达，同产 `session_archive`/ARC；属**用户显式动作**、合本项主权语义（§2 目标形态原样），非肇事面、非自动道。279 批勘定。

源码勘定（orz `0ba00262`，2026-10-16 逐点核对）——`session_archive` 事件的生产汇点唯一（`archive_raw_session_package`），触发入口共四道：〔道数已由 279 批勘定为五道，见上勘注〕

| # | 触发道 | 入口 | 时机 | 存活会话可被打包？ | 判定 |
|---|---|---|---|---|---|
| ① | **run 尾里程碑增量**（DP-11④ ACP 半边） | `handle_session_prompt` run 尾（`acp_server.rs:3593` spawn 块） | 每个成功 run 尾：`incremental_archive_due`（实际上下文 ≥500K 且 ≥ 水位＋500K）⇒ spawn 归档任务（`incremental=true`、`include_footprint=false`、**不删源**） | **是——正在 prompt 的就是当前会话** | **肇事面** |
| ② | close 归档（P2-13 B3） | `close_session`（生产调用点＝`codex_app.rs:622` codex 线程关＋`orz-signer` 管理命令；web 桥不调用——ACP 无 session/close） | 显式关闭时打包（`incremental=false`、含足迹）；run 进行中关闭＝推迟票（B3 P2-3）于 run 尾补触发 | 仅显式关闭动作的完成 | 保留（D3） |
| ③ | 0ak 无头里程碑 | `headless_session_archive`（`-p` 收尾） | 一次性 run 尾同判据 | 无头车道无「当前会话」语义 | 保留（D4） |
| ④ | 0cy 启动侧收卷 | `handle_session_new_with_options`＋orz-bin `run()` 起点 | 新会话起点扫描**非存活∧非当前**会话足迹 → 收卷（唯一删改路径） | 否（存活∧当前被保留集排除） | 保留（D5） |

**肇事链闭合**（①→用户摩擦的完整因果）：长单会话实际上下文跨 500K → run 尾（`run_finished` 后 80ms）打包 `incremental=true`（源不删）→ web 探索器分组判定 `isArchived(s8)＝有归档包 ∧ 包 mtime ≥ 最新真实 run mtime`（`orz-web/assets/app/widgets.js:250`；ARC 审计 journal 被排除在活动水位外）→ 包更新于 run 之后 ⇒ 判定成立 → 会话被挪入「归档会话」组 → 用户续聊被迫手动回档。**266 批「何者使用户必须手动回档待勘」就此勘定：①＋web 分组投影联动，④ 无涉**（旁证会话 e6508b76 被 ④ 收卷属 0cy 既定语义）。

## §2 主裁决（D2）——run 尾增量归档退役

- **删除** `handle_session_prompt` run 尾的 `incremental_archive_due` spawn 块（含 DP-11④ 注释）：交互车道 run 结束**零归档动作**，目标形态＝「仅用户显式动作归档」——web 组级「归档」键（`POST /api/archives/{s8}`）、CLI `orz archive <s8>`、新开对话触发的 0cy 收卷。
- **原语不动**（禁第二套实现纪律不破）：`incremental_archive_due`、里程碑水位（`<s8>.milestones.json` 单调幂等）、`record_archived_tokens`、`archive_session_package(incremental=true)` 全部保留——无头道（③）继续使用。
- **附带收尾**：`PendingArchiveTicket.incremental` 字段退役（① 删除后 ACP 票全为 close 道，恒 `false`）；`archive_session_package` 的 `incremental` 参数保留（无头道传 `true`）。
- **效应面**：`session_archive` 事件族、web 投影、TUI 事件渲染零契约变更——只是交互车道不再自发产生该事件。

## §3 联动面裁决（D3–D6）

- **D3 close 道与推迟票不动**：② 是显式关闭动作的完成（含 run 中关闭的推迟补触发），不是对存活会话的自动归档——B3 P2-3 语义原样。
- **D4 0ak 无头道不动**：ADR-0010 §14.68 边界原样（一次性 run 无 close 语义 ⇒ 里程碑增量归档保留）；与交互车道退役互不影响（原语共享、车道分立）。
- **D5 0cy 卫生闸口径＝保留现状，不改提示制**：① 266 §5 明示 0cy 核心语义不受挑战；② ④ 的触发点挂在用户显式动作链上（web「新开对话」＝`acpSessionId` 置空后 `session/new` 换新会话 id → 扫描收上一轮；CLI 每条 `-p` run 起点同理）——「用户发起转换、机械收尾」与主权语义相容；③ 提示制把删改决定推回用户、新增交互面与摩擦，违背 0cy 立项初衷（卫生自动化）；④ 肇事面不在 sweep（D1 勘定）。**差量回写**：0cy 设计稿 §2 触发时机表「on-demand `orz archive <s8>`／500K 增量」行随 S2 加勘注——「500K 增量」自 0dd 起仅指无头车道（0ak）；ACP 交互车道 run 尾增量已退役。
- **D6 web 分组投影不动**：`isArchived` mtime 判定原样——自动包消失后该判定如实反映现实（手动归档 → 入「归档会话」组；归档后续跑 → 回活跃组，widgets.js 既有注释语义）；零前端改动。run 中归档被前端禁跑守卫拦截（「该会话正在运行，暂不能归档」）原样。

## §4 `ORZ_GSA_HYGIENE=0` 边界注记（裁决 D7）

实测（源码逐点核对，266 批遗留问题就此裁决）：`hygiene_enabled()` **只**注入 `sweep_session_leftovers` 的两个生产调用点（`handle_session_new_with_options` 启动扫描＋orz-bin `run()` 起点扫描）。**不管辖**：close 归档（②）、run 尾增量归档（①，本项裁退役）、0ak 无头里程碑（③）、on-demand 归档。即 `=0` 只关「收卷删源闸」、**不关「自动打包闸」**——266 批「run 末归档是否受其管辖待核」答案为**否**（用户以 `=0` 自救本不会生效；S2 后该问题随①退役而消失）。注记随 S2 落 README env 面（如有表）与文档。

## §5 数据安全与残差（裁决 D8）

- **零数据丢失面**：对话侧车每个成功 prompt 恒落（`persist_conversation_sidecar`）、journal 逐事件恒在；归档包是**派生打包物**——不打包不丢内容，0cy 收卷在下一会话起点对上一轮兜底。
- **接受残差**：滑块 v7 当初以①闭合「长单对话不结束就没有存档」缺口——该缺口在交互车道**重新打开**，为本项用户主权的显式接受项（手动归档键常在；无头道保留里程碑）。滑块设计稿 §3.5.1「增量归档时点（v7 定案）」随 S2 dated 勘误：会话关闭一次保留＋无头里程碑保留＋**ACP run 尾半边退役（0dd）**。
- **登记残差**：交互会话里程碑水位不再推进（文件保留、无头道继续写）；ARC journal「每条会话存档一次」在交互车道仅在用户显式动作时发生。

## §6 实现触点（S2 清单）

| 文件 | 改动 |
|---|---|
| `orz/crates/orz-host/src/acp_server.rs` | 删 `handle_session_prompt` run 尾 spawn 块（§2）；`PendingArchiveTicket` 退役 `incremental` 字段（spawn/ticket 构造两处连带）；`incremental_archive_due`／水位／`archive_session_package(incremental)` 保留（无头道）；注释同步 |
| 设计稿转录 | 滑块设计稿 §3.5.1 dated 勘误；0cy 设计稿 §2 差量勘注；ADR-0010 §14 新节转录（版本号随转录时点现行头） |
| 文档 | 父 README L77「交互会话结束时由 session_archive 打包」措辞可选收紧（close/手动面口径）；env 注记随 §4 |

## §7 判据与钉子

- **钉子（S2 测试）**：N1 **run 尾零归档钉**——会话续作（continuation）≥500K 刻度成功 run 后：journal 零 `session_archive`、`archives/` 零新包、水位文件不动；N2 close 推迟票回归钉（run 中关闭 → run 尾补归档，B3 语义不翻）；N3 手动面恰等回归（archive/unarchive/delete 既有测试全绿，回档删包语义不动）；N4 无头里程碑幂等（既有 `headless_run_archive_is_idempotent_per_milestone` 不翻）；N5 纯判定函数不动（既有 `archive_keys_and_incremental_milestones_follow_the_design` 不翻）。
- **S3 载体重建进体**：判据＝行为面（①的退役无常量文案可作字节判据；触发例 journal 断言随 N1 离线可复验）＋测试套全绿；版本号随批。
- **S3 漂移窗口风险注记（279 批，2026-10-16）**：S3 载体重建放行前，0.9.0 在役载体仍含 run 尾自动归档行为（① 的退役未进在役二进制），真机用户仍会踩 266 同款摩擦（run_finished 后自动打包 → web 分组挪档 → 被迫手动回档）；窗口期以本句为登记，随 S3 进体消灭。
- **S4 真机复验**（266 批草案维持）：web 同会话跨 prompt／跨日续用不归档（探索器恒在「活跃会话」组）＋手动存档/回档/删除面回归＋新开对话收上一轮照旧（0cy 语义）＋0.9.0 载体对照 journal（增量 ARC 事件不再出现）。

## §8 边界

1. 本批（S1）零源码；S2 才动 `acp_server.rs`；orz 子模块本批不动（HEAD `0ba00262`）。
2. 不动 web 前端任何文件（D6）；不动 `orz-signer`／`codex_app` 调用点（D3）。
3. 0cy 设计稿与滑块设计稿的勘误**随 S2 落码批**同批落（S1 只登记义务，不改既有 dated 档）。
4. 计数不变 61（S1 非闭合批）。

## §9 裁决记录（D1–D8 浓缩）

| # | 事项 | 裁决 |
|---|---|---|
| D1 | 触发面勘定 | 肇事面＝①run 尾里程碑增量（`incremental=true` 实证锚定）＋web `isArchived` mtime 投影联动；④0cy 无涉 |
| D2 | 主修复 | ①退役（删 run 尾 spawn 块）；原语单一源保留给无头道；票字段 `incremental` 连带退役 |
| D3 | close 道 | 不动（显式关闭的完成；推迟票 B3 P2-3 原样） |
| D4 | 0ak 无头道 | 不动（§14.68 边界原样） |
| D5 | 0cy 收卷口径 | 保留现状不改提示制；§2 表差量勘注随 S2 |
| D6 | web 投影 | 不动（自动包消失后 mtime 判定如实） |
| D7 | `ORZ_GSA_HYGIENE=0` | 只关启动侧扫描闸，不关打包闸（266 遗留答案＝否） |
| D8 | 数据安全 | 零丢失面（侧车/journal 恒在）；「不结束没存档」缺口交互车道重开＝用户主权显式接受 |
