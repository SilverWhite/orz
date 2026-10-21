# 322 今晚摩擦修复批（会话 c23e0aaa：压缩 / 反例门 / 工作区切换 / 恢复）

- 日期：2026-10-21（夜间）
- 来源：用户报告「实际使用中感觉上下文压缩摩擦、反例问询疑似导致模型提前停止动作、最新会话恢复失败、工作区切出后切不回来且旧工作区信任记录从 Web UI 消失」
- 证据面：`C:\Users\1\OneDrive\Desktop\新建文件夹\.gsa\`（会话 `c23e0aaa-e752-4215-9a4f-0e435ff09b6a`，runs 13–17，2026-10-20 14:31–15:50 UTC）
- 边界：只改源码与前端资产；**未重建载体、未提交、未推送**（后台 `_R233_LV_SWAP` 训练在跑，用户令「先让实验跑」）。运行中的 Web 工作台仍是 0.9.4 旧二进制，代码修复须下次重建/重启后生效；工作区信任的即时缓解见 §4。

## 1. 台账读数（摩擦实证）

| # | 证据 | 读数 |
|---|---|---|
| F1 | `RUN-c23e0aaa-17` #005 `mechanical_audit_update(context_scale:320k)` | 每轮 run 起始均报 `model_face_estimate_tokens≈357699`，H1 硬提醒反复触发 |
| F2 | `context_compressed`（runs 11/13/14/17） | 触发 343–381K → 目标 314–361K，单次仅降 ≈20K；压缩后仍高于 320K，下一 run 再触发 |
| F3 | `RUN-c23e0aaa-13/14/15/16/17` 的 gate 序列 | 候选终答已自带「## 反例自查（输出前最后一次）」，gate 仍注入；同一 run 产出两份完整终答（3.4–5.3K 字） |
| F4 | `RUN-c23e0aaa-17` #074/#075/#076/#078 | 草稿含 `[SEMANTIC_SUMMARY]`；gate 后、终答前消费压缩：381014→361182；终答反称「分块 1–17 均已压缩，本轮无可压块」 |
| F5 | `compaction-RUN-c23e0aaa-17-0076.md` | 27,984 字符；「机械摘要行」内嵌整段 `run_terminal_cmd` 脚本（单段 7.2K/9.5K 字符），压缩释放≈34K token 被 marker 反吞 |
| F6 | `C:\Users\1\AppData\Local\orz\bin\grok-home\trusted_folders.toml` | 仅 `D:\CLI`；启动工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹` 从未写入持久信任 |
| F7 | 2026-10-20 23:53 进程树 | `WindowsTerminal -d "…\新建文件夹"` → `orz web --real`（启动 cwd＝桌面工作区）；切到 `D:\CLI` 后桌面工作区既非当前也非 TrustStore 授予项 |

## 2. 根因与修复

### 2.1 H1 闩位跨 run 丢失（F1/F2）

`ContextScaleState::from_notified_keys` 从侧车恢复 `fired`，但 `latched` 恒为空；`due()` 对硬档用 `latched.insert(..)` 判「新越线」。每个 prompt 都是新 run，状态重建后只要模型面仍在 320K 以上，就把「一直在线以上」误判成「再次越线」——每轮重发硬提醒，模型每轮压缩一点又立刻回到线上。

修复（`orz-loop/src/context_scale.rs`）：恢复时把已发 **H1（320K 硬提醒）** 同时置入 `latched`；仍在线以上不重发，真正落到线下时 `due()` 清闩，之后再越线才重发。**T1（500K 必定压缩）刻意不压闩**：安全线在恢复后仍须重新要求压缩，失败由 700K 守卫兜底。

### 2.2 反例门重复注入（F3/F4）

`agent_loop.rs` 终答候选处只判「无工具调用 + 有执行事实」，未看候选本身是否已做反例自查。模型从第二次起会主动写「## 反例自查」段，旧逻辑仍插 `[COUNTEREXAMPLE_GATE]`，逼出第二份完整终答（每份 380K prompt 的额外模型调用）。

修复（`orz-loop/src/agent_loop.rs`）：
- 新增 `final_answer_has_counterexample_selfcheck`（只认行首可选 `#` 后的「反例自查」），候选已自带该段时跳过 gate；
- 新增 `defer_semantic_once`：gate 确实注入时，**下一轮 loop-top 不消费** `pending_semantic`，摘要留到 run 尾（终答已产出）再压缩——门轮与草稿看到同一上下文，杜绝「门后压缩、终答重写」；
- 计划/纯文本短答等既有收窄判据不变；plan 变体与 grill 跳过口径不变。

### 2.3 压缩 marker 体积（F5）

`action_ledger::build_ledger_block` 直接渲染 `target/pointer/final_reply` 原文，未走外置台账文件已有的 300 字符字段上限；一条命令的整段脚本把 marker 撑到 28K 字符。

修复（`orz-loop/src/action_ledger.rs`）：
- 抽出共享 `sanitize_ledger_field`（换行转义 + 300 字符字段上限），外置行与模型面台账块同口径；
- 模型面台账块再加整行 `LEDGER_BLOCK_ROW_CAP=400`（设计「摘要行 ~200–350 字符/行」），防止三字段叠加近千字符/行。

### 2.4 工作区切出后不可切回（F6/F7）

`orz web` 的 B 形态只列 TrustStore 授予项；启动 cwd 不一定是授予项。从桌面工作区切到 `D:\CLI` 后，桌面既非当前、也不在授予清单，探索器整体消失；`POST /api/workspace/switch` 的信任门对桌面 403。旧会话回看 `?root=` 同病。

修复：
- `orz-web/src/server.rs`：`ServerState` 增进程内 `visited`（启动 cwd ＋ 每次切换前后根，去重保序）；`set_cwd` 记录前后根；`resolve_root` 接受 visited 根；`api_workspace_switch` 的 known 门＝TrustStore ∪ visited（仍 fail-closed：从未打开过的未信任根照旧 403）。
- `GET /api/boot` 与切换响应增 `recent_workspaces`；前端 `state.recentWorkspaces` ＋探索器新增「本次会话打开过（N）」子组（只代表本会话可切回，不冒充持久授信）；当前行标「（未持久授信）」并提示发消息走信任确认。
- 边界：visited 只活在本桥进程内，不写 TrustStore；进程重启后仅剩启动 cwd。要跨重启保留仍须用户显式信任。

### 2.5 恢复失败（跨工作区，F7）

前端 `resolveLoadCwd()` 用**当前** `state.workspaceCwd` 调 `session/load`；切换工作区后，旧会话侧车在旧根的 `.gsa/conversations/`，host 在新根找不到并回 `session not found`。

修复：
- `orz-web/src/runs.rs`：`ConversationSummary` 增 `root`（列清单时的归属根）；
- `main.js`：`openConversation` 把 `summary.root` 存进 `resumeTarget`；`resolveLoadCwd(target)` 优先用归属根，缺省才回退当前 cwd；
- 配合 §2.4，切回旧工作区后再恢复也走同一根。

## 3. 静态验证

- `rustfmt --edition 2024 --emit stdout` 逐文件解析：7 个 Rust 文件全过。
- `node --check`：`main.js`／`state.js`／`widgets.js`／`api.js` 全过。
- `cargo check -p orz-loop -p orz-web --offline -j 2`（Idle 优先级）：通过（依赖编译 1m44s；首次有一条已修掉的 unused doc comment 警告，复检 5.43s 零告警）。
- 定向单测（Idle 优先级，`cargo test --lib`）：
  - `context_scale::tests::` 15/15 通过（含新增/改口径的 H1、T1、水位往返三钉）；
  - `action_ledger::tests::ledger_block_bounds_every_field_and_keeps_one_line_per_row` 通过；
  - `controller::tests::counterexample_gate*` 3/3 通过（含新增 self-check 跳过钉与既有 once-only 钉）；
  - `agent_loop::tests::final_answer_semantic_summary_is_consumed_at_the_run_tail` 通过；
  - `server::tests::visited_workspace_survives_switch_and_resolves_old_root` 通过。
- 回归面：`action_ledger::tests::` 23/23、`server::tests::` 19/19、`controller::tests::run_turn_full_gate_sequence` 1/1 全过。
- 新增钉子清单：
  - `context_scale::hard_reminder_does_not_refire_across_resume_while_still_above`
  - `context_scale::hard_truncate_line_refires_after_resume`
  - `context_scale::session_watermark_round_trips_and_suppresses_already_notified_tiers`（改口径）
  - `action_ledger::ledger_block_bounds_every_field_and_keeps_one_line_per_row`
  - `controller::counterexample_gate_skipped_when_draft_already_has_selfcheck`
  - `server::visited_workspace_survives_switch_and_resolves_old_root`

## 4. 即时缓解（不需重建）

已在运行中 Web 桥所用的信任存储
`C:\Users\1\AppData\Local\orz\bin\grok-home\trusted_folders.toml`
追加当前桌面工作区的持久授予（原文件备份 `.bak-20261021`）：

```toml
[folders.'C:\Users\1\OneDrive\Desktop\新建文件夹']
trusted = true
decided_at = 1792512647
```

刷新 Web 页面后，该工作区应出现在「已信任工作区」并可切回；若仍不出现，说明运行中的 `orz web` 实际读的是另一个 GROK_HOME，需以启动终端 env 为准。

## 5. 残余与后续

- 运行中的 0.9.4 二进制不会因源码改动而变化；下次载体重建/重启后 §2 全部生效。
- H1 不重发后，若模型在 320–500K 之间长期不压缩，500K T1 仍会强制开窗（三步升级），700K 守卫兜底；这是设计允许的延迟。
- `compaction` marker 的历史文件不会回写缩小；新压缩才受益。
- 未在 322 当批更新 `CLI_PROJECT_INDEX.md`／BACKLOG／TODO；已由 [323 重建/换装批](323_CARRIER_REBUILD_V095_2026-10-21.md) 并入索引 v4.293，本档保留摩擦证据与代码修复口径。
