# Codex app-server lifecycle normalizer 审计（2026-07-25）

状态：ordered-capture、离线 normalizer 与独立 verifier 已实现；Codex CLI 0.145.0 隔离 live
app-server 的 no-model 握手、loopback completed/failed/interrupt synthetic-turn smoke 均已通过；
`thread/closed` 因官方 idle unload 需要 30 分钟，当前通过 fake/fixture 验证映射。

## 来源与边界

实现以当前 OpenAI Codex manual 和官方
[`codex-rs/app-server/README.md`](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
为协议来源。当前官方边界包括：

- stdio 默认传输是一条 JSONL message stream；
- `thread/started` 建立 thread，`turn/started` 建立 active turn；
- `turn/completed.turn.status` 为 `completed`、`interrupted` 或 `failed`；
- `turn/interrupt` 请求/响应本身不是 terminal，必须等待后续 `turn/completed`；
- `thread/closed` 是显式 close notification；transport EOF、archive 和 delete 不等同于正常完成。

离线阶段没有启动 Codex、没有发送 prompt、没有使用 LIF 内部研究任务。输入是由可信 supervisor 按接收顺序
包封的 fixture capture record；每条记录显式携带 `direction`、`source_stream_id`、连续 `source_record_sequence` 和
`received_at`。由于一个 app-server 连接可以承载多个 thread，而 `turn/started` notification 本身不携带
`threadId`，normalizer 必须先通过同一 capture 中的客户端 `turn/start(threadId)` 请求及其 response `turn.id`
建立 turn→thread 绑定；没有该绑定的 turn notification fail closed。

live 阶段使用临时安装于仓库忽略目录的官方 `@openai/codex` 0.145.0 binary，并把 `CODEX_HOME` 和
`CODEX_SQLITE_HOME` 隔离到 `.observed-runs/`。supervisor 在 Windows Job Object 中只发送
`initialize → initialized → thread/start(ephemeral=true, sandbox=read-only)`；没有 `turn/start`、prompt、
模型请求、工具调用或 canonical journal append。首轮用 `readOnly` 被该版本以明确 schema error 拒绝；未覆盖失败
产物，改为版本实际接受的 `read-only` 后用新目录重试成功。

第二个 live 阶段由专用 turn probe 启动同一真实 binary，但把 custom Responses provider 固定到临时
`127.0.0.1` listener。隔离配置关闭 apps、plugins 和 web search，provider 不配置 credential，
请求必须是唯一的 `POST /v1/responses`、`stream=true`、固定 synthetic marker 且无 Authorization。
loopback SSE 形状直接采用 Codex 官方测试 helper 的最小序列
[`response.created → response.output_item.done → response.completed`](https://github.com/openai/codex/blob/main/codex-rs/core/tests/common/responses.rs)，
而 Codex 官方 Responses endpoint 实现确认请求使用 `POST responses` 和 `Accept: text/event-stream`
（[`codex-api/src/endpoint/responses.rs`](https://github.com/openai/codex/blob/main/codex-rs/codex-api/src/endpoint/responses.rs)）。
该阶段只使用固定 synthetic input/output，不读取或迁移 LIF 上游文件，也不调用付费模型。

## 实现

`normalize_codex_app_server_lifecycle.py` 只读 capture，并输出 lifecycle observation v0.2 与 normalization receipt：

- `thread/started → session_started`；
- `turn/started → turn_started`；
- 三种 `turn/completed` 分别保留 `turn_status`，均只执行 `in_turn → active`；
- failed turn 只保存 canonical error object 的 SHA-256，不复制错误正文；
- `thread/closed → session_completed(outcome=thread_closed)`，只允许从 idle active thread 进入 terminal；
- item/delta、请求和响应可占据 source record sequence，但不会获得 lifecycle `source_sequence`；
- capture 结束在 active/in-turn 时保持 `partial`；EOF 不补写完成或取消。

`verify_codex_app_server_lifecycle.py` 独立重放 capture，并核对 source artifact、receipt 投影、observation bytes/digest
和正文省略。标准 observation 随后仍由既有
`append_cli_session_lifecycle_event.py` 与 `verify_cli_session_lifecycle_receipt.py` 进入 canonical journal，不新增
第二套 journal writer。

`capture_codex_app_server_lifecycle.py` 与 `verify_codex_app_server_lifecycle_capture.py` 补齐 live source：

- 显式 absolute executable、无 shell、固定 timeout/record/line/stderr 上限；
- Windows Job Object 或 POSIX process group 只清理本次 spawn 的进程树；
- capture、stderr、receipt 分离且拒绝覆盖，输出完成后原子落盘；
- receipt 不复制 command argv 和 stderr 正文，只保存 digest 与长度；capture verifier 独立检查 retained stderr；
- verifier 重建三条 client message、initialize/thread response、thread notification 和 thread id，并确认 no turn/no input。

`probe_codex_app_server_turn_lifecycle.py` 与 `verify_codex_app_server_turn_probe.py` 补齐受控真实 turn
及终端态矩阵：

- no-model capture 保持原安全默认；只有专用 probe 才发送固定 synthetic `turn/start`；
- 自动生成隔离 user-level provider config，模型 endpoint 只能是 literal `127.0.0.1` ephemeral port；
- provider 仅保留 request body digest、长度、model/stream/marker/header 布尔投影，不复制完整模型请求；
- probe receipt 同时绑定 config、provider exchange、stdio capture、stderr 与 thread/turn terminal identity；
- `--terminal-scenario` 覆盖 `completed`、`failed`、`interrupted` 与 `closed`；
- `failed` 使用有效 Responses request 后的 loopback HTTP 500，要求 app-server 返回
  `turn/completed(status=failed)` 且 error 只经 normalizer digest 化；
- `interrupted` 在 `turn/started` 后发送 `turn/interrupt`，要求先收到空成功响应，再等待
  `turn/completed(status=interrupted)`；实际 0.145.0 smoke 中取消发生在 provider request 之前，
  因此 provider request count 可为 0；
- `closed` 发送 `thread/unsubscribe` 并等待 `thread/closed`；真实 Codex 按官方 README 会在最后订阅者移除后
  等待 30 分钟 idle unload，因此当前只由 fake/fixture 覆盖，不作为短时 live smoke；
- verifier 独立重放各场景的实际 record order，并检查 synthetic input/output/SSE digest 或无 provider request
  的 interrupt 边界。

## Fixture 与反例

`runtime/fixtures/codex-app-server-lifecycle-v0.1/` 固定七类输入：

1. happy：response → thread start → turn start → item → completed → thread close；
2. interrupted 后同一 thread 开始第二个 turn；
3. failed turn 只保留 error digest；
4. interrupt request/response 后缺 terminal，保持 `in_turn/partial`；
5. duplicate source sequence；
6. cross-thread close；
7. truncated JSON。

定向测试还覆盖 observation/receipt verifier、observation 篡改，以及
capture → observation → canonical adapter → adapter verifier 的端到端链。

fake app-server 的 no-model 测试覆盖八项 supervisor 路径：正常握手、notification-before-response、timeout、malformed stdout、
response/notification thread mismatch、capture tamper、stderr tamper 和拒绝覆盖。正常路径继续送入 normalizer，结果必须是
`session_started` 一条、`active/partial`，不能因 EOF 变成 terminal。

synthetic-turn probe 另有七项测试：成功 provider exchange + probe verifier + normalizer 双重回放、failed /
interrupted / closed 终端态矩阵、invalid provider exchange、capture tamper、provider projection tamper、config tamper
和拒绝覆盖。

## 当前实测

- Codex capture + normalizer 既有定向测试：16/16（capture 8、normalizer 8）；
- synthetic-turn probe 定向测试：7/7；
- `python scripts/check_repository.py`：118 schemas、7 个 Codex lifecycle fixtures、0 errors；
- `python -m compileall -q prototype/fep_agent_proto scripts runtime/tests/...`：通过；
- Codex CLI 0.145.0 live capture：6 records，capture verifier 7/7；
- live normalization：1 个 `session_started`，`active/partial`，normalization verifier 7/7；
- Codex CLI 0.145.0 completed synthetic-turn live probe：18 records、唯一 loopback provider request、terminal
  `completed`，probe verifier 10/10；
- completed synthetic-turn live normalization：3 个 observations（session start、turn start、turn completed），
  `active/partial`，normalization verifier 7/7，raw content omission 通过；
- Codex CLI 0.145.0 failed synthetic-turn live probe：15 records、唯一有效 loopback provider request、HTTP 500 后
  terminal `failed`，probe verifier 10/10；normalization 为 3 个 observations，`active/partial`，
  normalizer verifier 7/7；
- Codex CLI 0.145.0 interrupted synthetic-turn live probe：14 records、`turn/interrupt` 空成功响应后 terminal
  `interrupted`，probe verifier 10/10；本次 request_count=0，说明取消发生在 provider request 之前；
  normalization 为 3 个 observations，`active/partial`，normalizer verifier 7/7；
- fake/fixture closed path：`thread/unsubscribe → thread/closed` 归一化为第 4 条
  `session_completed(outcome=thread_closed)`，normalization `complete/terminal`；
- 当前源码全量回归：230/230（prototype 58、Grok integration 44、runtime 80、assurance 48）。
- `fep-script-validation` legacy-review task-board：capture CLI 为 PASS 9 / WARN 14 / FAIL 0，
  capture verifier 为 PASS 12 / WARN 11 / FAIL 0，red-lines 均为空。WARN 主要是实验脚本专用字段不适用，
  以及静态扫描未跨模块识别共用 atomic/no-overwrite 实现；未为清空 board 添加无用参数。
- `fep-script-validation` legacy-review task-board：两个 CLI 均为 PASS 11 / WARN 12 / FAIL 0，red-lines 为空。
  WARN 主要是实验脚本专用的 seed/backend/checkpoint/神经动力学字段不适用于本 deterministic normalizer；
  atomic write 由共用 `io_utils.atomic_write_*` 实现，未为了清空通用 board 添加无用参数。
- `fep-script-validation` legacy-review task-board：turn probe CLI 为 PASS 9 / WARN 14 / FAIL 0，
  probe module 为 PASS 11 / WARN 12 / FAIL 0，probe verifier 为 PASS 12 / WARN 11 / FAIL 0；
  三者 catastrophic red-lines 均为空。WARN 同样主要来自科研实验模板字段不适用，未做迎合式代码修改。
- 本次 terminal matrix 扩展后的 `fep-script-validation` legacy-review task-board：probe module 为
  PASS 11 / WARN 12 / FAIL 0，probe CLI 为 PASS 9 / WARN 14 / FAIL 0，probe verifier 为
  PASS 12 / WARN 11 / FAIL 0，fake app-server fixture 为 PASS 7 / WARN 16 / FAIL 0；四者
  catastrophic red-lines 均为空。fake fixture 的 strict-json/finite WARN 来自测试夹具的简化 JSON 写法，
  不进入正式 observation/receipt artifact writer。

## 第二轮自查

| 检查问题 | 状态 | 证据 | 未核风险 |
|---|---|---|---|
| 数据/计数是否可追溯 | 已核实 | unittest 四组输出；repository checker JSON | 测试计数不是 live 事件完整性 |
| 代码与计算链路是否回查 | 已核实 | capture/normalizer/turn-probe modules、六个 CLI、七份 schema、20 项定向测试、真实 stdout capture | 尚无 live interrupt/failed turn |
| 参数性质是否透明 | 已核实 | CLI 显式要求 runtime version、run id、manifest digest 和三条路径 | 未做跨 Codex 版本兼容矩阵 |
| 新代码是否先定义反例 | 已核实 | sequence、cross-thread、unbound turn、truncated、timeout、malformed、identity mismatch、tamper | 多 connection 尚未设计 |
| 环境是否一致 | 已核实 | `D:\CLI` 当前源码；Python compile/test；官方 npm Codex 0.145.0；隔离状态目录 | Store-app binary ACL 仍不可直接执行 |
| 结论强度是否受限 | 已核实 | 文档只写受控 synthetic turn 的 mechanical PASS | 不声称 production 或真实任务正确性 |
| source→observation 映射是否直接 | 已核实 | request/response turn binding、notification mapping、端到端 adapter test | app-server 未公开内部因果顺序 |
| 普遍性是否过度外推 | 已核实 | runtime version 必填；当前只覆盖 ordered stdio 与一个 completed turn | WebSocket、多 connection、其他 CLI 未测 |
| 是否误写科学机制 | 已核实 | 无 LIF 数据、训练、机制或结果 claim | 无 |
| 来源与状态是否真实 | 已核实 | 当前 Codex manual、官方 OpenAI Codex app-server README、0.145.0 实际 capture/receipt/verifier | 未做跨版本矩阵 |
| 是否独立核查用户边界 | 已核实 | 实际读取 OneDrive 上游三文件；`D:\CLI` 未新增 LIF 路由副本 | 上游后续变化不由本仓库自动同步 |
| 是否继承外部模型置信度 | 已核实 | 协议字段由官方来源并由正反 fixture 验证 | manual/main branch 未来可能变化 |
| prior-existence/MAP 是否冲突 | 已核实 | 实际 `LIF_CURRENT_INDEX.md` 对 CLI/Codex/normalizer/撤回/降级词扫描均为 0 | 本任务不登记 LIF scientific claim，未进入 MAP6 |
| 表达与边界是否一致 | 已修正 | EOF 保持 partial；turn failure 不终止 thread；thread close 单独映射 | live 后需再次复核文档 |

## 保留边界

- 当前 capture wrapper 只证明 supervisor 在单一 stdio 连接上的接收顺序；
- 本机 WindowsApps 内 Codex executable 仍因 ACL 不可直接运行；live smoke 使用官方 npm 0.145.0 临时 binary；
- 早期 no-model live startup 的 stderr 显示一次 featured-plugin 远程预热尝试失败；新的 synthetic-turn
  隔离配置关闭 apps/plugins 后未再出现该请求，但 `read-only` 仍不等同于 app-server 宿主进程绝对零网络；
- 已测试真实 app-server + fake provider 的 completed、failed 与 interrupted turn；`thread/closed` 的短时 live
  路径未测，因为官方 idle unload 在最后订阅者移除后等待 30 分钟；仍未实现多 connection 排序、WebSocket、
  Gemini/Qwen hooks；
- receipt 和 observations 各自原子写入，但不构成跨文件原子事务；
- mechanical PASS 不证明模型质量、任务完成质量或科学结论。
