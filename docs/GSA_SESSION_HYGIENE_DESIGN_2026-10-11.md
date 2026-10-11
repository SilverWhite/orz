# .gsa 会话卫生设计（0cy S1：收卷·归档·净室）

> **批次**：246（0cy S1 设计稿＋S2 落码合批；本批不提交/推送/重建）。**日期**：2026-10-11。**状态**：S1 定稿，D1–D8 自裁在案（任务书授权）。
> **上游**：235 批档〔0cw 污染审计＋0cy 立项〕、0da 处理轮报告 §4/F1/F2（现场输入）、FOLD 设计（黑板/会话存档形态）。
> **一句话**：把 `.gsa` 与黑板对齐为「**新对话新台账、上一轮整卷进存档**」——会话足迹在**新会话启动**时被合并收卷进 `.gsa/archives/<s8>.json.gz`（包 schema 升 v0.3，增 `footprint` 段），活区清空；收尾侧（close/headless/on-demand）沿用既有「附加包」语义并升级为含足迹包（**不删源**）——删改动作只发生在启动侧扫描这一条路径。

---

## §0 事实与定位（为什么做）

1. **污染三因**（235 §2/§4/§7）：
   - ① `ledger/current.md` 固定路径单文件**跨会话滚动**——上一会话的机械行即下一会话「轮 1 之前的历史」（0cw 中 188/189 档读）；
   - ② 上一轮**会话足迹留活区**（`conversations/`、`orientation/`、`activations/`、他档 `runs/`）——82/189 档显式读前档会话数据（含逐条打印前档 messages）；
   - ③ 0ct 读向全开 ⇒「可读即可触」——不作隔离，任何留在活区的旧事实都会被下一对话吃到。
2. **实测证据**：0cw 容器内卷**无 `archives/` 目录**（上一轮从未进存档）；`D:\CLI` 根活区＝58 包＋2.8MB 旧 ledger（mtime 9/28）＋5 orientation＋5 activations（0da 报告 §4）。
3. **同族摩擦**（0da F1）：会话档案模板文案宣称机械行「外挂存档于 `.gsa/ledger/current.md`（本会话内固定）」，实况却是旧累积未重置、无本 run 行——文案与机制错位的根因即本机制缺失。
4. **定位**：只动「活区可见性」；**不动** 0ct 读全开语义、**不动**黑板（板面本已会话级）、**不动** retention 对其它机器目录的既有面向。

## §1 语义定义

- **会话足迹（session footprint）**＝该会话在工作区 `.gsa` 下的**全部对话性/机械性产物**的并集（清单见 §3.1）；**不含**机器面目录（keystore/snapshots/rollback/process_trees/…，见 §10 边界）。
- **收卷（rollup）**＝「收集足迹 → 合并旧包 → 原子写 `archives/<s8>.json.gz` → 成功后删源」。**先写后删**：包是移动的落点，不是删除动作；写失败永不删源。
- **活区 / 存档区**：活区＝`.gsa` 下模型可直读的运行时目录；存档区＝`.gsa/archives/`（仍可读，但形态为 gzip 信封、且**不属于任一后续会话的起点面**）。
- **与黑板对偶**：黑板＝新会话空起点（内存态装载、无残留）；`.gsa` 活区自此同规——**新会话起点为空台账＋无前档足迹**。

## §2 触发时机（裁决 D1）

| 时机 | 动作 | 删源 |
|---|---|---|
| **新会话启动**（ACP `session/new`；CLI `-p` run 起点） | 扫描非存活、非当前的会话足迹 → **收卷** | **是**（唯一删除路径） |
| **会话收尾**（`close_session`；CLI headless run 尾） | 既有附加包升级为**含足迹包**（`footprint` 收集，`complete` 语义＝当刻全量） | 否 |
| on-demand `orz archive <s8>`／500K 增量 | 原样（仅对话包；不收集足迹） | 否 |

- **理由**：删除路径**单一**（启动侧）＝可审计、可重试、不与「close 后同会话恢复」「评测证据（journal 拷贝在收尾前完成）」「本地分析期内文件仍在」相冲突；收尾侧保持既有时序（包含 PendingArchiveTicket 的推迟语义），只把包升级为含足迹。

## §3 扫描与归属（裁决 D2/D3/D8）

### §3.1 足迹清单（按名归属）

| 类别 | 路径口径 | 归属解析 |
|---|---|---|
| 对话侧车 | `.gsa/conversations/<s8>.json` | 文件名 stem |
| 定向 | `.gsa/orientation/<s8>.json` | 同上 |
| 激活 | `.gsa/activations/<s8>.json` | 同上 |
| 拷问（grill） | `.gsa/grill/<s8>.jsonl` | 同上 |
| 运行 journal | `.gsa/runs/RUN-<s8>-*`、`RST-<s8>-*`、`RUN-CLI-<s8>` | 目录名（沿 `reconstruct_conversation_from_journal` 同一口径） |
| 压缩摘要档 | `.gsa/compaction/compaction-<run_id>-NNNN.md` | run_id → s8 |
| 回放块档 | `.gsa/compaction/blocks/<s8>-block-NNNN.md`、`<s8>-clear.md` | 文件名前缀 |
| **共享滚动** | `.gsa/ledger/current.md`；`.gsa/session/terminal/*.log` | 归属「最近被收卷会话」（见 §3.3） |

- **豁免（保留不动）**：`runs/ARC-<s8>-*`（存档回执——存档审计链自身，含零对话事实；retention 兜底）。

### §3.2 存活判定与保留集（D2）

- **存活**＝本进程 `AcpServer` 会话表内（进程内即真）；**保留**＝`{当前新会话}` ∪ 存活集。
- 跨进程并发（同工作区两个活进程）＝**登记边界**：不支持净室语义（既有固定路径 ledger 亦未支持并发写；见 §10）。
- 扫描**不信 mtime 新鲜度**（非存活即收卷）：上一进程被杀（被测车道常态）与「空闲但未关闭」在文件面不可分——统一按非存活处理。

### §3.3 共享文件归属（D8）

- 候选人＝被收卷会话 ∪（保留集中的当前会话，若其已有足迹＝同会话恢复）。
- **winner**＝候选人中足迹 mtime 最新者：
  - winner ∈ 被收卷集 ⇒ ledger/terminal 随其收卷（进包）并**删除**活区件；
  - winner ＝ 当前会话（恢复场景）或候选为空 ⇒ **保留**（不删；报告记 note）。
- ledger 保持**固定路径** `.gsa/ledger/current.md`（「新对话新台账」由「扫描已移除旧文件」达成）——指针文案、`ledger_file_path`、既有测试**零改动**。

## §4 包布局（裁决 D4）

### §4.1 信封 v0.3

```json
{ "schema": "session-archive-package-v0.3",
  "conversation": <sidecar 原文 JSON，零内容变换；无对话时 null>,
  "archive_keys": { 三键互标段（既有结构不动） },
  "footprint": { "collected_at": "…Z",
    "ledger": {"path":"…","text":"…"} | null,
    "orientation": {…} | null, "activation": {…} | null,
    "runs": [ {"id":"RUN-<s8>-0","files":[{"name":"events.jsonl","text":"…"}]} ],
    "compaction": [ {"path":"…","text":"…"} ], "blocks": [ … ],
    "grill": [ … ], "terminal": [ … ],
    "counts": {"runs":N,"compaction":N,"blocks":N,"grill":N,"terminal":N,"bytes":M} } }
```

- `footprint` 在场 ⇒ **全量快照**（close/headless/收卷三处收集；on-demand/增量不收集、写 `null`）。
- 读取侧 tolerant 不变：`decode_archive_package`（旧裸包/旧 v0.2）与 web `split_envelope`（`schema`＋`conversation` 判据）均可读 v0.3；新增 `footprint` 对旧读者透明。

### §4.2 合并写包（防丢）

- 收卷时若**旧包在场**：旧包 members ∪ 盘面（盘面按 key 覆盖旧值；如 `runs` 按 run id、`compaction`/`blocks`/`grill`/`terminal` 按 path、单件直接覆盖；盘面缺失而旧包有 ⇒ 保留旧值）——覆盖「删源中途崩溃重试」「close 后再恢复再收卷」两类交错，**旧成员不因重包而丢**。
- 旧包不可读/损坏 ⇒ 记 warn，按盘面重包（边界登记）；写包原子替换沿既有 `tmp+rename`（先 remove 后 rename 的 Windows 语义不变）。

### §4.3 删源

成功后删除：§3.1 全部在场件（含 `<s8>.milestones.json`）＋（winner 时）共享件；`ARC-*` 豁免。删除失败逐件 warn（下次扫描重试；包已在 ⇒ 幂等安全）。

## §5 契约面（裁决 D5）

- **零新事件类型、零 payload 新字段**：收卷复用既有 `session_archive`（archive_id/path/digest/status/attempts/fatigue_pct），`runtime/session-archive-event-payload-v0.2.schema.json`（`additionalProperties:false`）**不动**；收卷性质由包内 v0.3 `footprint` 段**自描述**。
- 0ct 读全开语义**不动**；`.gsa/archives` 属尚在活区内的档案面（可被有意解包＝登记残差，见 §6/§10）。
- ARC journal 回执保留＝「每条会话存档一次」既有审计链不破。

## §6 净室更强隔离（裁决 D6）

- **`ORZ_GSA_ARCHIVE_ROOT`**：归档根覆盖（绝对路径原样；相对路径对工作区根解析；缺省 `.gsa/archives`）。orz-host 统一三处构造（打包/on-demand/水位），orz-web 读取侧同规则（跨 crate 小重复，登记为同一环境变量口径）。净室重跑可由宿主把归档根指到工作区外。
- **`ORZ_GSA_HYGIENE=0|off|false`**：关断启动侧扫描（逃生舱；缺省开）。测试走注入变体（`_with(..., enabled)`）避免 env 竞态。

## §7 unarchive / delete 升级（裁决 D7）

- **unarchive**（Web「回档」）＝删包＋**按需恢复**：仅当活区**缺件**时从包内恢复（conversation/ledger/orientation/activation/runs/compaction/blocks/grill/terminal），**绝不覆盖**现存文件；ledger 仅在 `current.md` 缺席时恢复；恢复后照旧删水位。
- **delete-session**＝删除面扩至新足迹目录（orientation/activations/compaction/blocks/grill；共享件不动），保持「彻底删除」口径。

## §8 实现触点（S2 清单）

| 文件 | 改动 |
|---|---|
| `orz/crates/orz-host/src/acp_server.rs` | 新节「0cy 会话卫生」：`archives_root`／足迹收集器／v0.3 信封／`rollup_session`／`sweep_session_leftovers(_with)`／merge 逻辑；`package_archive_raw` 增 footprint 参数；`handle_session_new_with_options` 入口接扫描；`close_session` 包升级；unarchive/delete 升级 |
| `orz/crates/orz-bin/src/main.rs` | `run()` 起点接扫描（keep=None）；`headless_session_archive` 收集足迹（不删源） |
| `orz/crates/orz-web/src/archives.rs` | `archives_dir` 读 `ORZ_GSA_ARCHIVE_ROOT`；注释同步 v0.3 |

## §9 判据与钉子

- **S4 真机判据**（联动轮）：harbor 多步重置语义复验＝① 新会话空板；② 活区无前档足迹；③ `archives/` 在案（含足迹成员）＋提交/推送/重建后不回归。
- **钉子（S2 测试）**：N1 台账空起点＋归档；N2 包成员逐字对账；N3 存活保护；N4 残留扫描（无表项文件）；N5 幂等（连扫两次不丢不重）；N6 unarchive 恢复；N7 共享归属；N8 关断开关；N9 归档根覆盖；N10 merge 防丢。CLI 起点扫描＝orz-bin 集成钉。

## §10 边界与残差登记

1. **跨进程并发会话**（同工作区多活进程）无净室保证（扫描按非存活处理）。
2. **档案包可被有意解包**：0ct 下 `.gsa` 全可读；净室重跑请用 `ORZ_GSA_ARCHIVE_ROOT` 移出（D6）。
3. **机器面目录**不在本批：keystore/snapshots/rollback/process_trees/pdf-*/one_shot_permit/blackboard-epoch/project-doc-index/grok-home（retention/既有机制各自覆盖）。
4. `RUN-PLAN-*`（--plan 车道 run journal）无会话归属，不入扫描（同样有可读性残差，登记）。
5. 共享文件归属在「多会话交错」下可能不精确（包不丢数据，仅归属落点可能非最优）。
6. 旧包损坏时重包可能丢已删源（先写后删窗口内的损坏＝磁盘级事故，登记）。
7. `close` 后同会话恢复再收卷＝merge 覆盖（N10 钉）；包替换非严格原子（沿既有 B3 登记）。

## §11 裁决记录（D1–D8 浓缩）

| # | 事项 | 裁决 |
|---|---|---|
| D1 | 收卷时机 | 启动侧扫描为唯一删改路径；收尾侧只升级包（不删源） |
| D2 | 扫描范围 | 非存活＋非当前；存活＝进程内会话表；跨进程＝边界 |
| D3 | 足迹清单 | §3.1；ARC 回执豁免；机器面不入本批 |
| D4 | 包布局 | schema v0.3＋footprint；合并写包；先写后删 |
| D5 | 契约面 | 零新事件类型/零 payload 新字段；自描述于包 |
| D6 | 净室 | `ORZ_GSA_ARCHIVE_ROOT`＋`ORZ_GSA_HYGIENE=0` |
| D7 | 回档/删除 | unarchive＝删包＋缺件恢复；delete 扩面 |
| D8 | ledger | 固定路径不动；共享件归 winner；空起点由扫描达成 |
