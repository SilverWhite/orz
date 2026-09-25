## 快速开始

### 前提

- Windows x86_64 或 Linux x86_64；
- DeepSeek API Key。

（注：目前 Linux 仅提供构建产物，未做专门体验适配与优化；其他厂商模型与api暂时未适配）

发布包包含三个程序：`orz`（主程序）、`orz-signer`（安全签发器）、`orz-acaf-provision`（一次性初始化工具），需要放在同一目录。Web 工作台（0.6.13 起）的静态前端内嵌在 `orz` 二进制内，`orz web` 只监听回环地址、需启动时打印的令牌访问。

## 配置方式

请按下表配置 Windows 环境与 Linux 环境：

| 步骤 | Windows（PowerShell） | Linux（sh） |
|---|---|---|
| 1. 解压 | 把 `orz.exe`、`orz-signer.exe`、`orz-acaf-provision.exe` 放入同一目录（例如 `C:\orz`）。 | `mkdir -p ~/orz && cd ~/orz`<br>`tar -xzf orz-0.6.13-linux-x86_64.tar.gz`<br>`chmod +x orz orz-signer orz-acaf-provision` |
| 2. 配置 API Key | 存入 Windows 凭据管理器（Generic，目标名 `orz-deepseek/agent`；一次即可）：<br>`cmdkey /generic:orz-deepseek/agent /user:agent /pass:你的DeepSeek_API_Key` | 用环境变量（Windows 凭据管理器通道的显式例外）：<br>`export ORZ_DEEPSEEK_API_KEY=你的DeepSeek_API_Key` |
| 3. 初始化安全签发（一次性） | `.\orz-acaf-provision.exe "$env:USERPROFILE\.orz-acaf\keystore" "$env:USERPROFILE\.orz-acaf\signer-manifest.json"`<br><br>ACAF 默认 fail-closed，未配置会拒绝启动。 | `./orz-acaf-provision "$HOME/.orz-acaf/keystore" "$HOME/.orz-acaf/signer-manifest.json"` |
| 4. 设置启动环境 | `$env:ORZ_ACAF_KEYSTORE = "$env:USERPROFILE\.orz-acaf\keystore"`<br>`$env:ORZ_ACAF_MANIFEST = "$env:USERPROFILE\.orz-acaf\signer-manifest.json"`<br>`$env:ORZ_ACAF_BINARY = "C:\orz\orz-signer.exe"` | `export ORZ_ACAF_KEYSTORE="$HOME/.orz-acaf/keystore"`<br>`export ORZ_ACAF_MANIFEST="$HOME/.orz-acaf/signer-manifest.json"`<br>`export ORZ_ACAF_BINARY="$HOME/orz/orz-signer"` |
| 5. 运行 | `.\orz.exe`（交互 TUI）<br>`.\orz.exe -p "你的任务" --real`（无头模式）<br>`.\orz.exe web`（Web 工作台，打印本地带令牌地址） | `./orz`（交互 TUI）<br>`./orz -p "你的任务" --real`（无头模式）<br>`./orz web`（Web 工作台，打印本地带令牌地址） |

发布包说明与完整性校验见 GitHub Release（最新 [v0.6.13](https://github.com/SilverWhite/CLI/releases/tag/v0.6.13)，双平台包＋`SHA256SUMS`）；0.1.0–0.5.1 试用包入口在 [`releases/`](releases/)。

### 从源码运行

Rust toolchain 由 [`orz/rust-toolchain.toml`](orz/rust-toolchain.toml) 固定；在 `orz/` 工作区内构建：

```powershell
cargo build -p orz-bin
cargo run -p orz-bin -- --fake-provider -p "hello"   # 离线试跑，无需凭据
cargo run -p orz-bin -- --real -p "你的任务"           # 真实 DeepSeek transport
cargo run -p orz-bin -- --fake-provider               # TUI
```

### 常用入口

| 场景 | 命令 |
|---|---|
| 交互 TUI | `orz` |
| Web 工作台（本地回环，启动时打印带令牌地址） | `orz web` |
| 无头执行 | `orz -p "<任务>" --real` |
| 计划模式（先记录计划再执行） | `orz --plan -p "<任务>" --real` |
| ACP stdio server | `orz --stdio` |
| 只读回放 journal | `orz --replay <events.jsonl>` |
| 离线试跑（无需凭据） | `orz --fake-provider -p "hello"` |
| 联网检索开关 | `orz --retrieval-enabled`（默认关闭；旧 `--retrieval-mode` 已废弃，仅兼容解析） |

联网检索默认关闭；需要联网的任务请显式加 `--retrieval-enabled`（独立检索启用门，fail-closed）。启用后由外部检索子代理执行：本地浏览器通道优先（引擎 SERP，Google 主序、Bing 回退），浏览器不可用时由模型自主改走原生 web 检索通道，启动结果与换道在事件链留痕（`browser_launch_result`）。

无头与批量场景的权限开关：

- `--allow-write`：允许修改本地文件；Bash 与网络仍默认拒绝。
- `--allow-shell` / `--allow-network`：打开评测/批量场景的 shell 与网络轴；必须与 `--allow-write` 同用，否则报错退出。
- `--max-wallclock <秒>`：整轮墙钟上限；超时以 `run_invalidated` 终态结束，不做进程级硬杀。

常用调优环境变量：`ORZ_STALL_TIMEOUT`（卡住看门狗，默认 360 秒）、`ORZ_TOOL_TIMEOUT_SECS`（单工具超时，默认 300 秒）。`ORZ_ACAF_FAIL_CLOSED=0` 可临时关闭安全层，但不推荐用于正式工作。

## 框架介绍

orz 为**本地优先**、**保障优先**、**直接进入真机而非沙箱环境**的终端 AI 编程 Agent/harness，制作全程使用 AI coding。

其中，控制面、Agent loop 与保障体系为自研内容，除此之外，框架内直接复用了部分 [`grok-build`](https://github.com/xai-org/grok-build) 中已成熟的工具与工作区组件，参考了 [`codex`](https://github.com/openai/codex) 的代码设计语言。执行侧服务调用形态大量借鉴了 Home Assistant (https://github.com/home-assistant) （`domain.service + target + data`），并少量参考了 [`deepseek-harness`](https://github.com/deepseek-ai/deepseek-harness) 与其他成熟产品。Web 工作台UI形态取自本仓三份 UI 设计稿；但外观实现直接照搬旧时代桌面主题 [`98.css`](https://github.com/jdan/98.css) 与 [`XP.css`](https://github.com/botoxparty/XP.css)；Markdown 渲染实现照搬 [`marked`](https://github.com/markedjs/marked)。

整体架构可主要分为两大块两小块。
两大块为**Agent 层**与**机械层**，两小块为作为核心面板的**黑板**和外挂的**时间与动作域判断组件**。

### Agent 层

- **主 Agent**：唯一任务推进者。系统提示近零，面对冻结的固定 10 工具面：`read_file`/`grep`/`search_replace`/`run_terminal_cmd`/`web_search`/`web_fetch`，加 `blackboard_read`、`submit`、`blackboard_write`（向黑板计划/笔记区写入，单条 ≤8K）与 `context_compress`（知情发起模型参与压缩）；默认模型注册为 DeepSeek v4 flash（thinking 默认 max）。任务最终经 submit 两阶段（请求 → 确认）交付，终答前有一轮机械审计与反例自查。
- **外部检索子代理**：联网检索经外部检索子代理执行（`web_search` 全局并发 1），未启用检索时关闭（启用见上方 `--retrieval-enabled`）。启用后子代理工具面恒注册本地浏览器与原生 web 双族检索工具（带车道名与推荐序的静态标注，本地浏览器优先），换道由模型自主选择：本地浏览器通道走引擎 SERP（Google 主序、Bing 回退；人化输入延迟＝逐字符键入 + 提交前停顿 + Enter，对模型不可见），浏览器启动可用性以事实事件（`browser_launch_result`）在事件链留痕。内部检索 lane 保留设计，触发工具当前封存。
- **会话与计划**：交互会话（TUI/ACP）可跨进程恢复；一次性 `-p` 不开启跨调用恢复，但同样落会话持久化，并按里程碑增量归档到 `.gsa/archives/`。`--plan` 提供机械计划状态机工作流，生产路径中 plan_first 休眠。

### 机械层

- **结构**：机械层承载全部机制、门禁与守卫；其执行侧可进一步拆解为**半助理层**（命令运行、写执行与检索派发，返回有界结构化结果）与**静默机械审查层**（运行中只记录审查事实、终答前给出事实报告，不给建议）。
- **执行**：模型直接提议工具调用，机械层按注册表路由 → 目标/契约校验 → 执行 → 验证逐层处理。命令、文件写入与联网访问（`web_fetch`/`browser_read`）先过权限与 ACAF 票据门，`web_search` 无 URL 目标不走票据；文件读写带内容锚点核证；去自身硬超时，长前台命令超阈（默认 180s）自动后台化并维持输出/CPU 活跃兜底（idle-kill）；失败由半助理层自动记录（进程/文件/环境实体登记），返回结构化错误信封（step/code/message/trace_id）。
- **安全**：指令来源门（IPG）、权限桥、ACAF（`orz-signer` 独立进程签发一次性票据，未配置即 fail-closed）、凭据目标注册与脱敏、URL 门禁与来源加权、检索候选计数。
- **审计、状态、上下文**：每次运行写入 hash-chained 事件 journal（事件 schema v0.2）并经 verifier 交叉校验；机械审计事实报告、会话黑板单包归档；上下文由机械滑窗与模型共同承接——模型面是自控注意力窗口（主滑块＋主滑块以外的分块指针，分块内容不流出模型面），机械按阶梯收窄模型面（软提醒 → 320K 硬打断 → 500K 硬截断），语义压缩经压缩窗口由模型产出结构化摘要（可经 `context_compress` 知情发起）；压缩不覆盖本地面，全量留档、按块回放；会话可恢复、journal 可 `--replay` 只读回放。
- **生成期守卫与轮预算**：复读检测（滚动哈希 + 3-gram 兜底）、空响应重试链、stall 看门狗（`ORZ_STALL_TIMEOUT`，默认 360 秒无活动即收尾）与整轮墙钟上限；轮预算默认无限制（`MAX_TOOL_ROUNDS=0`，撤除默认 120 轮硬限）；问询均为软门、不禁工具：首轮动作批次结束后一次性注入开局三问（方向自校验），此后每满 50 轮触发一次简短中立三问，询问动作目标与进度。

### 黑板

黑板是主 Agent 与机械层共用的单会话状态面板：分区保存计划、执行动作、实体（文件/进程/环境）、会话与门禁记录。主 Agent 通过 `blackboard_read` 按需读取（PULL），不常驻提示词；模型可经 `blackboard_write` 向计划/笔记区写入（单条 ≤8K），盖章、发放与归档仍由机械层完成；黑板为单会话作用域（conversation-scoped，旧 plan-epoch 生产语义已退役），写时按 `(domain, round)` 盖章；`blackboard_read` 按需进行域与轮数的折叠渲染（render fold），响应头携带黑板水位（【x.xM/10M】）与「滑块外可压缩 N 块」读数；交互会话结束时由 `session_archive` 打包为单 gzip 归档文件，无头 run 按里程碑增量归档。

### 时间与动作域判断组件

机械层维护本会话的时间性参考系（LIF）：持续计算时间、进度、错误率与“是否卡住”等特征（T̂、u_prog/u_err/u_stuck），按动作特征来做粗粒度的域区分并记录域切换（spike，随会话侧车存档）。计算与触发全在机械层完成、模型无感；模型需要时经 `blackboard_read` 的时间查询面（now/recent/history/feature）按需读取。fires 只内部留痕，不注入模型面——长任务的时间感知由框架替模型记账。

### 载体与组件

- 入口：`orz` 一个程序承载 TUI、`-p` 无头、`--plan`、`--stdio`（ACP）与 `--replay`；`orz-signer`/`orz-acaf-provision` 只用于安全层初始化（见上方配置）。
- Rust 生产 workspace（`orz/`）：`orz-loop`（Agent loop、黑板与守卫）、`orz-host`（工具执行、权限桥、凭据、本地浏览器）、`orz-assurance`（journal、事件、ACAF、verifier）、`orz-bin`（CLI 入口）、`orz-tui`（终端工作台）、`orz-web`（Web 工作台：回环桥＋内嵌静态前端，`orz web` 入口）。
- 支撑体系：`assurance/` 为 Python reference/conformance 参考；`runtime/` 为事件 Schema；`protocol/` 为结构化操作协议草案。

一次运行的路径大致是：入口 → 会话与 journal 初始化 → 主 Agent 轮次（近零提示 + 冻结 10 工具面）→ 工具直接调用执行 → 机械层权限/票据门 → 执行与检索 → 结果与事件回流 → submit 两阶段交付 → journal 收尾。之后可以 `--replay` 回放或恢复会话复查。

机制的完整状态、稳定 ID 与深入入口见下方「开发者入口」；设计权威为 [`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)，当前投影在 [`architecture/current/README.md`](architecture/current/README.md)。

## 开发者入口

- 全项目路由、状态与稳定 ID：[`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)
- 当前设计权威：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)
- 当前架构投影：[`architecture/current/README.md`](architecture/current/README.md)
- 统一待办：[`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) / [`TODO.md`](TODO.md)
- 实施审计：[`docs/audits/`](docs/audits/)
- Python reference/conformance：[`assurance/README.md`](assurance/README.md)
- 历史 README 快照：[`存档/readme/README.md`](存档/readme/README.md)

### 当前状态

- **设计**：ADR-0010 是唯一自然语言设计权威，`accepted / frozen`。
- **实现**：Rust production workspace 可运行，当前整体 `partial`；未闭合差距集中登记在 [`CLI_PROJECT_INDEX.md` §3.1](CLI_PROJECT_INDEX.md#31-已登记实现差距)，不在本 README 展开。
- **发布**：0.1.0–0.5.1 试用发布包入口在 [`releases/`](releases/)；0.5.4 起双平台安装包发布于 [GitHub Releases](https://github.com/SilverWhite/CLI/releases)（当前最新 v0.6.13，Windows zip／Linux tar.gz＋`SHA256SUMS`；0.6.13 起载体内嵌 Web 工作台，`orz web` 即起本地回环界面）；当前未提供 macOS 原生包。
- 测试全绿或单次跑分不构成架构符合性结论；符合性状态以索引与审计为准。

## License

Apache License 2.0。见 [`LICENSE`](LICENSE) 与 [`NOTICE`](NOTICE)。

第三方与 vendored 件的许可与来源另见：[`orz/THIRD-PARTY-NOTICES`](orz/THIRD-PARTY-NOTICES)（crate 依赖、vendored 源码移植，以及 Web 工作台内嵌静态件 98.css／XP.css／marked 与 Pixelated MS Sans Serif 字体）、[`组件册`](upstream/fusion-component-register-v0.1.yaml)、以及内嵌前端资产的逐件摘要 [`vendor/MANIFEST.sha256.txt`](orz/crates/orz-web/assets/vendor/MANIFEST.sha256.txt)。
