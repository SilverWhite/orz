# 395 批：0dx 活体验收（部分达成）＋Web 面缺口归 0dx＋0dy 立项（2026-11-07）

> 日期：2026-11-07（用户首启真机使用测试后令「请进行登记并直接立项吧」）
> 性质：验收＋登记立项批——0dx 活体判据核证（392 §5／394 §4 预留）＋两真机缺陷登记＋**0dy 立项（计数 71 → 72）**；零源码、零重建、零换装。
> 现场：会话 `6af001b9`（工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹`，2026-10-30 创建、11-07 21:22 于 0.9.14-6 续用），栈＝bridge `orz web --real` PID 22124 → child `orz --stdio` PID 22296（父＝bridge）→ signer PID 13860，全部出自装机位 `%LOCALAPPDATA%\orz\bin`（21:22:05/08/09 启动）。

## §0 结论

**0dx 时代机制核心活体面六件全部在真实使用中自然触发并逐位核证通过**：era_boundary journal 事件×2、时代折叠块落盘×2（SHA-256 与 journal digest 逐位一致）、对话内时代 marker×2、工作窗重置×2（首 face_fingerprint 均 3 msgs）、黑板模型分区清空（`board_entries_removed` 1/2）、无前缀输入换代（idle 路径×2）。装机位三件实测哈希与 392/394 账面逐位一致＝在役即 0.9.14-6，bridge/child 同文件 ⇒ version_skew 结构性 false。**两处命令面判据（`/checkpoint` 回执、`/btw` 排队）在 Web 上不可达＝缺陷而非未触发**（§2，归 0dx 范围）；Web 输入历史方向键劫持＝第二缺陷（§3，**0dy 立项**）。0dt 判据 4 本轮无 midstream 重试未触发，保持开放。S4 首批读数已收（§4），形态优于历史基线；阈值裁定继续待数。

## §1 0dx 活体验收证据（394 §4 清单逐条）

| 判据 | 结果 | 证据 |
|---|---|---|
| bridge/child 0.9.14-6、skew=false | ✅（进程级＋哈希级） | 运行进程树全部出自装机位；装机位三件实测 `orz.exe 1daa8706…`／signer `cd5cb7ec…`／provision `df2c2871…` 与 392/394 账面逐位一致；bridge 与磁盘 child 同一文件。保留＝`/api/boot` 直查需 per-boot token（仅 URL fragment 传递、盘面无落），Web 工具栏横幅可由用户一眼补证 |
| `era_boundary` journal 事件 | ✅ ×2 | RUN-2 seq4（era→1，boundary_round=43，`trigger=unprefixed_input`，21:25:38，`write_failed=false`，`board_entries_removed=1`）＋RUN-3 seq4（era→2，boundary_round=66，21:31:55，`write_failed=false`，`board_entries_removed=2`，usage＝24 calls／853,687 prompt tokens——era 用量跨 run 累计入账；era0 存量系 0dx 前轮次无账可入故为 0） |
| 折叠块 `.gsa/compaction/blocks/{tag}-era-{n}.md` 落盘 | ✅ ×2 | `6af001b9-era-001.md` 397,862 B digest `8ec0b36e…`、`-era-002.md` 558,904 B digest `a519224a…`，实测 SHA-256 与 journal `archive.digest_sha256` 逐位一致；两块均 `write_failed=false` |
| 时代 marker 入对话 | ✅ ×2 | v0.4 同构全要素（时代折叠／边界轮／存档路径／digest／边界时间戳），与 journal 逐位对应 |
| 工作窗重置 | ✅ ×2 | 两 run 首个 `face_fingerprint` 均 `model_round=1, message_count=3, total_chars≈960` |
| 黑板模型分区随时代清空 | ✅ | `board_entries_removed`＝1（era1 边界）/2（era2 边界） |
| 无前缀输入换代 | ✅ ×2（idle 路径） | 两次边界均由无前缀新输入自然触发（用户当日两条真实 prompt），非构造场景 |
| `/checkpoint`（idle）切换回执 | ❌ **Web 不可达**（§2） | 全会话 0 次；TUI 侧在位 |
| `/btw` 排队同代 | ❌ **Web 不可达**（§2） | 全会话 0 次、`fold_refused_same_era`=0；用户实试 `/btw` 被前端白名单吞 |
| 0dt 判据 4 活体单卡 | ⚠ 未触发 | 双 run 0 次 `stream_reset`＝无 midstream 重试，复位协议未被活体命中；UI 卡面盘面不可证，保持开放 |

附注：双 run 均 `run_finished`、零失败面零取消、journal 哈希链（`previous_event_sha256`）连续；会话 lease/owner 完好。RUN-1（10-30）journal 目录不在 `runs/`＝retention 7 天设计内清扫（`orz-host/src/retention.rs` A5 `DEFAULT_RETENTION_DAYS=7`），handoff 幸存，**非缺陷**。run_preflight 无载体版本章＝0dw 缺口③在体如登记（未修复，口径一致）。

## §2 缺陷一（归 0dx 范围，不占新计数）：时代协议 Web 面未接线

- 现象＝用户在 Web 输入 `/btw` 得「未知命令 /btw（见 /help）」、实质不可用（用户反馈原话「/btw显示无对应命令，实质不可用」）。
- 根因＝`orz-web/assets/app/main.js`：`submitFromInput()`（L1830-1833）把**所有 `/` 前缀文本**先送前端白名单 `runSlash()`（L1755-1808，仅 /help /status /stop /rewind /open /toggle-* /properties /snapshots /grill 十二项）；`/checkpoint`／`/btw` 不在册 → default 臂 flash「未知命令」且 `runSlash` 无条件返回 true → **输入框被清空、文本被本地吞掉、根本不到宿主**（宿主 `orz-loop/src/era.rs` 分流完整在位）。AddressBar 路径（L1741 `runAddress`）同门。连带＝手动时代切换与同代补充在 Web（0dk 裁决的唯一交互面）双不可达，且被吞时已敲文本一并丢失。
- 波及面对照＝**TUI 在位**：`orz-tui/src/input.rs:133-140` 按 391 词界规则镜像预过滤（`/btw` 前缀与 `/checkpoint` 带参直通 prompt、裸 `/checkpoint` 走 `orz/checkpoint` ext 面，测试钉在）。
- 归因＝390 S2 落码五件只接 loop＋TUI 两端、391 处置补 TUI 镜像钉，Web 前端无对应改动；392 在体字面量核证数宿主侧字符串测不出此断线；0dn 式真机套件未为 0dx 加 Web `/checkpoint`//btw` 场景——S4 合并核证窗口由此漏网。
- 定性改写＝本验收 §1 中两判据「未触发」实为「**触发不了**」；0dx 活体面两判据在 Web 面属实质不可达，修复前 0dx 活体面不可收口。
- 处置＝登记入 0dx 既有开放范围（沿 391 先例：开放项自身缺陷处置不占新计数），落码随 0dy 修复批承载（§5）。

## §3 缺陷二 → 0dy 立项：Web 输入历史方向键劫持

- 现象＝用户反馈「怎么下方向键被绑定成回到上一轮输出了？这个太别手了」。
- 根因＝`main.js:1847-1861`：输入历史导航中 `ArrowUp` 有「仅输入框为空」守卫（L1851），但 `ArrowDown` 仅凭 `historyIndex != null` 触发（L1855）——**无草稿编辑失效、无光标行边界守卫**；`historyIndex` 仅提交时复位（L1841）。上翻一次后任何编辑中按下键（多行草稿移光标必踩）即被历史条目整框替换＝「回到上一轮输出」，且与 shell 惯例（编辑即失效历史游标）相悖。
- 修复草图＝①`chatInput` `input` 事件复位 `historyIndex`（用户编辑即失效历史导航）②Up/Down 加光标首/末行边界守卫（多行文本内保原生光标移动）③回归钉。
- 立项＝**0dy**（71 → 72，本批），与 0dx Web 面补线并一案、一个修复批收口（用户裁决「请进行登记并直接立项吧」承接主会话「两缺陷并一案登记…一个修复批收口」提案）。

## §4 0dx S4 首批收数（先收数后定阈）

- 历史基线（逐用户轮工具调用数）：51da03ea 71→30→20→16→7→6／073b8892 30→36→12→9→6／224b39bd 17→4→3→1；同形＝开局高、逐轮塌缩至近零、`blackboard_read` 全程为零或早期归零。
- 本轮首批（0.9.14-6 真机）：era1 首用户轮 **34** 次触达（23 模型轮）；era2 首用户轮 **109** 次（97 模型轮）；**逐模型轮稳定 1–2 次/轮无塌缩**；黑板协议两时代均活跃（era1 write×2＋read×1；era2 write×3＋read×2＋self_check×1）——历史「blackboard_read 归零」形态未复现。
- 边界＝每时代现仅 1 个用户轮，时代内首末轮衰减尚不可测；逐调用 provider 用量未落 journal（0dw S1 未落码）⇒ 精确对账缺位。本读数即首笔样本，阈值裁定继续待数（依赖 0dw 对账面）。

## §5 处置与批序（0dy）

- **S1 落码两件随下一源码批**：①0dx Web 面补线＝`submitFromInput`/`runAddress` 按 391 词界规则把时代协议形态直通 prompt（`/btw` 前缀照发宿主剥前缀解析、`/checkpoint` 裸/带参照发），`/help` 补条目，web smoke 加镜像钉（沿 391 TUI 镜像先例）；②输入历史修复（§3 草图）。
- **S2 载体重建换装 0.9.14-7**（web 资产编译期进体，与后续源码批同车）。
- **S3 真机复验合并窗口**＝0dx 活体两判据（`/checkpoint` 切换回执、`/btw` 同代排队）＋0dt 判据 4（活体单卡，理想含一次 midstream 重试场景）＋方向键手感——四项同窗核证，达成即 0dx 活体面收口＋0dt 触发 69 → 68 待裁决。
- 眼下绕行＝无前缀切换活体可用（缺省路径，本验收实证）；需不换代补充暂走 TUI（`/btw` 在位）。

## §6 入口

- 0dx 设计权威 [`CONTEXT_CHECKPOINT_ERA_DESIGN v1.1`](../CONTEXT_CHECKPOINT_ERA_DESIGN_2026-10-30.md)／392 批档（§5 预留判据）／394 批档（§4 用户首启清单）／390/391 批档（落码与处置）。
- 证据面：会话 `6af001b9` runs/journal／`.gsa/compaction/blocks/6af001b9-era-{001,002}.md`／装机位三件实测哈希；代码面 `orz-web/assets/app/main.js`（L1741/1755-1808/1830-1861）／`orz-tui/src/input.rs`（L133-140）／`orz-loop/src/era.rs`／`orz-host/src/retention.rs`（A5）。
- 台账：TODO 395 行／BACKLOG 本批记录指针／索引头 v4.355；0dy 明细见 TODO P1-0dy／BACKLOG 开放项行。
