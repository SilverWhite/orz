# 267 批档：0de 上下文治理摩擦综合立项——收束后窗口空转＋全角括号解析＋UI 字段漂移＋遥测字段

> 日期：2026-10-13；性质：**立项批**（零源码、零 API）；计数：**62 → 63**。
> 来源：266 批同轮 web 真人测试（会话 2b1f85d2）的上下文治理追查，用户裁决四件合立一个综合摩擦项。

## 1. 用户令与裁决

- **立项令**：「请将真问题都立项为一个综合摩擦项吧」——指主会话追查报告中的真问题①②③（收束后窗口空转／全角括号解析／UI「估 ~0K」字段漂移）。
- **遥测项点名查办**：「7 个压缩事件的 rounds_since_last_compaction 全部恒为 0，疑似遥测字段未维护——这点要查一下并且对此进行处理」（本批已完成根因实锤，修复入 S2）。
- **量尺间距裁决（不处理）**：「模型在 693K 这一没有提醒的地方将上下文拉回而不是到 900K 硬拉，我考虑就已经足够了，不必做额外处理」——实际上下文软提醒线（500K）与必压线（900K）之间的模型自治理带维持现状，登记为已接受边界。

## 2. 追查证据（RUN-2b1f85d2-1，只读取证）

载体 0.9.0；工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹\`；本 run 共 7 个 `context_compressed` 事件＋2 次 `context_manage mode=clear`＋尾部机械通知，逐事件核查：

| 时刻(本地) | mode | reason | trigger | after | rounds_dropped |
|---|---|---|---|---|---|
| 17:52:36 | model_summary | context_scale_window | 537,319 | 239,724 | 32 |
| 18:02:18 | model_summary | model_selected | 394,554 | 315,556 | 2 |
| 18:08:19 | model_summary | context_scale_window | 411,384 | 354,221 | 1 |
| 18:23:47 | model_summary | context_scale_window | 635,129 | 314,884 | 5 |
| 18:43:28 | model_summary | model_selected | 693,563 | 272,682 | 5 |
| 18:50:31 | **clear**（模型主动） | — | 382,271 | 171,261 | 87 |
| 19:02:27 | **clear**（模型主动） | — | 364,589 | 148,186 | 96 |

- 模型两次 clear 均先写黑板 handover（【交接摘要·R221 成文】／v2），清零含旧压缩块（`blocks_cleared [30,31]`／`[33,34]`），清零后回读材料续写无断裂——**0cz 工具使用面为设计内良性使用**。
- 19:03:45 最终汇报发出、收尾门走完；**19:05:19–19:05:45 机械窗口又触发一轮**：模型被迫进压缩参与轮，报告「当前可压区间为空（全部块已清零）」且「压缩窗口摘要区间行未解析（原行含全角括号与说明文字）」→ 按审计项 `summary_not_landed`／`mandatory_summary_not_landed` 走「机械面补交」迟交流程后 `run_finished`。

## 3. 四件根因（S1/S2 落点）

> **勘注（2026-10-16，277 批 S1 勘定）**：本节 F1/F2 机理描述经 `RUN-2b1f85d2-1` journal
> seq 级重建后两处勘误（原文保留于下，勘正表述见
> [`设计档 §1/§3`](../CONTEXT_GOVERNANCE_FRICTION_COMPOSITE_DESIGN_2026-10-16.md)）——
> **F1**＝事发尾段并无「窗口重开」（H1 窗 19:02:27 已按设计收口 `model_participated=true
> rounds_used=1`，seq 536），逼轮实为**窗口收口后自愿迟交摘要消费路径的告知注入**
> （seq 556→558）；修复方向更正为「消费路径收束豁免（D3）＋H1 开窗可压集合前置（D1）」。
> **F2**＝「只认裸 N-M 形」不准确（0bk S1 宽容化已在事发载体 0.9.0 在役），真实根因＝
> **注解内全角逗号被分隔符切分误切**（事发行注解「…覆盖，本摘要为迟交落账）」切出的
> 第二段无数字核心 ⇒ 整条 Unrecognized）；修复＝分隔符切分前剥除平衡括号注解段（D5）。

- **F1（P1）收束后机械窗口空转＝触发面与模型治理状态缺联动**：机械 `context_scale_window` 的武装/触发不感知「窗口已被模型 clear 治理、run 已进入收束段」，终报长文使面回涨即再开窗。锚点＝`orz/crates/orz-loop/src/agent_loop.rs`（窗口触发面）；修复方向（S1 定案）＝治理状态感知（clear/model_selected 后窗口重武装延迟／收束段豁免）。〔机理已由 277 批勘误，见上勘注〕
- **F2（P1）压缩块区间行解析零宽容**：`[SEMANTIC_SUMMARY]` 的 `压缩块: N-M` 行只要带全角括号行内说明即解析失败，触发 `summary_not_landed` 补交。锚点＝0ap/0ah 压缩交互链摘要区间行解析；修复方向＝解析宽容化＋格式契约同步。〔根因已由 277 批勘误，见上勘注〕
- **F3（P2）web 投影「估 ~0K」字段漂移**：`clear` 模式事件 payload 用 `target_tokens`（本例 171,261），`orz/crates/orz-web/assets/app/projection.js:198` 模板固定读 `estimated_tokens_after`（model_summary 事件字段）→ `num()` 取 0 渲染「估 ~0K」。修复＝payload 字段单源或模板双字段兼容。
- **F4（P2，用户点名）`rounds_since_last_compaction` 恒 0 根因实锤**：v8 分块压缩写点 `orz/crates/orz-loop/src/agent_loop.rs:1625` **硬编码 0**，未接 `rounds_since` 计数器；机械模板轨老写点 `agent_loop.rs:1123` 有维护。本 run 7 事件全为 `mode=model_summary`（全走 :1625）→ 7/7 恒 0 与 journal 完全一致。修复＝计数器贯通两写点＋「分块压缩事件计数器非零」可复现钉。

## 4. 边界与豁免

- 693K 自拉回 vs 900K 必压线间距＝用户裁决「已经足够」，不处理、不立项（§1）。
- 模型的两次主动 clear 与摘要质量＝设计内良性使用，非缺陷、不立项；0cz/0da 工具线不动。
- `rounds_since_last_compaction` 恒 0 为**遥测失真**（事件字段误值），非压缩行为本身异常——修复属可观测性收口。
- 本批零 orz 源码（S1–S4 后续）；orz 子模块不动（HEAD `a942d6f3`）。

## 5. 批序（草案）

S1 勘定（F1 触发面与豁免口径、F2 解析点与格式契约定案）→ S2 落码＋钉子（F4 贯通钉＝分块压缩事件计数器非零可复现；F2 全角括号样例钉；F3 投影对拍钉）→ S3 载体重建进体（字节判据随 S1）→ S4 真机复验（长文写作轮收束段无空转窗＋带全角括号说明的摘要可解析＋UI 估数非 0）。

## 6. 登记面与计数

| 文件 | 位置 | 内容 |
|---|---|---|
| `docs/BACKLOG_AND_PRIORITIES.md` | 本批指针行／未闭合总数行／优先级总览 P1 行／P1 开放项锚点／`### 0de` 节（0dd 与 0ct 之间） | 立项登记＋计数 62 → 63 |
| `TODO.md` | 计数行（L20）／P1 路由行（L22）／`### P1-0de` 节（P1-0dd 与 P1-0cq 之间） | 勾选清单 S1–S4 |
| `CLI_PROJECT_INDEX.md` | 头行（v4.239 → v4.240）／canonical 条目 `CONTEXT-GOVERNANCE-FRICTION-COMPOSITE`（`pending`，紧随 0dd 条目） | 召回路由 |
| `docs/BACKLOG_AND_PRIORITIES_2.md` | §1.213 | 本批流水 |

贴限收缩：BACKLOG P1 行 0cv／0da 子句压缩；TODO L22 0cy／0da 子句压缩。计数：**62 → 63**；索引 v4.239 → v4.240；门禁随批运行（结果见提交信息）。
