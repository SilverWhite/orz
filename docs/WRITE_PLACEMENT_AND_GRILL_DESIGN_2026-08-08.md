# 写入落点策略 + Grill 机制设计（2026-08-08 讨论定稿）

**状态**: 讨论定稿；**L1 + Grill 已实施（2026-08-08，未提交）**。**L1**：前置写点枚举（§1 ②③④ 已回填）、`orz-host/src/grok_home.rs`（`redirect_grok_home(cwd)` + `GrokHomePlacement` 四态 + 降级链 + 4 测试）、三入口接线（orz-bin/orz-codex `main` 首行 + orz-tui `runner::run` 库级自守卫，均幂等 + UserFallback stderr 警告）；冒烟 `target/debug/orz.exe -p` → `{安装目录}/grok-home` 创建成功。**Grill**：TUI `/grill`（进入：注入模板一次，`{cwd}/.gsa/grill/SKILL.md` 可覆盖）+ `/grill-finish`（总结轮 + `terminal: finished` 归档 + 状态清除）；controller `run_grill_turn`（复用 run_turn_inner 全循环，EventWriter discard 模式——**零 run-journal 事件**，反例询问 gate 跳过，历史写回跨轮续接）；host `AcpServer::run_grill_turn/finish_grill`（ReadOnly 策略强制——写工具 Deny 无弹窗；GRILL-* bootstrap 目录仅含 host 机械 run_preflight；运行中拒绝）。验证：orz-loop 124 / orz-host 98 / orz-tui 177 / orz-bin 9 / orz-codex 34 全绿（grill 新增 loop 1 / host 2 / tui 3）、clippy 零新增。新窗口按此继续，勿重开讨论。配套 memory `fusion-phase-tracking.md`（2026-08-08 条目）。

## 背景

2026-08-08 C 盘写满事故（0 字节剩余 → Docker daemon 掉线、构建中断）。任务区产出已全落 D 盘（memory `task-area-c-drive-policy.md`），本设计将"工作区磁盘优先"内化为 orz 的通用写入策略。

## 1. 写入点盘查（实施前证据）

| 类别 | 落点 | 现状 |
|---|---|---|
| **A. 工作区树内** | `.gsa/` 全家族（journal/snapshots/keystore/permit/retention）、`.grok/roles/`（session 引导 session.rs:425）、`index.sqlite`（orz-memory `workspace_dir`） | 已符合，不动 |
| **B. 用户目录**（=C 盘，**改造对象**） | `$GROK_HOME` 家族（默认 `~/.grok`）：MCP credentials/oauth 锁/logs、hooks 信任文件（trusted_folders/disabled-hooks）、config.toml、memory 目录——全部经 `orz-config/src/paths.rs:35 grok_home()`（**已有 `GROK_HOME` env 覆盖，lazy-once + 自动建目录**） | 继承 crate 散写，一处注入全清零 |
| **C. 系统级（必要性，保留）** | Windows Credential Manager（凭据） | 保留 |
| 测试 | 生产路径无 temp_dir 写（session.rs:224/retention.rs:231 均为 `#[cfg(test)]`） | 无关 |

**前置工作（2026-08-08 实施前已完成）**：全量枚举继承 crate 写点，结论：

**① grok_home 解析器共 2 个实现，均尊重 `GROK_HOME` env（一处注入全清零的关键前提）**：
- `orz_config::paths.rs:35 grok_home()`——OnceLock 缓存（**注入必须在首次调用前**）+ `GROK_HOME` env + `std::env::home_dir()` fallback；`orz_tools::util::grok_home` 是它的 re-export
- `xai_fast_worktree::db::resolve_grok_home`（mod.rs:392）——worktree 专用（worktrees 落点），`GROK_HOME` env + `$HOME` env fallback + dunce canonicalize（deliberately standalone，与 orz-config 的 fallback 差异是既有设计）

**② 产品可达 B 类写点（orz-bin 依赖树 + 调用面确认，全部落 grok_home 下）**：
- 配置：`config.toml`/`requirements.toml`/managed 缓存（orz-config loader/validation/managed_cache + orz-workspace project_config/permission resolution/claude_settings 读取）
- 信任文件 ×2：`trusted-hook-projects`/`disabled-hooks`（orz-hooks trust.rs）+ **`trusted_folders.toml`（orz-workspace trust.rs:112——orz-host session.rs:80 每 session 调用 folder_trust::decide→persist_trust，产品可达，**新发现**）**
- 会话状态：`{grok_home}/sessions`（orz-workspace permission/state.rs:194）
- worktree：`{grok_home}/worktrees/{slug}`（orz-workspace worktree/mod.rs:640/655/704 + session/git.rs:1957，经 xai-fast-worktree 解析）
- workspace server 数据：`{grok_home}/workspace`（handle.rs:4026）；codebase index（file_system/codebase_index.rs:25）；hub auth（hub_auth.rs:76）
- sandbox：`sandbox-events.jsonl`（orz-sandbox logging.rs:97）+ bwrap 占位符 `{name}.{pid}`（lib.rs:374，Unix）；hooks 槽 `hooks/`+`hooks-paths/`（orz-config global_hook_sources.rs:319/337）
- MCP（服务器 spawn 时）：`logs/mcp`（servers.rs:3951）、`mcp_auth_*.lock`（oauth.rs:247）、`credentials.json`（credentials.rs:291）
- memory.log（orz-telemetry memory_log.rs:112）；memory 目录（orz-memory storage.rs:59，workspace_dir 优先→A 类）
- 读取面：skills/agents_md/rules（orz-agent prompt）、ripgrep 配置（orz-workspace util/ripgrep.rs:19）、marketplace 缓存

**③ 产品不可达写点（记录在案，接线时按 B 类原则；当前零写盘）**：
- **xai-crash-handler**：`crash_dir` 由调用者注入（`CrashHandlerConfig`），**零产品调用**（仅自身 tests/README）
- **orz-workspace daemonize**（workspace_server bin 专用，orz-host 零引用）：pidfile `DEFAULT_PIDFILE_PATH` = Unix `/tmp/workspace-server.pid` / **Windows `C:\Windows\Temp\workspace-server.pid`（C 盘写点！）** + `DEFAULT_LOG_PATH`；若未来启用 workspace server 需改造
- **orz-agent plugins**（git_install/install_registry/marketplace 克隆写 home 插件目录、hooks_adapter 写 hooks 目录）：**零产品调用**（orz-{loop,host,tui,codex,bin} 零 `orz_agent::plugins` 引用）
- **orz-mcp servers spawn 面**（start_mcp_servers）：仅 workspace_server bin 调用（当前 toolset 无 MCP，Slice #16 确认）

**④ 产品 crate 直接调用面**：orz-host 只用 orz-workspace 的 `permission`/`trust`/`folder_trust`（其余模块经 workspace_server bin 不可达）；orz-agent/orz-mcp/orz-sandbox/orz-telemetry 仅传递依赖（编译可达、调用不可达）。

## 2. 写入策略（用户裁决定稿）

**三类写点规则**：
- **A 类**（工作区相关内容）：落工作区，受"工作区树内 / 同盘优先"管制（现状保留）
- **B 类**（本体内容：配置/日志/凭据元数据/崩溃产物）：落 **orz 安装目录**（二进制所在文件夹，`current_exe` 父目录）——"orz 自身可审计性"集中于此，方便审计/清理
- **C 类**（系统必要性）：Credential Manager，保留

**实施**：直接复用继承 crate 既有 `GROK_HOME` env 机制——默认 `GROK_HOME = {安装目录}/grok-home`，进程启动早期注入（lazy-once 首次调用前）。

**降级链**（安装目录只读场景：Program Files 提权、评测容器 `/usr/local/bin` 易失）：
1. 安装目录可写 → `{安装目录}/grok-home`
2. 不可写 → `{cwd}/.gsa/grok-home`（工作区树内，仍受管）
3. 两者皆不可 → 用户目录（最后手段 + stderr 警告）

**裁决记录**（本次讨论）：
- **Q2 落点**：B 类落安装目录（用户 Q5 指示）——不落 `.gsa` symlink 树，评测每 trial 全新问题消解（B 类在评测容器本就易失，任务不需要 MCP/trust 配置）
- **Q3 cwd 语义**：`GROK_HOME` 固定于**进程启动时 cwd**（与 `{cwd}/.gsa` 同源；TUI 探索器导航是视图切换不改变运行语义）
- **Q4 显式判定**：**env 存在 `GROK_HOME` 即尊重**（不加 ORZ_ 前缀变体——避免两套语义破坏继承 crate 读取）；当前无平台注入风险（容器只透传 ORZ_*），文档记录
- **Q6 多实例锁竞争**：接受，风险低——原子写 + 独占锁已有先例（daemonize PidFile、permit O_EXCL），B 类读多写少，继承锁机制已处理并发；写入统一 temp+rename 原子写
- **Q7 评测副作用**：接受，当前零影响——toolset 无 MCP（评审确认不可触发）、Benchmark 策略无 hooks 依赖；**前提记录**：全量 89 保持"无 MCP 任务"，未来引入 MCP 化任务时重评
- **Q8**：**环境事实，非设计决策**——"orz 随 rust 误装 B 盘、迁移时带文件夹走"是机器一次性迁移产物；记 memory 环境记录，不进 ADR，无需处理

## 3. Grill 机制（借鉴 grill-me skill，形态 C——方案定稿 2026-08-08）

**来源**：Matt Pocock `grill-me` skill（MIT，可自由借鉴需引出处）——Socratic 拷问：一次一问、每问必带推荐答案、先探索代码库、决策树深度优先、依赖排序、全部分支解析完=共享理解达成。变体 `grill-me-codex`（chaseai-yt）双模型跨提供方红队（"规划者不能给自己的作品评分——回声室"）。

**架构事实（方案前提）**：orz 无 skill 引擎（skill 是 Claude Code 生态机制，`SKILL.md` 模板 + 工具脚本）；orz 继承 Grok 的是组件（tools/workspace/sandbox/mcp）而非 CLI 本体，TUI 斜杠命令体系（CommandRegistry）自研。**grill-me 实质 = MIT 文本模板 + 对话协议，零 Claude Code 特有机制依赖**——故不是"接入 skill 接口"，而是**移植协议**（模板 + 自研状态机，成本低）。

### 命令（TUI 斜杠体系加两条，显式开启/关闭）

| 命令 | 行为 |
|---|---|
| `/grill` | 进入 grill 模式：注入模板指令（一次一问/每问带推荐/决策树深度优先/先探索再问），会话开始 |
| `/grill-finish` | 显式结束：模型输出"共享理解达成"总结 + 锁定决策清单，会话归档 |

### 模式语义（六设计点）

1. **与 run 的关系**：grill 会话**独立于 run 完整性单元**（journal=单 run 单 terminal 不变量不破坏）——设计讨论夹在 run 之间不污染主链
2. **记录**：独立 JSONL（`{cwd}/.gsa/grill/<session>.jsonl`，每轮 Q/A/推荐）——可审计且**零 schema 改动**（不动 33 事件注册表/conformance/fixtures）
3. **工具**：grill 模式下模型**只读工具可用**（read_file/grep/list_dir——"先探索代码库"是协议核心；写工具禁用）——复用已有 `PermissionPolicy::ReadOnly`（Slice #16 先例，零新机制）
4. **模板**：内置默认模板 + `{cwd}/.gsa/grill/SKILL.md` 可覆盖——改模板不发版
5. **模型**：**单模型（主 agent deepseek）起步，盲区记录在案**（生成者与审查者同分布；人类补意图层不补技术层）；双模型进阶预留：grill 会话可路由独立模型实例（gateway 三实例先例，扩展点）
6. **状态机**：grill 模式是 controller 显式状态（类似 plan mode block 先例）——每轮用户输入=对上一问的回答，模型只产"下一问+推荐"；`/grill-finish` 前模式不退出

### 回声室论证摘要

生成者与审查者同分布 → 审查问题从同一信念集生成，无法触及生成分布之外的盲区（unknown unknowns）；人类补意图层（约束/背景/方向否决）但通常补不了技术层（API 语义/生态惯例/边界条件——本次讨论 Q2-Q7 委托即例证）；跨提供方模型盲区不重叠，是技术层独立性来源。单模型+人类≠完整回声室但技术层审查仍缺独立性。**盲区为已记录限制，后续可补双模型**。

### 初版价值判断

独特价值=设计期对话式拷问（反例询问/三代理审查均不覆盖此面），契合本项目审计文化；成本≈中等 slice（命令 ×2 + 模式状态 + 模板文件 + 独立 JSONL），**零 schema 改动是关键省钱点**；显式触发非门禁 → 轻量任务零感知。

## 4. 实施顺序（新窗口）

1. ✅ 前置：全量枚举继承 crate 写点（crash-handler/pidfile 等）→ §1 清单已回填（2026-08-08）
2. ✅ L1：GROK_HOME 注入（orz-bin/orz-codex/orz-tui 启动路径统一函数 `orz_host::grok_home::redirect_grok_home`）+ 降级链 + 测试（EnvVarGuard 先例）+ 文档（2026-08-08，见头部状态）
3. ✅ Grill：TUI `/grill` + `/grill-finish` 命令（CommandRegistry 单一事实源）+ controller `run_grill_turn`（discard EventWriter + 反例 gate 跳过 + 历史写回）+ 默认模板常量 + `{cwd}/.gsa/grill/<session>.jsonl` 记录 + ReadOnly 工具策略（2026-08-08，见头部状态）
4. ✅ ADR 记录（写入策略为不可变决策——`adr/ADR-0009-write-placement-policy.md`；grill 为设计补充，不进 ADR）
5. ✅ memory 更新（Q8 环境事实已记 `disk-space-and-migration`；裁决与实施记录 `fusion-phase-tracking` 2026-08-08 条目）

**明确不做**：L2 同盘检测（视 L1 实测后观察再定）、L3 journal fs_write 审计事件、grill 双模型（盲区已记录，后续补）。

## 5. 审查记录（2026-08-08 三独立代理审查——实施后闭环）

三代理（设计合理性/实现合理性/符合性）审查结论：**无 D1/P1/C1**，核心机制语义正确。已修复项与记录项如下。

### 已修复（代码）

| 项 | 来源 | 修复 |
|---|---|---|
| `run_grill_turn` 锁跨 await（std MutexGuard 跨 await 潜在死锁 + `await_holding_lock` 警告） | 实现 P2-2 / 符合性 C2-3 | controller 调用移出 `grill` 锁（messages `mem::take` 出锁、结束写回） |
| 失败轮吞用户回答（错误路径回答不进 history/JSONL，讨论连续性断裂） | 设计 D2-5 | 错误路径将 user_input 追加进 history + JSONL 记录 `error` 字段；turn 计数仍递增（重试得新 GRILL-* 目录） |
| `try_claim_dir` 残留探针永久降级安装目录 tier | 设计 D2-1 / 实现 P2-4 | `AlreadyExists` 时删除探针重试一次（自愈；双首启竞争也覆盖） |
| GRILL-* 目录永不清理（retention 只认 RUN-/RST-，长会话磁盘累积无界） | 设计 D2-4 | retention `prune_run_dirs` 前缀白名单加 `GRILL-`（按 age 同规则清理） |
| 同 JSONL 多 episode 歧义（finish 后再进入 turn 重启） | 设计 D2-7 | `GrillSession.episode` 单调计数（`AtomicU32`）+ JSONL 记录/terminal 带 episode 字段 |
| grill 轮 `compaction_whitelist_add` 可执行（ReadOnly 类 auto-allow 却写 .gsa，"写工具禁用"不严密） | 实现 P3-4 | grill 轮不声明该工具 |
| TUI grill 轮静默冻结（inline await 无渲染） | 实现 P2-1 | await 前 `render_frame`（运行中帧）；do_cancel 在 grill_active 时提示"不可取消"（D2-6） |
| JSONL 写失败静默吞掉 | 设计 D2-9 | `tracing::warn`（保留"不 fail turn"裁决） |
| `needless_option_as_deref` ×2 | 符合性 C2-3 | `grill.as_deref_mut()` → `match &mut grill` / `if let Some(g) = &mut grill` |

### 记录项（接受/已知边界）

- **C3-1**：GRILL-* bootstrap 目录含 host 机械 `run_preflight`（无终局开链）——controller 层零事件（discard 实测），TUI tail/snapshot 扫描只认 `RUN-` 前缀不受干扰；retention 现已清理（D2-4）。"零 run-journal 事件"精确含义 = 零 controller 事件（bootstrap 机械 preflight 除外）。
- **C3-2**：事件注册表现为 34 项（A4-A6 前置提交已加 `context_compressed` 33→34）；本实施零 schema/注册表/fixtures 改动（"零 schema 改动"实质成立）。
- **C3-3**：JSONL 文件名 `<session8>`（8 字符，run 目录既有惯例）；"推荐"未单列字段——内嵌于 response 全文（模板强制"推荐: ..."）。
- **C3-4**：controller 保持 stateless-between-turns 契约，模式状态在 host `GrillSession` + TUI `app.grill_active` 两层——语义全达标，结构上状态外移。
- **C3-5**：模板在首轮 `run_grill_turn`（turn==0）注入（非 `/grill` 命令时刻）——功能等价（会话开始=首轮）。
- **C3-6**：Q6"temp+rename 原子写"约束继承 crate 既有 B 类写路径；注入机制自身用 `create_new` 独占探针（permit O_EXCL 先例），两者不冲突。
- **C3-7**：TUI 库级自守卫锚定 `config.cwd`（--run-root 会改变锚点）——生产路径 orz-bin 首行已注入故必为 EnvRespected no-op，不构成实质偏差。
- **D2-2**（记录）：dev 工作流 `cargo clean` 会清空 `target/debug/grok-home`（B 类状态失忆：config/信任/凭据元数据）——安装目录=current_exe 父目录的固有后果，用户 Q5 指示；治理方案（dev 特判上移）需用户确认，当前记录。
- **P2-3**（记录）：grill 长会话上下文无界增长（compaction 的轮判定只认工具轮，Q/A 文本属 preamble 永不丢弃；rhythm 压缩依赖 counterexample gate 不触发）——safety 压缩（>250K）对纯文本轮 noop。缓解：/grill-finish 分段、错误路径保留状态可重试（finish 失败不清除 ✓）。
- **P3-3**（记录）：GRILL_FINISH_PROMPT 以 user_input 身份进 JSONL（terminal 记录已区分阶段）。
- **P3-5**（记录）：注入块（预算/熔断/中性询问）随 history 持久化进下一轮——无害易混淆。
- **P3-6**（记录）：ReadOnly deny 测试断言较弱（`x.py` 不存在可能因路径错而非权限拒）——脚本响应固定，无法区分；已由 permission 层单测钉死 ReadOnly 语义。
- **D3 其他**：Q4 前提可失效（评测/CI 预设 GROK_HOME 时策略静默失效——记录）；untrusted-cwd 也会先创建 `.gsa/grok-home`（无害）；多二进制异目录安装配置分叉；D3-10 模板以 User role 注入（工作正常）；D3-12 grill JSONL 落 A 类工作区（C 盘工作区时仍写 C 盘——已知交互）。
