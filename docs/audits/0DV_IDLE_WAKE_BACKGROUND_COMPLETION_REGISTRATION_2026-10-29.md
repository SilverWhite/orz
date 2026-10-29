# 0dv 立项登记：后台完成闲置唤醒（BG-DONE-IDLE-WAKE）（2026-10-29）

> **性质**：立项（计数 68 → 69；P1；S1 设计待排）。**来源**：用户实测报告「当前运行完成后，后台进程结束后不会再将结果回报给模型，这样也就是没有新运行了」＋用户令「请进行立项并登记吧」。勘定结论＝**结构性缺口，非回归**——与 0.9.14-4 换装及 0du 无关。

## §1 判定：run 外唤醒路径从未存在

- **设计权威自认**：即时回报设计（[0ac 设计稿 §1 问题 2](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md)，2026-09-13）明载——「后台任务完成……机械层没有『结果一出现就作为新的结果动作回报』的通用路径；D7 的『下一次工具结果带回』是 2026-08-28 为**不新增唤醒**而复用的既有机制，**不是理想形态**」。落地的投递边界仅 B1（工具结果）／B2（轮结束），**全部在 run 循环内**；B3（生成中）／B4（pending 工具）亦为 run 内形态。不存在「会话空闲」边界。
- **代码面坐实**：`drain_completed_tasks` 全仓仅两处调用（`agent_loop.rs` B1 工具轮间隙 / B2 终答候选），均在 run 循环内；`orz-web` 桥对后台任务零感知；`orz-host` 仅实现被动 drain（`lib.rs` `drain_completed_tasks`），无任何观察者/唤醒路径。
- **299 批只修了前半**（2026-10-18）：此前 run 尾 teardown `shutdown_all` 无差别杀光后台任务（「跨 run 存活被第二杀手结构性否决」）——进程活不到跑完；修复后会话级 terminal backend 使任务**跨 run 存活、能跑完**，但完成事实无人观察。同批已登记「教学文案承诺假不变量」家族；bash 工具描述至今承诺 *"you are notified on completion, so do not poll or sleep-wait for it"*——「完成通知」这半句在 run 外窗口仍为假承诺。
- **生产开关休眠**：B1/B2 机械投递受 `ORZ_IMMEDIATE_RESULT_DELIVERY`（缺省关）＋M1 子开关（`m1_enabled()`＝主开关∧子开关，均缺省关）双重门控；用户 env 未设 ⇒ 连 run 内机械投递通道也未启用。现行 run 内可见的后台完成仅靠工具级提醒（`surface_bg_completion_reminders` 缺省 true，工具结果顶部）。
- **现场时间线（会话 224b39bd）**：RUN-224b39bd-11（03:30–03:51）终盘将全量 MD 模拟转后台（seq 414「relaunching full…」），`run_finished` 03:51:41；进程约 03:51–04:07 间跑完，无人观察；04:07 RUN-12 为用户手打 prompt（「话说接上LIF的话就需要用CPU了吧」）。RUN-11/12/13 journal 零 `result_delivered` 事件。

## §2 设计草案（S1 细化基线）

1. **观察面**：会话域观察者挂在宿主侧后台任务注册表所在层（299 后的会话级 terminal backend／`StoredSession` 域）——任务终态产生处即观察处；run 结束不撤观察者（与任务同生命周期：session 结束／用户停止／30h 安全帽）。
2. **触发面**：会话 idle（无在途 run、无在途 prompt）且 drain 出完成事实 ⇒ 经既有会话命令通道自动开新 run；owner/lease 口径沿 0dk（bridge 只做传输，owner 在 child）。**在途 run 不打断**（结果走既有 B1/B2 边界）；唤醒 run 为真 run（usage metering／journal 全链照走）。
3. **投递面**：唤醒 run 开局注入一条机械事实消息 `[结果投递]`（复用 `immediate_delivery` 渲染与 delivery_queue 合并语义，多任务同时完成合并一条）；投递落 `result_delivered` 事件，`boundary` 闭枚举**新增一位**（如 `idle_wake`——schema 契约面变更，随 S1 走 registry/fixture/法官全链）。
4. **开关拓扑**：闲置唤醒独立子开关（从属主开关）；`ORZ_IMMEDIATE_RESULT_DELIVERY`／M1 子开关缺省翻案随批裁决（见 §3）。
5. **教学文案**：唤醒落地后 bash 工具描述"notified on completion"在 run 外窗口**坐实**；若裁决不做唤醒则改口。S1 一并盘点其余"假不变量"残句。
6. **边界**：① 多任务完成合并投递；② 唤醒 run 受 999 轮／墙钟等既有约束；③ owner 进程死亡后注册表消失＝既有跨进程持久化缺口（318 验收⑤「跨进程恢复待续」），唤醒不跨进程、不新增承诺；④ web 投影单卡收口依赖 0dt 流式重复卡修复（排期 382 批），不阻断本项。

## §3 随批裁决点（S1 定稿时收口）

| # | 裁决点 | 备注 |
|---|---|---|
| 1 | `ORZ_IMMEDIATE_RESULT_DELIVERY` 缺省是否翻 `on` | 不翻则唤醒了也无投递通道；B1/B2 亦随之激活 |
| 2 | M1 子开关缺省 | 从属主开关；两开关是否合并单键 |
| 3 | 唤醒 run 的提示面形态 | 纯 `[结果投递]` 机械事实 vs 附简短引导语（「后台任务已完成，结果如下」） |
| 4 | 唤醒去抖／风暴保护 | 多短任务连发完成的合并窗（delivery_queue TTL 沿用或独立） |

## §4 S4 判据草案

后台任务在 run 外完成 ⇒ N 秒内（TTL 去抖后）自动新 run，journal 见 `result_delivered`（boundary=`idle_wake`）且模型面收到完整结果；在途 run 不被打断（B1/B2 照旧）；web 投影不双卡（依赖 0dt 修复）；开关关时零唤醒零事件（零行为变化）。

## 入口

- BACKLOG `0dv`（开放项计数 69）／TODO `0dv` 条目／索引头 v4.343。
- 关联：`GAP-MECH-IMMEDIATE-FEEDBACK`（0ac，B1/B2 闭合档 [`098`](098_0AC_S4_CLOSURE_2026-09-27.md)）／299 批（跨 run 存活修复与「教学文案承诺假不变量」登记）／0dt（流式重复卡，382 批）。
