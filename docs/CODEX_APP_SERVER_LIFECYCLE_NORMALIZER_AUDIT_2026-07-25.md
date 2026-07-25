# Codex app-server lifecycle normalizer 审计（2026-07-25）

状态：no-model、离线 ordered-capture 实现；定向和全量回归已通过，尚未进行 live app-server smoke。

## 来源与边界

实现以当前 OpenAI Codex manual 和官方
[`codex-rs/app-server/README.md`](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
为协议来源。当前官方边界包括：

- stdio 默认传输是一条 JSONL message stream；
- `thread/started` 建立 thread，`turn/started` 建立 active turn；
- `turn/completed.turn.status` 为 `completed`、`interrupted` 或 `failed`；
- `turn/interrupt` 请求/响应本身不是 terminal，必须等待后续 `turn/completed`；
- `thread/closed` 是显式 close notification；transport EOF、archive 和 delete 不等同于正常完成。

本轮没有启动 Codex、没有发送 prompt、没有使用 LIF 内部研究任务。输入是由可信 supervisor 按接收顺序包封的
离线 capture record；每条记录显式携带 `direction`、`source_stream_id`、连续 `source_record_sequence` 和
`received_at`。由于一个 app-server 连接可以承载多个 thread，而 `turn/started` notification 本身不携带
`threadId`，normalizer 必须先通过同一 capture 中的客户端 `turn/start(threadId)` 请求及其 response `turn.id`
建立 turn→thread 绑定；没有该绑定的 turn notification fail closed。

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

## 当前实测

- `python -m unittest runtime.tests.test_codex_app_server_lifecycle_normalizer -v`：8/8；
- `python scripts/check_repository.py`：114 schemas、7 个 Codex lifecycle fixtures、0 errors；
- `python -m compileall -q prototype/fep_agent_proto scripts runtime/tests/test_codex_app_server_lifecycle_normalizer.py`：通过。
- 当前源码全量回归：215/215（prototype 58、Grok integration 44、runtime 65、assurance 48）。
- `fep-script-validation` legacy-review task-board：两个 CLI 均为 PASS 11 / WARN 12 / FAIL 0，red-lines 为空。
  WARN 主要是实验脚本专用的 seed/backend/checkpoint/神经动力学字段不适用于本 deterministic normalizer；
  atomic write 由共用 `io_utils.atomic_write_*` 实现，未为了清空通用 board 添加无用参数。

## 第二轮自查

| 检查问题 | 状态 | 证据 | 未核风险 |
|---|---|---|---|
| 数据/计数是否可追溯 | 已核实 | unittest 四组输出；repository checker JSON | 测试计数不是 live 事件完整性 |
| 代码与计算链路是否回查 | 已核实 | normalizer module、两个 CLI、三份 schema、8 项测试 | 尚无真实 stdout capture |
| 参数性质是否透明 | 已核实 | CLI 显式要求 runtime version、run id、manifest digest 和三条路径 | 未做跨 Codex 版本兼容矩阵 |
| 新代码是否先定义反例 | 已核实 | sequence、cross-thread、unbound turn、truncated、tamper 负例 | 未来 live supervisor 仍需单独设计审查 |
| 环境是否一致 | 已核实 | `D:\CLI` 当前源码；Python compile/test；WindowsApps Codex 执行被拒记录 | 本机 Codex schema 未通过 `generate-json-schema` 直接导出 |
| 结论强度是否受限 | 已核实 | 文档只写 no-model/offline/mechanical PASS | 不声称 production/live 正确性 |
| source→observation 映射是否直接 | 已核实 | request/response turn binding、notification mapping、端到端 adapter test | app-server 未公开内部因果顺序 |
| 普遍性是否过度外推 | 已核实 | runtime version 必填；当前只覆盖 ordered stdio capture | WebSocket、多 connection、其他 CLI 未测 |
| 是否误写科学机制 | 已核实 | 无 LIF 数据、训练、机制或结果 claim | 无 |
| 来源与状态是否真实 | 已核实 | 当前 Codex manual、官方 OpenAI Codex app-server README、实际命令输出 | 本机 executable help 无法运行 |
| 是否独立核查用户边界 | 已核实 | 实际读取 OneDrive 上游三文件；`D:\CLI` 未新增 LIF 路由副本 | 上游后续变化不由本仓库自动同步 |
| 是否继承外部模型置信度 | 已核实 | 协议字段由官方来源并由正反 fixture 验证 | manual/main branch 未来可能变化 |
| prior-existence/MAP 是否冲突 | 已核实 | 实际 `LIF_CURRENT_INDEX.md` 对 CLI/Codex/normalizer/撤回/降级词扫描均为 0 | 本任务不登记 LIF scientific claim，未进入 MAP6 |
| 表达与边界是否一致 | 已修正 | EOF 保持 partial；turn failure 不终止 thread；thread close 单独映射 | live 后需再次复核文档 |

## 保留边界

- 当前 capture wrapper 证明的是 supervisor 声明的单连接接收顺序；尚未实测真实 app-server stdout 捕获；
- 本机 WindowsApps 内 Codex executable 可发现，但当前执行环境拒绝直接运行其 `--version`/`app-server --help`；
- 未实现 live capture supervisor、多 connection 排序、WebSocket、Gemini/Qwen hooks；
- receipt 和 observations 各自原子写入，但不构成跨文件原子事务；
- mechanical PASS 不证明模型质量、任务完成质量或科学结论。
