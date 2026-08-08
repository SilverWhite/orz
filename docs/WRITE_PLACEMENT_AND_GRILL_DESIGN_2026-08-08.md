# 写入落点策略 + Grill 机制设计（2026-08-08 讨论定稿）

**状态**: 讨论定稿（未实施）。新窗口按此实施，勿重开讨论。配套 memory `fusion-phase-tracking.md`（2026-08-08 条目）。

## 背景

2026-08-08 C 盘写满事故（0 字节剩余 → Docker daemon 掉线、构建中断）。任务区产出已全落 D 盘（memory `task-area-c-drive-policy.md`），本设计将"工作区磁盘优先"内化为 orz 的通用写入策略。

## 1. 写入点盘查（实施前证据）

| 类别 | 落点 | 现状 |
|---|---|---|
| **A. 工作区树内** | `.gsa/` 全家族（journal/snapshots/keystore/permit/retention）、`.grok/roles/`（session 引导 session.rs:425）、`index.sqlite`（orz-memory `workspace_dir`） | 已符合，不动 |
| **B. 用户目录**（=C 盘，**改造对象**） | `$GROK_HOME` 家族（默认 `~/.grok`）：MCP credentials/oauth 锁/logs、hooks 信任文件（trusted_folders/disabled-hooks）、config.toml、memory 目录——全部经 `orz-config/src/paths.rs:35 grok_home()`（**已有 `GROK_HOME` env 覆盖，lazy-once + 自动建目录**） | 继承 crate 散写，一处注入全清零 |
| **C. 系统级（必要性，保留）** | Windows Credential Manager（凭据） | 保留 |
| 测试 | 生产路径无 temp_dir 写（session.rs:224/retention.rs:231 均为 `#[cfg(test)]`） | 无关 |

**前置工作（实施前必做）**：全量枚举继承 crate 剩余写点（xai-crash-handler 崩溃报告落点、orz-workspace pidfile 单实例锁落点）——"B 类归零"声明以清单为准。

## 2. 写入策略（用户裁决定稿）

**三类写点规则**：
- **A 类**（工作区相关内容）：落工作区，受"工作区树内 / 同盘优先"管制（现状保留）
- **B 类**（本体内容：配置/日志/凭据元数据/崩溃产物）：落 **orz 安装目录**（二进制所在文件夹，`current_exe` 父目录）——"orz 自身可审计性"集中于此，方便审计/清理
- **C 类**（系统必要性）：Credential Manager，保留

**实施**：直接复用继承 crate 既有 `GROK_HOME` env 机制——默认 `GROK_HOME = {安装目录}/grok-home`，进程启动早期注入（lazy-once 首次调用前）。

**降级链**（安装目录只读场景：Program Files 提权、评测容器 `/usr/local/bin` 易失）：
1. 安装目录可写 → `{安装目录}/grok-home`
2. 不可写 → `{cwd}/.gsa/grok-home`（工作区树内，仍受管）
3. 两者皆不可 → 用户目录（最后手段 + journal 警告）

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

1. 前置：全量枚举继承 crate 写点（crash-handler/pidfile 等）→ 更新 §1 清单
2. L1：GROK_HOME 注入（orz-bin/orz-codex/orz-tui 启动路径统一函数）+ 降级链 + 测试（EnvVarGuard 先例）+ 文档
3. Grill：TUI `/grill` + `/grill-finish` 命令（CommandRegistry 单一事实源）+ controller grill 模式状态 + 默认模板文件 + `{cwd}/.gsa/grill/<session>.jsonl` 记录 + ReadOnly 工具策略 + 测试
4. ADR 记录（写入策略为不可变决策；grill 为设计补充）
5. memory 更新（Q8 环境事实、裁决记录）

**明确不做**：L2 同盘检测（视 L1 实测后观察再定）、L3 journal fs_write 审计事件、grill 双模型（盲区已记录，后续补）。
