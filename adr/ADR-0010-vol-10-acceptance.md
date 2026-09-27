# ADR-0010 分卷 10：§10 验收条件

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 10（§10 验收条件）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 10. 验收条件

本 ADR 已满足以下冻结条件；后续实现验收必须持续保持：

- 明确确认 ORZ 融合 control plane 所有权以及成熟优先准入顺序；
- ADR-0003/ADR-0005/CN 的 supersede 状态无歧义；
- current architecture 不再同时声称“外部 runtime 拥有 loop”和“ORZ 自研 loop”；
- 子代理设计明确同构范围、允许差异、生命周期与结构化结果；
- 问询/评估设计明确六类职责、7 轮 session-level 语义、机械 Information Sufficiency，以及 normal close
  必须由 parent disposition 明确确认；
- 新事件 Schema 有稳定 discriminator，不依赖文案区分；
- Windows 产品设计与 process spike 分离；
- 归档方案保留历史证据、Schema replay 和 Git 历史；
- 已知错误测试已登记为 implementation migration blocker；删除/重建属于 Phase C 验收，不得因旧测试
  仍通过而宣称符合 current contract。

