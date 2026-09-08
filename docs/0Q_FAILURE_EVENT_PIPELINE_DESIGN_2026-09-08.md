# 0q 统一失败事件管线（F4 盖章治本）设计稿（2026-09-08）

> 状态：**S1 定稿（2026-09-08，v1.0）**——四点裁决权由用户授予主代理
> （「这一部分工程化内容我了解不多，你做决定更好」，逐字登记于 §6），
> 四点均按推荐方案裁定；S2–S4 排期见 §5。来源：0p S1 复审 F-C（2026-09-07
> 用户裁决「治标不治本，治本内容单独成一项」）。
> 现状基线：orz `7b00bbc9`（0.3.2；F-C 最小盖章点已落，0p S4 实跑验证）。

## 1. 问题陈述

失败进 `failure_agg` 聚合目前是**逐路径手动盖章**：每个失败路径各自调
`note_failure_agg`（`host_exec.rs` 四写点：锚点拒单 :468 / Ok 臂命令级失
败 `exit_{n}` :3330 / host ToolError :3393 / web_fetch 候选上限 :3644）。
结构风险两条已实证：命令级失败（pip install exit 1 类）漏盖至 0p S1 复
审才发现；console 订单业务失败至今不进聚合。任何新增失败路径都要靠开发
者记忆补盖章——漏盖即静默缺口，且无法机械证明"没漏"。

## 2. 调研：成熟框架的失败面处理方式

核心是自制，失败面收口借鉴六家成熟实现；每家给出对本设计的映射。

### 2.1 OpenTelemetry（语义约定 + 单一 API）

错误记录走单一 API（`span.setStatus(Error)` + `recordException`），而
**"什么算错误"由语义约定的形状规则定义**（如 HTTP span 约定：检测到错
误 → SHOULD 置 Error status + `error.type` 属性；各领域约定指回统一
的 Recording Errors 文档）。教训有二：① 错误边界可以且应该由**形状规
则**定义而非调用方判断；② producer 侧 API 单点是**约定强制**而非结构
强制——生态里长期存在 instrumentation 漏调/争议（RecordError vs
setStatus 的终止/非终止语义争论）。
来源：[Recording Errors](https://opentelemetry.io/docs/specs/semconv/general/recording-errors/) /
[Exceptions on Spans](https://opentelemetry.io/docs/specs/semconv/exceptions/exceptions-spans/)。

### 2.2 LangChain/LangSmith（执行边界单一包裹——最贴近本设计）

`ToolNode` 在**工具执行边界**统一捕获异常：转成结构化 ToolMessage 错误
回模型（保持 agent 循环活着）+ tracer 在**同一个包裹点**给 run 打
`error` 字段。模型可见错误与追踪错误来自**同一信封**，无双重记账；所有
失败（LLM/工具/链/agent）以同一形态进 run tree。教训：**边界包裹点天然
覆盖全部工具**，身份/结果在 args 可及处一并成型。
来源：[RunTree](https://reference.langchain.com/python/langsmith/run_trees/RunTree) /
[LangChain Tools 错误处理](https://docs.langchain.com/oss/javascript/langchain/tools)。

### 2.3 OpenAI Agents SDK（consumer 读形状的反面教材）

Span 携带可选 `SpanError`，TracingProcessor 消费导出。已知事故：某
processor 把所有 span 导出为 `STATUS_CODE_OK`——**从不读 error 字段**，
失败被静默丢弃（GitHub issue #64）。教训：纯消费侧抽取的覆盖力 = 形状
契约的执行力；没有结构执法的 consumer 形状扫描会静默失效。
来源：[Spans](https://openai.github.io/openai-agents-python/ref/tracing/spans/) /
[agentloop issue #64](https://github.com/dipeshbabu/agentloop/issues/64)。

### 2.4 SARIF（身份 = 规则 × 指纹）

每条失败带 `ruleId`（closed vocabulary，来自 tool.driver.rules）+
`level`（error/warning/note 枚举）+ `partialFingerprints`；下游
（GitHub/GitLab）用（类型，主标识，指纹）构造跨 run 稳定身份。教训：
**失败身份 = 规则码 × 目标指纹**，producer 无须知道 consumer；指纹是目
标的确定性哈希——与本仓 `cmd_target.id = sha256(canonical_cmd)` 同构。
来源：[SARIF v2.0](https://docs.oasis-open.org/sarif/sarif/v2.0/sarif-v2.0.html) /
[GitLab SARIF 身份构造](https://docs.gitlab.com/user/application_security/detect/sarif/)。

### 2.5 RFC 9457 Problem Details（错误信封标准形状）

`type`（身份，机器）/ `title`（类型摘要）/ `detail`（本次人类可读）/
`instance`（本次定位）。教训：**身份字段与人类描述分离**——本仓
`policy_denial {source, code, reason}` 已是该形状；F4 盖章沿用即可，无
需新信封。
来源：[RFC 9457](https://www.rfc-editor.org/info/rfc9457/)。

### 2.6 Kubernetes Events（消费侧 correlator + 结构化引用）

拆分契约：机器字段（`type` 枚举 Normal/Warning、`reason` UpperCamelCase、
`regarding/involvedObject` 类型化对象引用）vs 自由文本 `message`（文档明
言 message 不可机析、会漂移）；**聚合/去重/计数是消费侧 EventCorrelator
的职责**（series count + lastObservedTime），producer 原子发事件。教训：
聚合去重天然是消费侧机制，但身份关联必须靠**结构化引用**而非文本解析；
且 reason 词表"约定封闭、无注册执法"正是下游.vendor 各自硬编码词表的
乱象根源——封闭词表要有执法者。
来源：[Event v1](https://kubernetes.io/docs/reference/kubernetes-api/core/event-v1/) /
[client-go events](https://pkg.go.dev/k8s.io/client-go/tools/events)。

### 2.7 调研结论

| 维度 | 业界收敛形态 | 对 0q 的映射 |
|---|---|---|
| 错误判定 | 形状规则（OTel 语义约定），非调用方判断 | 收口判定 = 结果形状谓词，集中一处定义 |
| 捕获位置 | **执行边界单一包裹**（LangSmith ToolNode）是 agent 框架主流 | orz 对应物 = host_exec 工具完成装配点（args 仍可及的唯一通点） |
| 消费侧扫描 | 必须配结构执法，否则静默失效（OpenAI SDK #64） | 消费侧只做**执法**（对账），不做捕获 |
| 身份 | 规则码 × 目标指纹（SARIF）；身份/描述分离（RFC 9457） | 沿用 failure_target 四族 + code，不新增信封 |
| 聚合去重 | 消费侧 correlator（K8s） | failure_agg record/absorb 保持现状（按 kind+id 去重） |

## 3. 设计（推荐方案 α：边界单一漏斗 + 消费侧对账执法）

### 3.1 硬约束（选型前提）

**journal 事件面不带 args**（0p S4 实证：tool_started/tool_completed
payload 仅 tool+call_id+outcome 字段）。纯消费侧从 journal 抽取（备选 β）
在身份推导处即断：`failure_target(tool, arguments)` 需要 arguments，而
journal 里没有。β 要成立必须先把 args（或其指纹）写进事件面——扩大事件
面攻击面/体积，违背事件面最小化纪律。**故捕获必须在写入侧、args 可及
处**；消费侧职责收缩为对账执法。

### 3.2 方案 α 形态

1. **单一漏斗**：host_exec 的工具完成装配点（每个工具结果组装
   ToolCompleted/完成信封的公共路径，四写点的上游汇合处）设唯一的
   `stamp_failure(tool, args, outcome)`：
   - 形状谓词（集中一处，OTel 语义约定形态）：
     `host ToolError ∨ 策略拒绝信封 ∨ (Ok 臂 ∧ 命令级 exit≠0) ∧
     failure_target(args) 命中` ——逐项对齐现有四写点语义，不扩不缩；
   - 身份：复用 `failure_target()` 四族（SARIF ruleId×指纹同构），
     None-identity（无目标工具，如全库 grep）维持**不进聚合**现状；
   - 永远先于事件发出，事件与聚合同源同刻。
2. **四写点退役**：468/3330/3393/3644 就地删除，语义由漏斗谓词等价覆
   盖（逐写点对拍单测钉住：同一输入下旧写点行为 = 漏斗行为）。
3. **console 订单接入**：订单完成装配（receipt error 信封处）同过漏斗，
   身价新增第五族 `action_target {order_id, action}`（SARIF 指纹同构：
   `id = sha256(order_id)`，preview = action 名）。见 §4-②。
4. **消费侧执法**：journal-conformance 法官（任务 D 翻转后的 Rust 单一
   执法面）新增覆盖面对账规则——journal 中每个 error 形状 ToolCompleted
   必须存在对应聚合行（按 kind+id 对账），缺失即法官报错；旧 journal
   （漏斗前）不回溯执法（ grandfather 版本注记）。
5. **裁决四点落地**：见 §4。

### 3.3 不变式（结构保证的覆盖面）

- 每个工具/订单结果**恰过一次**漏斗（装配点结构上唯一）；
- 盖章判定 = 集中形状谓词（一处定义，单测钉住）；
- 覆盖面缺口可被法官机械证明（"没漏"从记忆问题变为可执行断言）。

## 4. 四个裁决点（推荐 + 备选）

### ① 收口点选型 —— **推荐：写入侧边界单一漏斗（3.2-1）**

备选 β：ToolCompleted 消费侧统一抽取。否决理由：args 不入 journal 的硬
约束（§3.1）使 β 需先扩事件面，得不偿失；OpenAI SDK #64 证明无执法的
consumer 扫描会静默失效，而执法本身也是本设计的一部分（3.2-4），β 相比
α 无覆盖力增益、只有事件面膨胀成本。**业界主流（LangSmith）同为边界包
裹捕获 + 消费侧只做展示/对账。**

### ② console 订单失败的 F4 身份来源 —— **推荐：receipt 信封补身份**

新增第五族 `action_target {kind:"action_target", id:sha256(order_id),
preview:action 名}`，订单完成装配处过漏斗。理由：console 订单是真实工
作动作，失败不进聚合正是本次治本清单上的已知缺口；SARIF/GitLab 模式证
身份可在装配点从既有字段（order_id/action）确定性导出，零新增管线。
备选：明确排除并文档标注——若裁决排除，漏斗对订单路径显式 no-op 并留
注释，缺口转为已接受边界。

### ③ 校验面同步 —— **推荐：Rust 法官为唯一执法，Python 对照冻结镜像同步**

沿用任务 D 终态（Rust 单一执法 + Python frozen reference 对拍）：
覆盖面对账规则落在 journal-conformance 法官（3.2-4）；Python
`_verify_v02_failure_target` 同步同一谓词形状作为对照参照（只对拍、不
执法），manifest/registry 若涉 failure_target 词汇变更随批重算。

### ④ 行集语义下游影响 —— **推荐：行集纯增量，零迁移**

新覆盖（console 订单 + 任何此前漏盖路径）只**新增行**，不改变既有行的
kind/id/code 语义；消费方（P2-12 压缩注意事项槽、0p `failures_only` 面）
均为有界行集消费者（cap/截断纪律已在位），行数上界不变式不受增量影响；
旧 journal 不回溯补行（法官 grandfather），跨版本无迁移。唯一可见变化：
`failures_only`/注意事项槽中可能出现此前缺失的 `action_target` 行——正
是治本目的本身。

## 5. 排期与边界（2026-09-08 定稿排期）

- S2 实施（orz 侧单批）：漏斗落 host_exec 装配点 + 四写点退役逐点对拍 +
  `action_target` 第五族 + 法官对账规则 + Python 对照镜像同步；
  registry/manifest 随批。验收：orz 全绿 + clippy 零新增 + 法官篡改负测
  通过。
- S3 测试与复验：覆盖面矩阵单测（四族+第五族 × 各失败形状 ×
  None-identity 排除）+ 逐写点等价对拍全绿 + 法官正反两测（正常刊 0 错 /
  删 agg 行篡改刊报错）。
- S4 收口：BACKLOG/TODO/索引/ADR §14.x 闭合转录 + manifest 重算 + 门禁
  Exit 0。
- 顺序纪律：S2 以本设计定稿为前置门（0n/0p 同款）；批内基线 bump 类
  独立提交先行，不复现 0p T0 排期顺序偏差。
- 边界：不做失败原因的语义分类/归因（只做覆盖面结构与身份）；不动
  failure_agg 渲染/截断纪律；0p failures_only 面消费接口零变更。

## 6. 裁决登记（2026-09-08 定稿）

> 裁决授权（用户 2026-09-08 逐字）：「就按照你的推荐判断进行裁决吧，这
> 一部分工程化内容我了解不多，你做决定更好。请定设计稿并排期吧。」——
> 四点裁决权授予主代理，按推荐方案裁定如下。

- ① 收口点：**写入侧边界单一漏斗**（host_exec 完成装配点
  `stamp_failure`；消费侧只承担法官对账执法）。
- ② console 身份：**receipt 信封补身份**，新增 `action_target` 第五族
  （`id=sha256(order_id)`，preview=action 名）。
- ③ 校验面：**Rust journal-conformance 法官唯一执法**（error 形状 ↔ 聚
  合行对账规则）+ Python `_verify_v02_failure_target` 冻结对照镜像同步
  （任务 D 终态沿用）。
- ④ 行集语义：**纯增量零迁移**（旧 journal 不回溯执法，消费方有界不变
  式不受增量影响）。
