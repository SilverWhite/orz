# 186 批：全面审查发现处置——0cq S2 补钉＋残余面收窄＋批档勘误＋设计卷边界登记（2026-10-04）

> **用户令**：「请对审查出的全部问题进行处理吧。全部处理完成后请进入重建」＝主会话全面审查（未推送面＝父仓 184/185 两落账＋orz `001c7768`/`c001d106` 两落码）5 项发现处置放行＋载体重建放行——**0cq S3 执行形态自此定案＝直接修码批收口＋重建进体**（185 批 §4「杂项狗粮轮同载 or 直接修码批」随本令裁决）；S4 级真机零误拦读数仍留下一真机轮。
> **性质**＝审查处置＋落码批；落点 `orz/crates/codegen/orz-tools/src/types/exec_policy.rs`（单文件）＋父仓两份文档（184 批档勘误注记／0am 预测段设计卷边界登记）；契约面零变化（`BLOCK_RULES` 封闭集、schema v0.3、`write_control_review` 事件族形状全部不动）。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| 发现处置 | **5/5 全部闭合**：P2×1＋P3×3＋P4×1（§1 逐条）；代码三件（单文件）、文档两件 |
| 代码① | **P2 闭集钉补齐**：`READ_PATTERN_OPTIONS`(15)／`READ_PATTERN_KV_PREFIXES`(6) 补入 `closed_trigger_tables_hold_their_registered_sizes` 防膨胀钉——185 批「闭集」声明自此有机械执法 |
| 代码② | **P3 detail 文案回归修复**：规则 1 fundamental-tree-root detail 恢复 `\` 续行（185 批误改为单行引入 34 个字面空格）；渲染回归单空格；无测试钉此文案（assurance 301/0 未红＝如实记） |
| 代码③ | **P3 复合残余面收口**：新增 `segment_has_real_redirect`（与全局 `redirect_arms` 同口径的段级版）——`segment_is_write` 重定向臂由「段内有 `>`」收窄为「段内有**非 nullish** 重定向」；复合命令无关写动词武装后，仅含 `2>/dev/null` 类弃音槽的读段不再被当写段扫描（修复前 `touch /proj/a && cat /proj/.gsa/… 2>/dev/null` 仍误拦）；`>` 段末无目标位保守按真 |
| 文档④ | **P3 184 批档 §3② 在盘勘误**（标注式，原文保留）：run2 回归件与 r178 基线档 **sha256 逐位全等**（对 r178 无翻转）；登记的 16→17／6→5 翻转实为对照 **175 时代基线** `s3_run2_20261003.json`（Oct 3 20:36，r178 修正前）才存在；载荷结论（本批零路由/喂入漂移）不变且更强 |
| 文档⑤ | **P4 设计卷 §2.3 边界补登记**：固定网格漏检语义入卷（样点间窄穿越向「θ上持续/不越线」保守误归、64 点分辨率下结构性罕见）——`scan_crossing` 代码注释「如实登记于设计卷」自此属实 |
| 验证 | orz-tools **2998/0**（2997→2998＝＋复合命令钉 1，全带真阳性对照臂）；触碰面 fmt 干净；clippy 零新增；assurance/loop 零触碰不需重跑（`rli.rs`/`controller.rs` 本批零改动） |
| 后续 | **重建批随令执行**（bump 0.8.12→0.8.13＋双平台重建＋进体判据，承接 184/185/186 三批进件）；打包/发行顺延（沿 155/170/180 口径）；不推送 |
| 计数 | **不变 58**（0cq 线内处置；无立项／无闭合） |

## §1 五项发现逐条处置

| # | 级别 | 发现（185 批后主会话全面审查） | 处置 |
|---|---|---|---|
| 1 | P2 | 两新闭集表未进 0cb 防膨胀钉——代码注释称「新增表项必改测试」但无机械执法 | 补 `assert_eq!(READ_PATTERN_OPTIONS.len(), 15)`＋`assert_eq!(READ_PATTERN_KV_PREFIXES.len(), 6)` 入 `closed_trigger_tables_hold_their_registered_sizes` |
| 2 | P3 | 规则 1 detail「fundamental tree root」文案 185 扁改单行引入 34 空格（原 `\` 续行＝单空格）；进模型面与 journal | 恢复续行写法（与 185 批前逐字节同形） |
| 3 | P3 | 复合命令残余误拦面：`segment_is_write` 把 nullish `>` 也算写段标志，无关写动词全局武装后 nullish 读段仍被扫描 | `segment_has_real_redirect`（非 nullish 判定段级化）＋钉 `compound_nullish_redirect_read_segment_stays_allowed`（Allow 臂＋真实写 `.gsa` 仍拦对照臂）；规则 5 注释块 (b) 同步 |
| 4 | P3 | 184 批档 §3②「run2 与 r178 基线一处翻转」与在盘工件不符（工件＝对 r178 全等） | 184 批档在盘标注式勘误（§3 条 2 尾）；185 批档不涉 |
| 5 | P4 | `scan_crossing` 注释称漏检「如实登记于设计卷」而设计卷未登记 | 设计卷 §2.3 补边界登记 bullet（保守误归方向＋分辨率论证引用） |

## §2 验证明细

- orz-tools lib **2998 passed / 0 failed**（2997→2998：＋`compound_nullish_redirect_read_segment_stays_allowed`；既有 2997 零破坏＝含 185 批全部真机回归钉与真阳性对照钉）。
- 触碰面 fmt 干净（`exec_policy.rs` 0 diff）；clippy 零新增（触碰文件告警均为先存、位于未触碰区域）。
- `rli.rs`／`controller.rs` 本批零改动——184 批验证读数（assurance 301/0、loop 850/0/3）继续有效。
- 修复自证：发现 3 的 Allow 臂在修复前树上必红（cat 段 nullish `>` 被算写段标志→`.gsa` 读路径命中 carrier-write）——本批红转绿即收口实证。

## §3 台账

- 本档：`docs/audits/186_REVIEW_FINDINGS_DISPOSAL_2026-10-04.md`。
- TODO：`P1-0cq` 节增 186 审查处置行；P1 路由行注记（计数不变 58）。
- BACKLOG：`0cq` 节增 186 条目；头部本批指针。
- BACKLOG 第二卷：§1.140 本批流水。
- 索引：头行 v4.157 → **v4.158**；`GAP-WRITE-CONTROL-FALSE-BLOCK` 条目补 186 处置注记。
- 源清单：`generate_orz_source_manifest.py` 随批再生成（差异恰 1 文件＝`exec_policy.rs`）。
- 提交：orz 子树本批（单文件）＋父仓（档＋勘误＋设计卷＋台账）；**不推送**。
- 机械门禁：orz 提交后源清单再生成＋`check_repository.py` valid。

## §4 关键词

186 批、审查发现处置、闭集长度钉补齐、READ_PATTERN_OPTIONS、detail 续行回归、34 空格、
segment_has_real_redirect、非 nullish 写段标志、复合命令残余误拦面、184 批档勘误、
r178 基线命名混淆、175 时代基线、网格漏检边界登记、保守误归、0cq S3 定案（直接修码批）、
计数 58 不变、重建放行。
