# ORZ / GSA 全项目宏观架构与工程完整性审查报告

> **审计日期**：2026-09-04  
> **审查性质**：全项目宏观架构、产品真实面、门禁完整性与代码质量联合审计（本地环境实测 + Grok 4.6 架构审查综合审定）  
> **审计状态**：`CRITICAL / ACTION REQUIRED`（核心门禁破损，权威与代码严重脱节，仓库卫生失控）  
> **前置决策**：在继续推进 TER（工具执行层改革）M1/M2 遗留实施之前，必须优先对齐本审查暴露出的结构性缺陷。
>
> **状态更新（2026-09-04 收口）**：Phase 1–3 与 Phase 4 任务 A/B/C 均已闭合
> （门禁修复、仓库卫生、权威对齐、render_fold 解耦、僵尸 crate 64→49、
> 读工具 canonical 沙箱 + ACAF fail-closed 下沉）；任务 D（双实现终局治理）
> 仍开放。闭合证据与三轴复核见
> [`P0-GOV 收口审计`](P0_GOV_UNCOMMITTED_REVIEW_HANDLING_2026-09-04.md)。
> 原文为审查时快照，保留不删。

---

## 1. 审查总评与核心判断

项目在理论机制设计（ACAF 票据、不可篡改 Journal、LIF 时间认知外挂、黑板会话折叠、输出健康哨兵）上具有极高深度，且实际存在一条**“可跑的自研控制面主链”**。

然而，项目当前存在致命的**“权威分裂、门禁崩溃、底座黑盒、双实现未消、撤回机制死代码滞留、安全边界虚标、仓库极度脏乱”**等 7 大系统性结构问题。
最为严峻的现状是：**设计权威（ADR-0010 / 索引）描述的产品 ≠ 代码里的实际产品**，且自动化门禁脚本 [scripts/check_repository.py](../../scripts/check_repository.py) 已经报错崩溃（Exit Code 1，62 处错误），全项目正处于“盲目向前叠设计，底层防线已失守”的危险边缘。

---

## 2. 真实产品面（当前代码真正跑通的主链）

经代码与运行链路交叉核验，目前仓库中**真正成立且运行的主产品面**为：

- **系统提示词**：近零系统提示，去除拟人化包裹。
- **工具面**：冻结的固定 8 工具（`read_file`, `grep`, `search_replace`, `run_terminal_cmd`, `web_search`, `web_fetch`, `blackboard_read`, `submit`），直接（direct）执行。
- **交付机制**：`submit` 两阶段协议（请求 → 确认），终答前静默机械审计与反例自查。
- **审计面**：hash-chained 事件 journal（schema v0.2），单向哈希防篡改。
- **安全与权限**：ACAF Slice 1–2 票据（写、命令、fetch）在 CLI 入口强校验（fail-closed）。
- **运行守卫**：生成期输出健康哨兵（滚动哈希复读检测 + stall 活跃看门狗）。
- **上下文管理**：机械会话黑板 + 渲染折叠与近窗快照（P2-13/P2-14）。

**已经休眠、撤回或与当前代码脱节的非真实设计**：
- `plan-first` 硬门（生产路径默认关闭）。
- `console` 动作栏代理下单（已转为 8 工具直接执行）。
- 120 轮硬限（代码中 `MAX_TOOL_ROUNDS=0`，已撤销硬限改为无限轮预算）。
- `plan-epoch` 周期轮换（生产语义已由会话黑板单实例取代）。

---

## 3. 结构性问题清单（按严重度排序）

### 【问题一】权威分裂：设计权威与实现严重脱节
1. **ADR-0010 描述了错误的产品**：
   - ADR-0010 正文与架构索引至今仍写着“120 轮预算限制”、“plan epoch 轮换”、“console 动作台账下单”，并标记为 `implemented` / `current-design`。
   - 实际代码中：`MAX_TOOL_ROUNDS = 0`，`plan_first` 默认关闭，8 工具直接调用，黑板为单会话生命周期。任何人阅读 ADR-0010 都会理解错系统形态。
2. **当前架构投影为空壳**：
   - [`architecture/current/README.md`](../../architecture/current/README.md) 缺少与当前代码一致的状态机和交互拓扑。

### 【问题二】核心自动化门禁脚本崩溃（62 处错误）
执行 `python scripts/check_repository.py` 直接返回 Exit Code 1，阻断项多达 62 个：
1. **源码清单全面失效**：
   - 7 个新增的 Rust 源文件未登记于 [`orz_source_manifest.sha256`](../../orz_source_manifest.sha256)（包括 `crates/codegen/orz-tools/src/computer/output_object.rs`, `crates/orz-host/src/env_snapshot.rs`, `crates/orz-loop/src/env.rs`, `processes.rs`, `render_fold.rs` 等）。
   - 50+ 个文件的内容 SHA256 已经与 manifest 脱节。
2. **测试夹具（Fixtures）契约分歧**：
   - T0.2 引入的 3 个 v0.2 payload 夹具（`tool-completed.output-object.valid.json`、`tool-running.idle-killed-missing-reason.constraint.invalid.json`、`tool-running.idle-killed.valid.json`）在检查器中未登记映射（unmapped）。
3. **Markdown 相对路径死链**：
   - [docs/CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN_2026-09-04.md#L246](../CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN_2026-09-04.md#L246) 引用了错误的相对路径（`docs/audits/...` 在 `docs/` 目录下解析为 404）。
   - [docs/audits/P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md#L92](P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md#L92) 存在类似相对路径嵌套错误。

### 【问题三】双实现未除，两套法官并存
- 2026-08-13 架构裁决明确要求“砍除双实现、降低复杂度”。
- 现状：Python 侧（`assurance/`）仍旧保留了独立的 schema 校验、journal verifier、HIGH-NIST sandbox 以及 `gsa` CLI；而 Rust 侧（`orz-assurance`）另有一套独立的 journal/gates。两者互为异构镜像，导致每次数据格式变更必须双重维护，极易产生验证分歧。

### 【问题四】65 个 Crate 全 `audit_required`，Grok 遗留底座黑盒
- Rust 工作区包含多达 40+ 个 crates（全量依赖包含 65+）。
- 自研核心仅约 6 个（`orz-loop`, `orz-host`, `orz-assurance`, `orz-bin`, `orz-tui`, `orz-codex`），其余绝大部分由历史 Grok 代码改名借用。
- 至今未建立清单证明哪些模块位于真实生产路径，哪些属于死依赖。

### 【问题五】安全面存在空子与虚标
1. **ACAF 保护边界外露**：
   - ACAF fail-closed 强校验仅是 `orz-bin`（CLI 入口）的特性；在 `controller`、`ACP` 以及 `TUI` 库层面，依然默认允许 shadow/bypass，未在核心执行器层强制收敛。
2. **工具权限虚标**：
   - `read_file` 实际无需票据且可读取工作区（cwd）以外的任意非隐藏文件，但注释声明“受 ACAF 覆盖”。
   - `web_search` 故意无票且检索逻辑绕过了 permission bridge。
   - HIGH-NIST 沙箱仅在 Python 原型与测试脚本中存在，未纳入 ACAF Slice 4 正式体系。

### 【问题六】撤回机制死代码大量霸占代码库
1. **历史代码未及时下线**：
   - [crates/orz-loop/src/epoch.rs](../../orz/crates/orz-loop/src/epoch.rs) 占用约 3300 行代码，`--plan` 依然在维护退役的 epoch 轮换逻辑，与会话黑板（`blackboard.rs`）并存，形成两套计划状态机。
   - Diagnostic Coverage（DC）事件类型在 Rust 侧依然可产生，但 Python schema v0.2 会直接拒收。
2. **上帝文件重现**：
   - [crates/orz-loop/src/host_exec.rs](../../orz/crates/orz-loop/src/host_exec.rs) 膨胀至约 7300 行，重新成为未拆分的庞大单体。

### 【问题七】Rust 编译告警与代码粗糙
- [`crates/orz-loop/src/host_exec.rs:187`](../../orz/crates/orz-loop/src/host_exec.rs#L187)：`pub(crate) async fn run_host_tool` 出现 `dead_code` 编译警告（生产路径已改为 wrapper 调用，遗留基础方法未标记 `#[cfg(test)]`）。
- [`crates/orz-host/src/local_browser/mod.rs:670`](../../orz/crates/orz-host/src/local_browser/mod.rs#L670)：`total_chars = KEYWORD_TOTAL_CHARS;` 后立即 `break`，产生 `unused_assignments` 告警。

### 【问题八】仓库卫生极度混乱与 Git 结构脆弱
1. **根目录沦为临时垃圾场**：根目录堆积了 25+ 个 `tmp*` 临时目录、调试文件（`HTTP`、`%{http_code}`）、临时脚本（`tmp_analyze_tokens.py` 等）与 1.3MB 巨大 JSON 回放日志。
2. **未跟踪测试日志泛滥**：`_windows_high_nist/` 堆积数十个未提交的批处理执行记录（`job-wf12-probe-*.txt` 等）。
3. **脆弱的同源 Git 子模块**：外层主仓（`SilverWhite/CLI:main`）嵌套了同源子模块（`orz` 指向同一仓库的 `feat/fusion-architecture`），极易引起提交混乱和分支漂移。

---

## 4. 处置决策与整改步骤（在继续 TER M1/M2 之前优先执行）

按照“先通门禁、再理权威、再清垃圾”的顺序，排定以下整改行动：

### Phase 1: 门禁与编译紧急修复（立即执行）
1. **修复 Markdown 断链**：
   - 修正 `docs/CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN_2026-09-04.md` 第 246 行链接为 `audits/P2-14_S2_E2E_2026-09-04.md`。
   - 修正 `docs/audits/P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md` 第 92 行链接为 `P2-13_B1_REVIEW_HANDLING_2026-09-03.md`。
2. **补齐门禁测试夹具映射**：
   - 在 `scripts/check_repository.py` 中登记新增的 3 个 v0.2 payload fixtures（`tool-completed.output-object.valid.json` 等）。
3. **重新生成源码指纹**：
   - 执行 `python scripts/generate_orz_source_manifest.py`，更新 `orz_source_manifest.sha256`。
4. **消除 Rust 编译告警**：
   - 修复 `host_exec.rs` 的 `run_host_tool` 未使用告警。
   - 清理 `local_browser/mod.rs` 的无效赋值。
5. **执行门禁验收**：确保 `python scripts/check_repository.py` 退出码为 0（PASS）。

### Phase 2: 仓库卫生清理与 Git 规范化
1. 清理根目录多余的 `tmp*` 临时目录、`HTTP`、`%{http_code}` 等一次性调试产物。
2. 规范 `.gitignore`，将 `_windows_high_nist/` 下的 `job-*.txt`、`job-*.ps1` 批量运行日志列入忽略。

### Phase 3: 权威对齐与状态登记
1. 在 [CLI_PROJECT_INDEX.md](../../CLI_PROJECT_INDEX.md) 中登记本次宏观审计结论，纠正设计与代码状态。
2. 在 [docs/BACKLOG_AND_PRIORITIES.md](../BACKLOG_AND_PRIORITIES.md) 与 [TODO.md](../../TODO.md) 中建立专门治理小节，标记为当前阻断性前置任务。
3. 更新主 [README.md](../../README.md)，移除过时的“epoch 轮换”描述，与当前“会话黑板单实例 + 8 工具直接执行”的代码事实统一。
