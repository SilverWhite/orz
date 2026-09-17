# 项目历史存档

本目录集中保存已经退出当前设计基线、但仍需保留用于来源回溯和演进审计的材料。归档内容不得独立产生当前实现需求；发生冲突时，以 [`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) 为唯一自然语言设计权威。

- [`architecture/`](architecture/)：ADR-0010 前的架构设计及早期 runtime 探针。
- [`docs/`](docs/)：已转录的设计输入和阶段性实现历史。
- [`index/`](index/)：退出当前维护面的完整项目索引快照和实施时间线。
- [`backlog/`](backlog/)：退出当前维护面的完整待办快照（含变更记录与已闭合分区明细）。
- [`todo/`](todo/)：退出当前维护面的实施勾选清单全量快照（含已闭合条目勾选明细）。
- [`readme/`](readme/)：退出当前维护面的仓库 README 快照和历史产品叙述。
- [`ter-review-2026-09-04/`](ter-review-2026-09-04/)：TER M1/M2 全面审查的分片原始报告（2026-09-17 由未随仓的 `.ter_review_2026-09-04/` 工作目录入库；供 [`TER_M1_M2_COMPREHENSIVE_REVIEW`](../docs/audits/TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md) 与 [`TER_REVIEW_HANDLING`](../docs/audits/TER_REVIEW_HANDLING_2026-09-04.md) 回溯）。

当前设计入口仍位于 [`architecture/current/`](../architecture/current/README.md)，审计、事故与案例记录仍分别保留在 `docs/audits/`、`docs/incidents/` 和 `docs/cases/`。

快照为归档时点的逐字节原样副本，其中的相对链接按归档时原路径（仓库根或 `docs/` 等）解析，在本目录下打开会失效；回查时请按各快照「原路径」栏锚定解析。仓库门禁的链接检查对归档区豁免（`scripts/check_repository.py` `_check_markdown_links`）。
