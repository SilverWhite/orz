# 任务 D S2d 批 2 复审处理审计（2026-09-06）

> **性质**：批 2（probe_accuracy 收窄至当前可见工具）实施后三路全面复审
> 的处置登记——语义符合性 + 设计合理性 + 生产面核实 / 实现质量 / 治理
> 登记与数字核验，三路并行只读复审。对象：orz `00b9a440` + 父仓库
> `9edf630`。**用户补裁决（2026-09-06）**：P1 处置采用收窄方案——
> `tool_availability_probe` 族 exact-partition 子句收窄为「complete ∪
> incomplete ⊆ WORK_TOOLS 且两集互斥」；P2/P3 按建议修复/登记。
> 设计转录：ADR-0010 §14.59（§14.58 两处内联修正 + §3.5 条 7 内联加注）。
> 关联：[批 2 实施审计](TASK_D_S2D_BATCH2_IMPL_AUDIT_2026-09-06.md) /
> [裁决登记](TASK_D_S2D_FLIP_ADJUDICATION_2026-09-06.md)。

## 1. 复审发现与处置总表

| # | 级别 | 发现 | 处置 |
|---|---|---|---|
| R1 | **P1** | 判官 `tool_availability_probe` 族（形状不变量）与收窄后生产刊必然冲突：两侧规则要求 complete ∪ incomplete **精确等于** 23 工具全集（Python `run_event_journal_validation.py:2114-2120` / Rust `families_s2c.rs:2152-2158` 为 parity 孪生），收窄后生产事件只含声明面工具——首个真实新刊被校验时必报错。历史 fixture 均为全量分区故测试全绿，破坏被延迟。裁决二「判官两侧零改动对真实刊零误报」的前提仅对 probe_accuracy 族成立，对该族不成立 | **用户补裁决 + 采纳修复**：两侧同步收窄 exact-partition 子句为「分区 ⊆ WORK_TOOLS（越界报错、缺工具合法）」，保留重叠检出与判断词检出两子规则；ADR-0010 §14.59 转录。端到端实证：收窄载荷两侧族均 0 错误、越界（ghost_tool）仍报、重叠仍报、18 历史 fixture 0 错误、判官测试 258 passed |
| R2 | P2 | 收窄快照的第四/第五消费点未披露：console 注册板块探针过滤（`console.rs:884-909`，`workspace.run_tests` 动作在 runner 在场会话中从黑板 actions 板块消失）与环境实体工具清单（`console_exec.rs:254-267`）；实施审计「收窄的可见面效果为零」表述不实。方向与 R1「看得见摸不着」禁令同向、板块为黑板数据不入 header、不影响 probe↔header 不变量 | **用户确认预期收敛 + 登记修正**：ADR §14.58 项 2 消费点枚举修正为五处并登记板块/实体面效应；实施审计 §1.2 加勘误指针 |
| R3 | P3 | 两装配点声明面谓词为复制代码（`controller.rs` / `agent_loop.rs`），一致性仅由 e2e 间接钉住，有漂移风险 | **采纳**：抽共享 helper `tool_probe::narrow_to_declared_face(snapshot, base_tool_defs)`（registry 在场 ∧ 非 R1 封存单一来源），两装配点统一改用；新增 helper 契约单测 |
| R4 | P3 | `MinimalProbeMap::from_snapshot` doc「every work tool carries exactly one status」与 `WORK_TOOLS` doc 随收窄语义过期 | **采纳**：两处文档补收窄注记（map 只含声明面工具；journal 记账为全集子集、raw 探针仍全集分区） |
| R5 | P3 | §3.5 条 7 本体缺 v1.58 收窄指向（先例：v1.19 于条 6 内联加注） | **采纳**：条 7 补内联加注（v1.58 记账收窄 + §14.59 形状子句收窄指向） |
| R6 | P3 | plan-gate 轮记账面大于暴露面（暴露面固定 plan_write/blackboard_read 且不过探针过滤，轮内声明工具翻转可刊发无 header 变化的事件）——生产 plan_first 休眠、形态早于批 2 | **登记**：随 plan_first 复活裁决一并议定（归 §14.58 项 4 边界视野） |
| R7 | P3 | 调用即探针回写守卫（`mark_incomplete`）不查声明面：封存工具失败回写可注入孤儿键 → 下轮一次冗余事件后重播种自愈；生产不可达（封存工具主面不可见；唯一可达路径需宿主 registry 声明封存工作工具且调用抵达） | **登记**：已知边缘、生产不可达、自愈不违判官；后续如需可把回写守卫收窄到声明面 |
| R8 | P3 | host_exec run_tests 测试改写后区分度削弱（原「探针而非 registry 是声明源」意图在 e2e 级削弱） | **登记**：残余覆盖仍在（tool_probe 单测钉住 runner 缺席→incomplete 的 raw 探针矩阵；flip e2e 钉住 registry 声明但探针 incomplete 的工具被投影剔除） |
| R9 | P3 | 覆盖缺口：① registry 缺席 + 全链组合无 e2e；② grill 车道收窄路径无专测；③ 回写交互（R7）无回归测试 | **采纳①**（helper 契约单测覆盖 registry 缺席组合）；**登记②③**（grill 与主车道共用 run_agent_loop 代码路径；③随 R7） |
| R10 | P3 | orz `00b9a440` 提交 message 测试数简写口径（204+18 中 18 为期刊数） | **登记不改**：精确套件口径 204+9+9+1 以实施审计为准 |
| R11 | P3 | 实施审计「clippy 仅存量告警」未在治理路复跑 | **登记**：本次复审处理批补跑确认（仅存量 lif/acaf） |

## 2. 复审确认成立的关键结论（无发现项摘要）

- 转录保真：§14.58 与裁决登记 §2 五项逐项对应，无漏项漂移。
- 判官零改动（probe_accuracy）：提交级实证两侧字面未动、schema 未动。
- 同源性：两装配点谓词语义逐点等价、判定输入（tool_defs）全程同一；
  「记账集 = 投影面工作工具部分」在 run-start/常规轮/orientation 软门轮/
  console 询问轮/grill/检索车道全路径成立。
- 时序：flip（loop-top）→ header change（请求前）→ model_output 顺序
  未被收窄破坏；幻影违规主通道确已封堵；无反向漏洞。
- R1_SEALED_MAIN_TOOLS 单一来源，封存工具重新开放自动回归记账集；
  模式投影后基表作判定输入无边界后果（被剔除族均在探针矩阵外）。
- 治理：登记四方一致、hash/数字全部实测吻合（728 lib、204、250×30=
  7500、232×30=6960、manifest 1438、门禁 Exit 0）；索引豁免成立；提交
  无夹带。

## 3. 复审处理批验证

| 项 | 结果 |
|---|---|
| **P1 修复端到端**（Python 判官实测）：收窄载荷 `tool_availability_probe` 族 0 错误 + `probe_accuracy` 族 0 错误；越界（ghost_tool）报 `probe partition must be a subset of the work tools; extra=[...]`；重叠仍报；18 历史 fixture 0 错误；判官测试 **258 passed** | ✅ |
| spec 表 **233 场景 × 30 族 = 6990 裁决格**（`probe_partition_gap` 改名 `probe_partition_extra` 语义随新子句 + 新增 `probe_partition_narrowed_legal` 收窄分区正例；计数守卫 232→233） | ✅ 1 passed |
| Rust↔Python 对拍 **251 语料（233 合成 + 18 fixture）× 30 族 = 7530 格 0 差** | ✅ 1 passed |
| orz-loop **729 lib 全绿**（+1 helper 契约单测）；orz-assurance 204 + 9 + 9 + 1 全绿 | ✅ |
| `cargo check --workspace` 零警告；`cargo fmt` 净；clippy 仅存量（lif/acaf） | ✅ |

## 4. 遗留

- R6（plan-gate 记账面）/ R7（回写守卫）/ R9②③ 随相应复活/接线裁决
  一并议定，不占当前工作集。
- S2d 正题（30 族全量对拍矩阵落盘 + registry 翻转准备）→ S3（门禁改接
  Rust 法官）→ S4（Python 双法官退役收口）依序推进；P1 收窄后新刊与
  历史刊在两侧判官下均可校验，S3 改接的前置风险已清除。
