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

- 未闭合总数：**29 项**
  - P0-C：0 项（PLAN-FIRST 阶段 A/B/C 全部闭合，2026-08-16）
  - P0 评测冒烟暴露：0 项（P0-E 主项 7 项 + FUS-TOOL-SCOPE-CONTRACT 后续 2 项全部闭合 2026-08-18，见 P0-E grep 项后续①/②）
  - P0 Benchmark 完全体：1 项（FUS-BENCHMARK-FULL-EXEC 实施完成待验证，见 P0-F；验证闭环后回 27）
  - P0 折叠历史外挂：1 项（LEDGER-FOLD-EXTERNAL-FILE，S1/S2 已闭合、S3/S4 待验证，见 P0-0c；验证闭环后回 28）
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
- [ ] 闭合：验证全过 → BACKLOG/TODO/索引状态同步，未闭合 28 → 27。

### P0-0c LEDGER-FOLD-EXTERNAL-FILE（P0；2026-08-18 用户裁决：先设计、不实施；
同日用户指示优先实施——命中率问题优先于 P0-F 验证；S1/S2 闭合，S3/S4 待验证）

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
- [ ] S3（待验证）：Linux musl 重建（ORZ-BUILD-MOUNT-001，输出
  `D:/tb-eval/orz-linux`）+ 时间戳校验。
- [ ] S4（待验证）：make-doom-for-mips 单题复验——命中率 ≥90%（provider
  usage 口径）、无 400、journal 断言不变（`workspace.run_terminal` 订单 →
  ACAF 票据路径）；`ORZ_DEBUG_VIEW=1` 在验证③闭合时一并移除。
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
- [ ] D1=(c) S3（待验证）：Linux musl 重建（ORZ-BUILD-MOUNT-001，输出
  `D:/tb-eval/orz-linux`）+ 时间戳校验。
- [ ] D1=(c) S4（待验证）：命中率复验（≥90% provider usage 口径、无 400）。
- [ ] 闭合：S3/S4 全过 → BACKLOG/TODO/索引状态同步，未闭合 29 → 28。

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
