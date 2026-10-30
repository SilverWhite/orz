# 0dw 立项登记：模型面量尺与 provider 用量对账缺口（CONTEXT-FACE-SCALE-RULER-ALIGNMENT）（2026-10-30）

> **性质**：立项（计数 69 → 70；P1；S1 修复随下一源码批，载体进体随下次重建）。**来源**：LIF 工作区会话 `224b39bd`（10-28 至 10-30）末轮诊断的只读取证＋用户令「这个上下文问题要立项，属于 bug 了已经，要排期修复」「直接按照项目习惯立项、排期、并同步本工作区的 index/todo/backlog」。**勘定结论＝对账能力缺失（三件机械事实），非已证实的单点越界**——诊断过程中曾口头归因「810K 单请求压穿量尺」，经账面复核**勘误**（见 §4）。

## §1 证据链（三件机械事实，全部只读取证）

### 1.1 模型面阶梯量尺＝chars/2 口径，已知有偏且与在库 tokenizer 单源脱接

- 模型面阶梯的全部量尺——块内估算（`model_face.rs` §140「块内轮次的估算体量（chars/2 口径）」）、主滑块 x／分块 y（§248「估算口径 chars/2」）、静态开销（§260「chars/2 同尺」）——均为 chars/2。
- chars/2 的偏差**方向随内容构成翻转**：ASCII（代码／JSON／工具输出）实际约 4 chars/token ⇒ chars/2 **偏高约 2×**；CJK（本会话讨论主体）实际约 1–1.7 chars/token ⇒ chars/2 **偏低约 1.3–2×**。仓内自有测试注释已实证此事（`tokenizer.rs` §96："The old chars/2 estimator would say 200 for 401 CJK chars"）。
- **tokenizer 单源已在库**（`DeepSeekTokenizer`，`tokenizer.rs` §49；377 挂账确认已换 V4.1-Flash 精确资产）且同族面已接入：state 分区硬预算（`state.rs` §305，376 批「provider tokenizer 硬预算」）、recenter（`recenter.rs` §137）。**模型面阶梯与 context_map（373 批账面「≤1K token（chars/2）」）未接**。
- 后果：阶梯触发点（192/224/256/288K 软提醒 → 320K 硬提醒 → 500K 硬截断，ADR-0010 §14.26 v8 口径）的**有效位置随会话语种构成漂移**，同尺不可比、跨会话不可预期。

### 1.2 逐调用 provider 用量未落 journal ⇒ 量尺对齐不可事后审计

- journal 用量面仅 run 级聚合：`run_usage_metered`（本次 `RUN-224b39bd-40`＝`model_calls:3`、`prompt_tokens:810155`、cache_hit 369,536／miss 440,619）；逐请求 `model_output` 事件不带 usage。
- 因此「某一轮模型面估算 vs provider 实测」的偏差**在账面上不可复算**——本次诊断正卡在此处：能拿到合计、拿不到分布，量尺是否漂移、漂多少，只能推断不能举证。v8 设计取舍（量尺＝模型面估算而非 provider 实测，§3.1）本身成立，但**取舍的前提是两条尺定期对得上账**，当前无任何对账点。

### 1.3 run_preflight 未盖载体版本/构建哈希 ⇒ 跨版本行为归因不可为

- `run_preflight` 载荷仅 run_id/created_at/schema_version；`.gsa` 全域（sessions/runs/process_trees/orientation）无二进制版本串。同日诊断「1017 版本是否回归」时**无法判定会话 41 个 run 各自运行的载体版本**（10-30 晚恰跨 0.9.14-4 → 0.9.14-5 换装），只能靠行为形状旁证。
- 该缺口与 0by 线「版本偏斜横幅」不同域：那是 web 桥对 child 的运行时探测，这是 journal 侧的离线取证锚。

## §2 缺陷表述（立项范围）

| # | 缺陷 | 类型 | 修复方向 |
|---|---|---|---|
| 1 | 阶梯/分块/context_map 量尺 chars/2 有偏，tokenizer 单源未接 | 精度缺陷 | 量尺换 `DeepSeekTokenizer` 单源；过 perf 关（逐 loop-top 全量编码开销，必要时缓存增量） |
| 2 | 逐调用 usage 未落 journal | 可观测缺口 | 逐请求 usage 事件（事件族 registry/fixture/法官/Python 镜像全链随批），或最低限度在既有事件补 per-call 字段 |
| 3 | run_preflight 无载体版本/构建哈希 | 可观测缺口 | preflight 盖 `carrier_version`＋`binary_sha256`（构建期注入单源，沿载体清单口径） |

**边界**：本项不断言「阶梯已实际失准到 X%」——单次偏差量在 §1.2 缺口修复前不可举证；修复批 S1 先落对账面，读数出来再定是否需要量尺参数级校准（避免按推断调尺）。

## §3 批序与排期

- **S1（随下一源码批）**：①③ 优先（preflight 版本章最薄、收益最直接）；② 量尺换 tokenizer 单源＋perf 读数；对账读数面随批。
- **S2（随下次载体重建/换装批）**：进体；0.9.14-5 在役容忍至下次重建（沿 385 停栈口径）。
- **S3（真机核证）**：CJK 主会话跑一轮，账面可复算「逐调用估算 vs 实测偏差」；阶梯触发与实测对齐读数入档。

## §4 验收判据草案

1. 任一 run 结束后，从 journal 即可复算每次调用的 prompt 估算 vs provider 实测偏差（无需旁路工具）。
2. run_preflight 携带载体版本＋构建哈希，且与载体清单一致。
3. CJK 主会话中模型面阶梯量尺与 provider 实测偏差收敛到设定阈值（阈值随 S1 读数定）；ASCII 面不回退。
4. 0dt/0du/0dv 判据面不受影响（本项零行为语义变更，量尺替换除外——S1 读数面核证）。

## §5 勘误注记

- 诊断会话中曾向用户口头表述「provider 实测 810K prompt tokens（单轮）」「阶梯触发线被推后 1.3–1.6×，810K 现实账面未越线」。账面复核：810,155 为 `run_usage_metered` **run 级合计（model_calls=3）**，单次约 270K，强断言不成立，随本档撤回。成立的部分＝§1 全部三条（chars/2 有偏、逐调用用量未落账、preflight 无版本章），本项按此口径立项。

## 入口

- BACKLOG `0dw`（开放项计数 70）／TODO `0dw` 条目／索引头 v4.348。
- 关联：0bz（`GAP-CONTEXT-FACE-TRANSIENT-FORK`，脸面瞬态分叉——本项补其对账缺面）／0ah（滑块 v8，量尺取舍出处 §3.1）／376 批（state tokenizer 硬预算先例）／377 挂账（tokenizer 资产与绑定边界）／0by（版本偏斜横幅——运行时面，与本项 journal 面互补）。
