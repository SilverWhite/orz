# 338 批：回档后会话回放顺序坍塌修复（真机 c23e0aaa，2026-10-22）

> **用户令**：「当前 `C:\Users\1\OneDrive\Desktop\新建文件夹` 工作区的会话 c23e0aaa 的会话记录不对，疑似渲染出现问题」。真机补充：「最后一条消息是旧消息，不是最新的，标记栏本身也是错乱的，点击恢复与继续后主窗口部分依旧如此，且标记栏丢失内容且错乱，仅有最后两条的标记……你可以杀掉当前正在运行的 orz 并重启一个自己看一下」。
>
> **本批**＝按 0dk S1／0br S2 的 journal 投影口径取证，定位为**排序键失效**（不是渲染器），在 `orz-web` 服务端与前端各改一处并加钉。计数 64 不变；载体未重建。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 症状定性 | **顺序坍塌**，非渲染错位：记录末尾落在 run 8/9（10-20 的 O1d 内容），标记栏按错误回放顺序铺开 |
| 触发条件 | 会话被「回档」（archive → unarchive）后，**同会话全部 journal 的 mtime 被压到同一刻** |
| 实测 | c23e0aaa 的 24 个 `events.jsonl`：`20:19:07.700–.744`（44 ms 跨度、16 个不同值） |
| 根因 | `runs.rs:list_runs` 只按 mtime 排序且无平局判据 → 退化为目录名字典序 `0,1,10,…,23,3,4,5,6,7,8,9`；`main.js` 原样反转该顺序铺记录，`syncLiveTail` 取 `runs[0]` 当“最新 run” |
| 修复 | ①服务端：`order_run_entries` 改为**先按会话分组、组内按序号降序**；②前端：`orderedSessionRuns`／`latestSessionRun` 显式按 `RUN-/ARC-/RST-<s8>-<n>` 的 n 排序；③ARC 审计 run 同轴；④撤回窗口起点改取 journal 的 `run_started`（`started`）而非 mtime |
| 读数 | `orz-web` lib 测试 **76 passed / 0 failed**；`frontend_smoke.mjs` **全部通过**（新增 17.1/17.2/17.3） |
| A/B | 旧 0.9.8 `/api/runs`：`RUN-8, RUN-9, RUN-7, …, RUN-1, RUN-10, RUN-0`；新开发版：`RUN-23, RUN-22, …, RUN-0` |
| 未做 | 双平台载体重建与换装、S2 真机重跑、归档包回写 mtime（见 §6） |

## §1 症状（用户真机四件）

1. 打开 `conversation://c23e0aaa`，记录**末尾是旧消息**（10-20 的 R231／O1d 段），最新回合（10-21 LIT-002／R236）不在末尾。
2. 标记栏乱序：`T16, T18, T17, T20, T3, T22, T21, T24, T23, T4, T5, T6, T7, T8, T10, T9`。
3. 点「恢复并继续」后主窗口**仍是同样内容**，标记栏只剩最后两条（`T9 输入`／`T9 最终输出`）。
4. 探索器该会话标为 `（27 次运行 · 未完成）`——27＝24 个 `RUN-` ＋3 个 `ARC-`（副证，见 §6）。

## §2 取证

**① 文件系统事实**（`.gsa/runs/RUN-c23e0aaa-*/events.jsonl`）

| 观测 | 值 |
|---|---|
| mtime 跨度 | `2026-10-21 20:19:07.700` – `.744`（44 ms） |
| 不同 mtime 值 | 16（24 个文件） |
| 排序结果 | 与目录名字典序同构：`0,1,10,11,…,23,3,4,5,6,7,8,9` |

**② 写入面事实**：`orz-host/src/acp_server.rs:2314 restore_session_from_archive` 逐件 `std::fs::write`；归档包（`{s8}.json.gz`）只存文件正文与相对路径，**不存时间戳**。c23e0aaa 的归档包已不存在（20:19:07 完成回档，`ARC-c23e0aaa-24` 20:18:33 落盘可对时）。

**③ 代码面事实**：`orz-web/src/runs.rs:list_runs` 以 `modified_ms` 降序（`sort_by_key(Reverse(mtime))`，无平局判据）；`assets/app/main.js:openConversation` 取 `state.runs` 过滤后 `.reverse()` 铺记录；`syncLiveTail` 用 `runs.find(...)` 取“最新 run”。

**④ A/B 实测**（同一 `.gsa` 快照，`/api/runs` 原样读取）

- 旧载体 0.9.8（`C:\Users\1\AppData\Local\orz\bin\orz.exe`，59,261,952 B）：
  `RUN-8, RUN-9, RUN-7, RUN-6, RUN-5, RUN-4, RUN-3, RUN-22, RUN-23, RUN-20, RUN-21, RUN-2, RUN-18, RUN-19, RUN-16, RUN-17, RUN-15, RUN-14, RUN-11, RUN-12, RUN-13, RUN-1, RUN-10, RUN-0`
  ⇒ 前端反转后记录末尾＝`RUN-9`／`RUN-8`（T10／T9），**与用户截图逐项吻合**。
- 新开发版（338 源码 debug 构建）：`RUN-23, RUN-22, …, RUN-1, RUN-0, ARC-24, ARC-18, ARC-1`
  ⇒ 反转后记录末尾＝`RUN-23`（LIT-002 最新回合）。

## §3 根因

```text
回档（unarchive）
  └─ restore_session_from_archive：逐件 std::fs::write（包内无时间戳）
       └─ 同会话 24 个 journal 的 mtime 压到同一刻（44 ms 内）
            └─ list_runs 仅按 mtime 排序、平局无判据
                 ├─ 会话回放（openConversation）原样反转 ⇒ 记录末尾落在旧 run
                 ├─ 标记栏按回放顺序重建 ⇒ 乱序
                 └─ syncLiveTail 取 runs[0] ⇒「恢复并继续」挂到 run 9
```

**判定**：渲染器（`projection.js`／`widgets.js`）本身按事件如实投影，问题在**喂给它的 run 顺序**。故本批不改投影逻辑，只改排序口径。

## §4 修复

**A. 服务端**（`orz-web/src/runs.rs`）

- 新增纯函数 `order_run_entries(Vec<(run_id, modified_ms)>) -> Vec<run_id>`：
  - `split_run_id` 取尾段数字为序号（`RUN-/ARC-/RST-<s8>-<n>`），无序号形态自成一键；
  - **先按会话分组，组间按组内最大 mtime 降序，组内一律按序号降序**，再以完整 id 降序兜底确定性；
  - `list_runs` 改为先收 `(id, journal, mtime)` 三元组、用该函数定序、再截 `LIST_CAP` 与内容探针。
- 语义保持：仍“新在前”；无序号形态（`RUN-CLI-<s8>`）行为不变。

**B. 前端**（`assets/app/main.js`）

- `runSequence`／`compareSessionRunOrder`／`orderedSessionRuns`／`latestSessionRun`：不依赖 `/api/runs` 顺序，回归序号轴；
- `openConversation`：`state.runs.filter(...).reverse()` → `orderedSessionRuns(state.runs, s8)`；
- `syncLiveTail`：`runs.find(...)` → `latestSessionRun(runs, s8)`（序号最大者；无序号形态回退 mtime）；
- 测试钩子 `__testHooks` 增出四支（不新增 UI 面）。

**C. ARC 同一序号轴**：`ARC-<s8>-<n>` 的 n 就是归档发生时的会话计数，与 `RUN-` 同轴参与排序 ⇒ 归档事实落在正确时间位置（真机：`RUN-0 → ARC-1 → RUN-1 → … → RUN-23 → ARC-24`）。

**D. 撤回水位改取 `run_started`（两处）**：
- `openConversation`：窗口起点由 `ordered[0].modified_ms` 改为 `Date.parse(ordered[0].started) || modified_ms`。回档把 mtime 压到“回档时刻”后，旧口径会把**每次回档后的会话**都误判成「窗口缺撤回前段」（真机：mtime 20:19 > 撤回时刻 17:14），整条水位被跳过、被撤回内容复活并附一条警告。
- `syncLiveTail`：撤回后是否跳过旧 run 的比较，同由 `modified_ms` 改为 `Date.parse(mine.started) || modified_ms`（否则被撤回的旧 run 看起来像“撤回之后发生”，会被重新尾随重放）。
- `started` 来自 journal 首行 `run_started.timestamp`（`list_runs` 已探针读取），不受回档影响。

## §5 读数

| 门 | 命令 | 结果 |
|---|---|---|
| orz-web lib | `cargo test -p orz-web --offline`（`CARGO_TARGET_DIR` 指向可写目录，`PROTOC=orz/bin/protoc.exe`） | **76 passed / 0 failed** |
| 新钉 | `runs::tests::order_run_entries_breaks_mtime_ties_by_session_sequence` | ok |
| 前端冒烟 | `node tests/frontend_smoke.mjs` | **全部通过**（16.x 全绿＋新增 17.1/17.2/17.3） |
| 新钉 | 17.1 回放按序号铺开（含 ARC 同轴、跨会话不混入）／17.2 最新 run 不挂到 run 9／17.3 回档 mtime 不影响撤回水位 | ok |
| 真机 A/B | `/api/runs`（§2④） | 旧乱序 ↔ 新严格序号倒序 |

## §6 残余与不做

1. **mtime 未回写**（本批未闭合的根因余面）：归档包仍不记时间戳，回档后 `modified_ms` 对其余 mtime 消费者依旧失真——探索器归档判定（`a.modified_ms >= latestRunMs`）、跨会话“最新”排序、`isArchived` 分组。**撤回水位面已改取 `run_started`（§4D），不再受影响**。建议后续批在包内记 `mtime_ms` 并在 restore 后 `File::set_times` 回写（旧包缺字段＝跳过，向后兼容）。
2. **`27 次运行 · 未完成`**：24 个 `RUN-` ＋3 个 `ARC-` 全部计入组内计数；且 `RUN-c23e0aaa-1` 无终态事件（10-19 的幽灵重发 run，同 prompt 于 run 0）⇒ 永久“未完成”。属**计数／状态口径**问题，与本批排序修复正交，未动。
3. **载体未重建**：修复只在源码与开发构建验证；用户可见需 339 批双平台重建＋换装。
4. **无头浏览器边界**：本沙箱禁命名管道，Chrome 多进程 IPC 起不来（`--headless=new` 进程即退），故真机验证走 **API 级 A/B ＋ 前端冒烟**，未产出截图；S2 真机重跑留 339。
5. **不做**：改 `projection.js` 投影逻辑、改归档包格式（本批只登记，见 1）、清理旧 run 的 mtime。

## §7 测试映射

| 不变量 | 现有测试 | 备注 |
|---|---|---|
| 同会话 run 顺序与 mtime 无关 | `order_run_entries_breaks_mtime_ties_by_session_sequence` | 纯函数钉（Rust） |
| 回放按序号铺开、ARC 同轴、跨会话隔离 | `frontend_smoke.mjs` 17.1 | 真机 id 形状 |
| “最新 run”取序号最大者 | `frontend_smoke.mjs` 17.2 | 含 `RUN-CLI-<s8>` 回退 |
| 回档 mtime 不破坏撤回水位 | `frontend_smoke.mjs` 17.3 | 窗口起点取 `started` |
| 归档/回档后仍正确（端到端） | 无（需真机：回档 → 重开会话） | 留 339 S2 |
