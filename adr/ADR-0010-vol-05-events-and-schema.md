# ADR-0010 分卷 05：§5 事件与 Schema 演进

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 05（§5 事件与 Schema 演进）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 5. 事件与 Schema 演进

### 5.1 明确类型，不用文案充当协议字段

事件必须具有显式 discriminator。v1.1 至少拆为以下独立 event type 和 payload Schema：

```text
orientation_checkpoint
diagnostic_coverage_checkpoint
information_sufficiency_assessment
retrieval_parent_disposition
retrieval_close_record
```

只有前两者带 `inquiry_family = neutral`，并以 const `inquiry_kind` 与 envelope `event_type` 交叉校验。
Information Sufficiency 与 Retrieval Close 是机械事件，不得带 inquiry family 或模型生成的充分性结论；
Parent Disposition 是主 Agent 的结构化生命周期命令，只能选择 `close|continue`，不能改写 assessment。
不再扩张通用 `neutral_inquiry` 的 `oneOf` 大对象。Counterexample 和 Stagnation 继续使用各自独立 event
type。不得通过文案前缀判断机制；文案用于模型交互，不是稳定协议身份。

### 5.2 Payload 边界

- Orientation payload：session/agent identity、触发点、轮次状态、message block、注入位置；
- Diagnostic Coverage payload：debug episode、threshold stage、去重后的 hard-signal identity、coverage block、
  一个最小补诊断动作；
- Information Sufficiency payload：task/retrieval contract、result/ledger identity、source counts、visibility、
  categories、missing/filtering facts、机械状态、reason codes、assessment version；
- Parent Disposition payload：disposition/parent/subagent/activation identity、assessment identity、
  `expected_contract_revision`、`close|continue`、`requirement_delta`、capability-gate result；`continue` 时
  requirement delta 必填；
- Retrieval Close payload：parent/subagent session、activation/contract/result/assessment/archive identity、
  validated disposition identity、terminal reason、resumable 和 live-state-reset；正常结果 close 必须引用
  `decision=close` 的 disposition，终止 authority close 必须引用 cancel/failure/wallclock reason；
- Stagnation payload：机械 metric、decision、restart/handoff evidence；不得混入 inquiry counters；
- 所有事件必须有明确 producer、consumer、verifier 与 migration version。

### 5.3 迁移原则

1. 先扩展 Schema 和 good/bad fixture，再修改 Rust producer；
2. producer 与 verifier 在同一迁移切片更新；
3. 旧事件版本只用于历史 journal replay，不作为新实现依据；
4. 任何测试不得锁定已知错误或已被裁决废弃的逻辑；
5. 测试名称中的“符合设计/逐字移植”必须引用当前权威设计和版本，不能只比较代码自身常量。

### 5.4 Journal、snapshot 与恢复

1. 产品 journal 使用 bounded background recorder 保持调用面轻量，但事件 append 必须有发送背压和
   acknowledgement；同一 session 内 sequence 单调，writer 串行化 hash chain。
2. 每个已接收事件在返回成功前完成 write、flush 和 `sync_all`。不采用“直到 gate 通过才首次落盘”的
   延迟 persist 旧草案；挂死或进程被杀时，应尽可能保留最后一个已确认事件。
3. Session bootstrap 冻结 agent role、model/provider、capability、source、history mode、parent identity
   和 schema version；恢复不重写旧事件，按 §11.6 建立跨版本链接。
4. 变异类工具在 permission 放行后、`tool_started` 前对可确定目标创建 snapshot，并记录
   `snapshot_created` 或 `snapshot_error`。snapshot 失败属于 evidence degradation，不自动阻断动作；
   无法静态确定目标的 shell 写入必须由 workspace delta/journal 补足证据。
5. Restore/revert 只在用户显式请求并通过相应 permission 后执行，产生独立 restore event/receipt；不得
   把“snapshot 模块存在”写成“恢复路径已完成”。
6. Reasoning text、credential、cookie、auth header 和私有 provider transcript 不进入 journal。必须记录的
   usage/config 只保留结构化数值、枚举、digest 和脱敏错误类别。

