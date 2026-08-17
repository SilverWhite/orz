# 古法机械执行助理（Classical Execution Assistant）设计（2026-08-13，v0.5 操作台模型）

> 状态：`pending`（P0-C 实施中；小样 1 已通、小样 2 已闭合、小样 3 已闭合，
> v0.5 操作台模型为当前设计形态；orz 内嵌集成 S1 已落地（2026-08-15：
> orz-loop `console` 操作台核心 + 黑板动作栏数据面；同日全面检查修复已闭合
> ——执行器错误细分/响应契约强制/TraceStore 提交语义/最小提示投影/S2 验收点
> 显式登记）；orz 内嵌集成 S2 已落地（2026-08-15：模型面投影 + 轮末发放，
> 见 §8「S2 落地」；S3 trace/PTC/Profile 待续）；正式组件决策门在内嵌集成
> 小样全面达标后裁决；v0.6 收编 PLAN-FIRST 模型面重构（2026-08-15 定案，见 §10））
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
> v0.6 修订（2026-08-15 全面检查修复，用户裁决三项）：**执行器适配层返回
> 可区分执行失败/策略拒绝的丰富结果**——策略拒绝统一映射 `step=policy` +
> `code=policy_denied`（权限/ACAF/taint/模式门）；**任何输出必须过机械
> 验证**——动作契约强制响应 schema，注册时校验并缓存（审计的一部分）；
> 成功退出码契约 `exit_code=Some(0)`（无显式退出码按执行失败；生产 host
> 成功输出已归一化为 0，拒绝由适配层归一化为策略拒绝）；TraceStore 采用
> 提交语义（发放收口后 commit，事件对 `assistant.trace` 可见；trace 满
> 200 后失败事件滚动保底）；注册板块投影=最小参数提示，不复制完整 schema；
> S2 验收点显式化（round/epoch 防重放、真实目标解析、ACAF 票据、策略表）。
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
  | `code` | 机器可读错误码（HA 同构：`invalid_arguments` / `unknown_service` / `not_found` / `out_of_scope` / `policy_denied` …） |
  | `message` | 失败反馈：中性、可解释，含具体细节（沿用 HA 带 `@ data['...']` 校验路径的做法） |
  | `upstream` | 上游执行结果：已发生的部分——解析后真实目标、票据/context id、部分输出（截断）、verifier 摘要；无则 `null`；受既有脱敏与截断约束 |
  | `trace_id` | 本次请求执行日志 id，恒存在；模型可用 `assistant.trace` 按 id 取回全文 |
  | `trace` | 仅 `step=execute` 失败存在：有界日志尾部（最近 N 条事件）——执行失败特殊反馈 |

- 成功信封（`ok:true`）：`response` + `trace_id`（审计关联；生产接
  run-event/journal context）。
- 约束：失败响应必须可独立排障——模型不需要再猜“哪一步错、之前发生了什么”；
  不因失败吞掉已发生事实，也不回传未经验证的内容。
- 策略拒绝（2026-08-15 裁决）：权限/ACAF/taint/模式门拒绝由执行器适配层
  归一化为 `step=policy` + `code=policy_denied`，不落入 execute 失败；执行器
  返回可区分执行失败与策略拒绝的丰富结果。
- 结构化策略拒绝信号（2026-08-15 补登记，P1-2 定案）：拒绝路径（权限门 /
  ACAF 票据门 / 检索模式门；taint 预留）在 `run_host_tool` 边界统一返回结构化
  信号——`ToolResult.policy_denial = {source, code, reason}`（source ∈
  permission | acaf | retrieval_mode | taint；`exit_code=Some(1)`）；console
  适配层（`run_console_target`）只按结构化信号映射 `ExecuteError::PolicyDenied`
  （detail 携带 source/code/reason），**删除「稳定输出前缀」字符串判定**
  （`console_policy_refusal` 退役，不依赖拒绝文案）；ToolCompleted 事件增可选
  `policy_denial` 对象（Schema v0.2 先行扩展，verifier/fixtures 同步；交叉规则：
  存在 policy_denial 时 exit_code 必须非 0 且工具命中已知拒绝路径）；与 denial
  breaker 统一（ACAF/模式门也产生 `PolicyFeedback::Denied`）为可选实施决策；
  测试=结构化断言（permission/acaf/mode → step=policy + source 明细）+ 内容
  碰撞回归（成功输出含旧拒绝前缀文案必须判成功）。实施随 S3 前置
  （TODO/BACKLOG P0-C）。
- 输出验证强制（2026-08-15 裁决）：动作契约必须声明响应 schema；输出无论
  结构化与否均过机械验证后才进入成功信封——验证既是规整性/安全手段，也是
  审计的一部分。schema 在注册时校验并缓存，非法 schema 注册即拒绝。
- 退出码契约（2026-08-15 定案）：动作执行要求显式成功退出码 `Some(0)`；
  `None` 或非零按 execute 失败。生产 host 成功输出已归一化为 `Some(0)`，
  拒绝路径由适配层归一化为策略拒绝，不会落入本分支。
- trace 存储（2026-08-15 修复）：`TraceStore.new_trace` 返回工作副本，
  发放收口后 `commit` 写回，`assistant.trace` 才能按 id 取回事件；单 trace
  满 200 条后失败事件滚动保底（替换最旧），普通事件按 POC 语义丢弃。
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
    墙钟 / 4 MiB 最终响应累计（含全部步骤响应与 `result`，未命名步骤同样计入；
    〔2026-08-16 审查收口：生产单订单上限改为 8 步，见下方 S3 审查收口〕）；
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
  - S2 验收点（2026-08-15 显式登记）：模型面投影=注册板块读取（最小参数提示，
    不复制完整 schema）+ 动作栏写单工具（已有 pending 订单机械拒绝）；轮末
    发放=round/plan_epoch 防重放与过期校验 → 注册表/契约 → 真实目标解析
    （路径/作用域，复用 orz-paths）→ ACAF 票据（解析后真实目标验票）→
    策略表（taint/模式门）→ 经 `run_host_tool` 执行（权限 + 事件链）→
    响应 schema 验证 → 结果栏 receipt + trace_id；策略拒绝映射
    `step=policy`；发放收口执行 `TraceStore.commit`。
  - S2 落地（2026-08-15）：模型面=扩展 `blackboard_read.section` 枚举至
    `actions`（注册板块/动作栏单槽/结果栏有界渲染，随 plan epoch 归档可读）
    + 新增 `blackboard_action_write` 写单按钮（ReadOnly 类、主车道专属——
    子代理投影剥除 + ToolFilter 车道门 + run_host_tool activation 守卫三重
    拒绝；round/plan_epoch/run_id 由机械层盖章，模型只给 action+arguments；
    已有 pending 订单机械拒绝 `order_slot_busy`）；轮末发放=post-tool-batch
    安全间隙（pending checkpoint 优先，推迟后按过期处理）→ 取单（消费一次）
    → `console::issue_action`（注册表/契约/目标/执行/验证；执行经
    ControllerConsoleExecutor 委托 `run_host_tool`，复用权限桥/ACAF 票据/
    模式门/事件链，工具回复丢弃、反馈走结果栏）→ 结果栏 receipt + trace_id
    → `TraceStore.commit`；round/plan_epoch/run_id 三重防重放与过期
    （`step=protocol` / `code=order_stale`，过期订单消费并显式拒绝）；策略
    拒绝归一化（权限门 PolicyFeedback::Denied + ACAF/模式门稳定输出前缀 →
    `step=policy` / `code=policy_denied`，不携带 execute trace 尾部；〔P1-2
    补登记：前缀判定随 S3 前置由结构化信号取代，见 §7〕）；注册
   板块每轮机械刷新（基础动作集 6 项：workspace.read_file/list_dir/grep/
    search_replace/run_tests/index，契约镜像生产 host 参数）。
  - S3 落地（2026-08-15）：注册板块升级为「Profile/Bundle ∩ 探针完整集」
    ——`ActionBundle`（standard/read_only/benchmark 三档，场景键=
    `ToolPolicy`）+ `ServiceRegistry::registrations_for`（Host 动作按工作
    工具探针完整集过滤，非工作工具不探不标；内部动作恒加载）；新增控制台
    内部动作 `assistant.trace`（`ActionKind::TraceRead`：按 trace_id 有界
    取回，读操作本身入 trace 与事件面）与 `workspace.run_script`
    （`ActionKind::RunScript`：PTC 线性脚本——`$ref` 数据引用、逐行契约
    校验 + trace、上限 8 步/30s/4MiB、任一步 fail-closed、失败保留内层
    step/code + `script_step`）；内部动作发放经 ToolStarted/ToolCompleted
    留痕（复用既有事件面，无 Schema 变更）。实施审计见
    [`GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15`](audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md)。
    边界：taint 组合禁令为设计项（运行时未实施），适配层已预留
    PolicyDenied 归一化；PTC 步骤仍逐行过既有权限/ACAF/模式门。
  - S3 审查收口（2026-08-16，用户逐项裁决；审计见同文件 §6）：
    - 单订单步数上限由 POC 定档的 20 改为 **8**（`MAX_SCRIPT_STEPS_PER_ORDER`，
      schema `maxItems` 同步）——单轮动作受控，且每步将计 1 个 tool-round
      预算单位（预算消耗语义随 S4 实施）。
    - `assistant.trace` 查无 trace_id 定案：`step=execute` +
      `code=not_found`——服务已解析、契约已过、存储查询失败属执行阶段，
      失败信封按契约附本订单有界 trace 尾部（可区分无效 id 与环形淘汰）。
    - checkpoint 轮（无探针间隙）**跳过注册板块刷新**，保留上一轮探针
      过滤后的内容；其他无探针轮次仍按 bundle-only 刷新。
    - 脚本内允许调用只读内部动作（含 `assistant.trace`）：非嵌套脚本、
      逐行过五步链与既有门，读步骤输出进入脚本 steps/result 并受 4MiB
      上限约束（有意边界，已登记）。
    - trace 生命周期定案为**会话级**：TraceStore 挂在会话级控制器、
      会话开始时空、结束即弃、侧车恢复不携带；50 条为会话内环形上限，
      跨会话 trace_id 一律 `not_found`（有意边界，已登记）。
    - 注册不变式补齐：内部动作携带 host 目标注册即拒绝（双向 fail-fast）、
      内部动作类全局唯一（嵌套脚本按 kind 拒绝，防第二个 RunScript 名称
      绕过）、bundle 至少启用一个场景。
  - S4 落地（2026-08-16；实施审计见
    [`GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16`](audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md)）：
    - 单步超时下沉 host 层（用户裁决）：`LoopHost::call_tool_with_timeout`
      为每调用携带显式 wall-clock 覆盖；host 按 `min(覆盖, 配置预算)` 截止，
      到期仍走既有进程树收口（`kill_active`，Windows Job Object/TaskKill）。
      脚本每步不传收缩剩余（`None`），由 host 每调用超时独立约束（配置
      预算；单步受控——〔复核裁决撤销 30s 总墙钟，见下方审查收口〕），host
      截止以结构化信号 `ToolResult.timed_out` 上浮——直接订单失败信封
      `step=execute`+`code=tool_timeout`，脚本 runner 归一化为
      `script_timeout` 并携带 `script_step`（不用文案前缀判定）。
    - 脚本消耗 tool-round 预算：每个实际执行动作计 1 单位（直接订单 1、
      脚本每步 1；执行前被拒步骤不计数）；发放前预检 = 当前模型轮 1 单位
      + 脚本长度 ≤ 剩余预算，不足零执行拒绝（`step=protocol` +
      `code=budget_insufficient`，不消耗预算、订单清槽、显式 receipt）；
      按实际执行步数减计并计入 `tool_rounds`，下一轮预算块机械反映，
      耗尽后同样进入最后无工具轮。
    - 端到端测试：FakeProvider 完整任务会话（写 run_script/trace 订单 →
      发放 → trace 读取 → 结果栏反馈 → 下一订单）、checkpoint 轮板块保留
      e2e、超时/预算边界；orz-loop 392 通过 / 0 失败。
    - S4 审查收口（2026-08-16 二次）与超时语义复核裁决：撤销「30s 总墙钟
      含进程时间」语义（对照 Codex `command/exec timeoutMs` 与 Grok Build
      `toolset.*.timeout_secs`/`ProcessScope` 成熟设计）——脚本每步由 host
      每调用超时独立约束（配置预算，默认 5 分钟，进程树收口不变）；host-owned
      同步工具（`project_doc_index`/`browser_read`/`pdf_read`/PDF 路由
      `web_fetch`）不经 host timeout 包装为既有边界，与直接订单一致（不由
      脚本层事后判失败）；预算预检先做脚本静态校验，失败/超上限不预检、交
      注册表/契约校验产生真实错误码；决策门材料补齐小样 1 结果工件
      （`sample1_result.json`，90/90 复跑）；orz-loop 395 通过 / 0 失败。
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
- 参数暴露面：模型只看到名称，还是名称 + 最小参数提示；高频动作如何枚举化
  （2026-08-15 定案：注册板块投影=最小参数提示，type/required/属性枚举/
  默认值；完整 schema 不进模型面）。
- 黑板分区与既有 blackboard_read/todo 机制的关系：复用黑板本体 + 新增分区，
  还是独立动作队列；动作栏写工具命名（v0.5 建议 `blackboard_action_write`，
  语义独立于 todo）（2026-08-15 定案：复用黑板本体，新增 `actions` 分区并
  随 plan epoch 快照归档/轮换；写工具命名已定案并在 S2 落地——
  `blackboard_action_write`）。
- 与单一探针面的事件/列表投影如何交互（2026-08-15 S3 已定案：注册板块 =
  Profile/Bundle ∩ 探针完整集，与模型可见工具投影共用同轮探针快照；
  非工作工具目标不探不标、按注册表声明保留）。
- trace 查无 id 的 step 归属（2026-08-16 定案：`step=execute` +
  `code=not_found`——服务已解析、存储查询失败属执行阶段；见 §8 审查收口）。
- 组件形态（v0.3 已裁决）：宿主内嵌——HA 操作台为 orz 的一部分；POC 的
  stdio 协议仅是原型隔离，不作为生产接缝（薄接缝在 orz ↔ 底座模型后端）。
- 来源/许可登记：进入组件登记表口径后逐项审计。

## 10. v0.6 收编：计划-执行分离与黑板化指挥（2026-08-15）

> 用户裁决（2026-08-15）：助理层承接全部下游执行（console 默认），主模型指挥助理层
> 并以只读方式核查；放弃「直接执行面永久移除」，定案双模式——direct 为受控降级
> （连续 3 次助理层故障面失败 → 无工具询问轮 → 模型选择后切换并留痕；权限不变；
> 计划门约束 console 订单，direct 为有记录的例外）。设计主体见
> [`PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md`](PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md)；
> 权威登记 ADR-0010 §14.17。
> 状态：**定案**（2026-08-15 用户裁决；进入实施路由 BACKLOG/TODO P0-C）。

- 模型面三块（console 默认面）：写面 `plan_write` + `action_write`；读面 `blackboard_read` +
  `assistant.trace` + 工作区只读；禁止直接执行/变更/shell/子代理 spawn/检索。
- 双模式：console 默认 + direct 受控降级（3 连败助理层故障面 → 无工具询问轮 → 模型选择
  switch 后写 `console_mode_transition` + gate_log；direct 动作带 transition_id；计划门
  约束 console 订单，direct 为有记录的例外（`console_step_done` 需证据置 done）；
  `console_return_to_console` 单向返回或 run 结束复位）。
- 首轮计划轮（硬门）：首轮只暴露黑板读取 + `plan_write`，不派发执行工具；注册板块
  = 探针投影唯一事实源（S2 静态基础集为接线前中间态）。
- 分步计划硬契约：有序步骤 + 步骤状态机 `pending → in_progress → done(receipt_id)`；
  下一步订单需上一步 receipt 机械放行，防惯性幻觉。
- 反馈回路：订单 → 发放 → 结果栏 receipt + trace_id → 模型只读核查 → 推进/诊断重试；
  核查面必须覆盖全部副作用出口。
- 阶段：A（去人格 + AGENTS.md 计划型包裹 + 首轮计划轮）、B（注册板块=探针投影、
  工具栏刷新绑定黑板模型栏；**2026-08-16 实施闭合**，审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md`）、C（助理层
  实战验证后模型面收敛为黑板+只读核查；direct 受控降级保留，不永久移除）。

## 11. 大文件读取契约（2026-08-17 用户裁决）

> 用户裁决（2026-08-17）：语义适配留在模型，助理层只提供机械原语（引用 + 范围读），
> 不做语义总结；超过粗门的文件返回读取句柄信封而非全文；黑板只放指针、不放内容本体。
> 权威登记 ADR-0010 §14.22（v1.22）；实施路由 BACKLOG 6f / TODO P1。
> 状态：**已实施**（2026-08-17 用户裁决定案；2026-08-17 本窗口实施闭合）。

### 11.1 问题与原则

- 现状：读取工具对文件返回全部内容，模型只能全量接受；最坏情况（中文 UTF-8，
  1 字符 ≈ 1 token）128KB ≈ 40K+ token，约占单轮 50K 注入预算 80–90%，仍会撞上
  注入层事后拒批，「全文返回」路径不可持续。
- 原则：助理层不理解动作语义（§1 不变量），语义适配（判断哪些内容相关、如何组织
  证据）留在模型；助理层只提供机械的结构化读取原语与引用。

### 11.2 读取句柄信封（工具契约层有界返回）

- 读取超过粗门的文件时，读取工具返回读取句柄信封，不返回全文：
  `{path, size, encoding, content_sha256, 可用范围, 有界预览, truncated, offset 续读指针}`。
- 有界预览 ≤2–4KB；`truncated` 标志 + `offset` 续读指针供模型按需续读。
- 小文件（低于粗门）保持全文一次返回（一次往返，不增加工具轮开销）。
- 粗门默认 16KB、可配 8–32KB（env/TOML 口子）；精门=单轮注入预算
  （默认 50K、`ORZ_MAX_INJECT_TOKENS_PER_ROUND`）为最终兜底。
- 设计意图：把「注入层事后拒批」前移为「契约层事先有界返回」，避免先生成全量再被
  预算拦截；与 pdf_read 的 `document_id` + `page_range` 先例对齐
  （文本文件用 path + offset）。

### 11.3 模型侧结构化读取

- 模型对超过粗门的文件采用结构化读取：grep/结构提取优先 → 证据关键文件才全文 →
  大文件 offset 分段；既有提示词策略化读取（ADR-0010 §3.6 v1.9）落成工具契约，
  不再依赖纯提示词纪律。
- 读取句柄信封是模型读面的机械契约（console 默认面与 direct 一致）：拿到信封后
  用 `read_file(offset)` / `grep` 自取所需范围；grep 空结果语义不再依赖纯提示词
  纪律——grep 工具契约升级为搜索信封（§12，2026-08-17 用户复核定案），
  「搜索 0 文件」与「真无匹配」机械分型。

### 11.4 黑板边界：只存指针、不存内容

- 黑板是控制面状态视图，不是内容缓冲；文件全文等大内容不得写入黑板。
- 黑板/结果栏只放指针（path/document_id/size/digest/offset），内容本体留在盘上
  或内容寻址证据区（如 pdf-evidence 管线），避免 epoch 快照膨胀、retention/恢复
  成本上升与过期副本；维持「不新增自由随记区」约束（ADR-0010 §3.6）。
- 与既有机制一致：压缩动作台账行（工具/目标/结果指针-digest）、黑板路径槽
  （Top-40 + 5K 字符 + 溢出指针）均为指针模式先例。

### 11.5 登记

- ADR-0010 §14.22（v1.22）；FUS-LARGE-FILE-READ-CONTRACT（`current-design`）；
  `docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §4（读面机械契约）；
  BACKLOG 6f / TODO P1（实施路由）。
- 2026-08-17 实施闭合：orz 子模块 172b14e（GrokBuild `read_file` 文本路径
  信封）；测试 orz-tools read_file 199 / orz-loop 440 / orz-host e2e 2；
  实施审计 `docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md`。
- 2026-08-17 全面检查修复：P2-1 空窗口/越界 offset 语义（past-EOF
  truncated=false/offset=None、范围内空窗口 offset=start_line、最后一行行内
  截断 offset=None）；P3-1 `[toolset.read_file]` 配置节端到端接线
  （coarse_gate_bytes 经 orz-config 分层装载注入工具参数、优先于 env）；
  P3-2 concise 描述同步；P3-3 envelope 不追加 cursor rules 边界；
  P3-4 提示词措辞精确化。orz 子模块 7c4a99e + bd8d485；
  ADR-0010 §14.22 项 4 登记。

## 12. 搜索范围契约（grep 搜索信封）（2026-08-17 用户复核定案）

> 用户复核（2026-08-17）：撤回「补文本范围报告」的方向——那只是让空结果更可
> 诊断，不修搜索本身，且局限在 grep 单点；定案=工具契约升级为搜索信封，与 §11
> 读取信封同属「范围/截断必须机械报告」契约家族。权威登记 ADR-0010 §14.23
> （v1.23）；实施路由 BACKLOG 0a / TODO P0-E。
> 状态：**定案**（2026-08-17 用户复核；纯设计登记，未实施）。

### 12.1 问题与证据

- 冒烟重跑（`D:\tb-eval\jobs\2026-08-17__03-48-57`）中 7 次 grep 全部
  exit_code=1、wall 1–36ms；pattern 含 vm.js 实测存在的字符串
  （syscallNum/entryPoint/sectionsToLoad/runElf 等），read_file/list_dir 正常——
  说明 rg 没有搜到文件，而非「无匹配」。
- 根因（工具层）：`finalize_grep` 把 exit 1 + 空 stdout（或 exit 2 +
  "No files were searched"）统一转成 "No matches found"；ORZ 总是显式传路径，
  而 ripgrep 只在隐式路径时打印 "No files were searched" 警告，该分支在 ORZ 下
  是死代码——「搜索 0 文件」（ignore/隐藏/glob/二进制/超限过滤干净）与
  「真无匹配」被合并。
- 后果：模型收到空结果只能猜（轨迹中自述「可能在工作区外被限制」），并叠加浅层
  list_dir 把空结果泛化为「无 C 源码」错误转向。

### 12.2 搜索信封（工具契约层范围报告）

- grep 返回结构化搜索信封：`{resolved_root, files_searched, files_skipped,
  match_count, truncated}`；**v1 实施定稿**：机械来源=空结果路径（非零退出 +
  双流为空）用 `rg --files` 探针计数（与主搜索同过滤集、10K 截断杀子进程），
  命中路径 `files_searched` 留空；弃用 `--stats`——rg 15 输出到 stdout、旧版
  输出到 stderr，跨版本位置差异会污染流式面与卡片一致性（后续如需每次返回可改
  `--json` 面）。
- 结局三型分型：
  - searched>0 且 match_count>0：正常命中；
  - searched>0 且 match_count=0：真无匹配；
  - searched=0：范围空——显式报「resolved root 下候选全部被过滤」及过滤类别
    （ignore/隐藏/glob/二进制/超限），绝不表述为 "No matches found"。
- exit 2 语法错误（非法正则/glob/type）保持硬失败（既有行为保留，不并入空结果）。

### 12.3 搜索范围语义

- 现状缺口：read_file/list_dir 可见而 grep 全空，三工具可见集不一致；rg 默认
  尊重 ignore/隐藏文件，read_file/list_dir 不。
- 定案方向（实施时按容器内冒烟结果定其一）：
  a. grep 默认与只读工具可见集对齐（关 ignore/隐藏过滤），或
  b. 保留 rg 默认语义但显式提供 `--no-ignore`/`--hidden` 开关（与 glob 同进参数
     面），且 skipped 计数必须可见。
- **2026-08-17 冒烟定案=b**：容器实机复现（`alexgshaw/make-doom-for-mips:
  20251031`）显示默认语义无问题（正常 rg 对 /app 搜 `syscallNum` 命中 20 处、
  `--stats` 10 files searched；doomgeneric .gitignore 只忽略构建产物）——本次
  空结果根因是内嵌 rg 的 glibc 版本不匹配（构建容器 trixie 要求 GLIBC_2.39、
  运行容器 bookworm 2.36），加载失败退出 1 + 空 stdout 被 `finalize_grep`
  吞成 "No matches found"。因此：保留 rg 默认语义，参数面新增
  `--no-ignore`/`--hidden` 开关；工具契约补机械规则 ①非零退出且 stderr 非空
  先显式报错；②`--stats` 解析 files_searched 入信封；构建侧改静态 musl rg。
  **2026-08-17 实施闭合**：结局三型 + `rg --files` 探针 + `hidden`/`no_ignore`
  开关 + build.rs 静态守卫 + 构建脚本静态 musl rg；grep 模块 42 / types 561 /
  orz-loop console 68 测试通过；list_dir ignored/truncated 计数为后续项。
- 模型侧：空结果不再等于「无文件」；范围空时按信封反馈换范围（--no-ignore/换
  路径），先 list_dir 建清单仍为次要契约提示。

### 12.4 泛化

- 「范围/截断必须机械报告」是工具契约家族约束：读取信封（§11/ADR-0010 §14.22）、
  搜索信封（本节/§14.23）、目录信封（list_dir 补 ignored/truncated 计数，随实施）——
  同一模式，避免逐工具打补丁。

### 12.5 登记

- ADR-0010 §14.23（v1.23）；FUS-TOOL-SCOPE-CONTRACT（`current-design`）；
  `docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §4（读面机械契约）；
  BACKLOG 0a / TODO P0-E（实施路由）。

## 13. Benchmark 完全体执行面（终端/网络两轴放开）（2026-08-18 用户裁决）

### 13.1 问题与原则

TB2 冒烟（`D:\tb-eval\jobs\2026-08-17__23-29-44`，make-doom-for-mips）reward 0
根因=评测配置 shell-less：Benchmark 策略对 shell/网络 fail-closed，且 console
默认面注册表缺终端动作——模型「下单无门」与「下单被拒」同时存在。设计原则：
shell 不开放为模型直接工具，只作为助理层注册动作经订单下发（与
`workspace.run_tests` 同构）；按任务合规放开策略允许面，审计面不减。

### 13.2 三层使能

1. 权限层（orz-host `permission.rs`）：`PermissionPolicy::Benchmark` 参数化为
   `Benchmark { allow_shell, allow_network }`（默认 false/false）；shell 工具
   （run_terminal_cmd/bash/cmd/powershell/pwsh）与 SandboxEscape 别名在
   allow_shell 下 AllowOnce，NetworkCall 在 allow_network 下 AllowOnce；MCP 恒
   deny、工作区读限定不变。
2. 探针层（`tool_probe.rs` / `host.rs`）：`ToolPolicy::BenchmarkFull`；
   `tool_policy()` 由 `Benchmark{allow_shell:true,..}` 映射；
   `policy_allows_exec` 增加 BenchmarkFull；`ActionBundle::allows` 加臂复用
   benchmark 档。
3. 注册表层（console.rs）：新增 `workspace.run_terminal`（见 §13.3）。

### 13.3 动作契约 workspace.run_terminal

- target_tool：`run_terminal_cmd`；bundle：`READ_WRITE`（standard+benchmark）。
- input_schema：`command`（必填）、`description`（必填）、`timeout`
  （1–300000 ms，可选）、`is_background`（可选）；`additionalProperties: false`。
  不暴露 env/cwd（host 决定；ACAF command_exec 摘要基于 host 侧 cwd/env）。
- response_schema：text_output 信封。
- 投影：探针 Complete ∩ bundle allows(BenchmarkFull) → 动作栏出现。

### 13.4 CLI 与适配器

- `--allow-shell` / `--allow-network` → ORZ_ALLOW_SHELL / ORZ_ALLOW_NETWORK；
  必须与 `--allow-write` 同用（exit 2）。
- 适配器：allow_shell 恒真；allow_network = task network_mode == PUBLIC。

### 13.5 安全面

- ACAF fail-closed 票据（command_exec_v1/network_v1）仍为最终授权兜底。
- 预算/墙钟/停滞守卫、事件审计链、订单/step 门不变。
- 容器层 harbor 按任务建网，NO_NETWORK 任务无网（纵深一致）。

### 13.6 登记

- ADR-0010 §14.24（v1.24）；`docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md`；
  `docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §4；CLI_PROJECT_INDEX
  （FUS-BENCHMARK-FULL-EXEC，`current-design`）。

## 14. 状态行缓存纪律与订单拒绝的步骤语义（2026-08-18 用户裁决）

### 14.1 问题与证据

TB2 复验 run（80 请求）命中率 66.5%（旧批次 95–99%）：18 次
`request_header_change` 全部为 system 变化，tools/config 恒定。根因链：
① console 步骤机每笔订单 receipt 成功即推进步骤（done），状态行
`[任务状态 v0.1]`（render_status_line）渲染 goal + 已完成 + 当前步；
② 状态行嵌入系统提示词（A4 常驻块）；③ 步骤推进 → 状态行变化 →
system_sha256 变化 → 前缀缓存整段失效（断点后请求命中 0.2–4%、
未命中 2万–13万 token）。

### 14.2 状态行缓存纪律（修复项 1）

- 系统提示词恢复完全静态：预算块 + 基础提示 + plan-first 框架（删除
  render_status_line 注入）。
- 状态行改为**变化时追加**的尾随用户消息（`sync_status_line_message`）：
  计算 render_status_line，与上次追加值比较，变化才推送并记录；无计划
  （None）不推送。与 `[TOOL_ROUND_BUDGET] REMAINING` 同纪律——前缀保持
  命中，变化轮仅小段新状态行未命中。
- 模型每轮仍可见当前步（尾随消息），不回归 P0-E 第 4 项 step_id 渲染。

### 14.3 订单拒绝的步骤语义（修复项 2）

- `record_console_receipt` 增 `mutate_step`：Ok → true（done）；Err →
  仅 execute/verify（执行失败）才迁移为 failed；发放期拒绝
  （registry/contract/target/policy，含 policy_denied/ACAF/模式门）不改
  步骤状态（保持发放时置的 in_progress，订单可重试）。
- `build_status_line` 当前步 = 第一个非 done（与
  `planning::current_step_index` 门禁一致）；failed 步骤显示为当前可重试。

### 14.4 登记

- ADR-0010 §14.25（v1.25）；CLI_PROJECT_INDEX 顶部登记行（状态行缓存纪律 +
  订单拒绝步骤语义）。**实施已闭合（2026-08-18，orz `ba86910`）**：
  orz-loop 443 通过 / 0 失败、clippy 与基线一致；实施审计
  `docs/audits/GAP_STATUS_LINE_CACHE_STEP_RECEIPT_IMPL_AUDIT_2026-08-18.md`。
