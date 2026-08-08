# ADR-0009：写入落点策略（B 类本体内容落 orz 安装目录）

- 状态：accepted
- 日期：2026-08-08
- 关联：`docs/WRITE_PLACEMENT_AND_GRILL_DESIGN_2026-08-08.md`、`orz-host/src/grok_home.rs`、`orz-config/src/paths.rs`、`xai-fast-worktree/src/db/mod.rs`、memory `task-area-c-drive-policy.md`

## 1. 背景

2026-08-08 C 盘写满事故（0 字节剩余 → Docker daemon 掉线、构建中断）暴露本体内容
（配置/日志/凭据元数据/信任文件）默认散落用户目录（Windows = C 盘）的问题。任务区
产出已全落 D 盘（memory `task-area-c-drive-policy.md`），本 ADR 将"工作区磁盘优先"
内化为 orz 的通用写入策略——B 类写点统一收拢到 orz 安装目录，一处注入全清零。

## 2. 决策

### 2.1 三类写点规则（不可变）

| 类别 | 内容 | 落点 |
|---|---|---|
| **A 类** | 工作区相关内容（`.gsa/` 全家族、`.grok/roles/`、orz-memory `workspace_dir`） | 工作区树内（现状保留，"工作区树内/同盘优先"管制） |
| **B 类** | 本体内容（`$GROK_HOME` 家族：config.toml/requirements.toml/凭据元数据/oauth 锁/logs/hooks 信任文件/trusted_folders.toml/sessions/worktrees/memory 目录等） | **orz 安装目录**（`current_exe` 父目录）——"orz 自身可审计性"集中于此 |
| **C 类** | 系统必要性（Windows Credential Manager 凭据） | 保留（GAK-CRED-001 / ADR-0006 机制不变） |

### 2.2 注入机制（不可变）

- 直接复用继承 crate 既有 `GROK_HOME` env 机制：默认 `GROK_HOME = {安装目录}/grok-home`，
  进程启动早期注入（`orz_host::grok_home::redirect_grok_home(cwd)`，三入口
  orz-bin/orz-codex `main` 首行 + orz-tui `runner::run` 库级自守卫，均幂等）。
- **两个 grok_home 解析器均尊重 `GROK_HOME` env**（`orz_config::grok_home()` OnceLock +
  `xai_fast_worktree::resolve_grok_home()` worktree 专用）——一处注入覆盖全部 B 类写点
  （写点清单见设计文档 §1 ②）。
- 注入时机纪律：`grok_home()` 是进程级 OnceLock，**注入必须在首次调用前**（启动早期）。

### 2.3 降级链（安装目录只读场景：Program Files 提权、评测容器 `/usr/local/bin` 易失）

1. 安装目录可写 → `{安装目录}/grok-home`
2. 不可写 → `{cwd}/.gsa/grok-home`（工作区树内，仍受管）
3. 两者皆不可 → 用户目录（最后手段 + stderr 警告）

## 3. 裁决记录（2026-08-08 讨论）

- **Q2 落点**：B 类落安装目录——不落 `.gsa` symlink 树（评测每 trial 全新问题消解；
  B 类在评测容器本就易失，任务不需要 MCP/trust 配置）。
- **Q3 cwd 语义**：`GROK_HOME` 固定于**进程启动时 cwd**（与 `{cwd}/.gsa` 同源；
  TUI 探索器导航是视图切换不改变运行语义；`--run-root` 不重新锚定）。
- **Q4 显式判定**：env 存在 `GROK_HOME` 即尊重（不加 `ORZ_` 前缀变体——避免两套语义
  破坏继承 crate 读取）；当前无平台注入风险（评测容器只透传 `ORZ_*`）。
- **Q6 多实例锁竞争**：接受，风险低——原子写 + 独占锁已有先例（daemonize PidFile、
  permit O_EXCL），B 类读多写少，继承锁机制已处理并发；写入统一 temp+rename 原子写。
- **Q7 评测副作用**：接受，当前零影响——toolset 无 MCP（评审确认不可触发）、Benchmark
  策略无 hooks 依赖；**前提记录：TB 全量 89 保持"无 MCP 任务"，未来引入 MCP 化任务时
  重评**。
- **Q8**：环境事实非设计决策（orz 随 rust 误装 B 盘、迁移时带文件夹走是机器一次性产物）
  ——记 memory 环境记录，不进 ADR 决策。

## 4. 实施与验证（2026-08-08，L1）

- `orz-host/src/grok_home.rs`：`redirect_grok_home(cwd)` + `GrokHomePlacement` 四态
  （`EnvRespected`/`InstallDir`/`WorkspaceFallback`/`UserFallback`）+ `try_claim_dir`
  （create_dir_all + 独占探针文件证明 ACL 可写）+ 4 测试（env 尊重/安装目录可写/降级/
  双不可写）。
- 冒烟：`orz.exe -p` → `{安装目录}/grok-home` 创建成功（fake provider）。
- 遗留：`xai-crash-handler`（crash_dir 调用者注入，零产品调用）与 orz-workspace
  daemonize pidfile（`C:\Windows\Temp\workspace-server.pid`，workspace_server bin 专用）
  为**产品不可达写点**，记录在案（设计文档 §1 ③）；若未来启用 workspace server 需按
  本策略改造。
