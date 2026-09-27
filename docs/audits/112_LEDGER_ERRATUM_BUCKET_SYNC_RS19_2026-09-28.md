# 112 账面勘误与桶行同步（ALL_FAMILIES 数字误标＋索引 §8 桶行＋0aq 补 RS-19；2026-09-28）

> **日期**：2026-09-28；**来源**＝用户令对 111 批未推送内容做三面全面审查（设计合理性／实现合理性／设计与实现符合性），审查报告三项发现（两 P3 文档面＋一 P2 门禁观察）的处置批；用户裁决「请处理登记文档数字偏差与索引桶行滞后问题，门禁观察可并进0aq」。
> **本批＝零代码、纯账面**：① 111 批「ALL_FAMILIES（66 → 67）」数字误标勘误（实为 **47 → 48**）；② 索引 §8 pending 桶 0bz 行滞后更正；③ 0aq 补 **RS-19**（P2）。**计数 56 不变**（RS-19 并入既有 0aq 线，不新增开放项；`0bz` 维持 `pending`）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 勘误① | 「ALL_FAMILIES 注册（66 → 67）」**误标**——`families.rs::ALL_FAMILIES` 注册表实数 **47 → 48**；66 → 67 是信封 `run-event-v0.2.schema.json` wire 枚举数（envelope 枚举 66 → 67 的写法权威档本就正确） |
| 勘误② | 0bz 状态滞后**两处**更正——索引 §8 pending 桶＋TODO P1 路由行「S1–S4 待启」→「**S1 已落码（111 批）；S2–S4 待续**」（111 批只更了索引 §6 与 BACKLOG 节，两路由行漏更） |
| 新登记 | 0aq **RS-19**（P2）＝门禁不含 Python 合约测试轨——`check_repository.py` 不运行 `pytest runtime/tests/test_run_event_conformance.py`，合约钉值漂移在 HEAD 上红 4 批、穿三次发行未拦截 |
| 边界 | 两笔提交信息（orz `7a6fe91e`／父仓 `e51bc8a3`）同句误标**不改写**（提交信息不可变；pin/manifest 完整性优先），勘误以本档为权威 |

## §1 勘误明细（ALL_FAMILIES 47 → 48）

- **事实核证**（审查批 2026-09-28 机械点数）：`orz-assurance/src/journal/families.rs` `ALL_FAMILIES` 数组 111 批前后＝47 → **48** 条（`face_fingerprint` 已注册，`verify_family` 派发与 Python 镜像 `validate_journal_text` 直调均在）；「66 → 67」的实际出处＝信封 wire 枚举（conformance 测试钉值 65 → 67 的两步校准即此数）。
- **误标位置与处置**：
  1. [TODO.md](../../TODO.md) `P1-0bz` S1 勾选行——就地更正＋勘误标记（本批）。
  2. [BACKLOG 第二卷 §1.63](../BACKLOG_AND_PRIORITIES_2.md) orz 单笔描述——就地更正＋勘误标记（本批）。
  3. orz 提交信息 `7a6fe91e`——**不改写**（提交信息不可变；该提交已被 pin `7a6fe91e`／源清单/索引引用，改写将连锁失效）。
  4. 父仓提交信息 `e51bc8a3`——**不改写**（同理；本批为其后续提交）。
- **写法正确的权威面**（无须改）：111 档 §1.3「信封枚举 66 → 67」／索引头行 v4.79「envelope 枚举 66 → 67」／BACKLOG 计数行同句。

## §2 0bz 状态滞后更正（两处路由行）

- 111 批台账只更了索引 §6 条目、BACKLOG `0bz` 节状态行与头行，**索引 §8 pending 桶行**与 **TODO P1 路由行**漏更，残留「S1–S4 待启」与账面「S1 已落码」自相矛盾。本批两处一并更正为「S1 已落码（111 批）；S2–S4 待续」。桶归属（`pending`）与 token 集不变。

## §3 RS-19 登记（并入 0aq；P2）

- **内容**：`check_repository.py` 门不运行 `pytest runtime/tests/test_run_event_conformance.py`（16 例，实测 0.43s）——合约钉值漂移（094 批 0bw③ 加 `write_control_review` 时 `test_v02_all_event_types_covered` 钉值漏校准 65 vs 枚举 66）自 2026-09-27 起在 HEAD 上连续红 **4 批**（095–110）、穿 **v0.7.1–0.8.2 三次发行**未被任何门拦截，直至 111 批随批勘误（65 → 67）才暴露。
- **登记位置**：BACKLOG `### 0aq.` 节（RS-18 之后）＋TODO `0aq` 节同步；索引 §6 `AUTH-FULL-PROJECT-STRICT-REVIEW` 条目补注记。
- **处置候选**（实施随处置批，本批只登记）：① conformance 套件纳入门禁；② 定期核。**与 0aq 既有主题（门禁覆盖／CI 测试轨 RS-01/RS-02/RS-04）同族，不新立项**（用户裁决「门禁观察可并进0aq」）。

## §4 台账同步

- TODO：计数行改指本批；P1 路由行 0aq「RS-01…RS-18」→「RS-01…RS-19」（零字符增量）＋0bz 状态段滞后更正；`P1-0bz` S1 勾选行勘误；`0aq` 节补 RS-19。
- BACKLOG：本批记录指针 → 第二卷 §1.64；计数行改指本批；`0aq` 节补 RS-19。
- 第二卷：§1.63 勘误标记；新增 §1.64。
- 索引：头行 v4.79 → **v4.80**（v4.79 滚入[存档卷](../../存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md)，155 → **156 行**）；§8 桶行更正；§6 0aq 条目补 RS-19 注记。
- **门禁**：`python scripts/check_repository.py` ⇒ `valid: true`；`git diff --check` 净。

## §5 关联与关键词

[`111 S1 落码档`](111_0BZ_S1_FINGERPRINT_IMPLEMENTATION_2026-09-28.md)／[`110 立项档`](110_CONTEXT_FACE_TRANSIENT_FORK_REGISTRATION_2026-09-28.md)／[`FULL_PROJECT_STRICT_REVIEW`](FULL_PROJECT_STRICT_REVIEW_2026-09-18.md)（0aq 线权威）／BACKLOG `0aq`／TODO `0aq`／`families.rs::ALL_FAMILIES`。

关键词：账面勘误、ALL_FAMILIES 47→48、信封枚举 66→67、数字误标、§8 桶行滞后、RS-19、门禁测试轨、conformance 纳门禁候选、112 批、计数 56 不变。
