# 古法机械执行助理（Classical Execution Assistant）设计（2026-08-13，v0.5 操作台模型）

> 状态：`pending`（P0-C 实施中；小样 1 已通、小样 2 已闭合、小样 3 已闭合，
> v0.5 操作台模型为当前设计形态；正式组件决策门在小样全面达标后裁决）
> 日期：2026-08-13
> v0.2 修订（2026-08-13 用户裁决）：**不需要理解层**。助理层不解析意图、不理解动作
> 语义，只按主模型输出（工具名/动作名 + 参数）逐层定位到组件并实际实施；
> 模型面只注册名称——给主模型套一个真正的“操作台”，而不是工作环境。
> 文法/FST/NLU 不再是核心，降为可选增强（v0.1 内容保留作参考）。
> v0.3 修订（2026-08-13 用户裁决）：**助理层与 orz 深度融合**——HA 操作台
> 是 orz 的一部分；薄接缝改置 orz 本体 ↔ 底座（模型后端，Grok/Codex 等），
> 不是 orz ↔ ConsoleClient。POC 的 stdin/stdout JSON 协议仅为原型隔离形态，
> 不作为生产接缝。借鉴 DeepSeek Harness（2026-08-13 发布 v0.1 预览、MIT）
> 两个设计点：PTC 程序化工具调用、Profile/Bundle 动作组合（只借设计本身，
> 不引入其技术栈）。
> v0.4 修订（2026-08-13 用户裁决）：**执行失败特殊反馈 + 模型可见执行日志**。
> 错误信封恒带 `trace_id`，`step=execute` 失败附有界 trace 尾部；注册只读服务
> `assistant.trace`，主模型可结合日志与操作台覆盖未预录内容。机械组合模型
> （PTC 线性脚本模式）与主模型配合规则登记（小样 3 候选）。
> v0.5 修订（2026-08-13 用户提案）：**指令发放改为“搭积木”**——黑板新增
> 注册板块（当前轮可用动作，机械刷新、常驻按需读）与动作栏（模型写订单，
> 写无副作用），模型轮结束后机械层发放（单一出口），结果写回结果栏
> （receipt + trace_id）。模型面降为“读板块 + 写订单”——执行幻觉只能污染
> 订单，被机械校验拦下，不能直接产生执行。
> 定位：主 Agent 的机械执行配合部件——操作台形态（注册表路由 + 契约校验 +
> 动作实施 + 验收），全部确定性、可解释、可测试；不新增平行协议，不拥有决策权。
> 待办与决策门：[`BACKLOG_AND_PRIORITIES.md`](BACKLOG_AND_PRIORITIES.md) P0
> （P0-C，POC 已通）。

## 1. 目标与不变量

- 目标：把主模型从执行细节（路径、行号、hunk、命令构造）中解放出来；
  模型只输出“名称 + 参数”，机械层负责“定位 → 实施 → 验收”。
- 不变量 1：执行层全部确定性——无模型参与、无随机、无隐式切换。
- 不变量 2：任何输出必须过机械验证（schema/verifier），失败 fail-closed；
  失败返回必须含具体失败点（step）、失败反馈（code/message）与上游执行结果
  （upstream），供主模型独立排障。
- 不变量 3：不触碰权限、票据、claim、MAP/R 等决策链；本组件只做
  “意图 → 动作计划 → 验收”，不改变任何既有设计条款。
- 不变量 4：复用现有机制（探针/ACAF/run-event/verifier），不建第二套审计链、
  不建第二套协议、不引入新语言运行时。
- 不变量 5（v0.2）：模型可见面 = 注册名称（工具名/动作名），模型不接触工作环境；
  助理层按名称逐层定位组件并执行，不理解动作语义。

## 2. 成熟开源参考（“古法”来源）

| 来源 | 许可 | 形态 | 改装借鉴点 |
|---|---|---|---|
| **Home Assistant / hassil**（首选参考） | Apache-2.0 | 服务注册表（domain.service + schema 参数 + 确定性执行 + REST/WS API）+ HassIL 文法意图解析；活跃维护 | 操作台本体：细粒度服务注册、意图文法、执行与响应；自定义服务承载 orz 动作 |
| StackStorm | Apache-2.0 | 事件驱动自动化：actions（参数 schema）+ rules + workflows + 执行审计 | ops 级规则与审计形态；基础设施重（MongoDB + RabbitMQ） |
| Node-RED | Apache-2.0（OpenJS 基金会） | 流式低代码执行：node = 细粒度动作，HTTP 入口 | 动作接线/流程执行；引入 Node.js 运行时 |
| OpenVoiceOS（ovos-core） | Apache-2.0 | 旧时代助手 OS：skills = 细粒度动作 + intent service | 旧助理整体形态；语音向、重，不作为主选 |
| Rhasspy / fsticuffs | MIT | 离线助手；FST 意图解析 | **2025-10 已归档**；仅借 fsticuffs 算法形态 |
| Mycroft Adapt | Apache-2.0 | 关键词/实体意图解析器 | 轻量意图 + 槽位提取（参考） |
| CLIPS | public domain | NASA 规则引擎 | 规则表/决策策略形态（不引入 C） |
| RiveScript | MIT | 触发-响应脚本引擎 | 固定话术/兜底消息脚本化思路 |
| Microsoft Bot Framework | MIT（官方标记 deprecated） | waterfall 对话框 | 状态流形态（仅借鉴，不接入） |
| Rust 生态（fst/pest/tree-sitter/fuzzy-matcher/similar/petgraph/rhai） | MIT/Apache 系 | 原生库 | 文法、FST、代码锚点、模糊匹配、diff、决策图、规则脚本——均已在或可用 |

改装原则：取架构与算法形态，不整机移植；实现走 Rust 原生，与现有 host 同构。

> v0.2 范围修正：文法/FST 意图解析不再是核心——主模型输出本就是结构化工具调用；
> 核心参考转为“注册表 + 路由 + 契约 + 命令模式”：Home Assistant 服务模型
> （domain.service + schema + 执行 + 响应）为当前最接近的成熟参考；hassil 文法
> 意图保留为可选的自然语言绑定层；现有 orz-host dispatch、tool registry、
> ACAF action kinds、CLIPS 规则形态（策略表）继续作为既有部件。
> 调研结论（2026-08-13）：Rhasspy 已归档（2025-10）、Rasa 进入维护模式、
> n8n 非真开源（Sustainable Use License）、Windmill 为 AGPL——均不作为底座候选；
> Home Assistant / hassil（Apache-2.0、活跃）为操作台模型的首选成熟参考。

## 3. 组件形态（host-owned 窄组件）

```
主模型受限结构化意图
  → 控制台注册表解析（动作名 → 域 → 组件 → 入口，逐层定位）
  → 契约校验（输入 schema + 值枚举/约束 + 路径/作用域解析，复用 orz-paths）
  → ACAF 票据（动作种类 + 解析后真实目标）
  → 组件执行（现有工具实现 / host-owned 工具，如删除安全、编辑执行器）
  → run-event journal + verifier 验收（receipt）
```

- 感知面：工具探针（A+C→B 单一探针面）提供“什么可以被提议”的机械事实。
- 操作台：模型面只有按钮（注册名称）；参数仍由模型输出，但被契约收紧
  （类型、枚举、作用域、上限），高频动作设计成“参数尽量少/枚举化”的按钮。
- 接口（v0.5）：操作台不是独立进程，而是 orz 内部模块——HA 助理层与 orz
  深度融合，HA 是 orz 的一部分；薄接缝位于 orz 本体 ↔ 底座（模型后端，
  Grok/Codex 等）。生产协作形态 = 黑板动作栏（见 §8）：模型面只有“读注册
  板块 + 写动作栏”，不直接构造执行调用；POC 的 stdin/stdout JSON 协议
  仅为原型隔离形态，不作为生产接缝。
- 借鉴登记（v0.3，DeepSeek Harness 2026-08-13 发布 v0.1 预览、MIT）：
  只借设计本身，不引入 TS/Node/Cordis 栈——
  - **PTC 程序化工具调用**：主模型输出程序化动作脚本（一段确定性的工具
    调用编排），操作台逐行执行、任一步失败 fail-closed；把多轮工具往返
    压缩为单个可校验执行单元，减少模型轮次与幻觉累积。
  - **Profile/Bundle 动作组合**：按场景（Benchmark / ReadOnly / 标准）
    预打包动作集，操作台按当前模式加载对应“按钮组”；与策略表/模式门
    合并，决定当前加载哪套动作。
- 动作粒度（v0.3 方向裁决）：细粒度优先——粗按钮（run_terminal_cmd 什么都做）
  等于把参数幻觉交给模型，才是限制模型；细动作必须配套机械组合层
  （PTC 程序化调用 / 管道 / 意图），组合发生在机械层、模型只写脚本，否则
  细=碎片化，同样限制。切分按“机械可验证的最小可复用步骤”，变体由参数
  枚举/契约收紧吸收（如 read_file 的 encoding/offset），避免语义重叠动作膨胀。
- 负担与稳定性（2026-08-13 评估）：细粒度把成本从模型运行期搬到构建期——
  每动作一份注册 + 输入/输出 schema + handler + 探针/策略映射 + 冒烟 fixture；
  数量线性时成本线性，运行时开销近零（查表路由 + schema 校验）。真正的风险
  是边界漂移与变更连锁；护栏：动作契约按版本化 API 管理（新增优先、废弃走
  迁移期、参数向后兼容）、schema/声明驱动注册、探针决定可见性、
  Profile/Bundle 分区控制本轮按钮集、契约即测试（表驱动）压低测试成本。
- 执行日志与模型可见性（v0.4）：每次请求生成有界 trace（step/action/ok/
  code/message/upstream 事件序列，单 trace 上限 200 条、保留最近 50 个请求）；
  错误信封恒带 `trace_id`，`step=execute` 失败附有界尾部（执行失败特殊反馈）；
  注册只读服务 `assistant.trace`（按 trace_id 取回，有界、脱敏、读操作本身
  入 journal）——主模型可结合日志与操作台覆盖未预录内容。
- 编辑执行器（候选首块）：主模型只提交“目标文件 + 语义锚点 + 期望修改”，
  规则层做锚点匹配（fuzzy-matcher / tree-sitter）、hunk 计算（similar）、
  dry-run 与上下文断言，全对才落地。
- 槽位表（v0.2+，POC 已闭合）：由工作区索引动态生成——`workspace.index`
  服务有界扫描 allow root（跳过隐藏/构建/依赖目录），hassil 槽位取值来自
  索引，自然语言面只认识索引内对象；结构化调用不受此限（契约 + 作用域兜底）。
  生产实现复用 orz `project_doc_index` 缓存。
- 测试输出分诊器（候选次块）：按 pytest/JUnit/CTRF 确定性解析失败，
  决策表映射到下一步动作；模型只拿结构化摘要。
- 命令构造器（候选次块）：模型从封闭模板选动作 + 填参数，
  规则层校验模板、参数类型、路径作用域。

## 4. 与现有机制的关系

- **OPS-PROTOCOL**：本组件承接其裁剪方向——保留删除安全（回收站 + 缓存机械分类 +
  容量 fail-closed）为 host-owned 工具、跨环境桥接内部化、执行器单一化；
  本组件不再作为模型协议。
- **ACAF**：授权面不变；动作计划必须持票后执行，验票对象仍是解析后真实目标。
- **探针**：感知面不变；探针决定模型可见集，本组件决定“被提议后如何编译执行”。
- **明确不做什么**：不做对话/语音、不做通用理解、不做第二 runtime、
  不替代主模型决策、不修改 permission/claim 语义。

## 5. 验证口径（最小小样）

1. **小样 1（控制台路由小样，已跑通 2026-08-13）**：注册 3–5 个动作
   （如 search_replace / run_tests / run_terminal_cmd / read_file / file.delete），
   输入 = 名称 + 参数 fixtures → 逐层定位组件 → 契约校验 → 执行 → verifier。
   通过标准：fixtures 全过、路由确定、无模型参与、无自由文本解析；
   POC 当前覆盖 `workspace.read_file` / `workspace.list_dir` /
   `workspace.index` / `assistant.trace` + hassil en/zh 意图；错误信封
   step/code/message/upstream + trace_id（执行失败附有界 trace 尾部），
   28/28 检查通过。
2. **小样 2（后续）**：编辑执行器对照实验——主模型直接生成 hunk vs
   意图 → 规则层 hunk；指标 = 编辑应用成功率、主模型工具轮数。
   通过标准：编辑失败率显著下降或持平且轮次不增。
   边界：baseline 的 hunk 尝试序列为人工编写模拟（非真实模型输出），语料固定，
   证据为 POC 级，不构成生产端到端结论。
3. **小样 3（机械组合脚本模式，已闭合 2026-08-15，用户裁决通过 + 独立判定一致）**：线性脚本
   （步骤 = 注册动作实例 + `$ref` 数据引用）把多轮工具往返压缩为单个可校验
   执行单元；指标 = 脚本执行成功率、主模型工具轮数（baseline=逐轮串行调用，
   candidate=单脚本一轮）；通过标准 = 候选成功率 ≥ 基线且候选平均轮数 ≤ 基线
   （目标为严格减少轮数）；fail-closed 与静态校验场景全部按契约拒绝且无部分
   执行副作用。POC 实现与结果见 `prototype/classical_console/`（小样 3 一节）。
   边界：baseline 的串行调用序列为人工编写模拟（非真实模型输出），语料固定，
   证据为 POC 级，不构成生产端到端结论。
4. **失败处置**：小样不达标即撤，不扩为正式机制（符合复杂度治理判定：
   只砍冗余、不砍不变量、不做机制×不变量清单）。

## 6. 策略执行点与威胁模型（2026-08-13 补充）

- **助理层 = 单一策略执行点**：主模型所有动作只经操作台出口；现有围栏
  （permission scope、ACAF 票据、连续拒绝熔断、工具轮预算/wallclock、
  模式门）合并为一张策略表，不新增第二套机制。
- **动作组合禁令（taint）**：结构化输入输出契约使数据流可见——不可信来源
  （web_fetch/browser_read 内容）进入 session 后，file_write /
  run_terminal_cmd / credential_read 需重新授权或拒绝；阈值判定（连续拒绝、
  候选上限、失败率）统一走策略表。
- **威胁模型（用户裁决 2026-08-13）：只防幻觉与注入**——即被误导的主模型
  做出越权/错误动作；不防已攻破的执行器/助理层自身、同用户恶意进程、
  管理员/内核（与 ACAF §2.2 一致）。
- **沙盒分层**：动作级策略沙盒成立（默认拒绝、单一出口、不可绕过）；
  OS 级执行沙盒不自动成立——底层进程仍以宿主权限运行，需按动作挂载
  Job Object / AppContainer / Windows Sandbox VM / 容器；操作台提供统一
  挂载接缝（隔离作为执行器参数）。

## 7. fail-closed 返回契约（2026-08-13 补充）

- 依据（HA 原项目实测结论）：Home Assistant 对调用方的明确返回只有两层——
  成功 `result:{context,response}`（context 为审计关联 id，response 可为 null）；
  失败 `success:false + error:{code,message}`，message 携带具体校验路径
  （如 `required key not provided @ data['period']`）。**没有失败点，也没有
  上游执行结果**——对主模型排障不够，本组件必须扩展。
- 错误信封（`ok:false`）固定四键，全部机械构造、无模型参与：

  | 字段 | 含义 |
  |---|---|
  | `step` | 具体失败点，枚举：`protocol` / `intent` / `registry` / `contract` / `target` / `execute` / `verify` / `policy`（预留 ACAF/taint/模式门） |
  | `code` | 机器可读错误码（HA 同构：`invalid_arguments` / `unknown_service` / `not_found` / `out_of_scope` …） |
  | `message` | 失败反馈：中性、可解释，含具体细节（沿用 HA 带 `@ data['...']` 校验路径的做法） |
  | `upstream` | 上游执行结果：已发生的部分——解析后真实目标、票据/context id、部分输出（截断）、verifier 摘要；无则 `null`；受既有脱敏与截断约束 |
  | `trace_id` | 本次请求执行日志 id，恒存在；模型可用 `assistant.trace` 按 id 取回全文 |
  | `trace` | 仅 `step=execute` 失败存在：有界日志尾部（最近 N 条事件）——执行失败特殊反馈 |

- 成功信封（`ok:true`）：`response` + `trace_id`（审计关联；生产接
  run-event/journal context）。
- 约束：失败响应必须可独立排障——模型不需要再猜“哪一步错、之前发生了什么”；
  不因失败吞掉已发生事实，也不回传未经验证的内容。
- 模型可见日志：`assistant.trace` 只读服务（输入 trace_id/tail，输出有界事件
  序列）；trace 事件结构化（step/action/ok/code/message/upstream），读操作
  本身入 journal；生产按脱敏与截断约束（不暴露凭据/令牌/未授权路径）。

## 8. 机械组合与主模型配合（v0.4）

- 目标：细动作 + 机械组合层 + 模型可见日志，使“覆盖不足”不阻塞任务——只要
  基础动作足够，主模型可结合操作台与日志发放实际指令。
- 组合模型（PTC 脚本模式，小样 3 候选）：主模型输出确定性脚本，操作台逐行
  执行。

  ```json
  {"script": [
    {"do": "workspace.read_file", "with": {"path": "a.txt"}, "as": "a"},
    {"do": "workspace.read_file", "with": {"path": {"$ref": "a.path"}}, "as": "b"}
  ]}
  ```

  - 步骤 = 注册动作实例；`as` 命名输出，`$ref` 引用先前步骤输出字段；
    无任意代码、无隐式控制流；每步独立 schema 校验 + trace 事件。
  - 静态可检查：引用存在性、类型匹配、作用域；上限：步骤数、墙钟、trace。
  - 循环/条件暂不做（线性优先），跑分有需要再加有界 map/条件。
  - POC 定档（2026-08-15；2026-08-15 全面审查后 P1 修复）：上限=20 步 / 30s
    墙钟 / 4 MiB 最终响应累计（含全部步骤响应与 `result`，未命名步骤同样计入）；
    禁止嵌套脚本；`$ref` 运行期解析失败以 `invalid_reference`
    （`step=execute`，`upstream` 带 `ref` 与 `script_step`）结构化返回；失败信封
    保留内层 `step`/`code`，`upstream` 携带 `script_step` 与内层失败明细，
    后续步骤不再执行。
- 黑板动作栏（v0.5 用户提案，定为生产协作形态）：“搭积木”发放——黑板拆三块：
  1. **注册板块**（助理层维护，模型只读）：当前轮可用动作投影——探针
     （A+C→B）∩ Profile/Bundle 加载集；内容 = 动作名 + 最小参数提示 +
     枚举/作用域；每轮机械刷新，板块常驻、内容按需读取（不整块塞上下文；
     用户确认）。
  2. **动作栏**（模型写）：拼装出的订单——单动作或脚本（PTC）；写动作栏
     本身无副作用；带本轮 round/epoch。
  3. **结果栏**（助理层写）：发放后 receipt + trace_id + fail-closed 错误
     （step/code/message/upstream/trace）。
  - 发放语义：模型轮结束后机械层检测动作栏未消费订单即发放（模型面不出现
    执行/发送类工具）；消费一次、发放后清空（或标记 consumed）；round/epoch
    防重放与过期；发放时过注册表 + 契约 + 策略表（ACAF/taint/模式门）。
  - 控制原理：写订单无副作用，副作用只发生在单一发放出口——执行幻觉最多
    污染订单，被机械校验拦下，不会直接产生执行。
  - 单轮一单（用户确认，先定）：本轮订单未发放完不进入下一轮写单，反馈闭环驱动。
- 配合规则（v0.5）：
  1. 模型需要动作时查看注册板块（按需读取）→ 拼装订单（单动作或脚本）→
     写入动作栏；模型面不出现执行/发送工具。
  2. 模型轮结束，机械层发放：注册表/契约/策略校验 → 执行 → 写结果栏
     （receipt + trace_id）。
  3. 失败分两类：请求错误（protocol/registry/contract/target → 模型改订单）；
     执行失败（execute/verify → 模型查结果栏/trace 诊断后重试/换动作/组合）。
  4. 未覆盖内容：模型用结果栏 + `assistant.trace` 看已发生步骤，再以注册
     板块的基础动作组合覆盖；不要求动作全集预先覆盖一切。
  5. 反馈闭环：动作栏 → 发放 → 结果栏（envelope + trace_id）→ 模型下轮读取 →
     调整订单 → 重试；全部确定性、可审计。

## 9. 开放问题

- 动作粒度：方向已裁决——细粒度优先 + 机械组合层配套（见 §3）；具体动作
  分解仍按小样 2 与跑分失败模式产出，避免语义重叠动作膨胀。
- 参数暴露面：模型只看到名称，还是名称 + 最小参数提示；高频动作如何枚举化。
- 黑板分区与既有 blackboard_read/todo 机制的关系：复用黑板本体 + 新增分区，
  还是独立动作队列；动作栏写工具命名（v0.5 建议 `blackboard.action_write`，
  语义独立于 todo）。
- 与单一探针面的事件/列表投影如何交互。
- 组件形态（v0.3 已裁决）：宿主内嵌——HA 操作台为 orz 的一部分；POC 的
  stdio 协议仅是原型隔离，不作为生产接缝（薄接缝在 orz ↔ 底座模型后端）。
- 来源/许可登记：进入组件登记表口径后逐项审计。
