# ADR-0010 分卷 09：§9 实施顺序

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 09（§9 实施顺序）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 9. 实施顺序

### Phase A：权威冻结

1. **已完成**：接受并冻结本 ADR；
2. **已完成**：标记 ADR-0003/0005/0008 与历史融合设计的 supersede 状态；
3. **已完成**：建立 current design authority index 和 archive 路由；
4. 从本 ADR 冻结起，历史设计不得直接指导新实现。

### Phase B：设计与 Schema 重整

1. 本 ADR 已承担融合架构唯一自然语言基线，不再另写并列的“current 总设计”；
2. Schema/contract 只细化同构检索子代理 state machine，不得复制或改变本 ADR 的产品语义；
3. 问询、机械 assessment 与生命周期通过版本化 Schema/fixture 细化，不再创建并列的方向 ADR；
4. 扩展 orientation/diagnostic/retrieval assessment/close/event Schema 和 fixtures；
5. 独立编写 Windows platform compatibility design。

### Phase C：实现重构

1. 子代理复用主 Agent runtime；
2. 接入真实检索工具、结构化结果与持久 session；
3. 分离 orientation、diagnostic coverage、mechanical information sufficiency、retrieval close、counterexample
   和 stagnation；
4. 接入显式 retrieval mode、结构化 source binding、受控 `run_tests` 语义和新的 denial breaker；
5. 删除废弃字段、旧 producer 和只为错误逻辑存在的代码；
6. 按新 acceptance matrix 重建测试。

### Phase D：文档归档

1. **首批已完成**：以 §12 处置矩阵和归档 metadata 形成 supersede/引用关系；
2. **首批已完成**：移动 17 份历史设计、设计输入、实现计划与 runtime spike；
3. **首批已完成**：更新项目索引、README 和所有命中旧路径的 current 引用；
4. **文档侧已完成 / 实现侧待 Phase B/C**：已运行旧路径、authority wording 和归档 metadata 检查；Schema、journal replay 与实现测试在 producer/Schema 迁移切片执行。

