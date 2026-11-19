# 0dv S2 落码批：后台完成闲置唤醒（403 批，2026-11-19）

> **性质**：S2 落码批（计数 73 不变；零 bump 零重建零换装；不推送）。**来源**：用户令「我没有异议，可以开始落码了」——402 设计稿三裁决推荐案全数追认生效（主开关 `ORZ_IMMEDIATE_RESULT_DELIVERY` 缺省翻 on／M1 合并退役＋兼容读取／提示面机械事实＋引导语）；`ORZ_IDLE_WAKE` 新子开关缺省 on、去抖窗缺省 5s（env 可调、下限 1s）。设计权威：[402 设计稿](../IDLE_WAKE_BACKGROUND_COMPLETION_DESIGN_2026-11-19.md)。子仓 orz `2b534ffb`（16 文件 +890/−51）。

## §1 六件落码清单（设计稿 §5 对应）

1. **orz-loop**（`immediate_delivery.rs`／`agent_loop.rs`／`controller.rs`／`era.rs`／`lib.rs`）：
   - 开关拓扑：`switch_enabled()` 缺省翻 on（env `Err` 回退 true；关臂 `0/false/off/no` 保留）；`m1_enabled()` 坍缩主开关单键（名号保留、恒等主开关）＋`m1_env_set()` 兼容告知面（M1 env 设值不再门控）。
   - 契约常量：`BOUNDARY_IDLE_WAKE="idle_wake"`；`IDLE_WAKE_TRIGGER="[idle_wake]"`／`IDLE_WAKE_PROMPT`（机械触发 prompt 全文）；`IDLE_WAKE_ENV`／`IDLE_WAKE_DEBOUNCE_ENV`（缺省 5 000ms、下限钳 1 000ms）；`idle_wake_enabled()`（主∧子）、`idle_wake_debounce_ms()`；`render_idle_wake_message()`（`[结果投递] 后台任务已完成，结果如下：`＋合并事实体）；`wake_open_deliveries()` 机械核（facts→队列 round 0 admit→due，抽纯函数供钉测）。
   - B2 收尾 gate 两处 `m1_enabled()`→`switch_enabled()`（注释随退）；唤醒 run 开局投递块＝Main 车道∧prompt 机械前缀∧主开关开 → `drain_completed_tasks()` **消费** → 机械核 → 逐事实 `result_delivered{boundary:idle_wake}` 落账 → 注入一条合并 `[结果投递]` 消息（drain 空＝零事件零注入照常跑）；controller 侧机械 prompt 不进 `note_user_authored_input`（写回豁免表不受机械文本污染）。
   - era 单源：换代裁决抽 `era::resolve_era_trigger(prompt, conversation_is_empty)`——机械源 prompt 永不换代（四臂既有语义逐位保持），宿主 `handle_session_prompt` 改调单源＋五臂钉。
2. **orz-tools**（`reminders/task_completion.rs`／`bridge.rs`／`persistence.rs`／`registry/types.rs`）：账本非消费读 `is_reported`／`reported_snapshot`；`bridge::peek_between_turn_bash_completions`（与 drain 同过滤、不写账）＋两钉（连排 peek 同结果、尊重预留/已投）；`ResourcesPersistence::load_read_only`（不开后台写任务的只读加载，`load` 重构复用）。
3. **orz-host**（`acp_server.rs`／`lib.rs`／`stdio.rs`／`codex_app.rs`）：`AcpServer::idle_wake_watcher` 单 ticker 观察者（2s tick；windows/seen 任务本地 HashMap；开关关清窗；closing/run-active 清窗跳过；peek 非空开去抖窗；窗满仍 idle 经 `handle_session_prompt` 创建机械唤醒 run——busy 拒绝＝窗内用户 prompt 到达竞态臂，facts 未消费由胜出 run 的 B1 收走）；`idle_wake_peek`（`&dyn TerminalBackend`：completed∧非 Monitor∧owner 同会话或无主兼容∧未投持久化账∧观察者未见）；`load_reported_task_ids`（`cwd/.gsa/resources_state.json` 只读）；`OrzHost::drain_completed_tasks` 消费后即席 `persist_state`（账本新鲜度闭环——run 外唯一 mark_reported 通道，工具执行驱动的自动存档不覆盖此点）；stdio/codex 两入口 `spawn_local` 起挂观察者（`orz-bin` stdio＋`orz-codex` 全覆盖）。
4. **契约链**：`runtime/result-delivered-event-payload-v0.2.schema.json` boundary 闭枚举就地扩位 `idle_wake`（纯加位，历史 journal 回放安全；399 严格 schema 教训在案）；正型 fixture `result-delivered.idle-wake.valid.json` 手写入库（沿 390+ 手写先例——生成器锁步表落后 `era_boundary`/`model_call_usage`/`run_preflight` 三件被守卫拦截，据实注记不代改）；全链 fixture journal `idle-wake-delivery.jsonl`（Python 侧同构链式规则构造、参考法官 `validate_journal_file` 自验零违例：run_preflight→tool_availability_check→run_started〔机械源 prompt〕→唤醒开局投递两事件→model_output→counterexample_gate→model_output→run_finished）＋Rust `V02_JOURNALS` 登记＋对拍 crosscheck 自动拾取；`families.rs` 共享 corpus `+result_delivered_idle_wake_ok`（boundary 第四位经 accounting 族全绿、两 dedupe_key 互不判重），计数钉 266→**267**；Python 专钉 `test_v02_result_delivered_idle_wake_boundary_contract`（正型过／越界 `B4_pending_tool` 拒／历史界 `B1_tool_result` 仍过）；双法官零改动（boundary 无关、schema-faithful；Rust `verify_result_delivered_accounting` 与 Python `_verify_v02_result_delivered_accounting` 均不闭枚举 boundary）。
5. **orz-bin**（`main.rs`）：`delivery_switch_banner()`——仅非缺省姿态打一行（缺省＝主 on＋wake on 零输出不加噪；M1 env 设值在行内如实告知「retired 403: no longer gates」）；`render_delivery_switch_line` 纯渲染供字面钉；`-p` 面与 `orz web` 启动面两处 `permission_mode_banner()` 调用点随挂。
6. **测试**：见 §4。

## §2 实现修订注记（与 402 设计稿的差异，随批如实登记）

- **peek 位置**：观察者不走 `bridge::peek_between_turn_bash_completions`，走「terminal backend 任务表过滤＋持久化已投账＋观察者已见账」三重过滤——per-run toolset 拓扑下（`build_toolset` 每 prompt 重建），bridge 内存账本与宿主观察者不同源；**持久化账**（drain 消费后即席 persist 保鲜）才是跨 run 真账。bridge peek API 保留作库面 API＋钉测；设计稿 §2.1 的 peek 位置随实现修订，「peek 恒不消费」语义不变。观察者已见账兜住持久化滞后的空唤醒面。
- **宿主 watcher 全链 e2e 未落自动测试**（busy 竞态臂／窗满创建臂）：机械核、过滤链、换代裁决、开关面各有单测钉；端到端九条判据归 S4 真机窗（设计稿 §6）。
- **跑法勘记**：`RUST_MIN_STACK=32M` 后缀形态在本机工具链**未生效**（测试进程仍默认栈 → `STATUS_STACK_OVERFLOW` 立即崩，399 档记载按此勘正）；字节形态 `RUST_MIN_STACK=33554432` 生效。

## §3 测试卫生与既有漂移顺带

- 缺省翻转影响面核查：loop/assurance 全量绿——投递面在测试网恒空账（测试宿主 drain 恒返空），翻转不改变既有事件计数/序列断言；orz-host 串行全量绿（观察者不在测试进程起挂——仅 stdio/codex 两入口起挂）。
- **orz-tools `registry/types.rs` 两处教学文案钉＝397（0dz）漏网既有漂移**：0dz 把 bash 边界句改为「until they exit, the session ends, you stop them, or an explicit timeout fires」，此钉仍断言旧措辞 `until the session ends`；干净基线（stash `5d3bec4d`）对照同形失败实证，沿 146/298/398「随批对齐」先例改两臂（`bash_descriptions_track_system_reminders_setting`）。

## §4 读数

| 套件 | 读数 | 备注 |
|---|---|---|
| orz-loop lib | **993/0/4**（基线 988＋净增 5 钉） | 翻案钉／M1 坍缩钉（原从属钉改判重写）／唤醒开关+去抖钉／开局消息+payload 钉／机械核钉＋era 单源五臂钉 |
| orz-tools lib | **3064/0/6**（＋2 peek 钉） | 0dz 漂移钉两臂随批对齐（§3） |
| orz-assurance lib | **311/0** | corpus 计数钉 266→267、对拍 crosscheck（Rust↔Python 布尔）含新场景全绿 |
| fixture conformance / cli | **11/0**／**4/0** | idle-wake-delivery.jsonl 入刊全链绿 |
| Python runtime conformance | **18/0**（17＋1 专钉） | `test_run_event_conformance.py` |
| Python journal 族 | **282/0** | `test_run_event_journal_validation.py`；参考法官对自构 journal 自验零违例 |
| orz-host 全量串行 | **464/0/6**（基线 462＋2 新钉） | `RUST_MIN_STACK=33554432`＋`--test-threads=1`（§2 勘记） |
| orz-bin 六套件 | **24/2/15/24/2/1 全绿** | banner 行零输出（缺省）不破既有输出钉 |
| clippy 四 crate | **129＝基线 129 持平** | 干净基线 stash 对照；改动行零命中 |
| fmt / git diff --check | **净** | fmt 随批格式化 5 文件（纯空白）后复验 |

## §5 边界与不做清单

- **零 bump 零重建零换装**（0dv 进体随下次重建，在役 0.9.16 容忍）；**不推送**。
- 缺省翻转行为（B1/B2 run 内投递激活＋唤醒缺省 on）在在役 0.9.16 **未生效**——翻转面随下次重建进体，S4 真机核证同窗。
- S4 九条判据（设计稿 §6）随下次换装窗，可与 0dw S3／0dy S3／0dz S3／0dx 活体两判据／0dt 判据 4 合并首启窗同车。
- 去抖窗与 peek tick 为实现常量面不入契约（设计稿 §2.7）；不做跨进程唤醒、不做 UI 新面（§7 不做清单照设计稿）。

## §6 入口

- 设计权威：[402 设计稿](../IDLE_WAKE_BACKGROUND_COMPLETION_DESIGN_2026-11-19.md)／立项档 [0DV_*_REGISTRATION](0DV_IDLE_WAKE_BACKGROUND_COMPLETION_REGISTRATION_2026-10-29.md)。
- 契约件：`runtime/result-delivered-event-payload-v0.2.schema.json`（boundary 四位）／`runtime/fixtures/run-event-v0.2/payloads/result-delivered.idle-wake.valid.json`／`runtime/fixtures/run-event-v0.2/journals/idle-wake-delivery.jsonl`。
- 台账：TODO 403 批行／BACKLOG 本批记录指针／索引头 v4.363。
