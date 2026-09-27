# ADR-0010 分卷 07：§7 设计权威与归档

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 07（§7 设计权威与归档）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 7. 设计权威与归档

### 7.1 当前设计权威层级

从高到低：

1. 当前 accepted ADR；
2. current architecture；
3. subsystem contract/Schema；
4. implementation plan 与 acceptance matrix；
5. source code 与测试；
6. audit/incident/case evidence；
7. archive 中的历史设计。

实现与测试不能反向把已废弃逻辑升级为设计。审计和事故是证据，不自动成为产品规范。

### 7.2 归档规则

1. 被取代的设计用 `git mv` 移入统一 archive，保留历史；
2. 归档文件头必须记录原路径、归档日期、最后状态和 `superseded_by`；
3. current index 只路由当前权威，不把 archive 作为默认实现依据；
4. 历史 audit、incident、raw evidence 不因设计过时而删除；它们进入独立 evidence/history 路由；
5. Schema 若仍需 replay 历史 journal，必须保留并标明 legacy/replay-only，不能简单移动后失联；
6. 批量归档前必须做引用扫描，同一提交更新 current index、链接和 supersede 标记。

冻结目标结构：

```text
adr/                                      # 当前 accepted ADR
architecture/current/                     # 仍需独立呈现的 current projection/contract
存档/architecture/pre-adr-0010/        # 已由本 ADR 取代的融合前/过渡设计
存档/architecture/runtime-spikes/      # 早期 process/runtime spike
docs/audits/
docs/incidents/
docs/cases/
存档/docs/design-inputs/
存档/docs/implementation-history/
```

