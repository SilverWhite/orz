# P0-0x S1 实施记录 — 初始轮中立问询（2026-09-11）

> **范围**：S1（常量 + 会话一次性状态 + 触发接线 + 测试矩阵最小集）。
> **设计权威**：[`INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11`](../INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md)
> / [`ADR-0010 §14.66`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / BACKLOG 0x / TODO P0-0x。
> **状态**：S1 完成（**未放行实机、未重建载体**）；S2–S4 待续。

## 1. 落码清单（与设计触点清单逐项对照）

| 设计触点（§5） | 落地位置 | 状态 |
|---|---|---|
| ① `INITIAL_ROUND_INQUIRY_BLOCK` 常量（三问原文） | `orz-assurance/src/orientation/checkpoint.rs` | 完成 |
| ① 注入块前缀登记（绝不持久化回会话） | `orz-loop/src/prompt.rs`（`INITIAL_ROUND_INQUIRY_INJECTED_PREFIX` → `is_injected_block_text`） | 完成 |
| ② 会话一次性触发状态（随会话持久化） | `orz-loop/src/orientation.rs`（`OrientationSessionState.initial_round_fired`，`#[serde(default)]`） | 完成 |
| ② `build_initial_round_fire_record`（`trigger = "initial_round"`） | 同上（+ `commit_initial_round_fire`；`TRIGGER_*` / `ORIENTATION_POST_TOOL_BATCH_GAP` 常量） | 完成 |
| ③ 控制器分派该类触发 | `orz-loop/src/controller.rs::maybe_fire_orientation` | 完成 |
| ④ `agent_loop.rs` 在 `post_tool_batch_gap` 增加首轮触发判定 | 见 §2 **落点更正**：判定并入控制器分派，`agent_loop` 只保留调用点 + 常量引用 | 完成（落点更正） |
| ⑤ `orz-signer` 第二模板摘要登记 | — | **留 S2** |
| ⑥ 事件面 `trigger` 枚举新增 `initial_round`（schema / fixtures / 法官族 / Python 镜像） | runtime v0.2 payload schema 枚举 + 描述已加；fixtures 正负例、`run_event_journal_validation` 族、Python 镜像 | **schema 完成，其余留 S2** |
| ⑦ 测试矩阵最小集 | `orz-loop` 单测 + 集成（见 §3） | 完成 |

## 2. 与设计清单的差异（需登记）

1. **触发判定落点**：设计 §5-④ 原写「`agent_loop.rs` 在 `post_tool_batch_gap` 增加首轮
   触发判定」。实际实现把**判定并入既有唯一分派点**
   `controller::maybe_fire_orientation`（该函数已是周期问询的构建/票据/事件/注入唯一出口），
   `agent_loop.rs` 保持原调用点并改用 `ORIENTATION_POST_TOOL_BATCH_GAP` 常量。
   **语义不变**（同一 gap、同一 pending 闸、同一票据、同一注入路径），且避免把
   「哪个 trigger 该发」的分支散到循环里；`loop_top_gap` 仍只承担周期问询。
2. **schema 枚举提前**：设计的 S2 含「runtime schema 枚举」，但 S1 产出的 v0.2 事件若
   不在枚举内即为**生产者/合约不一致**（发出去的 journal 过不了自己的 schema）。
   故 S1 内仅提前落 **枚举值 + 描述** 一行；*fixtures 正/负例、法官族、Python 镜像、
   signer 第二摘要* 仍全部留 S2。
3. **commit 语义分派**：`PendingCheckpoint::Orientation` 现承载两种 fire，
   `checkpoint::commit_pending` 按 `record.is_initial_round()` 分派——
   初始轮**只置一次性标志并推进 fire 序号**（`checkpoint_id` 单调唯一），
   **不重置周期计数**（§3.2「互不影响、互不重置」）；周期问询语义未动。

## 3. 测试矩阵（设计 §5-⑦）

| 判据 | 测试 | 结果 |
|---|---|---|
| 触发恰好一次 | `initial_round_record_is_one_shot_and_keeps_periodic_counter` | 通过 |
| 旧侧车兼容（无新字段 → 未触发） | `initial_round_flag_defaults_false_for_legacy_sidecar` | 通过 |
| 无工具首轮顺延到首个动作批次 | `initial_round_fires_once_at_first_tool_batch_and_defers_without_one`（run1 纯文本零触发 → run2 动作批次触发一次） | 通过 |
| 与周期问询不互扰 | `initial_round_does_not_interfere_with_periodic_threshold`（阈值 3：初始轮 completed=1、周期 completed=3；周期提交后计数归零） | 通过 |
| 软门不阻断工具 / 触发轮工具面不变 | 同上（触发轮工具面 == 前一普通轮，逐字段比对） | 通过 |
| 纯文本回答被消费后续跑 | `orientation_soft_gate_consumes_text_answer_and_continues`（扩为双 fire） | 通过 |
| 终答前审查报告 + 反例质询**完全不变且不含三问** | `final_answer_audit_blocks_unchanged_and_carry_no_inquiry`（gate event payload == 常量、`once_only=true`；审查报告与门块均不含三问；两块都不落会话侧车） | 通过 |
| 块形状与措辞纪律 | `initial_round_inquiry_block_shape`（`initial_round` 独立版本标识、三问原文、无强制模板/工具禁令/追责式措辞、与周期块互不前缀命中） | 通过 |
| 注入块过滤登记 | `is_injected_block_text_detects_blocks`（新增初始轮块正/负例） | 通过 |

**回归影响（行为性，非缺陷）**：初始轮问询对**所有带 orientation 状态的主车道会话**生效，
凡「一个动作批次 + 固定脚本」的测试都要多一个模型轮。本刀同步更新了受影响的既有测试：

- `orz-loop`：`subagent_lane_feeds_and_fires_orientation`（主车道初始轮 + 检索车道周期轮
  两条 fire，按 trigger 取值而非按序取）、`orientation_soft_gate_consumes_text_answer_and_continues`。
- `orz-host`：`acp_server` 5 例（脚本各 +1 轮）、`codex_app` 3 例（`console_exec_script`
  与两处脚本各 +1 轮）。
- 判据「不因初始轮而丢周期计数」在 `retrieval/dispatch.rs` 例中同时被钉住
  （主车道 4 轮、检索车道提交后 1）。

## 4. 顺带修复（本刀发现，均非 0x 语义）

1. **`orz-bin` 测试目标编译缺口（0v S2 遗留）**：`main.rs` 的 conformance stub
   `BrowserControlOutcome` 初始化缺 `engine` / `engine_attempts` / `results` 三个字段，
   `cargo check --all-targets` 直接失败（0v S2 报「测试矩阵绿」时未覆盖 `orz-bin` 测试目标）。
   已补 `..Default::default()`（stub 只走导航级动作）。
2. **`orz-bin` 三条守卫测试的 ACAF 环境隔离**：`stall_watchdog_*` /
   `wallclock_dropped_run_*` 与本刀无关地依赖进程环境 `ORZ_ACAF_FAIL_CLOSED`——
   同进程的 env 测试会移除该变量，未显式配置 signer 时 fail-closed 默认拒跑，
   导致 `--test-threads=1` 也稳定失败。三条测试改为显式
   `.with_acaf_fail_closed(false)`（守卫测试与 ACAF 无关），测试目标转全绿。

## 5. 预存在失败（**改动前即存在**，已在 stash 后的干净树上复现，不属本刀）

| 位置 | 现象 | 复现 |
|---|---|---|
| `orz-assurance` `tests/fixture_journal_conformance.rs::family_stage_tamper_detected_end_to_end` | `assert!(!report.valid)` 失败——篡改后的 v0.2 fixture journal 未被 family 阶段判为无效 | 干净树同样失败 |
| `orz-tui` lib 9 例（`acp_client::tests::*` / `runner::tests::*`） | test 二进制里 orz-loop 以非 `cfg(test)` 编译 → 未设 `ORZ_ACAF_FAIL_CLOSED` 时 fail-closed 默认生效、无 signer 即拒跑 | 干净树同样失败 |
| `orz-host` lib 2 例（`call_tool_timeout_kills_process_tree`、`session_volume_symlink_windows_end_to_end`） | 并行负载下的 2s 墙钟 / 输出截断时序 flake | `--test-threads=1` 全绿（286 passed） |

## 6. 本刀验证证据

- `cargo test -p orz-loop`：**762 passed / 0 failed**（3 ignored）。
- `cargo test -p orz-bin`：全部目标绿（bin orz 12 passed、signer 14、acaf_e2e 23、stdio_e2e 1、real_flag 2）。
- `cargo test -p orz-host --lib -- --test-threads=1`：**286 passed / 0 failed**。
- `cargo test -p orz-assurance`：lib 211 passed；预存在 1 例见 §5。
- `cargo clippy -p orz-loop -p orz-assurance -p orz-host -p orz-bin --all-targets`：
  新增代码零告警（警告全部落在未改动的既有行）。
- `cargo check --workspace --all-targets`：零错误（`PROTOC=D:\CLI\orz\bin\protoc.exe`）。

## 7. S2 余项（未做，不属 S1）

1. runtime fixtures 正/负例（`trigger = initial_round` 的合法 payload / 非法枚举负例）
   与 journal fixture 重捕；`run_event_journal_validation` 法官族同步。
2. Python 镜像 `assurance/orientation_runtime_guard.py`（+ 其 tests）同步。
3. `orz-signer` 第二模板摘要常量（`INITIAL_ROUND_INQUIRY_BLOCK` 绑定）——现票据
   `template_sha256` 仍只钉 `ORIENTATION_BLOCK`（低风险：签发与校验同源，未产生
   拒绝；但绑定对初始轮块目前是空的）。
4. `scripts/check_repository.py` 全绿收口（本刀在提交 orz 后即可全绿）。

## 8. 未做（明确边界）

- **未**跑实机 / 未重建双平台载体（S3/S4 需另行走放行门）。
- **未**新增 consumption 类事件（用户裁决：不做消费审计）。
- **未**落黑板锚点、**未**改周期问询三问与阈值、**未**改终答前审查报告与反例门注入。
