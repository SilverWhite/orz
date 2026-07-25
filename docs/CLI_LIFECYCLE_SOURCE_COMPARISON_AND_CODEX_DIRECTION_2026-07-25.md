# CLI lifecycle source 比较与 Codex normalizer 方向（2026-07-25）

状态：设计与实现记录；通用 lifecycle v0.2 已冻结，首个 Codex app-server 离线只读 normalizer、独立 verifier、
live capture supervisor 和正反 fixture 已实现；Codex CLI 0.145.0 的隔离 no-model live smoke 已通过。

## 1. 术语边界

“单一、有序 lifecycle source”是本项目对控制器输入证据的工程约束，不是某家 CLI 的官方产品术语：

- **单一**：在 adapter 边界上只有一个权威入口；CLI 仍可产生其他日志，但它们不能与权威入口共同竞争状态解释权。
- **有序**：事件位置由同一传输的接收顺序或单调 source sequence 决定，不能靠多个文件的 wall-clock timestamp
  事后猜测。
- **lifecycle**：入口至少覆盖 session/thread start、turn start、turn terminal status，以及 session/thread terminal。
- **source**：事件来自运行时协议本身，或由可信 supervisor 在运行时串行化；离线拼接多个独立来源不能自动升级为
  真实运行顺序。

该要求不等于“只能有一个物理文件”。一条有序 RPC/JSONL 连接、一个带单调序号的 event bus，或由 supervisor
原子追加的单一 journal 都可以满足。多个来源可以作为补充证据，但必须保留各自 provenance 和顺序强度。

## 2. 当前 Grok 边界

当前 Grok `events.jsonl` fixture 已出现 `turn_started`、tool lifecycle 和 `turn_ended`，但 session start/end、ACP
响应、stdout、updates 与 supervisor terminal 分散在不同来源。post-run bridge 明确记录
`cross_source_runtime_order=not_established`；其 bridge sequence 是确定性追加顺序，不是被证明的运行时因果顺序。

因此 Grok 不被弃用，而被重新定位为：

1. 多来源、弱顺序证据的 compatibility/adversarial target；
2. 检验系统是否会把 deterministic merge 错认成 runtime order 的反例；
3. 若未来 Grok 暴露完整 ACP/session ordered stream，再实现对应 normalizer。

## 3. 成熟 CLI 比较

本轮只把官方文档或官方开源仓库作为设计依据。

| CLI | 官方 lifecycle surface | 可复用设计 | 尚不能直接假定的部分 | 本项目定位 |
|---|---|---|---|---|
| OpenAI Codex app-server | stdio 上的双向 JSON-RPC 2.0；JSONL notification stream | `thread/started`、`turn/started`、`item/*`、`turn/completed`、`thread/closed`；turn terminal status 明确为 `completed`、`interrupted` 或 `failed`；取消后必须等 terminal notification | transport EOF 不等于 thread 正常关闭；一个 app-server 进程也可能承载多个 thread | 第一 concrete normalizer |
| Google Gemini CLI | Session/Agent/Model/Tool hooks | `SessionStart/End`、`BeforeAgent/AfterAgent` 提供较完整 callback 边界，适合无 core patch 接入 | 未承诺跨 hook 的全局单调序号；`SessionEnd` 为 best-effort；部分 hook 异步 | 第二 hook-based compatibility target |
| Alibaba Qwen Code | Session、prompt、stop/failure、tool、subagent hooks | `Stop`/`StopFailure` 区分正常停止和失败，terminal taxonomy 可参考 | 部分 hook fire-and-forget；未见统一全局 source sequence 契约 | 补充 hook 兼容参考 |
| GitHub Copilot CLI | 官方 lifecycle hooks | session end reason 包含 complete/error/abort/timeout/user_exit，可参考 run terminal reason | hooks 本身不证明完整、单调的单源事件流 | 协议语义参考，不作为首个开源实现样本 |

官方来源：

- OpenAI Codex：
  [app-server protocol README](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)；
  [protocol v1](https://github.com/openai/codex/blob/main/codex-rs/docs/protocol_v1.md)。
- Google Gemini CLI：
  [Hooks reference](https://geminicli.com/docs/hooks/reference/)；
  [Writing hooks](https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/writing-hooks.md)。
- Alibaba Qwen Code：
  [Hooks](https://github.com/QwenLM/qwen-code/blob/main/docs/users/features/hooks.md)。
- GitHub Copilot CLI：
  [Hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference)。

## 4. 比较暴露的本项目 schema 缺口

冻结的 observation v0.1 只有 `turn_completed`，却没有 turn terminal status。若把 Codex
`turn/completed(status=interrupted)` 映射为 `session_cancelled`，会错误终止仍可继续的 thread；若一律映射为无状态
`model_output`，又会把 interrupted/failed 的语义丢失。

v0.2 保留 `turn_completed` 作为“turn terminal envelope”，并新增：

```text
turn_status = completed | interrupted | failed
error_sha256 = nullable
source_stream_id
source_sequence
source_record_sequence
```

约束如下：

- `turn_started`：必须有 `turn_id`，`turn_status/outcome/error_sha256` 必须为空；
- `turn_completed`：必须有同一 active `turn_id` 和非空 `turn_status`，`outcome` 必须为空；
- `error_sha256` 只允许出现在 failed turn 或 `session_failed`，且只保存 digest，不复制错误正文；
- completed、interrupted、failed 都只执行 `in_turn → active`；
- 只有 `session_completed/session_failed/session_cancelled` 执行 run terminal；
- canonical `model_output` 在该接入面表示“turn terminal record”，不单独构成成功声明，消费者必须读取
  `turn_status`。

其中 `source_sequence` 是连续的 lifecycle observation 序号；`source_record_sequence` 是同一
`source_stream_id` 内原始 transport message 的位置，只要求严格递增、允许因忽略 `item/*` 等非 lifecycle 消息而跳号。
二者不能混用，否则 normalizer 会把合法的投影稀疏性误判成 source gap。

v0.1 schema 文件继续保留，以维持提交 `6f28192` 的可复现基线；当前 adapter 和 verifier 使用 v0.2。

## 5. 第一 concrete normalizer：Codex app-server

已实现 read-only `Codex app-server ordered capture → cli-session-lifecycle-observation v0.2` normalizer。离线阶段不接
TUI 文本、不解析 `codex exec` 的人类可读 stdout，也不把 post-run 文件合并顺序当作 source order；随后新增的
live supervisor 只启动显式 app-server executable，发送 initialize、initialized 和
ephemeral/read-only `thread/start`，不发送 `turn/start` 或模型输入。

### 5.1 输入与排序

1. 可信 supervisor 在读取/写入 app-server stdio JSONL 时，把每条完整 message 包封为带
   `direction/source_stream_id/source_record_sequence/received_at` 的双向 capture record；
2. 离线 normalizer 只读该单一 capture，要求 `source_record_sequence` 从 0 连续递增；
3. 只对产生 lifecycle observation 的 message 分配连续 `source_sequence`；
4. observation 保存 capture record 的 SHA-256；原始 message 仍只留在 capture，prompt/reasoning/tool/output 正文不复制；
5. 用固定 `adapter_id + runtime_version + thread_id + source_stream_id` 绑定 source identity；`turn/started`
   必须先由同一 capture 中的 `turn/start(threadId)` 请求及其 response `turn.id` 建立 thread 绑定；
6. malformed JSON、倒退/重复 source record sequence、thread/stream identity 漂移和连接异常 fail closed，不伪造缺失
   lifecycle event。

`source_record_sequence` 证明的是“supervisor 在一条连接上的接收顺序”，不是模型内部并行工作的隐藏因果顺序。
capture wrapper 是 normalizer 的 source record。Codex CLI 0.145.0 live smoke 已实际捕获六条 stdio JSONL message；
`thread/started` 归一化为唯一 `session_started`，连接关闭后保持 `active/partial`。

### 5.2 首批映射

| Codex notification | observation v0.2 | 状态效果 |
|---|---|---|
| `thread/started` | `session_started` | `new → active` |
| `turn/started` | `turn_started` | `active → in_turn` |
| `turn/completed`, status=`completed` | `turn_completed`, turn_status=`completed` | `in_turn → active` |
| `turn/completed`, status=`interrupted` | `turn_completed`, turn_status=`interrupted` | `in_turn → active` |
| `turn/completed`, status=`failed` | `turn_completed`, turn_status=`failed` | `in_turn → active` |
| `thread/closed` | `session_completed`, outcome=`thread_closed` | `active → terminal` |

`turn/interrupt` 请求本身不生成 terminal observation；只有后续 `turn/completed(status=interrupted)` 才结束 turn。
同理，turn failed 不自动等于 session failed。

### 5.3 fixture 与验收

首个 implementation spike 使用 no-model、离线 ordered-capture JSONL fixture：

1. happy path：thread start → turn start → item events → completed turn → session terminal；
2. interrupted turn 后同一 thread 能开始第二个 turn；
3. failed turn 保留 error digest，session 仍 active；
4. interrupt request 到达但 terminal notification 缺失时保持 in-turn/partial，不自行补写成功或取消；
5. 重复、乱序、跨 thread identity 和 truncated JSON 全部 fail closed；
6. raw source digest、observation digest、canonical journal event 和 verifier receipt 可独立重放；
7. fixture 通过后才进行受限 live smoke；live smoke 仍不使用 LIF 内部研究任务。

实现入口：

- `scripts/capture_codex_app_server_lifecycle.py`
- `scripts/verify_codex_app_server_lifecycle_capture.py`
- `scripts/normalize_codex_app_server_lifecycle.py`
- `scripts/verify_codex_app_server_lifecycle.py`
- `prototype/fep_agent_proto/codex_app_server_capture.py`
- `prototype/fep_agent_proto/codex_app_server_lifecycle.py`
- `runtime/fixtures/codex-app-server-lifecycle-v0.1/`
- `runtime/fixtures/fake_codex_app_server.py`
- `runtime/tests/test_codex_app_server_lifecycle_capture.py`
- `runtime/tests/test_codex_app_server_lifecycle_normalizer.py`

### 5.4 暂不纳入第一阶段

- item/tool 内容标准化；
- prompt、reasoning、tool output 正文复制；
- 多 app-server connection 的全局排序；
- 从 wall-clock timestamp 合成缺失 sequence；
- 把 process exit、transport EOF 或 turn failure未经证明地升级成某种 session terminal；
- Gemini/Qwen hook writer。

这些边界用于保持第一 normalizer 足够小，使它首先证明 lifecycle identity、顺序和 terminal 语义，而不是扩张成新的
通用 CLI runtime。
