# 382 批：0dt S1 落码——流式重复卡最优修复（协议修：重发前流式重置，三层四端）（2026-10-29）

> 日期：2026-10-29（用户令「请进行0dt吧」）
> 性质：实施批——0dt（`STREAMING-CARD-RESET`，380 批立项）S1 全量落码＋测试三钉；零重建、零换装、零 bump（在役 0.9.14-4 不含本修复，症状偶发容忍至下次换装，380 §4 既定口径）。
> 计数：**69 不变**（0dt 保持开放：S2 真机复验随下次换装核证顺带）。

## §0 结论

R-379-00（web UI 同一终答双卡：midstream 重试残段＋重发全文拼接的流式卡 vs journal 收口卡）按 380 §3 最优修复设计落码——**协议修**：传输层重试**重发前**向 delta 通道发流式重置控制信号，UI 清卡后只收重发流，journal 收口时 P1-3 精确相等去重折叠，单卡收口。loop/gateway＋orz-host＋orz-web 前端＋orz-tui 四端同批同修（TUI 同病同修为 380 §3 既定范围）；bridge（orz-web Rust 侧）零改动（帧透明转发）。journal/schema **零变更**（reset 是 live-only 控制面，与 F-03 text_delta「不进 journal」先例一致；重试事实已有 `transport_retry` 事件）。

## §1 落码清单（四端）

| 端 | 改动 | 落点 |
|---|---|---|
| loop/gateway | `generate_stream`／`run_round` 回调面新增并行控制通道 `on_reset: &mut (dyn FnMut() + Send)`（380 §3 两案中取「并行控制面」案：既有 30+ 处 `|chunk|` 测试闭包零扰动）；`stream_once_with_retry` 重试循环在**退避后、重发前**发 `on_reset()`（midstream/zero-chunk 统一触发：zero-chunk 时卡内无本请求内容，重置为无害 no-op；混合序列幂等）；`stream_once` 透传不自发 reset；退化守卫中断不重试不 reset；缺省缓冲实现（`model.rs` trait default）单块整发从不重试——忽略 reset | `orz-loop/src/gateway/{model,transport,fake}.rs`＋`agents/{main,retrieval}.rs`＋`agent_loop.rs`（RoundAgent trait＋run_round 调用点）＋`controller.rs`（三个测试替身） |
| loop/run 层 | 重置闭包语义：`partial_text` 改 `std::sync::Mutex<Vec<_>>` 共享（delta/reset 两闭包并行传参、回调面 `FnMut + Send` ⇒ 捕获须 Send，`RefCell` 不满足；运行期单线程无争用）——重置时**清空 partial 累计**（abort 路径 journal partial 只含重发流，与 live 卡同口径，不拼残段）＋主车道调 `host.on_stream_reset()`（子代理/headless no-op） | `orz-loop/src/agent_loop.rs:3422,3650-3666,3778` |
| loop/host trait | `LoopHost::on_stream_reset(&self)` 缺省 no-op（与 `on_text_delta` 同纪律：live-only、不进 journal、fire-and-forget） | `orz-loop/src/host.rs:1026` |
| host | `OrzHost::on_stream_reset`：转发 `orz/stream_reset` ext notification（线格式 `_orz/stream_reset`，params `{sessionId}`）给 live ACP 客户端；dropped receiver 静默降级（debug 留痕）；headless（无 gateway/无会话）直接丢弃 | `orz-host/src/lib.rs:2329` |
| web 前端 | acp.js handlers 增 `streamReset` 槽＋`_orz/stream_reset` 分支；state.js `vm.resetStreamingCard()`（清空 currentModelIndex 卡内容、指针保留，重发 delta 原地重填；无卡 no-op）；main.js `acp.on('streamReset')` 与 update 同一跨会话过滤（只清当前绑定/恢复目标）后清卡＋scheduleRender | `orz-web/assets/app/{acp,state,main}.js` |
| TUI | `TuiClientHandler::ext_notification` 覆写：`orz/stream_reset` ⇒ `ClientMsg::StreamReset`（其余 ext 通知维持缺省 no-op）；view_model `reset_streaming_card()`（web 同语义）；runner 主循环 StreamReset 臂清卡；另四处穷举 match 补忽略臂（runner×4＋acp_client×3＋orz-bin×1） | `orz-tui/src/{acp_client,view_model,runner}.rs` |
| 桥 | **零改动**——帧透明转发；`_` 前缀 ext 通知（agent-client-protocol 0.10.4 线格式约定）直达浏览器 | — |

## §2 测试面（判据 1–3 全过；判据 4 真机随下次换装）

1. **transport 信号序钉**：`generate_stream_midstream_retry_resets_stream_before_resend`——midstream 截断→重试成功场景，信号序断言 `Delta("half"), Delta("-done"), Reset, Delta("ok-after-retry")`（reset 严格介于首试残段与重发流之间、恰一次）。
2. **host 钉**：`on_stream_reset_forwards_ext_notification_to_gateway`——有 bridge 时收到 `AcpClientMessage::ExtNotification{method:"orz/stream_reset", params.sessionId}`；headless no-op（沿 on_text_delta 测试形）。
3. **前端 projection 单测**（frontend_smoke §7.5）：delta×2 → reset → delta×1 → `model_output` ⇒ 单卡且内容==journal 全文、指针复位；无卡 reset 零副作用。
4. **真机复验**：随下次换装核证顺带（复现 midstream 重试后终盘单卡；`transport_retry` 仍落账可观测）——0dt 保持开放至该判据。

**读数**：orz-loop **970/0/3**（基线 969＋新钉 1）＋集成 4/0；orz-host 串行 **454/0/6**（基线 453＋新钉 1，`RUST_MIN_STACK=134217728`＋`--test-threads=1` 本机常设口径 FR-N03）；orz-web frontend smoke **64 ok**（基线 63＋新钉 1，`node tests/frontend_smoke.mjs` 全过）；orz-bin 六套件全绿（24/12i＋2＋15＋24＋2＋1，新指针口径）；orz-tui `--lib bridge::tests` **8/0**（377 同款口径）＋view_model 13/0；clippy 四 crate **186＝基线持平零新增**；fmt 四 crate `--check` 净。

## §3 环境口径三则（非代码缺陷，随批取证）

1. **orz-host 套件须 `RUST_MIN_STACK=134217728`**：不带时 `checkpoint_survives_tool_round_when_run_fails_before_terminal_persist` 栈溢出（0xC00000FD）——**干净树（stash 对照）同样溢出**＝预存环境口径非本批引入；FR-N03 常设口径（0AP/0AM 批档先例）即含此值，本机此前批次均带大栈跑。
2. **orz-bin/orz-tui 套件须新 ACAF 指针**：本会话 shell 继承 0du 前旧死指针（`D:\tb-eval\orz-windows\…` 已删）⇒ 子进程启动即 `acaf: signer spawn failed (os error 3)` fatal（stdio_e2e EOF）；带 0du 新根三指针（`%LOCALAPPDATA%\orz\…`）后全绿。这正是 0du S1「只对新起进程生效」的活体注脚。orz-tui 其余 9 个 in-process 失败（ACAF fail-closed 拒启动）**干净树同败**＝预存、非批口径（377 起批口径＝`bridge::tests` 8/0），不在本批处置。
3. **父仓门禁五个预存 error**（本批前已红，HEAD 381 即然）：TODO 路由缺 0dt/0du/0dv、计数 66≠69（380/381 登记批未同步 TODO）、BACKLOG 13/25 两行超长、子模块脏——前三者随本批落账收口（见 §5），超长行随本批指针行收缩消一行，子模块脏随本批提交消除。

## §4 边界与不做

- journal/schema 零变更；数据层（journal/侧车/交付面）不动（380 已证干净）。
- 不做前端覆盖式兜底（用户裁决弃最小修）；P1-3 精确相等去重口径不变。
- 缺省缓冲 gateway 路径、退化守卫中断路径、用户取消路径：零行为变化（不重试即不 reset）。
- 在役 0.9.14-4 不含本修复，无需回滚；下次换装（版本划界随重建批）进体。
- 0dv（闲置唤醒）不受影响：其 §2 边界④「web 投影单卡收口依赖 0dt」的本批部分已就绪。

## §5 同批顺带（零代码面）

- **0du S2 活体复验证据达成（闭合待用户裁决，计数不变）**：现役栈已于 2026-10-29 02:40:34 重启（bridge PID 16640＋child 14716，均自装机位 `%LOCALAPPDATA%\orz\bin` 起，晚于 0du S1 env 改动）——注册表 User 域三指针读回＝新根；会话 `224b39bd` 重启后 RUN-11～RUN-18 `control_ticket_rejected` 全零、`control_ticket_issued/consumed` 正常配对（RUN-18 各 6 次，最后事件 19:34）、`signer_unreachable` 零残留 ⇒ 0du S2 判据（票据道恢复）已由活体 journal 坐实。0du 登记档补 §7。闭合（69 → 68）待用户裁决。
- 父仓门禁账本一致性收口：TODO 计数 66→69 对齐＋P1 路由补 0dt/0du/0dv＋382 要点行；BACKLOG 本批记录指针换头并收缩至 ≤1200 字符（超长告警消一行；:25 计数行超长为预存、未动）。

## §6 入口

- 前序：[380 立项档](380_STREAMING_CARD_DUPLICATION_R37900_REGISTRATION_2026-10-28.md)（根因链/最优修复设计/测试判据）／[0du 登记档](0DU_ACAF_KEYSTORE_ROOT_EXIT_TB_EVAL_REGISTRATION_2026-10-29.md)（§7 S2 证据）。
- 关键源位：`orz-loop/src/gateway/transport.rs`（stream_once_with_retry 重试循环 on_reset 决策点）／`orz-loop/src/agent_loop.rs`（run_round 双闭包＋partial Mutex 共享）／`orz-loop/src/host.rs`（LoopHost::on_stream_reset）／`orz-host/src/lib.rs`（OrzHost 转发）／`orz-web/assets/app/{acp,state,main}.js`／`orz-tui/src/{acp_client,view_model,runner}.rs`。
- orz 子模块提交：见父仓 382 批提交信息（本批先提交 orz 再提交父仓）。
