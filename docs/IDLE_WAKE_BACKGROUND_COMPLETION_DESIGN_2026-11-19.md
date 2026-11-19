# 0dv S1 设计定稿：后台完成闲置唤醒（BG-DONE-IDLE-WAKE）（2026-11-19）

> **性质**：S1 设计定稿批（402 批；零产品码零跑批；计数 73 不变）。**来源**：[0dv 立项档](audits/0DV_IDLE_WAKE_BACKGROUND_COMPLETION_REGISTRATION_2026-10-29.md) §2 草案基线细化＋397/400 两批换装后的现状对齐。**父权威**：[0ac 即时结果投递设计](IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md)——本稿显式修订其 §10.3「全部带开关（默认关）」的缺省姿态（§2.4），其余 0ac 语义不变。**裁决点**：§3 三项推荐定案在案（用户未即时裁决，主代理按推荐案定稿、随批可改判；S2 执行前为最终确认窗）；一项工程参数（去抖窗）本稿已定。**S2** 落码随下一源码批；**S4** 判据 §6 随档。

## §1 现状勘定（2026-11-19 代码面复核；0.9.16 在役载体）

- **run 外唤醒路径仍不存在**：`drain_completed_tasks` 全仓仍恰两处产品调用——B1＝`agent_loop.rs:5969`（全部 tool replies 后间隙，gate＝主开关 `switch_enabled()`）、B2＝`agent_loop.rs:4593`（终答候选，gate＝`m1_enabled()`＝主开关∧M1）；宿主实现 `orz-host/lib.rs:2033`，trait `orz-loop/host.rs:908`。观察者/唤醒路径零命中，立项档判定不变。
- **投递账本是单次消费共账**：宿主 drain 走 `ToolBridge::drain_between_turn_bash_completions`，与 per-call `TaskCompletionReminder` 共用 `ReportedTaskCompletions` 记账（内部 `mark_reported`——同一任务只经一个通道投给模型一次；owner 会话过滤同源；文本单源 `format_bash_completion`，loop 只注入不重写）。**设计推论**：观察者不得与 run 内 B1/B2 并发抢账——消费必须发生在「唤醒 run 已定案创建」之后（§2.2 peek-then-drain）。
- **开关拓扑未变**：`ORZ_IMMEDIATE_RESULT_DELIVERY`（缺省关，`immediate_delivery.rs:33`）＋`ORZ_IMMEDIATE_RESULT_DELIVERY_M1`（缺省关，`:306`）；生产 env 未设 ⇒ B1/B2 全休眠，run 内可见的后台完成仅靠工具级提醒（`surface_bg_completion_reminders` 缺省 true）。
- **契约面**：`result_delivered` 载荷 schema v0.2（`runtime/result-delivered-event-payload-v0.2.schema.json`），`boundary` 闭枚举三位 `B1_tool_result/B2_turn_end/B3_mid_generation`；双法官（Rust `ALL_FAMILIES`＋Python 镜像）与 conformance corpus 在役。
- **教学文案现状（0dz 后）**：`"you are notified on completion"` 承诺已是条件渲染（`system_reminders_enabled` 门控，prod CLI 缺省 true＝承诺在渲染）——但该承诺指向的完成通知仅 run 内通道（工具级提醒＋B1/B2），**run 外窗口承诺仍假**（299「教学文案承诺假不变量」家族在案）；`is_background` 描述的「or an explicit timeout fires」措辞在 0dz 退役 30h 兜底后已如实。
- **0dz 退役实证**：`DEFAULT_BG_MAX_RUNTIME`／`BG_MAX_RUNTIME_FLOOR`／`GROK_BG_MAX_RUNTIME_MS` 全仓零命中——立项档 §2① 观察者寿命里的「30h 安全帽」**已失效**，本稿 §2.1 按新口径改写。
- **现场时间线**（会话 224b39bd）沿用立项档 §1 不赘。

## §2 设计

### 2.1 观察面

- 会话域观察者挂在宿主会话任务域（`acp_server.rs` 会话运行时内独立 tokio 任务，与 `StoredSession` 同生命周期；owner＝child 沿 0dk，bridge 只做传输）。**寿命＝任务自然退出／`kill_task`／会话结束为止**（0dz 后无机械上限，与 0dz 用户裁决同向；唤醒机制自身不新增任何超时）。
- 观察动作＝**peek 不消费**：`orz-tools` 账本新增非消费列观察 API（读「终态且未 reported」的任务清单，不动 `mark_reported`）；tick 间隔 2s（常量，不入 env）。
- run 结束不撤观察者（与任务同生命周期）；owner 进程死亡注册表消失＝既有跨进程缺口（318 验收⑤），唤醒不跨进程、不新增承诺（立项档 §2.③ 沿用）。

### 2.2 触发面（peek-then-drain 竞态守卫）

1. 会话 idle 判定＝0dk `session_ops` 投影：无在途 run 且无在途 prompt（`SessionState::Idle` 同口径）。
2. idle 且 peek 非空 ⇒ 开**去抖合并窗**：缺省 5000ms（`ORZ_IDLE_WAKE_DEBOUNCE_MS` env 可调，下限 1000ms）；窗内后续完成事实并入同一条投递。
3. **窗内用户 prompt 到达 ⇒ 取消唤醒、facts 不消费**（peek 不动账，留给该 run 的 B1 间隙 drain 自然收走——零丢失零重复，单次消费账语义自洽）。
4. 窗满且仍 idle ⇒ 经既有 `SessionCommand`/prompt 通道创建唤醒 run（忙碌则沿队列语义排后，不打断在途）；**run 创建提交之后**才 drain（消费）账本。
5. 唤醒 run 为真 run：usage metering／journal 全链／999 轮等既有约束照走（立项档 §2.② 沿用）。

### 2.3 投递面

- 唤醒 run 开局注入一条机械事实消息（User role，与 B1/B2 中性事实消息同构同间隙纪律）：`[结果投递]` ＋引导语＋合并后的事实体（复用 `immediate_delivery` 渲染与 `delivery_queue` 合并语义，多任务合并一条）。
- 投递落 `result_delivered` 事件，`boundary` 扩位 **`idle_wake`**（闭枚举第四位）；`suppressed=false`。事件在唤醒 run 开局落账（投递事实与注入消息同轮）。

### 2.4 开关拓扑（§3 裁决推荐定案的落地形态）

- 主开关 `ORZ_IMMEDIATE_RESULT_DELIVERY` 缺省翻 `on`（B1/B2 run 内机械投递随之激活）。
- M1 子开关合并退役：`m1_enabled()` 坍缩为单键（＝主开关）；env 保留兼容读取（设值不再门控，启动面一行告知）。
- 新增唤醒子开关 `ORZ_IDLE_WAKE`（缺省 on、从属主开关）：关＝零唤醒零 `idle_wake` 事件，B1/B2 不受影响。
- 测试卫生沿 363 先例：现有断言旧缺省的测试显式设 env 关（保留旧面回归钉），生产缺省经 env 回退读新缺省，另加缺省翻转钉。

### 2.5 与 0dx 时代机制交互（立项档未勘定，本稿补）

- 唤醒 prompt 带 ext 机械源标记（`mechanical_source="idle_wake"`）；`era::parse_input_intent` 对机械源 prompt **跳过换代判定**——唤醒 run 落当前时代内，`era_boundary` 计数不变、黑板不清、折叠账不动。
- 与 0dw 对账面：唤醒 run 照走 `model_call_usage`／量尺全链，S3 读数可取唤醒 run 为附加样本，无特殊路径。

### 2.6 教学文案

- 唤醒落地（缺省 on）后 `"notified on completion"` 承诺在 run 外窗口**坐实**，文案不动。
- `ORZ_IDLE_WAKE=0` 臂＝承诺降格为 run 内真实（299 家族既有登记口径，非缺省姿态不另改文案）；S2 一并盘点其余残句（0dz 已清四处 30h 文案）。

### 2.7 边界

① 多任务完成合并一条投递；② 唤醒 run 受既有轮数/墙钟约束，无新增豁免；③ 唤醒不跨进程；④ web 投影走既有 run 面（0dt 已进体 0.9.16，立项档 §2.④ 的依赖已解除；真机双卡判据归 0dt 首启窗）；⑤ 去抖窗参数与 peek tick 为实现常量面，不入契约。

## §3 裁决点（三推荐定案＋一工程参数）

| # | 裁决点 | 推荐定案 | 依据 | 改判影响 |
|---|---|---|---|---|
| 1 | 主开关缺省 | 翻 `on` | 立项档注记「不翻则唤醒了也无投递通道」；0dp「该开的全都开」裁决口径 | 不翻则 B1/B2 仍休眠，唤醒面须走自有注入路径（§2.4 备选） |
| 2 | M1 子开关 | 合并退役（单键＋兼容读取） | 双门缺省关系历史包袱；少一层假开关 | 保留两级则 B2 gate 维持 `m1_enabled()` |
| 3 | 提示形态 | 机械事实＋一句引导语 | 给模型明确开局锚点；零常驻注入不变 | 纯机械事实则去引导语，其余不变 |
| 4 | 去抖窗（工程参数，本稿已定） | 5000ms＋env 可调（下限 1000ms） | 复用窗内合并；防多短任务风暴 | 参数级，随时可调 |

## §4 契约面变更清单（S2 执行）

- `runtime/result-delivered-event-payload-v0.2.schema.json`：`boundary` enum 就地扩位 `idle_wake`（纯加位——历史 journal 回放安全，399 严格 schema 教训在案；schema id 与 registry 条目不变）。
- fixtures：`idle_wake` 正型一件；conformance corpus 扩案（Rust↔Python 布尔对拍计数随批更新）。
- 双法官：Rust `verify_result_delivered` 族放行四位＋Python 镜像同形（399 同步纪律）。
- 无新 `EventType`（复用 `ResultDelivered`）；envelope 不变。

## §5 S2 落码清单（随下一源码批；批次内可拆）

1. `orz-loop/immediate_delivery`：开关缺省翻转＋M1 坍缩＋`BOUNDARY_IDLE_WAKE` 常量＋`delivered_fact_payload` 扩位＋B1/B2 gate 更新＋钉。
2. `orz-tools`：账本 peek API（非消费）＋bash 教学文案条件渲染核对。
3. `orz-host`：会话域观察者任务（peek→去抖→唤醒 run 创建→drain→开局注入）＋`SessionCommand` 接线＋era 机械源标记放行＋`ORZ_IDLE_WAKE`/`ORZ_IDLE_WAKE_DEBOUNCE_MS` 解析＋M1 兼容告知行。
4. runtime/fixtures/corpus/双法官：§4 全链。
5. orz-bin：启动面 env 告知（M1 兼容＋唤醒开关回显沿 banner source 口径）。
6. 测试：观察者状态机钉＋竞态钉（窗内 prompt 到达零消费／在途 run 不打断）＋era 不换代钉＋开关负型钉＋缺省翻转钉＋corpus 对拍。

## §6 S4 判据（真机，随下次换装窗）

正型：①后台任务 run 外完成 ⇒ 去抖窗后自动新 run，journal 见 `result_delivered{boundary:idle_wake, suppressed:false}` 且模型面收到完整结果；②在途 run 不被打断（B1/B2 照旧）；③多任务完成合并一条；④唤醒 run 不换代（`era_boundary` 计数不变）；⑤`model_call_usage` 在唤醒 run 照常落账；⑥web 投影单卡无重卡。
负型：⑦主开关 off 或 `ORZ_IDLE_WAKE=0` ⇒ 零唤醒零事件零行为变化；⑧去抖窗内用户 prompt 到达 ⇒ 唤醒取消、facts 由该 run B1 收走（零丢失零重复）；⑨M1 env 设值 ⇒ 仅告知行、行为同单键。

## §7 不做清单

不新增常驻注入；不做跨进程唤醒恢复；不做唤醒 run 的 UI 新面（web/TUI 沿既有 run 投影）；不动 0dh「不做收割」边界；不引入任务完成 webhook/外部通知面；去抖窗不做按任务数自适应（固定窗）。

## §8 入口

- 立项档：[0DV_IDLE_WAKE_BACKGROUND_COMPLETION_REGISTRATION_2026-10-29](audits/0DV_IDLE_WAKE_BACKGROUND_COMPLETION_REGISTRATION_2026-10-29.md)。
- 父权威：[0ac 即时结果投递设计](IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md)（本稿修订其 §10.3 缺省姿态）。
- 关联：0dz（397 批，30h 兜底退役）／0dx（时代机制，era 交互 §2.5）／0dt（382 批，web 单卡）／0dk（session_ops 状态机）。
- 台账：TODO P1-0dv 条／BACKLOG 本批记录指针／索引头 v4.362。
