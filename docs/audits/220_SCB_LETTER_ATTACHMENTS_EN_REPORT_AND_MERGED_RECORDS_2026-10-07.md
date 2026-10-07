# 220 批：SCB 询问信附件 1／2 产出——英文精简版全轮报告＋196 档合并逐档记录（2026-10-07）

> **用户令**：「请继续」（承接 219 批尾「要不要我接着起草附件里唯一还缺的实质件——全轮报告
> 英文精简版」，同批取附件 2 的合并逐档记录）。
> **性质**：**附件产出批**——新增英文报告档（仓内）＋合并记录工件（仓外 `0cr_official/`）；
> 零源码、零跑批；**计数不变 54**；未提交、未推送。
> **编号注记**：218＝邻窗 0.8.15 双平台重建批（本窗收口）；219＝本窗询问信修订；本批 220。
> **结论先行**：附件 1（英文精简版全轮报告 14 节全文）与附件 2（196 行合并逐档记录＋CSV＋
> summary）双双产出，读数与中文 authority／报告 §13 逐位一致；附件 3／4 成包仍未做。

---

## §1 附件 1：英文精简版全轮报告

- **新档**：[`docs/en/SCB_V1_36_FULL_ROUND_REPORT_EN.md`](../en/SCB_V1_36_FULL_ROUND_REPORT_EN.md)，
  对照中文 authority [`docs/SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md`](../SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md)。
- **结构**：14 节齐备（摘要 → 口径与装置 → 收官总账 → 36 题逐题表 → 成绩面形态学 → 官方对照与
  口径边界 → 摩擦台账 8 小节 → 转换忠实度脚注 → 数据完整性与账目更正 → 披露要件清单 → 边界与
  不可外推项 → 复现入口 → 官方多维聚合 → 成本与消耗），逐段压缩为英文精简形态；36 行逐题表与
  4 处账目更正全数保留。
- **术语**：按 [`TRANSLATION_GLOSSARY`](../en/TRANSLATION_GLOSSARY.md) v1.0（carrier／write
  control／session volume／blackboard／journal／friction／k=1 screening round／at-the-wall pass
  等）；**题单位用 SCBench 官方词 `problem`**（术语表该条限定 TB 语境，且与询问信一致）。
  固定声明行在件：`LLM-assisted translation; the Chinese originals are authoritative.
  Corrections welcome.`
- **关键读数对表（与中文版逐位一致）**：checkpoint **134/196＝68.4%**；全档 **11/36＝30.6%**
  （Easy 7／Medium 2／Hard 2）；isolated 满通过 **60/196＝30.6%**；strict 满通过 **44/196＝22.4%**；
  verbosity 0.289／erosion 0.613；**¥157.49 ≈ $22.0**；**$/CKPT $0.112**。

## §2 附件 2：196 档合并逐档记录（仓外工件）

- **生成器**：`D:/tb-eval/scbench/0cr_official/merge_checkpoint_results.py`（只读枚举 36 个
  0cr run 目录的官方 `checkpoint_results.jsonl`）。
- **产物**（同目录）：
  - `merged_checkpoint_results.jsonl`——196 行＝36 题逐档，官方记录原字段＋`problem`／
    `difficulty`／`run_dir`；
  - `merged_checkpoint_results.csv`——同 196 行、复核友好子集 18 列；
  - `merged_checkpoint_results.summary.json`——计数＋两产物 sha256。
- **自校验读数**：problems＝36、checkpoints＝**196**、isolated_full_pass＝**60**、
  strict_full_pass＝**44**、core_full_pass＝**134**——与报告 §13 逐位一致。
- 该件即询问信附件 2 中「合并逐档记录」的兑现（`multidim_aggregate.json` 为 210 批既有件）。

## §3 台账

- **信件本体**：附件清单第 1／2 两条改为已产出并补入路径（`docs/en/SCB_V1_36_FULL_ROUND_REPORT_EN.md`／
  `merged_checkpoint_results.jsonl|.csv|.summary.json`）。
- **术语表**：§3 文档映射 +1 行（`SCB_V1_36_FULL_ROUND_REPORT_EN.md` ↔ 中文 authority）。
- BACKLOG：计数行（**54 不变**）＋本批指针；第二卷 §1.166。
- TODO：头部计数行（54 不变）本批指针。
- 索引：头行 v4.192 → **v4.193**。

## §4 边界

1. **零源码、零跑批**；附件 2 生成器只读 36 个 run 目录，未改写任何原始件。
2. 英文报告系 LLM 辅助译文，**中文原版为权威**（固定声明在件）；中文后续若修订，英文须同步
   （术语表 §4 维护纪律）。
3. 附件 3（4 缺陷明细）与附件 4（逐题工具分布 `readings_*.txt`）**尚未成包**——发送前核对清单
   「附件包生成（上列四件）」项仍未勾。
4. 未提交、未推送（与邻窗 218 批在途改动同树，提交批另行放行）。

## §5 关键词

220 批、SCB 询问信附件、附件 1 英文精简版全轮报告、SCB_V1_36_FULL_ROUND_REPORT_EN、附件 2
196 档合并记录、merged_checkpoint_results、60/44/134 自校验、术语表文档映射、problem 词形、
计数 54 不变、索引 v4.193、附件 3/4 待成包。
