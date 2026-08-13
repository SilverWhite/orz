# Classical Console POC — 第一条通路

> 目的：验证“操作台”最小通路：主模型输出（结构化服务调用或自然语言）→
> 服务注册表 → 契约校验 → 确定性执行 → 结构化响应 → verifier。
> 位置：CLASSICAL-EXEC-ASSISTANT 小样 1（控制台路由小样）。
> 来源：借用 Home Assistant 服务模型（domain.service + schema + 执行 + 响应）
> 与 hassil 文法意图（均为 Apache-2.0）；本文件 JSON 协议为原型隔离层
> （v0.3 起不作为生产接缝）。

## 原型接缝（stdin/stdout JSON-lines）

stdin/stdout JSON-lines 协议，两个入口：

- service 模式：`{"proto":1,"id":"...","service":"workspace.read_file","data":{...}}`
- intent 模式：`{"proto":1,"id":"...","text":"read the file README.md"}`

响应：`{"proto":1,"id":"...","ok":true,"response":{...},"trace_id":"t000001"}`

错误：`{"proto":1,"id":"...","ok":false,"error":{"step":"contract","code":"invalid_arguments","message":"...","upstream":null,"trace_id":"t000001"}}`

执行失败（`step=execute`）额外附有界日志尾部：
`"trace":[{"seq":1,"step":"registry","action":"...","ok":true}, ...]`

> v0.3 定位修正：本协议只是原型隔离形态，不作为生产接缝。生产形态是 HA
> 助理层与 orz 深度融合（HA 为 orz 的一部分），薄接缝改置 orz 本体 ↔ 底座
> （模型后端，Grok/Codex 等）——底座只输出注册名称 + 参数，操作台在 orz
> 内部逐层定位执行；后端替换点 = 底座适配面，不是 console。
> v0.5 定位修正：生产协作形态 = 黑板动作栏（注册板块/动作栏/结果栏），模型面
> 只“读板块 + 写订单”，发放为机械单一出口；本协议仍只是原型隔离。

### fail-closed 返回契约（2026-08-13 落地）

错误信封固定含 step/code/message/upstream/trace_id；`step=execute` 失败时
另附有界 trace 尾部：

- `step`：具体失败点，枚举 `protocol` / `intent` / `registry` / `contract` /
  `target` / `execute` / `verify` / `policy`（预留）。
- `code`：机器可读错误码（HA 同构：`invalid_arguments`、`unknown_service`、
  `not_found`、`out_of_scope` 等）。
- `message`：失败反馈，中性且含具体细节（沿用 HA 带 `@ data['...']` 校验路径）。
- `upstream`：上游执行结果——解析后真实目标、票据/context id、部分输出、
  verifier 摘要；无则 `null`。
- `trace_id`：本次请求执行日志 id；主模型可经 `assistant.trace` 取回全文。
- `trace`：仅 `step=execute` 失败存在——执行失败特殊反馈（最近 N 条事件）。

HA 原项目只回 `error:{code,message}`（成功回 `result:{context,response}`），
无失败点与上游结果；本扩展是模型排障的必要内容。

## 执行日志（v0.4）

每次请求生成有界 trace（单 trace 200 条事件、保留最近 50 个请求）；成功/失败
信封恒带 `trace_id`；`assistant.trace` 按 id 取回（只读、可选 tail）。
执行失败（`step=execute`）时错误信封直接附最近事件尾部，模型无需先查日志
即可定位。

## 服务（v0.1，细粒度起点）

| 服务 | 输入 | 输出 | 机械约束 |
|---|---|---|---|
| `assistant.trace` | `trace_id`（可选 `tail` 1–200） | `trace_id/request_id/events[]/truncated` | 只读；有界（50 个请求 × 200 事件）；未知 trace_id 拒绝；读操作自身入 trace |
| `workspace.read_file` | `path` | `path/content/size/encoding` | realpath 在 allow root 内；1 MiB 截断 + 页脚；固定解码链（BOM→UTF-8→GB18030→lossy） |
| `workspace.list_dir` | `path` | `path/entries[]` | realpath 在 allow root 内；1000 条上限；稳定排序 |
| `workspace.index` | — | `root/files[]/total/truncated` | 有界扫描（深度 4、2000 条）；跳过隐藏/构建/依赖目录；相对路径稳定排序 |

所有服务：输入/输出均过 schema 校验（jsonschema），未知字段拒绝，作用域 fail-closed。

## 意图（v0.1）

`intents.en.yaml` / `intents.zh.yaml`：hassil 文法。
“read the file X” / “读取文件 X” → `workspace.read_file`。
**槽位表由工作区索引动态生成**：`workspace.index` 服务有界扫描 allow root
（跳过隐藏/构建/依赖目录，相对路径稳定排序），hassil 槽位取值来自索引——
自然语言只能引用索引内文件；主模型结构化调用走 service 模式不受此限
（契约 + 作用域兜底）。生产实现复用 orz `project_doc_index` 缓存。

## 运行

```powershell
python console.py --allow-root D:\CLI
python smoke_test.py
```
