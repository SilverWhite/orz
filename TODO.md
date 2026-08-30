# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。

## 未闭合扫描快照（2026-08-16；P0-C 转正式组件 + PLAN-FIRST 阶段 A 闭合后更新）

> 来源：2026-08-15 全量扫描（CLI_PROJECT_INDEX → BACKLOG → 本文件勾选状态；
> P0-C S3 前置（P1-2 结构化策略拒绝）闭合后更新；2026-08-16 S3 全面审查
> 收口；2026-08-16 S4 实施闭合，未闭合总数 35 → 34；2026-08-16 用户裁决
> P0-C 转正式组件（决策门闭合）+ PLAN-FIRST 阶段 A 闭合，未闭合总数
> 34 → 32；2026-08-16 PLAN-FIRST 阶段 B 实施闭合，未闭合总数
> 32 → 31；2026-08-16 PLAN-FIRST 阶段 C 实施闭合，未闭合总数
> 31 → 30；2026-08-16 ACAF fail-closed 生产启用翻转闭合，未闭合总数
> 30 → 29；2026-08-16 会话监测度量重定（chars/2 校准项废止，29 → 28）；
> 2026-08-16 DeepSeek 主/子代理 transport/retry/thinking 同构复核闭合，
> 未闭合总数 28 → 27；2026-08-17 评测冒烟暴露问题登记（P0 +2：ACAF 评测链路 /
> console 工具名点号；P1 +2：plan_write 提示词、actions 校验），未闭合总数
> 27 → 31；2026-08-17 GAP-CONSOLE-TOOLNAME-PATTERN 闭合（改名下划线 +
> verifier/schema/文档同步 + Linux 重建 + 冒烟重跑验证，未闭合 31 → 30；
> 2026-08-17 ACAF 跑分决策 + 下一步实施项对齐（两项 P1 观察升 P0-E；冒烟重跑定位
> 新增计划视图步骤 ID 渲染项，未闭合 30 → 31；2026-08-17 grep 侦查纪律项登记
> （用户确认一并处理，未闭合 31 → 32）；2026-08-17 订单发放前拒绝入事件面项登记
> （用户指示，未闭合 32 → 33）；2026-08-17 GAP-ACAF-HARNESS-PASSTHROUGH 闭合
> （文件型密钥库 + 适配器容器供应 + Linux 重建 + 冒烟通过，未闭合 33 → 32）；
> 2026-08-17 计划视图渲染步骤 ID 闭合（渲染 step.id + 状态行 + 工具描述提示
> + 测试，未闭合 32 → 31）；2026-08-17 订单发放前拒绝入事件面闭合
> （v0.2 `console_order_rejected`：Schema/verifier/fixtures 先行 + 生产者
> 三处 pre_issue / 发放期 issue 路径发事件 + TUI 投影 + 测试断言，未闭合
> 31 → 30）。
> 2026-08-17 P0-E 第四/六项复核更正（用户复核 + 证据回查；纯文档/待办更正，
> 计数不变，未闭合仍 30 项）——第四项 plan_write 放弃特化示例方向，收窄为校验
> 错误消息形状明确；第六项 grep 归因更正为系统性工具层空结果（前两次 grep 实际
> 未执行、其余 7 次全无匹配含实测存在的字符串），实施前置=容器内 grep 冒烟定位
> 根因 + 结果补搜索范围报告。
> 2026-08-17 grep 搜索范围方向再修正（用户复核定案；纯文档/待办更正，计数不变，
> 未闭合仍 30 项）——撤回「补文本范围报告」方向：根因定位=finalize_grep 合并
> 「搜索 0 文件」与「真无匹配」（ORZ 总传显式路径、rg 不打印 "No files were
> searched" 警告、该分支死代码）；定案=grep 搜索信封（resolved root /
> files_searched / files_skipped / match_count / truncated）+ 结局三型分型
> （searched=0 显式报范围空，不叫 "No matches found"）+ 范围语义显式化（与只读
> 工具可见集对齐或 --no-ignore/--hidden 开关）+ 读/搜/列三族统一契约
> （ADR-0010 §14.23 v1.23 / FUS-TOOL-SCOPE-CONTRACT / 操作台设计 §12）。
> 2026-08-17 grep 搜索范围实施闭合（容器冒烟复现根因=glibc 动态 rg 与 bookworm
> GLIBC 不匹配 + finalize_grep 吞 stderr；工具契约三型分型 + --files 探针 +
> hidden/no_ignore 开关 + build.rs 静态守卫 + 构建脚本静态 rg，测试通过；
> Linux 重建后容器冒烟回归待执行）——未闭合 30 → 29，P0-E 3 → 2 项。
> 2026-08-17 plan_write 校验消息形状 + actions 形状探针闭合（plan 错误消息带
> 形状说明与 got 类型；actions 实证审计确认 14 种宽松形状全被拒、原观察不成立、
> 探针测试锁定）——未闭合 29 → 27，P0-E 2 → 0 项（P0-E 全部闭合）。
> 2026-08-17 FUS-TOOL-SCOPE-CONTRACT 后续两项补记进计数（用户指示）——
> grep 面实施审计「边界与后续项」两条正式入账：list_dir ignored/truncated
> 计数（读/搜/列三族统一）、grep 命中路径 files_searched 留空（后续 --json
> 面或 stats 位置收敛后再定）——未闭合 27 → 29，P0-E 0 → 2 项（后续项）。
> 2026-08-17 ORZ-LARGE-FILE-READ-CONTRACT 实施闭合（本窗口）——GrokBuild
> read_file 文本路径有界返回（粗门默认 16KB、可配 8–32KB，env/TOML 口子；
> 读取句柄信封 + 有界预览 ≤4KB + offset 续读；SKILL.md/`skills` 豁免；
> FileTooLarge 文本路径被取代为防御兜底）；提示词/工具描述/console 注册表
> 同步；测试 orz-tools read_file 199 / orz-loop 440 / orz-host e2e 2；
> orz 172b14e。该组 4 项此前未计入 P1 分组计数（口径遗漏，若计则 P1 13 /
> 总数 33）；本次全部闭合，P1 回 9、未闭合总数维持 29（补计与闭合相抵）。
> 2026-08-18 FUS-TOOL-SCOPE-CONTRACT 后续两项闭合（本窗口）——list_dir
> 目录信封（ListDirContent 增 listed/ignored/truncated 计数 + 卡片
> (scope: ...) 脚注）+ grep files_searched 全结局探针收敛（v1 `rg --files`
> 探针扩展为每次完成搜索都运行，命中摘要内嵌 searched N files）——
> 未闭合 29 → 27，P0 评测冒烟暴露 2 → 0 项（P0-E 后续项全部闭合）。
> 2026-08-18 FUS-BENCHMARK-FULL-EXEC 入账（本窗口，用户裁决实施、暂不测试）
> ——Benchmark 完全体执行面三层使能 + CLI 旗标 + TB 适配器透传实施完成待验证
> （实施前登记见下；验证项未勾选）——未闭合 27 → 28，P0 0 → 1 项。
> 2026-08-18 LEDGER-FOLD-EXTERNAL-FILE 入账（本窗口，用户指示优先实施——
> 「得先处理命中率问题，不然成本太高了」；设计轮先定案不动计数，实施轮
> 入账 1 项）——折叠历史外挂文件：固定指针消息 + 最近 1 轮原文视图、
> 折叠行外挂追加（序号跨压缩连续）；S1/S2 已闭合、S3/S4 待验证——
> 未闭合 28 → 29，P0 1 → 2 项（P0-F 1 + LEDGER-FOLD-EXTERNAL-FILE 1）。
> 本快照只做计数与分组召回，明细以下方各分组勾选清单为唯一入口，不新增独立条目；
> 后续扫描更新时同步替换本快照日期与计数。

- 未闭合总数：**32 项**（2026-08-19 0c 验证闭环 29 → 28；2026-08-20
  0d 验证闭环 29 → 28；2026-08-20 OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD
  S1 实施放行入账 28 → 29，**S3/S4 验证闭环 29 → 28**；2026-08-20
  THINKING-DEFAULT-HIGH-LADDER S1 实施放行入账 28 → 29，**S3/S4 换题
  复验（make-doom-for-mips）验证闭环 29 → 28**；2026-08-21
  CONTEXT-SCAFFOLDING-PULL-REDESIGN（0e）S1-S4 验证闭环 28 → 27；
  2026-08-21 FUS-READ-ANCHOR-WRITE-GUARD（0f）S1 代码实施入账 27 → 28
  （S1/S2 已闭合；S3 重建登记 2026-08-23 随 orz 05231f7 构建轮；**S4 复验
  闭环 2026-08-23，计数 28 → 27**）；2026-08-23 REPETITION-DETECTOR-ROLLING-HASH
  （P0-0d 后续 6）滚动哈希再校准 **S3/S4 验证闭环（设计口径 29 → 28；
  S1 实施放行入账 28 → 29 已登记于设计 §4.9，快照补记）**）
  2026-08-23 AGENT-DELIVERY-FLOW（P0-0d 后续 7）S1 代码 + S2 测试闭合
  实施入账 28 → 29，**S4 复验闭环 2026-08-23，计数 29 → 28（见下）**；
  2026-08-23 NGRAM-GUARD-CALIBRATION
  （P0-0d 后续 8）S1 代码实施入账 29 → 30（S2-S4 待续，见下）；
  2026-08-23 NGRAM S4 复验闭环 30 → 29（补记）；2026-08-24
  MECHANICAL-AUDIT-LAYER（0g）S1 实施放行入账 29 → 30（补记）；
  2026-08-25 RETRIEVAL-SUBAGENT-WIRING（0h）S1 实施放行入账 30 → 31
  （补记）；2026-08-25 0g S4 复验闭环 31 → 30；2026-08-25 0h S4
  复验闭环 30 → 29；2026-08-25 冒烟对拍登记入账 29 → 31
  （GAP-EVENT-SCHEMA-DRIFT + GAP-REPETITION-DETECTOR-DNA-
  FALSE-POSITIVE，见 P0-0i）；2026-08-26 GAP-EVENT-SCHEMA-DRIFT
  事件面三类 Schema 漂移修复完成并复验，未闭合 31 → 30；
  2026-08-26 SEQUENCE CONTENT GATE（P0-0i）S4 复验闭环，未闭合 30 → 29）；
  2026-08-28 THIN-HARNESS-REDESIGN-V2（0j）设计定稿登记（设计轮不动计数，
  仍 29）→ **R1 S1 代码 + S2 测试实施放行入账 29 → 30**、**R2 S1 代码 +
  S2 测试实施放行入账 30 → 31**（S3 重建 / S4 复验待续，见下）
  2026-08-30 RETRIEVAL-ORCHESTRATION-MECHANICAL（0k）检索问题最终评判
  定稿登记（用户确认无异议；设计轮不动计数，仍 31，见下）→ **第一批
  S1 代码 + S2 测试实施放行入账 31 → 32**（第 1–4 项完成 2026-08-30；
  S3 重建完成 2026-08-30——三件套 12:06 HKT（orz 106,411,880 B / signer
  1,388,496 B / provision 1,206,464 B）+ bookworm 冒烟符合预期（provision
  usage / signer manifest 缺失 / orz TTY io error）+ ldd 静态链接 + 警告面
  14 项与基线持平，对应源码 orz b604773 + 父 b0b903e 均已推送；容器内
  chromium 注入完成 2026-08-30（0k-5 前置，tb_agents/orz.py eval_browser
  开关，端到端实测 bookworm=apt chromium / ubuntu:24.04=官方快照，见下）；
  容器内 chromium 注入完成 2026-08-30（0k-5 前置，tb_agents/orz.py
  eval_browser 开关，端到端实测 bookworm=apt chromium / ubuntu:24.04=
  官方快照，见下）；Google 门禁观察实验 + 实机验证完成 2026-08-30
  （2/5 解出、5 题零检索调用、通道验证通过，见下））
  - P0-C：0 项（PLAN-FIRST 阶段 A/B/C 全部闭合，2026-08-16）
  - P0 冒烟对拍暴露：0 项（GAP-REPETITION-DETECTOR-DNA-FALSE-POSITIVE
    复读检测 DNA 误杀已闭合 2026-08-26；GAP-EVENT-SCHEMA-DRIFT 事件面
    三类 Schema 漂移已闭合 2026-08-26；2026-08-25 登记，正式 89 题提交
    前处理，见 P0-0i）
  - P0 评测冒烟暴露：0 项（P0-E 主项 7 项 + FUS-TOOL-SCOPE-CONTRACT 后续 2 项全部闭合 2026-08-18，见 P0-E grep 项后续①/②）
  - P0 Benchmark 完全体：1 项（FUS-BENCHMARK-FULL-EXEC 实施完成待验证，见 P0-F；验证闭环后回 26）
  - P0 折叠历史外挂：0 项（LEDGER-FOLD-EXTERNAL-FILE，S3/S4 验证闭环 2026-08-19，见 P0-0c）
  - P0 输出退化防护：0 项（OUTPUT-DEGENERATION-GUARD，S3/S4 验证闭环 2026-08-20，见 P0-0d）
  - P0 输出预算恢复与空流止损：0 项（OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD，S1-S4 全部闭合 2026-08-20，见 P0-0d 后续）
  - P0 默认 high + 三级降级梯：0 项（THINKING-DEFAULT-HIGH-LADDER，S3/S4 换题复验闭环 2026-08-20，见 P0-0d 后续 2）
  - P0 上下文结构块 PUSH→PULL：0 项（CONTEXT-SCAFFOLDING-PULL-REDESIGN，S1-S4 验证闭环 2026-08-21，见 P0-0e）
  - P1 可并行审计/证据：9 项（组件登记 1、Windows 证据 3、DeepSeek 1、会话上下文监测 4）
  - P2 生产化决策门：5 项（Slice 3、Slice 4、ACAF 可选工程项、OPS 裁剪设计、OPS 生产接线裁决）
  - P3 收尾/清理：7 项（EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、observed-scope 枚举、V11-IMPL-003、V11-IMPL-007、orz-host flaky、DC 硬信号 4/6）
  - 条件触发/审计登记边界：6 项（不占当前优先级）
  - 成熟复用调研（2026-08-16，只读）：明确可复用 8 项、部分可参考 15 项、无可复用 11 项；逐项注记见各分组条目后。
- 备注：ACAF fail-closed 生产启用已闭合（2026-08-16，用户 2026-08-15 裁决
  放行；翻转执行 + 核查清单 ⑦⑨⑩⑪ 收口 + 审计/文档同步完成，见
  `docs/audits/GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md`）；
  其余用户裁决门为 OPS 生产接线与 P0-C 正式组件决策门（已闭合）。

## P0 — 当前工作集

### P0-E 评测冒烟暴露问题（最优先；2026-08-17；用户将在新窗口处理）

- [x] GAP-ACAF-HARNESS-PASSTHROUGH：TB2 适配器 ACAF 配置透传 / 容器内供应——正式跑分
  强制模式决策（容器内 manifest/keystore/signer，参考 `scripts/orz_acaf_run.ps1`）。
  **2026-08-17 用户裁决：跑分保持 ACAF 强制开启**（容器内供应，不接受影子模式）。
  **2026-08-17 闭合**：前置设计变更——Linux 下 DPAPI 不可用，用户裁决
  新增 `file-0600-installation` 文件型安装密钥库（ADR-0010 §14.21 项 4）；orz 子模块
  e8274e1（keystore 文件型后端 + storage 分派 + provision/signer 非 Windows 分支），
  orz-bin 全量通过、仓库门禁 valid；适配器 install 容器内落 manifest+keystore、run 设
  ORZ_ACAF_MANIFEST/KEYSTORE/BINARY/FAIL_CLOSED=1；`.env` 已移除 FAIL_CLOSED=0 覆盖。
  Linux musl 重建后容器冒烟通过：provision 0600 keystore + manifest 真哈希；signer
  stdio initialize_session 应答；`orz --real` 带签发器启动 3 次成功（启动期 fail-closed
  不变量通过、wallclock 正常收尾、退出码 0）。证据：
  `D:\tb-eval\jobs\2026-08-17__05-45-ACAF-SMOKE`。
  - 证据：`D:\tb-eval\jobs\2026-08-17__01-05-31`、`2026-08-17__01-07-59`；
    错误 `ACAF fail-closed is enabled but no signer client is configured`。
- [x] GAP-CONSOLE-TOOLNAME-PATTERN：console 面工具名点号→下划线改名
  （`blackboard.action_write`→`blackboard_action_write`、`console.step_done`→
  `console_step_done`、`console.return_to_console`→`console_return_to_console`），
  同步约 91 处 Rust、Python verifier 交叉校验、schema 注释、设计文档；重建 Linux
  二进制并重跑。**2026-08-17 闭合**：orz 子模块 11 文件 91 处 + Python
  verifier/schema/测试 + ADR/设计文档同步（orz commit 0304b23）；orz-loop 434 /
  orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 + real_flag 2 通过；clippy 无新增
  告警；manifest 重生成 1401 条目；Linux musl 重建后冒烟重跑
  （`D:\tb-eval\jobs\2026-08-17__03-48-57`）30m21s 跑满 1740s 预算、
  `run_invalidated{wallclock}` 正常收尾（对比旧运行 400 即死）。
  - 证据：`D:\tb-eval\jobs\2026-08-17__01-09-43`；journal events 1-18（plan-first
    全链后 run_failed）；错误 `Invalid 'tools[5].function.name' ... '^[a-zA-Z0-9_-]+$'`。
- [x] （P1 观察，2026-08-17 复核收窄）plan_write 校验消息/形状机械明确：首次模型把
  计划序列化为 JSON 字符串被拒（`missing_required_field: plan` → refill_requested），
  重填对象后通过——单点偶发（首个试跑第 1 次，重跑 4 次 plan_write 均为正确对象），
  **放弃特化示例方向**（过拟合、无回归价值）。实施=校验错误消息写明形状（plan 必须
  是含 plan_id / goal / steps[] 的对象，got string 时明示）+ 补回归测试（字符串计划
  → 机械拒绝 → 对象重填 → 错误消息含形状说明）。**2026-08-17 闭合**：
  `parse_and_validate_plan` 区分 plan 缺失（含形状说明）与非对象（`got string`
  明示类型）；回归测试 `plan_as_string_error_states_shape` /
  `missing_plan_error_states_shape`；orz 子模块 11540fa。
- [x] （P1 观察）`steps[].actions` 形状校验收紧：空字符串当前仍通过校验，动作形状
  校验偏宽。**2026-08-17 闭合=实证审计 + 探针测试锁定**：14 种宽松形状（空串/
  裸串/空对象/null/缺 with/缺 do/空 step_id/step_id 不匹配/with 错类型/actions
  非数组等）全部被机械拒绝，原观察不成立（与 grep 归因更正同类）；`with` 内容与
  `do` 注册表核对仍留订单发放时契约校验（plan 层只约束结构与长度）；
  测试 `action_shape_probe_rejects_loose_shapes` /
  `action_shape_probe_accepts_minimal_valid`。
- [x] （新发现，冒烟重跑定位）计划视图渲染步骤 ID：`blackboard_read section=plan` 当前
    仅渲染 `[status] goal (actions: N; evidence: M)`，不渲染 `step.id`；步骤门要求订单
    `step_id` 精确绑定 → 模型靠猜测大量空转（轨迹 13/18/23 步、4 次 plan_write）。补渲染
    `step.id`（含归档 epoch 读）+ 测试；ADR-0010 §14.21 登记。
    **2026-08-17 闭合**：`epoch.rs` plan 段每步行首渲染 `step.id`
    （`- [状态] <step_id>: <目标> (actions: N; evidence: M)`，live 视图与
    归档 epoch 读同源）+ 系统提示词状态行当前步补 `[step_id]` +
    `blackboard_read`/`blackboard_action_write` 描述补取 id 提示；测试三层
    （epoch 渲染单测、工具级 `blackboard_read section=plan` 回达、跨 epoch
    归档读）；orz 子模块 0d1e01b；orz-loop 436 / orz-tui 178 / orz-assurance
    152 / orz-bin 全量通过、clippy 无新增告警、manifest 1401、仓库门禁 valid。
- [x] （新观察，2026-08-17 复核更正归因 + **用户复核定案**；主项已闭合，后续
  2 项见下）grep 搜索范围与空结果语义：冒烟重跑中两次
  `doomgeneric_mips|frame\.bmp` grep 实际被 plan/console 门
  机械拒绝未执行（journal 序列 24/38）；执行的 7 次 grep 全部无匹配（wall_ms
  1–36ms），含 vm.js 中实测存在的 entryPoint/symbolName/sectionsToLoad/
  syscallNum/runElf/program counter。**根因在工具层**：`finalize_grep` 将
  exit 1 + 空 stdout（或 exit 2 + "No files were searched"）统一转为
  "No matches found"，而 ORZ 总传显式路径、rg 不打印 "No files were searched"
  警告（该分支死代码），「rg 搜索 0 文件」（ignore/隐藏/glob/二进制/超限过滤）
  与「真无匹配」机械不可分；模型将浅层 list_dir 与系统性空 grep 叠加泛化为
  「/app 无 C 源码」错误转向。**撤回「补文本范围报告」方向**，定案=工具契约
  升级：
  - [x] grep 返回结构化搜索信封：`GrepSearchOutput.files_searched` +
    空结果分型；机械来源定稿=v1 用 `rg --files` 探针（仅空结果路径、同过滤集、
    10K 截断；弃用 `--stats`——rg 15 stdout / 旧版 stderr 位置差异污染流式面）；
  - [x] 结局三型分型：非零退出 + stderr 非空→显式报错 / searched=0→
    "Searched 0 files…Retry with --no-ignore/--hidden" / 空 stdout +
    searched>0→"No matches found in N files"；exit 2 硬失败保留；
  - [x] 搜索范围语义显式化：定案=b（保留 rg 默认），参数面新增
    `--no-ignore`/`--hidden` 开关（GrepSearchInput + console 注册表 schema）；
  - [x] 后续①（2026-08-17 补记进计数；**2026-08-18 闭合**）：契约泛化——
    list_dir 目录信封补 listed/ignored/truncated 计数（ignored=未过滤走−
    可见走、同过滤语义、200K 封顶；truncated=可见总数−实际渲染）+ 卡片
    `(scope: ...)` 脚注；legacy/codex 面不报；exit 2 语法错误保持硬失败
    （既有行为保留，无工作量）。orz 614bb3b；list_dir 60 通过。
  - [x] 后续②（2026-08-17 补记进计数；**2026-08-18 闭合**）：grep 命中
    路径 `files_searched` 全结局探针收敛——机械来源定稿=v1 `rg --files`
    探针扩展为每次完成搜索都运行（含命中，摘要行内嵌 searched N files；
    错误路径 stderr 非空保持 None）；`--stats` 跨 rg 版本位置差异污染
    流式面、`--json` 需重写输出契约，均不采用。orz 614bb3b；grep 99 通过。
  - [x] 实施前置=容器内 grep 冒烟已执行（根因复现：glibc 动态 rg 与 bookworm
    GLIBC 不匹配；正常 rg 同命令命中 20 处）；回归验证=Linux 重建后容器冒烟
    （`rg --version` + 已知字符串断言）**2026-08-17 已执行通过**——重建后打包
    rg 静态、任务容器内 version + 断言命中；端到端 `orz --real` grep
    vm.js `syscallNum` 命中 20 行并 done、exit 0（对照旧二进制 7/7
    "No matches found"），证据
    `D:\tb-eval\jobs\2026-08-17__GREP-FIX-SMOKE\smoke-notes.md`；模型侧侦查
    纪律与注册板块 grep 参数提示降为次要契约提示。
  **2026-08-17 冒烟结论（容器实机复现，`alexgshaw/make-doom-for-mips:20251031`）**：
  根因=构建侧打包 glibc 动态 rg（`GROK_TOOLS_BUNDLE_RG_PATH=/usr/bin/rg`，trixie
  产物要求 GLIBC_2.39）进 musl orz；任务容器 bookworm（glibc 2.36）加载失败、
  退出码 1 + 空 stdout，stderr 被 `finalize_grep` exit-1 分支丢弃 → 全部 grep
  显示 "No matches found"。正常 rg 同命令命中 20 处（--stats: 10 files
  searched），默认搜索语义无问题。**a/b 定案=保留 rg 默认语义（b）+ 参数面新增
  `--no-ignore`/`--hidden` 开关**；工具契约补两点：①非零退出且 stderr 非空
  显式报错（先于空结果判断）；②`--stats` 解析 files_searched 入信封。构建侧
  修复=Linux musl 不再用 glibc 覆盖路径，改官方静态 musl rg 下载；冒烟回归=
  容器内 `rg --version` 可运行 + 已知字符串断言匹配。
  **2026-08-17 已实施闭合**：`finalize_grep` 结局三型（非零退出 + stderr 非空
  →显式报错；searched=0→"Searched 0 files…Retry with --no-ignore/--hidden"；
  空 stdout + searched>0→"No matches found in N files"）；机械来源定稿=v1 用
  `rg --files` 探针（仅空结果路径、同过滤集、10K 截断；弃用 --stats——rg 15
  stdout / 旧版 stderr 位置差异污染流式面）；`hidden`/`no_ignore` 开关入
  GrepSearchInput 与 console 注册表；`GrepSearchOutput.files_searched` 入信封；
  build.rs 非 Windows 覆盖路径 ELF PT_INTERP 静态守卫；两份构建脚本改静态 musl
   rg。测试 grep 模块 42 / types 561 / orz-loop console 68 通过；Linux 重建后
   容器冒烟回归已执行通过（见上）。后续①/② 已于 2026-08-18 闭合（见上，
   未闭合 29 → 27 项，P0-E 后续项全部闭合）。
- [x] （新观察，用户 2026-08-17 指示处理）订单发放前拒绝入事件面：冒烟重跑中
  ORD-000011（workspace.run_tests，`arguments:{}`）写入后发放前被拒，失败只进结果栏
  receipt + TraceStore（`consume_console_order` 不写 journal 事件），journal 无结构化
  拒绝记录 → 事后核对看不到拒绝码。**2026-08-17 闭合**：v0.2 `console_order_rejected`
  （order_id / step / phase / code / reason / round / plan_epoch / run_id），发放前
  拒绝统一入事件面——pre_issue（order_stale / step_not_done /
  budget_insufficient，step=protocol）+ issue（registry / contract / target /
  ACAF / policy / mode 门，归一化 step=policy / code=policy_denied）；
  execute/verify 不入本事件（已有 tool_started/tool_completed 留痕）。
  Schema/verifier/fixtures 先行（verifier 交叉核对：拒绝须先有同 run 同
  order_id 的 console_order_written、盖章一致、每订单至多一次拒绝、
  phase/step/code 一致性）；orz 子模块 c67a452（事件变体 + 三处 pre_issue
  路径 + 发放期 Err 分支 issue 路径发事件 + TUI 投影 + 测试断言）；
  orz-loop 436 / orz-tui 178 / orz-assurance 152 / orz-bin 全量通过、
  clippy 与基线一致、manifest 重生成 1401、仓库门禁 valid。结果栏 receipt 保留。

### P0-F FUS-BENCHMARK-FULL-EXEC（`pending`=实施完成待验证；P0，2026-08-18 用户裁决实施）

> 入口：[设计](docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md)；ADR-0010 §14.24
> （v1.24）；BACKLOG 0b。实施路由=实施前登记本项与 BACKLOG（已登记）。
> 2026-08-18 用户指示：实施完成、暂不进行测试（验证项保留未勾选）。
> orz 子模块：`3f43478`（feat/fusion-architecture，6 文件）。
> 2026-08-18 审查收口处理提交：orz `4e9e61b`（4 文件，见下方勾选项）。

- [x] 实施前登记：BACKLOG 0b / 本清单（设计轮不动计数；实施轮 27 → 28）。
- [x] 权限层：`PermissionPolicy::Benchmark { allow_shell, allow_network }`
  两轴参数化（默认 false/false）；决策表=LocalMutation 非 shell AllowOnce、
  shell 工具/SandboxEscape 在 allow_shell 下 AllowOnce、NetworkCall 在
  allow_network 下 AllowOnce、MCP 恒 deny、ReadOnly 恒走 manager；旧测试
  语义同步 + 新增两轴用例。
- [x] 探针层：`ToolPolicy::BenchmarkFull`（`tool_policy()` 由
  `Benchmark{allow_shell:true,..}` 映射）；`policy_allows_exec` 增
  BenchmarkFull；console `ActionBundle::allows` 加臂复用 benchmark 档。
- [x] console 注册表：`workspace.run_terminal`（target=run_terminal_cmd、
  kind=Host、bundle=READ_WRITE；input 镜像 BashToolInput：command/description
  必填、timeout/is_background 可选、不暴露 env/cwd；响应 text_output 信封）；
  注册板块默认列表/投影测试同步。
- [x] CLI：`--allow-shell`/`--allow-network`（→ ORZ_ALLOW_SHELL/ORZ_ALLOW_NETWORK，
  沿用 --allow-write 先例）；未带 `--allow-write` 时 exit 2；`--help` 文本同步；
  `build_cli_host` 构造 Benchmark{allow_shell,allow_network}。
- [x] 适配器：`tb_agents/orz.py` allow_shell=True、allow_network 按任务
  有效 agent-phase `network_policy.network_mode == PUBLIC` 透传（实施注记：
  取 environment.network_policy 而非 task_env_config 基线，89 题全 PUBLIC
  结果一致、严格不更宽）；env 按存在性增 ORZ_ALLOW_SHELL/ORZ_ALLOW_NETWORK；
  运行脚本 belt-and-braces 同传 `--allow-shell`/`--allow-network`。
- [x] 审查收口处理（2026-08-18 全面审查后）：`is_shell_tool` 补 `sh`
  （permission.rs/tool.rs + 两轴与 risk_class 断言）；`workspace.run_terminal`
  timeout 契约 lenient（anyOf integer/纯数字字符串 + default 120000）、
  is_background 补 default false + 契约测试；CLI `=value` 形式显式报错
  exit 2（parse_benchmark_flags + 4 组单测）；bundle 保持 READ_WRITE 实施
  选择确认；`is_background` 后台完成提醒留验证④观察。详见设计 §12 /
  ADR §14.24 / BACKLOG 0b。
- [x] 验证①（2026-08-18 执行，用户放行）：orz 各 crate 全量测试——
  orz-loop 453 / orz-host 221 / orz-tui 178 / orz-assurance 152 /
  orz-bin（lib 11 + benchmark_flags 14 + acaf_e2e 23 + real_flag 2 +
  stdio_e2e 1）/ orz-tools 2761，0 失败；clippy --workspace --all-targets
  无新增可归因告警（43 条均为既有项）；manifest 1401 + 仓库门禁 valid。
  **过程中修复既有测试漂移**（PLAN-FIRST/console 双模式落地后未同步，
  orz 子模块 c4772fc）：codex_app 12 项 + acp_server 1 项改写为
  plan-first + console 订单流；console.rs 注册表默认列表断言按字典序
  修正；orz-host 需 `--test-threads=1` 规避负载敏感的进程树超时竞争。
- [ ] 验证②（用户指示暂缓）：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，输出
  `D:/tb-eval/orz-linux`）。
- [ ] 验证③（2026-08-18 修复后复验一次，机制断言全过、reward 项未达标，
  待加预算重跑）：单题 make-doom-for-mips 复验——reward > 0、journal 出现
  `workspace.run_terminal` 订单→run_host_tool→ACAF `command_exec`
  issued/consumed、机制门不回归（无 400、step_id 绑定、无异常
  policy_denied）。复验 `D:\tb-eval\jobs\2026-08-18__19-40-12`：0 异常、
  无 400、6 笔订单→5 组票据、零拒绝；`context_compressed` 后会话继续
  102 条事件零失败（400 场景闭环）；reward 0=墙钟内未产出可运行 ELF
  （`node vm.js` 超时、无 `/tmp/frame.bmp`），非机制回归。
- [x] 修复：折叠视图 400 根因（验证③前置；处理文档
  `docs/LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING_2026-08-18.md`）——
  - [x] S1 代码修复（orz 子模块）：`run_template_compact` retain 后移
    （guard 判定后，GuardBlocked/NoOp 零副作用）+ 执行路径 `kept_start`
    重算（marker 删除左移 1）；`safe_fold_cut` → `Option`（idx==0 放弃
    折叠）；`build_request_view` preamble 边界校验（末条为声明即回原文）。
  - [x] S2 测试：新增 6 项单测（guard 无副作用 / kept_start 重算 /
    preamble 校验 / idx==0 None / 中轮回退适配 / 旧 bug 场景回原文）；
    `cargo check -p orz-loop` + `cargo test -p orz-loop -j 1` + fmt +
    clippy 无新增（460 通过、基线一致）。
  - [x] S3 Linux musl 重建（build_orz_aliyun.sh，ORZ-BUILD-MOUNT-001）+
    时间戳校验（orz/orz-signer/orz-acaf-provision 三件套 19:39 新构建）。
  - [x] S4 单题复验（验证③断言：无 400、ACAF 票据路径全过；reward 项
    未达标——见下）+ 取证开关 `ORZ_DEBUG_VIEW=1` 暂保留登记为常驻诊断
    （验证③闭合后移除）。
  - [x] S5 文档同步：ADR-0010 §14.27（v1.27）/ 折叠设计 §3.5 / BACKLOG
    0b / TODO 勾选 / CLI_PROJECT_INDEX；提交 orz 子模块 + 父仓库。
  **2026-08-18 取证进展**：三次复验均 reward 0（400 `insufficient tool
  messages` 退出）。核心机制已验证：`workspace.run_terminal` 订单→发放→
  `run_terminal_cmd` 执行 exit=0、ACAF 票据路径生效（journal 证据）。
  400 根因链已闭合（取证存档
  `D:\tb-eval\jobs\2026-08-18__08-44-56\ROOTCAUSE_FORENSICS_20260818.md`；
  根因复核修正见处理文档 §1.3）：真正破坏点=压缩触发（未执行）时
  `run_template_compact` 顶部 retain 删除 marker 而折叠索引未失效
  （GuardBlocked 无 reset），冻结 preamble 吞入首轮 plan_write 声明——
  非「折叠 cut 硬截断」。修复已实施闭合（上方勾选项，orz 提交见下）；
  修复后复验（19:40-20:10）0 异常、无 400、压缩执行后会话继续零失败。
- [ ] 验证④（用户指示暂缓）：2–3 题交叉（compile-compcert、hf-model-inference
  等 build/run 与网络类）。
- [ ] 验证⑤（用户指示暂缓）：`run_official_2.1.sh` 89 题 5 批。
- [ ] 闭合：验证全过 → BACKLOG/TODO/索引状态同步，未闭合 27 → 26。

### P0-0c LEDGER-FOLD-EXTERNAL-FILE（P0；2026-08-18 用户裁决：先设计、不实施；
同日用户指示优先实施——命中率问题优先于 P0-F 验证；S1-S4 全部闭合
2026-08-19，计数 29 → 28）

> 入口：[设计](docs/LEDGER_FOLD_EXTERNAL_FILE_DESIGN_2026-08-18.md)；
> ADR-0010 §14.28（v1.28）；BACKLOG 0c。取代
> `docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md` §3.2/§3.5 视图内
> 台账块；§14.27 400 修复不变量全部保留。
> 来源：复验运行 `D:\tb-eval\jobs\2026-08-18__19-40-12` 命中率 81.9%
> （39 次折叠推进 + 1 次压缩贡献 93.5% miss）。
> 2026-08-18 用户指示：优先实施（暂缓 P0-F 验证序列）；实施前登记本项
> 与 BACKLOG（实施轮入账 1 项，未闭合 28 → 29）。
> orz 子模块提交：见 ADR-0010 §14.28（待提交）。

- [x] S1 代码（2026-08-18 闭合）：
  - [x] action_ledger：`ledger_file_path`（`{session_cwd}/.gsa/ledger/current.md`）、
    `build_pointer_message`（字节级固定指针消息，`LEDGER_FOLD_POINTER_PREFIX`）、
    `external_row_line`（`[<全局序号>] 轮次 <序号>: …`）、`append_ledger_rows`
    （尾行续号 + O_APPEND 原子追加 + flush）；`advance_fold` 返回
    `Option<Vec<ActionLedgerRow>>`（仅本次新增折叠行，纯函数、IO 由调用方
    执行；指针消息首次推进设置后不再重写）。
  - [x] agent_loop：推进触发点写外挂文件（失败回滚 fold 状态 + warn 重试、
    不阻塞会话）；折叠视图尾轮改用 `fold_tail_rounds`。
  - [x] controller：`ContextCompactConfig.fold_tail_rounds` 默认 1 +
    `ORZ_FOLD_TAIL_ROUNDS` 解析 + `with_fold_tail_rounds` 测试缝。
  - [x] summary：marker 追加「历史摘要累积于 <abs-path>」；归档段改
    「折叠视图（冻结快照：外挂指针）」。
- [x] 二次审查修复（2026-08-18 闭合，S1 收口）：
  - [x] 写失败降级：不再 `continue` 空转——回滚 + 计数，连续 3 次失败后
    本循环禁用折叠（视图退回全量原文，压缩兜底）；新增事件面
    `ledger_fold_write_failed`（schema v0.2 同步 + TUI 映射）。
  - [x] 外挂文件仅主车道：检索车道不折叠，其压缩 marker 不携带台账提示。
  - [x] 行格式解耦：`[<全局序号>]`（per-row 跨压缩连续）与 `轮次`
    （窗口内 round_index+1）分离；`tail_seq` 长行稳健化 + 非空文件尾行
    无 `[seq]` 报错回滚。
  - [x] 推进侧 400 校验：`advance_fold` 落行前 preamble/safe_fold_cut
    校验（视图拒绝折叠时行不入文件）。
  - [x] marker 路径提示条件化（文件已存在或本窗口已折叠）；`with_context_compact`
    保留 `fold_tail_rounds`；`ledger_fold_advance` 增 `view_estimate_after`
    触发复位断言。
- [x] S2 测试（2026-08-18 闭合）：action_ledger 18 项（新增：外挂追加续号、
  跨压缩续号（fold reset 后文件续号）、指针字节稳定、advance 仅返回新增行、
  前缀跨推进稳定）；summary 12 项；controller 折叠 e2e 2 项更新（指针前缀
  全请求字节稳定 + 外挂文件断言 + marker 路径提示 + 压缩后继续续号）；
  orz-loop 全量 462 通过（-j 1）、fmt 干净、clippy 与基线一致
  （lib 21 / lib test 26）。
- [x] S3（2026-08-19 闭合）：Linux musl 重建（ORZ-BUILD-MOUNT-001，输出
  `D:/tb-eval/orz-linux` 07:07 新二进制；USTC/清华镜像源 502 不可达改
  用阿里云镜像源）+ 时间戳校验。
- [x] S4（2026-08-19 闭合）：path-tracing 单题复验（job
  `2026-08-19__07-08-57`）——provider 口径命中率 95.33%（journal
  95.54%）≥90%、无 400、折叠重付 3,742/6,493 ≤ ~15K、截断频率 0%
  ≤30%；`ORZ_DEBUG_VIEW=1` 遗留清理项已于 2026-08-20 冒烟预检移除。
- [x] B 定案（机械压缩；2026-08-18 用户裁决 D1=(b)，ADR-0010 §14.29）
  ——S4 复验账单对账定位「未处理的部分」= 压缩摘要调用换前缀重付整段视图
  miss（账单 89.71% vs 事件 93.26%；摘要调用两轮全失败零产出）：
  - [x] summary.rs：`summary_system_prompt`/`summary_user_prompt`/
    `parse_model_output`/退化门/重试常量全部退役；`MECHANICAL_NOTES/
    CONTINUATION_PLACEHOLDER` 固定占位；archive/marker 去掉
    `summary_incomplete` 终止态文案与「生成失败」。
  - [x] agent_loop.rs：`run_template_compact` 移除模型摘要调用
    （`agent`/`cancel`/`heartbeat` 参数退役、`CompactDecision::Executed`
    简化）；五段槽位=黑板 + 占位；存档恒写入、marker 恒带 digest；
    fallback 紧急机械截断保留；事件 `mode=mechanical`。
  - [x] schema/fixtures/verifier：`context-compressed-event-payload-v0.2`
    mode 改 enum（mechanical + template_summary 回放）；5 个 fixtures 改
    mechanical；verifier 增 mechanical 恒完整交叉校验。
  - [x] controller e2e：终止态测试改写为机械模式零模型调用断言
    （received==脚本数）；6 处 summary_response 脚本项退役；orz-loop 465
    通过、fmt 干净、clippy 与基线一致（lib 21 / test 26）、Python
    verifier 214 通过。
- [x] 2026-08-19 全面审查处理（B 定案收口）：
  - [x] fallback 轮数口径：`dropped` 累加、存档/marker「被压轮次」用总
    轮数、事件估计 marker 插入后重算（agent_loop.rs + 新增单测
    `fallback_second_stage_truncation_accounts_total_rounds`；
    orz-loop 466 通过、fmt 干净、clippy 回基线 21/26）。
  - [x] `ledger_fold_write_failed` 契约补齐：payload schema + envelope/
    payload fixtures + verifier 交叉校验（attempt 连续 +1、disabled 预算
    契约、成功追加重置计数）+ 生成器 + conformance 52→53（闭合 2b755d6
    遗留红；Python verifier+conformance 230 通过）。
  - [x] 生成器 `context_compressed` 模板对齐 mechanical（防重生成把已提交
    fixtures 回退为 template_summary）。
  - [x] `orz_source_manifest.sha256` 重生成（orz 提交后）+ 仓库门禁 valid。
  - [x] 文档/注释/schema 描述清理：summary.rs 退化门注释、schema
    `summary_incomplete`/`retained_rounds` 终止态描述、LEDGER 设计
    widened tail 文本。
- [x] D1=(c) HA 结构化事实聚合设计定稿（2026-08-19 用户裁决：先设计、不直接
  动作；纯文档登记、未实施；ADR-0010 §14.30 / 压缩设计 §4.4）：
  - [x] 查看 HA 上游实现与源码（State/CompressedState 压缩、StateMachine
    变更判定、recorder 字典化/哈希去重、assist 管线事件流、GetLiveContext
    静态投影+动态回查分离）。
  - [x] 注意事项槽=HA 结构化事实聚合（助理层唯一新增输出）：数据源全部为
    controller 已机械写入的结构化记录（`plan.steps` Failed/Blocked +
    `exec.errors` 最近 5 + `actions.results` 失败 receipt 最近 3）；排序=
    计划面失败/阻塞 → 执行错误 → 动作失败；空时「（无注意事项）」；≤3K
    超限截断+指针；压缩内部失败继续走 marker 既有标注不进本槽。
  - [x] 后续衔接槽不交助理层：固定中性占位 + 回查入口，由主模型自行判断
    （避免限制或机械性误导）；不聚合任何当前步/下一步/待办内容。
  - [x] 不变项：零模型调用；五槽结构与 17K 上限；存档恒写入、marker 恒带
    digest；schema 无变化；仅 `summary.rs` 聚合渲染与 `run_template_compact`
    接线变化。
- [x] D1=(c) S1 代码（2026-08-19 闭合）：`summary.rs` 新增
  `render_facts_notes`（HA 结构化事实聚合：plan 失败/受阻步骤 + exec 错误
  最近 5 条（每条截断约 200 字符）+ 动作失败 receipt 最近 3 条；排序=计划面
  →执行错误→动作失败；空时「（无注意事项）」；≤3K 超限截断+「其余 N 条见
  blackboard_read 分区/摘要存档」指针）与 `render_notes_capped` /
  `truncate_chars` / `failure_envelope_fields` 辅助；后续衔接占位改中性
  措辞（「由主模型自行判断」+ 回查入口含外挂台账）；`run_template_compact`
  notes 槽接线 `render_facts_notes`；存档/marker 空 notes 防御回退同步为
  「（无注意事项）」。
- [x] D1=(c) S2 测试（2026-08-19 闭合）：summary 新增 6 项事实聚合单测
  （空态/三源排序/最近 5+截断/最近 3 失败含信封缺失回退/3K 溢出指针/单条
  超长退化指针）；
  压缩 e2e 新增 1 项（marker+存档三源事实槽同序）+ 空黑板 e2e 断言
  「（无注意事项）」；orz-loop 473 通过 / 0 失败、fmt 干净、clippy 与
  基线一致（lib 21 / test 26）。
- [x] 2026-08-19 全面审查处理：登记口径更正（6 项 / 473）＋设计补充登记
  （O1 双视角有意冗余、O2 单条超长退化指针、O3 黑板无界可选后续，见压缩
  设计 §4.4.1/§4.4.3 与 ADR §14.30）；计数不变。
- [x] D1=(c) S3（待验证）：Linux musl 重建（ORZ-BUILD-MOUNT-001，输出
  `D:/tb-eval/orz-linux`）+ 时间戳校验。**2026-08-25 核证闭合**：5d55567
  （2026-08-19 01:24）早于 0c 重建轮，随 0c S3 重建（2026-08-19）进入
  三件套，并在现役链上（merge-base 核证）。
- [x] D1=(c) S4（待验证）：命中率复验（≥90% provider usage 口径、无 400）。
  **2026-08-25 核证闭合**：随 0c S4 复验闭环（2026-08-19，provider
  95.33% ≥90%、零 400、115 请求）一并覆盖。
- [x] 2026-08-19 命中率归因 + 黑板读取缓存成本设计定稿（S4 前置；用户裁决：
  大机制不再更改、只补回应命中而未命中的部分；ADR-0010 §14.31 /
  BLACKBOARD_READ_CACHE_COST_DESIGN）——path-tracing 复验（正式 1800s 预算、
  40 请求）provider 85.43% / journal 86.04%；归因=8/8 大 miss 尖峰（~190K=
  64%）紧跟 `blackboard_read`（actions/exec），单次分区 17–32K token 作为
  全新工具结果注入无法命中；非折叠频率、非大机制。定案=actions 结果板去
  response JSON（order_id/ok/step/code/trace_id）、exec 行截断 200 字符 +
  段总长 4K、registration/order 板不变；（阶段 2 可选）since 扩展 actions +
  读取频率引导；零模型、契约/schema 不变、阈值不动；预期 86%→95%。
- [x] 黑板读取缓存成本处理（S1/S2 渲染瘦身已实施，orz 提交 8dbaa01 未推送；
  F1 处理=方案 B 已实施 S1 代码 + S2 测试，orz 提交 ad74714 + 全面审查处理
  未推送）：S1 渲染瘦身 + 单测与 S2 测试已完成
  （orz-loop 479 通过）；2026-08-19 全面审查发现 F1（console 面动作详情
  不可回查——assistant.trace 不在直接工具面、receipt 响应同样被瘦身隐藏）
  → 用户裁决=方案 B 按需点读（设计 §4.5 / ADR §14.31 第 2 项，已登记并
  实施）：`blackboard_read` 新增可选 `receipt_id`（单条完整内容、8K 上限
  截断+指针、epoch 归档点读、非法/未找到显式报错、无 receipt_id 整段逐
  字节恒定）——S1 代码（epoch.rs 点读分支 + controller 参数解析 + 工具
  定义）与 S2 测试已完成（orz-loop 488 通过 / fmt 干净 / clippy 与基线
  一致）；2026-08-19 全面审查处理完成（N1-N4 + O1-O3 登记，见设计 §4.5 /
  ADR §14.31 第 2 项；section 非字符串显式报错 + 新增工具级测试，orz-loop
  489 通过 / fmt 干净 / 无新增 clippy 告警）→ S3 重建 → S4 复验（≥90%、
  无 400）。**2026-08-19 S3/S4 已执行（04:13 运行，path-tracing 1800s）**：
  重建成功（orz-linux 新三件套）；provider 口径命中率 88.84%（对照上次
  窗口 84.07%），仍低于 90%——归因=3 次折叠重付 47.8K/50.4K/56.4K
  （66.5% miss），折叠重付 ≈ 保留尾大小 + 新内容（保留尾实测 44–56K 真实
  token）。→ 折叠桥接设计定稿（见下条）。0c S4 前置，计数不变。
  **2026-08-25 核证闭合（取代归档）**：S3/S4 已执行（04:13，88.84% 未
  达标）；后续由折叠桥接截断（52d698c/d39da27）接管并随 0c 验证闭环
  （95.33% ≥90%）闭合；代码均在现役链（merge-base 核证）。
- [x] 折叠桥接截断（设计定稿 2026-08-19，ADR-0010 §14.32 /
  LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN；**S1 代码 + S2 测试已实施
  2026-08-19，orz 提交 52d698c，S3/S4 待续**）：折叠后其余进外挂台账、
  视图只留最新桥（默认 8K 真实 token，`ORZ_FOLD_TAIL_TOKENS` 可配；
  字符预算近似 + S4 实测校准）；形态甲=先定裁剪再对桥内容级截断（最新
  完整轮结构全保留、超预算只截内容 + 指针、保留尾部优先）；
  `reasoning_content` 剔除（不进桥/不进审计）；`fold_tail_rounds` 退役；
  触发 128K、外挂台账/压缩/白名单/400 防线不变。推算命中率 → 约 94%。
  全面审查处理完成（N1-N3 + O1-O3，见设计文档 §3.1–§3.4/§7 与 ADR
  §14.32 第 2 项；orz-loop 503 通过 / fmt 干净 / 无新增 clippy 告警）。
  路由：S1 代码 ✓ → S2 测试 ✓ → S3 重建 → S4 复验（≥90%、无 400、
  每窗重付 ≤~15K、截断频率 ≤30% 校准）。**2026-08-25 核证闭合**：S3
  重建 + S4 复验已随下方「闭合」条目完成（2026-08-19：95.33% ≥90%、
  零 400、重付 3,742/6,493、截断 0%）；52d698c + d39da27 在现役链。
- [x] 闭合（2026-08-19）：S4 复验 + provider 对拍完成——journal 口径
  95.54% / **provider 口径 95.33%**（hit 4,896,384 / miss 239,752 /
  115 请求，≥90% 达标）、无 400、折叠后首请求重付 3,742 / 6,493
  （≤~15K）、截断频率 0%（≤30%）；换算系数按实测校准 4 → 2
  （`FOLD_TAIL_CHARS_PER_TOKEN`，桥回到 8K 真实 token；orz 校准调整待
  提交、下次正式跑分使用）。**0c 验证闭环，未闭合 29 → 28**；BACKLOG/
  TODO/索引状态同步完成（ADR-0010 §14.32 第 4 项）。

### P0-0d OUTPUT-DEGENERATION-GUARD（P0；2026-08-19 用户裁决：先设计、
不实施；设计定稿；**S1-S4 全部闭合 2026-08-20**）

> 入口：[设计](docs/OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19.md)；
> ADR-0010 §14.33（v1.33）；BACKLOG 0d。
> 来源：make-doom-for-mips 两次失败（2026-08-19）——模型退化复读
> （201,230 字符、2,316 次省略标注重复；历史先例 2026-08-11 479,957
> 字符同任务）+ 160K max_tokens 放大 + 工具结果截断无可再读闭环；
> 探针 60/60 排除网络；停滞守卫仅事后评估、失败轮次未调用。
> 定案（用户裁决）：8K 全统一限值（终端 20K→8K、点读 8K 确认、桥 8K
> 不动）+ 补读闭环硬约束（截断末尾 read_file 指针、落盘可读已验证、
> 点读指针改向、桥截断保留工具结果自身指针）+ 生成期实时复读检测
> （on_chunk 连续相同块 N=5 / 1K token 窗口重复率 >60% / 连续 3 次退化
> 中断 → run_invalidated）+ `REQUEST_MAX_TOKENS` 160K→32K。
> 实施路由：S1 代码 → S2 测试 → S3 重建 → S4 复验（无退化中断、无 400、
> 命中率 ≥90%、补读路径可用）。设计轮不动计数（28）。
>
> **2026-08-19 S1/S2 实施闭合（用户放行）+ 全面审查处理**：S1 落地（orz
> 提交 5e968ec + 审查处理 19b839f，未推送）——on_chunk 实时退化检测（N=5 连续相同 delta /
> 1K token 窗口 3-gram 重复率 >60%）、中断不重试、会话级连续计数达 3 →
> `run_invalidated{status: degeneration}`（schema 先行）、单轮上限
> 160K→32K、终端 8K + read_file 补读指针、点读指针改向落盘文件、桥截断
> 保留工具结果自身指针、失败轮次 stagnation 审计。S2：orz-loop 510 /
> orz-tools 2763（沙箱外）/ fmt / clippy 基线 / 仓库门禁 valid。审查处理：
> P1 点读终端判定改按 `ActionResult.action`（信封形状被 text-output 动作
> 共用，原判定给非终端 receipt 死指针）+ 回归测试；P3 指针块计入 8K
> 预算、160K 注释清理（探针改 32K）、.gsa 可读性专属测试、登记同步。
> 计数：实施放行入账（28→29），S3/S4 验证闭环后 29→28。
> **2026-08-20 S3 重建 + S4 复验闭环 + S4 缺口修复（用户放行）**：S3
> Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，两轮重建成功、三件套
> 时间戳更新）；S4 make-doom-for-mips 单题复验（三轮 + 端到端探针）——
> 无退化复读中断/无 hang、零 400、journal 口径命中率 94.25% 与 91.91%
> 均 ≥90%、补读路径真实可用。S4 发现并修复缺口：权限层拒绝 .gsa 读取
> 使截断收据的补读指针不可用（第二轮回执尝试补读被 policy_denied），
> 修复=白名单会话 .gsa 卷内 `session/terminal/*.log` 的 read_file/grep
> （lexical 限目录 + canonical 限会话卷；run_tests 白名单补 symlink-aware
> 比较），orz-host 单测 221 通过 / fmt 干净 / clippy 无新增；端到端探针
> 确认模型按指针 read_file 三次成功（1200 行完整取回）。计数：验证闭环
> 29 → 28。
> **2026-08-20 换题复验（gpt2-codegolf）+ 流式重试节奏设计定稿（用户
> 裁决：先设计、不动作）**：P0-0d 闭环后按用户指示换题复验稳定性——
> gpt2-codegolf（历史 7 次失败、需读约 500MB GPT-2 权重、写 <5000 字节
> C 程序）三轮运行：第一轮瞬时连接错误（20s idle → 90s 内中断 →
> zero-chunk 重试 2 次后 32s 窗口耗尽）；第二/三轮首轮 request→
> model_output 约 10.5/10.6 分钟（AgentTimeoutError 正常收尾）、有效
> 工作时间约 3 分钟、8 请求、命中率 85.19%（样本不足非机制退化；零
> 400、无退化中断、ACAF 全过）。环境排查=容器/网络/API 均正常（对照
> 昨天 make-doom 首轮 8.3s；差异在 DeepSeek 端首轮生成慢 + 瞬时连接
> 错误）。**定案（用户裁决：重试间隔缩短）**：`stream_idle_warn`
> 20s→5s、`stream_idle_timeout` 90s→50s（=5s×10 轮）、
> `request_retry_window` 32s→50s、`request_max_retries` 10 次不变；
> idle 只看完全无数据（慢速 reasoning 流不误杀）、重试仍指数退避、
> 退化中断不重试纪律不变、无新增旋钮；实施路由 S1 代码 → S2 测试 →
> S3 重建 → S4 复验（≥90%、无 400、无退化中断、首轮不再 10 分钟级
> 长等）；设计轮不动计数（28）。登记于 ADR-0010 §14.34 /
> STREAM_RETRY_RHYTHM_DESIGN_2026-08-20 / BACKLOG 0d /
> CLI_PROJECT_INDEX。
> [x] S1 代码
> [x] S2 测试（含审查处理补项）
> [x] S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）
> [x] S4 复验（无退化中断、无 400、命中率 ≥90%、补读路径可用）

### P0-0d 后续：STREAM-RETRY-RHYTHM（P0 派生；2026-08-20 设计定稿，
**2026-08-25 取代归档**——原定案被后续 3（180s 窗口）+ OUTPUT-BUDGET
（idle 30s）取代，原 S1-S4 不再单独实施）

> 入口：[设计](docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md)；
> ADR-0010 §14.34（v1.34）；BACKLOG 0d。
> 来源：P0-0d 换题复验（gpt2-codegolf）——第一轮瞬时连接错误在 32s
> 窗口内仅重试 2 次即放弃；第二/三轮首轮请求约 10 分钟（DeepSeek 端
> 首轮生成慢，idle 20s/90s 判定粒度粗）；环境排查确认容器/网络/API
> 均正常。
> 定案（用户裁决）：idle 无数据判定 5s 一轮、10 次上限（总 50s 窗口）
> ——`stream_idle_warn` 20s→5s、`stream_idle_timeout` 90s→50s、
> `request_retry_window` 32s→50s、`request_max_retries` 10 次不变；
> idle 只看完全无数据、重试仍指数退避、退化中断不重试纪律不变。
> 实施路由：S1 代码 → S2 测试 → S3 重建 → S4 复验（≥90%、无 400、
> 无退化中断、首轮不再 10 分钟级长等）。设计轮不动计数（28）。

- [x] S1 代码（RetryPolicy 默认值三参数修订 + 注释同步）
- [x] S2 测试（默认值/指纹断言 + 既有策略测试核对）
- [x] S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）
- [x] S4 复验（≥90%、无 400、无退化中断、首轮不再 10 分钟级长等）
  ——**2026-08-25 取代归档**：对应功能由后续 3 S1/S2（a96faab，
  request_retry_window 50s→180s）与 OUTPUT-BUDGET S1（idle 30s）
  实现，并随后续构建轮（0g/0h/0.1.0）合入现役链。

### P0-0d 后续：OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD（P0 派生；2026-08-20
设计定稿，待实施）

> 入口：[设计](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)；
> ADR-0010 §14.35（v1.35）；BACKLOG 0d。
> 来源：官方 deepseek-harness 对照（默认 high/256K/EMPTY_RESPONSE 快速有界
> 重试）+ 今晚空流链实证（max+32K 截断→空响应→D-6 原样重试放大；检测器
> content 盲区）。
> 定案（用户方向，先设计不实施）：`REQUEST_MAX_TOKENS` 32K→256K（回落档
> 128K）；D-6 空流链官方化收窄（完成型空响应快速有界重试 ≤2 次、500ms→10s+
> 10% jitter → thinking 禁用降级；reasoning 族异常不原样、直接降级）；退化
> 检测器升级为输出健康哨兵（content+reasoning+tool 观测、reasoning 复读、
> reasoning-stall 600s/64K 双信号（预算兜底）——空转预算与 max_tokens 解耦、
> 重试分类=有可见输出不重试/无可见输出降级、DEGENERATION_LIMIT=3 三族共享）；
> 二轮修订（实测校准）：合法难题首轮 17,757 reasoning/184s 正常产出
> （RUN-CLI-6a85f668），120s/16K 会误杀；三轮修订（用户裁决：兜底兼容 max
> 思考）：成本账（32K≈¥0.147、64K≈¥0.294、现状空流链 2×32K≈¥0.30）——
> 64K 兜底单次最坏 ≈ 现状整条链且消除链式等待，兜底定 600s/64K（S4 校准
> 300–900s/32–128K）；同日修订——idle 死线 50s→30s（取代
> STREAM-RETRY-RHYTHM 未实施的 50s）、160K 复读归因已修正作为恢复依据、
> 重试层保留 transport 内链 + 官方节奏。
> 实施路由：S1 代码 → S2 测试 → S3 重建 → S4 复验（难题单题、账单对账、
> 空流率观测、stall 校准）。设计轮不动计数（28）。

- [x] S1 代码（256K + D-6 链改造 + 检测器升级）——**2026-08-20 实施完成**
  （用户放行实施、暂不重建/测试）：`REQUEST_MAX_TOKENS`/max_tokens
  32K→256K、idle 死线 50s→30s、`generate_stream` D-6 链官方化收窄
  （完成型空响应快速重试 ≤2 次 + 降级出口；reasoning 族直接降级）、
  退化检测器升级为输出健康哨兵（content+reasoning+tool 观测、
  reasoning 复读灵敏层、reasoning-stall 600s/64K 预算兜底与 max_tokens
  解耦、detail 前缀/分类器、run 层审计留痕）；既有断言与注释同步。
  计数：实施放行入账（28 → 29）。登记于 ADR-0010 §14.35 第 2 项 /
  [设计 §4.1](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / BACKLOG 0d / CLI_PROJECT_INDEX。
- [x] S2 测试（请求头/stall 双信号/链路径/估算校准/回归）——**2026-08-20
  实施完成**：新增 15 项（退化检测器单测 10 项 + 空流链 e2e 5 项），
  orz-loop lib 527 通过 / 0 失败 / 3 ignored；fmt 干净、clippy 无新增
  告警、`cargo check --workspace` 通过。计数不变（仍 29）。登记于
  ADR-0010 §14.35 第 3 项 /
  [设计 §4.2](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / BACKLOG 0d / CLI_PROJECT_INDEX。
- [x] S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）——**2026-08-20
  完成**：三件套时间戳更新（orz 104.4MB，18:45）。
- [x] S4 复验（≥90%、无 400、无空流链、账单对账、stall 校准；不可接受
  回落 128K）——**2026-08-20 完成（S1-S4 全部闭合）**：gpt2-codegolf
  单题（job `2026-08-20__18-46-52`，RUN-CLI-6a86db35）——完成型空流 0、
  零 400、首输出约 66s（对照 32K 时代 10.5 分钟级）、journal 命中率
  95.28%（85 请求）、`reasoning_repetition` 灵敏层拦截 1 次并降级收尾、
  stall 兜底（600s/64K）零触发零误杀、单请求最大 completion 20,776
  （无预算放大异常）；输出成本估算 ≈ ¥0.88（¥4.592/M）；控制台 CSV 待
  刷新补精确对账；600s/64K/30s 初值维持不调。**计数：S3/S4 验证闭环
  29 → 28**。登记于 ADR-0010 §14.35 第 4 项 /
  [设计 §4.3](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / BACKLOG 0d / CLI_PROJECT_INDEX。

### P0-0d 后续 2：THINKING-DEFAULT-HIGH-LADDER（P0 派生；2026-08-20
设计定稿，S1/S2 已完成——orz b72a0a4 已推送；S3/S4 待续）

> 入口：[设计 §3.6/§4.4](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)；
> ADR-0010 §14.35（v1.35 第 5 项）；BACKLOG 0d。
> 来源：S4 复验（256K+max）实测机制已稳（空流 0、命中率 95.28%、复读
> 灵敏层拦截 1/85 并降级收尾、stall 兜底零误杀），max 不再是必要工作点；
> 思考禁用本身质量影响大（「快答模式」）。
> 定案（用户裁决：方案 B + 中间档）：①默认 `reasoning_effort` max →
> **high**（官方默认档；`EnabledMax` 保留显式可选档，仍受哨兵保护）；
> ②降级梯插入 **low** 中间档——**high → low → disabled → 失败**（空
> 响应快速重试与 reasoning 族哨兵跳转共用；「middle」= DeepSeek
> `reasoning_effort=low`）；③兜底/重试节奏不变（stall 600s/64K、idle
> 30s、`EMPTY_RESPONSE_MAX_RETRIES=2`、退避 500ms→10s+10% jitter）；
> ④指纹含 thinking → 部署后一次性变化。设计轮不动计数（28）。
> S1/S2 进度（2026-08-20）：`ThinkingMode` 增 `EnabledLow`、默认
> `EnabledHigh`（max 保留显式可选档）；`apply_thinking` 双旋钮统一；
> `generate_stream` 降级梯 high → low → disabled → 失败（空响应每档
> ≤2 次快速重试、换档重置计数与退避；reasoning 族哨兵逐级下降；max
> 显式档保留 S4 直跳 disabled 基线）；请求头指纹含 high/low；测试
> 531 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 基线一致、workspace
> check 通过。登记于 ADR-0010 §14.35 第 6 项 /
> [设计 §4.5](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)。
> 审查处理（2026-08-20，用户指示处理审查全部问题；orz 1651f59，已提交、
> 未推送）：未发现功能缺陷；O1 非流式 generate 链不引入 low 档登记为
> 有意不对称（已知边界）；O2 双份映射补同步注释；O3 60s 退避兜底登记；
> O4 新增指纹 thinking 档断言（+1 项，orz-loop lib 532 通过 / 0 失败 /
> 3 ignored）；O5 子串匹配登记为已接受。计数不变（仍 29）。登记于
> ADR-0010 §14.35 第 7 项 /
> [设计 §4.6](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)。
> S3/S4 复验登记（2026-08-20，用户指示换题复验）：S3 Linux musl 重建
> 成功（三件套 12:08）；S4 **换题 make-doom-for-mips**（P0-0d 退化起源
> 题）单题复验（job `2026-08-20__20-08-50`，RUN-CLI-6a86ee6c，wallclock
> 1740s 跑满、reward 0.0、零异常）——完成型空流 0、零 HTTP 400、零 idle
> 死线、journal 命中率 92.18%（194 请求，≥90% 达成）、哨兵/stall 全零
> 触发零误杀（600s/64K/30s 维持不调）、首输出 5.5s（对照 max 66s）、
> output 169,182（reasoning 77%）估算 ¥0.78（对照 max ¥0.88）；high vs
> max 跨题参照（任务不同非严格同题）：194 请求/333 工具轮 vs 85 请求，
> 每轮更快、output 成本更低；命中率差异属跨题非档位回归。**计数：
> S3/S4 验证闭环 29 → 28**。登记于 ADR-0010 §14.35 第 8 项 /
> [设计 §4.7](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)。

- [x] S1 代码（`ThinkingMode` 增 `EnabledLow`、默认 `EnabledHigh`、三级梯接线）
- [x] S2 测试（high/low 请求头断言、三级梯路径、回归全绿）
- [x] S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）
- [x] S4 复验（换题 make-doom-for-mips；空流 0、零 400、命中率 92.18% ≥90%、无 stall 误杀；计数 29→28）

### P0-0d 后续 3：ZERO-CHUNK-RETRY-WINDOW-180S（P0 派生；2026-08-21
用户裁决：简单拉长窗口，S1/S2 已闭合 2026-08-21、S3 已闭合 2026-08-25
（合并批次随 0g/0h/0.1.0 构建轮覆盖）、S4 待续）

> 入口：[设计修订](docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md)；
> ADR-0010 §14.36（v1.36）；BACKLOG 0d（变更记录注记）。
> 来源：第一轮 5 题冒烟扫描（sweep-r1-g1）llm-inference-batching-
> scheduler trial——约 1 分钟级 DeepSeek 节点抖动，zero-chunk 重试在
> 50s 窗口内 5 次即耗尽、run 非零退出（reward 0）。网络抖动本质是
> 节点超时，降级无实际作用（用户裁决）。
> 定案：`RetryPolicy::request_retry_window` 50s → **180s**；
> `request_max_retries` 10 不变（双上限先到者止，实测 10 次 ≈ 约 2 分钟
> 重试跨度，窗口放宽后次数上限成为主要约束）；非流式 create 退避窗口
> 同步放宽；retry 参数参与请求头指纹（部署后首次请求一次性指纹变化）。
> 终端解码错误重试兜底设计（dna-assembly Disabled 档 `error decoding
> response body`）待用户确认边界后与本项批次合并 S3 重建一次到位。

- [x] S1 代码（model.rs `request_retry_window` 默认值 50s→180s + 注释同步）
- [x] S2 测试（`default_retry_policy_matches_stream_retry_rhythm` 断言
  50s→180s；orz-loop 测试全绿）
- [x] S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约；与终端解码重试
  兜底批次合并时一次到位）——**2026-08-25 核证闭合**：a96faab（180s）
  与 a0d85f8（fail-fast/解码重试）已随 0g S3（033fd26/4ca60c2）、0h S3
  （f4f1b81）与 0.1.0 发布重建（8bcf18c）合入 Linux musl 三件套，
  merge-base 祖先核证通过；现役 orz 104,796,704 B（2026-08-25 03:56）。
- [ ] S4 复验（无 400、命中率 ≥90%、断连窗口内可骑过节点抖动）

### P0-0d 后续 4：STALL-DEGENERATION-FAILFAST（P0 派生；2026-08-21
设计定稿；S0 证据门通过 + S1/S2 已闭合 2026-08-21、S3 已闭合 2026-08-25
（合并批次）、S4 待续）

> 入口：[设计](docs/STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md)；
> ADR-0010 §14.37 第 1 项；BACKLOG 0d。
> 来源：sweep r1-g1 12 次哨兵归因——跨请求烧 stall 是恢复机制副产品
> （降级梯每请求回 high、成功清零计数器，stall→成功→stall 可无限烧
> 64K）；harness 对照=step 边界有界重试、空即 step 失败。
> 用户裁决：fail-fast 方向有道理；前置证据门（S0）=确认空转/重复与
> 架构无关；确认后实施、显式标明终止原因。
> 主案：会话级 thinking 档位（哨兵后不再回 high）+ 哨兵计数单调
> （成功不清零、达 3 → run_invalidated 显式终止）+ disabled 档哨兵
> 即终止；严格案（哨兵即 step 失败）留对照。
> **2026-08-21 S0 通过 + S1/S2 实施 + 全面审查处理闭合（用户放行实施、
> 审查全部问题处理）**：S0 证据门通过（r1-g1+g2 两批 10 题 7 次触发对、
> 0 例机械块强相关、分布不均，判定模型/任务侧）。S1 代码=会话级档位
> `session_thinking` + 计数 run 内单调（成功不清零）+ disabled 档即终止
> + detail 显式化（族 + consecutive + round，schema 增可选 detail）；
> **正式路径 per-run 隔离**=`ModelGateway::for_new_run()` 每 run 换新
> 实例（controller run_turn_inner 开头，主/子代理共享 run 实例；
> run_retrieval_subagent 不再引用常驻 subagent 字段）——长驻进程
> （ACP server）跨 run 零泄漏、并发会话零干扰；`generate_stream` 达限
> 独立分支（不依赖档位/分支顺序）；日志分流（零 chunk/中段）、注释修正
> （has_complete_tool_call 保守上界、saw_chunk 哨兵语义、混合序列）。
> S2=新增 3 项（for_new_run 跨逻辑 run 档位重置、recovered/exhausted
> 事件）orz-loop 544 / 0 失败 / 3 ignored、orz-tui 178、Python
> conformance 230 通过。

- [x] S0 证据门（下一批扫描采集哨兵触发上下文，按设计 §2.1 判定；
  r1-g1+g2 两批通过）
- [x] S1 代码（会话级档位 + 计数单调 + disabled 档终止 + detail 显式化
  + for_new_run per-run 隔离）
- [x] S2 测试（跨请求档位保持、计数不重置、达限/disabled 终止、
  for_new_run 隔离、回归全绿）
- [x] S3 重建（Linux musl；与窗口 180s + 解码兜底批次合并一次到位）
  ——**2026-08-25 核证闭合**：同后续 3 S3 注（随 0g/0h/0.1.0 构建轮覆盖）。
- [ ] S4 复验（单 run 哨兵预算有界 ≤3 次触发 × 单次预算、显式终止可观测、
  命中率 ≥90%、零 400）

### P0-0d 后续 5：MIDSTREAM-DECODE-RETRY（P0 派生；2026-08-21
设计定稿；S1/S2 已闭合 2026-08-21、S3 已闭合 2026-08-25
（合并批次）、S4 待续）

> 入口：[设计](docs/MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md)；
> ADR-0010 §14.37 第 2 项 / ADR-0007 修订注记；BACKLOG 0d。
> 来源：dna-assembly Disabled 档重试遇 `error decoding response body`
> （已见 chunk 后截断）按「已见输出不重试」直接杀 run；幂等性核对=
> 错误路径工具从未执行、重发无副作用。
> 用户裁决：按「无完整 tool_calls 即重试（有界）」实施，无异议。
> 定案：重试判定从「零 chunk」改「无完整 tool_calls」——已见 chunk 的
> Transport/解码截断有界重试 1 次后显式失败；已见完整 tool_calls /
> Model / Parse / Cancelled 不重试；重试计数入事件面。
> **2026-08-21 S1/S2 实施闭合（用户放行 + 全面审查处理）**：判定改
> 「无完整 tool_calls」（`wrap_no_tool_side_effects` +
> `has_complete_tool_call` 双保险）+ 中段有界 1 次
> （`CHUNKED_MIDSTREAM_MAX_RETRIES` 编译期常量）+ `StreamInterrupted`
> 增 `saw_chunk`；**事件面计数一并实施**——v0.2 新增 `transport_retry`
> （recovered/exhausted、zero_chunk/midstream、retries、reason；
> run-event enum 53→54、schema/fixtures/conformance/TUI 同步）；
> 日志/注释类审查项同步处理。orz-loop 544 / Python conformance 230
> 通过。

- [x] S1 代码（判定改「无完整 tool_calls」+ chunked 有界 1 次 + 事件面计数）
- [x] S2 测试（中段截断重试成功/耗尽、完整 tool_calls 不重试、Parse 不重试、
  与降级梯交互、零 chunk 纪律回归全绿、recovered/exhausted 事件）
- [x] S3 重建（Linux musl；与窗口 180s + fail-fast 批次合并一次到位）
  ——**2026-08-25 核证闭合**：同后续 3 S3 注（随 0g/0h/0.1.0 构建轮覆盖）。
- [ ] S4 复验（dna 类场景不再因解码错误杀 run、零 400、命中率 ≥90%）

### P0-0d 后续 6：REPETITION-DETECTOR-ROLLING-HASH（P0 派生；2026-08-21
设计定稿；**S1-S4 全部闭合 2026-08-23，计数 29 → 28**）

> 入口：[设计 §3.3/§4.8](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)；
> ADR-0010 §14.35 第 13 项；BACKLOG 0d。
> 来源：dna-assembly 复跑（RUN-CLI-6a885faa）误杀实证——seq 95 完整
> 输出 6439 字符连贯正常 DNA 分析被路径①「连续 5 个相同 content delta」
> 误判复读（低熵特征 ttttt/aaaaa/ggggg/N N N N N/GGTCTC + 小 chunk 粒度）。
> 用户裁决：误杀必须处理、判定粒度提高；定案=滑动窗口滚动哈希任意
> 偏移检测。
> 定案：路径①替换为滚动哈希任意偏移——144 字符缓冲（比较区 96 字符）+
> 48 字符 L-gram 哈希集，新 L-gram 哈希在比较区内已见（偏移 ≥48）即触发
> （字符级比对防碰撞）；窗口内任意周期可命中（p ≤ 96）、同字符连串 ≥96
> 触发、DNA 正常序列免疫、
> O(1)/字符；3-gram 路径②（≥1K token >60%）保留兜底；content/
> reasoning 两族共用；stall 兜底（600s/64K）与 fail-fast 纪律不变。

- [x] S1 代码（transport.rs `feed_repetition` 路径①改滚动哈希；新常量
  REPETITION_MIN_RUN_CHARS=48 / REPETITION_WINDOW_CHARS=96；3-gram 保留；
  **2026-08-21 实施闭合**：RollingRepetitionWindow + 既有断言同步，
  orz-loop 550 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增、
  `cargo check --workspace` 通过；计数 28 → 29）
- [x] S2 测试（**2026-08-21 实施闭合**：既有断言已在 S1 同步；新增 8 项
  ——短低熵不触发/周期 10 循环触发（content+reasoning）/poly-A 95 不触发
  ·96 触发/单一大 chunk 不触发/`spans_equal` 字符级比对（碰撞构造不可行
  登记为已接受边界）/近重复不触发+精确复读触发；orz-loop **558 通过 /
  0 失败 / 3 ignored**、fmt 干净、clippy 无新增、`cargo check --workspace`
  通过）
- [ ] S3 重建（Linux musl；与 fail-fast/解码兜底/180s 窗口批次合并）
- [ ] S4 复验（dna-assembly 低熵误杀消除、真复读仍触发、零 400、
  命中率 ≥90%）——**2026-08-22 已执行、发现缺口未闭环**：job
  `dna-assembly-s4`（RUN-CLI-6a887a1a，15m6s、118 请求、reward 0.0
  无异常）——命中率 98.2%（hit 4,379,264 / miss 80,340）≥90% 达成；
  真实 HTTP 400 为 0（journal 中 10 处 "400" 均为订单号/UUID 子串）；
  content 层滚动哈希零触发（旧「5 identical deltas」误杀源消除）。
  **发现缺口 A**：reasoning 层滚动哈希触发 2 次（run 早期 36s/96s，
  consecutive 1→2、降级梯 EnabledMax→EnabledLow→Disabled），但触发
  内容无日志留痕，无法判定真循环还是误杀（模型降级后仍正常完成全部
  工作 147 工具轮 + 完整交付，倾向于误杀）；**发现缺口 B**：会话级
  停滞守卫 `STAGNATION-NGRAM-REPEAT`（controller `evaluate_stagnation`
  对全会话 user+assistant 消息做 3-8-gram 统计、阈值 10）在 run 结束
  时以 max_ngram_repeat=40 判 `restart_requested`——重复内容为常见
  代码惯用式/过渡短语（`for line in open(...).read().splitlines():`
  ×25、"I need to" ×29），非病态复读；跨历史 run 核对几乎所有长会话
  均触发（12-341，全部 run_invalidated restart_requested），为普遍
  误杀，非 DNA 特有问题。**缺口处理待裁决（见下）**。
- [x] S4 缺口处理·审计补充（P0-0d 后续 6 派生；2026-08-22 实施闭合 +
  现场定性）：transport 退化检测器触发时落盘窗口文本（重复 48 字符
  span + 两匹配偏移 + 窗口尾部）至 WARN（缺口 A）；新增
  `RollingRepetitionWindow::last_match`/`match_context` 与
  `DegenerationDetector::trigger_context`，单测
  `degeneration_trigger_context_records_repeated_span`；orz-loop 557
  通过。**现场验证（DNA 重跑 RUN-CLI-6a88905f，26m26s）**：reasoning
  层 2 次触发（consecutive 1→2，降级梯 EnabledMax→EnabledLow→
  Disabled）均留痕——第 1 次重复 DNA 序列等式
  `"tgaggatcccgggaattctcgagtaag..." = "...gggttaa"`（偏移 47/97），
  第 2 次重复技术短语 "bases to the 3' side of the recognition
  sequence"（偏移 16/97）——均为正常思考对任务内容的重复引用，
  **误杀坐实**（非病态复读）；降级后模型继续完成全部工作（177 工具
  轮、136 请求、run_finished completed）。**停滞守卫退役对照**：本次
  run 不再 `run_invalidated restart_requested`（上次 s4 15m 被会话级
  守卫误杀中断），命中率 98.64%（hit 6,954,240 / miss 96,065）。
  **后续待裁决**：reasoning 灵敏层对任务内容重复引用的误杀处理
  （区分病态循环 vs 正常引用）。入口：设计
  [§3.3](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / [transport.rs](orz/crates/orz-loop/src/gateway/transport.rs)。
- [x] 灵敏层再校准设计（**2026-08-22 设计定稿**）——用户裁决：
  L=48→**200**（`REPETITION_MIN_RUN_CHARS`）、W=96→**400**（=2L、缓冲
  144→600）；触发门槛改**流内累计命中 ≥3 次才中断+降级**（间隔不
  重置；1–2 次仅审计留痕；计数随流结束丢弃；会话级 consecutive 与
  `DEGENERATION_LIMIT` 不变）；content/reasoning 统一；3-gram 兜底与
  stall 兜底/fail-fast 纪律不变；判定语义=流内 ≥3 次完全相同的 200
  字符 span（任意偏移、起点距离 ≥200）。设计轮不动计数（29）。入口：
  设计 [§3.3/§4.11](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / ADR-0010 §14.35 第 16-17 项。
- [x] 灵敏层再校准 S1 代码（**2026-08-22 实施闭合**：transport.rs
  `REPETITION_MIN_RUN_CHARS` 48→200、`REPETITION_WINDOW_CHARS` 96→400
  （缓冲 600）、新增 `REPETITION_HIT_LIMIT=3`；滚动窗口命中后继续喂入
  （不早停）、流内累计命中 ≥3 次才 trip、1–2 次仅审计留痕
  （`audit_hits` → 流循环逐条 WARN）；content/reasoning 统一、3-gram/
  stall 兜底不变；单族状态聚合 `RepetitionFamilyState`；trigger detail
  带 `{hits}/{REPETITION_HIT_LIMIT}`；**同日 S1 全面审查处理闭合
  （4 项全部处理）**：`feed_chars_capped` 命中上限喂入（达门槛停止
  消费超大退化帧，子门槛全量消费语义不变）、审计 WARN 与触发判定同
  chunk 聚合（触发时不再输出「audit only」误导文案）、3-gram/stall
  触发清空 `trigger_context`、设计信号表/头部同步再校准参数）
- [x] 灵敏层再校准 S2 测试（**2026-08-22 实施闭合**：poly-A 399/400/401
  不触发（0/1/2 次命中仅审计）、402 触发；周期 10 `repeat(41)`=410 字符
  3 次命中触发；近重复（单字符差异）不计数、精确复读 1 次命中仅审计、
  第 3 次触发；新增命中门槛测试与间隔不重置测试（命中 1 与 2/3 之间插
  入 50 个互异字符仍累计）；缺口 A 单测改 200 字符 span；3-gram 兜底
  用例加唯一标记杜绝 200 字符 span 复现（滚动路径保持静默）；e2e 五处
  改 402 同字符 + 新增子阈值不中断 e2e；orz-loop **560 通过 / 0 失败 /
  3 ignored**、fmt 干净、clippy 无新增（transport.rs 仅 2 条既有 doc
  告警）、`cargo check --workspace` 通过；计数不变仍 29）
- [x] 灵敏层再校准 S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）
  ——**2026-08-25 取代归档**：L=200 校准被下方二级再校准（L=400）取代，
  其 S3 重建（2026-08-23 02:01，orz 6178050）已覆盖本批次。
- [x] 灵敏层再校准 S4 复验（reasoning 正常引用不误杀、真循环仍触发、
  零 400、命中率 ≥90%；DNA 重跑对照）——**2026-08-25 取代归档**：由
  二级再校准 S4 冒烟复验（2026-08-23 sweep-s4-g4/g5：sam-cell-seg
  零复读触发、零真实 400、命中率 96.30–98.34%）闭合。
- [x] 复读判定二级再校准设计（**2026-08-23 设计定稿**）——G4 冒烟实证
  sam-cell-seg 代码引用误杀（203/204 字符 span、引用-再确认循环，非
  设计泄露）；用户裁决：L=200→**400**（W=800、缓冲 1200、命中门槛 3
  不变、同字符连串触发线 802）+ 新增二级「标点块内部重复确认」（按
  标点+空白切块、内部标点块重复覆盖占比 ≥0.50 初值才计命中、每对都过、
  无切分点直接判真）；3-gram/stall 兜底不变；判定语义=流内 ≥3 次「400
  字符完全相同且内部由重复标点块构成（或无切分点）」的 span 才触发。
  设计轮不动计数（29）。入口：设计
  [§3.3/§3.5/§4.11](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / ADR-0010 §14.35 第 18 项。
- [x] 复读判定二级再校准 S1 实施（transport.rs：L/W/缓冲常量、二级
  标点块内部重复确认、每对命中确认、无切分点直接判真；2026-08-23 实施
  完成，单元测试 + 既有阈值测试适配，562 项全绿；2026-08-23 **S1 全面
  审查处理完成**——拒绝候选审计按 delta 聚合（一条摘要+命中计数）、
  触发 chunk 审计留痕、切分符集合边界登记、§3.3 信号表同步，并补阈值
  边界/每对确认累计/G4 短引用形态回放三项测试，565 项全绿）
- [x] 复读判定二级再校准 S2 测试（**2026-08-23 实施闭合（用户放行 S2）**：
  测试矩阵项已随 S1 审查处理提前覆盖——代码引用不计数、`aaa, aaa` 触发、
  无切分点直接判真、覆盖占比阈值边界（49.6%/50.0%/50.8%）、每对确认累计、
  G4 短引用（203 字符）形态等价回放静默；正式 S2 轮=离线回放记录 + 结果
  归档——新增字节级真实回放测试
  `repetition_second_stage_g4_real_span_replay_stays_silent`：从
  `sweep-r1-g4-official\sam-cell-seg__CT5JrD3\agent\orz.txt` WARN 记录
  提取旧 L=200 检测器两次触发的真实匹配 span（各 200 字符；设计早前
  203/204 为完整重复引用区域口径），以 reasoning 族按「引用-再确认循环」
  形态回放 3 遍——首级 L=400 均不命中（无候选、无审计、无触发）；orz-loop
  lib **566 通过 / 0 失败 / 3 ignored**（565→566）、fmt 干净、clippy 无
  新增；计数不变仍 29。登记于设计 §4.11 / ADR-0010 §14.35 第 18 项 /
  BACKLOG 0d / CLI_PROJECT_INDEX）
- [x] 复读判定二级再校准 S3 重建（**2026-08-23 实施闭合（用户放行）**：
  Linux musl（ORZ-BUILD-MOUNT-001 契约，`build_orz_aliyun.sh`；
  `rust:1.97-slim` 增量构建，挂载 `D:\CLI:/orz`、工作目录 `/orz/orz`，
  -j 1）**BUILD_EXIT=0**；三件套时间戳 2026-08-23 02:01（orz
  104,521,992 B / orz-signer 1,388,592 B / orz-acaf-provision 1,206,568
  B，SHA256 见设计 §4.11）；最小可执行冒烟通过（orz 无 TTY 报 TUI io
  error 属预期、provision 打印 usage、signer 报 manifest 缺失）；对应
  源码=orz 6178050 + 父仓库 dfe39a1；计数不变仍 29。登记于设计 §4.11 /
  ADR-0010 §14.35 第 18 项 / BACKLOG 0d / CLI_PROJECT_INDEX）
- [x] 复读判定二级再校准 S4 冒烟复验（**2026-08-23 实施闭合（用户放行）；
  S3/S4 验证闭环 29 → 28**）：G4/G5 对照（sweep-s4-g4 / sweep-s4-g5，
  k=1、官方方式、n-concurrent=1）——G4 4/5（sam-cell-seg **零复读触发**
  对照旧 L=200 同题 2 次触发+2 次降级、误杀消除且 reward 仍 1.0；
  portfolio-optimization 零触发）、G5 2/5（1 题 AgentTimeoutError=官方
  超时）；全 10 试次**零真实 400**；有 journal 9 试次命中率
  96.30%–98.34% 全 ≥90%；构造真循环触发由 S2 测试套件覆盖（566 全绿）；
  观察项=video-processing 3-gram 兜底触发 1 次（0.60 边界、降级一次、
  任务继续，与旧轮 portfolio-optimization 同界，登记不改范围）。登记于
  设计 §4.11 / ADR-0010 §14.35 第 18 项 / BACKLOG 0d / CLI_PROJECT_INDEX）

### P0-0d 后续 7：AGENT-DELIVERY-FLOW（P0 派生；2026-08-23 设计定稿，
S1 代码 + S2 测试闭合 + S1 全面审查处理 + S3 重建登记 2026-08-23，
实施入账 28 → 29（计数不变仍 29）；**S4 复验闭环 2026-08-23（用户指示
补登记），计数 29 → 28（S1-S4 全部闭合）**；**最优先——2026-08-23
用户指示，覆盖本轮提案 1+2**）

> 入口：[设计](docs/AGENT_DELIVERY_FLOW_DESIGN_2026-08-23.md)；
> ADR-0010 §14.35 第 19 项；BACKLOG 0d 后续 7。
> 来源：S4 冒烟失败归因暴露——计划步自动推进语义（订单成功即 step done，
> mteb「计划完成但计算未跑」）、引用校验硬阻断无修正、订单反馈缺
> diff/delta（核实压力）。
> 定案（用户三轮讨论裁决）：步骤重定义=执行顺序标记（目标交交付门仲裁）；
> 模板末步固定「递交/完成」且不随普通订单自动推进；递交状态=黑板 plan
> 机械渲染（workspace_delta 过滤 .gsa/临时文件、上限 20+计数、无模型
> 声明、无复读风险）；订单反馈增强（编辑类回显 diff、终端类挂 delta、
> actions 板只加 changed: N files）；引用失败→有界修正机会一次
> （reason_codes+markers、同失败 2 次恢复硬阻断、journal 记 attempt）；
> 终端内容登记引用暂缓。实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验
> （计划无空转、末步递交走机械状态、引用失败可修正一次、订单反馈含
> diff/delta、零 400、命中率 ≥90%）。设计轮不动计数（28）。

- [x] 设计定稿（2026-08-23；先落设计、未实施）
- [x] S1 代码（**2026-08-23 实施闭合（用户放行）**：计划语义 + 末步模板/
  校验 + submit 递交路径 + 交付状态渲染 + 订单反馈 + 引用修正机会；详见
  设计 §6 / ADR-0010 §14.35 第 21 项）
- [x] S2 测试（**2026-08-23 闭合**：末步校验、submit 两阶段、submit_
  not_current、console_step_done 末步拒绝、diff/delta receipt、changed
  短计数、引用修正一次/二次硬阻断；orz-loop 574 通过 / 0 失败 / 3 ignored、
  fmt 干净、clippy 无新增告警）
- [x] S1 全面审查处理（**2026-08-23 闭合（用户指示处理审查全部问题）**：
  修复——F1 `workspace_delta_walk` 排除面单测、O2 terminal 判定按末步 id
  （旧/恢复计划保持 S1 前语义）、O6 截断 delta `changed: N+ files`；口径
  登记——F2 修正机会=至多 1 次总数、O1 递交为信息展示非硬门、O3 反例门
  once-only、O4 基线捕获时机、O5 walk 成本、O7 不设豁免；orz-loop 579
  通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增；详见设计 §6.4 /
  ADR-0010 §14.35 第 22 项）
- [x] S3 重建（**2026-08-23 闭合（用户指示登记 S3）**：本主题代码已随
  orz HEAD 05231f7 进入 NGRAM-GUARD-CALIBRATION S3 构建轮——Linux musl，
  ORZ-BUILD-MOUNT-001，BUILD_EXIT=0，三件套 2026-08-23 08:50 HKT；对应
  源码 orz 05231f7 + 父 35788a0；计数不变仍 29（S4 闭环后 29 → 28）；
  详见设计 §6.5 / ADR-0010 §14.35 第 28 项）
- [x] S4 复验（**2026-08-23 闭合（用户指示补登记）**：复用 NGRAM S4 实机
  数据（sweep-s4n-g4/g5/g5-cfi，10 试次）——计划无空转（8 完成试次按步
  执行至末步、终答绑定 receipts，历史 mteb 空转形态未再现）；末步递交走
  机械交付状态（8/8 完成试次 submit 双阶段：[delivery] 状态行渲染进黑板
  plan → 复核 → 确认置 done；git-multibranch 终答引用交付状态行；
  sam-cell-seg 末步订单被 step_not_done 拒绝）；引用修正一次/二次硬阻断
  （portfolio/break-filter/mteb retry attempt=1 → block attempt=2；
  git-multibranch/sam-cell-seg retry 后通过）；订单反馈 receipt 点读链全
  走通（diff/delta 机械形态由 S2 锁定）；零真实 400（10/10）；命中率
  10/10 有 journal 94.11%–98.55% 全 ≥90%；计数 29 → 28；详见设计 §6.6 /
  ADR-0010 §14.35 第 29 项）

### P0-0d 后续 8：NGRAM-GUARD-CALIBRATION（P0 派生；2026-08-23 设计定稿，
S1-S4 **全部闭合** 2026-08-23（S1 代码 + S2 测试 + S1 审查处理 + S3 重建
+ S4 复验），**计数 30 → 29**；**最优先——2026-08-23 用户指示，本轮
提案 3，已闭环**）

> 入口：[设计](docs/NGRAM_GUARD_CALIBRATION_DESIGN_2026-08-23.md)；
> ADR-0010 §14.35 第 20 项；BACKLOG 0d 后续 8。
> 来源：两次真实任务 3-gram 路径②边界误触发（旧轮 portfolio-optimization
> + 本轮 video-processing，均显示 0.60、实际 0.600–0.609、正常推理自引用、
> 单发 trip、非致命）。
> 定案（用户裁决）：阈值 `DEGENERATION_NGRAM_REPEAT_RATIO` 0.60→0.70
> （`>` 保留）；流内累计命中（≥3 才 trip、1–2 仅审计（ratio+窗口+族）、
> 间隔不重置、流结束丢弃）；统一口径（WARN 精度、信号表）。边界：0.60–0.70
> 近重复循环漏判由 stall 兜底（600s/64K）；无原始字节、验证靠单测+e2e+
> 实机观察。实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（正常任务零
> 3-gram trip、0.6x 仅审计、≥0.70 三连才 trip、零 400、命中率 ≥90%）。
> 设计轮不动计数（28）；**2026-08-23 S1 代码实施入账 29 → 30（S2-S4
> 待续）**；**2026-08-23 S2 测试闭合（实施入账不变仍 30，S3-S4 待续）**。

- [x] 设计定稿（2026-08-23；先落设计、未实施）
- [x] S1 代码（**2026-08-23 实施闭合（用户放行）**：`DEGENERATION_NGRAM_REPEAT_RATIO`
  0.60→0.70（`>` 保留）+ 新增 `NGRAM_HIT_LIMIT=3` 流内累计命中（每次 feed
  超阈值计 1 次、≥3 才 trip、1–2 次仅审计留痕=ratio+窗口 token 数+族、
  间隔不重置、流结束丢弃）+ WARN 口径 `{:.2}`→`{:.3}`；信号表/参数表
  同步；既有 3-gram ratio 用例适配（单份 core 0.694 恰为 0.69x 边界样本
  保留给 S2、双份 core 0.825 过 0.70 + 2 次审计 + 3/3 trip 断言）；
  orz-loop 579 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增；
  详见 ADR-0010 §14.35 第 23 项）
- [x] S2 测试（**2026-08-23 实施闭合（用户放行）**：0.69x 边界不计数
  不触发（content ratio≈0.694 + reasoning 镜像）、0.70x 边界三连触发
  （9 词 core ratio≈0.720：审计 1/3→2/3→trip 3/3）、单超大 feed 计 1 次
  命中（feed 粒度）、间隔不重置（300 互异 token 压窗口后计数保持 2、
  重灌恢复第 3 次命中 trip）、流结束丢弃（新流首命中只审计不 trip）；
  既有用例适配已于 S1 完成；orz-loop 585 通过 / 0 失败 / 3 ignored、
  fmt 干净、clippy 无新增；详见 ADR-0010 §14.35 第 24 项）
- [x] S1 全面审查处理（**2026-08-23 闭合（用户指示处理审查全部问题）**：
  D1 边界登记修正——0.60–0.70 近重复循环的 stall 兜底仅覆盖 reasoning
  族、content 族为已接受漏判（精确循环由路径①兜住、带变体循环成本受
  REQUEST_MAX_TOKENS 约束、S4 观察）；I1 审计日志按路径标注命中门槛
  （rolling/ngram 两字段）；orz-loop 585 通过 / 0 失败 / 3 ignored、
  fmt 干净、clippy 无新增；详见设计 §2.4 / ADR-0010 §14.35 第 25 项）
- [x] S3 重建（**2026-08-23 闭合（用户指示进行重建）**：Linux musl，
  ORZ-BUILD-MOUNT-001 契约，BUILD_EXIT=0，三件套 2026-08-23 08:50 HKT
  （orz 104,664,664 B / signer 1,388,592 B / provision 1,206,568 B），
  最小可执行冒烟=三件正常加载执行；对应源码 orz 05231f7 + 父 35788a0；
  计数不变仍 30（S4 复验闭环后 30 → 29）；详见设计 §3 / ADR-0010
  §14.35 第 26 项）
- [x] S4 复验（**2026-08-23 闭合（用户放行）**：G4/G5 对照 k=1、官方
  方式——10 试次全零 3-gram trip/零哨兵触发/零 400（历史 0.60 边界误
  触发题 portfolio-optimization / video-processing 均零触发）；命中率
  10/10 有 journal、94.11%–98.55% 全 ≥90%；构造流行为 0.694 不触发、
  0.720 三连才 trip、e2e 中断+降级全绿；观察项=10 试次均无 3-gram 审计
  条目（0.60–0.61 边界带未再现，content 0.60–0.70 实机样本仍空）；
  计数 30 → 29；详见设计 §3 / ADR-0010 §14.35 第 27 项）

### P0-0e CONTEXT-SCAFFOLDING-PULL-REDESIGN（P0；2026-08-21 设计定稿，
S1-S4 全部闭合 2026-08-21，计数 28 → 27）

> 入口：[设计](docs/CONTEXT_SCAFFOLDING_PULL_REDESIGN_DESIGN_2026-08-21.md)；
> ADR-0010 §14.35 第 10 项；BACKLOG 0e。
> 来源：2026-08-20 跑分冒烟失控（content_repetition 0.87 / reasoning 64K
> 空转）——API 实测钉死触发条件=256K × high × 重复性上下文；重复性上下文
> 源自每轮 PUSH 的 `[TOOL_ROUND_BUDGET] REMAINING` 机械块（333 轮 ≈333 条，
> 2026-08-07 前缀缓存修复副作用）。
> 定案（用户裁决）：方案 A 预算块 PUSH→PULL——退役每工具轮 REMAINING 尾随
> 注入；`blackboard_read` 新增 `section=session` 按需读（预算剩余 + 状态行；
> live 面不进归档；越权组合显式报错）；机械硬门禁兜底；总预算块保留。C 暂缓
> （S4 复验后裁决）；状态行保留现状。实施路由 S1 代码 → S2 测试 → S3 重建 →
> S4 复验（重点观测缓存命中率对照 98%+ 不减、零 400、哨兵触发率下降、输入
> 增长放缓）。设计/实施轮不动计数（28），S3/S4 闭环后 28→27。
> **2026-08-21 S1 实施 + 全面审查处理闭合（用户放行；orz 7529a71）**：
> 退役 REMAINING 尾随注入（零残留）；session 面（BUDGET/USED/REMAINING +
> `render_status_line`；数据源=in-run tool_rounds 含 activation 累计 /
> max_tool_rounds；越权组合参数级显式报错 exit_code 1 + error 字段）；
> 工具定义 enum/描述增量；系统提示词总预算块改为指向按需读取。S2 测试随
> S1 交付：协议形态 5→4、无 REMAINING 尾随断言、session 面渲染/越权单测 +
> 工具级回达、`budget_insufficient` 拒绝文本仍含剩余；orz-loop 536 通过 /
> fmt / clippy 基线（31）/ workspace check / 事件一致性 15 通过。全面审查
> 处理：O1/O2/O3 已接受边界（口径差异/PULL 读取消耗轮/状态行双通道）、O4
> 越权组合口径收紧、O5 工具级测试补齐、O6 工具描述去内部标签（设计 §8）。
> **2026-08-21 S4 复验阻断 + 缺口修复**：make-doom-for-mips 单题复验首轮
> 工具轮后第二轮请求 400（`reasoning_content must be passed back`）——
> PUSH→PULL 退役 REMAINING 尾随消息后暴露 2026-08-04 遗留「工具输出汇总
> assistant 文本消息」（`assistant_parts` 冗余副本）为请求末条；API 探针
> V1–V7 实测钉死触发面。修复=退役 `assistant_parts` 汇总消息（工具结果已
> 以 Role::Tool 完整落库，协议形态 4→3；顺带每轮输入 token 节省）；两处
> 协议形状测试同步更新；orz-loop 536 通过 / fmt / clippy 无新增。登记于
> ADR-0010 §14.35 第 11 项 / 设计 §9 / BACKLOG 0e。
> **2026-08-21 S3 重建（修复版）+ S4 make-doom 复验闭环（计数 28→27）**：
> 修复版 Linux musl 重建（orz cf0be20，三件套哈希见 FROZEN）；S4 单题
> 复验（make-doom-for-mips，job `2026-08-21__01-48-26`，RUN-CLI-6a873e04，
> wallclock 1740s 跑满、reward 0.0、零异常）：147 请求、journal 命中率
> 94.45%（hit 6,503,808 / miss 382,108）≥90% 且高于上轮同题 92.18%；
> 零 HTTP 400、零 idle 死线、零哨兵触发（content_repetition /
> reasoning_stall 全零）；output 165,644 tokens（reasoning 77.8%）对照
> 基线 169,182/77% 基本持平；工具轮 226（基线 333）——REMAINING 零残留
> + 汇总消息退役，输入增长放缓达成；首输出 8.1s（基线 5.5s，样本差异）。
> 终态 `run_invalidated{status:wallclock}`（正常预算耗尽）。
> **2026-08-21 方案 C 裁决（用户：无必需性、先看当前情况）**：0e S4 复验
> 闭环后哨兵/stall 全零触发、命中率 94.45%，`REQUEST_MAX_TOKENS` 维持
> 256K 暂不收紧；后续正式跑分中观察哨兵触发率与输出预算，若复发再评估
> 64K。登记于设计 §6 / ADR-0010 §14.35 第 12 项 / BACKLOG 0e /
> CLI_PROJECT_INDEX。

- [x] S1 代码（退役 REMAINING 尾随注入 + session 面 + 工具定义增量 + 机械门禁回归）
- [x] S2 测试（无 REMAINING 尾随、session 面渲染/越权、拒绝文本含剩余、既有断言更新）
- [x] S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约；两轮：7529a71 初建 + cf0be20 修复版）
- [x] S4 复验（make-doom-for-mips：命中率 94.45% ≥90%、零 400、零哨兵触发、输入增长放缓；计数 28→27）

### P0-0f FUS-READ-ANCHOR-WRITE-GUARD（P0；2026-08-21 设计定稿，
纯文档登记、未实施；设计轮不动计数 27；**S1 代码已实施 2026-08-21，
实施入账 27 → 28；S2 测试已闭合 2026-08-21；S3 重建登记 2026-08-23
（随 orz 05231f7 构建轮，计数不变仍 28）；**S4 复验闭环 2026-08-23
（用户指示补登记），计数 28 → 27（S1-S4 全部闭合）**）

> 入口：[设计](docs/READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md)；
> ADR-0010 §14.38；BACKLOG 0f。
> 来源：主 agent 经 read_file 读取快照后向助理层下发写订单，窗口内文件被
> 别处改动时可能基于旧快照修改更新文档；助理层纯机械无提示词，只需"是否
> 同一份"的等值判定。
> 用户裁决（2026-08-21）：锚=哈希值——内容摘要太重、read_file 须机械返回、
> 助理层零理解、简单机械核证；仅针对 orz。
> 定案：read_file 文本路径统一返回内容锚点 {size, mtime, sha256}（大文件
> 信封已有 content_sha256、补 size/mtime；小文件同样返回）；写订单携带期望
> 锚点（可选字段）；orz 写门禁写前机械核证——mtime/size stat 快速预检 +
> sha256 权威比较，不匹配拒单（复用 order_stale 错误信封形态、入
> console_order_rejected 事件面）不执行任何编辑，强制重读重下。时间戳可被
> 保留/取整不作权威；校验-写入 TOCTOU 窗口接受（可选后续=临时文件+原子
> 替换）；expected 必填加严为可选后续。实施放行后入账 27 → 28，验证闭环
> 28 → 27。
> **2026-08-21 S1 代码实施闭合（用户指示开始 S1；orz 工作树未提交）**：
> read_file 锚点（ReadAnchor；小文件 FileContent.read_anchor + prompt
> `[read anchor]` 尾行；信封补 mtime；同一读取快照）；search_replace 契约
> schema 增可选 expected_anchor（size/sha256 必填、mtime 可空）；发放前
> pre_issue 门（stat 快筛 size/mtime + sha256 权威；不匹配拒单
> content_anchor_mismatch 入 console_order_rejected 事件面、清槽零编辑；
> 新建路径跳过；其余 stat/read I/O 错误 fail-closed 拒单——同 code、消息
> 注明失败原因，不放行未核证编辑（2026-08-21 审查收口，补 console_anchor
> 2 条用例）。验证：read_file 202 / types::output 84 / console 69 /
> orz-loop 全量 544 + console_anchor 2 通过、fmt 干净、clippy 无新增告警
> （审查修复 build_read_anchor collapsible_if）；orz-tools 全量 44 个
> grep/glob 失败为本机 rg 环境性既有失败（stash 基线复现一致）。计数：
> 实施入账 27 → 28。登记于 ADR-0010 §14.38 第 2 项 / 设计 §8 / BACKLOG 0f。
> **2026-08-21 S2 测试闭合（用户指示开始 S2；orz 工作树未提交）**：
> 新增 10 条用例——read_file 锚点返回正确性 3（小文件 sha256/size/mtime +
> prompt 尾行、空文件 size=0/空串哈希、大文件信封 mtime/content_sha256）、
> PDF 无锚点 1、prompt 尾行渲染与 ReadAnchor serde 2、写门禁 4（锚点匹配
> 放行（mtime null 跳快筛）、同 size 同 mtime 异内容 sha256 兜底 fixture
> （FileTimes 保留 mtime）、陈旧拒绝→重读重下成功、缺失锚点保持既有行为）
> + 错误信封/事件面完整断言（upstream expected/actual、re-read 指引、
> trace 末事件、机械盖章、零编辑）。验证：orz-tools read_file 207 /
> types::output 86 / orz-loop 全量 550 通过（0 失败）、fmt 干净、clippy
> 无新增告警（30 条既有位置核对）、cargo check --workspace 通过。计数：
> 仍 28（S3-S4 待续）。登记于 ADR-0010 §14.38 第 3 项 / 设计 §9 / BACKLOG 0f。

- [x] S1 代码（read_file 锚点字段：大文件信封补 size/mtime、小文件返回锚点；
  写订单 expected_anchor 参数；写门禁核证 + 拒单错误码 + console_order_rejected
  事件面接线；工具定义/schema 同步）
- [x] S2 测试（锚点返回正确性：小文件/空文件/大文件信封 mtime；PDF 无锚点；
  同 mtime 异内容 sha256 兜底 fixture；错误信封与事件面完整断言；匹配放行
  与缺失锚点保持既有行为；陈旧拒绝→重读重下成功。验证：read_file 207 /
  types::output 86 / orz-loop 550、fmt/clippy/workspace 全绿）
- [x] S3 重建（**2026-08-23 闭合（用户指示登记 S3）**：本主题代码已随
  orz HEAD 05231f7 进入 NGRAM-GUARD-CALIBRATION S3 构建轮——Linux musl，
  ORZ-BUILD-MOUNT-001，BUILD_EXIT=0，三件套 2026-08-23 08:50 HKT；对应
  源码 orz 05231f7 + 父 35788a0；计数不变仍 28（验证闭环后 28 → 27）；
  详见设计 §10 / ADR-0010 §14.38 第 4 项）
- [x] S4 复验（**2026-08-23 闭合（用户指示补登记）**：复用 NGRAM S4 实机
  数据（sweep-s4n-g4/g5/g5-cfi，10 试次）——read_file 锚点返回实机可见
  （mteb-retrieve 终答引用「读取锚点 sha256=1dec6f9c…d24bb，size=…」；
  break-filter 137 次 read_file）；写门禁陈旧拒单→重读重下由 S2 单元级
  预演覆盖、实机无陈旧写入发生；零误拒（10/10 零 content_anchor_mismatch，
  22 次订单拒绝全为模型输入错误）；事件面留痕（console_order_rejected 22
  条机械盖章）；命中率 10/10 有 journal 94.11%–98.55% 全 ≥90%、10/10 零
  HTTP 400；计数 28 → 27；详见设计 §11 / ADR-0010 §14.38 第 5 项）

### P0-0g MECHANICAL-AUDIT-LAYER（P0；2026-08-24 设计定稿；S1/S2 已实施
闭合 + 全面审查处理已完成 + S3 重建完成 2026-08-24；设计轮不动计数；
S4 复验闭环 2026-08-25（补登记）；S1-S4 全部闭合）

> 入口：[设计](docs/MECHANICAL_AUDIT_LAYER_DESIGN_2026-08-24.md)；
> ADR-0010 §14.39（v1.39）；BACKLOG 0g。
> 来源：2026-08-24 用户多轮裁决——同模型 lean harness 对照（Codex
> pass@1 78.65% / Maka 73.03% / 官方 DeepSeek V4-Flash-0731 TB2.1=82.7）
> vs 本机 10 题 50%、每真实动作 2.3–2.9 模型往返、检索 10 试次零调用、
> 引用校验 5/8 触发 3/8 硬 block 吞终答、P1 读范围门可绕过且拦合法读取。
> 定案（用户裁决）：首轮 plan-first 硬门形态不变（钉死极简模式）；
> 第 2 轮起 direct 执行面（模型直接调工具）；助理层拆机械审查层＋半助理层
> （静默记录、每对象键仅最后一轮结果覆盖写、不给建议、报告随最终答案前
> 中立问询轮注入——报告收敛为执行事实摘要=动作/文件 delta/预算/异常事实，
> step/契约类只事件留痕不上报告；半助理层承接命令运行/写执行/检索派发）；ACAF/权限轴/
> 预算/候选计数/run_tests host-owned/read-anchor 写前核证维持前置硬门；
> round 2+ 不要求 step_id；引用校验器整体删除；读范围放开（保留 .gsa
> 不可见、16KB 信封、head_limit）；主面恢复检索（web_search/web_fetch/
> browser_read/retrieve_project_*）。实施路由 S1 代码 → S2 测试 → S3
> 重建 → S4 复验（同一 10 题 k=1 + 构造题：检索调用出现、审计报告覆盖写、
> 零 400、命中率 ≥90%、轮次/耗时下降、reward 对比）。设计轮不动计数。

- [x] 设计定稿（2026-08-24；先落设计、未实施；登记于 ADR-0010 §14.39 /
  BACKLOG 0g / CLI_PROJECT_INDEX v2.19）

- [x] S1 代码（2026-08-24；合并实施包：读放开 + 引用删除 + 检索恢复 +
  direct 面 + 审计层 + submit 非硬门；orz-loop 563 通过 / 0 失败 /
  3 ignored、fmt 干净、clippy 无新增；ADR-0010 §14.39 第 2 项）

- [x] S2 测试（2026-08-24；删/改既有 gate 测试——orz-host/tui 脚本化
  计划统一补固定末步 deliver（AGENT-DELIVERY-FLOW 末步校验适配）、
  acp_server 事件计数 16→18（S1 审计事件留痕）；新增 6 项专项测试：
  审计报告随终答前反例自查轮注入（执行事实/预算/异常事实三类、无建议、
  journal 留痕 plan_gate/budget/tool_result）、direct 面 journal 零
  退役工具调用、终答未绑定引用原样交付（行为侧零残留）、主面检索族声明、
  主车道 web_search 派发子代理（构造题检索可达）、检索候选计数/超限分类；
  orz-loop 569 通过 / 0 失败 / 3 ignored、orz-tui 178 / 0、orz-host
  串行 221 / 0（并行仅 1 项预存时序 flake：call_tool_timeout_kills_
  process_tree）、orz-assurance 144 / 0、orz-bin 11 / 0、fmt 干净、
  clippy 无新增；ADR-0010 §14.39 第 3 项）

- [x] 全面审查处理（2026-08-24；ADR-0010 §14.39 第 4 项）：P1-1——
  read-anchor 写前核证补 direct 面落点（verify_content_anchor 解耦为
  (file_path, label) 签名，run_host_tool_with_timeout 执行前核证门，
  search_replace 声明补 expected_anchor 参数；错误锚点拒绝/正确锚点放行/
  无锚点放行/新建跳过四类 direct 测试）；P1-2——三个退役 console 工具
  （blackboard_action_write/console_step_done/console_return_to_console）
  调用面窄门拒绝（retired_tool_denied、零 ToolStarted、零订单事件、零副作用）；
  P2-1——cmd: 审计摘要格式修复（括号配对 + stdout 截断机械标记替代文件
  delta 顶替）；P2-2——候选拒绝结构化错误码透传（candidate_cap_exceeded /
  candidate_count_unbound 精确异常分类）；测试迁移——21 项已退役订单链
  e2e 删除、acp_server/codex_app/tui 8 处订单脚本迁移为 direct 工具调用、
  订单层 FUS-READ-ANCHOR 四场景保留为休眠路径验证。验证：orz-loop 551
  通过 / 0 失败 / 3 ignored、orz-tui 178 / 0、orz-host 串行 220 / 0（并行仅
  预存 flake call_tool_timeout_kills_process_tree，单跑通过）、fmt 干净）

- [x] S3 重建（2026-08-24；Linux musl，ORZ-BUILD-MOUNT-001 契约，
  build_orz_aliyun.sh，rust:1.97-slim 增量构建 -j 1）：BUILD_EXIT=0；
  三件套 2026-08-24 06:15 HKT（orz 104,718,240 B / signer 1,388,592 B /
  provision 1,206,576 B），编译 6m47s；最小可执行冒烟=三件正常加载执行
  （orz 无 TTY io error 属预期、provision usage、signer manifest 缺失）；
  守卫符号 retired_tool_denied/content_anchor_mismatch 各 8 命中；
  musl 静态确认；对应源码 orz 033fd26 + 父 54560b4（均已推送）；
  ADR-0010 §14.39 第 5 项；二次构建轮 s3b（2026-08-24 08:19 HKT，
  reasoning_content 修复 orz 4ca60c2 + 父 c1ce4c1，三件套 orz
  104,726,432 B，编译 5m04s，冒烟同前））

- [x] S4 复验闭环（2026-08-25 补登记；2026-08-24 sweep-mal-s4 实机 5
  题 k=1 全过 reward 1.00、零异常、48m06s；5/5 journal 事件链完整性
  100%、零 fail 事件、[MECHANICAL_AUDIT] 报告注入齐全（22–106 处）；
  检索可达项该轮暴露缺口——由 0h S4 单题实测补足（2026-08-25
  mteb-leaderboard：web_search×19 / web_fetch×10、降级 transition
  落盘、reward 1.00）；计数 31 → 30；ADR-0010 §14.39 第 6 项）

### P0-0h RETRIEVAL-SUBAGENT-WIRING（P0；2026-08-25 设计定稿；设计轮
不动计数；S1 实施 + S2 测试 + 全面审查处理 + S3 重建已完成；S4 复验闭环
2026-08-25；S1-S4 全部闭合）

> 入口：[设计](docs/RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md)；
> ADR-0010 §14.40（v1.40）；BACKLOG 0h。
> 来源：MECHANICAL-AUDIT-LAYER S4 复验（2026-08-24）主模型工具面 13
> 工具、检索族零可见——harness 不传 `--retrieval-mode`、会话停默认 off
> （mode=off 投影剔除检索族）；内部子代理触发工具 `retrieve_project_docs`/
> `retrieve_project_source_ledger` 从未注册（仅 relay 路由与测试引用）。
> 定案（2026-08-25 用户裁决）：外部子代理=模式 A 自动定档——local_browser
> probe 失败（browser_launch_failed）机械降级 framework_fallback 并记
> `retrieval_mode_transition`（取代 §3.7.1 禁止自动切换）；页面级失败
> （§3.7.2 显式状态）不降级；工具面定档=浏览器可用只有 browser_read、
> 不可用只有 web 族；TB harness PUBLIC 时传 `--retrieval-mode
> local_browser`。内部子代理重新定位=结构化检索外包（主代理点读保留；
> 多文件/跨目录调研打包派发，`[DOC]` 结构化结果+ledger 回传）；controller
> 声明面注册 `retrieve_project_docs` ToolDef，内部 lane 工具面仅读族。
> prompt 以框架使用提示（tips，≤1 句）告知使用方式，模型自主决定。
> 实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（构造题：检索调用出现、
> 降级 transition、零 400、命中率 ≥90%、reward 不降）。设计轮不动计数。

- [x] 设计定稿（2026-08-25；登记于 ADR-0010 §14.40 / BACKLOG 0h /
  CLI_PROJECT_INDEX v2.21）

- [x] S1 代码（2026-08-25；A 档降级 + 内部工具注册 + prompt tips +
  harness 传参——apply_mode_a_auto_degrade + transition 元数据、
  retrieve_project_docs 声明、subagent_tool_projection role 过滤、
  prompt ≤1 句 tips、orz.py --retrieval-mode local_browser；
  orz-loop 556 / orz-host 222 / orz-tui 178 / orz-assurance 144 /
  orz-bin 11 全绿、fmt 干净、clippy 无新增）

- [x] S2 测试（2026-08-25；新增 5 项专项测试：模式 A 降级规则、
  降级 transition 元数据、主面 retrieve_project_docs 声明、
  主车道内部派发、内部 lane 投影仅读族；全量全绿）

- [x] 全面审查处理（2026-08-25；P0：CLI 运行路径接通检索模式——共享
  probe 入口 `probe_retrieval_with_mode_a`（ACP/CLI 共用）+ `run()` 接线
  + transition 落盘；P1：`scope`/`max_results` 机械并入检索任务契约；
  P1：工具面跟随模式 A 定档（local_browser 隐藏 web 族、
  framework_fallback 隐藏 browser_read、外部 lane browser_read 恢复按
  模式门控）；收紧 `browser_launch_failed` 前缀；降级元数据跨 run 持久化；
  新增 4 项专项测试；orz-loop 558 / orz-host 223 / orz-tui 178 /
  orz-assurance 144 / orz-bin 11 + 集成全绿、fmt 干净、clippy 无新增；
  登记于 ADR-0010 §14.40 第 4 项）

- [x] S3 重建（2026-08-25；Linux musl 三件套，ORZ-BUILD-MOUNT-001
  契约，build_orz_aliyun.sh -j 1，BUILD_EXIT=0；三件套 02:46 HKT
  （orz 104,796,704 B / orz-signer 1,388,496 B / orz-acaf-provision
  1,206,480 B），编译 6m10s；冒烟=三件正常加载执行、守卫符号
  retired_tool_denied / content_anchor_mismatch 各 8 命中、新检索
  接线符号在二进制内、musl 静态；对应源码 orz f4f1b81 + 父 48cb030）

- [x] S4 复验闭环（2026-08-25；单道检索题实机 mteb-leaderboard k=1：
  reward 1.00、零异常、33m23s；检索调用出现（web_search×19 /
  web_fetch×10 + 外部子代理结果侧车 2 份）；降级 transition 落盘
  （local_browser→framework_fallback、mechanical_probe /
  browser_launch_failed）；零 400 / 零 tool_failed / 零
  run_invalidated；journal 事件链完整性 100%（619/619，命中率 ≥90%）；
  终答 GritLM/GritLM-7B 通过 verifier；DoD 1-7 全部满足；计数 30 → 29；
  ADR-0010 §14.40 第 6 项）

### P0-0i FINAL-SMOKE-2026-08-25 对拍暴露问题（P0；2026-08-25 登记；
正式 89 题提交前处理）

> 入口：索引 GAP-EVENT-SCHEMA-DRIFT / GAP-REPETITION-DETECTOR-DNA-
> FALSE-POSITIVE；BACKLOG 0i。
> 来源：5 题 k=1 官方标准冒烟（final-smoke-2026-08-25）确定性对拍——
> 2/5 reward 1.00（llm-inference-batching-scheduler、sam-cell-seg），
> 3/5 官方墙钟超时（make-doom 900s、dna/feal 1800s）；机制面零 400、
> 零 run_failed/run_invalidated、ACAF 票据 50/50、submit 双阶段走通；
> 严格校验 5/5 未过（哈希链完好），dna run 复读误杀降级 disabled。

- [x] 登记（2026-08-25）：BACKLOG 0i / TODO P0-0i / CLI_PROJECT_INDEX
  两个 GAP canonical entry；未闭合 29 → 31。
- [x] 事件面 Schema 漂移修复（GAP-EVENT-SCHEMA-DRIFT；**2026-08-26 完成**）：
    - [x] retrieval_mode_transition 枚举补 mechanical_probe /
      browser_launch_failed（runtime schema v0.2 authority/reason_code
      enum + description 登记 + 模式 A 降级正例 fixture 锁）
    - [x] ledger_fold_advance 补 view_estimate_after（按设计定案=生产者
      保留、schema 补必填字段 + verifier 交叉检查
      view_estimate_after < view_estimate_tokens + 正/反例 fixture 更新）
    - [x] control_ticket_issued 动作票 activation_id 放开检索 lane
      绑定（D-13 语义入 schema：动作票可选绑定，检索 lane network 票
      携带真实 activation_id、主 lane 保持 null；正例 fixture + 测试
      语义更新）
    - [x] 事件链复验：5 个冒烟 run 严格校验 0 错误（三类漂移全部清零；
      3 个墙钟超时 run 缺终止事件为 harness 杀进程边界、按
      require_terminal=false 回放语义豁免）+ 全量测试全绿（runtime/
      assurance pytest 1898 passed、check_repository valid）
- [ ] 复读检测 DNA 误杀处理（GAP-REPETITION-DETECTOR-DNA-FALSE-POSITIVE）：
  - [x] 设计定案（2026-08-25 用户裁决：L=400 维持 + 序列内容门——无
    切分点路径 sequence_like 判定（ACGTNU 占比 ≥0.90）+ 序列族命中门槛
    3→5；否决 L=1000 提档与整体走路径②；ADR-0010 §14.41 /
    REPETITION_DETECTOR_SEQUENCE_CONTENT_GATE_DESIGN）
  - [x] S1 代码（2026-08-25 实施放行 31 → 32：新常量
    REPETITION_SEQUENCE_LIKE_RATIO=0.90 / REPETITION_SEQUENCE_HIT_LIMIT=5、
    sequence_like 判定（整数比较）、无切分点分支三态确认分派
    （Confirmed/SequenceGated/Rejected）、序列族流内命中分别计数、
    sequence_gated 审计标注（ratio+hits/limit）与 WARN
    sequence_hit_limit 字段、双门槛 feed 封顶；transport.rs，
    orz-loop 563/0/3 全绿、fmt 干净、clippy 无新增；既有 'a'/'t'
    重复字符测试按序列门语义更新（'x' 保持非序列 3 门槛回归））
  - [x] S2 测试（2026-08-26 完成：final-smoke dna EGFP 400 字符真实
    span 原样回放——4 段引用=3 次命中 0 trip、6 段=5 次命中触发，
    审计带 sequence_gated/ratio/hits-limit 且按命中时刻累计标注；poly-A
    399/400/401 静默；序列族间隔不重置 + 流结束丢弃（新建流重新计数、
    同流 5 次触发）；路径②/stall/标点块（sam G4 真实 span）/混合分别
    计数/占比边界回归全绿；orz-loop 567/0/3、fmt 干净、clippy 无新增）
  - [x] S1 全面审查处理（2026-08-26：蛋白/氨基酸序列覆盖扩展——严格
    20 氨基酸字母表 + 独立阈值 REPETITION_PROTEIN_LIKE_RATIO=0.95 +
    sequence_kind 单次扫描双族判定 + 审计/触发 detail 带 kind
    （dna_rna/protein）；覆盖面实证=英文无间隔长串蛋白占比 0.8975
    （否决蛋白共用 0.90）、真实蛋白 UniProt P35579 1.00；阈值百分比
    提为编译期常量、同 delta 多次命中按累计时刻标注、设计 §5 数字非
    切分符勘误；新增蛋白真实 span 3 次仅审计/5 次触发 + 英文守卫 +
    0.94/0.95/0.96 边界测试；orz-loop 569/0/3、fmt 干净、clippy 无新增）
    - [x] S3 重建（Linux musl，ORZ-BUILD-MOUNT-001 契约）——**2026-08-26
      完成**：Docker Desktop 恢复后 rust:1.97-slim 容器构建 6m19s，三件套
      orz 104,796,752 B / orz-signer 1,388,496 B / orz-acaf-provision
      1,206,480 B（2026-08-26 01:47）；守卫符号 retired_tool_denied /
      content_anchor_mismatch 各 8 命中、检索接线符号（browser_launch_
      failed/retrieval-mode/retrieve_project_docs）与序列门审计字段
      （sequence_gated/dna_rna/protein）在二进制内、无 PT_INTERP（musl
      静态）；容器冒烟=provision usage / signer manifest 缺失 / orz TTY
      io error 均符合预期；对应源码 orz 6b208fb（工作树干净、已推 cli
      远端）。
  - [x] S4 复验（dna-assembly 重跑零误杀降级、真复读仍触发、零 400、
    命中率 ≥90%）——**2026-08-26 闭环（未闭合 30 → 29）**：新二进制
    orz 6b208fb（104,796,752 B）k=1 官方标准重跑（s4-dna-2026-08-26 /
    dna-assembly__3EyUPDL，1800s 墙钟超时 reward 0.0，与旧 run 结局类别
    一致）——EGFP 式合法引用零误杀降级（1–4/5 次命中全部仅审计
    sequence_gated kind=dna_rna，旧 3/3 即直降 Disabled）；真复读仍触发
    （同一 DNA span 5/5 次命中 trip，但只降 EnabledLow、非旧实现的直降
    Disabled，降级后继续工作）；零真实 400（事件链 8 处“400”均为哈希
    串）；事件链严格校验除「缺终止事件」1 项豁免（墙钟超时 harness 杀
    进程边界，require_terminal=false 回放语义；与旧 run 同构）外 0 错误；
    journal 口径命中率 82.36%（22 请求，762,496/925,805）vs 旧 82.33%——
    持平，web 检索注入相关，观察项不阻塞；请求 36→22、reasoning tokens
    46,626→136,956、request_header_change 6→3、无 reasoning_stall 触发。
- [x] 观察项登记（dna 82.36%（S4 重跑实测）/ feal 88.21% 命中率 <90%
  与 web 检索注入相关，成本观察不阻塞；feal 一次 reasoning_stall 64K
  预算设计内触发；S4 dna 重跑无 stall 触发）——并入对拍审计记录。

### P0-0j THIN-HARNESS-REDESIGN-V2（P0；2026-08-28 设计定稿，实施待放行）

> 入口：[设计](docs/THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md) /
> [HA 调研](docs/HA_SERVICE_MODEL_RESEARCH_2026-08-28.md)；BACKLOG 0j。
> 定案摘要（用户裁决）：复读门槛统一 20 + 序列门全删 + 3-gram 15 + 802 保留
> + 空响应链 low 封顶 + 触发显式拦截不降档 + 审计结构化字段判定 + 半助理层
> 加厚（诊断/实体/黑板）+ HA 服务模型（target=实体级）。设计轮不动计数（29）。
> **2026-08-29 定案（S4 失败归因后，用户裁决）**：前 20 道错题 k=1 重跑
> 2/20，归因五类（submit 门死锁 / verifier 环境错误 / 真实交付质量 /
> 提前收束 / 超时）；提前收束根因=orientation 硬门文本残留 + R1 无头接线
> （50 轮纯文本回答被当终答）。定案：prompt 全空 + orientation 软门 +
> submit 门无 plan 放行/降级（W4-R4 实施批次，见下）。
> **2026-08-29 超时定案（§9.5 B1/B2 深挖后，用户裁决）**：终端命令分层
> 默认超时（普通 300s / 程序脚本 600s）+ 满 300s 单次中间回报（默认继续、
> 可中断，模型自行判断）+ web_search 客户端总超时 120s（connect 10s、
> 结构化错误、不自动重试）。实施分两批（压力评估定案）：S5-1=A/B 修复 +
> web_search 超时（轻量同批）；S5-2=终端超时 + 中间回报（新机制独立批，
> 先补设计小节定案）。

- [x] W1-R1 S1 代码：复读门槛 20 统一 + 序列内容门全删 + 3-gram 15 +
  802 保留回归（改独立同字符触发线）+ 空响应链 low 封顶 + 触发改显式
  拦截不降档（移除 `session_thinking` 降级分支）+ 审计只消费结构化字段。
  **2026-08-28 完成**：transport.rs 常量/判定/触发全量落地，orz-loop
  `cargo check`/`fmt` 通过、clippy 无新增告警。
- [x] W1-R1 S2 测试：19/20 命中边界、3-gram 14/15、801/802、空响应链
  low 封顶、run_invalidated 达限、trip 后 thinking 档位不变、含"400"
  哈希串 fixture 零误报 + 真实 400 事件精确报出。**2026-08-28 完成**：
  transport.rs 91 项 + orz-loop lib 全量 567 项通过；事件链校验器新增
  `StructuredAuditFieldConsumptionTests`（216 项全绿）。
- [x] W1-R1 S3 重建（Linux musl 三件套，ORZ-BUILD-MOUNT-001 契约）
  **2026-08-28 完成**：rust:1.97-slim 容器构建 5m28s，Linux musl 三件套
  orz 105,025,280 B / orz-signer 1,388,744 B / orz-acaf-provision
  1,206,728 B（07:00 HKT，日志 D:\tb-eval\build-20260828-s3.log）；
  R1/R2 关键符号（target_mismatch×2 / target_missing /
  target_type_mismatch / tail_is_raw×3 / not_executable /
  diagnose_failure / blackboard_read×30）在二进制内、ldd 静态链接（无
  PT_INTERP）；容器冒烟=provision usage / signer manifest 缺失 / orz
  TTY io error 均符合预期；对应源码 orz 6cc8586 + 父 a21fcd1；警告面
  14 项与上次基线持平；S4 复验待实施。
- [ ] W1-R1 S4 复验：EGFP / sam-cell-seg 真实 span 回放静默、构造真循环
  触发、零真实 400、命中率 ≥90%
- [x] W2-R2 失败诊断：`diagnostics.diagnose` 服务 + 各工具域结构化签名词典
  （file/process/environment）+ ≤2KB 极简记录 + 执行失败自动派发。
  **2026-08-28 完成**：`diagnostics.rs` 签名词典（terminal/file/process/
  environment 11 签名，P5 只消费结构化信号）+ `diagnose_failure`（≤2KB
  序列化上限 / tail ≤12 行 / key_fields ≤8）+ 执行失败自动派发
  （`run_console_target`：非零退出/超时/工具不可用/委托错误 → 极简记录
  附着失败对象实体 + 错误信封 `upstream.detail.diagnostic`）。
- [x] W2-R2 实体登记与状态：process / file / environment 三域（锚点 hash、
  pid/exit_code、环境可用性），半助理层执行工具时登记/更新。
  **2026-08-28 完成**：`entities.rs` 实体注册表（file 锚点 size/sha256/
  mtime/encoding、process exit_code/timed_out/status、environment shell+
  探针工具可用性；总容量 128 / 渲染上限 24）+ `host::ToolResult.
  tool_error_kind` 结构化透传 + `output_encoding` 重建透传修复。
- [x] W2-R2 服务调用形态收敛：`domain.service + target(实体级) + data`，
  无作用对象省略 target；与现有操作台订单格式合并。
  **2026-08-28 完成**：`ActionSpec.target_policy`（None/File/Process/
  Environment/AnyEntity）+ `ActionOrder.target` 实体 id + 发放前机械校验
  （target_required / target_not_allowed）+ PTC 脚本步骤 target 透传；
  file 四服务声明 File 策略，diagnostics.diagnose 声明 AnyEntity。
- [x] W2-R2 黑板接线：实体状态分区渲染（条目/总上限）+ `blackboard_read`
  工具描述简定义（黑板=框架状态区…）+ 返回面纪律（摘要+指针+上限）。
  **2026-08-28 完成**：Blackboard `entities` 分区（live-only，不进 epoch
  快照）+ `blackboard_read section=entities` 有界文本渲染（摘要清单 +
  总上限 + has_diagnostic）+ 工具描述补简定义与 entities 分区枚举。
- [x] W2-R2 全面审查处理（2026-08-28，三路并行审查 + 主 Agent 复核）：
  **P1-1** file 域签名匹配改消费 stat 探针信号（exists/kind + 参数键推断
  期望类型）——`target_missing` 仅 exists=false 命中、`target_type_mismatch`/
  `not_executable` 恢复可达、目标存在的失败落 raw 兜底；**P1-2** target↔data
  二选一 + 双写一致性校验 + 域前缀校验（`step=target`/`code=target_mismatch`
  拒单），契约校验移至 target 解析之后；grep 改 `FileOptional`（无 path
  工作区搜索免 target）；timed_out 优先于 command_not_found；诊断信封补
  `tail_is_raw`、process 域 `target_state`（command/status）；ErrorKind 与
  `host::ToolErrorKind` 统一（删手写映射）；实体渲染补总字节上限、
  `file_entity_id` 无状态归一（分隔符/`./` 前缀）、16MB 哈希不变式 registry
  层强制（目录跳过 read）；run_tests 特例 `output_encoding` 透传；P3 注释/
  设计文档措辞勘误（§3.3/§4.2/§4.5/§8）。测试补 16 项（file 域正反例、
  126/type_mismatch 可达性、127+timed_out、PTC target 透传、渲染字节上限、
  16MB 边界等）。**验证**：orz-loop 608/0/3、pytest 216、fmt/clippy 干净。
- [ ] W3-R3 HA 目标架构落地余项：实体 id 形态、分区命名、注册表 Rust 形态
  （含实体 id 相对/绝对/大小写归一——本轮仅落地无状态部分：分隔符与
  `./` 前缀；process/environment 状态探针扩展入账 R3）
- [ ] W3-R3 A/B 验证：小样本跑分（含 THIN-HARNESS v0.4 R2b 按需读取观察 /
  R2c `parse_retrieval_result_json` 兜底回收判定）
- [ ] W3-R3 清理与登记：旧序列门文档标记 withdrawn、ADR-0010 减法修订、
  CLI_PROJECT_INDEX 登记（含 BACKLOG/TODO 计数入账）
- [x] W4-R4 prompt 全空：BASE_SYSTEM_PROMPT 置空（含"工具按需使用"与
  submit 引导行），near-zero 测试反转断言空；契约确认已由工具描述/信封/
  机械门承载（read_file offset / search_replace 锚点 / blackboard_read
  分区 / submit 两阶段）。**2026-08-29 完成**：prompt.rs 置空 +
  `build_system_prompt` 简化为仅返回探针块 + 测试反转
  （`base_system_prompt_is_empty`）。
- [x] W4-R4 orientation 软门：块文本改简短方向检查（去强制模板措辞）、
  触发轮不禁工具、纯文本回答消费后续跑、终答只由模型自发；阈值 50 维持；
  强制模板轮代码保留不启用。**2026-08-29 完成**：`ORIENTATION_BLOCK`
  v0.4（三问方向检查，去"只输出 JSON 模板/不要调用任何工具"）；主车道
  fire 延迟 commit + pending 轮软消费（纯文本消费续跑、工具调用照常执行、
  无 checkpoint_response）；检索车道维持 fire-and-continue；
  `force_template_round` 休眠参数保留；DC 强制模板轮机制不变。
- [x] W4-R4 submit 门修复：无 plan 会话放行/降级为状态展示 + submit 工具
  描述清 plan 措辞 + 回归测试。**2026-08-29 完成**：`submit_no_plan`
  拒绝路径删除（无 plan 渲染交付状态、两阶段 requested→confirmed、不推进
  计划步骤）；描述去 plan 措辞（"requires no plan and no step state"）；
  新增 `submit_no_plan_renders_status_and_confirms` 回归测试。
- [ ] W4-R4 S2 测试 + S3 重建 + S4 复验：错题重跑回归——提交不再被拒、
  长任务不再 50 轮提前收束、命中率对比 official-r1（58/89）。
  **2026-08-29 S2 完成**：orz-loop 611/0/3（新增 1）、orz-assurance
  144/0、orz-bin bins 14/0、pytest 216、fmt 干净、clippy 无新增；
  **审查处理（2026-08-29）**：无 plan 确认文案不虚构机械最终回答流程
  （抽 `submit_confirm_message` + 单测）、孤儿 fire 边界注释（代码 +
  设计 §9.2）、`TEMPLATE_ANSWER_INSTRUCTIONS` 零引用注释、设计 §9.4
  状态翻转。
  **S3 重建（2026-08-29 完成）**：rust:1.97-slim 容器构建 6m25s，
  Linux musl 三件套 orz 105,020,808 B / orz-signer 1,388,744 B /
  orz-acaf-provision 1,206,728 B（07:33 HKT，日志
  D:\tb-eval\build-20260829-s3.log）；容器冒烟（bookworm）：provision
  usage / signer manifest 缺失 / orz TTY io error 均符合预期、三件套
  ldd 静态链接；W4-R4 关键标记（ORIENTATION v0.4 / 方向检查 /
  无计划批准基线 / requires no plan and no step state）在二进制内；
  警告面 14 项与上次基线持平；对应源码 orz 5b3fe27 + 父 10c99f0；
  S4 复验待实施。
- [ ] W4-R4 S4 复验（2026-08-29 完成，31 题全量 k=1，harbor 实机）：错题
  重跑 8/31 解出（official-r1 此 31 题全 0）——query-optimize /
  pytorch-model-cli / largest-eigenval / caffe-cifar-10 / circuit-fibsqrt /
  mteb-retrieve / video-processing / build-pov-ray；c1/c2 同批对照旧
  R2 二进制 2/20 → 4/20（旧解 model-extraction-relu-logits /
  protein-assembly 单样本回落 0）；submit 门全链零 `no plan in force`
  拒绝、真实 HTTP 400 零、5 个硬门题全部越过 50 轮（chess 74 / make-doom
  103 / make-mips 75 / caffe 79 / gcode 52，caffe-cifar-10 因此解出）、
  path-tracing 55 步无 orientation 循环（AgentTimeout 属 harness 按
  官方 task.toml agent.timeout_sec 掐的墙钟超时，与 orz 检索超时无关）。
  **S4 新暴露两问题（S5-1 已实施修复，2026-08-29）**：A=fold 桥剥
  reasoning_content
  （`action_ledger.rs build_bridge` `m.reasoning_content = None`）→
  DeepSeek 400（make-doom 103 步 / gcode-to-text 52 步，均为 orientation
  纯文本回答落入桥内末条被剥；同 run 纯工具轮 fold 与 video-processing
  不触发，机制边界已实证）；B=orientation 触发轮工具面仍为空
  （`agent_loop.rs` `pending_checkpoint.is_some() → Vec::new()`），与
  设计 §9.2「触发轮不禁工具」不符（触发轮模型把 XML 工具调用写成纯
  文本，浪费一轮真实工作）。记录于设计 §9.2/§9.4；修复与补跑见下。
- [x] W4-R4 S5 修复（A/B；2026-08-29 S5-1 实施 + 全面审查处理完成，
  orz ad5f9ee）：A=桥内保留纯文本 assistant 消息 reasoning_content
  （声明仍剥，双形态回归测试 `bridge_keeps_plain_text_assistant_reasoning_but_strips_declarations`）；
  B=orientation pending 放行工具、DC 强制模板轮保留禁工具（拆两条路径，
  回归测试 `orientation_trigger_round_keeps_tool_face_projected`）。
  原四题补跑批次（make-doom / gcode-to-text / chess-best-move /
  make-mips-interpreter，official-r2-failures-s5-1）**作废重排**
  （2026-08-29 用户指示：不再沿用旧批次，全部处理完成后重新安排补跑）；
  R3 纪律项随收口执行（ADR-0010 §14.42 已补写，CLI_PROJECT_INDEX
  登记含 S4 入账与计数）。
- [x] W4-R4 S5-1 超时（web_search 120s，与 A/B 同批；2026-08-29 实施 +
  全面审查处理完成）——`web_search/client.rs` reqwest builder 补总超时
  120s + connect 10s；超时经 `map_transport_error` 返回结构化 Timeout
  错误（已用时长 + 建议重试/换查询/直读页面；审查处理补阶段上下文/连接
  超时措辞与两条单测），不新增自动重试；事件面 `tool_completed` 补
  wall_ms 与超时标记（S5-1 已落：schema + 通用/run_tests 路径；审查
  处理 P2-1 补 run_tests F-09 超时结构化 `TestRunResult.timed_out`
  端到端透传）。设计登记：设计 §9.6 / 调研笔记 §5。
- [ ] W4-R4 S5-2 终端分层超时 + 中间回报（2026-08-29 用户裁决，独立批）——
  普通命令默认 300s / 程序脚本类 600s（两档；模型可传 timeout 覆盖、
  上限维持 900s）；任何命令运行满 300s 未完成 → 机械插入一次「运行 +
  工具自身情况」中间状态（单次仅一次，不累积/不周期），回报后默认继续
  等待、模型可主动中断（kill），由模型自行判断；完成/超时后正常返回
  终态。机制定案已落：设计 §9.7 / 调研笔记 §6——分类规则=宿主按命令
  形态启发式注入默认 timeout（程序/脚本 600s、普通 300s）；后台路径=
  复用终端 actor 自动后台化（满 300s 且解析超时 >300s 才后台化，
  后台截止=原解析超时，终态经既有完成提醒送达，工具面封闭）；
  事件面=`tool_running`（v0.2）+ ToolCompleted `running: true`。
- [x] W4-R4 S5-2 S1 代码：宿主分类注入 + 自动后台化报告 + actor 后台
  截止期 + `tool_running` 事件面（schema/verifier/fixtures）
  **2026-08-29 完成**：`terminal_tier_default_timeout_ms`（程序 600s /
  普通 300s 启发式）+ `inject_terminal_default_timeout`（模型未传才注入）、
  `BashParams.hide_background_input`（schema 隐藏/调用拒绝 `is_background`，
  描述与 timeout 文案同步）+ 自动后台化 `pre_formatted` 中间状态（时长/
  PID/部分输出/落盘指针/可中断提示）、actor 后台截止=原解析超时
  （`min(timeout, BACKGROUND_MAX_RUNTIME)`，`transition_to_background`
  保留原 timeout）、逐调用 auto-bg 判定（解析超时 > 有限回报点才后台化）、
  `ToolResult.mid_run` 结构化透传 + `tool_running` 事件（v0.2 schema/
  verifier 链规则/fixtures）+ ToolCompleted `running: true`；工具面保持
  「一次调用 = 一个结果」。
- [x] W4-R4 S5-2 S2 测试：分类/注入/后台化边界/中间状态内容/事件链
  **2026-08-29 完成**：orz-tools（auto-bg 报告、hide_background_input
  拒绝/schema/描述、actor 原超时截止）+ orz-host（分类/注入/mid_run
  映射 4 条）+ orz-loop（`tool_running` + `running:true` 事件链）+
  TUI 投影 + Python `_verify_v02_tool_running` 正反例 5 条；事件侧
  pytest 236、orz-loop 616、orz-host 229、orz-tools 2659（排除沙箱
  rg 基线 grep/glob）、orz-tui 178、orz-assurance 144 全绿；fmt/clippy
  无新增；manifest 1403；check_repository 仅剩 orz 子模块脏（未提交属
  预期）。
- [x] W4-R4 S5-2 全面审查处理（2026-08-29 完成）：**P1-1** console 订单
  执行边界（issue_action_inner）把 mid_run 当 ExecutionFailed 误判 +
  故障连败连锁——补 `mid_run.is_none()` 豁免 + 正反例测试；**P1-2** 机械
  审计层 classify 把 mid_run 记为 `exit -1` 虚假异常——摘要改中性「运行
  中」、anomaly=None + 测试；**P2-1** run 结束未决后台任务语义定案（保持
  运行至截止、宿主生命周期收口，设计 §9.7.2 登记）；**P2-2** 用户主动
  后台化（Ctrl+G/is_background）恢复 10h 硬上限截止、自动后台化保留原
  超时（transition_to_background 分型 + 回归测试）；**P2-3** verifier
  direct 关联规则扩展至 tool_running + 测试；**P2-4** verifier 补
  `running:true ⟹ exit_code=null` 与单完成事件断言 + 反例；**P2-5/P3-5**
  设计 §9.7 边界（compound 命令/Windows `.\` 启发式边界、终态送达依赖
  后续工具调用）；**P3-1/P3-2/P3-3/P3-4** 注释勘误、测试真实性、schema
  描述、跨 call_id/跨 run 反例。待跑受影响 crate 测试与事件侧 pytest 复
  验后进 S3。
- [x] W4-R4 S5-2 审查处理补充（2026-08-29）：**P1-3** console 订单面
  `workspace.run_terminal` 动作 schema 仍暴露 `is_background` 且 timeout
  上限 300s——与 §9.7 工具面封闭 / 模型可传上限 900s 不符（host 工具面
  已改、订单面漏）；已移除 is_background、timeout 上限 300s→900s、默认
  120s→600s、描述同步两档注入与 300s 中间回报 + 契约测试更新。
- [ ] W4-R4 S5-2 验证期发现（2026-08-29 登记，S5-1 遗留回归，独立排查）：
  orz-bin `acaf_e2e` 7 项失败（`controller_control_events_carry_tickets` /
  `fail_closed_continue_consumes_goal_revision_ticket` /
  `fail_closed_goal_revision_rejected_does_not_migrate` /
  `fail_closed_web_search_executes_unticketed_with_zero_ticket_events` /
  `goal_revision_continue_flow_re_derives_session_key` /
  `missing_browser_read_url_refuses_before_acaf_with_count_gate` /
  `signer_unreachable_shadow_records_rejection_and_proceeds`），16 通过。
  归因链：上一提交 5b3fe27（W4-R4）记录 orz-bin 14/0 全绿 → 当前工作树
  16/7；失败全部在 ACAF 控制事件/disposition/票据域（如 disposition_v1
  票据未签发、signer-unreachable 路径 `retrieval_parent_disposition` 事件
  未落），与 S5-2 审查处理改动路径（console mid_run 订单、机械审计
  run_terminal 分类、终端 actor 后台化截止、事件校验器）不相交——S5-2
  未提交 controller 生产改动仅 compact/run_tests/mid_run 三处，均不触
  disposition/票据路径。判定为 S5-1 提交（ad5f9ee：fold 桥 reasoning
  保留 / orientation 触发轮工具面 / web_search 超时）引入的遗留回归，
  需独立轮次定位（建议先核对 disposition 处理链与 orientation 触发轮
  交互）后再进 S3/S4。
- [x] CONTROLLER-SPLIT 二轮（2026-08-30 用户指示；设计 §3.5）——
  controller.rs 24,861 行（生产 7,548 / 测试 17,313），目标全文件
  ≤10,000 行。批次 N1 `acaf_flow.rs`（ACAF 票务 ~940 行，低风险）→
  N2 `host_exec.rs`（host 工具家族 ~2,600 行）→ N3 控制台订单/模式
  转换并入 `console_exec.rs`（~790 行）→ N4 类型/纯函数归位（~690 行）
  → N5 测试区按主题归位各模块（17,300 行潜力）。每批独立提交 +
  全量回归（orz-loop / orz-tui / orz-bin / orz-host / pytest 事件链），
  行为不变纪律同 B1–B8；完成一批勾选一批并同步设计 §3.5 与索引。
  - [x] N1（2026-08-30，`8f5e058`）：ACAF 票务/动作事件簇 → `acaf_flow.rs`（932 行），orz-loop 618/0/3 + pytest 1588/14 全绿。
  - [x] N2（2026-08-30，`d5cbd60`）：host 工具执行家族 → `host_exec.rs`（2,647 行），orz-loop 618/0/3 + pytest 1589/14 全绿。
  - [x] N3（2026-08-30，`f1cf0b0`）：控制台订单执行 + 模式转换并入 `console_exec.rs`（788 行），orz-loop 618/0/3 + pytest 1588/14 全绿。
  - [x] N4（2026-08-30，`13908c0`）：类型/纯函数归位（模式面 → `retrieval/mode.rs`，压缩/消息预算 → `compact.rs`，denial 状态机 → `denial.rs`），orz-loop 618/0/3 + pytest 1589/14 全绿。
  - [ ] N5 测试区按主题归位（17,300 行潜力，分批）。
    - 分批设计（2026-08-30）：N5-0 脚手架独立 → N5-1 检索模式/证据/激活 → N5-2a 检索调度/子代理 → N5-2b 投影面 → N5-3 console → N5-4 host/denial → N5-5 compact → N5-6 plan/blackboard → N5-7 收尾；设计详见 §3.5。
    - [x] N5-0（2026-08-30，`541c99b`）：共享脚手架独立为 `controller_test_support.rs`（349 行），controller.rs 20,264 → 19,935 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-1（2026-08-30，`74ef451`）：检索模式/证据/激活 22 项 → `retrieval/mode.rs` + `evidence.rs` + `activation.rs`，controller.rs 18,533 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-2a（2026-08-30，`fb8682c`）：检索调度/子代理/候选门 34 项 → `retrieval/dispatch.rs`，controller.rs 15,798 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-2b（2026-08-30，`0e4f4ff`）：投影面 12 项 → `retrieval/projection.rs`，controller.rs 15,065 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-3（2026-08-30，`693c260`）：控制台簇 24 项 → `console_exec.rs`（denial 单测 → `denial.rs`），controller.rs 12,820 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-4（2026-08-30，`00f298a`）：host 工具/权限/run_tests 37 项 → `host_exec.rs`，controller.rs 10,039 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-5（2026-08-30，`e828299`）：压缩/折叠/白名单 21 项 → `compact.rs` + `summary.rs`，controller.rs 8,033 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-6（2026-08-30，`52ce3f5`）：plan/epoch/plan_first 15 项 → `planning.rs` + 黑板上读 16 项 → `blackboard.rs`，controller.rs 5,632 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
    - [x] N5-7（2026-08-30，`d6f771b`）：收尾归位（delivery/dc/orientation/checkpoint/probe/mechanical_audit，含 final_answer 误入纠正），controller.rs 4,142 行，orz-loop 618/0/3 + pytest 1589/14 全绿。
  - N5 全部闭合（2026-08-30）：controller.rs 29,091 → 4,142 行，测试区 202 项归位、留守 19 项主循环测试；验收 ≤10,000 行达成。

### P0-0k RETRIEVAL-ORCHESTRATION-MECHANICAL（P0；2026-08-30 检索问题最终评判定稿，实施待放行）

> 入口：[调研 §8/§9](docs/RETRIEVAL_AND_SUBAGENT_COMMUNICATION_BORROW_RESEARCH_2026-08-30.md) /
> [双检索验证](docs/DUAL_RETRIEVAL_MODE_VALIDATION_DESIGN_2026-08-29.md) /
> [controller 拆分](docs/CONTROLLER_SPLIT_DESIGN_2026-08-29.md)；BACKLOG 0k。
> 定案摘要（2026-08-30 最终评判，用户确认无异议）：双模式保留（引擎 SERP
> Google 主序 + 原生 web_search 机械兜底）；SERP 页面级失败显式降级原生兜底；
> 人化输入延迟机械层（模型不可见）；不做全量 A/B（Google 门禁观察实验先行）；
> 同轮读类并行批准（写类串行）；web_search 并发维持 1；第一批五项 + 第二批
> 三项 + 暂缓/否决清单；顺序纪律=先闭合 S5-2 S3/S4 + acaf_e2e 7 项回归再落
> 检索批次。设计轮不动计数（仍 31）；实施放行入账 +1；S4 闭环 -1。

**前置（顺序纪律；2026-08-30 用户裁决修订）**：S5-2 S3/S4 形式化重建/复验
**暂停不做**（用户裁决：本项目是完全工程化项目、非 LIF 主项目，不要求逐步
严格可比；当前问题属于"没修完"，修完即闭环，不跑仪式化 S3/S4）。acaf_e2e
7 项回归经核验在当前 HEAD（d6f771b）已 23/23 全绿（拆分批次中 S5-2 审查
处理代码随 B1 落盘后自然解除，2026-08-30 实测），"没修完"闭环。基线核验
通过后直接落第一批。

**第一批（收口排序；第 1–4 项 S1 代码 + S2 测试完成 2026-08-30；汇总 S3
重建 + 实机验证待续）**：
- [x] 同轮读类/检索类工具并行（`agent_loop.rs` 工具批次执行处；
  FuturesUnordered，写类保持串行；web_search 信号量维持 1）。
  **2026-08-30 完成**：批首 ≥2 连续只读 Host 调用并发执行、按声明序提交
  （EventWriter buffered 模式 + 预算超限按串行语义拒绝）；并发峰值与事件/
  消息序专项测试。orz-loop 623/0/3 全绿。
- [x] 子代理 run 级预算/超时（`retrieval/dispatch.rs`；轮数 + 墙钟双层）。
  **2026-08-30 完成**：默认墙钟 600s（`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS`
  可配，0 禁用）+ 独立轮数上限 60（`ORZ_RETRIEVAL_MAX_TOOL_ROUNDS`，与主
  车道取 min）；超时经 `subagent_timeout` 收口、父 run 继续；解析规则 +
  超时收口测试。orz-loop 623/0/3 全绿。
- [x] [DOC] 回传机械截断（结构化头部 + evidence 指针）。
  **2026-08-30 完成**：声明行行数上限 16 + 字节上限 8K（逐条装填），截断
  附结构化标注；blackboard 分区保留全文（留痕不变），ledger/inline 通道用
  有界视图；单测覆盖行数/字节/原样路径。orz-loop 623/0/3 全绿。
- [x] 浏览器容器参数 + 资源拦截 + 等待语义（`--no-sandbox` /
  `--disable-dev-shm-usage` / `--disable-gpu`；`Network.setBlockedURLs`；
  等待「可用文本就绪」替代 loadEventFired，full 模式保持既有终态）。
  **2026-08-30 完成**：容器三件套 + 文本读取资源优化参数；ReadMode 下传
  CDP 层，preview/keywords 拦截图片/字体/媒体 + 文本就绪轮询（full 保持
  loadEventFired）；ALLOWED_CDP_METHODS 精确集合同步；text-ready 返回/超时
  测试。orz-host 231/0/4 全绿。
- [x] 第 1–4 项 S1 全面审查处理轮（2026-08-30 完成；子代理三路审查 +
  本人逐条核验，无 P0）：
  - **P1（schema 漂移）**：`retrieval_close_record.terminal_reason` 枚举
    补 `subagent_timeout`（本批子代理墙钟超时收口产生）+ `auto_close`
    （R1 每次派发即闭环，既有漂移同批修复）——真实超时 run 的 journal
    此前会经严格 jsonschema 校验失败（GAP-EVENT-SCHEMA-DRIFT 同类，
    pytest 全绿因 fixtures 仅 normal_close + verifier 对非 normal_close
    不交叉检查）；schema 补两值 + 两个正例 fixture（subagent-timeout /
    auto-close）+ check_repository 映射 + 独立防回归测试
    （`runtime/tests/test_retrieval_close_reason_enum.py`，不混入 S5-1
    遗留测试文件）。
  - **P2-1（黑板分区序）登记接受**：并行批次下 `tool_actions`/`exec`
    分区为完成序、事件链为声明序——记录带真实 timestamp、审计权威以
    事件链为准，分区展示序差异不补写（agent_loop 注释登记）。
  - **P2-2（候选硬 cap 并发超限）**：`candidate_gate` 决策+预留原子化
    （锁内检查 cap 并占位，消除并行批次「决策/提交分离」竞态）；permission
    /ACAF 门拒绝时 `rollback_candidate` 回滚（保持「被拒调用不消耗候选」
    语义）；测试改 `candidate_gate_reserves_atomically_and_rollback_releases`。
  - **P2-3（预算拒绝审计语义）**：并行批次预算拒绝时事件按声明序重放
    （已执行调用留痕 ToolStarted/ToolCompleted）、后处理（direct trace/
    evidence/DC/机械审查）照常，消息面仍按串行语义注入拒绝消息、结果不
    计入注入预算——审计面与事实一致。
  - **P2-4（text-ready 断连忙循环）**：事件通道关闭（recv None）→ 显式
    `CDP event channel closed`；evaluate 无内部超时（写半开挂起）→
    `remaining` 包裹兜底为 LoadTimeout；测试
    `text_ready_channel_close_is_bounded_not_hang`。
  - **P3 组**：批次启动前 cancel 检查；批次内 Err 延迟传播（已执行调用
    仍提交留痕）；子代理超时孤儿 ToolStarted 经 in-flight 槽补合成
    ToolCompleted(error)（SharedLoopServices.in_flight_tools，仅串行路径
    入槽）；轮数 env `0`=禁用独立上限（与墙钟 0=禁用对齐）；`*.png*`
    通配带 query 命中；`--no-sandbox` 仅 headless（桌面保留沙箱）；标注
    字节口径登记（8K 只约束声明行，prose/标注不计入）；`round_inject_tokens
    ==0` 冗余条件清理；tool_rounds_used 超时路径注释登记。
  - 验证：orz-loop 623/0/3、orz-host 232/0/4（新增 text-ready 断连测试）、
    runtime pytest 18+（conformance 全绿）、check_repository 除 orz 源码
    清单（提交后重生成）外零错误、fmt 干净、clippy 无新增告警。
- [x] Google 门禁观察实验 + 实机验证（小批检索密集题 k=1；job
  `gate-google-20260830-1`，D:\tb-eval\jobs-gate；报告
  `docs/GATE_GOOGLE_OBSERVATION_2026-08-30.md`）——**2026-08-30 完成**：
  5/5 容器 chromium 注入（2×apt Chromium 151 + 3×官方快照 Chromium 154，
  agent_setup 1.7–4.6min），全部 run local_browser 定档 available、零
  browser_launch_failed。通过率 2/5：mteb-leaderboard 1.0（3600s 超时前
  提交正确答案、verifier 通过）首解、rstan-to-pystan 1.0 首解；
  configure-git-webserver / mteb-retrieve 0 分（对比 r1/r2 基线为回归）、
  path-tracing-reverse 超时 0 分。**核心发现：5 题 agent 执行全程检索
  调用 0 次**（web_search/web_fetch/browser_read/外部子代理均 0）——
  Google 门禁/pacing 无 SERP 样本可观察，主序裁决无数据；与官方 minimal
  82.7% 无检索同向；解出全靠终端+已有知识（mteb-leaderboard 走终端内
  curl 仓库 API）。后续需"强制/引导检索"样本才能度量门禁与 DUAL 引用
  维度（机械引导检索属模型面改动，待用户裁决）。
- [x] 汇总 S3 重建（Linux musl 三件套）——2026-08-30 完成：
  `build_orz_aliyun_trixie.sh`（rust:1.97-slim 容器，ORZ-BUILD-MOUNT-001
  契约；日志 `D:\tb-eval\build-20260830-s3.log`）：三件套 12:06 HKT
  （orz 106,411,880 B / orz-signer 1,388,496 B / orz-acaf-provision
  1,206,464 B）；bookworm 冒烟符合预期（provision usage / signer manifest
  缺失 / orz TTY io error）、三件套 ldd 静态链接；警告面 14 项与上次基线
  持平（含 orz-host local_browser 既有 2 项）；对应源码 orz b604773 + 父
  b0b903e（均已推送）。
- [x] 容器内 chromium 注入（0k-5 前置；CONTROLLER_SPLIT §4.3 / DUAL §3.3）
  ——**2026-08-30 完成**：`tb_agents/orz.py`（host 侧文件，非 git 仓库；
  SHA256 047830C892AE1DB9E26D20318141120605893CD4F64D35902D8110880A712605）
  install() 新增 `eval_browser` 开关（`--ak eval_browser=true` /
  `ORZ_EVAL_BROWSER=1`，缺省关闭）：
  - apt 优先：`apt-get install -y --no-install-recommends chromium`（Debian
    bookworm 等真实包，实测 Chromium 151，产物 /usr/bin/chromium）。
  - 官方快照 fallback：apt 无真实包（ubuntu:24.04 仅 snap 过渡包）→
    系统依赖 + `storage.googleapis.com/chromium-browser-snapshots/Linux_x64`
    `LAST_CHANGE` 动态取 revision，解压到 /opt/chrome-linux（实测 Chromium
    154，headless 渲染 OK）。
  - run() 显式白名单注入 `ORZ_BROWSER_PATH` + `ORZ_BROWSER_HEADLESS=1`
    （env_clear 不发继承）；apt 失败时快照兜底、非 apt 系跳过，
    local_browser → framework_fallback 机械降级不变。
  - 容错：harbor `_exec` 对非零退出码抛异常——snapshot 各失败路径
    `|| true` 落到 SNAPSHOT_FAIL（零退出），version 校验带 `|| true`，
    install 永不因浏览器失败 fail。
  - 端到端实测（真实容器 + 真实 install 命令 + 分支逻辑）：bookworm →
    /usr/bin/chromium；ubuntu:24.04 → /opt/chrome-linux/chrome；apt 异常
    时快照兜底路径亦验证（install 不失败）。
- [x] 实机验证（检索题冒烟 + 超时/并行/截断行为观察）——2026-08-30
  并入门禁观察实验完成（见上）。

**第二批（独立设计轮）**：
- [ ] `project_doc_index` v2（git HEAD 基线 + 工作树增量层 + Blake3 内容哈希 +
  索引驻留）。
- [ ] 会话级 tab 池 + 同轮多页并行读取。
- [ ] 委托契约复杂度分档。

**暂缓/否决（不实施）**：`retrieve_project_docs` 维持封存
（`R1_SEALED_MAIN_TOOLS` 不动）；向量语义检索；web_search 并发 >1；浏览器
daemon 跨 run 常驻；模式内混合路由（需修订 ADR-0010 §3.7.12）。

### P0-B FUS-RETRIEVAL-MECH（`implemented`；批次 1-6 已全部闭合，2026-08-14，保留供核对）

依赖顺序：1（B-1，已闭合）→ 2/3/5（已闭合）→ 4（已闭合）→ 6。

- [x] 步骤 1（B-1）：web_search citations 结构化透传进 loop（`ToolResult.structured` 接缝 + 证据账本 `candidate_urls` / `raw_source_refs` 镜像）。
- [x] 步骤 2：web_fetch 候选机械计数门禁与计数反馈——`ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8（2026-08-14 用户裁决；activation 累计 + 侧车持久化、去重后 URL 计数、超限显式拒绝、无 ToolStarted、熔断同面、`tool_completed` 计数字段 Schema 先行）。
- [x] 步骤 3：机械预筛模块——候选池净化（canonical URL / host 级去重、已知失败形态、相关性粗筛）+ tier/weight 排序标签，进结构化结果（2026-08-14 闭合；`candidate_urls` 升级为预筛后保留池，`candidate_pool`/`prefilter_log` 契约先行）。
- [x] 步骤 5：输出级引用校验器与交付边界接线——`[来源]` 结构化解析、source_id/visibility/claim 上限校验、失败显式降级（覆盖 V11-IMPL-004 的 source binding 与 claim verifier；2026-08-14 闭合，审计见 `docs/audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md`）。
- [x] 步骤 4：browser_read 范围/模式参数扩展（全文/预览/关键词提取；local_browser 第二段复用同一计数域——2026-08-14 用户裁决：计数域挂载在检索子代理 activation（复用 web_fetch per-activation 语义），主 Agent 不执行检索任务、主车道投影移除 browser_read（投影移除与子代理恢复已实施），见设计 §1.3 注；2026-08-14 闭合，审计见 `docs/audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md`（含 2026-08-14 全面审查修复：计数消费后置、12K 严格上限、输入截断、maxLength、页脚 terms 语义、字段改名））。
- [x] 步骤 6：提示词相应缩短（计数/预筛/引用规则）与测试更新——主提示词引用纪律
  缩减为"标记格式 + verifier 交付前机械校验"，检索提示词移除候选 ≤5 软约束改指
  机械预算反馈（"候选 N/M，剩余 K"），来源加权/引用规则去冗余；prompt.rs 测试同步
  （2026-08-14 闭合，审计见 `docs/audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md`）。
- 注：DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）未并入
  批次，已转 P3 遗留小项独立跟踪。

入口：[检索机械控制设计](docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md) / [B-1 实施审计](docs/audits/GAP_RETRIEVAL_MECH_B1_CITATIONS_IMPL_AUDIT_2026-08-13.md) / [步骤 2 实施审计](docs/audits/GAP_RETRIEVAL_MECH_STEP2_WEB_FETCH_CANDIDATE_COUNT_IMPL_AUDIT_2026-08-14.md) / [步骤 4 实施审计](docs/audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md)。

### P0-C CLASSICAL-EXEC-ASSISTANT（`pending`；POC 已通，v0.5 操作台模型）

- [x] 小样 1：控制台路由（服务注册表 + hassil 意图 + stdio JSON；28/28 检查通过）。
- [x] 槽位表由工作区索引动态生成（POC 闭合；生产接线复用 orz `project_doc_index` 缓存）。
- [x] 小样 2：编辑执行器 `workspace.search_replace`（2026-08-15 闭合，用户裁决通过 + 独立判定一致；结果工件 `prototype/classical_console/sample2_result.json`）。
- [x] 小样 3：机械组合脚本模式（线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）——已闭合（2026-08-15 用户裁决通过 + 独立判定一致）；结果工件 `prototype/classical_console/sample3_result.json`。
- [x] orz 内嵌集成：HA 操作台接线（薄接缝在 orz ↔ 底座模型后端；黑板动作栏为生产协作接缝；POC stdio 仅原型隔离）。
  - [x] S1：操作台核心与黑板动作栏数据面——`orz-loop/src/console.rs`
    （ServiceRegistry / ActionSpec 契约 / issue_action 五步路由 /
    fail-closed 信封 / 有界 Trace+TraceStore，执行经 ActionExecutor 抽象
    委托，生产复用 run_host_tool 权限/ACAF 链）+ `blackboard::ActionBoard`
    （注册板块 / 动作栏单槽 / 结果栏有界 50，随 plan epoch 归档/轮换）。
    2026-08-15 落地；全面检查修复已闭合（ExecuteError 执行失败/策略拒绝、
    响应契约强制必填并注册时校验缓存、exit_code=Some(0) 契约、TraceStore
    commit + 失败事件滚动保底、注册板块最小提示投影）；新增 18 项测试
    （console 15 + blackboard 3），orz-loop 356 通过。
  - [x] S2：模型面投影（`blackboard_read` section=actions 读取最小参数提示
    + `blackboard_action_write` 写单，pending 机械拒绝）与轮末机械发放
    （round/plan_epoch/run_id 防重放与过期 → 注册表/契约 → 真实目标解析 →
    ACAF 票据 → 策略表/模式门 → 经 `run_host_tool` 执行 → 响应 schema
    验证 → 结果栏 receipt+trace_id；策略拒绝映射 `step=policy`；发放收口
    `TraceStore.commit`）。2026-08-15 落地，实施审计见
    `docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`；
    orz-loop 364 通过 / 0 失败。
  - [x] S3 前置：结构化策略拒绝（P1-2 定案，2026-08-15 闭合）——拒绝路径统一
    返回结构化信号（`ToolResult.policy_denial = {source, code, reason}`；
    source ∈ permission/acaf/retrieval_mode/taint），controller 删除字符串
    前缀判定（`console_policy_refusal` 退役）；ToolCompleted 增可选
      `policy_denial`（Schema/verifier/fixtures 先行）；内容碰撞回归测试
      （成功输出含旧拒绝前缀必须判成功）。实施审计见
      `docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`。
      2026-08-15 全面审查修复（F1-F8）闭合：拒绝事件补 `exit_code=1` +
      `status=error`（含 host 级拒绝）、verifier ACAF 家族补
      `web_fetch`/`browser_read`、新增生产者事件→验证器对拍测试、
      `orz/` 登记为父仓库 git 子模块（SilverWhite/CLI
      `feat/fusion-architecture`，提交 a0c9ffc 已推送远端）+ 源码完整性
      清单（`orz_source_manifest.sha256`）接入仓库门禁。
  - [x] S3：`assistant.trace` 只读服务接线 + `workspace.run_script`（PTC）
    生产化 + Profile/Bundle 按钮组加载——2026-08-15 闭合：
    - `assistant.trace` 注册为控制台内部动作（`ActionKind::TraceRead`）：
      按 trace_id 有界取回（默认 tail=20）、`not_found` fail-closed；
      发放链对内部动作补 ToolStarted/ToolCompleted 留痕（读操作本身入
      journal；复用 v0.1 事件面，无 Schema 变更）。
    - `workspace.run_script` 注册为控制台内部动作（`ActionKind::RunScript`）：
      PTC 线性脚本生产化（`$ref` 静态/运行时校验、逐行契约校验 + trace、
      上限 8 步/30s/4MiB、禁嵌套、失败保留内层 step/code + `script_step`）；
      内层步骤逐行经 `issue_action` 复用注册表/契约/目标/执行/验证五步链，
      仍走 `run_host_tool` 全链路（权限/ACAF/模式门）。
    - Profile/Bundle 按钮组加载：`ActionBundle`（standard/read_only/
      benchmark，场景键=`ToolPolicy`）+ 注册板块投影 = Profile/Bundle ∩
      探针完整集（`registrations_for`；Host 动作按工作工具探针过滤，
      非工作工具不探不标，内部动作恒加载）；同轮探针快照同时驱动工具
      投影与注册板块。
    - 验证：orz-loop 377 通过 / 0 失败（3 ignored live）；fmt/clippy 无
      新增告警；orz-host/tui/bin/assurance check 通过；仓库门禁 valid
      （`orz_source_manifest.sha256` 已重新生成）。
    - 2026-08-16 全面审查收口：单订单步数上限 20→8
      （`MAX_SCRIPT_STEPS_PER_ORDER`）；`assistant.trace` 查无 id 定案
      `step=execute`+`not_found`；checkpoint 轮跳过注册板块刷新；注册
      不变式补齐（内部动作带 target 拒绝/内部动作类唯一/bundle 非空/
      嵌套按 kind 拒绝）；最终超限信封补 `script_step`；测试补齐 7 项
      （orz-loop 384 通过 / 0 失败）。详见审计 §6。
    - 实施审计：
      `docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md`。
  - [x] S4：端到端测试、实施审计与正式组件决策门材料（2026-08-16 闭合）。
    - [x] 单步超时：30s=总墙钟+单步受控；脚本截止时间下沉 host 层
      （`call_tool_with_timeout` 覆盖 + 进程树收口；结构化 `timed_out` →
      `tool_timeout` / 脚本归一化 `script_timeout`+`script_step`）。
    - [x] 脚本消耗 tool-round 预算：每执行一步扣 1 个预算单位；发放前
      预检（当前轮 1 单位 + 长度 ≤ 剩余，不足零执行拒绝
      `budget_insufficient`）；按实际执行步数减计；下一轮预算块机械反映。
    - [x] 端到端测试（FakeProvider 完整任务会话：写 run_script/trace 订单
      → 发放 → trace 读取 → 结果栏反馈 → 下一订单；含 checkpoint 轮
      板块保留断言与超时/预算边界）。orz-loop 392 通过 / 0 失败。
    - [x] 实施审计（`docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`）、
      正式组件决策门材料（小样 1/2/3 + S1-S4 审计汇总）与文档同步。
    - [x] 二次审查收口与超时语义复核（2026-08-16）：撤销「30s 总墙钟含
      进程时间」语义（对照 Codex/Grok 成熟设计）——脚本每步由 host 配置
      预算独立约束；预算预检先静态校验（不掩盖内层错误）；小样 1 结果工件
      `sample1_result.json`（90/90 复跑）；max=1 零剩余边界测试；orz-loop
      395 通过 / 0 失败（详见 S4 审计 §8）。
- [x] 正式组件决策门：小样全面达标后裁决；不达标即撤（2026-08-16 用户
  裁决「P0-C 可转正式组件」通过，不达标即撤条款未触发）。
  - 成熟复用评估（2026-08-16，只读）：无——决策门本身；材料已含 HA/hassil 与 DSH 借鉴。

入口：[设计](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [POC](prototype/classical_console/README.md)。

### P0-C2 PLAN-FIRST-BLACKBOARD（`pending`；模型面重构，2026-08-15 用户定案）

- [x] PLAN-FIRST 阶段 A：模板去人格（主/子代理/apply-patch 模板 +
  `ORCHESTRATOR_PROMPT_BODY`；XOR 加密模板同步重生成；渲染测试锁定无人格关键词）
  + AGENTS.md 计划型机械包裹（用户内容之前；唯一机制）+ 首轮计划轮硬门
  （首轮只暴露黑板读取 + `plan_write`；一次重填 + 机械降级留痕；计划落黑板
  plan epoch）。2026-08-16 闭合：`plan_write` v0.2 事件/校验/落板/会话级
  门（ACP server + CLI run 生产接线）；实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md`。
  2026-08-16 审查收口：计划轮不计工具轮预算、whitelist 窗口顺延、结构
  上限定稿（32/8/16/32K）、包裹无条件注入、P1/P2/P3 修复（详见审计 §7）。
  - 成熟复用评估（2026-08-16，只读）：明确——Grok 模板为改造对象、AGENTS.md 为 Codex 生态成熟约定、首轮硬门复用 checkpoint 机制。
- [x] PLAN-FIRST 阶段 B：注册板块=探针投影（移除静态基础集中间态）；工具栏刷新
  绑定黑板模型栏。2026-08-16 闭合：controller 单一探针源
  （`console_probe_source`，仅内存/随轮覆盖、run 起始复位）→
  `sync_console_registrations`
  为派生唯一路径（Profile/Bundle ∩ 探针完整集）；无探针轮次不再 bundle-only
  刷新、沿用上一轮内容（移除静态基础集中间态）；`blackboard_read
  section=actions` 读取时由最近探针源派生并持久化（live 视图），归档 epoch
  读保持快照（测试锁定）；工具投影与注册板块共用同一探针源（同源一致性
  测试锁定；审查收口：归档读不派生与 run 起始复位单测）。
  实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md`。
  - 成熟复用评估（2026-08-16，只读）：部分——复用已闭合的 FUS-TOOL-PROBE 单一探针面（自研成熟机制）。
- [x] PLAN-FIRST 阶段 C：双模式（console 默认 + direct 受控降级）——投影切换、
  3 连败助理层故障面计数、无工具询问轮、`console_mode_transition` + gate_log、
  `console_step_done` 证据门、`console_return_to_console`、plan_write/分步计划
  状态机、ActionOrder 增 `step_id`。2026-08-16 闭合：
  - [x] 契约层：v0.2 `console_mode_transition` / `console_order_written` 事件
    （Schema/verifier/fixtures 先行；action_write ToolCompleted 收敛通用形状，
    阶段 A 审计 §7.4 债务收口）；tool-started/completed 增 direct 盖章字段；
    `ActionOrder.step_id`；`StepStatus` 状态机化（done/failed 带 receipt，旧归档兼容）。
  - [x] 双模式状态机：run 级模式/连败/询问标记/transition_id/direct trace
    证据面；故障面机械分类（默认阈值 3，`ORZ_CONSOLE_DIRECT_FALLBACK_THRESHOLD`
    可调）；无工具询问轮（一次重填、降级默认 stay、每 run 至多一次）；
    switch/stay/return 写 `console_mode_transition` + gate_log。
  - [x] 步骤门：console 订单必须绑定当前可执行步骤，`step_not_done` 显式拒绝；
    发放时 in_progress、receipt 置 done/failed；`console_step_done` 证据门
    （transition_id + trace_id 交叉，不匹配拒绝）。
  - [x] 模型面收敛：console 默认面（黑板读写 + 只读核查，无执行工具）+ 调用面
    门禁 `console_mode_tool_denied`；direct 恢复工作工具投影、直接动作事件
    全链路盖章；生产接线 CLI run + ACP server。
  - [x] 测试与审计：orz-loop 433 / orz-tui 178 / orz-assurance / orz-bin /
    orz-host acp_server 36 / Python runtime 306 + assurance 1614 通过；
    实施审计见
    `docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md`。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex plan/approval 与 ACP 模式切换可参考，主体自研。

入口：[设计](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [ADR-0010 §14.17](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。

### P0-D ORZ-COMPACTION-REDESIGN（`implemented`；S1-S6 已全部闭合，2026-08-14）

- [x] S1：D2-2 恢复超窗预估算截断——恢复加载后首请求前估算预检 + 整轮截断 + `context_recovery_truncated` 事件（完整侧车审计副本 `recovery-conversation-full.json`）。
- [x] S1：D3-1 恢复保留 marker/白名单——恢复回写过滤放行 `[前文上下文已压缩` 与白名单块，首请求可见（主车道 + 子代理 snapshot 同规则）。
- [x] S2：工具调用记录机械坍缩——动作台账行（工具/目标/指针/digest/最终回复）+ 配对纪律 + 有界尾部（请求视图，侧车保留全文）。
- [x] S3：五段模板摘要接线——机械槽位（黑板目的/计划/路径）+ 模型槽位校验（≤17K、退化拒绝 300 等效字符门（CJK 一字折算 2）、重做 ≤3 次、summary_incomplete 终止态 + fallback 机械截断）。
- [x] S3：`context_compressed` 事件 Schema v0.2 扩展（mode/reason/summary_id/digest/path/summary_incomplete/retained_rounds）+ verifier/fixtures 同步 + `.gsa/compaction/` 存档与 7 天 retention + TUI 投影。
- [x] S4：实施审计、ADR-0010 §14.10 补写、索引/BACKLOG/TODO 状态同步、设计文档状态更新。
- [x] S5：审查修复（ADR-0010 v1.14）——守卫失败重试 3 次后强制压缩并 `guard_failed` 报告；
  会话结束自动压缩（`session_end`，marker 固定进 sidecar，恢复治本）；存档写失败显式重试
  ≤3 次并 `archive_write_failed` 报告；退化守卫 300 等效字符（CJK 一字折算 2）；黑板 edit
  窗口随压缩滚动（v1.15 已废止：改为 plan epoch 轮换，见 P1 ORZ-BLACKBOARD-PLAN-EPOCH）；
  冷却 3→2 模型轮；摘要 120s 超时；路径槽 Top-40 双上限；
  Schema/verifier/fixtures 扩展与 `$id` 修正。
- [x] S6：二次复查处理（2026-08-14）——设计投影 §7/§6 与 ADR 口径对齐（session_end 160K
  阈值补写、复用边界内联化）、schema/代码注释冷却残留修正、终止态 marker 占位 digest
  改显式"（未生成）"、fixtures 生成器回写 P0-B step 5 手工修订并统一 LF 行尾、
  chars/2 中文低估登记为 6d 实施前置校准项。

入口：[压缩设计](docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md) / [ADR-0010 §14.10/§14.14](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [P0-D 实施审计](docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md) / [BACKLOG 3b](docs/BACKLOG_AND_PRIORITIES.md)。

## P1 — 可并行审计 / 证据

### FUS-COMPONENT-REGISTER（`partial`）

- [ ] 逐 crate/component 采用审计——65 组件全 `audit_required`；从当前代码可达性与 local diff 出发，不得由 crate 名/编译推断采用档位（V11-IMPL-008）。
  - 成熟复用评估（2026-08-16，只读）：部分——审计对象即 65 个 Grok Build/Rust 生态成熟组件，产出即复用裁决（直接复用/薄适配/fork）。

入口：[register yaml](upstream/fusion-component-register-v0.1.yaml) / [register schema](upstream/fusion-component-register-v0.1.schema.json)。

### GAP-WINDOWS-EVIDENCE（`partial`）

- [ ] ORZ-WIN-PROC-001/002/003 案例晋级——真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
  - 成熟复用评估（2026-08-16，只读）：明确——可复用 Grok child-tree probe 证据与 GAK AppContainer/Job Object 审计。
- [ ] 建立 Windows 平台兼容性设计文档（ADR-0010 §6/§11.7 目标：`architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md`）。
  - 成熟复用评估（2026-08-16，只读）：明确——设计输入含既有 Windows 审计/事故材料与 Codex/Grok Windows 行为对照。
- [ ] 建立 `regression/windows/` 与 `.observed-runs/windows/` 路由（自动回归/人工复核入口与 git-ignored 原始运行目录）。
  - 成熟复用评估（2026-08-16，只读）：明确——路由可承接既有 observed runs/探针矩阵先例（Grok probe）。

入口：[incidents](docs/incidents/windows/README.md) / [cases](docs/cases/windows/README.md)。

### IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- [x] transport/retry/thinking 按主/子代理同构约束复核（ADR-0007 / DEEPSEEK_ADAPTER_CONTRACT）——
  2026-08-16 闭合：三实例共享单一 `DeepSeekTransport`（ModelConfig/RetryPolicy/
  ThinkingMode::EnabledMax 单一来源）、`REQUEST_MAX_TOKENS=160_000` 单一常量、
  请求级 thinking 覆盖仅 `-p` 预检轮（F-07 文档化例外）；边界=压缩摘要/预检轮为
  loop 外辅助请求（非同构范畴）、契约 §2.1 旧别名拒绝与 /models 预检未实现（另行跟踪）。
  - 成熟复用评估（2026-08-16，只读）：部分——DeepSeek API 契约与 ADR-0007 传输/重试策略为成熟参照。
- [ ] DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7；当前仍为 offline / 构建时 evidence）。
  - 成熟复用评估（2026-08-16，只读）：部分——DeepSeek 官方 API/文档为成熟参照；主要工作是证据收集而非实现复用。

入口：[ADR-0007](adr/ADR-0007-transport-retry-policy.md) / [DEEPSEEK_ADAPTER_CONTRACT](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)。

### ORZ-CACHE-CONTEXT-COST（`approved`；P1，2026-08-14 登记）

- [x] 请求 header 变化留痕：模型请求 header（system+tools 摘要 + config + 原因 initial/change）变化时 journal 留痕（翻转可审计、miss 可归属）。
- [x] 探针准确性与稳定性：误判审计（假完整/假不完整）、翻转与 header 留痕事后核对、可选后端接线同步补翻转测试。
- [x] 单轮工具结果注入预算（默认 50K、`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 可调、按轮累计、超限拒批 + offset 续读）与提示词策略化读取（grep/结构优先、证据关键文件才全文）。

> 2026-08-15 三项全部闭合：`request_header_change` v0.2 事件（Schema/verifier/
> fixtures/TUI）、验证器翻转↔header 交叉核对 + `assurance/probe_accuracy_audit.py`
> 误判审计、50K 注入预算（env 可调、无 ToolStarted 拒批、offset/grep 提示、读取
> 纪律进提示词）；验证 orz-loop 337 / orz-assurance 151 / orz-tui 178 / orz-bin
> 全绿、Python 1888+14 skipped、仓库门禁 valid；审计见
> `docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md`。

入口：[ADR-0010 §14.9](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [探针设计](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [BACKLOG](docs/BACKLOG_AND_PRIORITIES.md)。

### ORZ-ORIENTATION-FORCED-TEMPLATE（`implemented`；P1，2026-08-15 闭合）

- [x] ADR-0010 §4.2 正文修订登记（v1.13 裁决已登记；v1.16 正文修订 2026-08-15）：「注入」→「触发点暂停并填写模板」。
- [x] 强制模板轮实现：触发点下一安全动作间隙暂停、独立无工具 checkpoint 轮、模板字段/机械校验、一次错误反馈重填 + 降级兜底（pending 单槽、Orientation 优先、主车道；检索车道旧行为不变）。
- [x] 缓解必做：`progress_evidence` 与 journal 证据身份存在性交叉校验（found/missing 随 `checkpoint_response` 记录）；`next_action=gather_evidence` 必填缺失面（`missing_evidence`）。
- [x] 事件/Schema/verifier/fixtures 同步（新增 v0.2 `checkpoint_response` 响应事件：模板/响应/校验结果/证据交叉校验/降级原因）+ 测试（触发→暂停→填表→恢复、重填、降级、checkpoint 轮工具拒绝、主车道隔离、计数语义；orz-loop 332 通过）。
- [x] 实施审计、BACKLOG/TODO/索引状态同步（审计见 `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`）。
- [x] 二次全面审查修复（2026-08-15 复核）：DC fire 增 `agent_role=main`（Schema/fixtures/生成器）+ 验证器兼容旧 fire；验证器新增 outcome↔validation 与 gather_evidence 条件交叉；`checkpoint_response.agent_role` 收紧主车道；Rust trim 长度口径；conformance 计数更名；审计/设计/ADR 同步（orz-loop 333 / Python runtime 264 通过）。
- [x] 二次全面审查修复（2026-08-15 复核）：DC fire 增 `agent_role=main`（Schema/fixtures/生成器）+ 验证器兼容旧 fire；验证器新增 outcome↔validation 与 gather_evidence 条件交叉；`checkpoint_response.agent_role` 收紧主车道；Rust trim 长度口径；conformance 计数更名；审计/设计/ADR 同步（orz-loop 333 / Python runtime 264 通过）。

入口：[设计](docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md) / [ADR-0010 §14.13](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [BACKLOG 6c](docs/BACKLOG_AND_PRIORITIES.md)。

### ORZ-SESSION-CONTEXT-MONITOR（`approved`；P1，2026-08-14 登记；2026-08-16 度量重定）

- [x] 压缩恢复预检估算校准（chars/2 中文低估）：2026-08-16 用户裁决度量改次数制后废止，
  不再依赖 token 估算口径。
- [ ] 度量接线：监听 `context_compressed` 事件（reason=rhythm/fallback）累计会话内压缩次数；
  `session_end` 不计；同一次压缩只计一次。
  - 成熟复用评估（2026-08-16，只读）：有——复用既有 `context_compressed` v0.2 事件面，无新 Schema。
- [ ] 阈值配置（env/TOML，默认 2 次提醒 / 3 次总结推荐）；同一阈值只触发一次。
  - 成熟复用评估（2026-08-16，只读）：部分——2/3 为旧 384K/500K 语义对应，默认待校准。
- [ ] 最简实现：阈值到达的最后一轮模型输出末尾机械附言（附压缩次数）；headless/自动化仅写日志；3 次附五段模板 + 新窗口开场提示骨架。
  - 成熟复用评估（2026-08-16，只读）：部分——3 次推荐复用压缩五段模板（Grok compaction 血统）。
- [ ] 测试（到达/未到达、session_end 不计、一次一计、headless 分支、幂等）+ 实施审计 + BACKLOG/TODO/索引状态同步。
  - 成熟复用评估（2026-08-16，只读）：无——自有测试/审计工作。

入口：[设计](docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md) / [ADR-0010 §14.13](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [ADR-0010 §14.18](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [BACKLOG 6d](docs/BACKLOG_AND_PRIORITIES.md)。

### ORZ-BLACKBOARD-PLAN-EPOCH（`implemented`；P1，S1-S5 已闭合，2026-08-14，保留供核对）

- [x] S1 plan epoch 身份与批准事件接线：`plan_id`/`plan_epoch` 随 plan 批准事件写入（PlanApproved payload 增 `plan_epoch`，Schema/fixtures 同步）；同 plan_id 修订不清板、新 plan_id 批准原子轮换。
- [x] S2 原子轮换与归档：归档旧 epoch 快照 → 清 edits/tool_actions/exec 工作区并复写 plan → 写新 plan；gate_log/白名单/检索分区豁免；epoch 快照落盘 `.gsa/blackboard/epoch-<id>.json`（批准/修订持久化当前 epoch，轮换归档旧 epoch；写入有界重试 + warn）。
- [x] S3 压缩解耦：移除压缩成功后 `blackboard.edits.clear()`；路径槽=本 plan epoch 增量（Top-40 + 5K，溢出指针指向 epoch 快照）；marker 携带 `plan_epoch`；archive dir 构造时装载最新 epoch 快照（恢复入口）。
- [x] S4 `blackboard_read` 跨 epoch 回查：`epoch` 参数走归档读取，保持按分区/时间范围取用契约；缺失/未配置显式提示，不静默回退 live 视图。
- [x] S5 测试与审计：压缩不清板、轮换原子性/范围、跨 epoch 回查、恢复、契约扩展（Schema/verifier/fixtures/journals）；实施审计 + BACKLOG/TODO/索引状态同步；retention 覆盖 `.gsa/blackboard` 7 天清扫。
- [x] S6 复查补强（2026-08-15，F1/F3，用户裁决）：`plan_epoch` 时间戳单调编号（unix 毫秒基底、`max(now_ms, 磁盘 max+1)`）；身份不变式强制（同 plan_id 同 epoch、新 plan_id 严格递增；`rotate_to_plan`/`try_with_plan` 返回错误、`with_plan` fail-fast、拒绝先于变更）；retention 保留最高编号快照（恢复入口）；Schema 描述/设计/ADR/审计同步。验证：orz-loop 312 / orz-host 210 / orz-tui 178 / orz-bin 全部通过；Python runtime 251 通过；仓库门禁 valid、0 错误。
- [x] S7 复查遗留闭合（2026-08-15，v1.15⑨）：F2 归档写盘原子化（临时文件+rename、恢复降序回退）、F4 跨进程 `.claim-<n>` 原子占号、F5 归档目录单一来源、F6 非法 epoch 显式报错、F7 归档失败入事件面（新 v0.2 `epoch_archive_write_failed`）、F9 `persisted_at` 更名（serde alias 兼容）、F10 设计 §5 措辞对齐；BACKLOG 6e / ADR §14.15 ⑨ / 审计 §7 同步。

入口：[设计](docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md) / [ADR-0010 §14.15](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [实施审计](docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md) / [BACKLOG 6e](docs/BACKLOG_AND_PRIORITIES.md)。

### ORZ-LARGE-FILE-READ-CONTRACT（`implemented`；P1，2026-08-17 设计定案，同日实施闭合）

- [x] 读取工具契约：超过粗门（默认 16KB、可配 8–32KB，env/TOML 口子）的文件返回
  读取句柄信封（path / size / encoding / content_sha256 / available_range /
  有界预览 ≤4KB / truncated / offset 续读指针），不返回全文；小文件保持全文一次
  返回；精门=50K 单轮注入预算兜底。**2026-08-17 闭合**：GrokBuild `read_file`
  文本路径（console 默认面 `workspace.read_file` 与 direct 共用）；信封
  terminal-only 不流式；单行超长预算内截断并报 truncated（offset 指向下一行）；
  SKILL.md/`skills` 路径保持全量豁免；PDF/PPTX/图片路径不变；`FileTooLarge`
  文本路径被信封取代、保留为防御兜底。
- [x] 模型面契约提示：grep/结构提取优先、证据关键文件才全文、大文件 offset 分段、
  grep 空结果语义（空结果 ≠ 无文件）——提示词读取纪律（v1.9/v1.22）落成工具契约。
  **2026-08-17 闭合**：BASE_SYSTEM_PROMPT 读取纪律补信封语义、`read_file` 工具
  描述（DESCRIPTION_FULL）补粗门/信封说明、console 注册表 `workspace.read_file`
  描述同步。
- [x] 黑板/结果栏只放指针（path/document_id/size/digest/offset），内容本体留盘上/
  内容寻址证据区；维持不新增自由随记区。**2026-08-17 闭合**：信封即结果栏承载的
  有界负载（≤4KB 预览 + 元数据指针），全文内容不上黑板/结果栏。
- [x] 测试（阈值边界、信封字段完整性、offset 续读、小文件全文路径）+ 实施审计 +
  BACKLOG/TODO/索引状态同步。**2026-08-17 闭合**：orz-tools read_file 199 /
  output 信封序列化 1 / orz-loop 440 / orz-host read_file e2e 2 通过；
  clippy 无新增可归因告警；orz 172b14e；实施审计
  `docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md`。
- [x] 全面检查修复（2026-08-17 审查后处理）：P2-1 信封空窗口/越界 offset 语义
  （past-EOF truncated=false/offset=None + 渲染实际行数；范围内空窗口
  offset=start_line；最后一行行内截断 offset=None）；P3-1 `[toolset.read_file]`
  配置节端到端接线（coarse_gate_bytes 经 orz-config 分层装载注入工具参数、
  优先于 env；AgentBuilder `with_read_file_params` 通路）；P3-2 concise 描述
  补信封说明；P3-3 envelope 不追加 cursor rules 边界登记；P3-4 提示词措辞
  精确化。验证：orz-tools read_file 201 / output 84 / orz-agent 1 /
  orz-loop 440 / orz-host read_file e2e 5；clippy 无新增可归因告警；
  orz 7c4a99e + bd8d485；manifest 1401、仓库门禁 valid。

入口：[设计 §11](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) /
[黑板设计 §4](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) /
[ADR-0010 §14.22](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) /
[BACKLOG 6f](docs/BACKLOG_AND_PRIORITIES.md)。

### FUS-LEDGER-FOLD-STATE（`implemented`；P1，2026-08-18 设计定案，同日实施闭合）

- [x] S1 fold 三态 + 有状态请求视图：`action_ledger.rs` 新增 `LedgerFoldState`
  （`fold_start`/`fold_cut`/`folded_ledger` + 不变量）、`build_request_view`
  （未折叠=原文；已折叠=preamble+冻结台账+`[fold_cut..]`）、`advance_fold`
  （整轮配对、轮次号延续、防空转）、`rounds_before`。**2026-08-18 闭合**：
  action_ledger 5 项单测（未折叠原文 / 首推进形状 / 防空转 / 轮次连续旧行
  不变 / 折叠前缀跨追加字节稳定）。
- [x] S2 loop-top 推进触发：每请求前估算折叠视图 ≥ `fold_trigger_tokens`
  （默认 128K、`ORZ_FOLD_TRIGGER_TOKENS` 可配）时机械推进（checkpoint 轮
  优先、零模型调用、不打断批次）。**2026-08-18 闭合**：orz-loop 循环级测试
  `fold_state_advances_once_and_prefix_stays_stable`（触发前无台账、推进后
  按冻结台账版本锚定纯追加）。
- [x] S3 压缩联动 + 摘要同源 + 恢复：`run_template_compact` 摘要输入与主请求
  同一折叠视图；drain 保留起点=已折叠时 `fold_cut`；压缩后三态重置；折叠
  状态为每轮循环实例局部（随 `LoopOutcome` 返回，主/检索车道 session_end
  复用；避免嵌套子代理调度污染共享字段）；恢复/跨 prompt 一律 None 重新
  累积。**2026-08-18 闭合**：orz-loop 循环级测试
  `fold_state_resets_after_compaction_and_summary_uses_same_view`。
- [x] S4 参数接线 + 测试 + 文档同步 + 审计：压缩普通触发 160K→192K、兜底
  200K→256K；`with_fold_trigger_tokens` 测试 seam；默认值断言补全。
  **2026-08-18 闭合**：orz-loop 450 / orz-assurance / orz-tui 178 / orz-bin
  全量通过、clippy 无新增告警、`cargo fmt --all` 收口、manifest 1401、仓库
  门禁 valid；orz a5bea77；ADR-0010 §14.26 / BACKLOG 6g / 索引 / 实施审计
  `docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md` 同步。
- [x] 2026-08-18 二次全面审查收口：① 设计 §3.5 第 1 步归档补实现（冻结台账
  进摘要存档，成功分支「折叠台账（冻结快照）」段；终止态不落盘=接受边界）；
  ② 新增 v0.2 事件 `ledger_fold_advance`（fold_start/fold_cut/rounds_folded/
  view_estimate_tokens/agent_role；真实推进才发；Schema/verifier/fixtures/
  TUI 全链，verifier 窗口不变量）；③ `collapsed_cut` 完整性回退覆盖全部
  被折叠轮（新增中途不完整轮单测）；④ 死代码 `collapsed_round_count` 删除；
  ⑤ 压缩联动测试修正（prompt_tokens 与视图量级一致、两次 rhythm 成功路径、
  归档含台账断言）；⑥ 口径/文档修正（session_end 显式区分、设计 §3.1
  per-loop local 注记、审计计数 450→452）；⑦ fixture 生成器回填 console
  三事件与身份覆盖。orz 5274b39；orz-loop 452 / orz-tui 178 / orz-assurance
  152、Python conformance 15 + journal validation 214 通过、clippy 无新增
  可归因告警、manifest 1401、仓库门禁 valid。

## P2 — 生产化决策门

### IMPL-CONTROL-FABRIC（`partial`）

- [x] fail-closed 生产启用（2026-08-16 闭合）——默认翻转（未设置即强制，
  显式 `0|false|no|off` 影子，非法值 exit 2）；CLI run / ACP stdio / TUI
  三入口接线；核查清单 ⑦⑨⑩⑪ 收口（web_search 显式排除、host 稳定面、
  URL 规范化等价、重定向逐跳 gate）；`orz-acaf-provision` 供应工具 +
  `scripts/orz_acaf_run.ps1` 启动链；审计见
  `docs/audits/GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md`。
  - 成熟复用评估（2026-08-16，只读）：无——机制已自研完成，剩翻转+核查清单+审计。
- [ ] Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
  - 成熟复用评估（2026-08-16，只读）：部分——HKDF-SHA256/HMAC 与单调计数器为成熟标准原语；ACP 模式切换为成熟先例。
- [ ] Slice 4：Windows Sandbox backend（D-11）。
  - 成熟复用评估（2026-08-16，只读）：明确——Windows Sandbox（Hyper-V）、AppContainer、Job Object 为成熟 OS 能力；GAK/P2 审计可直接支撑。
- [ ] 可选：检索车道 web_fetch activation 绑定接线；conformance capture 票据场景；normalize_lexical 单源化；ACP 会话路径接 ACAF。
  - 成熟复用评估（2026-08-16，只读）：部分——ACP 规范/xai-acp-lib、orz-paths、既有 D-13 机制与 fixture 体系可复用。

入口：[ADR-0011](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [ACAF 设计](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md) / [fail-closed 审计](docs/audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。

### OPS-PROTOCOL（`pending`；裁剪方向已定）

- [ ] 产出裁剪设计：删除安全保留为 host-owned 工具、跨环境桥接内部化、双执行器收敛单一参考（生产走 Rust 工具面）。
  - 成熟复用评估（2026-08-16，只读）：部分——Windows 回收站（BitBucket）与 XDG trash 为成熟 OS 约定；既有执行器作参考。
- [ ] 生产接线裁决（先验票，再由协议执行器执行）。
  - 成熟复用评估（2026-08-16，只读）：无——用户决策门。

入口：[协议](protocol/structured-operation-protocol-v0.1.md) / [Schema](protocol/structured-operation-protocol-v0.1.schema.json)。

## P3 — 收尾 / 清理

- [ ] EVIDENCE-LOCAL-BROWSER：Python 路径（retrieval_workflow / evidence_store / pdf_evidence）按 ADR-0010 重新符合性审查或退役。
  - 成熟复用评估（2026-08-16，只读）：明确——生产 Rust 已复用 CDP（Chrome）与 pdf_oxide；Python 路径按此审查/退役。
- [ ] GATE-CHAIN：分层 gate 链与融合 runtime 的最终接线随切片审计复核。
  - 成熟复用评估（2026-08-16，只读）：无——复核既有 assurance 链接线。
- [ ] （可选）提示词补列 observed scope 合法枚举（`full_text_observed` / `partial_text_observed` / `metadata_only`）——P0-B 步骤 6 复核观察登记，verifier 已机械兜底，暂不实施。
  - 成熟复用评估（2026-08-16，只读）：无——提示词枚举补列。
- [ ] V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：无——复核登记。
- [ ] V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：部分——orz 即 Grok Build fork，原 toolbar/session 代码在仓库内；Codex app-server 投影为成熟参考。
- [ ] orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`）——复核并登记闭合或转 gap。
  - 成熟复用评估（2026-08-16，只读）：无——测试修复。
- [ ] DC 硬信号 4/6：`same_module_no_evidence` / `key_surface_unexamined` 接线（原建议并入 P0-B，批次已闭合，独立待办）。
  - 成熟复用评估（2026-08-16，只读）：无——自有机械接线。
- [x] 前置收尾：提交当前未提交登记（995a384，2026-08-13）。

## 审计登记边界（条件触发，不占当前优先级）

- [ ] orz-host 可选后端接线（lsp / memory / 图像 / 视频 / MCP）：接线时翻转能力访问器并补翻转测试（FUS-TOOL-PROBE 边界）。
  - 成熟复用评估（2026-08-16，只读）：明确——LSP/MCP 为成熟开放标准，仓库内已有 orz-mcp；图像/视频走成熟服务 API。
- [ ] headless 计划模式能力信号（plan 模式探针当前以交互用户信号代理，未来 headless 计划模式需独立信号）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex headless/plan 模式可参考。
- [ ] 下一次真实运行捕获自然携带 23 工具分区 journals（当前 12 个为已提交 fixtures 重建）。
  - 成熟复用评估（2026-08-16，只读）：无——纯运行收集。
- [ ] B-1 后续：canonical URL/host 级去重留待预筛步骤 3；若 host/loop 拆为跨进程边界，补 `ToolResult.structured` 序列化契约。
  - 成熟复用评估（2026-08-16，只读）：部分——成熟 url 库/PSL 与 serde/JSON Schema。
- [ ] ORZ-RECOVERY-TOOL-OUTCOME：崩溃恢复工具结果词汇（`TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN` 合成 Tool 消息 + 只重试只读/幂等指引）——出现恢复面 400 或副作用未知证据时实施（单点修复）。
  - 成熟复用评估（2026-08-16，只读）：部分——Codex/Grok 工具生命周期与句柄化控制可参考。
- [ ] ORZ-STAGNATION-TOOL-SIGNAL：停滞守卫「同工具同参数」信号——出现「同参循环且输出持续变化」证据时在 stagnation guard 内加最小计数信号。
  - 成熟复用评估（2026-08-16，只读）：明确——仓库内已有 Grok 血统 stagnation 模块（orz-assurance/orientation/stagnation.rs）可直接扩展。

## 近期已闭合（供核对，不计入开放项）

- [x] FUS-TOOL-PROBE：P0-A 步骤 1-7 与 P0-A-2 全部闭合（ADR-0010 v1.8，23 个工作工具单一探针面）。
- [x] FUS-SOURCE-WEIGHTING-IMPL：来源加权实现闭合（机械三档 + 机器可读种子名单 + 模型加权标注）。
- [x] GAP-ENCODING-GATE：机械编码门控闭合。
- [x] GAP-ACAF-SLICE1 / SLICE2A / SLICE2B / FAILCLOSED 与 GAP-DENIAL-POLICY-REVISION：ACAF 实施切片闭合（fail-closed 生产启用仍待裁决）。
- [x] OPS-PROTOCOL 审查判定登记（裁剪方向定案；裁剪设计待产出）。
