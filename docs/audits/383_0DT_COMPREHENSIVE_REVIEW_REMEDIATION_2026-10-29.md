# 383 批：0dt 全面审查与补强——设计/实现/符合性三面检查，审查发现全数处理（2026-10-29）

> 日期：2026-10-29（用户令：「请对当前完成的0dt进行全面检查，包括设计合理性、实现合理性、设计与实现的符合性」；随后：「请对审查出的全部问题进行全面处理，直接都处理完吧，不单独立项了」）
> 性质：审查批＋补强落码——零重建、零换装、零 bump；**0dt 计数 69 不变**（0dt 保持开放：判据 4 真机复验仍随下次换装）。
> orz 子模块提交：`4d3f508b`（382 批 `b20ff8d7` 之上）。

## §0 总评

四子代理分端审查（loop/gateway 层、run 层＋LoopHost trait、orz-host 转发＋web 前端、TUI＋bin）互相独立、结论一致：**设计合理、实现质量高、382 批档声明与代码逐条相符且无虚报，四端均无缺陷级发现**。审查同时产出风险 3 项、测试缺口 3 项、备忘若干、记账勘误 2 处（§7 of 382 档）；本批按用户令全数处理，不立项。

## §1 审查方式与证据

- 逐层只读评审：loop/gateway（`stream_once_with_retry` reset 触发点/负空间/顺序保证/9 个网关实现者穷举）、run 层（Mutex 共享/Send 链/主车道 gate/abort 口径）、orz-host（线格式出处/fire-and-forget 同构/FIFO 保序）、web 前端（清卡保指针/跨会话过滤/P1-3 折叠链路）、TUI（同病确认/线格式端到端对齐/八处补臂）。
- 实跑核证：orz-loop 全量 970/0/3、host 新钉、web frontend_smoke、orz-tui bridge::tests 8/0＋view_model 13/0 均复现 382 读数。

## §2 发现与处置对照（全数处理）

| # | 发现（级别） | 处置 |
|---|---|---|
| 1 | **P1-3 sanitize 不对称**（风险，预存）：流式卡内容逐 delta 净化拼接，journal 原文 strict `===` 硬比——含 ANSI/C0 控制符的输出必失配回退双卡；web/TUI 同构 | **已修（sanitize 对称）**：web `projection.js` 与 TUI `projection.rs` 均以同一净化函数处理 journal 原文后再比（web 剥完整 CSI 序列、TUI 剔控制字符本身，两侧各自对称即折叠成立）；TUI 新钉 `model_output_with_escape_sequences_still_folds`，web 7.5 扩展钉 |
| 2 | **悬垂指针＋zero-chunk reset 交互**（风险，预存）：P1-3 失配后指针停在已收口 journal 卡上，下轮 zero-chunk 重试的 reset 误清上一轮可见答案；且新轮 delta 会追加到收口卡上（同卡拼接＝另一形态双卡病） | **已修（settled 收口卡不变量）**：`ChatMessage` 加 `settled` 字段／web 卡对象加 `settled`——journal 投影卡（`add_model_message`/`addModelMessage`/`addArchivedMessage`）＝true、流式追加卡＝false；TUI 建卡改 `begin_streaming_card`（收口卡拒绝复用）、web `ensureCurrentModelCard` 同语义收紧；`reset_streaming_card`/`resetStreamingCard` 拒绝落在收口卡上。TUI 钉×3、web 7.5 扩展钉 |
| 3 | **journal tail 与 ACP WS 跨通道毫秒级序竞态**（风险，预存）：`model_output`（journal WS）理论可越过末尾 delta（ACP WS）到达 | **定策：已消除腐损态、残余为设计内回退，登记为已知限制**——走查各分支后果：前缀补全会把「小残卡」恶化成「整卡重复」，弃用；settled 不变量已保证最坏＝独立小残卡（不腐损、不吞内容）；跨通道真正定序需协议级改造，与本缺陷比例失衡。代码注释＋本档登记 |
| 4 | **zero-chunk reset 触发无钉＋负向断言缺**（判据缺口） | **已补（reset 触发矩阵）**：zero-chunk EOF 钉 reset 恰一次；midstream 耗尽链钉「重发前恰一、终败后零」；退化守卫中断钉零 reset；取消钉零 reset（transport.rs 四处既有测试扩展） |
| 5 | **TUI 端 reset 零测试**（判据缺口；web 有完整钉而 TUI 空档） | **已补 8 钉**：view_model `reset_streaming_card` 语义×2＋`begin_streaming_card` 复用/freshen＋settled 标志；projection 全链场景（reset→重发→收口单卡）＋escape 折叠＋settled 存活场景；acp_client `ext_notification` 映射（`orz/stream_reset`⇒`StreamReset`、其余 no-op） |
| 6 | **前端钉未穿透分发/接线层**（判据缺口：acp.js 分支＋main.js 过滤各一行，笔误抓不住） | **已补（smoke 16.7b）**：`_orz/stream_reset` 线格式帧经 `currentAcpSocket().onmessage` 驱动——同会话清卡、异会话拒绝 |
| 7 | **空卡壳**（备忘）：reset 清空后到重发 delta 到达前（退避＋RTT 可达数秒）空壳可见；重试链最终失败时空壳永久残留 | **已修（渲染跳空）**：web `widgets.js` renderContent 与 TUI `widgets.rs` render_content_pane 跳过空正文模型卡（items 事实不动、重发照常原地重填）；web `scrollToItem` 以同一谓词 `isEmptyModelCard` 扣除隐藏卡寻址，锚点不漂移（smoke 12.5 钉）；TUI 空正文工具轮占位卡随之与 web 2026-09-25「不留空卡」口径对齐 |
| 8 | **F-06 注释漂移**（备忘）：reset 后「partial 不得丢失审计」已有意例外，旧注释误导后读者 | **已修**：delta 闭包与 abort 落账两处补注记（残段有意不入审计、仅 `transport_retry` 记重试事实；schema 零变更约束内的取舍不变） |
| 9 | **锁风格不一致**（备忘）：新三处 `.lock().unwrap()` 与同函数 `PoisonError::into_inner` 惯例不一致 | **已修**：三处统一为 `unwrap_or_else(std::sync::PoisonError::into_inner)` |
| 10 | **「运行期单线程」措辞**（勘误）：无争用靠 `&mut` 独占内联串行构造性互斥，非线程模型 | **已修**：`agent_loop.rs` 注释精校＋[382 档 §7](382_0DT_S1_STREAMING_CARD_RESET_IMPL_2026-10-29.md) 勘误 |
| 11 | **补臂数记账失准**（勘误）：批档「四处」vs 提交信息「七处」vs 实际 8 处 | **已勘**：[382 档 §7](382_0DT_S1_STREAMING_CARD_RESET_IMPL_2026-10-29.md)（提交信息不可改，随档勘误） |

## §3 读数

- orz-loop **970/0/3**（读数不变——reset 矩阵为既有四处测试扩展断言，无新测试函数）＋集成 **4/0**。
- orz-tui `--lib` 全量 **185 过/9 败**（9 败＝ACAF fail-closed 拒启动环境预存，382 §3.2 同败口径；`bridge::tests` **8/0** 批口径；`view_model` **17/0**＝基线 13＋新 4）；新 8 钉全过。
- orz-web frontend smoke **67 ok**（382 基线 64＋新 3：7.5 扩展／12.5 渲染跳空＋寻址映射／16.7b 分发穿透）。
- clippy 四 crate **持平零新增**（stash 对照：orz-loop 124=124、orz-tui 117=117）；fmt 四 crate `--check` 净。
- orz-host/orz-bin Rust 侧本批零改动；journal/schema 零变更；bridge 零改动。

## §4 边界与不做

- P1-3「精确相等」口径不变（净化对称后仍是严格相等，未改设计冻结的去重语义）。
- reset 语义不变：重发前恰一、指针保留、live-only、journal/schema 零变更。
- 跨通道毫秒级序竞态不做协议级定序（§2-3 定策）；TUI 标记列纯标签、无行寻址跳转，渲染跳空不引入漂移。
- 在役 0.9.14-4 仍不含 0dt 修复（382/383 均零重建零换装）；判据 4 真机复验随下次换装一并核证。
- TUI sanitize 与 web sanitize 的呈现分歧（TUI 剔控制字符保留 `[31m`、web 剥完整 CSI）为预存面，本批只保证**各自对称**（折叠判据两端一致），呈现统一不属本批。

## §5 入口

- 前序：[380 立项档](380_STREAMING_CARD_DUPLICATION_R37900_REGISTRATION_2026-10-28.md)／[382 实施档](382_0DT_S1_STREAMING_CARD_RESET_IMPL_2026-10-29.md)（§7 勘误）。
- 关键源位：`orz-tui/src/view_model.rs`（settled＋begin_streaming_card＋reset 守卫）／`orz-tui/src/projection.rs`（P1-3 净化对称＋建卡改道）／`orz-tui/src/widgets.rs`（渲染跳空）／`orz-tui/src/acp_client.rs`（映射钉）／`orz-web/assets/app/{state,projection,widgets,main}.js`／`orz-web/tests/frontend_smoke.mjs`（7.5 扩展＋12.5＋16.7b）／`orz-loop/src/agent_loop.rs`（注释/锁）／`orz-loop/src/gateway/transport.rs`（reset 矩阵）。
