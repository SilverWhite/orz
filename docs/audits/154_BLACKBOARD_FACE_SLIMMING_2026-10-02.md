# 0cl 黑板模型面瘦身——blackboard_read 描述＋guide 说明书简明化（154 批，2026-10-02）

> 类型：单独立项＋落码批（S1/S2 同批达成；S3 进体、S4 真机核证待续）。
> 上游：153 批档 §4 测量＋用户令「请先进行黑板瘦身吧，说明书也要简明扼要，可以做成框架内部名词解释和组件关系交代」。
> 权威：BACKLOG `0cl`（58 → 59）；与 0ck 同族（模型面可见性线）、单独立项单独批。
> 排序含义：与 0ck S3 同代窗口重建进体后，0ci／0cj 新一轮验证自带瘦身后的模型面。

---

## 1. 立项（0cl）

- **依据（153 批 §4 测量实锤）**：blackboard_read 工具描述 5,593 字符，含约 10＋ 处批次/审计引用（`0am 改造四项③`、`0be 四项`、`0cf`、`P2-10 F2 §3.3`、`0p S1`、`FR-5`、`B2 折叠视图(P2-13)`、`0bg S2`、`TER T1.6/T1.12`…）——对模型使用工具零价值；参数面四条同文风。
- **用户定形态**：说明书（`section=guide`）简明扼要，做成**框架内部名词解释＋组件关系交代**。

## 2. S1 落码（154 批达成）

1. **主描述瘦身**：5,593 → **2,492 字符**（−55%；批次引用 **10＋ → 0**）——每 section 一句「何时读」（plan/notes/exec/edits/tool_actions/actions/session/entities/deps/processes/env/internal_ret/external_ret/temporal/rli/journal/guide）＋折叠视图与展开参数要点＋`[黑板增量]` 未读徽章语义。
2. **参数描述去考古**：section/selector/k/name（原 `P2-10 F2 §3.3` 前缀四条）＋receipt_id/domain/round_from/round_to/failures_only/search 六条——保留全部行为语义（互斥组合、上限、错误面），删批次号与日期。
3. **模型面与派发器一致性修复（顺带两处，如实记）**：① section 枚举补 **`journal`/`notes`**——派发器（tool_run.rs）本就支持、guide 旧正文也在教模型用 `section=journal`，唯 enum 未声明＝自相矛盾；② 补声明 **`anchor`** 参数——journal 定位符点读的入口参数，派发器校验在用、schema 未声明。二者属既支持面的模型面交代，非工具面扩张。
4. **guide 正文重写（用户定形态）**：**621 字符**（含头行与结束自述语法行整响应远 ≤1 KiB）——`【名词】`黑板/journal/轮 r·块 b·seq s/定位符 `r<轮>·b<块>·s<seq>[#sha8]`/压缩与回放/域五值；`【组件关系】`模型提议工具→机械层校验与门禁→执行→结果回流黑板与 journal→两阶段交付＋机械审计；时间与压力由机械层替模型记账（temporal/rli 按需查）；检索由子代理执行（指针回读 internal_ret/external_ret）；写计划/笔记用黑板写工具（单条 ≤8K）。
5. **测试钉同步**（blackboard.rs）：0cf 句原文断言 → `section=guide`＋`名词解释与组件关系` 双断言（guide 枚举断言不动）。
6. **复核补笔（用户过瘦检查后令）**：deps 分区补回边界提示半句 `command/retrieval side effects are NOT graphed`（153 批对账 D-3——静默缺失边界，防模型误以为命令副作用在图内）；补后 fmt／lib 复跑同绿（847/0）。

## 3. S2 验证读数

| 面 | 读数 |
|---|---|
| 触碰面 fmt | controller.rs＋blackboard.rs `rustfmt --edition 2024 --check` 双干净 |
| orz-loop lib | **847 过／0 挂／3 忽略**（全绿；过程中三处自纠＝guide 闭引号弯引号、断言格式收行、描述补字面 `section=guide`——均为落码当轮修复） |
| 批次引用 | 新描述＋guide 内 **0**（原 10＋） |
| 行为语义 | 零删改——枚举/参数仅增声明（journal/notes/anchor 本就受派发器支持）；全部互斥组合与上限文本保留 |

## 4. 边界与未决

1. **S3 重建进体未执行**——与 0ck S3 同代窗口裁决；进体判据＝新描述串＋enum（journal/notes）＋anchor 字节进体核证。
2. 其余工具描述不动（read_file 2,172 字符属 orz-tools grok_build 系，另案留裁决）。
3. 未提交、未推送（orz 子树脏＝controller.rs＋blackboard.rs 两处，既知开发态）。
4. 工具面冻结纪律＝通用修正（一致性修复＋文案精简），非跑分特化。

## 5. 账本改动清单与门禁

- 新增：本档。
- BACKLOG：指针行（154 批）＋计数行（58 → 59）＋P1 总览行（＋0cl）＋P1 锚点行（＋0cl）＋新增 `0cl` 节。
- TODO：计数行（58 → 59）＋P1 路由行（＋0cl）＋新增 `P1-0cl` 勾选节。
- CLI_PROJECT_INDEX：头行 v4.122 → v4.123＋§8 pending 桶＋`BLACKBOARD-FACE-SLIMMING`。
- BACKLOG 第二卷：§1.106（本批）。
- orz 子树：controller.rs（描述/参数/枚举/anchor/guide）＋blackboard.rs（测试钉）两处。
- 门禁：`check_repository.py` 复跑结果见主会话报告（预期唯一红＝orz 子树脏，既知开发态）。
