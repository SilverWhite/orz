# 0ai 重文件拆分报告：`orz-loop/src/host_exec.rs` → `host_exec/` 七文件（含框架内部摩擦记录）

> 状态：**实施件已落工作区（未提交、未推送、未重建镜像）**；日期 2026-09-16。
> 基线：父仓 `HEAD=255af3b03f3b`（run 起始 worktree 干净）；orz 子仓 `HEAD=a580eb08`（detached，= 0.5.2 源冻结基线）。
> 关联：[0ai 重文件拆分勘察](HEAVY_FILE_SPLIT_SURVEY_2026-09-13.md) / [TODO `0ai`](../../TODO.md) / [索引 `GAP-HEAVY-FILE-SPLIT`](../../CLI_PROJECT_INDEX.md) / [BACKLOG 0ai](../BACKLOG_AND_PRIORITIES.md) / [0.5.2 载体重建与放行](052_CARRIER_REBUILD_AND_0AI_RELEASE_2026-09-16.md)。
> 口径声明：本件只登记**事实与读数**；未运行者标「未跑」，未能核证者标「未核」。台账（TODO / BACKLOG / 索引 §8）登记与计数随收尾批，本批按指令**不提交**。

---

## 1. 一句话结论

`orz-loop/src/host_exec.rs`（9,710 行）已按职责域**机械拆分**为 7 个文件（最大 `tool_run.rs` 6,784 行 < 10,000 硬门禁），代码实体逐行搬迁、零逻辑改写；`lib.rs:36 mod host_exec;` 与 `crate::host_exec::*` 外部路径不变；`cargo test -p orz-loop` 818 passed / 0 failed / 3 ignored，`cargo fmt` 干净，`cargo check` 下 `orz-loop` **零告警**；改动面仅 `D host_exec.rs` + `?? host_exec/`（无其它文件）。

## 2. 任务口径（TODO 0ai 三段式）

- **S1 切分图**：按职责域定模块边界与迁移清单；验收＝单文件 ≤10,000 行＋职责域单一。
- **S2 机械搬移**：`pub(crate)` 机械拆分，**行为不变**（事件序列与 journal 哈希链不动、可见性收敛、无逻辑改写）。
- **S3 回归核验**：orz-loop / orz-host / orz-assurance ＋ fmt/clippy ＋ 门禁 `valid: true`；产出按正常批次合回（本批按指令不提交）。

## 3. 拆分结果（事实表）

| 模块 | 行数 | 职责域 |
|---|---|---|
| `host_exec/mod.rs` | 24 | 目录入口：`mod` 声明 + `pub(crate) use` 再导出（保 `crate::host_exec::X` 路径） |
| `host_exec/facts.rs` | 855 | 宿主事实 → journal 事件（browser launch / 进程树回收 / 宿主资源事实 / idle kill 补账） |
| `host_exec/failure.rs` | 917 | 失败信封与漏斗（`ToolFailureOutcome` / 失败聚合 / LIF deny 通道 / 订单失败回写） |
| `host_exec/serp.rs` | 817 | SERP 预算结算与取证（预算码 / 导航计数 / attempt 落盘 / 拒绝路径） |
| `host_exec/candidate.rs` | 186 | 候选计数门（`candidate_gate` / `refuse_candidate`） |
| `host_exec/dep_graph.rs` | 269 | 依赖图事实（P2-11 锚点链 / 工具→实体变更边） |
| `host_exec/tool_run.rs` | 6,784 | host 工具执行编排三件套（`run_host_tool` / plan gate / 超时主循环＋家族回归测试） |
| **合计** | **9,852** | 原单文件 9,710 行 ＋ 142 行模块头 / 导入 / 胶水 |

行数口径与时点：勘察档（2026-09-13）记 **9,184 行**（同 commit `0b2a8f5b` 实读同值）；拆分基线 `a580eb08` 实读 **9,710 行**（＋526 行，属期间 0ah S1 / 0.5.2 等批次增长；两侧均按物理行计）。

## 4. 行为不变判据（读数）

1. **行多重集等价**（非注释代码行、空白归一）：MISSING＝9 / INVENTED＝118。
   - MISSING 9 条全是**同一批签名的重排前形态**（`record_dep_graph_fact` / `refuse_serp_budget` / `refuse_serp_session_floor` / `is_serp_search_call` / `serp_navigations_from_output` 的签名折行 5 条）＋ 4 条长 `use` 列表折行；
   - INVENTED 118 条＝67 条 `use` / `pub(crate) use` ＋ 10 条 `mod` 声明 ＋ 5 条 `impl AgentLoopController {` ＋ 14 条 `#[cfg(test)]` ＋ 5 条 `pub(crate)` 升可见签名 ＋ 11 条闭合花括号 ＋ 6 条 `use` 列表折行变体——**全部为模块胶水与导入面**，无一条业务语句；
   - **差分自检（非空转证明）**：把域模块复制到临时目录、只翻转 1 个比较运算符（`candidate.rs:85` `u == &url` → `u != &url`），同一验证器立即报 dMISSING＝＋1 / dINVENTED＝＋1。
2. **测试**：`cargo test -p orz-loop --lib` → `818 passed; 0 failed; 3 ignored`（running 821）；host_exec 家族测试函数拆分前后 **58 ＝ 58**。
3. **格式**：`cargo fmt --all -- --check` 退出 0。
4. **编译**：`cargo check -p orz-loop --lib --tests` Finished；`orz-loop` **0 告警**（余下 1 条 `xai-tty-utils` dead_code 为既有）。
5. **外部面**：`crates/orz-loop/src/lib.rs:36` 仍为 `mod host_exec;`（零改动）；`host_exec/mod.rs` 以 `pub(crate) use` 保持 `crate::host_exec::{ToolFailureOutcome, is_serp_search_call, serp_navigations_from_output}` 等旧路径。
6. **改动面**：orz 子仓 `git status --porcelain` ＝ `D crates/orz-loop/src/host_exec.rs` ＋ `?? crates/orz-loop/src/host_exec/`；父仓仅新增本报告。

## 5. 与「纯机械」的两处偏移（如实登记）

- **可见性收敛**：跨模块互调项按需升 `pub(crate)`（如 `record_dep_graph_fact` / `refuse_serp_budget` / `refuse_serp_session_floor` 由私升 `pub(crate)`）。更紧的写法是 `pub(in crate::host_exec)`；本批沿用 `CONTROLLER-SPLIT` 先例的 `pub(crate)`，属**放宽**而非收紧。
- **导入面收敛（S3 追加）**：拆分后各模块沿用原文件导入清单，rustc 报 10 条 `unused import`——它们只在**非测试构建**下未用，测试代码经 `use super::*` 实际需要（首轮以 `cargo fix --lib` 直接删除，导致测试构建 72 处 `cannot find ...` 报错，已回退）。最终处置＝删除纯冗余项 ＋ 测试专用项改为 `#[cfg(test)] use ...`（净增 14 条 `#[cfg(test)]`），处置后 `orz-loop` 编译告警 10 → 0，且测试与 `fmt` 复验仍绿。

## 6. 回归核验读数（S3）

| 套件 | 命令 | 读数 |
|---|---|---|
| orz-loop | `cargo test -p orz-loop --lib` | **818 passed / 0 failed / 3 ignored** ✓ |
| orz-assurance | `cargo test -p orz-assurance` | 229 passed（lib）＋ 9 / 4 / 9 / 1（集成）全绿 ✓（该 crate 不依赖 orz-loop；读数取自导入面收敛前，收敛只动 orz-loop 内部测试导入，不影响其读数） |
| orz-host | `cargo test -p orz-host` | **317 passed / 8 failed**：其中 4 条 `grok_home::tests::*` 为**既有失败**（stash 到拆分前同样 4 条失败，已复现，与拆分无关）；另 4 条（`tests::call_tool_timeout_kills_process_tree` / `call_tool_with_timeout_override_is_honored` / `session_volume_symlink_windows_end_to_end` / `tool_spawn_is_really_bounded_by_the_run_job`）在串行基线读数 `321 passed / 4 failed`（仅 `grok_home`）中不存在，**疑为并行负载抖动，本轮未逐一复验**。本批不修。 |
| 格式 | `cargo fmt --all -- --check` | 退出 0 ✓ |
| clippy | `cargo clippy -p orz-loop --lib` | **未跑通**：沙箱缺 `protoc`，clippy 触发 `orz-tools-api` build script 失败（见 F1）；替代证据＝rustc 告警面 0 条 |
| 父仓门禁 | `python -m scripts.check_repository` | `valid=false, error_count=8`：7 条为冻结克隆缺 `.ter_review_2026-09-04/` 的旧文链接（搬迁前既有），1 条为 `orz submodule working tree is dirty`（＝本批刻意未提交）；**本批新增 0 条** |

## 7. 框架内部摩擦记录（本 run 实测）

- **F1 clippy 面不可用**：`protoc` 不在 PATH ⇒ 任何需重建 `orz-tools-api` build script 的 cargo 子命令（clippy、冷启动 test）都可能失败。本次 `check/test` 命中缓存得以通过；clippy 判据留待有 `protoc` 的环境复核。
- **F2 门禁在「冻结克隆」里导入原始树（严重）**：site-packages 存在指向原始树的可编辑安装（实测 `cd C:\ ; python -c "import assurance; print(assurance.__file__)"` → `D:\CLI\assurance\__init__.py`）。因此按文档口径 `python scripts/check_repository.py`（`sys.path[0]` ＝ `scripts/`）会导入**原始树**的 `assurance.run_event_journal_validation`（其 `ROOT=D:\CLI`），与克隆根做 `relative_to` 直接抛 `ValueError`（`scripts/check_repository.py:3066`）——门禁崩掉；若两侧路径恰好同形则更糟：**静默校验错误的树**。绕行：`python -m scripts.check_repository`（CWD 先入 `sys.path`）或先置 `PYTHONPATH=<克隆根>`。
- **F3 控制台编码**：`run_terminal_cmd` 回显中文为 mojibake（GBK 代码页）；`>` 重定向落盘为 UTF-16（Python 按 utf-8 读报 `0xff` 解码错）。本次绕行＝「here-string 写 `.py`（内容 ASCII）＋落盘后 `read_file` 读回」。
- **F4 折叠台账不足以检索**：`cli/.gsa/ledger/current.md` 仅存「轮号 ＋ 工具名 ＋ sha256」，无内容摘要 ⇒ 与本 run 提示词的「按行检索」期望不符；折叠后 S1/S2 的设计决策只能由 `tmp_0ai/*` 产物反推。建议折叠行附 ≤80 字符命令/结果摘要。
- **F5 计划面未承担跨折叠记忆**：本 run 两次 `blackboard_write` 后再读 `plan` / `notes` 均为空（`goal: (no goal set)` / `（无）`）⇒ 计划面失效，被迫以工作区文件作为唯一记忆。建议核查写入-读回一致性。
- **F6 搬迁遗留链接**：`docs/audits/TER_*` 旧文引用 `../../.ter_review_2026-09-04/`（克隆未随附）⇒ 门禁常驻 7 条链接错误，淹没真实错误。建议入 `存档/` 或改写链接。
- **F7 终端工具面**：本 shell 无 `grep/head/tail/sed/find`、不支持 `&&`（本次统一 `Select-String` / Python / `;` 绕行）。
- **F8 orz-host 并行测试不稳定**：Windows 进程树 / Job 对象 / 卷符号链接用例在并行负载下抖动（同一代码两次读数 321 passed / 4 failed ↔ 317 passed / 8 failed，多出的 4 条在串行读数中不存在）⇒ 该套件读数需固定 `--test-threads=1` 才可复现；多出的 4 条本轮**未逐一复验**，因果归因待复核。

## 8. 复现命令（本机）

```
# S1/S3 事实表、多重集差集与差分自检
python tmp_0ai/facts0ai.py
python tmp_0ai/selfcheck0ai.py        # 输出 tmp_0ai/selfcheck_0ai.txt
# S3 套件
cd orz ; cargo test -p orz-loop --lib ; cargo test -p orz-host -- --test-threads=1 ; cargo fmt --all -- --check
# 父仓门禁（须用 -m 形式，见 F2）
cd .. ; python -m scripts.check_repository
```

证据文件（本机工作区，非入库面）：`tmp_0ai/split_plan.txt`（S1 事实表）、`tmp_0ai/gen.py`（S2 生成器）、`tmp_0ai/split_verify.txt`（S2 验证输出）、`tmp_0ai/rustfmt_apply_diff.txt`（导入面 diff：81 行，全为 `use` 行）、`tmp_0ai/selfcheck_0ai.txt`、`tmp_0ai/test_orzloop_final.txt`、`tmp_0ai/test_orzhost_final.txt`、`tmp_0ai/gate_after_split.json`。

## 9. 未完成 / 后续

- 未提交、未推送、未重建镜像（按指令）；台账登记与计数（TODO `0ai` S1–S3 勾选、索引 `GAP-HEAVY-FILE-SPLIT` 状态、BACKLOG 0ai 条目）留待收尾批。
- clippy 判据复核（需 `protoc`）；orz-host 4 条 `grok_home` 既有失败独立立项（非本批范围）。
- `tool_run.rs`（6,784 行）仍在「千行级」：如需继续下行，可按 run_tests 家族 / 权限预算家族 / 快照家族再细拆；本批不做。
