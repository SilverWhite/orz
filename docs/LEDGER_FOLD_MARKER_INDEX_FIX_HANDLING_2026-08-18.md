# 折叠视图 400 修复处理文档（2026-08-18）

> 状态：`implemented`（S1-S5 已执行，2026-08-18 闭合；S4 机制断言全过，
> reward 项未达标见 §4 复验注——验证③ reward 保持开放）。
> 用途：FUS-BENCHMARK-FULL-EXEC 验证③（make-doom-for-mips 单题复验）400
> （`insufficient tool messages`）根因的修复处理方案——明确处理步骤与具体
> 方式，供实施轮直接执行。
> 关联：ADR-0010 §14.26（折叠状态化设计）/ §14.27（本修复登记，实施轮补写）；
> `docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md`（折叠设计，§3.5 联动
> 修订点）；`D:\tb-eval\jobs\2026-08-18__08-44-56\ROOTCAUSE_FORENSICS_20260818.md`
> （取证存档，根因链已闭合）；BACKLOG 0b / TODO P0-F。

## 1. 根因（已闭合，2026-08-18 复核修正）

### 1.1 现象

三次单题复验（`jobs/2026-08-18__07-43-34` / `__08-13-30` / `__08-44-56`）
均 reward 0、以 400 `insufficient tool messages` 退出；核心机制
（workspace.run_terminal 订单 → 发放 → run_terminal_cmd exit=0 + ACAF
`command_exec` issued/consumed）已验证通过，问题仅在请求视图配对。

### 1.2 决定性证据（第三次取证运行，ORZ_DEBUG_VIEW=1）

400 请求视图共 12 条：

```
U A[call_00_J0tKLL...] U A[call_00_DnCi...] T[DnCi] A[] U U A[call_00_48UN...] T[48UN] A[] U
```

- 尾部 9 条 = `messages[180..]`（DnCi 轮 + 48UN 轮，完整平衡，与 WARN
  `fold_cut=175→174` 窗口逐项对拍一致）；
- 前 3 条 = `messages[..fold_start=2]` = `[U0, A[J0tKLL]]`——**压缩 marker
  已不在索引 1**，首轮 plan_write 声明（J0tKLL，00:45:29 refill）落入冻结
  preamble；
- T[J0tKLL]（00:45:29 tool_completed 后由 controller.rs plan_write handler
  push 的 Role::Tool 消息）位于索引 2、落在被折叠区；视图里声明后紧跟
  ledger（User）→ Provider 判定「assistant tool_calls 后无 tool 回复」→ 400。

### 1.3 根因链（修正版）

1. 压缩执行后 marker 插入索引 1、`fold_state.reset()`；折叠以 `fold_start=2`
   重新累积（运行 2/3 事件均 `fold_start:2`，`fold_cut` 163→175→180）；
2. 后续 loop-top 再次命中压缩触发（rhythm/fallback）→
   `run_template_compact` 首行 `messages.retain()` 无条件删除旧 marker
   （`orz/crates/orz-loop/src/agent_loop.rs:502`），消息数组整体左移一位
   （A[J0tKLL] 从索引 2 回到索引 1）；
3. reduction 守卫不满足 → 返回 `CompactDecision::GuardBlocked`
   （agent_loop.rs:521-522，不 journal 任何事件）；而 `fold_state.reset()`
   只存在于执行路径（agent_loop.rs:657/732），`GuardBlocked`/`NoOp` 返回
   前折叠三态保持冻结；
4. 冻结 `fold_start=2` 过期：preamble = `messages[..2]` = `[U0, A[J0tKLL]]`
   （首轮声明无回复）；`fold_cut=180` 经 `safe_fold_cut` 修正后仍指向完整
   轮（A[DnCi]），尾部配对完整——于是唯一裸露声明就是 preamble 里的
   A[J0tKLL] → 400。

**修正说明**：此前登记「折叠 cut 破坏 plan_write 轮配对」表述不精确——折叠
cut 始终落在完整轮起点（`collapsed_cut` 整轮纪律 + `safe_fold_cut` 双向
校验），本轮 400 的真正破坏点是**压缩触发（未执行）的 marker 删除使冻结
折叠索引失效**，且 `safe_fold_cut` 只校验尾部 cut、不校验 preamble 边界。

## 2. 修复设计定案

### 2.1 主修复（根治）：压缩触发不得在未执行时变更 messages

不变量：**冻结的折叠索引（`fold_start`/`fold_cut`）仅在 messages 保持纯追加
时有效；任何就地变更必须走折叠状态失效（执行压缩路径 = `fold_state.reset()`），
未执行路径必须零副作用。**

具体方式（`agent_loop.rs::run_template_compact`）：

1. 把 502 行 `messages.retain(...)` 从函数首部移到守卫判定（521-522）之后；
2. `NoOp`/`GuardBlocked` 判定使用「含旧 marker」的 `kept_start`/`after` 口径
   （触发时不动数组，口径与现状一致），两路径返回时 messages 与 fold 三态
   均不变；
3. 确定执行后（guard 通过或 force）：检测并删除旧 marker → **重算
   `kept_start`**（marker 删除使索引左移：折叠态 `fold_cut - had_marker`，
   未折叠态重算 `collapsed_cut`）→ 重算 `after`（不含 marker，事件口径与
   现状一致）→ 后续 `drain`/`insert`/`rounds_before`/事件照旧；
4. 执行路径末端的 `fold_state.reset()`（657/732）保持不变。

### 2.2 防御补强（第二道防线）：折叠视图整体配对校验

- `safe_fold_cut` 改为返回 `Option<usize>`：`idx==0` 且首轮不完整时返回
  `None`（放弃折叠）而非原样返回 cut（`action_ledger.rs:372-375`）；
- `build_request_view`：`safe_fold_cut` 返回 `None` 时直接返回
  `messages.to_vec()`（视图回原文）+ warn 留痕；
- `build_request_view` 新增 preamble 边界校验：`messages[..fold_start]`
  末条若为 assistant 声明（`tool_calls` 非空）→ 放弃折叠返回原文（其回复
  必在折叠区）——补齐「safe_fold_cut 只防 cut 不防 fold_start」的缺口。

### 2.3 明确不做

- 不改折叠触发时机（loop-top 完整轮间隙，已正确）；
- 不改压缩触发参数（192K/256K）与 drain/marker 执行语义；
- 不改 gateway 1:1 序列化；
- 不把 shell 工具开放为模型直接工具（Benchmark 完全体安全面不变）。

## 3. 具体处理步骤（实施轮按序执行）

### S1 代码修复（orz 子模块）

| # | 文件 | 改动 | 验收 |
|---|---|---|---|
| 1 | `orz/crates/orz-loop/src/agent_loop.rs` | `run_template_compact` retain 后移 + 执行路径 `kept_start` 重算（§2.1） | `cargo check -p orz-loop` 通过 |
| 2 | `orz/crates/orz-loop/src/action_ledger.rs` | `safe_fold_cut` → `Option` + `idx==0` 返回 `None`；`build_request_view` 放弃折叠 + preamble 校验（§2.2） | 新增单测覆盖 |
| 3 | 上述两文件注释 | 根因注记 + 索引失效不变量写入 | 审查 |

### S2 测试

新增单测（名字与断言）：

1. `guard_blocked_leaves_messages_and_fold_untouched`——折叠已冻结 + marker
   在索引 1 + 守卫不满足 → 返回 `GuardBlocked` 后 messages 未变（marker 仍
   在）、fold 三态未变；
2. `compaction_execution_recomputes_kept_start_after_marker_removal`——
   执行路径（含 marker 场景）drain 数量与事件口径同现状；
3. `build_request_view_abandons_fold_when_preamble_ends_with_declaration`
   ——preamble 末条为声明 → 返回原文；
4. `safe_fold_cut_first_round_unbalanced_returns_none`——`idx==0` 首轮
   不完整 → `None`（放弃折叠）；
5. `safe_fold_cut_mid_round_falls_back_to_round_start`——既有测试
   （action_ledger.rs:495）适配 `Option` 签名后保留；
6. `marker_removal_without_fold_reset_builds_full_view`——模拟旧 bug 场景
   （retain 后未 reset）断言视图回原文、无 400 形态。

执行（既有纪律）：

- `B:\.cargo\bin\cargo.exe check -p orz-loop`
- `B:\.cargo\bin\cargo.exe test -p orz-loop -j 1`（全量按各 crate `-j 1`
  纪律；orz-host 需 `-- --test-threads=1`）
- `B:\.cargo\bin\cargo.exe fmt --all`（必须在 `D:\CLI\orz` 下执行）
- `clippy --workspace --all-targets` 无新增可归因告警

### S3 Linux musl 重建

- `build_orz_aliyun.sh`（ORZ-BUILD-MOUNT-001 契约：挂 `D:/CLI` 为 `/orz`、
  `-w /orz/orz`、cargo-config 挂 `/root/.cargo/cfg`、target=`/target`、
  输出 `/out`）；
- 校验 `D:\tb-eval\orz-linux\orz` 时间戳为新构建时间。

### S4 单题复验（验证③）

- harbor run 同参数：terminal-bench@2.0 / make-doom-for-mips /
  deepseek-v4-flash / max_wallclock=1740 / gsa 卷 b4-900s / ×2 超时 /
  mounts 数组 JSON（`[{...}]`）；
- 断言：reward > 0；无 400；journal 出现 `workspace.run_terminal` 订单 →
  run_host_tool → ACAF `command_exec` issued/consumed；无异常 policy_denied；
- 取证开关收口：适配器 `tb_agents/orz.py` 的 `ORZ_DEBUG_VIEW=1` 为临时取证，
  验证③通过后移除（若保留需登记为常驻诊断）。

### S5 文档同步 + 提交

1. ADR-0010 §14.27（v1.27）：根因修正 + 修复定案 + 实施/验证结果（版本索引
   头同步：v1.26 后追加 v1.27）；
2. `docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md`：§3.5 补「触发未执行
   不得变更 messages」不变量；§8 风险补记（marker 删除与索引失效）；
3. BACKLOG 0b：验证③根因句修正（§1.3）+ 修复实施/复验结果；
4. TODO.md P0-F：新增修复项勾选 + 验证③勾选（未闭合计数不变：修复属
   FUS-BENCHMARK-FULL-EXEC 验证③前置，28 → 27 在验证③闭合时记）；
5. CLI_PROJECT_INDEX：登记本处理文档 + 实施闭合行；
6. 提交：orz 子模块（修复 + 测试 + 注释）+ 父仓库（ADR/设计/BACKLOG/TODO/
   索引）。

## 4. 验收标准（Definition of Done）

- S2 全绿、clippy 无新增可归因告警；S3 重建成功且时间戳更新；S4 reward > 0
  + journal 断言 + 无 400；S5 文档同步并提交（orz + 父仓库）。

## 5. 风险与回滚

- 若复验仍 400：保留 `ORZ_DEBUG_VIEW` 再抓 dump 核对视图配对；检查其余消息
  变更路径（pending_policy 注入、checkpoint 块、状态行追加等）是否在折叠
  冻结后改动数组；
- 回滚：retain 位置改动为单提交可逆；防御补强（放弃折叠）最坏代价=触发时
  一次全量视图（低频，接受，属既有 §8 风险范畴）。
