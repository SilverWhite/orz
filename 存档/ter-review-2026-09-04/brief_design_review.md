# 工单：TER 设计层审查（design reasonableness）

你是 TER（Tool Execution Layer Reform）设计层审查代理。环境：Windows PowerShell，cwd=D:\CLI。只读审查设计文档并输出书面结论。

## 纪律
- 【禁止】修改/删除/创建任何仓库文件（唯一例外：报告写入 `D:\CLI\.ter_review_2026-09-04\00_design_review.md`）；禁止 git 写操作（add/commit/push/reset/checkout/clean/restore）；禁止 cargo、禁止联网型命令、禁止触碰工作树未提交内容（并行工作）。
- 证据给出文件:行号或章节。无法验证写 NOT-VERIFIED。

## 审查目标
对 TER M0 设计门与 M1/M2 设计规格做“设计合理性”全面审查：内部一致性、与 ADR-0010 及仓库治理的关系、可验收性/可测试性、单一事实来源声明、边界与逃生阀、已知裁决是否正确固化。

## 必读材料（绝对路径）
- `D:\CLI\docs\TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md`（主设计稿）
- `D:\CLI\TODO2.md`（勾选树，含每步“验收”行）
- `D:\CLI\docs\BACKLOG2.md`（TER-0 开放项）
- `D:\CLI\adr\ADR-0010-fusion-runtime-and-agent-architecture.md`：文件很大，用 Select-String 定位 “14.53” 相关段落后按行号区间读；重点核对 §14.53 候选项 3 条与设计稿、实施状态的关系（候选未转正问题）。
- M0 审计：`D:\CLI\docs\audits\TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md`、`TER_T0_2_SCHEMA_VERIFIER_FIXTURES_CONTRACT_2026-09-03.md`、`TER_T0_3_ADR_CANDIDATE_REGISTRATION_2026-09-03.md`、`TER_T0_4_M0_RELEASE_2026-09-03.md`
- 复审裁决：`D:\CLI\docs\audits\TER_T1_2_REVIEW_HANDLING_2026-09-04.md`
- M2 背景：`D:\CLI\docs\WINDOWS_HIGH_NIST_MAX_FRICTION_DESIGN_2026-09-01.md`、`D:\CLI\_windows_high_nist\README.md`、`D:\CLI\docs\audits\TER_T2_2_WF12_LOCAL_TRANSPARENT_LAYER_2026-09-04.md`（其中 2026-09-03 “no-appcontainer+allowlist 生产墙”裁决是否有设计层权威登记）
- 治理规则参考：`D:\CLI\CLI_PROJECT_INDEX.md`（AUTH-TOOL-EXECUTION-REFORM 条目、§1 权威顺序、§0.2 状态词）

## 审查维度
1. 权威一致性：设计稿 vs ADR §14.53（候选）vs BACKLOG2 vs TODO2 的矛盾/状态错位；设计稿头部状态（如“未实施”）与实施进展（M1 全闭、M2 部分闭）是否冲突；ADR §4.5 守卫表等旧条款（300s kill、120 轮等）与 TER 新语义是否冲突且无取代登记。
2. 单一事实来源：参数链逐项核对（auto_background_on_timeout 默认 true、hide_background_input 默认 true、foreground_block_budget_ms=180_000、timeout 分层语义、后台化后 10h 绝对兜底、idle 300s/0=禁用、ORZ_MAX_WALLCLOCK、ORZ_F6_PUSH 默认 off 与 <600/300/120 档、≤3~4 次/run、read_file 64K、output retrieval 三字段、env 快照 ≤5s、BashParams/schema/backend 三处收敛）在设计稿中是否各自有唯一权威出处。
3. 语义完整性与可验收性：每个 T0.x/T1.x/T2.x 验收行是否可机械判定（数值/事件/测试名）；T2.3/T2.4 验收是否足以支撑“实机证据”；设计稿对 Job/LOW IL/AppContainer/no-appcontainer 论述与 M2 口径是否一致。
4. 边界/逃生阀/回滚：显式 false、评测墙钟例外、0=禁用、0=unlimited、fail-closed 默认与可关开关；有无实施自行裁决而设计未定的灰色地带（如 10h 绝对兜底是否违反“无自身硬超时”表述；后台任务关管道后 idle-kill 不适用等）。
5. 范围控制：“首轮明确不做”清单一致性；T2.2 是否从“本地透明层”扩张到 enforcement-probe/runner 改造。
6. 后续开放点：T2.4/T3.x 实施前必须澄清的设计问题。

## 输出要求
最终回答中文：总评 → 发现清单（ID D-xx、P0/P1/P2/P3、类别：权威一致性/参数链/可验收性/边界/范围控制/开放点、位置=文件:行或章节、问题、短原文证据、建议修法）→ 设计侧核查表（参数/验收项 + 状态：一致/冲突/未定义/待澄清）→ 治理问题单列小节。
同时把完整报告写入 `D:\CLI\.ter_review_2026-09-04\00_design_review.md`（只限该目录）。
