# Upstream-first 集成与范围收缩 v0.1

状态：2026-07-21 工程路线裁决；不修改 protocol v0.1、reason code、gate 或 claim 语义。
产品与术语定位见
[`PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`](PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md)。

## 1. 裁决

基于 Grok Build 继续做本项目，**不需要重做一个完整 Agent CLI**。正式实现采用：

```text
LIF 专项科学保障层
        |
        +-- Grok Build Windows prebuilt (headless first; future ACP)
        |      `-- model / session / tools / workspace / permission / sandbox
        |
        +-- launcher / Windows supervisor / DeepSeek adapter
        |
        `-- scientific assurance hard gates
               `-- source / evidence / claim / validator / scenario / leak / evaluation
```

Grok Build 已原生提供 custom models、headless structured output、session persistence、工具、权限和
sandbox。本项目只在这些能力之外保留 LIF 专项科学保障，以及 DeepSeek 与 Windows 的窄兼容桥。“保障层”
描述 Agent 为 LIF 研究服务时的来源、证据和验证约束，不表示 LIF/FEP 理论参与 runtime 控制算法。

## 2. 职责边界

| 能力 | 正式所有者 | 本仓库动作 |
|---|---|---|
| TUI、headless、ACP、session、工具、workspace | Grok Build | 直接采用，不复刻 |
| 通用 HTTP/SSE、retry、模型会话循环 | Grok Build | 直接采用；本地旧实现只作对照夹具 |
| permission 与 OS sandbox | Grok Build | 配置并做 Windows 验证，不另造通用 broker |
| DeepSeek request shape / thinking continuity | 窄 adapter 或必要的上游 patch | 先做 fake conformance，发现真实缺口后才写代码 |
| Windows 凭据注入与进程启动 | 薄 launcher | 只向 Grok 子进程注入最小环境，不接管其 runtime |
| SourceRouter、EvidenceKernel、ValidatorBridge | LIF 专项科学保障层 | 保留为产品差异层 |
| ScenarioExporter、LeakScanner、EvaluationRunner | LIF 专项科学保障层 | 保留为 hard gate，不放入 fail-open hook |

机器可读边界见 [`../upstream/grok-build.lock.json`](../upstream/grok-build.lock.json)。

## 3. 现有 prototype 的重新分类

`prototype/` 不再被视为生产 runtime 的累积实现。保留它是为了固定以下已发现的边界：

- DeepSeek thinking + tool call 的 `reasoning_content` 连续性；
- ordinary journal 的 private-reasoning 脱敏；
- Windows 取消、进程树与 bounded output 行为；
- LIF manifest、journal、scenario 与 evidence schema 的机械不变量。

以下组件冻结为 disposable conformance fixtures，不再横向扩展：通用 model transport、loopback
HTTP、fake HTTPS provider、通用 model loop、网络 permit broker 与交互 approval ledger。若 Grok
Build 已满足同一语义，正式集成直接删除对应生产需求；fixture 可继续用于回归对照。

## 4. DeepSeek 最小适配路线

Grok Build 的 `[model.*]` 已能配置 `model`、`base_url`、`env_key`、`api_backend`、token 上限和
context window。因此第一步不是实现新 transport，而是用
[`../integration/grok/deepseek-custom-model.example.toml`](../integration/grok/deepseek-custom-model.example.toml)
做 fake endpoint conformance。

当前唯一尚未证明可直接复用的关键点是 DeepSeek thinking tool-call 多轮协议：请求需要显式
thinking 配置，并在工具结果的后续请求中带回完整 `reasoning_content`。只有 source-level probe
证明上游缺失时，才增加最窄 provider adapter 或向上游提交 patch；不得先建平行会话栈。

## 5. Windows-first 路线

- 优先使用官方发布的 Windows 预编译程序，避免把 Rust workspace/source build 变成本项目门槛。
- CI 保留 Windows；Ubuntu 只作为 schema/Python portability 检查，不要求用户迁移系统。
- 薄 launcher 负责定位显式 release metadata 对应的版本、建立最小子进程环境、注入短时凭据并收集 headless 输出。
- observed baseline 用于重放，current candidate 用于升级评估；任一版本身份不匹配或 conformance 未通过时
  fail closed。不静默下载、不自动登录、不真实调用。版本策略见
  [`UPSTREAM_VERSION_STRATEGY_v0.1.md`](UPSTREAM_VERSION_STRATEGY_v0.1.md)。
- Windows 源码构建仍按上游的 best-effort 状态处理，除非以后确有必须修改上游 Rust 的缺口。

官方 Windows 安装器不是单纯解压到当前目录：它默认写入 `~/.grok`、生成 `config.toml` 和
PowerShell completion、安装 `grok.exe`/`agent.exe`，并修改用户 PATH。审计时 GitHub Releases
页面没有 artifact，binary 由 `x.ai/cli` stable channel 分发。因此不得直接执行
`irm ... | iex`；先独立下载并检查脚本，解析精确版本，确认 Content-Length/磁盘预算，下载到
git-ignored 的 `.tools/`，计算 SHA-256 后再运行。当前网络未能取得 stable 指针，所以
[`../upstream/grok-build.lock.json`](../upstream/grok-build.lock.json) 将 binary version/digest 明确标为
`unresolved`，这不会被 source commit 或 `SOURCE_REV` 冒充替代。

## 6. 下一 implementation spike

安装检查、无密钥 fake provider、DeepSeek 两轮 reasoning continuity 与 Windows Job Object 接入已经完成。
workspace trust、local trace/export、event bridge 与 hash-only fixture checkpoint/delta 也已完成首轮机械验证。
进一步复核上游后，下一步不是增加新的通用 Agent 组件，而是：

1. no-model Global Progress Sentinel fixture 已完成：从计划/journal 生成全局摘要、WARN 与结构化 disposition；
2. no-model ACP `initialize`/capability 探针已完成：protocol v1、capability/meta、扩展通知、Job Object、
   全出站阻断与独立 verifier 均有 Windows 实测；
3. ACP structured tool/permission/cancel/terminal continuity 的双场景 launcher、Schema、session-file 对账与独立
   verifier 已通过 synthetic/tamper tests 和管理员级 Windows fake-only live smoke；
4. 用独立 shadow Git fixture 承担恢复，现有 hash-only checkpoint 继续作为 audit receipt；
5. child-tree timeout/cancel/parent-exit 管理员矩阵已完成，candidate `0.2.111` 通过并提升为默认；
6. manual `/compact` provenance 已完成 fake-only observed 验证：lifecycle、source span/digest、request、
   checkpoint、`derived_unverified` 和 unknown mapping 边界均由独立 verifier 重放；
7. 这些边界通过后，用户于 2026-07-23 单独授权的最小真实 DeepSeek transport
   conformance 已取得 HTTP 200、模型/marker 匹配和 1 request / 0 retry；
8. 正式 Grok 薄 launcher 已完成 two-stage plan、固定 binary/model/endpoint、Credential
   Manager 内部注入、隔离 profile/workspace、Job Object、双 trust receipt、无工具/
   debug/memory/subagent/web 和 artifact leak scan 的实现；
9. 用户于 2026-07-23 单独授权的一次真实 Grok→DeepSeek attempt 已在 Grok session 中取得
   `deepseek-v4-pro`、固定 marker、`completed`、1 turn / 0 tools / 0 errors；模型成功后的
   terminal scan 因内置帮助文档的 Bearer 占位符误报而 fail closed，扫描器已离线修复且未
   自动补跑第二次付费请求。

六个开源 Agent 的固定版本比较、取舍理由与 source ledger 见
[`OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`](OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md)。其中 repo map、
architect/editor 双模型和云端 sandbox 明确延期；不迁入旧研究 `index/map/self-check`。
职责深拆与 ACP-first 收缩见
[`MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md)。
防止单方向过推进的全局回看层见
[`GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`](GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md)。

## 7. 官方依据

- [Grok Build custom models](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/11-custom-models.md)
- [Official Grok Build Windows installer](https://x.ai/cli/install.ps1)
- [Grok Build headless mode](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/14-headless-mode.md)
- [Grok Build sessions](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/17-sessions.md)
- [Grok Build sandbox](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/18-sandbox.md)
- [Grok Build permissions](https://github.com/xai-org/grok-build/blob/main/crates/codegen/xai-grok-pager/docs/user-guide/22-permissions-and-safety.md)
- [DeepSeek official first API call](https://api-docs.deepseek.com/)
- [DeepSeek thinking mode](https://api-docs.deepseek.com/guides/thinking_mode/)
