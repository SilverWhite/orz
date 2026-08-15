# Classical Console POC — 第一条通路

> 目的：验证“操作台”最小通路：主模型输出（结构化服务调用或自然语言）→
> 服务注册表 → 契约校验 → 确定性执行 → 结构化响应 → verifier。
> 位置：CLASSICAL-EXEC-ASSISTANT POC（小样 1 控制台路由已跑通、小样 2
> 编辑执行器已闭合、小样 3 机械组合脚本已闭合）。
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
| `workspace.search_replace` | `path` / `old_string` / `new_string` / `replace_all?` | `path/old_string/new_string/applied/replaced_all/size/encoding/crlf_normalized` | 唯一精确匹配（默认）；`replace_all` 全量替换；CRLF 感知匹配并保留行尾；固定解码链；原子写盘；空 `old_string` 仅允许新建或填空，禁止静默覆盖 |
| `workspace.replace_by_intent` | `path` / `intent` | 同 search_replace + `intent` | 确定性规则层把引号化意图解析为 hunk（en/zh 文法 + 全量标记），再走同一执行器；无模型参与 |
| `workspace.run_script` | `script`（步骤数组：`do` / `with` / `as`） | `steps[]` / `result` | 线性脚本逐行执行：`$ref` 引用先前步骤输出字段；执行前静态校验（引用存在/作用域/类型、名称唯一、服务已知、禁嵌套脚本）；上限=20 步 / 30s 墙钟 / 4 MiB 最终响应累计（含全部步骤响应与 result）；每步独立 schema 校验 + trace；任一步失败 fail-closed |

所有服务：输入/输出均过 schema 校验（jsonschema），未知字段拒绝，作用域 fail-closed。

## 小样 2 — 编辑执行器对照实验（2026-08-15 实施）

对照两路编辑通路：

- baseline：主模型直接生成 hunk（`workspace.search_replace` 依序尝试，每次尝试 = 1 工具轮）；
- candidate：主模型表达意图 → 确定性规则层生成 hunk（`workspace.replace_by_intent`，1 工具轮）。

指标 = 编辑应用成功率、主模型工具轮数（总计/均值）；通过标准 = 编辑失败率显著下降或
持平且轮次不增（BACKLOG P0-C 小样 2，见
[`docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md`](../../docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)）。

边界：baseline 的 hunk 尝试序列为人工编写模拟（非真实模型输出），语料固定，证据为
POC 级，不构成生产端到端结论（正式结论待正式组件决策门）。

语料（固定，10 场景）：[`sample2_corpus.json`](sample2_corpus.json)——覆盖精确单行、
过期上下文/缩进错误、多匹配歧义、replace_all、CRLF 文件、GB18030 文件、幻觉缺失文本、
old==new 错误、弯引号意图、多行 hunk。

运行：

```powershell
python sample2_editor_benchmark.py --out sample2_result.json
python sample2_editor_benchmark.py --smoke --out sample2_smoke_result.json
python smoke_test.py
```

结果 JSON 含 run 清单（script/corpus sha256、python、argv、时间）、两臂指标、失败模式
分布与通过标准布尔值；原子写入，默认不覆盖已有结果（`--overwrite` 显式放行）。

> 状态：已闭合（2026-08-15 用户裁决通过，独立判定一致；2026-08-15 全面审查后
> 完成 P1 修复——基线人工模拟边界显式声明）。结果工件：
> [`sample2_result.json`](sample2_result.json)。

## 小样 3 — 机械组合脚本模式（2026-08-15 实施，已闭合）

线性脚本（PTC 程序化调用）把多轮工具往返压缩为一个可校验执行单元：模型输出
确定性脚本，操作台逐行执行，任一步失败 fail-closed。

```json
{"script": [
  {"do": "workspace.read_file", "with": {"path": "a.txt"}, "as": "a"},
  {"do": "workspace.read_file", "with": {"path": {"$ref": "a.path"}}, "as": "b"}
]}
```

机械约束（全部确定性、无模型参与）：

- 步骤 = 注册动作实例；`as` 命名输出（可省略），`$ref` 引用先前步骤输出字段
  （点路径，数字下标可进数组元素）；无任意代码、无隐式控制流。
- 执行前静态校验：引用存在性、作用域（只能引用更早的命名步骤）、类型匹配
  （按注册表输入/响应 schema 推导）、步骤名唯一、服务已知、禁止嵌套脚本。
- 每步独立 schema 校验 + trace 事件（复用 `ServiceRegistry.call`，不建第二套审计链）；
  失败信封保留内层 `step`/`code`，`upstream` 携带 `script_step`/`action`/内层
  code/message/upstream，后续步骤不再执行。
- 上限：20 步（schema `maxItems`）、30s 墙钟、4 MiB 最终响应累计（含全部步骤
  响应与 `result`，未命名步骤同样计入）、既有单 trace 200 条事件上限；静态错误
  `step=contract`，运行期超限 `step=execute`。

对照实验（baseline=逐轮串行调用，每次调用计 1 工具轮；candidate=单脚本，
计 1 工具轮）：固定语料 [`sample3_corpus.json`](sample3_corpus.json)（8 场景，
覆盖 `$ref` 路径/数组/对象、读后建文件、内容注入编辑、顺序编辑、GB18030、
重复引用、无名步骤）。通过标准 = 候选成功率 ≥ 基线且候选平均轮数 ≤ 基线
（目标为严格减少轮数）。

边界：baseline 的串行调用序列为人工编写模拟（非真实模型输出），语料固定，证据为
POC 级，不构成生产端到端结论（正式结论待正式组件决策门）。

运行：

```powershell
python sample3_script_benchmark.py --out sample3_result.json
python sample3_script_benchmark.py --smoke --out sample3_smoke_result.json
python smoke_test.py
```

结果工件：[`sample3_result.json`](sample3_result.json)。

> 状态：已闭合（2026-08-15 用户裁决通过 + 独立判定一致；2026-08-15 全面审查后
> 完成 P1 修复——4 MiB 最终响应累计、`$ref` 运行期错误结构化、基线人工模拟边界
> 显式声明）；下一步=orz 内嵌集成（黑板动作栏为生产协作接缝）。

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
python sample2_editor_benchmark.py --out sample2_result.json
python sample3_script_benchmark.py --out sample3_result.json
```
