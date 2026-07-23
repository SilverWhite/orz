# 可观测 Grok execution envelope v0.1

状态：Windows-first implementation contract；优先准确性与可审计性，不修改 protocol v0.1 语义。

## 1. 目标与边界

本项目要控制和记录的是**可观察执行过程**，不是猜测模型脑内过程。wrapper 应保留输入、来源、配置、
模型/工具事件、权限裁决、进程状态、文件变化和最终产物，使“未知”能够保持未知，并让每个结论回指到
可检查事实。

即使 provider 暴露 `thought`/`reasoning_content`，它也只是 provider-private diagnostic：不得被当成
evidence、precommitment 或正确性证明。API key、credential value 和无关环境变量永不记录。

## 2. 运行前冻结

复用既有 `runtime/run-manifest-v0.1.schema.json`，每个新 run 在启动 Grok 前固定：

- workspace trust/restricted receipt，以及将被发现的项目配置、规则、hook、plugin、skill 与 MCP digest；
- Grok binary version/build ID/SHA-256/Authenticode verdict；
- workspace root、Git commit/dirty summary、输入文件与 source ledger digest；
- Grok config、project rules、system/generated prompt digest；
- provider、requested model、reasoning effort、context/token/turn/tool/time budgets；
- sandbox profile、writable roots、network policy、tool allow/deny、memory/subagent/web-search 状态；
- session UUID、output mode、debug/trace路径和 redaction policy digest；
- 环境变量**名称/是否存在/来源类别**，不保存 secret value。

任一字段在运行中改变，都生成新 run ID；不能修改原 manifest 来“解释”已经发生的行为。

## 3. 运行时记录层

### 必须无损

- Grok `streaming-json` stdout 原始字节流与逐事件解析结果；
- stderr、wrapper 自身决策与所有 terminal state；上游 debug-file 只有在证明不泄密或启动即进入 sealed
  private 层后才能启用；
- tool proposal、权限决定、参数 digest、开始/结束、exit code、stdout/stderr digest；
- workspace 中创建/修改/删除文件的 before/after digest 与 artifact registration；
- session ID、模型 requested/resolved、token usage、retry/compaction/timeout/cancel；
- manifest 与 append-only `run-event` hash chain。

每条桥接事件必须注明实际来源（Grok stdout/trace/export、hook、provider capture、workspace scan 或
supervisor）和 completeness。来源无法一一对应时保持 `unknown`/`ambiguous`，禁止按时间邻近补出 tool event。

### 可降采样但不得伪装无损

- Windows process tree、CPU、working set、private bytes、handle/thread count；
- I/O byte counters与磁盘/网络连接元数据；
- 长时间无语义事件期间的 heartbeat。

默认 1 秒采样；稳定空闲期可指数退避，tool/model 状态切换时恢复高频。采样间隔和丢样必须写入记录，
不能插值成“观测值”。

### 运行后封存

- `grok trace --local`、session `export`、最终 debug log；
- Git diff/status、workspace artifact 清单、每个文件 SHA-256；
- journal replay、session invariant、schema、leak/redaction 与 domain validator verdict；
- observed facts、direct comparisons、bridge hypotheses、missing checks 的分层摘要。

## 4. 三类存储

| 层级 | 内容 | 默认策略 |
|---|---|---|
| audit | digest、时间、状态、命令类别、计数、裁决、资源采样 | append-only，可长期保存 |
| sealed private | raw prompt/context、模型 raw output、tool stdout、provider reasoning | Windows DPAPI 加密，按 run 隔离，显式解封 |
| never record | API key、token、credential value、无关环境内容 | 只记录来源类别与 presence |

raw stream 采用 content-addressed blob + 压缩；journal 只引用 digest，避免多份复制增加磁盘与运存。写入使用
bounded queue 和 backpressure：允许降低 telemetry 频率，不允许静默丢弃语义事件或 terminal event。

## 5. 减少幻觉补全的机械约束

1. 外部事实必须来自已登记 source/tool result；模型记忆只能标为 unverified proposal。
2. 缺失字段使用 `unknown`/`not_observed`，禁止由邻近文本自动补全。
3. 每个 action 前记录输入与接受条件；每个 action 后记录实际输出和未满足项。
4. 工具成功只证明执行完成，不自动证明 artifact、evidence 或 claim 合格。
5. claim 输出必须区分 observed fact、direct comparison、bridge hypothesis 与 unsupported。
6. final 前运行 source-reference、artifact hash、journal chain、terminal-state 和 contradiction checks。
7. validator PASS 只表示机械检查未发现对应问题，不提升为科学证明。

## 6. Grok 0.2.106 映射

首个 observed fake run 使用 headless `-p`，并至少启用：固定 `--session-id`、
`--output-format streaming-json`、`--max-turns`、`--no-memory`、
`--no-subagents`、`--disable-web-search`、窄 `--tools`/`--deny` 和显式 `--sandbox`。

锁定的 Grok `0.2.106` 在一次被拒绝的 loopback probe 中把 Authorization value 写进了
`--debug-file`。因此当前 wrapper 禁用该参数；在上游修复或实现启动即加密/安全截获前，debug log 不能进入
普通 audit 层。该发现不影响 stdout streaming events、provider 侧脱敏 capture 或 protocol 语义。

禁止 `--always-approve` 与 `bypassPermissions`。全局参数必须位于子命令之前。run 后只执行本地
`trace --local` 与 `export`，不上传 trace。

## 7. 实现顺序

1. `new_grok_observed_dry_run.ps1`：只生成 binary/input/Git digest、命令、目录和安全声明；当前已实现为
   `dry-run-no-execution` 计划，明确不是正式 run manifest。
2. fake provider：捕获 Grok 最终 URL/headers/body 与 streaming events；loopback fixture、临时防火墙、
   clean child environment、no-overwrite 和 schema 已实现。一次 pre-fix run 发现 debug-file 泄漏假 credential
   并据此禁用；修复后的严格 smoke 已通过，详见
   [`GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`](../docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md)。
3. Windows Job Object supervisor：Grok 已接入 kill-on-close Job Object，并在成功与 timeout development run
中记录 create/assign/close；当前仍有 start-to-assignment race。显式 child-tree timeout/cancel/parent-exit
fixture 与管理员 observed 矩阵均已完成；candidate `0.2.111` 三场景通过，baseline `0.2.106` 的 tool-timeout
在输出终态后未退出并由外层超时收束。
4. workspace trust/restricted launch gate 已接入：双 receipt 早于任何 Grok 进程并在 agent process 前复查；
   fake-provider `0.2.0` 与 tool-continuity `0.3.0` 管理员实测通过。local trace/export + event completeness
   bridge 已完成首轮 `partial` 实测；纠正后的 v4 已成功取得 local trace/export。
5. fixture-only hash-only checkpoint + workspace delta + 独立 verifier 已完成；它是 audit receipt，不是恢复系统。
6. compaction provenance 的 manual `/compact` fake-only observed run 已完成：`PreCompact`/`PostCompact`、
   source span/digest、request、checkpoint 与压缩后 continuation 均经独立 verifier 通过；summary 标为
   `derived_unverified`，不能替代原始 observation。未直接记录的 retained/discarded 映射保持 `unknown`，
   automatic threshold 尚未观测。
7. DeepSeek conformance 通过后，才请求一次真实 development call 的单独授权。

local event-completeness bridge 现已完成第一轮实测：stdout、session events/updates、provider、supervisor 与
三张 receipt 被合并为 34 条 metadata-only hash-chain event；session-store 直接补足 tool lifecycle/ID。
纠正 Windows post-run 环境注入后，v4 成功取得 5702-byte local trace 与 297-byte export；整体仍正确标记为
`partial`，因为跨来源顺序、sealed encryption 与 stdout tool lifecycle 尚未观测。空 search index 只是派生全文
检索缓存状态，不表示 session 丢失。详见
[`GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md`](../docs/GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md)。hash-only
checkpoint/delta 结果见
[`GROK_FIXTURE_CHECKPOINT_DELTA_SPIKE_2026-07-21.md`](../docs/GROK_FIXTURE_CHECKPOINT_DELTA_SPIKE_2026-07-21.md)。

两轮 fake tool/reasoning continuity 与 Job Object 结果见
[`GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`](../docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md)。Grok
`streaming-json` 未输出显式 tool event；当前必须用 provider 第二轮 `role=tool` 消息与 artifact digest 补足
观测，不能声称 stdout 单独构成完整事件日志。

阶段 1–6 均不得要求 API key，也不得产生模型费用。

开源 Agent 对照和 adopt/adapt/defer/reject 裁决见
[`OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`](OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md)。Grok 已有 permission、
hook、sandbox、session 和 compaction lifecycle；本 envelope 只补它们无法提供的 LIF 可验证边界，
不建立平行 runtime。

进一步核对 Grok ACP、后台任务、shadow-Git recovery 与权限所有权后，生产路线改为 ACP-first；详见
[`MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md)。

云端 runtime 已由 [`ADR-0002`](../adr/ADR-0002-defer-cloud-runtime.md) 延期；本执行 envelope 当前只覆盖
Windows 本机，不把 relay、云端队列或 D cloud-brain 纳入 implementation spike。

## 8. Project D salvage 约束

[`D_SALVAGE_MATRIX_v0.1.md`](D_SALVAGE_MATRIX_v0.1.md) 只贡献三个方向：authoritative event log、
runtime/supervisor 分离、dangerous action confirmation。D 中的 prompt-only 反幻觉、读取失败默认值、
raw thought 日志、未鉴权 remote Python 和 queued-as-success 不进入 wrapper。dry-run 首先机械证明这些被拒绝
行为不存在，再进入 fake provider。
