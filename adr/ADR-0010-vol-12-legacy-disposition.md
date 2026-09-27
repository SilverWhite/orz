# ADR-0010 分卷 12：§12 旧设计条款处置

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 12（§12 旧设计条款处置）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 12. 旧设计条款处置

| 来源 | 处置 | 仍有效内容/说明 | 归档路由 |
|---|---|---|---|
| ADR-0003 | partially superseded | provider-neutral assurance boundary 保留；外部 runtime 完整拥有 loop/session 的裁决废止 | `adr/` 原位保留并加状态标记 |
| ADR-0005 | superseded | 历史阈值仅供 replay/案例解释；当前方向问询以 §4.2 为准 | `adr/` 原位保留并加状态标记 |
| ADR-0006 | inherited | 三个 Credential Manager target 与统一注入方式 | `adr/` current |
| ADR-0007 | inherited | transport retry、stream timeout、cancel/timeout 分离 | `adr/` current |
| ADR-0008 | partially superseded | 40 改为 120；其余预算语义保留 | `adr/` 原位保留并加状态标记 |
| ADR-0009 | inherited | A/B/C 写点与 `GROK_HOME` 降级链 | `adr/` current |
| Fork/Agent Loop/Integrated v0.1 系列 | transferred + superseded | 融合来源、LoopHost、blackboard、relay 保留；旧逐 crate/component 结论待 §11.5 重新审计 | `存档/architecture/pre-adr-0010/` |
| Integrated v0.2 | transferred + superseded | 薄 host、同构 Agent、assurance 分类已转录；旧 runtime ownership matrix、thinking disabled 与旧问询废止 | `存档/architecture/pre-adr-0010/` |
| Implementation Deviation and Correction | evidence_only | sidecar/fork 与早期 inquiry 偏差历史 | `存档/architecture/pre-adr-0010/` |
| CN Agent Base review | transferred + evidence_only | 成熟优先、双子代理、显式检索模式、Diagnostic Coverage、Global Review、IDE lifecycle 边界已转录；不自建 loop 限制废止 | `存档/docs/design-inputs/` |
| Self-Question/Counterexample design | transferred + evidence_only | neutral/counterexample/stagnation 分离和一次性 answer gate 已转录 | `存档/docs/design-inputs/` |
| Inquiry Fix and Blackboard supplement | clause-split | output/tool/action 方向信号废止；blackboard、compaction、whitelist 转录 | `存档/docs/implementation-history/` |
| Run Stall Guards plan | transferred + evidence_only | tool timeout、wallclock、heartbeat 与实施修正已转录 | `存档/docs/implementation-history/` |
| FIX_PLAN 2026-08-06 | clause-split + evidence_only | model/thinking/transport/budget、D-1 来源绑定和 D-9 测试反馈经重裁后保留；D-3 累计拒绝 10 次废止 | `存档/docs/implementation-history/` |
| Retrieval Sub-Agent audit | transferred + audit | task/result/close lifecycle 保留为审计证据；no-model fixture 不代表生产能力 | `docs/audits/` |
| Source Fulltext Visibility rule | transferred | visibility 与 claim 边界已转录 §3.7 | `存档/docs/design-inputs/` |
| Local Browser Retrieval/PDF design | transferred | 显式状态/异常、browser ownership、PDF evidence、URL/JS/prompt-injection 边界已转录；旧线性流程和具体 timeout/tab 数不冻结 | `存档/architecture/pre-adr-0010/` |
| Session Persistence/Layout | clause-split | Grok-owned session 废止；Toolbar 与只读 session/run-history 投影保留为可演进 current UI baseline | `存档/architecture/pre-adr-0010/` |
| Windows Runtime Contract | clause-split | §1–5 为 runtime spike；产品 compatibility 内容转录 §6 | `存档/architecture/runtime-spikes/` |
| Python Reference-Spec Contract | inherited | Schema authority、镜像同步与变更纪律继续有效 | `architecture/current/`（后续移动） |
| audit/incident/raw run | evidence_only | 不因设计转录而删除或升级为规范 | `docs/audits/`、`docs/incidents/`、ignored evidence store |

