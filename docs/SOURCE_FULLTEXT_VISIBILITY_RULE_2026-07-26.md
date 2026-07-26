# Source full-text visibility rule（2026-07-26）

## 裁决

凡涉及文献、帖子、网页、论坛 thread、报告或其他外部文本来源的检索与讲解，输出必须固定回报每个关键来源的全文可见性状态。

不能完整抓取并浏览全文时，必须显式标注为 partial、metadata-only 或 unavailable；不得把摘要、引言、搜索片段、引用页、二手转述或模型记忆当作全文证据使用。

该规则优先保护机制解释、方法细节、限制条件、反例和语境依赖结论。若未看到完整全文，只能基于已观察片段做窄结论，并把可能缺漏的机制、实验设置、限定条件和相反证据列为未排除风险。

## 必报字段

每次使用外部文本来源时，来源 ledger 至少应记录：

- `source_id`：稳定标识符、URL、DOI、帖子链接或本地路径；
- `source_type`：paper、preprint、book chapter、blog、forum_post、thread、documentation、report、other；
- `access_time`：访问时间或本地读取时间；
- `visibility_status`：`full_text_observed`、`partial_text_observed`、`metadata_only` 或 `unavailable`；
- `observed_scope`：实际读到的部分，例如 title/abstract/introduction/methods/excerpts/comments/search snippet/full body；
- `missing_scope`：未能读取或未确认读取的部分；
- `claim_allowed`：允许使用该来源支撑的最高 claim 层级；
- `limitation_note`：缺漏如何影响机制、方法、数据、限制和反例判断。

## 状态语义

- `full_text_observed`：已读取完整正文或完整帖子/thread 内容，并能说明访问路径。附件、补充材料和外链仍需单独标注。
- `partial_text_observed`：只读取摘要、引言、节选、页面预览、部分楼层、部分评论、引用片段或非完整 PDF/HTML。
- `metadata_only`：只看到标题、作者、日期、DOI、索引页、搜索结果、引用信息、摘要数据库条目或其他元数据。
- `unavailable`：来源存在但无法访问、无法解析、被 paywall/权限/格式阻断，或只从二手来源获知。

## Claim 边界

1. 未达到 `full_text_observed` 时，不得声称“论文/帖子认为”“全文显示”“作者机制是”等全文级结论。
2. 只看到摘要或引言时，机制、方法细节、限制条件、负结果、补充实验和讨论中的 caveat 默认未知。
3. 对文献综合任务，必须把 full-text coverage 作为证据完整性维度；未读全文的来源不得用于关闭相反证据缺口。
4. 对帖子或论坛 thread，必须区分主帖、编辑记录、评论、回复链和被引用外链；只读主帖不等于读完整 thread。
5. 二手转述可以作为线索，但不能替代原文全文；使用时必须标为 secondary/partial，并保留回源需求。

## 当前实现状态

本规则已新增首个轻量机械门禁：

- `assurance/source-visibility-ledger-v0.1.schema.json`：登记来源、引用、检索预算、已读范围和缺失范围；
- `assurance/source-visibility-gate-receipt-v0.1.schema.json`：输出每条引用的 `allow/defer/block`、所需可见性、实际可见性和可降级 claim；
- `assurance/source_visibility.py` 与 `assurance/source_visibility_cli.py`：对 ledger 执行 deterministic gate；
- `assurance/fixtures/source_visibility/mixed-visibility-ledger.json`：覆盖 metadata-only、partial abstract 和 full thread 的正反混合 fixture。

当前仍未实现 crawler、全文下载器、runtime adapter 或 EvaluationRunner 接入。在 CLI 魔改版可用前，应先让检索工具或模型 wrapper
生成 ledger，再由 gate receipt 决定哪些引用可用、哪些必须降级或继续分批抓取。

该规则不替代独立 counterexample queue、中性 orientation checkpoint、runtime stagnation guard 或后续脱敏机制；
它是更靠前的来源完整性保护层。
