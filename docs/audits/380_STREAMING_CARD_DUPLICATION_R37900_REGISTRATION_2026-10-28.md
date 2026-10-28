# 380 批：R-379-00 流式重复卡缺陷登记与修复排期（0dt 立项；仅登记，零代码）

> 日期：2026-10-28
> 用户令：「请先明确登记这一问题并排期修复吧，实际数据层没问题就好。不用考虑什么最小修复，按照最优修复走」
> 性质：**登记批**——缺陷立项＋排期＋修复设计定稿；本批零代码、零重建、零换装。
> 计数：**66 → 67**（0dt 立项）。

## §0 结论

379 批换装后首次活体外检索会话暴露两个观察，经父仓侧全面取证定性：

1. **R-379-00（本批立项，代号 `0dt STREAMING-CARD-RESET`）**：web UI 内容面终盘出现**同一主车道终答的两张卡**（流式卡＝重试残段＋重发全文拼接；journal 收口卡＝完整终答），仅收口卡带 `[RUN_END]` 尾标记。**数据层干净**（journal 与会话侧车各恰一条完整终答；loop 交付面正确），重复纯发生在 UI 流式投影面。根因＝midstream 传输重试同请求原样重发、delta 通道无重置协议、projection.js P1-3 去重仅精确相等折叠——三环节叠加。修复走**协议修**（用户裁决，弃前端覆盖式最小修）：传输重试时向 delta 通道发流式重置控制消息，UI 清卡后只收重发流。**排期 381 批**（2026-10-29 可执行；不阻断 M1.5 11-07／M2 11-10 里程碑）。
2. **终答带裸 `[RUN_END]`（不立项，用户自行处置）**：触发源＝用户工作区自己的协议文档（`self_check_protocol.md` 等，开局读入消息明示「输出带 `[RUN_END]`」）；loop 声明解析只识别完整闭合声明块（`[RUN_END]`＋`intent:`/`reason:`＋`[/RUN_END]`），裸标记不识别 ⇒ 按 353 §5.1 fail-open 原文交付（`terminal_source=text_without_terminal`），行为符合 0dm/353/355 既定口径。用户改工作区协议为完整声明形态后即走 `strip_model_stop` 剥离，零代码项。

另：检索子代理内环终盘文本随父 run journal 入链并渲染为独立模型卡（本例 540 号「本批检索结果」卡）——经用户确认**属正常形态**（子代理 delta 本就不外推，仅 journal 尾随投影；标记栏不标中间卡亦是正常语义），不立项。

## §1 取证（会话 `224b39bd`，run `RUN-224b39bd-0`，2026-10-28 23:12）

- 版本面：在役栈确认 **0.9.14-4** 无 skew——web＋child 均为装机位换装后二进制（68,385,792 B，22:15 落位），signer 载体位；会话 22:22 启动晚于 379 换装。
- run 正常收尾：`run_finished{status=completed, tool_rounds=59, terminal_source=text_without_terminal, terminal_nudged=false, self_check_called=true}`；终答完整交付。
- 数据面单一性：journal `model_output` EVT-…-686 与会话侧车消息 [126] 均为同一 10,859 字符终答，**各恰一条**；无重复落账。
- 触发事件：EVT-…-685 `transport_retry{agent_role=main, outcome=recovered, kind=midstream, retries=1, reason="error decoding response body"}`（23:12:15，与终答同秒）。同 run 另有两次 midstream 重试（65／557 号），但均为纯工具轮（text_len=0）不产卡，故无症状。
- UI 面：终盘两张模型卡——流式卡（残段＋重发全文拼接）＋收口卡（journal 全文，尾带裸 `[RUN_END]`）；`anchorFinalOutput` 锚定最后一张（收口卡），符合正常锚语义。

## §2 根因链（三环节）

1. **transport 层**：midstream 重试语义＝**同一请求原样重发**（`gateway/transport.rs`「wrapper re-sends the IDENTICAL request body」；midstream 有界 1 次）。重发流的 content delta 继续从**同一个 delta 回调**进入 host（`agent_loop.rs` run_round 的 `&mut |chunk|`）。
2. **host 层**：`on_text_delta` 把主车道 delta 以 `agent_message_chunk` ACP 通知 fire-and-forget 直推 UI（live-only、不进 journal、headless 丢弃）。**重试发生时无任何「清空重来」控制消息**——重试前残段与重发全文叠进同一张流式卡。
3. **前端**：projection.js `model_output` 分支的流式去重（P1-3 口径）只在「流式卡内容 == journal 全文**精确相等**」时折叠指针；发散即按「工具轮后的下一段输出」**新开卡**。残段≠全文 ⇒ 对账失败 ⇒ 第二张卡。
   - 注：`transport_retry` journal 事件在**轮结束后**才落账（`run_round` 返回 Ok 后 record），前端拿它做事中重置来不及——这是「前端覆盖式兜底」不可靠的结构性原因之一，亦是协议修（事中控制消息）被选为最优的根据。

## §3 最优修复设计（0dt，381 批执行）

**目标语义**：传输重试后，UI 流式卡内容必须与最终 journal 收口文本一致（单卡收口）；任何导致流式/journal 发散的源头（midstream／zero-chunk 重试）统一由流式重置控制消息收敛。

| 层 | 改动 | 落点 |
|---|---|---|
| loop/gateway | delta 回调管道从纯 text chunk 扩展为可承载控制信号（`enum {Delta, Reset}` 或并行控制面）；midstream/zero-chunk 重试**重发前**发 `Reset` | `gateway/transport.rs` 重试 wrapper 决策点＋`agent_loop.rs` run_round 回调签名 |
| host | 新增 `on_stream_reset()`：转发 `orz/stream_reset` ext notification 给 ACP client（fire-and-forget 同 `on_text_delta` 语义；headless 静默丢弃） | `orz-host/src/lib.rs` |
| web 前端 | `applyAcpUpdate`／ext 通知处理新增 reset 分支 → `vm.resetStreamingCard()`（清空 currentModelIndex 卡内容，指针保留）；重发 delta 重新填充；journal `model_output` 到达时精确相等恢复 ⇒ P1-3 去重折叠，单卡收口 | `orz-web/assets/app/acp.js`＋`projection.js`＋`state.js` |
| TUI | bridge 投影同用 P1-3 去重口径，S1 核查同病同修 | `orz-tui`（随批取证） |

**边界**：
- journal/schema **零变更**——reset 是 live-only 控制面（与 F-03 text_delta「不进 journal」先例一致）；重试事实已有 `transport_retry` 事件可观测。
- 不做前端覆盖式兜底（用户裁决弃最小修）；P1-3 精确相等去重口径不变。
- 数据层（journal/侧车/交付面）本批已证干净，**不动**。

**测试面**（381 批验收判据）：
1. loop transport 钉：midstream 重试路径必须先发 Reset 再重发（fake 网关断言信号序）。
2. host 钉：`on_stream_reset` 转发 ext notification；headless no-op（沿 4049 测试形）。
3. 前端 projection 单测：delta×N → reset → delta×M → `model_output` ⇒ 单卡且内容==journal 全文。
4. 真机复验：复现 midstream 重试（或注入 fake 重试）后终盘单卡；`transport_retry` 事件仍落账可观测。

## §4 排期

- **381 批（下一批）**：0dt S1 落码（loop＋host＋web 前端三层）＋测试面；目标 **2026-10-29**。
- S2 真机复验随 381 批或下次换装核证顺带；不阻断 M1.5（2026-11-07）／M2（2026-11-10）/M3（2026-11-17）里程碑。
- 在役 0.9.14-4 不含本修复——症状在传输重试轮偶发，容忍至 381 修复进体换装；无需回滚。

## §5 入口

- 取证会话：`C:\Users\1\OneDrive\Desktop\新建文件夹\.gsa`（`224b39bd`／`RUN-224b39bd-0` events 539-542／685-688、conversations `224b39bd.json` [126]）
- 关键源位：`orz-loop/src/gateway/transport.rs`（midstream 重发）／`orz-loop/src/agent_loop.rs:3638-3656`（delta 回调＋重试落账）／`orz-host/src/lib.rs:2310`（on_text_delta）／`orz-web/assets/app/projection.js:92-116`（P1-3 去重）／`orz-web/assets/app/state.js:229-241`（appendTextDelta）
- 前序：[`379 换装`](379_SWAP_V0914_DASH4_WIN_2026-10-28.md)／[`377 挂账`](377_M1_V1_OPEN_ITEMS_2026-10-28.md)
