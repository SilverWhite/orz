# ORZ 统一待办与优先级（BACKLOG）

> 状态：living（单一待办路由）；建立：2026-08-13。
> 定位：本文件只做未闭合项召回、优先级和决策门登记；不替代 ADR、Schema、审计、索引或源码。设计裁决以 ADR-0010 / ADR-0011 和 [`CLI_PROJECT_INDEX.md`](../CLI_PROJECT_INDEX.md) 的 canonical entry 为准。
> 维护纪律：新增、关闭或调整优先级只在本文件登记；设计文档与审计的“待办/下一步”小节只保留指针或审计时点历史，不重复维护明细；索引只登记本文件的召回路由。
> 实施勾选清单：见 [`TODO.md`](../TODO.md)（派生投影，勾选状态随本文件同步；优先级、决策门与状态以本文件为准）。

## 优先级总览

| 优先级 | 含义 | 未闭合项 |
|---|---|---|
| P0 | 当前工作集：设计已冻结，裁决后立即实施 | 评测冒烟暴露问题（P0-E 主项 7 项 + FUS-TOOL-SCOPE-CONTRACT 后续 2 项全部闭合 2026-08-18：ACAF 容器供应、console 工具名、计划视图步骤 ID、订单发放前拒绝入事件面、grep 搜索范围契约、plan_write 校验消息形状、actions 形状探针、list_dir 范围计数、grep files_searched 全结局探针，见 0a）；CLASSICAL-EXEC-ASSISTANT（生产组件，2026-08-16 用户裁决转正式，S1-S4 全部闭合）；PLAN-FIRST-BLACKBOARD（阶段 A/B/C 全部闭合 2026-08-16）；**FUS-BENCHMARK-FULL-EXEC（P0，实施完成待验证——2026-08-18 用户指示实施、暂不测试；验证闭环后闭合，见 0b）**；**LEDGER-FOLD-EXTERNAL-FILE（P0，S1-S4 验证闭环 2026-08-19——命中率问题优先于 P0-F 验证；provider 口径 95.33% ≥90% 达标，见 0c）**；**OUTPUT-DEGENERATION-GUARD（P0，S1-S4 全部闭合 2026-08-20——make-doom 退化复读失败防护；8K 全统一 + 补读闭环 + 实时检测 + 32K，见 0d）**；**CONTEXT-SCAFFOLDING-PULL-REDESIGN（P0，S1-S4 全部闭合 2026-08-21——预算块 PUSH→PULL + 工具输出汇总消息退役；命中率 94.45%、零哨兵触发、输入增长放缓；方案 C 维持 256K 暂不收紧（用户裁决），见 0e）**；**FUS-READ-ANCHOR-WRITE-GUARD（P0，S1-S4 全部闭合 2026-08-23——read_file 内容锚点下传 + 写前机械核证；S4 复用 NGRAM S4 实机复验：10 试次零误拒、锚点实机可见、命中率 94.11%–98.55% 全 ≥90%、零 400，计数 28 → 27，见 0f）**；**AGENT-DELIVERY-FLOW（P0 派生 0d 后续 7，S1-S4 全部闭合 2026-08-23——计划无空转、末步机械递交、引用修正一次/二次阻断、订单反馈；S4 复用 NGRAM S4 实机复验：8/8 完成试次走 submit 双阶段、零 400、命中率全 ≥90%，计数 29 → 28，见 0d 后续 7）**；**FINAL-SMOKE-2026-08-25 对拍暴露（P0，2026-08-25 登记：GAP-EVENT-SCHEMA-DRIFT 事件面三类 Schema 漂移（2026-08-26 修复完成并复验，事件链非终止错误 0）+ GAP-REPETITION-DETECTOR-DNA-FALSE-POSITIVE 复读检测 DNA 误杀（S1/S2/S3 完成 2026-08-26、S4 复验待实施），正式 89 题提交前处理，见 0i）** |
| P1 | 无需裁决，可与 P0 并行 | FUS-COMPONENT-REGISTER、GAP-WINDOWS-EVIDENCE、IMPL-DEEPSEEK-TRANSPORT / SEC-CREDENTIALS、ORZ-SESSION-CONTEXT-MONITOR |
| P2 | 生产化决策门：需用户裁决 | IMPL-CONTROL-FABRIC（fail-closed 启用、Slice 3/4）、OPS-PROTOCOL |
| P3 | 收尾 / 清理 | EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、DC 剩余信号、V11-IMPL-003/007、工作区收尾 |

- 复杂度治理判定（2026-08-13）：LIF 为高要求主项目、实验含相当程度自动运行；复杂度降低只砍冗余（OPS 平行执行层、双实现、文档仪式），保留服务 LIF 不变量的机制（journal/verifier、permission fail-closed、运行守卫、Windows 进程控制、来源证据、ACAF Slice 1/2）；ACAF Slice 3/4 暂缓，按实际自动化模式再定；不做机制×不变量清单，避免后续审查被带偏。
- DSH 借鉴复核（2026-08-14，用户裁决）：orz 自身（除成熟底座外的一切）即整体化二进制薄层，底座可较简单切换；个人开发者无插件生态，不支付子系统化复杂度。三项机制复核结论——A（Windows ACL 沙箱）挂起不立项；B（文件观察策略）收编为 `workspace.search_replace` 动作契约规则（随 CLASSICAL-EXEC-ASSISTANT 小样 2 裁决）；C（工具结果裁剪）收编为纯函数（随 COMPACTION-REDESIGN S2 或 50K 注入预算实施）；其余 DSH 层已覆盖或不适配，不引入。本复核不新增独立实施项。

## P0 — 当前工作集

> 2026-08-13：P0 决策登记——FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH 经用户裁决放行实施；执行顺序 P0-A（工具探针）优先，P0-B（检索机械控制）紧随，P0-C（经典操作台）POC 已通后进入实施序列。

### 0a. 评测冒烟暴露问题（最优先；2026-08-17 登记；P0-E 主项与 FUS-TOOL-SCOPE-CONTRACT 后续 2 项全部闭合 2026-08-18）

> 来源：正式跑分前最难错题试跑——make-doom-for-mips（TB2 2.0，deepseek-v4-flash，
> plan-first + console 默认 + headless，2026-08-17 01:05–01:12）。证据：
> `D:\tb-eval\jobs\2026-08-17__01-05-31` / `2026-08-17__01-07-59`（ACAF 拒启）、
> `D:\tb-eval\jobs\2026-08-17__01-09-43`（工具名 400）；journal
> `D:\tb-eval\gsa-volumes\b4-900s\a5da4937-2775-47ff-8202-98bf03a74780\runs\RUN-CLI-6a81eeed\events.jsonl`
> （plan-first 首轮→计划接受→console 面→run_failed 全链）。处理窗口=新 Codex 窗口。

- **GAP-ACAF-HARNESS-PASSTHROUGH**（P0，2026-08-17）：ACAF fail-closed 生产默认强制后，
  TB2 harbor 适配器（`tb_agents/orz.py`）只向任务容器转发固定环境变量集合，签发器配置
  （`ORZ_ACAF_MANIFEST`/`ORZ_ACAF_KEYSTORE`/`ORZ_ACAF_FAIL_CLOSED`）无法进入容器，
  `orz --real` 启动即拒（`assurance invariant: ACAF fail-closed is enabled but no signer
  client is configured`）。本轮已做临时解阻：`D:\tb-eval\.env` 加 `ORZ_ACAF_FAIL_CLOSED=0`
  （影子模式）+ 适配器增该变量透传。**2026-08-17 用户裁决：跑分保持 ACAF 强制开启**——
  容器内供应 manifest/keystore/signer（参考 [`scripts/orz_acaf_run.ps1`](../scripts/orz_acaf_run.ps1)
  供应链），不接受影子模式；实施=适配器透传签发器配置 + 任务容器挂载/供应 + 移除
  `D:\tb-eval\.env` 的 `ORZ_ACAF_FAIL_CLOSED=0` 覆盖 + 冒烟验证 `orz --real` 带签发器启动。
  **2026-08-17 实施（本窗口，已闭合）**：前置设计变更——Linux 下 DPAPI 不可用、signer/provision
  原 fail-closed（Slice1 审计 §6 登记边界），用户裁决新增 `file-0600-installation` 文件型
  安装密钥库（ADR-0010 §14.21 项 4 / ADR-0011 §5）。orz 子模块 e8274e1：keystore.rs
  文件型后端（0600、MAGIC‖secret、schema 同构）+ storage 分派 `load_installation_secret`
  + provision 非 Windows 分支 + signer 分派加载；orz-bin 全量通过（acaf_e2e 23 /
  signer 14 / provision 2 / stdio_e2e 1 / real_flag 2）；schema 枚举与正向 fixture 同步、
  仓库门禁 valid（1401 条目）。适配器 `tb_agents/orz.py`：install 上传 orz-signer /
  orz-acaf-provision 并在容器内 `/etc/orz-acaf` 落 manifest+keystore，run 设
  ORZ_ACAF_MANIFEST / ORZ_ACAF_KEYSTORE / ORZ_ACAF_BINARY / ORZ_ACAF_FAIL_CLOSED=1；
  `D:\tb-eval\.env` 已移除 `ORZ_ACAF_FAIL_CLOSED=0`。Linux musl 重建（父仓库挂载
  `/orz`、工作目录 `/orz/orz`，15m45s）后容器冒烟验证通过——provision 落 0600
  keystore（`installation-key.bin`）+ manifest 哈希真实 signer 二进制；signer stdio
  `initialize_session` 应答（K_install 自文件密钥库加载成功）；`orz --real` 带签发器
  启动 3 次成功（启动期 fail-closed 不变量通过=signer 客户端已配置、wallclock 正常收尾
  `run_invalidated`、退出码 0；强制写入订单被步骤门拒属 P0-E 计划视图缺口，与本案无关；
  DeepSeek 传输层瞬时错误 1 次重试成功）。冒烟 journal 证据：
  `D:\tb-eval\jobs\2026-08-17__05-45-ACAF-SMOKE`。
- **GAP-CONSOLE-TOOLNAME-PATTERN**（P0，2026-08-17；**已闭合 2026-08-17**）：console 默认面三个工具名含点号
  （`blackboard.action_write`/`console.step_done`/`console.return_to_console`），违反
  OpenAI 兼容工具名模式 `^[a-zA-Z0-9_-]+$`；计划落板后下一轮请求 400（`invalid_request_error`）
  → `run_failed`。FakeProvider 不校验工具名，orz-loop 433 单测未暴露。修复=改名下划线
  （`blackboard_action_write`/`console_step_done`/`console_return_to_console`），同步约
  91 处 Rust、Python verifier 交叉校验（`run_event_journal_validation.py` 中
  `payload.get("tool") == "blackboard.action_write"` 等）、schema 注释、设计文档；改后
  重建 Linux 二进制并重跑。闭合证据：orz 子模块 0304b23（11 文件 91 处 + fmt 收口）；
  orz-loop 434 / orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 + real_flag 2 通过；
  clippy 无新增告警；manifest 重生成 1401 条目、仓库门禁 valid；Linux musl 重建后冒烟
  重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57`（30m21s 跑满 1740s 预算、
  `run_invalidated{wallclock}` 正常收尾，对比旧运行 400 即死）。
- 冒烟重跑（`D:\tb-eval\jobs\2026-08-17__03-48-57`，工具名修复后）任务结果 reward 0.0——
  机制层面通过（无 400/无异常/跑满 1740s 预算），任务层面未完成（无 ELF/无帧）。失败原因
  定位：① **步骤门模型面缺口**——`blackboard_read section=plan` 只渲染
  `[status] goal (actions: N; evidence: M)`，不渲染步骤 `id`；步骤门又要求订单
  `step_id` 精确绑定，模型只能猜测（轨迹 13/18/23 步自述 "the plan view strips them /
  my guessed step_id values get rejected"），导致大量读板/计划重写轮次（4 次 plan_write）；
  ② grep 系统性空结果（2026-08-17 复核更正归因）——轨迹中两次
  `doomgeneric_mips|frame\.bmp` grep 实际被 plan/console 门机械拒绝、未执行
  （journal 序列 24/38）；执行的 7 次 grep 全部无匹配（wall_ms 1–36ms，含 vm.js
  实测存在的 entryPoint/syscallNum/runElf 等），模型将浅层 list_dir 与系统性空
  grep 叠加泛化为「/app 无 C 源码」并一度错误转向（疑工具层搜索范围/路径解析异常，
  非模型纪律问题）；③ 预算耗尽于侦查/步骤门摩擦，未及完成 ELF 构建（vm.js 契约已
  正确读出，最终发起 search_replace 但未闭环）。
- **P0-E 下一步实施项**（2026-08-17 对齐确认）：
  1. GAP-ACAF-HARNESS-PASSTHROUGH 实施（用户裁决：跑分保持 ACAF 强制，容器内供应）；
  2. plan_write 校验消息/形状机械明确（P1 观察①：首次模型把计划序列化为 JSON 字符串
     被拒 `missing_required_field: plan` → refill，重填对象后通过；**2026-08-17
     用户复核：放弃特化示例方向**——单点偶发（重跑 4 次 plan_write 均为正确对象），
     示例强化属过拟合；改为校验错误消息写明形状（plan 必须是含 plan_id/goal/
     steps[] 的对象，got string 时明示）并补回归测试：字符串计划 → 机械拒绝 →
     对象重填）；**2026-08-17 闭合**——`parse_and_validate_plan`：plan 缺失报
     `missing required field: plan (expected an object with plan_id / goal /
     steps[])`、非对象报 `plan must be an object with plan_id / goal / steps[]
     (got string)`；回归测试=字符串计划→机械拒绝→错误含形状说明（orz 子模块
     11540fa）；
  3. `steps[].actions` 形状校验收紧（P1 观察②：实证审计空/宽松形状并补探针测试；
     **2026-08-17 闭合=实证审计 + 探针测试锁定**——14 种宽松形状（空串/裸串/
     空对象/null/缺 with/缺 do/空 step_id/step_id 不匹配/with 错类型/actions
     非数组等）全部被机械拒绝，「空字符串仍通过校验」原观察不成立（与 grep 归因
     更正同类），无需收紧；`with` 内容与 `do` 注册表核对仍留订单发放时契约校验
     （plan 层只约束结构与长度，ADR-0010 §10 不变量））；
  4. **计划视图渲染步骤 ID**（新发现，步骤门模型面闭环：`section=plan` 补 `step.id`，
     模型无需猜测；ADR-0010 §14.21 登记；**2026-08-17 闭合**——`epoch.rs` plan 段
     每步行首渲染 `step.id`（live 视图与归档 epoch 读同源）+ 状态行当前步补
     `[step_id]` + `blackboard_read`/`blackboard_action_write` 描述补取 id 提示；
     测试三层（epoch 渲染单测 / 工具级回达 / 跨 epoch 归档读），orz-loop 436、
     orz-tui 178、orz-assurance 152、orz-bin 全量通过，clippy 无新增告警，
     manifest 1401、仓库门禁 valid；orz 子模块 0d1e01b）；
  5. **grep 搜索范围与空结果语义**（新观察，用户 2026-08-17 确认一并处理；
     **2026-08-17 复核更正归因 + 用户复核定案**——撤回「补文本范围报告」方向，
     定案=工具契约升级）：两次 `doomgeneric_mips|frame\.bmp` grep 实际被门机械
     拒绝未执行；执行的 7 次 grep 全部无匹配（wall_ms 1–36ms），含 vm.js 实测存在
     的 entryPoint/symbolName/sectionsToLoad/syscallNum/runElf/program counter。
     根因在工具层：`finalize_grep` 将 exit 1 + 空 stdout（或 exit 2 +
     "No files were searched"）统一转为 "No matches found"，而 ORZ 总传显式路径、
     rg 不打印 "No files were searched" 警告（该分支死代码），「rg 搜索 0 文件」
     （ignore/隐藏/glob/二进制/超限过滤干净）与「真无匹配」机械不可分。定案实施
     方向=grep 返回结构化搜索信封（resolved root / files_searched /
     files_skipped / match_count / truncated，机械来源 `rg --stats` 或
     `--json`）+ 结局三型分型（searched>0 有匹配 / searched>0 无匹配=真无匹配 /
     searched=0=范围空，显式报过滤类别，不叫 "No matches found"）+ 搜索范围语义
     显式化（grep 默认与只读工具可见集对齐，或显式 `--no-ignore`/`--hidden`
     开关，二选一按容器内冒烟结果定）+ 契约泛化（读/搜/列三族统一，list_dir 补
     ignored/truncated 计数）；exit 2 语法错误保持硬失败。实施前置=容器内 grep
     冒烟（对已知字符串断言匹配 + `rg --debug` 定位过滤来源）；回归验证（重跑
     round 数/计划重写次数下降 + searched=0 分型断言）。设计登记 ADR-0010
     §14.23（v1.23）/ 操作台设计 §12 / FUS-TOOL-SCOPE-CONTRACT；模型侧侦查纪律
     （先 list_dir 建清单、pattern 用实测存在的字符串、空结果≠无文件）与注册板块
     grep 参数提示降为次要契约提示；
     **2026-08-17 冒烟结论（容器内实机复现，`alexgshaw/make-doom-for-mips:
     20251031`）**：根因=构建侧打包了 glibc 动态 rg（`GROK_TOOLS_BUNDLE_RG_PATH=
     /usr/bin/rg`，rust:1.97-slim/trixie 产物，要求 GLIBC_2.39）进 musl-static
     orz 二进制；任务容器为 bookworm（glibc 2.36）加载失败——`version
     'GLIBC_2.39' not found`，退出码恰为 1、stdout 空，stderr 在
     `finalize_grep` exit-1 分支被丢弃，全部 7 次 grep 因此统一显示
     "No matches found"。同一容器装正常 rg（13.0.0）后：精确 orz 命令对
     /app/vm.js 搜 `syscallNum` 命中 20 处（`--stats`: 10 files searched），
     /app/doomgeneric 的 .gitignore 只忽略构建产物——默认搜索语义无问题。
     **a/b 定案=保留 rg 默认语义（b）**：grep 继续尊重 ignore/隐藏，参数面新增
     `--no-ignore`/`--hidden` 开关（与 glob 同进）；工具契约必须补两点——
     ①任何非零退出且 stderr 非空必须显式报错（先于空结果判断，杜绝把加载/运行
     失败吞成 "No matches found"）；②`--stats` 解析出 files_searched 等入搜索
     信封。**构建侧修复**（同项实施）：Linux musl 构建不再用 glibc 覆盖路径，
     改走 build.rs 官方静态 musl ripgrep 下载（或显式静态 rg 路径）；冒烟回归=
     容器内 `rg --version` 可运行 + 已知字符串断言匹配。
     **2026-08-17 实施闭合**：工具契约——`finalize_grep` 结局三型（非零退出 +
     stderr 非空→显式报错；`files_searched=Some(0)`→"Searched 0 files…Retry
     with --no-ignore/--hidden"；空 stdout + searched>0→"No matches found in
     N files"；exit 2 硬失败保留）；机械来源定稿=v1 用 `rg --files` 探针（仅空
     结果路径、与主搜索同过滤集、10K 计数截断；弃用 `--stats`——rg 15 stdout /
     旧版 stderr 位置差异会污染流式面一致性）；参数面新增 `hidden`/`no_ignore`
     开关（含 console 注册表 schema）；`GrepSearchOutput.files_searched` 入信封。
     构建侧——build.rs 对非 Windows 覆盖路径加 ELF PT_INTERP 静态链接守卫
     （动态即构建失败并提示），两份评测构建脚本改 `cargo install ripgrep
     --target x86_64-unknown-linux-musl` 产静态 rg。测试：grep 模块 42 /
     types 561 / orz-loop console 68 通过。**2026-08-17 冒烟回归已执行**：
     Linux musl 重建（10m04s，构建脚本补 `make`）后打包 rg 静态、任务容器内
     `rg --version`=15.0.0 + 已知字符串断言命中；端到端 `orz --real`
     （ACAF 签发器 + 文件密钥库）grep vm.js `syscallNum` 报告命中 20 行并
     done、exit 0（对照旧二进制 7/7 "No matches found"）；证据
     `D:\tb-eval\jobs\2026-08-17__GREP-FIX-SMOKE\smoke-notes.md`。
     P0-E 剩 2 项、未闭合 29 项。
  6. **订单发放前拒绝入事件面**（新观察，用户 2026-08-17 指示处理；**2026-08-17 已闭合**）：
     冒烟重跑中 ORD-000011（workspace.run_tests，`arguments:{}`）写入后发放前被拒，
     失败只进结果栏 receipt + TraceStore（`consume_console_order` 不写 journal 事件），
     事后核对看不到拒绝码。实施=v0.2 `console_order_rejected`（order_id / step /
     phase / code / reason / round / plan_epoch / run_id），发放前拒绝统一入事件面——
     phase=pre_issue（order_stale / step_not_done / budget_insufficient，
     step=protocol）+ phase=issue（registry / contract / target / ACAF / policy /
     mode 门，ACAF/模式/权限归一化 step=policy / code=policy_denied）；
     execute/verify 不入本事件（已执行订单经 tool_started/tool_completed 留痕）。
     Schema/verifier/fixtures 先行（verifier 交叉核对：拒绝须先有同 run 同 order_id
     的 `console_order_written`、机械盖章一致、每订单至多一次拒绝、phase/step/code
     一致性）；结果栏 receipt 保留为人类可读视图。orz 子模块 c67a452（事件变体 +
     三处 pre_issue 路径 + 发放期 Err 分支 issue 路径发事件 + TUI 投影 +
     测试断言 stale/step_not_done/budget×3/policy）；orz-loop 436 / orz-tui 178 /
     orz-assurance 152 / orz-bin 全量通过、clippy 与基线一致（lib 21 / lib test 26）、
     manifest 重生成 1401、仓库门禁 valid。
- **FUS-TOOL-SCOPE-CONTRACT 后续项 2 项（2026-08-17 用户指示补记进计数；
  未闭合 27 → 29；**2026-08-18 全部闭合，29 → 27**）**：grep 面实施审计
  「边界与后续项」两条正式入账，P0-E 主项仍 0 项、后续 2 项：
  1. **list_dir 目录信封（2026-08-18 闭合）**——`ListDirContent` 增
     listed/ignored/truncated 机械计数：ignored=未过滤走（关 hidden/ignore
     全系过滤）−可见走（同过滤语义；SCOPE_COUNT_CAP=200K 封顶，超限为下界、
     可见侧触顶报 None）+ truncated=可见总数−实际渲染（含条目上限与字符预算）；
     卡片尾部附 `(scope: ...)` 脚注；legacy/codex 面保持 None 不报；
     描述模板补范围脚注说明。
  2. **grep files_searched 全结局探针（2026-08-18 闭合）**——机械来源收敛
     为 v1 `rg --files` 探针，扩展为每次完成搜索都运行（命中亦返回；
     摘要行内嵌 `(searched N files)`；错误路径 stderr 非空保持 None）；
     `--stats` 跨 rg 版本位置差异污染流式面、`--json` 需重写输出契约，
     均不采用（位置收敛定案=探针）。
  两项均无裁决依赖；实施（orz 子模块 614bb3b）：grep 99 / list_dir 60 /
  orz-tools 全量 2761 通过、clippy 无新增告警。入口见
  [`grep 面实施审计`](audits/FUS_TOOL_SCOPE_CONTRACT_GREP_IMPL_AUDIT_2026-08-17.md)
  边界与后续项 / [TODO P0-E](../TODO.md) / ADR-0010 §14.23。

### 0b. FUS-BENCHMARK-FULL-EXEC（P0；`pending`=实施完成待验证，2026-08-18 用户裁决实施）

- 入口：[设计](BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；ADR-0010 §14.24（v1.24）/
  CLASSICAL-EXEC-ASSISTANT §13 / PLAN_FIRST_BLACKBOARD §4。
- 来源（2026-08-17 TB2 冒烟，`D:\tb-eval\jobs\2026-08-17__23-29-44`，
  make-doom-for-mips reward 0）：Benchmark 配置 shell-less 导致三层全关——
  权限层按名级排除 shell 工具、探针层 `policy_allows_exec` 仅 Interactive、
  console 注册表无 `run_terminal_cmd` 动作；2026-08-18 用户裁决 orz 完全体
  （shell 不开放为模型直接工具，执行全经助理层订单，与 run_tests 同构）。
- 实施路由登记：2026-08-18 实施前登记本项与 [TODO P0-F](../TODO.md)（设计轮
  不动计数；本实施轮入账 1 项，未闭合 27 → 28，验证闭环后 28 → 27）。
- **2026-08-18 实施完成（用户指示：实施、暂不测试）**，orz 子模块
  `3f43478`（feat/fusion-architecture，6 文件 358+/24-，见 TODO P0-F）。
  三层同时使能：
  1. 权限层 `PermissionPolicy::Benchmark { allow_shell, allow_network }`
     （默认 false/false 保持旧语义与旧测试）；决策表=ReadOnly 恒走 manager、
     LocalMutation 非 shell AllowOnce（不变）、shell 工具与 SandboxEscape
     （bash/sh/cmd/pwsh）在 allow_shell 下 AllowOnce、NetworkCall 在
     allow_network 下 AllowOnce（web_fetch/web_search 直调面）、MCP 恒 deny、
     工作区读限定不变。
  2. 探针层 `ToolPolicy::BenchmarkFull`（`tool_policy()` 由
     `Benchmark{allow_shell:true,..}` 映射；`policy_allows_exec` 增
     BenchmarkFull；console `ActionBundle::allows` 加臂复用 benchmark 档）。
  3. console 注册表 `workspace.run_terminal`（target=run_terminal_cmd、
     kind=Host、bundle=READ_WRITE；input 镜像 BashToolInput：command/
     description 必填、timeout 1–300000 可选默认 120000、is_background 可选
     默认 false、additionalProperties=false、不暴露 env/cwd；响应
     `{"output": string}` 信封；动作栏仍由探针收敛）。
  4. CLI `--allow-shell`/`--allow-network`（headless benchmark 专用 →
     ORZ_ALLOW_SHELL/ORZ_ALLOW_NETWORK，沿用 --allow-write 先例）；未带
     `--allow-write` 时 exit 2（fail-closed，防静默无效）；`--help` 同步。
  5. 适配器 `tb_agents/orz.py`：`allow_shell=True`（TB 本质 shell 评测）、
     `allow_network = environment.network_policy.network_mode == PUBLIC`
     （实施注记：取 trial 按 agent 阶段设置的有效 network_policy 而非
     task_env_config 基线——89 题全 PUBLIC 结果一致、严格不更宽；
     allow_internet 已废弃）；env 按存在性增 ORZ_ALLOW_SHELL/ORZ_ALLOW_NETWORK，
     运行脚本 belt-and-braces 同传 `--allow-shell`/`--allow-network`。
- **2026-08-18 审查收口处理（全面审查后）**：① `is_shell_tool`（permission.rs /
  tool.rs）补 `sh` 名级兜底（默认轴 Deny / allow_shell 下 AllowOnce，与设计
  §3 名单一致；两轴测试补断言）；② `workspace.run_terminal` timeout 契约改
  anyOf（integer 或纯数字字符串、补 default 120000）、is_background 补
  default false——对齐 BashToolInput lenient 数字语义，非数字字符串在契约层
  显式拒绝（新增契约测试）；③ CLI `--allow-shell=<v>` / `--allow-network=<v>`
  值形式由静默忽略改显式报错 exit 2（解析抽 parse_benchmark_flags + 4 组
  单测）；④ bundle 保持 READ_WRITE 实施选择确认（交互式 console 亦出现按钮、
  走 Interactive 权限询问）；⑤ `is_background` 后台任务完成提醒的 console 面
  可见性留验证④实机观察。详见设计 §12。
- 安全面不变：ACAF fail-closed 票据（command_exec_v1/network_v1）仍为最终
  授权兜底；PermissionRequested/PermissionDecision、ACAF issued/consumed、
  ToolStarted/ToolCompleted、console_order_written/rejected 审计链全部保留；
  预算/墙钟/停滞守卫与模式门不变；「放开」=策略允许面，非审计面。
- 待验证（2026-08-18 用户指示暂缓，同日放行执行）：① orz cargo 测试
  （权限决策表、探针映射、console 注册表投影、订单→run_host_tool→ACAF
  票据路径）+ clippy 无新增告警——**2026-08-18 已闭合**：orz-loop 453 /
  orz-host 221 / orz-tui 178 / orz-assurance 152 / orz-bin（lib 11 +
  benchmark_flags 14 + acaf_e2e 23 + real_flag 2 + stdio_e2e 1）/
  orz-tools 2761 全绿；clippy 无新增可归因告警；manifest 1401 + 仓库门禁
  valid；过程中修复 PLAN-FIRST/console 双模式落地后的既有测试漂移
  （codex_app 12 + acp_server 1，orz c4772fc；orz-host 需
  `--test-threads=1` 规避负载敏感超时竞争）；② Linux musl 重建
  （ORZ-BUILD-MOUNT-001，输出 `D:/tb-eval/orz-linux`）；③ 单题
  make-doom-for-mips 复验（reward > 0、journal 出现 `workspace.run_terminal`
  订单→run_host_tool→ACAF `command_exec` issued/consumed、无 400/无异常
  policy_denied）——**2026-08-18 取证进展**：核心机制已验证（订单→发放→
  run_terminal_cmd exit=0、ACAF 票据路径生效），但三次复验均因 400
  （`insufficient tool messages`）退出；**根因复核修正（处理文档
  `docs/LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING_2026-08-18.md`）**：真正
  破坏点=压缩触发（未执行）时 `run_template_compact` 顶部 retain 删除
  marker 而折叠索引未失效（GuardBlocked 无 reset），冻结 preamble 吞入
  首轮 plan_write 声明（其回复在折叠区）——折叠 cut 本身始终在完整轮起点；
  `safe_fold_cut` 只防 cut 不防 fold_start（idx==0 兜底为防御项之一）；
  取证存档 `D:\tb-eval\jobs\2026-08-18__08-44-56\ROOTCAUSE_FORENSICS_20260818.md`
  （orz 3bd09fc/5bc3add 取证 WIP）。**修复已实施闭合（2026-08-18，处理
  文档 S1-S5）**：S1 代码修复（`run_template_compact` retain 后移 + 执行
  路径 kept_start 重算；`safe_fold_cut`→`Option` + `build_request_view`
  preamble 校验）；S2 新增 6 项单测，orz-loop 全量 460 通过、fmt 干净、
  clippy 无新增告警；S3 Linux musl 重建三件套时间戳更新；**S4 复验
  （`D:\tb-eval\jobs\2026-08-18__19-40-12`）**：0 异常、无 400，会话跑满
  29 分钟墙钟——`context_compressed`（fallback 终止态）执行后继续 102 条
  事件零失败（此前必现 400 的场景已闭环）；6 笔 console 订单→5 组 ACAF
  control_ticket issued/consumed、零 permission 拒绝，机制断言全过；
  **reward 仍 0**：agent 未在墙钟内产出可运行 `doomgeneric_mips` ELF
  （验证器 `node vm.js` 超时、`/tmp/frame.bmp` 缺失）——任务完成度问题，
  非机制回归，验证③ reward 项保持开放（可加预算重跑）；`ORZ_DEBUG_VIEW=1`
  暂保留并登记为常驻诊断（验证③闭合后移除）；④ 2–3 题交叉
  （build/run 类 compile-compcert、网络类
  hf-model-inference）；⑤ `run_official_2.1.sh` 89 题 5 批。

### 0c. LEDGER-FOLD-EXTERNAL-FILE（P0；2026-08-18 用户裁决：先设计、不实施；
同日用户指示优先实施——命中率问题优先于 P0-F 验证；S1-S4 全部闭合
2026-08-19，计数 29 → 28）

- 入口：[设计](LEDGER_FOLD_EXTERNAL_FILE_DESIGN_2026-08-18.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；ADR-0010 §14.28（v1.28）；
  取代 `LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md` §3.2/§3.5 视图内台账块。
- 来源：缓存命中率复验（`D:\tb-eval\jobs\2026-08-18__19-40-12`，57 请求）
  实测 81.9%（miss 1,116K；39 次折叠推进 + 1 次压缩贡献 ~1,044K = 93.5%）；
  根因 A 触发线不复位 / B 每次推进重写视图内台账块 / C 压缩整段重写。
  2026-08-18 用户裁定：先设计、不实施（设计轮不动计数）。
- **2026-08-18 用户指示优先实施**（「得先处理命中率问题，不然成本太高了」）
  ——实施前登记本项与 [TODO 0c](../TODO.md)（实施轮入账 1 项，未闭合
  28 → 29；验证闭环后 29 → 28）。
- **2026-08-18 S1/S2 已闭合（orz 子模块提交见 ADR-0010 §14.28）**：
  S1 代码——action_ledger 外挂文件（`{session_cwd}/.gsa/ledger/current.md`，
  per-row 全局序号跨压缩连续、尾行续号 + 原子追加）、固定指针消息
  （`folded_ledger` 语义改写，首次推进设置后字节稳定）、`advance_fold`
  返回 `Option<Vec<ActionLedgerRow>>`（仅新增行、IO 由调用方执行）；
  agent_loop 推进写文件（失败回滚 fold + 重试）+ 压缩 marker 路径提示；
  controller `fold_tail_rounds` 默认 1 + `ORZ_FOLD_TAIL_ROUNDS`；
  summary marker「历史摘要累积于」行 + 归档段改外挂指针。S2 测试——
  action_ledger 18 / summary 12 / controller 折叠 e2e 2；orz-loop 全量
  462 通过（-j 1）、fmt 干净、clippy 与基线一致（lib 21 / lib test 26）。
- **2026-08-18 二次审查修复（S1 收口，orz 提交见 ADR-0010 §14.28）**：
  写失败不再 `continue` 空转（连续 3 次失败禁用折叠 + 新增事件面
  `ledger_fold_write_failed`，schema v0.2 同步）、外挂文件仅主车道（检索
  车道不折叠）、行格式 `[全局序号]`/`轮次` 解耦、`tail_seq` 长行稳健化 +
  损坏报错、`advance_fold` 落行前 preamble/safe_fold_cut 校验、marker
  条件路径提示、`view_estimate_after` 触发复位断言。orz-loop 全量 468
  通过、fmt 干净、clippy 与基线一致；计数不变（仍在 29，S3/S4 验证闭环
  后 29 → 28）。
- 待验证：S3 Linux musl 重建（ORZ-BUILD-MOUNT-001，输出
  `D:/tb-eval/orz-linux`）+ 时间戳校验；S4 make-doom-for-mips 单题复验
  （断言命中率 ≥90%（provider usage 口径）、无 400、journal 断言不变
  ——`workspace.run_terminal` 订单→ACAF 票据路径；`ORZ_DEBUG_VIEW=1` 在
  验证③闭合时一并移除）。验收 DoD 见设计文档 §7。
- **B 定案（机械压缩；2026-08-18 用户裁决：D1=(b)，ADR-0010 §14.29）——
  实施已闭合**：S4 复验账单对账暴露「未处理的部分」= 压缩摘要调用以独立
  系统提示词重付整段视图 miss（22:17 运行 10 个账单请求无 journal 对应、
  额外 miss ≈ 676K、账单口径命中率 89.71% <90%；摘要调用两轮全部失败零
  产出）。定案=纯机械压缩：`run_template_compact` 移除模型摘要调用
  （agent/cancel/heartbeat 退役、`CompactDecision::Executed` 简化），
  五段槽位=黑板（目的/计划/路径）+ 固定机械占位（注意事项/后续衔接，
  阶段 (c) HA 结构化事实聚合落地前）；存档恒写入、marker 恒带 digest、
  无 `summary_incomplete` 终止态；事件 `mode=mechanical`（schema enum 保留
  template_summary 回放）；fallback 紧急截断保留。实现=orz 子模块提交
  （2026-08-18）；orz-loop 465 通过、fmt 干净、clippy 与基线一致
  （lib 21 / test 26）、Python verifier 214 通过。计数不变（仍在 29——
  0c 的 S3/S4 复验闭环后 29 → 28；阶段 (c) HA 事实聚合待重看 HA 项目后
  另行裁决）。
- **2026-08-19 全面审查处理（B 定案收口）**：fallback 轮数口径修复
  （`dropped` 累加 + 存档/marker 总轮数 + 事件估计 marker 后重算，新增
  单测 `fallback_second_stage_truncation_accounts_total_rounds`）；补
  `ledger_fold_write_failed` payload schema / fixtures / verifier 交叉
  校验 / 生成器（conformance 52→53，闭合 2b755d6 遗留红）；
  `orz_source_manifest.sha256` 重生成；生成器 `context_compressed` 模板
  对齐 mechanical（防重生成回退）；文档/注释/schema 描述清理（summary.rs
  退化门注释、schema 终止态字段、LEDGER 设计 widened tail）。
  orz-loop 466 / Python verifier+conformance 230 / 仓库门禁 valid。
  计数不变（0c S3/S4 复验闭环后 29 → 28）。
- **2026-08-19 D1=(c) HA 结构化事实聚合设计定稿（用户裁决：先设计、不直接
  动作；纯文档登记、未实施；ADR-0010 §14.30 / 压缩设计 §4.4）**：重看 HA
  （Home Assistant）上游实现与源码后定稿——注意事项槽=HA 结构化事实聚合
  （助理层唯一新增输出；数据源=controller 已机械写入的 `plan.steps`
  Failed/Blocked + `exec.errors` 最近 5 + `actions.results` 失败 receipt
  最近 3；排序=计划面→执行错误→动作失败；空时「（无注意事项）」；≤3K 超限
  截断+指针；压缩内部失败继续走 marker 标注不进本槽）；后续衔接槽不交助理层
  （固定中性占位 + 回查入口，由主模型自行判断，避免机械性误导）；零模型调用、
  五槽 17K 上限、存档恒写入、marker 恒带 digest、schema 不变。实施路由：S1
  代码（`summary.rs` 聚合函数 + `run_template_compact` 接线）→ S2 测试 →
  S3 重建 → S4 命中复验。未闭合计数不变（29，0c 验证闭环后 29 → 28）。
- **2026-08-19 D1=(c) S1/S2 已闭合（用户放行实施；orz 子模块待提交）**：
  S1 代码——`summary.rs` 新增 `render_facts_notes`（HA 结构化事实聚合：
  `plan.steps` Failed/Blocked 步骤（step id+目标+receipt_id）→ `exec.errors`
  最近 5 条（每条截断约 200 字符）→ `actions.results` 最近 3 条失败 receipt
  （order_id/step/code/trace_id，信封字段缺失机械回退 `?`）；排序=计划面→
  执行错误→动作失败；空时「（无注意事项）」；≤3K 超限截断 + 「其余 N 条见
  blackboard_read 分区/摘要存档」指针；辅助 `render_notes_capped` /
  `truncate_chars` / `failure_envelope_fields`；后续衔接占位改中性措辞
  （「由主模型自行判断」+ 回查入口含外挂台账路径）；`run_template_compact`
  notes 槽接线 `render_facts_notes`；存档/marker 空 notes 防御回退同步为
  「（无注意事项）」）。S2 测试——summary 新增 6 项事实聚合单测（空态/三源
  排序/最近 5+截断/最近 3 失败含信封缺失回退/3K 溢出指针/单条超长退化指针）
  + 压缩 e2e 新增
  1 项（marker+存档三源事实槽同序）+ 空黑板 e2e 断言「（无注意事项）」；
  orz-loop 473 通过 / 0 失败、fmt 干净、clippy 与基线一致（lib 21 /
  test 26）。计数不变（0c S3/S4 复验闭环后 29 → 28）。
- **2026-08-19 D1=(c) S1 全面审查处理（审查结论：实现无代码缺陷、设计/
  实现/符合性成立）**：①登记口径更正——summary 事实聚合单测实为 6 项、
  orz-loop 473 通过（原 5 项 / 472）；②设计补充登记——O1 同一失败事件可
  同时以步骤行+动作失败行双视角呈现、属有意冗余；O2 单条超长整行退化为仅
  指针、N 计 1，由 blackboard_read 回查恢复；O3 `exec.errors` 黑板侧无界
  为已知边界、控制器侧加保留上限属可选后续（不占计数）——登记于压缩设计
  §4.4.1/§4.4.3 与 ADR §14.30；③CLI_PROJECT_INDEX 条目杂散控制字符清理。
  计数不变（0c S3/S4 复验闭环后 29 → 28）。
- **2026-08-19 命中率归因与黑板读取缓存成本设计定稿（S4 前置；用户裁决：
  大机制不再更改、只补回应命中而未命中的缓存成本）**：path-tracing 复验
  （正式 1800s 预算、40 请求、reward 0.0、无 400）provider 口径 85.43% /
  journal 86.04%；归因=**8/8 大 miss 尖峰（≥10K，合计约 190K = 总 miss
  64%）全部紧跟 `blackboard_read`（actions/exec）**——单次分区结果 17–32K
  token 作为全新工具结果注入，前缀缓存按位置匹配无法命中；非折叠频率、非
  大机制问题。定案=`blackboard_read` 渲染瘦身：actions 结果板去 response
  JSON（order_id/ok/step/code/trace_id，缺失回退 `?`）、exec 行截断 200
  字符 + 段总长 4K 字符上限、registration/order 板不变；（阶段 2 可选）
  `since` 扩展至 actions + 读取频率提示词引导；零模型调用、工具契约/schema
  不变、折叠（128K）/压缩（192K/200K）阈值不动；预期命中率 86%→95%。
  登记 ADR-0010 §14.31 / `BLACKBOARD_READ_CACHE_COST_DESIGN_2026-08-19.md`；
  实施路由 S1 渲染瘦身 → S2 测试 → S3 重建 → S4 复验（0c S4 前置，计数
  不变 29）。
- **2026-08-19 S1/S2 实施完成 + 全面审查 + F1 处理=方案 B 定稿（0c S4
  前置修订，未提交；B 未实施）**：S1 渲染瘦身已实施（orz 工作树未提交）——
  actions 结果板固定形态行（去 response JSON、缺失回退 `?`）、exec 行截断
  200 + 段 4K 上限；orz-loop 479 / fmt / clippy 基线一致。审查发现 F1
  （P1）：console 面动作详情不可回查（assistant.trace 不在直接工具面、其
  receipt 响应同样被瘦身隐藏；执行类订单 response / 失败 message/upstream
  均不可取回）——设计假设「经 trace 回查」机械上不成立。用户裁决=方案 B：
  `blackboard_read` 新增可选 `receipt_id` 点读（仅 actions；8K 字符上限
  截断+指针；支持 epoch 归档点读；非法/未找到显式报错；无 receipt_id 整段
  与 S1 逐字节一致）。零模型、数据面/事件面不动、工具定义增量扩展；路由
  S1 代码 → S2 测试 → S3 重建 → S4 复验；计数不变（仍在 29）。
- **2026-08-19 方案 B S1 代码 + S2 测试已实施（用户放行；orz 提交 ad74714、
  未推送；0c S4 前置修订）**：epoch.rs `render_section` 增可选 `receipt_id`（非
  actions 分区携带=显式报错；actions 分支点读优先）+ `render_receipt_point_
  read`（固定形态行 + 成功 `response=<JSON 完整内容（重序列化）>` / 失败
  `error=<JSON 完整内容（重序列化）>`——键/值/嵌套完整、非字节级原文，
  `RECEIPT_DETAIL_MAX_CHARS=8_000` 超限截断 + 「…」+ 存档/TraceStore 指针行；
  未找到显式 not found + 旧 epoch 归档提示）；controller.rs 参数解析
  （非字符串/空串=显式报错、exit_code 1）与透传（live 与 epoch 归档两条路径
  共用）、blackboard_read 工具定义参数/描述增量扩展（事件面不变）；S2 单测
  5 项（完整 response/error、8K 截断+指针、未找到、非 actions 报错、无
  receipt_id 与 S1 逐字节相等）+ 工具级 4 项（点读回达、非法参数报错、非
  actions 报错、跨 epoch 点读）；orz-loop 488 通过 / fmt 干净 / clippy 与
  基线一致（lib 21 / test 26）。S3 重建 → S4 复验（≥90%、无 400）待续；
  计数不变（仍在 29）。
- **2026-08-19 方案 B 全面审查处理登记（用户指示处理审查全部问题；orz 提交
  + 父仓库指针、未推送）**：N1 点读截断尾部记账注释修正；N2 receipt_id trim
  规范化口径登记（前后空白忽略、trim 后为空同空串报错）；N3 `blackboard_read`
  section 非字符串显式报错（绝不静默回退 "plan"，消除与 receipt_id 组合时
  误导性报错；新增工具级测试）；N4 提交状态措辞统一为「已提交、未推送」；
  O1 点读次数无机械上限登记为已接受边界（频率引导属阶段 2 §4.4 可选后续）；
  O2 当前 epoch 超 8K live receipt 尾部指针边界登记为已接受（8K 用户定档）；
  O3 「JSON 原文」措辞收敛为「重序列化完整内容」。实施侧：orz-loop 489 通过 /
  fmt 干净 / 无新增 clippy 告警；S3 重建 → S4 复验（≥90%、无 400）待续；
  计数不变（仍在 29）。
- **2026-08-19 折叠桥接截断设计定稿登记（用户裁决：形态甲 + 桥 8K 真实
  token + 触发 128K + 思维链不进桥/不进审计；先设计、不动作；ADR-0010
  §14.32 / LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN_2026-08-19）**：S4 复验
  （provider 88.84%）归因=3 次折叠重付 47.8K/50.4K/56.4K（66.5% miss）——
  折叠重付 ≈ 保留尾大小 + 新内容，本次保留尾实测 44–56K 真实 token。
  定案=折叠后其余进外挂台账、视图只留最新桥（默认 8K 真实 token，
  `ORZ_FOLD_TAIL_TOKENS` 可配；字符预算近似 + S4 实测校准）；形态甲=先定
  裁剪再对桥内容级截断（最新完整轮结构全保留、超预算只截内容 + 指针、
  保留尾部优先）；`reasoning_content` 剔除（思维链具幻觉性、不进审计——
  审计仅保留计数）；`fold_tail_rounds` 退役；指针文案更新一次；外挂台账/
  压缩/白名单/400 防线不变。推算命中率 → 约 94%（零折叠上限 95.9%）。
  实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（≥90%、无 400、每窗
  重付 ≤~15K、截断频率 ≤30% 校准）；0c S4 前置，计数不变（仍在 29）。
- **2026-08-19 S1/S2 实施闭合 + 全面审查处理（用户放行实施；orz 提交
  52d698c、未推送；ADR-0010 §14.32 第 2 项 / 设计文档 §3.1–§3.4/§7）**：
  S1 代码——`bridge_cut` 桥预算裁剪（完整轮累加至预算、最新轮恒入桥、
  不完整轮回退；4 字符/token ÷ 2 换算，默认 8K → 16K 估计口径）、
  `build_request_view` 桥视图（reasoning 剔除、超预算内容级截断=工具回复
  保留尾部 + 「…（前略）」+ sha256 指针、最终回复 199+…、声明 content/
  tool_calls 完整、消息不删轮不拆；`bridge_end` 冻结=推进时消息末尾、
  折叠之间纯追加）、`advance_fold` 入账范围不变、指针文案更新（「约 8K
  桥接内容，更早轮次已按行归档于 <abs-path>」）；`controller.rs`
  `fold_tail_rounds` 退役 → `fold_tail_tokens`（`ORZ_FOLD_TAIL_TOKENS`
  可配）；`agent_loop.rs` 推进与请求视图两处透传。S2 测试：orz-loop 503
  通过（+13 桥测试 +1 越界防御）/ fmt 干净 / clippy 与基线一致（lib 21 /
  test 26）。审查处理——N1 指针文案按定稿原文落地；N2 实现决策登记
  （新增 `bridge_cut`、`collapsed_cut` 保留服务压缩）；N3 `bridge_end`
  冻结机制补记；O1 中间 assistant 文本口径；O2 索引越界防御守卫；
  O3 非默认配置文案固定不变。S3 重建 → S4 复验（≥90%、无 400、每窗
  折叠重付 ≤ ~15K、截断频率 ≤30%）待续；计数不变（仍在 29）。
- **2026-08-19 S3/S4 复验执行 + 换算系数校准（用户指示重建 + 单题
  复验；orz 校准调整待提交；ADR-0010 §14.32 第 3 项 / 设计文档
  §3.2/§3.6）**：S3 重建成功（orz-linux 07:07 新二进制；USTC/清华镜像
  源 502 不可达，改用阿里云镜像源）。S4 单题复验（path-tracing 1800s，
  07:08 运行，job 2026-08-19__07-08-57）：reward 0.0（wallclock 耗尽）、
  **无 400**、113 请求；**journal 口径命中率 95.54%**（hit 4,775,422 /
  miss 222,869，对照上次 88.57%）；折叠 2 次（37/82 轮），**折叠后首
  请求重付 3,742 / 6,493 真实 token**（DoD ≤ ~15K，对照上次
  47.8K/50.4K/56.4K）、**截断频率 0%**（≤30% 达标）。校准：第二次折叠
  桥 12,948 字符 → 重付 6,493 → 实测 ≈ **2 字符/真实 token**，
  `FOLD_TAIL_CHARS_PER_TOKEN` 4 → 2（桥回到 8K 真实 token 目标；最新轮
  6.5K < 8K 不截断）；orz-loop 503 通过 / fmt 干净 / clippy 基线一致。
  provider 口径待账单 CSV（07:00–08:00 时段）对拍；计数不变（29，
  provider 对拍确认后 29 → 28）。
- **2026-08-19 provider 口径对拍确认 + 0c 验证闭环（用户上传新账单）**：
  provider 口径命中率 **95.33%**（hit 4,896,384 / miss 239,752 / 115
  请求，≥90% DoD 达标）；与 journal 口径（95.54%）差 2 请求（重试/
  边界），费用 1.9082 元验算吻合。S4 四项判定全达标（命中率 ≥90%、
  无 400、每窗折叠重付 3,742/6,493 ≤ ~15K、截断频率 0% ≤30%）——
  **0c 验证闭环，未闭合计数 29 → 28**；换算系数校准（4 → 2）为 S4
  既定产出，校准后代码待提交、下次正式跑分使用。登记于 ADR-0010
  §14.32 第 4 项 / 设计文档 §3.2/§3.6 / TODO P0-0c / 索引。

### 0d. OUTPUT-DEGENERATION-GUARD（P0；2026-08-19 用户裁决：先设计、不实施；**S1-S4 全部闭合 2026-08-20**）

- 入口：[设计](OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；ADR-0010 §14.33（v1.33）。
- 来源：2026-08-19 make-doom-for-mips 两次失败（模型退化复读 201K 字符 +
  160K max_tokens 放大 + 工具结果截断无可再读闭环；历史先例 2026-08-11
  479K 字符同任务；探针 60/60 排除网络；停滞守卫事后评估未拦截）。
- **2026-08-19 设计定稿（用户裁决：8K 全统一 + 桥 8K 不动 + 补读闭环
  硬约束 + 前两层补强）**：①限值统一 8K——终端工具输出 20K→8K、点读 8K
  确认、桥 8K 不动；②补读闭环硬约束——截断末尾"完整内容见 \<路径\>，
  请使用 read_file（offset/limit 分页）"，落盘 `.gsa/session/terminal/
  *.log` 可读性已验证，点读指针改向、桥截断保留工具结果自身指针；
  ③生成期实时复读检测（on_chunk 连续相同块 N=5 / 1K token 窗口重复率
  >60% / 连续 3 次退化中断 → run_invalidated）；④`REQUEST_MAX_TOKENS`
  160K→32K。准确度判定=8K 截断信息守恒、闭环吸收差异；桥 8K 不扩窗。
  实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（无退化中断、无 400、
  命中率 ≥90%、补读路径可用）；设计轮不动计数（28）。
- **2026-08-19 S1 代码 + S2 测试实施闭合（用户放行）+ 全面审查处理**：
  S1 落地（orz 提交 5e968ec + 审查处理 19b839f，未推送）——on_chunk 实时退化检测
  （连续相同 delta N=5 / 1K token 窗口 3-gram 重复率 >60%）、退化中断
  不重试、会话级连续计数达 3 转 `run_invalidated{status: degeneration}`
  （schema 先行）、`REQUEST_MAX_TOKENS`/`ModelConfig::max_tokens`
  160K→32K、终端 8K + read_file 补读指针三面、点读指针改向落盘文件、
  桥截断保留工具结果自身指针、失败轮次 stagnation 审计评估。S2 测试：
  orz-loop 510 通过 / orz-tools 2763 通过（沙箱外）/ fmt 干净 / clippy
  基线一致 / 仓库门禁 valid。**全面审查处理**：P1=点读终端判定改按发放
  时订单动作名（`ActionResult.action`）——响应信封 `{"output": string}`
  被 read_file/grep/run_tests 等共用，原按信封判定会给非终端 receipt
  死指针（违反「指针路径必须真实可读」），修复 + 非终端 text-output
  回归测试；P3=指针块计入 8K 截断预算、160K 陈旧注释清理（live 探针改
  32K）、.gsa 符号链接可读性专属测试、登记同步；解释登记=「同一 run
  连续 3 次」字面不可达，实现为会话级连续计数（成功请求重置）。计数
  纪律：实施放行入账（28→29），S3/S4 验证闭环后 29→28。S3 重建 → S4
  复验（无退化中断、无 400、命中率 ≥90%、补读路径可用）待续。详见
  [设计 §9](OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19.md)。
- **2026-08-20 S3 重建 + S4 复验闭环 + S4 缺口修复（用户放行；orz
  提交 f2cb1e0，已推送）**：S3 Linux musl 重建（ORZ-BUILD-MOUNT-001
  契约，两轮：build-20260819.log 11m04s / build-20260819b.log 3m56s，
  三件套时间戳更新）。S4 make-doom-for-mips 单题复验（三轮运行 + 端到端
  探针）：无退化复读中断 / 无 hang（agent 全程活跃至 900s 任务预算
  耗尽）、零 400、journal 口径命中率 94.25%（124 请求，job
  2026-08-19__22-50-01）与 91.91%（73 请求，job 2026-08-19__23-25-53）
  均 ≥90%、补读路径真实可用。**S4 发现并修复缺口**：终端截断收据指向
  `.gsa/session/terminal/<order>.log`，但权限层 `access_in_scope` 按
  「.gsa 树 agent-invisible」拒绝全部 .gsa 读取（第二轮回执尝试补读被
  policy_denied）——修复=白名单会话 .gsa 卷内 `session/terminal/*.log`
  的 read_file/grep（对齐 run_tests_output.txt 先例；lexical 限目录 +
  canonical 限会话卷防符号链接外逃；run_tests 白名单补 symlink-aware
  比较），orz-host 单测 221 通过 / fmt 干净 / clippy 无新增；端到端
  探针确认模型按指针 read_file 三次成功（1–1000 行、offset/limit 分页
  1001–1200 行、操作台订单复核 line-1190–1200，1200 行完整取回）。
  计数：S3/S4 验证闭环 **29 → 28**。登记于 ADR-0010 §14.33 第 3 项 /
  [设计 §6/§9](OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19.md) /
  CLI_PROJECT_INDEX / TODO P0-0d。
- **2026-08-20 换题复验（gpt2-codegolf）+ 流式重试节奏设计定稿（用户
  裁决：重试间隔缩短——idle 无数据判定 5s 一轮、10 次上限（总 50s
  窗口），zero-chunk 重试窗口同步 50s；先设计、不动作）**：P0-0d 闭环
  后按用户指示换题复验稳定性（gpt2-codegolf，历史 7 次失败、build/run
  型需读约 500MB 权重），三轮运行（00-22-19 / 00-24-29 / 00-41-54）：
  第一轮首请求 20s idle → 90s 内连接中断 → zero-chunk 重试 2 次后
  **32s 窗口耗尽**（瞬时连接错误）；第二/三轮首轮 request→model_output
  约 10.5/10.6 分钟（`AgentTimeoutError` 正常收尾），有效工作时间约
  3 分钟、8 请求、命中率 85.19%（样本不足非机制退化；三轮零 400、无
  退化中断、ACAF 票据全过）。**环境排查结论=容器/网络/API 均正常**
  （容器 TTFB 0.38s、宿主机流式 0.19s、带 tools 大请求 60–80s 完整流完、
  对照昨天 make-doom 首轮 8.3s；差异在 DeepSeek 端首轮生成慢 + 一次
  瞬时连接错误，非机制退化）。**定案**：`stream_idle_warn` 20s→5s、
  `stream_idle_timeout` 90s→50s（=5s×10 轮）、`request_retry_window`
  32s→50s（zero-chunk 重试窗口同步；非流式 create 退避窗口同步放宽）、
  `request_max_retries` 10 次不变；idle 只看完全无数据（慢速 reasoning
  流不误杀）、重试仍指数退避、退化中断不重试纪律不变、无新增旋钮、
  retry 参数参与请求头指纹（部署后首次请求一次性指纹变化，既有纪律）。
  实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（≥90%、无 400、无
  退化中断、首轮不再 10 分钟级长等）；设计轮不动计数（28）。登记于
  ADR-0010 §14.34（v1.34）/ [设计](STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。

- **2026-08-20 官方 harness 对照 + 输出预算恢复与空流止损设计定稿（用户
  方向：评估 256K+max、空流处理官方化、退化检测器大升级；先设计、不动作）**
  ：官方 llm-deepseek/llm-retry 源码对照（默认 high/256K/5min idle；
  EMPTY_RESPONSE 空即错即退 5 次、退避 500ms→10s+10% jitter、重试在
  step 边界；无生成期退化防护）。今晚 7 次运行 + pcap + 账单定论根因链
  =max 档思考 + 32K 截断→完成型空响应→D-6 原样重试放大（10 分钟级）；
  检测器只喂 content delta、空流全程沉默。**定案**：①`REQUEST_MAX_TOKENS`
  32K→256K（回落档 128K、S4 实测校准）；②D-6 链官方化收窄——完成型空响应
  快速有界重试 ≤2 次（500ms→10s+10% jitter）→ thinking 禁用降级；
  reasoning 族异常（stall/复读）不原样、直接降级；③退化检测器大升级为
  输出健康哨兵——观测面扩到 content+reasoning+tool arguments、新增
  reasoning 复读（灵敏层，循环特征即触发）+ reasoning-stall（预算兜底：
  首 chunk 起 600s 无 content/tool_calls、或 reasoning 估算 ≥64K tokens，
  OR 触发——**空转预算与 max_tokens 解耦**；系数 2 字符/token）、重试分类=
  有可见输出不重试（content 族，ADR-0007）/无可见输出降级、
  `DEGENERATION_LIMIT=3` 三族共享。**同日修订**：idle 死线 50s→30s（取代
  STREAM-RETRY-RHYTHM 未实施的 50s 定值；warn 5s/retry window 50s 不变）；
  160K 复读归因=架构工具设计（无再读闭环）已由 P0-0d 修正、作为恢复 256K
  安全依据；重试层结论=保留 transport 内链 + 吸收官方空流节奏，不迁移 step
  边界。**二轮修订（实测校准）**：合法难题首轮 17,757 reasoning/184s 正常
  产出（RUN-CLI-6a85f668）——120s/16K 会误杀合法轮。**三轮修订（用户裁决：
  兜底兼容 max 思考、灵敏层负责快速）**：成本账（实测 ¥4.592/M output，
  32K≈¥0.147/64K≈¥0.294/256K≈¥1.176；现状空流链 2×32K≈¥0.30）——64K
  兜底单次最坏 ≈ 现状整条链且消除链式等待，兜底定 **600s/64K**（S4 校准
  300–900s/32–128K，合法锚点 3.6 倍思考空间）；兜底管单次上限、D-6 管重试
  次数，两本账解耦。设计轮不动计数（28）。登记于
  ADR-0010 §14.35（v1.35）/
  [设计](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。
- **2026-08-20 S1 代码实施完成（用户放行实施；暂不重建/测试，S2-S4
  待续）**：`REQUEST_MAX_TOKENS`/`ModelConfig::max_tokens` 32K → **256K**
  （回落档 128K 注释保留）；`stream_idle_timeout` 50s → **30s**（RetryPolicy
  默认值 + 注释同步）；D-6 流式空流链官方化收窄（`generate_stream`：
  完成型空响应快速有界重试 ≤2 次、退避 500ms→10s+10% jitter → thinking
  禁用降级 → 仍空显式失败；reasoning 族哨兵中断不原样、直接跳降级；content
  族维持不重试透传）；退化检测器升级为输出健康哨兵（观测面扩到 content +
  reasoning + tool arguments；`content_repetition` / `reasoning_repetition`
  （灵敏层，仅 content/tool_calls 全空时启用）/ `reasoning_stall`
  （600s/64K 预算兜底，OR 触发，空转预算与 max_tokens 解耦）三族信号；
  `REASONING_CHARS_PER_TOKEN=2` 估算 + usage 到达时复核留痕；detail 前缀
  `degeneration_detected:content_repetition|reasoning_repetition|reasoning_stall`；
  `is_degeneration_detail` 收窄为 content 族 + `is_reasoning_guard_detail`
  新增；`DEGENERATION_LIMIT=3` 三族共享不变；run 层失败轮次审计补触发族
  标签留痕）。既有断言同步（256K 请求头 / idle 30s 默认值 / 检测器 feed
  签名）；live 探针 probe_thinking_max 与指纹/注释同步。计数：实施放行
  入账 1 项（**28 → 29**），S3/S4 验证闭环后 29 → 28。登记于
  ADR-0010 §14.35 第 2 项 / [设计](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。
- **2026-08-20 S2 测试实施完成（用户指示进行 S2）**：新增 15 项测试
  （orz-loop lib，527 通过 / 0 失败 / 3 ignored）——退化检测器单测 10 项
  （reasoning 复读灵敏层、content/tool 可见输出停用 reasoning 族、stall
  双信号 600s/64K OR 语义、无首 chunk 不触发（idle 互补）、估算校准、
  空转预算与 max_tokens 解耦、空流重试参数/退避形状）+ 空流链 e2e 5 项
  （完成型空响应快速重试 2 次→降级、链尾显式失败、reasoning 复读→降级、
  reasoning-stall→降级、重试中途哨兵→跳过剩余原样重试）；S1 已同步断言
  继续覆盖 256K 请求头 / idle 30s / content 退化不重试。回归：fmt 干净、
  clippy 无新增告警（transport.rs 零告警，lib 21 与基线一致）、
  `cargo check --workspace` 通过。计数不变（仍 29），S3/S4 验证闭环后
  29 → 28。登记于 ADR-0010 §14.35 第 3 项 /
  [设计 §4.2](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。
- **2026-08-20 S3 重建 + S4 复验 + 对账（用户指示重建与复验＋对账；
  S1-S4 全部闭合）**：S3 Linux musl 重建成功（ORZ-BUILD-MOUNT-001 契约，
  三件套时间戳更新，orz 104.4MB，日志 build-20260820.log）。S4
  gpt2-codegolf 单题复验（job `2026-08-20__18-46-52`，RUN-CLI-6a86db35，
  wallclock 1740s 跑满、reward 0.0、无异常）：**完成型空流 0**（32K 截断
  空流链根因消除，首请求约 66s 出首输出，对照 32K 时代 10.5 分钟级）；
  零 400、零 idle 死线、零 timeout；**journal 口径命中率 95.28%**（85
  请求，hit 3,110,656 / miss 153,990，≥90% 达成）；`reasoning_repetition`
  灵敏层触发 1 次（10:58:24）并直接降级收尾（10:58:34 降级轮正常产出），
  reasoning-stall 兜底（600s/64K）零触发零误杀（合法 reasoning 峰值
  20,234 tokens；对照合法锚点 17.7K/184s）；用量（journal）：output
  191,623 tokens（含 reasoning 170,906）、prompt 3,264,646（hit
  3,110,656 / miss 153,990）、单请求最大 completion 20,776；按 ¥4.592/M
  估算输出成本 ≈ ¥0.88；控制台 CSV 待刷新补精确对账。校准结论：600s/64K/
  30s 初值维持不调。**计数：S3/S4 验证闭环 29 → 28**。登记于
  ADR-0010 §14.35 第 4 项 /
  [设计 §4.3](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。
- **2026-08-20 默认档 high + 三级降级梯修订设计定稿（用户裁决：方案 B +
  中间档；先设计、不动作）**：S4 实测（256K+max：空流 0、命中率 95.28%、
  复读灵敏层拦截 1/85 并降级收尾、stall 兜底零误杀）表明机制已稳、max
  不再是必要工作点；思考禁用本身质量影响大（「快答模式」）。**定案**：
  ①默认 `reasoning_effort` max → **high**（官方默认档，官方工作点即
  256K+high），`EnabledMax` 保留显式可选档（仍受哨兵保护）；②降级梯插入
  **low** 中间档——**high → low → disabled → 失败**（空响应快速重试与
  reasoning 族哨兵跳转共用；「middle」= DeepSeek `reasoning_effort=low`）；
  ③兜底/重试节奏不变（stall 600s/64K、idle 30s、`EMPTY_RESPONSE_MAX_
  RETRIES=2`、退避 500ms→10s+10% jitter、`REASONING_CHARS_PER_TOKEN=2`）；
  ④指纹含 thinking → 部署后首次请求一次性变化。代价=失败路径多一轮完整
  思考（每级受 64K/600s 兜底保护），病态率低（S4 1/85）且复读数 K 内被
  抓，可接受。实施路由 S1 代码（`ThinkingMode` 增 `EnabledLow`、默认
  `EnabledHigh`、三级梯接线）→ S2 测试（high/low 请求头、三级梯路径、
  回归）→ S3 重建 → S4 复验（难题单题 + high vs max 成本/产出对照）。
  设计轮不动计数（28）；实施放行 28 → 29，验证闭环 29 → 28。登记于
  ADR-0010 §14.35 第 5 项 /
  [设计 §3.6/§4.4](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。
  **2026-08-20 S1/S2 实施登记**（用户指示进行 S1 与 S2；orz b72a0a4 已
  推送）：`ThinkingMode` 增 `EnabledLow`、默认档 max → high（max 保留
  显式可选档）；`apply_thinking` 双旋钮统一（修复 high 配置降级到 low 时
  effort 仍为 high 的隐患）；`generate_stream` 降级梯 high → low →
  disabled → 失败（空响应每档 ≤2 次快速重试、换档重置计数与退避；
  reasoning 族哨兵逐级下降；max 显式档保留 S4 直跳 disabled 基线）；
  请求头指纹含 high/low；S2 测试=默认 high/max/low 请求头断言 + 三级梯
  e2e 3 项（空响应全链失败、哨兵 high→low、low 级哨兵→disabled）+ 既有
  max 基线核对；orz-loop lib 531 通过 / 0 失败 / 3 ignored、fmt 干净、
  clippy 基线一致、`cargo check --workspace` 通过。计数：S1 放行入账
  28 → 29，S2 不变（仍 29），S3/S4 闭环后 29 → 28。登记于
  ADR-0010 §14.35 第 6 项 /
  [设计 §4.5](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。
  **2026-08-20 S1 全面审查处理登记**（用户指示处理审查全部问题；orz
  1651f59，已提交、未推送）：审查结论=未发现功能缺陷，设计合理、实现
  合理、设计与实现符合；处理 5 项观察级建议——O1 非流式 `generate` 链
  不引入 low 档登记为有意不对称（已知边界，三级梯作用域=generate_stream，
  generate 仅服务 preflight/gate 快轮）；O2 `build_request` 与
  `apply_thinking` 双份映射补同步注释；O3 `empty_response_backoff` 60s
  max_elapsed_time 登记为参数表外兜底（每档 ≤2 次重试不可达）；O4 新增
  `config_fingerprint_reflects_thinking_tier`（默认档指纹 == 显式
  EnabledHigh、与 low/max/disabled 互异；设计 §3.6「指纹含 thinking 档」
  补断言）——orz-loop lib 532 通过 / 0 失败 / 3 ignored（+1 项）、fmt
  干净、clippy 基线一致（lib 21 均既有位置）；O5 e2e 请求体子串匹配登记
  为已接受边界（mock 可控、无实际风险）。计数：审查处理不改变未闭合计数
  （仍 29），S3/S4 闭环后 29 → 28。登记于 ADR-0010 §14.35 第 7 项 /
  [设计 §4.6](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。
  **2026-08-20 S3 重建 + S4 复验登记（用户指示推送后重建、换题复验；
  S3/S4 验证闭环 29 → 28）**：S3 Linux musl 重建成功（三件套时间戳
  12:08，orz 104.4MB）。S4 **换题 make-doom-for-mips**（P0-0d 退化
  防护起源题）单题复验（job `2026-08-20__20-08-50`，RUN-CLI-6a86ee6c，
  wallclock 1740s 跑满、reward 0.0、零异常）——**完成型空流 0**、
  零 HTTP 400、零 idle 死线、**journal 命中率 92.18%**（194 请求，
  hit 8,122,240 / miss 689,249，≥90% 达成）、哨兵/stall 全零触发零误杀
  （600s/64K/30s 初值维持不调）、首输出 5.5s（对照 max 基线约 66s）、
  output 169,182 tokens（reasoning 77%）输出成本估算 ¥0.78（对照 max
  ¥0.88）。high vs max 跨题参照：high 档 30 分钟内 194 请求/333 工具轮
  （max 85 请求）、每轮更快、output 成本更低；命中率 92.18% 达标但低于
  max 95.28%，属跨题差异非档位回归；input 侧不可直接对照。**计数：
  S3/S4 验证闭环 29 → 28**。登记于 ADR-0010 §14.35 第 8 项 /
  [设计 §4.7](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
  / CLI_PROJECT_INDEX / TODO P0-0d。

> - **2026-08-21 第一轮 5 题冒烟扫描（sweep-r1-g1）+ zero-chunk 重试窗口
>   50s → 180s 修订（用户裁决：简单拉长窗口；节点超时降级无实际作用）**：
>   冻结版 cf0be20 跑分环境预检全绿（Docker/镜像/数据集/orz 三件套哈希/
>   API 探活），r1-g1 五题 2 通过（schemelike-metacircular-eval、
>   build-pov-ray）3 未过（dna-assembly 哨兵链→Disabled 档解码错误非零退出、
>   llm-inference-batching-scheduler 网络零 chunk 重试窗口耗尽非零退出、
>   feal-linear-cryptanalysis 1800s 超时）；零 HTTP 400、零 run_invalidated、
>   12 次哨兵（8× reasoning_stall + 4× reasoning_repetition，约 ¥2.5）。
>   **归因**：llm-batching 约 1 分钟级 DeepSeek 节点抖动，transport 内链
>   50s 窗口内 5 次重试即耗尽、run 非零退出（30 分钟 trial 被 1 分钟抖动
>   杀死）；dna-assembly 哨兵链=降级梯按请求重置（每次新请求回 high）+ 
>   Disabled 档重试遇 `error decoding response body`（已见 chunk 截断按
>   ADR-0007 不重试）→ 直接杀 run。**修订**：`request_retry_window`
>   50s → **180s**（model.rs 默认 + 测试断言；`request_max_retries` 10
>   不变，双上限先到者止、实测 10 次 ≈ 约 2 分钟重试跨度）；终端解码
>   错误重试兜底设计待用户确认边界（与本次修订批次合并 S3 重建一次到位）。
>   S1/S2 已闭合、S3/S4 待续。登记于 ADR-0010 §14.36（v1.36）/
>   [设计修订](STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md) /
>   CLI_PROJECT_INDEX / TODO P0-0d 后续。

> - **2026-08-21 哨兵 fail-fast 化 + 流式中段解码重试设计定稿登记（用户
>   裁决：fail-fast 方向有道理、确认非架构原因后实施并显式标明；解码
>   兜底按「无完整 tool_calls 即重试（有界）」；先设计、不动作）**：
>   sweep r1-g1 12 次哨兵（8× stall + 4× rep，约 ¥2.5）归因=跨请求烧
>   stall 是恢复机制副产品（降级梯每请求回 high、成功清零计数器）；
>   harness 源码对照=step 边界有界重试（normal 默认 2 次）、无降级/无
>   生成期哨兵、空即 step 失败。**主案**=会话级 thinking 档位（哨兵后
>   不再回 high）+ 哨兵计数单调（成功不清零、达 3 显式 run_invalidated）
>   + disabled 档哨兵即终止；严格案（哨兵即 step 失败）留对照。S0 证据
>   门=下一批扫描采集哨兵触发上下文、与机械结构块无稳定相关才放行 S1。
>   **解码兜底**=重试判定从「零 chunk」改「无完整 tool_calls」——已见
>   chunk 的 Transport/解码截断（含 dna 的 `error decoding response
>   body`）有界重试 1 次后显式失败；错误路径工具从未执行、重发幂等；
>   重试计数入事件面。设计轮不动计数（27）。登记于 ADR-0010 §14.37
>   （v1.37）/ ADR-0007 修订注记 / [fail-fast 设计](STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md)
>   / [解码重试设计](MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md) /
>   CLI_PROJECT_INDEX / TODO P0-0d 后续。

> - **2026-08-21 S0 证据门通过 + S1/S2 实施闭合 + 全面审查处理登记
>   （用户放行实施、处理审查全部问题）**：S0 通过（r1-g1+g2 两批 10 题
>   7 次触发对、0 例机械块强相关、分布不均）→ 放行 S1。实施=会话级
>   thinking 档位 + 计数单调（成功不清零）+ disabled 档即终止 + detail
>   显式化（族 + consecutive + round）；解码兜底=判定改「无完整
>   tool_calls」+ 中段有界 1 次 + `saw_chunk` 预算区分；**事件面重试
>   计数一并实施**=v0.2 `transport_retry`（recovered/exhausted，
>   schema/fixtures/conformance/TUI 同步，run-event 54 项）。**正式路径
>   per-run 隔离**= `ModelGateway::for_new_run()` 每 run 换新实例
>   （长驻进程 ACP server 跨 run 零泄漏、并发零干扰；run_turn_inner
>   开头调用，主/子代理共享 run 实例）。S4 口径修正=单 run 哨兵预算
>   有界 ≤3 次触发 × 单次预算。orz-loop 544 / orz-tui 178 / Python
>   conformance 230 通过；S3 重建 → S4 复验待续（与窗口 180s 批次合并
>   一次到位）。登记于 ADR-0010 §14.37 第 3 项 / 两份设计文档状态
>   implemented / ADR-0007 修订注记 / CLI_PROJECT_INDEX / TODO P0-0d
>   后续 4/5。

> - **2026-08-25 后续 3/4/5 合并 S3 重建核证闭合（TODO 同步更新；计数
>   不变仍 29，S4 复验待续）**：180s 重试窗口（orz a96faab）与
>   fail-fast + 中段解码重试（orz a0d85f8）已随 0g S3（2026-08-24，
>   orz 033fd26/4ca60c2）、0h S3（2026-08-25 02:46，orz f4f1b81）与
>   0.1.0 发布重建（2026-08-25，orz 8bcf18c）进入 Linux musl 三件套；
>   merge-base 祖先核证通过（a96faab/a0d85f8 均在现役源码链），现役
>   orz 104,796,704 B。S4 复验（无 400、命中率 ≥90%、断连窗口骑过
>   节点抖动、哨兵预算有界、dna 类解码错误不再杀 run）待续。登记于
>   TODO P0-0d 后续 3/4/5 / CLI_PROJECT_INDEX。

> - **2026-08-21 复读检测粒度修订设计定稿（用户裁决：同意滚动哈希任意
>   偏移重复检测；先登记、不直接动作）**：dna-assembly 复跑
>   （RUN-CLI-6a885faa）误杀实证——seq 95 完整输出 6439 字符连贯正常
>   DNA 分析（无 ≥20 字符连续重复、含 ttttt/aaaaa/ggggg/N N N N N/
>   GGTCTC 低熵特征），现有路径①「连续 5 个相同 content delta」在
>   小 chunk 粒度 + 低熵文本下天然命中。**定案**：路径①替换为滑动
>   窗口滚动哈希任意偏移检测——L+W=144 字符缓冲（比较区 96 字符）+
>   48 字符 L-gram 哈希集，新 L-gram 哈希在比较区内已见（偏移 ≥48）即
>   触发（字符级比对防碰撞）；窗口内任意周期可命中（p ≤ 96，修复固定
>   偏移相位对齐缺陷）、同字符连串 ≥96 触发、
>   DNA 正常序列免疫、O(1)/字符；3-gram 路径②（≥1K token >60%）保留
>   兜底；content/reasoning 两族共用；stall 兜底（600s/64K）与
>   fail-fast 纪律（会话级档位/计数单调/disabled 即终止）不变。设计轮
>   不动计数（28）。登记于 ADR-0010 §14.35 第 13 项 /
>   [设计 §3.3/§4.8](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
>   / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。

> - **2026-08-21 复读检测粒度滚动哈希 S1 代码实施登记（用户放行实施；
>   orz 工作树未提交）**：`DegenerationDetector` 路径①（连续相同 delta
>   N=5）替换为滑动窗口滚动哈希任意偏移（`RollingRepetitionWindow`：
>   144 字符缓冲 + 48 字符 L-gram 哈希集 + 字符级比对防碰撞；
>   `REPETITION_MIN_RUN_CHARS=48` / `REPETITION_WINDOW_CHARS=96`）；
>   `feed_repetition` 路径②（≥1K token 3-gram >60%）保留兜底；
>   content/reasoning 两族共用、detail 前缀不变。既有断言同步（5×delta
>   不再触发 → 96 字符重复 span；3-gram 测试内容重构）。orz-loop
>   **550 通过 / 0 失败 / 3 ignored**、fmt 干净、clippy 无新增告警、
>   `cargo check --workspace` 通过。计数：实施放行入账 **28 → 29**。
>   登记于 ADR-0010 §14.35 第 14 项 /
>   [设计 §4.9](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
>   / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。

> - **2026-08-21 复读检测粒度滚动哈希 S2 测试实施登记（用户指示进行
>   S2）**：新增 8 项测试（orz-loop **558 通过 / 0 失败 / 3 ignored**）
>   ——短低熵块不触发、DNA 低熵样本 6439 字符不触发、poly-A 95/96 精确
>   阈值、周期 10 短语循环触发（content + reasoning 对齐缺陷回归）、
>   单一大 chunk 不触发、`spans_equal` 字符级比对直接验证（哈希碰撞
>   构造不可行登记为已接受边界）、近重复不触发 + 精确复读触发。回归：
>   fmt 干净、clippy 无新增告警、`cargo check --workspace` 通过。计数
>   不变（仍 29）。登记于 ADR-0010 §14.35 第 15 项 /
>   [设计 §4.10](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
>   / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。

> - **2026-08-22 S3 重建 + S4 复验 + 停滞守卫退役 + 缺口 A + 灵敏层
>   再校准设计（用户裁决：生成期检测覆盖实际退化面，停滞守卫一并全部
>   退役）**：S3 Linux musl 重建（ORZ-BUILD-MOUNT-001，两轮含缺口 A
>   留痕轮）。S4 dna-assembly 复验（job `dna-assembly-s4`，15m6s、
>   118 请求）——命中率 98.2%、零真实 400、content 层零触发；发现
>   缺口 A（reasoning 层触发 2 次无留痕无法定性）与缺口 B（会话级
>   停滞守卫 `STAGNATION-NGRAM-REPEAT` 普遍误杀：跨历史 run 12–341
>   全部 restart_requested）。**停滞守卫退役**（主/子代理终止判定、
>   失败轮次审计、`stagnation.rs`、`RuntimeStagnationGuard` 事件 v0.2
>   面、TUI、Python reference/verifier/schema/doctor 清单；v0.1 冻结
>   面保留；tokenize 迁移 transport）——orz-loop 557 通过、verifier
>   230 通过、assurance 1588 通过、doctor 仅剩 orz dirty。**缺口 A
>   （审计留痕）**：触发时 WARN 输出重复 span+偏移+窗口尾部；DNA 重跑
>   （job `dna-assembly-s4b`，26m26s、136 请求、命中率 98.64%、
>   `run_finished completed`）实证 reasoning 2 次触发均为正常任务内容
>   重复引用（DNA 序列等式/技术短语 48 字符 span）——**误杀坐实**；
>   停滞守卫退役后 run 不再被误杀中断。**灵敏层再校准设计定稿（用户
>   裁决：L=200 + 流内累计 3 次命中才中断+降级；content/reasoning
>   统一；先落设计、未实施）**——`REPETITION_MIN_RUN_CHARS` 48→200、
>   `REPETITION_WINDOW_CHARS` 96→400（缓冲 144→600）、命中门槛 3
>   （间隔不重置、1–2 次仅留痕、计数随流结束丢弃；会话级 consecutive
>   与 `DEGENERATION_LIMIT` 不变）；3-gram/stall 兜底与 fail-fast 纪律
>   不变；判定语义=流内 ≥3 次完全相同的 200 字符 span。设计轮不动
>   计数（仍 29；S3/S4 闭环因缺口 A 推迟，缺口 A 定性完成、待灵敏层
>   再校准实施后闭环）。登记于 ADR-0010 §14.35 第 16-17 项 /
>   [设计 §3.3/§4.11](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
>   / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。

> - **2026-08-22 灵敏层再校准 S1 代码 + S2 测试实施登记（用户指示进行
>   S1 与 S2；orz 工作树未提交；计数不变仍 29）**：`REPETITION_MIN_RUN_
>   CHARS` 48→200、`REPETITION_WINDOW_CHARS` 96→400（缓冲 600）、新增
>   `REPETITION_HIT_LIMIT=3`——滚动窗口命中后继续喂入（不早停）、
>   流内累计命中 ≥3 次才中断+降级（间隔不重置）；1–2 次命中仅审计留痕
>   （`audit_hits` → 流循环逐条 WARN，缺口 A 语义）；content/reasoning
>   统一、3-gram/stall 兜底不变；单族状态聚合 `RepetitionFamilyState`
>   保持自由函数参数在 clippy 阈值内。S2 测试：poly-A 399/400/401 不
>   触发（0/1/2 次命中仅审计）、402 触发；周期 10 `repeat(41)` 3 次命中
>   触发；近重复不计数、精确复读第 3 次触发；新增命中门槛与间隔不重置
>   测试；3-gram 兜底用例加唯一标记杜绝 200 字符 span 复现；e2e 五处
>   改 402 同字符 + 新增子阈值不中断 e2e。orz-loop **560 通过 / 0 失败
>   / 3 ignored**、fmt 干净、clippy 无新增（transport.rs 仅 2 条既有
>   doc 告警）、workspace check 通过。同日 S1 全面审查处理（4 项全部
>   处理）：信号表/头部同步再校准参数；`feed_chars_capped` 命中上限
>   喂入；审计 WARN 与触发判定同 chunk 聚合（消除「audit only」误导
>   文案）；3-gram/stall 触发清空 `trigger_context`。登记于
>   ADR-0010 §14.35 第 17 项 /
>   [设计 §3.3/§4.11](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
>   / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。待续：S3 重建 + S4 复验
>   （DNA reasoning 正常引用不误杀、真循环仍触发）闭环后 29 → 28。

> - **2026-08-23 复读判定二级再校准设计定稿（用户裁决：L=400 + 二级
>   标点块内部重复确认；G4 冒烟实证代码引用型误杀仍存在；先落设计；
>   2026-08-23 **S1 实施**（transport.rs 常量 + 二级确认逻辑），S2 待
>   放行；设计轮不动计数，仍 29）**：第四轮冒烟（sweep-r1-g4-official
>   官方方式 k=1、无 max_wallclock）5/5 reward 1.0、零真实 400、命中率
>   93.66–98.48%；sam-cell-seg reasoning 2 次触发（203/204 字符 span）
>   均为模型推理中完整引用代码块（引用-再确认循环习惯，非设计泄露）。
>   **定案**：`REPETITION_MIN_RUN_CHARS` 200→400、`REPETITION_WINDOW_
>   CHARS` 400→800（缓冲 1200）、命中门槛 3 不变（同字符连串触发线
>   802）；新增二级「标点块内部重复确认」——按标点+空白切块，大块内部
>   标点块重复覆盖占比 ≥0.50（初值）才计命中（每对都过）、无切分点
>   直接判真；3-gram/stall 兜底不变。判定语义=流内 ≥3 次「400 字符
>   完全相同且内部由重复标点块构成（或无切分点）」的 span 才触发。
>   漏判边界=<400 字符短循环、结构性重复内容、无标点长引用（概率
>   可控）。验证方案=G4 样本离线回放 + 构造样本 + S3 重建 + S4 冒烟
>   复验。登记于 ADR-0010 §14.35 第 18 项 / 设计 §3.3/§3.5/§4.11 /
>   TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。
> - **2026-08-23 S1 全面审查处理（审查发现 5 项全部处理；实施不改变
>   计数，仍 29）**：① §3.3 信号表 L=200/W=400 旧参数同步为
>   L=400/W=800 + 二级并补脚注；② 拒绝候选审计按 delta 聚合为「一条
>   摘要 + 命中计数」（用户裁决——实现探明滑动窗口每对语义下同一直复
>   内容的移位 span 文本互异、按精确 span/重复块签名均无法稳定折叠，
>   改每 delta 一条代表 span + 块统计 + 候选对数，同 delta 多重复内容
>   合并为一条已登记边界）；③ 触发 chunk 内已取走的审计条目不再被 trip
>   分支丢弃（同样逐条 WARN，文案不复用 not-tripping）；④ 切分符集合
>   常用子集边界登记（全角变体与生僻中文标点未含）；⑤ 补阈值边界
>   （49.6%/50.0%/50.8%）/每对确认累计/G4 短引用（203 字符）形态等价
>   回放三项测试并适配聚合语义。验证：orz-loop lib 565 通过 / 0 失败 /
>   3 ignored、fmt 干净、clippy 无新增。详设计 §3.3 修订 / §4.11；
>   登记于 ADR-0010 §14.35 第 18 项 / TODO P0-0d 后续 6 /
>   CLI_PROJECT_INDEX。
> - **2026-08-23 S2 测试实施登记（用户放行 S2；正式 S2 轮=离线回放记录 +
>   结果归档；实施不改变计数，仍 29）**：新增字节级真实回放测试
>   `repetition_second_stage_g4_real_span_replay_stays_silent`——源证据=
>   `sweep-r1-g4-official\sam-cell-seg__CT5JrD3\agent\orz.txt` WARN 记录
>   （2026-08-21T21:35:29 / 21:36:00）：旧 L=200 检测器两次触发的真实
>   匹配 span 各 200 字符（检测器打印口径；设计早前 203/204 为完整重复
>   引用区域口径），以 reasoning 族按「引用-再确认循环」形态原样引用 3
>   遍（间隔互异再确认文本）回放——首级 L=400 均不命中（无候选、无审计、
>   无触发）；构造样本正/负对照由 S1 审查处理已覆盖。验证：orz-loop lib
>   566 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增。详设计 §4.11；
>   登记于 ADR-0010 §14.35 第 18 项 / TODO P0-0d 后续 6 /
>   CLI_PROJECT_INDEX。
> - **2026-08-23 S3 重建登记（用户放行；Linux musl，ORZ-BUILD-MOUNT-001
>   契约，`build_orz_aliyun.sh`；计数不变仍 29）**：`rust:1.97-slim`
>   容器增量构建（挂载 `D:\CLI:/orz`、工作目录 `/orz/orz`、-j 1）
>   **BUILD_EXIT=0**；三件套时间戳 2026-08-23 02:01（orz 104,521,992 B /
>   orz-signer 1,388,592 B / orz-acaf-provision 1,206,568 B，SHA256 见
>   设计 §4.11）；最小可执行冒烟通过（orz 无 TTY 报 TUI io error 属预期、
>   provision 打印 usage、signer 报 manifest 缺失——headless 真机面由
>   S4 任务容器验证）；对应源码=orz 6178050 + 父仓库 dfe39a1。详设计
>   §4.11；登记于 ADR-0010 §14.35 第 18 项 / TODO P0-0d 后续 6 /
>   CLI_PROJECT_INDEX。
> - **2026-08-23 S4 冒烟复验登记（用户放行；G4/G5 对照，sweep-s4-g4 /
>   sweep-s4-g5，k=1、官方方式、n-concurrent=1；**S3/S4 验证闭环 29 →
>   28**）**：G4 4/5 reward 1.0——sam-cell-seg **零复读触发**（对照旧
>   L=200 二进制同题 2 次触发+2 次降级，误杀消除且 reward 仍 1.0）、
>   portfolio-optimization 零触发（旧轮 3-gram 0.60 边界 1 次，本轮无）；
>   G5 2/5 reward 1.0（path-tracing-reverse AgentTimeoutError=官方超时，
>   机制无异常）；全 10 试次零真实 400；有 journal 9 试次命中率
>   96.30%–98.34% 全 ≥90%；构造真循环触发由 S2 测试套件覆盖（566 全绿）；
>   观察项=video-processing 3-gram 兜底触发 1 次（0.60 边界、降级一次、
>   任务继续，与旧轮 portfolio-optimization 同界，登记不改范围）。详设计
>   §4.11；登记于 ADR-0010 §14.35 第 18 项 / TODO P0-0d 后续 6 /
>   CLI_PROJECT_INDEX。
> - **2026-08-23 AGENT DELIVERY FLOW 设计定稿（用户三轮讨论裁决；先落
>   设计、未实施；**最优先——2026-08-23 用户指示，本轮提案 1+2**）**：来源=S4 失败归因暴露——计划步自动推进（订单成功即
>   step done，mteb「计划完成但计算未跑」）、引用校验硬阻断无修正、订单
>   反馈缺 diff/delta。定案：步骤重定义=执行顺序标记（目标交交付门仲裁；
>   动作幻觉由「交付门拒绝→有界修正→失败」收敛）；模板末步固定「递交/
>   完成」且不随普通订单自动推进；递交状态=黑板 plan 机械渲染
>   （workspace_delta 过滤 .gsa/临时文件、上限 20+计数、无模型声明、无
>   复读风险）；订单反馈增强（编辑类回显 diff、终端类挂 delta、actions
>   板只加 changed: N files）；引用失败→有界修正机会一次（reason_codes+
>   markers、同失败 2 次恢复硬阻断、journal 记 attempt）；终端内容登记
>   引用暂缓。路由 S1-S4；设计轮不动计数（28）。登记于 ADR-0010 §14.35
>   第 19 项 / [设计](AGENT_DELIVERY_FLOW_DESIGN_2026-08-23.md) / TODO
>   P0-0d 后续 7 / CLI_PROJECT_INDEX。
> - **2026-08-23 AGENT DELIVERY FLOW S1 实施 + S2 测试闭合（用户放行；
>   实施入账 28 → 29；S3-S4 待续）**：计划步语义重定义（框架块 + plan_write
>   校验强制末步 id ∈ {deliver, submit}）；末步不随普通订单自动推进 +
>   console_step_done 末步拒绝；新增无参 `submit` 工具两阶段（首次渲染
>   [delivery] 机械交付状态进黑板 plan、再次确认置 done）；订单反馈增强
>   （workspace delta 单源化 + size、search_replace diff、终端挂 delta、
>   actions 板 changed: N files）；引用校验有界修正一次（retry/attempt/
>   correction_allowed + 二次硬阻断）。Schema/verifier 同步；S2 测试闭合
>   （orz-loop 574 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增）。
>   登记于 ADR-0010 §14.35 第 21 项 / [设计 §6](AGENT_DELIVERY_FLOW_DESIGN_2026-08-23.md)
>   / TODO P0-0d 后续 7 / CLI_PROJECT_INDEX。
> - **2026-08-23 AGENT DELIVERY FLOW S1 全面审查处理（用户指示处理审查
>   全部问题；计数不变仍 29，属 S1 内收尾）**：修复——F1 补
>   `workspace_delta_walk` 排除面单测（.gsa/.git/缓存族不得进 walk）、O2
>   terminal 判定按末步 id（`planning::is_terminal_step`，旧/恢复计划保持
>   S1 前自动推进语义，新增 predicate + legacy 行为单测）、O6 截断 delta
>   短计数渲染 `changed: N+ files`；登记口径/边界——F2 引用修正=至多 1 次
>   修正总数（attempt 不按失败类型分）、O1 递交状态为信息展示非最终回答
>   硬门（S4 观察跳过递交形态）、O3 修正轮重走引用校验而 counterexample
>   门 once-only、O4 基线仅 plan_write 新 epoch 捕获（恢复/run_plan
>   fail-closed）、O5 walk 成本、O7 末步强制不设豁免。orz-loop 579 通过 /
>   0 失败 / 3 ignored、fmt 干净、clippy 无新增。登记于 ADR-0010 §14.35
>   第 22 项 / [设计 §6.4](AGENT_DELIVERY_FLOW_DESIGN_2026-08-23.md) /
>   TODO P0-0d 后续 7 / CLI_PROJECT_INDEX。
> - **2026-08-23 AGENT DELIVERY FLOW S3 重建登记（用户指示登记 S3；
>   Linux musl，ORZ-BUILD-MOUNT-001；计数不变仍 29，S4 闭环后 29 →
>   28）**：本主题 S1/S2/审查处理代码（orz 663be55 + ed8e902）已随
>   orz HEAD 05231f7 进入 NGRAM-GUARD-CALIBRATION S3 构建轮（同一
>   Linux musl 三件套，无需重复构建）——BUILD_EXIT=0，三件套时间戳
>   2026-08-23 08:50 HKT（orz 104,664,664 B / signer 1,388,592 B /
>   provision 1,206,568 B），最小可执行冒烟=三件正常加载执行；对应源码
>   orz 05231f7 + 父 35788a0。S4 复验待续（计划无空转、末步递交、引用
>   修正、订单反馈、零 400、命中率 ≥90%）。登记于 ADR-0010 §14.35
>   第 28 项 / [设计 §6.5](AGENT_DELIVERY_FLOW_DESIGN_2026-08-23.md) /
>   TODO P0-0d 后续 7 / CLI_PROJECT_INDEX。
> - **2026-08-23 AGENT DELIVERY FLOW S4 复验闭环（用户指示补登记；复用
>   NGRAM-GUARD-CALIBRATION S4 同一批实机数据，sweep-s4n-g4 / g5 /
>   g5-cfi，k=1、官方方式；计数 29 → 28，S1-S4 全部闭合）**：新二进制
>   （orz 05231f7）10 试次复验——计划无空转（8 完成试次按步执行至末步、
>   终答绑定 receipts，历史 mteb 空转形态未再现）；末步递交走机械交付
>   状态（8/8 完成试次 submit 双阶段：[delivery] 状态行渲染进黑板 plan →
>   复核 → 确认置 done；git-multibranch 终答引用交付状态行；sam-cell-seg
>   末步订单被 step_not_done 拒绝）；引用修正一次/二次硬阻断（portfolio/
>   break-filter/mteb retry attempt=1 → block attempt=2；git-multibranch/
>   sam-cell-seg retry 后通过）；订单反馈 receipt 点读链全走通（diff/delta
>   机械形态由 S2 锁定）；零真实 400（10/10）；命中率 10/10 有 journal
>   94.11%–98.55% 全 ≥90%。登记于 ADR-0010 §14.35 第 29 项 / [设计
>   §6.6](AGENT_DELIVERY_FLOW_DESIGN_2026-08-23.md) / TODO P0-0d 后续 7 /
>   CLI_PROJECT_INDEX。
> - **2026-08-23 N-GRAM GUARD CALIBRATION 设计定稿（用户裁决；先落设计、
>   未实施；**最优先——2026-08-23 用户指示，本轮提案 3**）**：来源=两次真实任务 3-gram 路径②边界误触发（旧轮
>   portfolio-optimization + 本轮 video-processing，均显示 0.60、实际
>   0.600–0.609、正常推理自引用、单发 trip、非致命）。定案：阈值
>   0.60→0.70（`>` 保留）；流内累计命中（≥3 才 trip、1–2 仅审计（ratio+
>   窗口+族）、间隔不重置、流结束丢弃）；统一口径（WARN 精度、信号表）。
>   边界：0.60–0.70 近重复循环漏判由 stall 兜底；无原始字节、验证靠单测
>   +e2e+实机。路由 S1-S4；设计轮不动计数（28）。登记于 ADR-0010 §14.35
>   第 20 项 / [设计](NGRAM_GUARD_CALIBRATION_DESIGN_2026-08-23.md) /
>   TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
> - **2026-08-23 S1 代码实施闭合（用户放行；实施入账 29 → 30，S2-S4
>   待续）**：`DEGENERATION_NGRAM_REPEAT_RATIO` 0.60→0.70（`>` 保留）；
>   新增 `NGRAM_HIT_LIMIT=3`——3-gram 路径②流内累计命中（每次 feed
>   超阈值计 1 次、≥3 才 trip、1–2 次仅审计留痕=ratio+窗口 token 数+
>   族、间隔不重置、流结束丢弃）；WARN 口径 `{:.2}`→`{:.3}`；基础设计
>   信号表/参数表同步；既有 3-gram ratio 用例适配（0.694 边界样本留给
>   S2、双份 core 0.825 过 0.70 + 2 次审计 + 3/3 trip 断言）；orz-loop
>   579 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增。登记于
>   ADR-0010 §14.35 第 23 项 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
> - **2026-08-23 S2 测试闭合（用户放行；orz 1c87681；实施入账不变仍
>   30，S3-S4 待续）**：新增 6 项——0.69x 边界不计数不触发（content
>   ratio≈0.694 + reasoning 镜像）、0.70x 边界三连触发（9 词 core
>   ratio≈0.720：审计 1/3→2/3→trip 3/3）、单超大 feed 计 1 次命中
>   （feed 粒度）、间隔不重置（单 feed 300 互异 token 压窗口至 0.70 以下、
>   计数保持 2，重灌恢复后第 3 次命中 trip 3/3）、流结束丢弃（新流首
>   命中只审计不 trip）；既有 3-gram 用例适配已于 S1 完成。orz-loop
>   585 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增。登记于
>   ADR-0010 §14.35 第 24 项 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
> - **2026-08-23 S1 全面审查处理（用户指示处理审查全部问题；计数不变仍
>   30）**：D1 边界登记修正——0.60–0.70 近重复循环漏判的「stall 兜底」
>   仅覆盖 reasoning 族（stall 要求无 content/tool_calls）；content 族
>   为已接受漏判（精确循环由路径①兜住、带变体循环成本受
>   REQUEST_MAX_TOKENS 约束、S4 观察）；I1 审计日志按路径标注命中门槛
>   （`rolling_hit_limit` / `ngram_hit_limit`）。orz-loop 585 通过 /
>   0 失败 / 3 ignored、fmt 干净、clippy 无新增。登记于 ADR-0010
>   §14.35 第 25 项 / 设计 §2.4 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
> - **2026-08-23 S3 重建登记（用户指示进行重建；Linux musl，
>   ORZ-BUILD-MOUNT-001 契约；计数不变仍 30，S4 复验闭环后 30 → 29）**：
>   容器增量构建 BUILD_EXIT=0，三件套时间戳 2026-08-23 08:50 HKT
>   （orz 104,664,664 B / signer 1,388,592 B / provision 1,206,568 B）；
>   最小可执行冒烟=三件正常加载执行（orz 无 TTY io error 属预期、
>   provision usage、signer manifest 缺失）；对应源码 orz 05231f7 +
>   父 35788a0。登记于 ADR-0010 §14.35 第 26 项 / 设计 §3 / TODO
>   P0-0d 后续 8 / CLI_PROJECT_INDEX。
> - **2026-08-23 S4 复验闭环（用户放行；G4/G5 对照，sweep-s4n-g4 /
>   sweep-s4n-g5（code-from-image 经充值后单题重跑 sweep-s4n-g5-cfi），
>   k=1、官方方式、n-concurrent=1；**计数 30 → 29，NGRAM-GUARD-
>   CALIBRATION 全部闭合**）**：新二进制（orz 05231f7 构建轮）实机运行
>   ——G4 全 5 题零 3-gram trip、零哨兵触发、零 400（历史误触发对照
>   portfolio-optimization / video-processing 均零触发；video-
>   processing AgentTimeoutError=官方超时、机制无异常）；G5 全 5 题零
>   3-gram trip、零触发、零 400（主轮 4/5 后 harbor 遇 httpx
>   ConnectError 网络抖动退出；resume 补跑 code-from-image 遇 API 余额
>   不足 NonZeroAgentExitCodeError，充值后单题重跑 reward 1.0——两次
>   异常均非哨兵、非 400、非设计问题）；命中率 10/10 有 journal、
>   94.11%–98.55% 全 ≥90%；构造流行为（0.694 不触发、0.720 三连才
>   trip、e2e 中断+降级）实测全绿。观察项：10 试次均无 3-gram 审计
>   条目（历史 0.60–0.61 边界带未再现；content 族 0.60–0.70 实机样本
>   仍空，保持已接受漏判登记）。登记于 ADR-0010 §14.35 第 27 项 /
>   设计 §3 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。

### 0e. CONTEXT-SCAFFOLDING-PULL-REDESIGN（P0；2026-08-21 设计定稿，
**S1-S4 全部闭合 2026-08-21，计数 28 → 27**）

- 入口：[设计](CONTEXT_SCAFFOLDING_PULL_REDESIGN_DESIGN_2026-08-21.md)；
  索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；ADR-0010 §14.35
  第 10 项（v1.35）。
- 来源：2026-08-20 跑分冒烟失控（content_repetition 0.87 / reasoning 64K
  空转 / 256K 概率性无限生成）→ API 实测钉死触发条件=256K × high × 重复性
  上下文；重复性上下文源自我们每轮 PUSH 的框架自有机械块（333 轮 ≈333 条
  `[TOOL_ROUND_BUDGET] REMAINING`，2026-08-07 前缀缓存修复副作用）。
- **2026-08-21 设计定稿（用户裁决：方案 A 先行、C 暂缓、状态行保留）**：
  方案 A=预算块 PUSH→PULL——退役每工具轮 REMAINING 尾随注入；`blackboard_read`
  新增 `section=session` 按需读（预算剩余 + 状态行；live 面不进归档）；机械
  硬门禁兜底；系统提示词总预算块保留。方案 B（历史替换去重）破坏前缀缓存
  否决；方案 C（256K→64K）S4 复验后裁决；方案 D 状态行保留现状。S4 重点
  观测缓存命中率（对照 98%+ 基线不减）、零 400、哨兵触发率下降、输入增长
  放缓。计数纪律：设计/实施轮不动计数（28），S3/S4 验证闭环后 28→27。
- **2026-08-21 S1 实施 + 全面审查处理闭合（用户放行；orz 7529a71）**：
  退役 REMAINING 尾随注入（零残留）；session 面（BUDGET/USED/REMAINING +
  `render_status_line`，数据源=in-run tool_rounds 含 activation 累计 /
  max_tool_rounds；越权组合参数级显式报错 exit_code 1 + error 字段）；
  工具定义 enum/描述增量；系统提示词总预算块文案改为指向按需读取。S2 测试
  随 S1 交付（协议形态 5→4、无 REMAINING 尾随、session 面渲染/越权单测 +
  工具级回达、`budget_insufficient` 拒绝文本仍含剩余）；orz-loop 536 通过 /
  fmt / clippy 基线（31）/ workspace check / 事件一致性 15 通过。全面审查=
  无功能缺陷；处理 O1-O6（口径差异/PULL 读取消耗轮/状态行双通道登记已接受
  边界，越权组合口径收紧，工具级测试补齐，工具描述去内部标签——详见设计
  §8）。S3 重建 → S4 复验（重点观测缓存命中率）待续。
- **2026-08-21 S4 复验阻断 + 缺口修复（make-doom-for-mips 单题复验启动
  即失败；orz 修复提交，见下）**：7529a71 冻结版 S4 首轮工具轮后第二轮
  请求被 DeepSeek 拒绝（`reasoning_content must be passed back`，
  invalid_request_error）。根因=PUSH→PULL 退役 REMAINING 尾随 user 消息
  后暴露 2026-08-04 遗留的「工具输出汇总 assistant 文本消息」
  （`assistant_parts` 冗余副本）成为请求末条，DeepSeek thinking 模式对
  「工具结果后紧跟的 assistant 文本轮」强制回传 reasoning_content；
  API 探针 V1–V7 实测钉死（V1 400 / V2 旧形态 200 / V3 带 rc 200 /
  V4 无汇总 200 / V5–V7 纯文本轮 200）。**修复=退役 `assistant_parts`
  汇总消息**（工具结果已以 Role::Tool 完整落库，协议形态 4→3；顺带每轮
  输入 token 节省）；orz-loop 536 通过 / 0 失败 / 3 ignored、fmt 干净、
  clippy 无新增。登记于 ADR-0010 §14.35 第 11 项 / 设计 §9 /
  CLI_PROJECT_INDEX / TODO P0-0e。S3 重建（修复版）→ S4 make-doom 重跑
  （命中率 ≥90% 对照基线不减、零 400、哨兵触发率下降、输入增长放缓）
  待续；计数纪律不变（28），S3/S4 闭环后 28→27。
- **2026-08-21 S3 重建（修复版）+ S4 复验闭环（0e 转 implemented，计数
  28→27）**：修复版 Linux musl 重建成功（orz cf0be20，三件套哈希已刷新
  FROZEN；bookworm 冒烟：provision/signer/fake headless 全过）。S4 单题
  复验（make-doom-for-mips，job `2026-08-21__01-48-26`，RUN-CLI-6a873e04，
  wallclock 1740s 跑满、reward 0.0、零异常）——**147 请求、journal 命中率
  94.45%**（hit 6,503,808 / miss 382,108）≥90% 且高于上轮同题 92.18%；
  零 HTTP 400、零 idle 死线、**哨兵/stall 全零触发**（content_repetition /
  reasoning_stall 0）；output 165,644 tokens（reasoning 77.8%）对照基线
  169,182/77% 基本持平；工具轮 226（基线 333）——REMAINING 零残留 + 汇总
  消息退役，输入增长放缓达成；首输出 8.1s（基线 5.5s，样本差异）。终态
  `run_invalidated{status:wallclock}`（正常预算耗尽）。**S1-S4 全部闭合，
  计数 28 → 27**。登记于 ADR-0010 §14.35 第 11 项 / 设计 §9 /
  CLI_PROJECT_INDEX / TODO P0-0e。
- **2026-08-21 方案 C 裁决登记（用户裁决：暂不收紧、先看当前情况）**：0e
  S4 复验闭环后实测哨兵/stall 全零触发（0/147）、命中率 94.45%，
  `REQUEST_MAX_TOKENS` 256K 无收紧必需性，维持现状；后续正式跑分
  （冒烟扫描 → 89 题 5 批）中观察哨兵触发率与输出预算，若回升/失控再
  评估 64K（方案 C 重新启用）。登记于设计 §6 / ADR-0010 §14.35 第 12 项 /
  TODO P0-0e / CLI_PROJECT_INDEX；计数不变（27）。

### 0f. FUS-READ-ANCHOR-WRITE-GUARD（P0；2026-08-21 设计定稿，
纯文档登记、未实施；设计轮不动计数 27；**S1 代码已实施 2026-08-21，
实施入账 27 → 28；S2 测试已闭合 2026-08-21；S3 重建登记 2026-08-23
（随 orz 05231f7 构建轮，计数不变仍 28）；**S4 复验闭环 2026-08-23，
计数 28 → 27（S1-S4 全部闭合）**；2026-08-21 审查收口：I/O 错误
fail-closed + clippy 修复）

- 入口：[设计](READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；ADR-0010 §14.38（v1.38）。
- 来源：2026-08-21 用户设计讨论——主 agent 经 read_file 读取快照后向助理层
  下发写订单，窗口内文件被别处改动时可能基于旧快照修改更新文档；助理层为
  纯机械无提示词执行层，只需要"是否同一份"的等值判定。
- **2026-08-21 设计定稿（用户裁决：锚=哈希值；read_file 机械返回、助理层
  零理解、简单机械核证；仅针对 orz；先设计、不动作）**：read_file 文本路径
  统一返回内容锚点 {size, mtime, sha256}（大文件信封已有 content_sha256，
  补 size/mtime；小文件同样返回锚点）；写订单携带期望锚点（可选字段）；
  orz 写门禁写前机械核证——mtime/size stat 快速预检 + sha256 权威比较，
  不匹配拒单（复用 order_stale 错误信封形态、入 console_order_rejected
  事件面）不执行任何编辑，强制重读重下。时间戳可被保留/取整（git checkout /
  cp -p / touch -r）故不作权威。边界：校验-写入 TOCTOU 极小窗口接受（可选
  后续=临时文件 + 原子替换）；expected 必填加严为可选后续；哈希只答"是否
  同一份"、补救=重读。实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验；
  设计轮不动计数（27），实施放行入账 27 → 28，验证闭环 28 → 27。
  核证期 I/O 错误语义（审查收口）：目标不存在（新建）跳过；其余 stat/read
  错误 fail-closed 拒单（同 code、消息注明原因），不放行未核证编辑。
- **2026-08-21 S1 代码实施闭合（用户指示开始 S1；orz 工作树未提交）**：
  read_file 文本路径统一返回内容锚点（`ReadAnchor {size, mtime, sha256}`；
  小文件 `FileContent.read_anchor` + prompt `[read anchor]` 尾行；大文件信封
  补 `mtime`）；`workspace.search_replace` 契约 schema 增可选
  `expected_anchor`（size/sha256 必填、mtime 可空）；`issue_pending_
  console_order` 发放前 pre_issue 门——stat 快筛 size/mtime + sha256 权威，
  不匹配拒单（phase=pre_issue / step=protocol / code=content_anchor_mismatch）
  入 console_order_rejected 事件面、清槽零编辑，指引重读重下；目标不存在
  （新建）跳过；其余 stat/read I/O 错误 fail-closed 拒单（同 code、消息
  注明失败原因）。验证：orz-tools read_file 202 / types::output 84 / orz-loop
  console 69 / orz-loop 全量 544 + console_anchor 2 通过、fmt 干净、clippy
  无新增告警（审查修复 build_read_anchor collapsible_if）；orz-tools 全量
  44 个 grep/glob 失败为本机 rg 环境性既有失败（stash 基线复现一致）。
  计数：实施入账 27 → 28（S2-S4 待续）。登记于 ADR-0010 §14.38 第 2 项 /
  设计 §8 / TODO P0-0f / CLI_PROJECT_INDEX。
- **2026-08-21 S2 测试闭合（用户指示开始 S2；orz 工作树未提交）**：新增 10
  条用例——锚点返回正确性 3（小文件 sha256/size/mtime + prompt 尾行、空文件
  size=0/空串哈希、大文件信封 mtime/content_sha256）；PDF 路径无锚点 1；
  prompt 尾行渲染与 ReadAnchor serde round-trip 2；写门禁四场景（锚点匹配
  放行（mtime null 跳快筛）、同 size 同 mtime 异内容 sha256 兜底 fixture
  （FileTimes 保留 mtime）、陈旧拒绝→重读重下成功、缺失锚点保持既有行为）；
  错误信封完整断言（upstream expected/actual、消息含 re-read 指引、trace
  末事件 protocol/content_anchor_mismatch、事件面机械盖章、零编辑）。
  验证：orz-tools read_file 207 / types::output 86 / orz-loop 全量 550
  通过（0 失败）、fmt 干净、clippy 无新增告警（30 条既有位置核对）、
  cargo check --workspace 通过。计数：仍 28（S3-S4 待续）。登记于
  ADR-0010 §14.38 第 3 项 / 设计 §9 / TODO P0-0f / CLI_PROJECT_INDEX。
- **2026-08-23 S3 重建登记（用户指示登记 S3；Linux musl，ORZ-BUILD-
  MOUNT-001；计数不变仍 28，验证闭环后 28 → 27）**：本主题 S1/S2 代码
  （orz 3ad09e4 + 554c131）已随 orz HEAD 05231f7 进入 NGRAM-GUARD-
  CALIBRATION S3 构建轮（同一 Linux musl 三件套，无需重复构建）——
  BUILD_EXIT=0，三件套时间戳 2026-08-23 08:50 HKT（orz 104,664,664 B /
  signer 1,388,592 B / provision 1,206,568 B），最小可执行冒烟=三件
  正常加载执行；对应源码 orz 05231f7 + 父 35788a0。S4 复验待续（read_file
  锚点返回、写门禁陈旧拒单→重读重下、零误拒、事件面留痕；命中率 ≥90%、
  零 400）。登记于 ADR-0010 §14.38 第 4 项 / [设计 §10](READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md)
  / TODO P0-0f / CLI_PROJECT_INDEX。
- **2026-08-23 S4 复验闭环（用户指示补登记；复用 NGRAM-GUARD-CALIBRATION
  S4 同一批实机数据，sweep-s4n-g4 / g5 / g5-cfi，k=1、官方方式；计数
  28 → 27，S1-S4 全部闭合）**：新二进制（orz 05231f7）10 试次复验——
  read_file 锚点返回实机可见（mteb-retrieve 终答引用「读取锚点
  sha256=1dec6f9c…d24bb，size=…」；break-filter 137 次 read_file）；
  写门禁陈旧拒单→重读重下由 S2 单元级预演覆盖、实机无陈旧写入发生；
  零误拒（10/10 零 content_anchor_mismatch，全部写入订单正常执行；
  22 次 console_order_rejected 全为模型输入错误：step_not_done 8 /
  invalid_arguments 12 / policy_denied 2）；事件面留痕（22 条机械盖章，
  read-anchor 专属形态由 S2 断言锁定）；命中率 10/10 有 journal
  94.11%–98.55% 全 ≥90%、10/10 零 HTTP 400。登记于 ADR-0010 §14.38
  第 5 项 / [设计 §11](READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md) /
  TODO P0-0f / CLI_PROJECT_INDEX。

### 0g. MECHANICAL-AUDIT-LAYER（P0；2026-08-24 设计定稿；**S1 已实施
2026-08-24（合并实施包）+ S2 测试已实施 2026-08-24 + S3 重建完成
2026-08-24 + S4 复验闭环 2026-08-25（补登记）**；设计轮不动计数；
S1-S4 全部闭合）

- 入口：[设计](MECHANICAL_AUDIT_LAYER_DESIGN_2026-08-24.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；ADR-0010 §14.39（v1.39）。
- 来源：2026-08-24 用户多轮裁决——同模型 lean harness 对照（Codex
  pass@1 78.65% / Maka 73.03% / 官方 DeepSeek V4-Flash-0731 TB2.1=82.7）
  vs 本机 10 题 50%、每真实动作 2.3–2.9 模型往返、检索 10 试次零调用、
  引用校验 3/8 硬 block 吞终答、P1 读范围门可绕过且拦合法读取。
- **2026-08-24 设计定稿（用户裁决）**：①首轮 plan-first 硬门形态不变
  （钉死极简模式）；②第 2 轮起 direct 执行面（模型直接调工具，一次调用
  一个往返）；③助理层拆机械审查层＋半助理层——机械审查静默记录、每对象键
  仅保留最后一轮结果覆盖写、不给建议、报告随最终答案前中立问询轮注入
  （2026-08-24 裁决收敛：报告仅执行事实摘要=动作/文件 delta/预算/异常
  事实，step/契约类只事件留痕、不上报告）；半助理层承接命令运行/写执行/
  检索派发（host 拥有 cwd/env/超时）；
  ④ACAF、权限轴、预算/墙钟、候选计数、run_tests host-owned、read-anchor
  写前核证维持前置硬门；⑤round 2+ 不要求 step_id 绑定，step/契约/receipt
  仪式退役；⑥输出级引用校验器整体删除；⑦读范围放开（保留 `.gsa` 不可见、
  16KB 信封、head_limit）；⑧主面恢复检索（web_search/web_fetch/browser_read/
  retrieve_project_*，路由仍派发子代理）。合并为一个 S1 实施包。
- 实施路由：S1 代码 → S2 测试 → S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）
  → S4 复验（同一 10 题 k=1 + 构造题：检索调用出现、审计报告覆盖写、
  零 400、命中率 ≥90%、轮次/耗时下降、reward 对比）。计数：设计轮不动；
  实施放行入账 +1；S4 闭环 -1。
- **S1 实施（2026-08-24，合并包）**：读放开（access_in_scope 删 cwd 包含性、
  保留 .gsa 不可见/16KB 信封/head_limit）；引用校验器整体删除（Rust 模块、
  事件、TUI 投影、schema/fixtures/Python verifier 零残留，提示词引用纪律
  改轻量）；主面恢复检索（web_search/web_fetch/browser_read/
  retrieve_project_*，relay 派发不变）；direct 执行面（round 2+ 直接调
  工具；blackboard_action_write/console_step_done/console_return_to_console
  退役出声明面；submit 非硬门）；机械审查层（mechanical_audit.rs：对象键
  覆盖写/上限 128/无建议；mechanical_audit_update 轻量事件留痕；
  [MECHANICAL_AUDIT v0.1] 报告随终答前中立问询轮注入，step/契约类不上
  报告）；半助理层硬门（ACAF/权限/预算/候选计数/read-anchor）维持。
  验证：orz-loop 563 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增；
  orz-host/tui 失败集与 HEAD 基线一致（预存）。登记于 ADR-0010 §14.39
  第 2 项 / TODO P0-0g。
- **S2 测试（2026-08-24）**：删/改既有 gate 测试——orz-host/tui 脚本化
  计划统一补固定末步 deliver（AGENT-DELIVERY-FLOW 末步校验的既有失配
  23+5 项全部适配）、acp_server 事件计数 16→18（S1 审计事件留痕）；新增
  6 项专项测试（审计报告随终答前反例自查轮注入且三类收敛无建议、direct 面
  journal 零退役工具调用、终答未绑定引用原样交付、主面检索族声明、主车道
  web_search 派发子代理、检索候选计数/超限分类）。验证：orz-loop 569 通过
  / 0 失败 / 3 ignored、orz-tui 178 / 0、orz-host 串行 221 / 0（并行仅
  `call_tool_timeout_kills_process_tree` 预存时序 flake，HEAD 基线同样
  失败、单跑通过）、orz-assurance 144 / 0、orz-bin 11 / 0、fmt 干净、
  clippy 无新增。登记于 ADR-0010 §14.39 第 3 项 / TODO P0-0g。
- **全面审查处理（2026-08-24）**：P1-1——read-anchor 写前核证补 direct
  面落点（verify_content_anchor 解耦为 (file_path, label) 签名 +
  run_host_tool_with_timeout 执行前核证门 + search_replace 声明补
  expected_anchor 参数；错误锚点拒绝/正确锚点放行/无锚点放行/新建跳过四
  类 direct 测试，审计层锚点拒单异常真实化）；P1-2——三个退役 console
  工具调用面窄门拒绝（retired_tool_denied、零 ToolStarted、零
  console_order_written/rejected 事件、零副作用）；P2-1——cmd: 审计摘要
  格式修复（括号配对 + stdout 截断机械标记替代文件 delta 顶替）；P2-2——
  候选拒绝结构化错误码透传（candidate_cap_exceeded / candidate_count_
  unbound 精确异常分类）；测试迁移——21 项已退役订单链 e2e 删除、
  acp_server/codex_app/orz-tui 8 处订单脚本迁移为 direct 工具调用、订单层
  FUS-READ-ANCHOR 四场景保留为休眠路径验证（upstream order_id → label）。
  验证：orz-loop 551 通过 / 0 失败 / 3 ignored、orz-tui 178 / 0、orz-host
  串行 220 / 0（并行仅预存 flake call_tool_timeout_kills_process_tree，
  单跑通过）、fmt 干净。登记于 ADR-0010 §14.39 第 4 项 / TODO P0-0g。
- **S3 重建（2026-08-24）**：Linux musl（ORZ-BUILD-MOUNT-001 契约，
  build_orz_aliyun.sh，rust:1.97-slim 增量构建 -j 1）BUILD_EXIT=0；
  三件套时间戳 2026-08-24 06:15 HKT（orz 104,718,240 B / orz-signer
  1,388,592 B / orz-acaf-provision 1,206,576 B），编译 6m47s；最小
  可执行冒烟=三件正常加载执行（orz 无 TTY io error 属预期、provision
  usage、signer manifest 缺失）；守卫符号 retired_tool_denied /
  content_anchor_mismatch 各 8 命中；无 glibc 动态解释器字符串（musl
  静态确认）；对应源码 orz 033fd26 + 父 54560b4（提交并推送）。登记于
  ADR-0010 §14.39 第 5 项 / TODO P0-0g。二次构建轮 s3b（2026-08-24
  08:19 HKT）：reasoning_content 修复 orz 4ca60c2 + 父 c1ce4c1（均已
  推送），三件套 orz 104,726,432 B / signer 1,388,592 B / provision
  1,206,576 B，编译 5m04s，冒烟同前。
- **S4 复验闭环（2026-08-25 补登记）**：2026-08-24 sweep-mal-s4 实机
  （5 题 k=1：git-multibranch / break-filter-js-from-html /
  code-from-image / mteb-retrieve / sam-cell-seg，deepseek-v4-flash）
  5/5 reward 1.00、零异常、48m06s；5/5 journal 事件链完整性 100%、零
  fail 事件、[MECHANICAL_AUDIT] 报告注入齐全（22–106 处引用）；审计
  报告覆盖写由 S2 专项测试预演覆盖；检索可达项在该轮暴露缺口（检索族
  零可见，即 0h 主题根因）——由 RETRIEVAL-SUBAGENT-WIRING（0h）S4
  单题实测补足（2026-08-25 mteb-leaderboard：web_search×19 /
  web_fetch×10、降级 transition 落盘、reward 1.00）。计数 31 → 30
  （0g S1 放行入账 29 → 30 补记于 TODO 快照）。登记于 ADR-0010 §14.39
  第 6 项 / TODO P0-0g。

### 0h. RETRIEVAL-SUBAGENT-WIRING（P0；2026-08-25 设计定稿；S1 已实施
2026-08-25 + S2 测试已实施 2026-08-25 + 全面审查处理已完成 2026-08-25
（CLI 运行路径接通检索模式、scope/max_results 入契约、工具面跟随模式、
前缀收紧、降级元数据持久化）+ S3 重建已完成 2026-08-25；设计轮不动
计数；S4 复验闭环 2026-08-25；S1-S4 全部闭合）

- 入口：[设计](RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；ADR-0010 §14.40
  （v1.40）。
- 来源：MECHANICAL-AUDIT-LAYER S4 复验（2026-08-24）主模型工具面 13
  工具、检索族零可见——harness 不传 `--retrieval-mode`、会话停默认 off
  （mode=off 投影剔除检索族）；内部子代理触发工具 `retrieve_project_docs`/
  `retrieve_project_source_ledger` 从未注册（仅 relay 路由与测试引用）。
- **2026-08-25 设计定稿（用户裁决）**：①外部子代理=模式 A 自动定档——
  local_browser probe 失败（browser_launch_failed）机械降级
  framework_fallback 并记 `retrieval_mode_transition`（old/new 实值、
  authority=mechanical_probe、reason=browser_launch_failed），取代
  §3.7.1「禁止因浏览器不可用自动切换」；页面级失败（LOGIN_REQUIRED/
  CAPTCHA/PAGE_BLOCKED 等 §3.7.2 显式状态）不降级；工具面定档=浏览器
  可用只有 browser_read、不可用只有 web 族；TB harness（tb_agents/
  orz.py）PUBLIC 时传 `--retrieval-mode local_browser`（容器无浏览器→
  自动降级 web 族）。②内部子代理重新定位=结构化检索外包（主代理点读保留；
  多文件/跨目录调研打包派发，`[DOC]` 结构化结果+ledger 回传，隔离上下文、
  降低主对话污染，§3.7 条 8 语义）；controller 声明面注册
  `retrieve_project_docs` ToolDef（relay 路由已存在，声明即触发），内部
  lane 工具面仅读族（read_file/list_dir/grep/search_tool/
  project_doc_index），web 族与 browser_read 不进入。③prompt 以框架
  使用提示（tips，≤1 句）告知使用方式（多文件调研用检索外包、点读用
  read_file/grep），模型自主决定、不做硬门不加仪式。候选计数/并发=1/
  ACAF 前置/子代理状态机/写域 deny-only 不变。
- 实施路由：S1 代码（A 档降级 + 内部工具注册 + prompt tips + harness
  传参）→ S2 测试 → S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）→ S4
  复验（构造题：检索调用出现、降级 transition、零 400、命中率 ≥90%、
  reward 不降）。计数：设计轮不动；实施放行入账 +1；S4 闭环 -1。
- **S1 实施（2026-08-25）**：A 档降级——`apply_mode_a_auto_degrade`
  （acp_server：local_browser probe 失败 → framework_fallback +
  transition pending）+ controller `with_retrieval_mode` 可选 transition
  元数据（mechanical_probe/browser_launch_failed）；主面声明
  `retrieve_project_docs`（controller 声明面、relay 路由已存在、mode=off
  剔除）；`subagent_tool_projection` 加 role 参数（内部 lane 仅读族、
  外部 lane 维持 web 族 + browser_read）；prompt ≤1 句内部检索 tips；
  TB harness PUBLIC 时传 `--retrieval-mode local_browser`。验证：
  orz-loop 556 / orz-host 222 / orz-tui 178 / orz-assurance 144 /
  orz-bin 11（+ 集成目标全过）、fmt 干净、clippy 无新增。登记于
  ADR-0010 §14.40 第 2 项 / TODO P0-0h。
- **S2 测试（2026-08-25）**：新增 5 项专项测试（模式 A 降级规则、
  降级 transition 元数据、主面 retrieve_project_docs 声明、主车道内部
  派发、内部 lane 投影仅读族）；全量全绿。登记于 ADR-0010 §14.40 第 3 项
  / TODO P0-0h。
- **S3 重建（2026-08-25）**：Linux musl 三件套（ORZ-BUILD-MOUNT-001
  契约，build_orz_aliyun.sh，rust:1.97-slim 增量构建 -j 1）
  BUILD_EXIT=0；三件套 2026-08-25 02:46 HKT（orz 104,796,704 B /
  orz-signer 1,388,496 B / orz-acaf-provision 1,206,480 B），编译
  6m10s；冒烟=三件正常加载执行（orz 无 TTY io error 属预期、provision
  usage、signer manifest 缺失）、守卫符号 retired_tool_denied /
  content_anchor_mismatch 各 8 命中、新检索接线符号（retrieval-mode /
  browser_launch_failed / retrieve_project_docs）在二进制内、musl 静态
  （无 PT_INTERP）；对应源码 orz f4f1b81 + 父 48cb030（均已推送）。
  登记于 ADR-0010 §14.40 第 5 项 / TODO P0-0h。
- **S4 复验闭环（2026-08-25）**：单道检索题实机（terminal-bench 2.1
  mteb-leaderboard，k=1，deepseek-v4-flash，sweep-0h-s4-lb，
  33m23s）reward 1.00、零异常——模型经 GitHub API 锁定 2025-08-29
  结果仓库快照（71f6b62）后用 mteb 1.38.41 计算 Scandinavian 全任务
  Mean (Task) 并按全任务过滤，终答 GritLM/GritLM-7B（17 字节，
  read_file 锚点核证）；检索调用出现（主面 web_search×19 /
  web_fetch×10，外部子代理检索结果侧车 2 份落盘）；降级 transition
  落盘（old=local_browser → new=framework_fallback，
  authority=mechanical_probe、reason_code=browser_launch_failed、
  capability_status=available）；零真实 400 / 零 tool_failed / 零
  run_invalidated；journal 事件链完整性 100%（619/619，命中率 ≥90%
  达标）；DoD 1-7 全部满足。计数 30 → 29（0h S1 放行入账 30 → 31
  补记于 TODO 快照）。登记于 ADR-0010 §14.40 第 6 项 / TODO P0-0h。

### 0i. FINAL-SMOKE-2026-08-25 对拍暴露问题（P0；2026-08-25 登记；
正式 89 题提交前处理）

- 入口：索引 GAP-EVENT-SCHEMA-DRIFT / GAP-REPETITION-DETECTOR-DNA-
  FALSE-POSITIVE；TODO P0-0i。
- 来源：最终冒烟（5 题 k=1 官方标准，final-smoke-2026-08-25；结果
  llm-inference-batching-scheduler 1.00 + sam-cell-seg 1.00，
  make-doom-for-mips 900s 墙钟超时、dna-assembly/feal 1800s 墙钟
  超时）确定性对拍——机制面零 400、零 run_failed/run_invalidated、
  ACAF 票据 50/50、交付流 submit 双阶段走通；严格校验暴露两类问题：
    1. **GAP-EVENT-SCHEMA-DRIFT**：5/5 run 严格 payload 校验未过（哈希
       链 0 错误、envelope 0 错误）——① retrieval_mode_transition 枚举
       缺 mechanical_probe/browser_launch_failed（0h 模式 A 降级值未
       同步 runtime schema）；② ledger_fold_advance 生产者多写
       view_estimate_after（schema additionalProperties=false）；
       ③ control_ticket_issued 外部检索 lane network 票带 activation_id
       （D-13 检索 lane 绑定语义，schema 要求动作票 null）。修复=runtime
       schema + run_event_journal_validation.py + fixtures/conformance
       + 复验。**2026-08-26 修复完成**：① authority 枚举补
       mechanical_probe、reason_code 枚举补 browser_launch_failed；
       ② view_estimate_after 按设计定案入 schema 必填 + verifier 交叉
       检查（推进后估算 < 推进前估算，即回落触发阈值之下）；③ 动作票
       activation_id 放开为可选绑定（D-13 语义入 schema，检索 lane 票
       带真实 activation_id、主 lane 保持 null）。fixtures 补
       mechanical-degrade / network-lane-bound 正例锁，测试语义更新，
       check_repository 补 fixture 全映射；事件链复验=5 个冒烟 run 三类
       漂移清零（3 个墙钟超时 run 缺终止事件为 harness 杀进程边界、
       按 require_terminal=false 回放语义豁免），runtime/assurance
       pytest 1898 passed、check_repository valid。
  2. **GAP-REPETITION-DETECTOR-DNA-FALSE-POSITIVE**：dna-assembly 中
     EGFP 400 字符序列合法重复引用触发 3/3 命中（无切分点直接判真）、
     降级 high→low→disabled；缺口 A 延续。**2026-08-25 设计定案（用户
     裁决）**：L=400 维持 + 序列内容门（无切分点路径 sequence_like
     判定、序列族命中门槛 3→5）；否决 L=1000 提档与整体走路径②（检测
     盲区）；见 REPETITION_DETECTOR_SEQUENCE_CONTENT_GATE_DESIGN /
     ADR-0010 §14.41；**S1 已实施（2026-08-25 实施放行 31 → 32：
     REPETITION_SEQUENCE_LIKE_RATIO=0.90 /
     REPETITION_SEQUENCE_HIT_LIMIT=5、sequence_like 判定、无切分点
     分支双门槛分派、sequence_gated 审计标注；orz-loop 563/0/3 全绿、
     fmt/clippy 无新增；核心测试随 S1 自证）**；**S2 测试完成
     （2026-08-26：EGFP 400 字符真实 span 回放 3 次命中 0 trip / 5 次
     触发、poly-A 399/400/401 静默、序列族间隔不重置 + 流结束丢弃、
     路径②/stall/标点块/混合分别计数回归全绿；orz-loop 567/0/3）**；
     **S1 全面审查处理完成（2026-08-26：蛋白/氨基酸序列覆盖扩展——
     严格 20 氨基酸字母表 + 独立阈值 `REPETITION_PROTEIN_LIKE_RATIO`
     =0.95 + `sequence_kind` 单次扫描双族判定 + 审计/触发 detail 带
     kind（dna_rna/protein）；覆盖面实证=英文无间隔长串蛋白占比 0.8975
     （否决蛋白共用 0.90）、真实蛋白（UniProt P35579）1.00；阈值百分比
     提为编译期常量、同 delta 多次命中按累计时刻标注、设计 §5 数字非
      切分符勘误；orz-loop 569/0/3、fmt/clippy 无新增）**；**S3 重建完成
      （2026-08-26：Linux musl 三件套，orz 104,796,752 B / orz-signer
      1,388,496 B / orz-acaf-provision 1,206,480 B，编译 6m19s；守卫符号
      retired_tool_denied / content_anchor_mismatch 各 8 命中、序列门
      审计字段（sequence_gated/dna_rna/protein）与检索接线符号在二进制
      内、无 PT_INTERP（musl 静态）；容器冒烟三件正常加载执行；对应源码
      orz 6b208fb 已推 cli 远端）**；**S4 复验闭环（2026-08-26，未闭合
      30 → 29）**：新二进制（orz 6b208fb，104,796,752 B）k=1 官方标准
      重跑（s4-dna-2026-08-26 / dna-assembly__3EyUPDL，1800s 墙钟超时
      reward 0.0，与旧 run 结局类别一致）——EGFP 式合法引用零误杀降级
      （1–4/5 次命中全部仅审计 sequence_gated kind=dna_rna、5/5 次命中
      才 trip；旧 3/3 即直降 Disabled）；真复读仍触发（trip 只降
      EnabledLow、非直降 Disabled，降级后继续工作）；零真实 400（事件链
      8 处“400”均为哈希串）；事件链严格校验除「缺终止事件」1 项豁免
      （墙钟超时 harness 杀进程边界，require_terminal=false 回放语义；
      与旧 run 同构）外 0 错误；journal 口径命中率 82.36%（22 请求，
      762,496/925,805）vs 旧 82.33% 持平（web 检索注入，观察项不阻塞）；
      请求 36→22、reasoning 46,626→136,956、request_header_change 6→3、
      无 reasoning_stall 触发。登记于 ADR-0010 §14.41 第 3 项 / TODO
      P0-0i / CLI_PROJECT_INDEX（GAP `partial` → `implemented`）。
  3. 观察项（**2026-08-26 S4 重跑实测更新**）：dna 82.36%（S4 重跑）/
     feal 88.21% 命中率 <90%（web 检索注入相关，成本观察不阻塞）；feal
     一次 reasoning_stall 64K 预算设计内触发（记录）；S4 dna 重跑无
     reasoning_stall 触发。
  - 下一步：**schema 三类漂移修复 ✓ + 事件链复验 ✓（2026-08-26，5 run
    非终止错误 0）+ DNA 域 S1/S2/S3/S4 全部闭合 ✓（2026-08-26，S4 重跑
    零误杀降级、真复读仍触发、零 400；命中率观察项登记）**；下一项=
    P0-F make-doom 加预算重跑 → 89 题官方跑分（k=5、`--upload --public`，
    先 `harbor auth login`）。

### 0. 前置收尾（提交前需用户确认）

- 已完成（995a384）：提交当前未提交登记——CLI_PROJECT_INDEX 索引更新、两份设计文档（含优先级标记）、本文件与各指针更新。

### 0j. THIN-HARNESS-REDESIGN-V2（P0；2026-08-28 设计定稿，实施待放行）

- 入口：[设计](THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md) /
  [HA 调研](HA_SERVICE_MODEL_RESEARCH_2026-08-28.md)；TODO P0-0j。
- 来源：THIN-HARNESS-REDESIGN（2026-08-27 v0.4）减法延续——思维侧收尾 +
  执行侧重设计；同模型官方极简 harness 82.7% vs 厚 harness 65.2%，模型能力
  非瓶颈，回归"降低模型压力"原路。
- 定案（2026-08-28 用户裁决）：复读守卫统一命中门槛 20 + 序列内容门全删 +
  3-gram 门槛 15 + 802 线保留 + 空响应链 thinking 降档最多 low（不关闭）+
  复读触发改显式拦截不降档 + 审计只消费结构化字段（杜绝"400 在哈希串"类误报）
  + 半助理层加厚（失败自动诊断 ≤2KB 极简记录 / process-file-environment
  三实体域 / 实体状态并入黑板 / 黑板定义进工具描述不进 prompt）+ HA 服务模型
  （domain.service + target + data，target=实体级、无作用对象省略）。
- 路由：W1（R1）复读后置化 + 空响应链 + 审计治理；W2（R2）半助理层失败诊断
  + 实体登记 + 服务调用形态收敛 + 黑板接线；W3（R3）A/B 验证与清理 +
  ADR-0010 修订 + CLI_PROJECT_INDEX 登记（含 THIN-HARNESS v0.4 遗留
  R2b/R2c 观察与回收判定）。
- 计数：设计轮不动（仍 29）；**2026-08-28 R1 S1 代码 + S2 测试实施放行
  入账 29 → 30**（S3 重建 / S4 复验待续）；**2026-08-28 R2 S1 代码 +
  S2 测试实施放行入账 30 → 31**（W2-R2 四块：失败诊断 `diagnostics.rs`
  + 实体登记 `entities.rs` + 服务调用形态收敛（target 实体级）+
  黑板 entities 分区接线；orz-loop 592 通过 / pytest 216 通过 /
   fmt 干净 / clippy 无新增；S3 重建 / S4 复验待续）；验证闭环按既有纪律。
   **2026-08-28 全面审查处理（R1+R2 S1/S2 三路审查）**：R1 无返工（仅
   文档措辞勘误）；R2 两个 P1 修复（file 域签名消费 stat 探针 /
   target↔data 二选一 + 双写一致性校验 + 域前缀校验）与 P2/P3 处理完成
   （明细见 TODO P0-0j W2-R2 全面审查处理）；orz-loop 608/0/3、pytest
   216、fmt/clippy 干净；S3 重建 / S4 复验仍待容器/实机放行。

### 1. FUS-TOOL-PROBE（`implemented`；P0-A 批次 1-7 与 P0-A-2 已闭合）

- 入口：[设计](TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：2026-08-13 用户已裁决放行实施；ADR-0010 §3.5 修订采纳按批次末第 7 步登记。
- 进度（2026-08-13）：步骤 1 已完成——面 B 探针模块（orz-loop `tool_probe.rs`：两态中性判定 + 失败兜底 + 面矩阵成员与判定单测 15 项）；步骤 2 已完成——v0.2 事件升级（新 payload Schema + verifier 交叉校验 + fixture 重生成 + producer 接线 + TUI 消费面适配），13 个真实 journals 重捕并验证；步骤 3 已完成——`run_tests` 条件声明迁移（controller 删除直接 `test_runner()` 条件声明，声明改由面 B 探针快照驱动：完整才列出、不完整即移除，中性 reason `缺少测试运行器`；新增 registry 已声明但 runner 缺失/存在两条迁移语义测试）；步骤 4 已完成——列表投影接线（tool_probe 补面 A/面 C 常量与 `is_main_agent_work_tool` 判定；controller 模型可见列表 = 面A + (面B完整集 ∩ 会话声明集) + 面C + 非工作工具，仅名称；ReadOnly 下 `search_replace` 写探针不完整即移除，面 C/非工作工具不探不标；新增投影分区、交互会话、面 A/C 成员判定测试，Benchmark/ReadOnly 全量声明测试同步新语义）；步骤 3/4 全面审查清理已完成——run_tests 无 runner 竞态调用改为 dispatch 前无 ToolStarted 中性拒绝（`missing_test_runner`），补 workspace 不可读投影移除与竞态兜底测试；步骤 5 已完成——最小上一轮映射与翻转事件（`MinimalProbeMap` 仅存 `tool → 完整/不完整`、不缓存 reason、不跨 run；每个模型请求构造前重算面 B 快照并重算列表投影，翻转才发 `tool_availability_check` 事件、无变化不发；调用即探针：面 B 工具 ToolCompleted(error) 回写最小映射——run_tests 竞态拒绝/spawn 失败/ACAF fail-closed 拒绝/宿主调用错误，回写仅主/grill 车道、检索车道不写主映射（审查修复）；主/grill 车道探、检索车道不探不发；新增翻转重投影、调用失败回写恢复翻转、检索车道零污染、跨 run 重置四项集成测试）；步骤 6 已完成——兜底消息中性化改造（权限门禁拒绝消息改 `tool 'X' — 本次调用未获权限门禁放行`；连续拒绝熔断块改中文中性陈述，移除 available/refused 旧措辞；系统提示词改"工具列表由运行时按轮声明"；检索车道按复核裁决保持原设计，拒绝/宿主执行消息不脱敏；run_tests 管道错误改 unreadable；DC 最小动作改"完整错误输出"；补中性词契约测试）；步骤 7 已完成——ADR-0010 §3.5 修订登记为 v1.8（单一探针面取代 v1.5「registry 全量 + 零可用性承诺」语义，A+C→B 定档合并登记，ADR §14.8；§14.5 补复核注）；P0-A-2 已闭合——单一探针面覆盖全部 23 个工作工具（见下）。orz-loop/host/tui 测试全绿、仓库门禁 valid。
- 方向裁决（2026-08-13，定档）：**A 面与 C 面全部并入探针面（B）**——单一规则：本轮模型可见 = 机械链路完整 ∩ 会话声明集（非工作工具与检索车道走各自既有门）。A 面 7 个控制工具（`blackboard_read` / `todo_write` / `update_goal` / `enter_plan_mode` / `exit_plan_mode` / `compaction_whitelist_add` / `retrieval_disposition`）与 C 面 9 个工具（`run_terminal_cmd` / `lsp` / `memory_get` / `memory_search` / `image_gen` / `image_edit` / `image_to_video` / `reference_to_video` / `use_tool`）各自补机械链路探针（journal/工作区/goal context/会话类型/权限策略/后端配置/MCP 能力注册等），面 A/C 撤销，B 面升级为全工作工具探针面。该定档与步骤 7 ADR-0010 §3.5 修订合并登记。
- 审查裁定（2026-08-13，子代理三路审查）：步骤 4 列表投影语义定为 `面A + (面B完整集 ∩ registry 声明集) + 面C`（部分会话变体可移除 ask_user_question/search_tool，探针不得声明会话不存在的工具）；探针粒度=工作区根级机械检查、写探针 metadata-grade、交互用户信号=ACP live gateway，均已登记进设计文档。（该投影语义已被 v0.2 单一探针面取代，见上一条方向裁决。）
- 实施序列：
  1. 面 B 每工具探针实现（路径/权限/策略/runner/交互用户判定）；
  2. `tool_availability_check` Schema/fixture/verifier 升级与 producer 接线；
  3. run_tests 条件声明迁移；
  4. 列表投影接线（面 A + 面 B 完整集 + 面 C，仅名称）；
  5. 最小上一轮映射与翻转事件；
  6. 兜底消息中性化改造；
  7. ADR-0010 §3.5 修订裁决与登记。

### 1b. FUS-TOOL-PROBE v0.2 单一探针面扩展（P0-A-2，已闭合）

- 完成内容（2026-08-13）：面 A/C 撤销——`tool_probe.rs` 以单一 `WORK_TOOLS`（23 个工具）取代
  三面常量，全部工具按设计 §2.0 表各自补机械链路判定（读/写/存储/goal 上下文/plan 模式/
  检索激活/终端/lsp/memory/图像/视频/MCP 注册，全部为廉价确定性检查）；`LoopHost` 新增
  terminal/lsp/memory/image/video/MCP 六项 fail-closed 能力访问器（orz-host 覆盖终端=true，
  其余按当前 build_toolset 未接线=false）；controller 新增 goal_context_present /
  has_live_activation（待处置激活，2026-08-13 审查复核收紧为未决 pending assessment）
  探针信号；列表投影切换为 探针完整集 ∩ 会话声明集 + 非工作工具；
  `tool_availability_check` 事件 complete/incomplete 覆盖全部工作工具（Schema 描述、Python
  verifier `_WORK_TOOLS`、fixture 生成器与 12 个 v0.2 journal fixtures 同步并重算哈希链）；
  调用即探针回写覆盖全部工作工具（仍限主/grill 车道）；投影/翻转/车道隔离测试更新。
- 验证：orz-loop 236 通过 / 0 失败、orz-host 199 通过 / 0 失败、orz-tui 178 通过 / 0 失败；
  Python verifier 与相关 assurance 测试 395 通过；仓库门禁 valid、0 错误。
- 边界：orz-host 可选后端（lsp/memory/图像/视频/MCP）当前全部未接线——探针按 fail-closed
  移除这些工具；未来接线须翻转 orz-host 对应能力访问器并补翻转测试。真实 journals 为已提交
  fixtures 重建（12 个含探针事件），非新捕获运行。
- 边界（2026-08-13 审查复核）：`retrieval_disposition` 探针收紧为"未决 pending
  assessment"（Active 无 pending / continue 已决均不完整）；plan 模式探针以交互用户
  信号代理，未来 headless 计划模式会话需独立能力信号。

### 2. FUS-RETRIEVAL-MECH（`implemented`；P0-B，批次 1-6 已闭合 2026-08-14）

- 入口：[设计](RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：2026-08-13 用户已裁决放行实施。
- 进度（2026-08-13）：**步骤 1（B-1 闭合）已完成**——`ToolResult` 新增
  `structured` 接缝（host 仅 web_search 填充 citations），controller 机械提取
  为证据 `candidate_urls` 并写入 `source_ledger`/`raw_source_refs`（Schema
  与 Python verifier 先行，`_verify_v02_search_candidate_pool` 校验镜像）；
  orz-host 200 / orz-loop 238 / orz-assurance 139 / orz-tui 178 通过，Python
  全量 1821 通过，仓库门禁 valid。边界：候选池暂不进模型提示词（预筛步骤负责
  展示），web_search 摘要条目仍不写 tier。审计：
  [GAP-RETRIEVAL-MECH B-1 实施审计](audits/GAP_RETRIEVAL_MECH_B1_CITATIONS_IMPL_AUDIT_2026-08-13.md)。
- 进度（2026-08-14）：**步骤 2 已完成**——web_fetch 候选机械计数门禁与计数
  反馈（`ORZ_WEB_FETCH_CANDIDATE_CAP` 用户裁决定档 8；per-activation 计数域
  随 activation 侧车持久化，经 LoopProfile 穿过子代理循环；精确字符串去重；
  未超限结果携带“候选 N/M，剩余 K”反馈 + `tool_completed` 审计字段
  candidate_count/candidate_cap（Schema 先行）；超限无 ToolStarted 拒绝 +
  连续拒绝熔断同面；缺 url/无计数域 fail-closed）；orz-loop 244 / orz-host 200 /
  orz-assurance 139 / orz-tui 178 / orz-bin 全绿，Python 全量 1828 通过，
  仓库门禁 valid（含审查修复：verifier 派发包装误报、黑板计数反馈一致、
  continue 跨派发累计测试）。审计：
  [GAP-RETRIEVAL-MECH 步骤 2 实施审计](audits/GAP_RETRIEVAL_MECH_STEP2_WEB_FETCH_CANDIDATE_COUNT_IMPL_AUDIT_2026-08-14.md)。
- 进度（2026-08-14）：**步骤 3 已完成**——机械预筛模块（候选池净化 + 排序：
  canonical/host 级去重、已知失败形态 bad_url/login_wall/redirect_chain 移除、
  tier/weight + 词法相关性排序；`candidate_urls` 升级为预筛后保留池，新增
  `candidate_pool` 每候选元数据与 `prefilter_log` 移除日志，Schema/verifier
  先行；配置种子 `runtime/candidate-prefilter-config-v0.1.json` +
  `ORZ_CANDIDATE_PREFILTER_CONFIG` 覆盖；orz-assurance 新模块 + orz-loop
  接线）；边界：超大页/robots 无法离线判定保留 unknown、计数域精确去重
  语义不变、模型面展示留待步骤 6。审计：
  [GAP-RETRIEVAL-MECH 步骤 3 实施审计](audits/GAP_RETRIEVAL_MECH_STEP3_CANDIDATE_PREFILTER_IMPL_AUDIT_2026-08-14.md)。
- 进度（2026-08-14）：**步骤 5 已完成**——输出级引用校验器与交付边界接线
  （主 Agent 最终回答在 counterexample gate 同一交付边界机械校验 `[来源: ...]`
  标记：结构化解析、ledger source_id / 主车道 path:line / URL+observed scope /
  文档身份绑定、§3.7.5 claim 上限；失败以 `[CITATION_VALIDATION_FAILED]` 机械
  降级块替代交付并 journal `citation_validation` 事件，reason codes + marker
  明细；Schema/verifier/fixtures 先行，v0.2 事件枚举 42→43；主车道补
  main_evidence 读取证据 + run_source_ledgers，TUI 投影同步；conformance 新增
  第 14 个场景并捕获 journal）；验证：orz-loop 268 / orz-assurance 151 /
  orz-tui 178 / orz-bin 42 通过，Python runtime 239 / assurance 1621 通过，
  仓库门禁 valid；既有 13 份 journals 重捕无漂移。审计：
  [GAP-RETRIEVAL-MECH 步骤 5 实施审计](audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md)。
- 进度（2026-08-14）：**步骤 6 已完成**——提示词相应缩短与测试更新：
  主 Agent 提示词引用纪律缩减为"标记格式 + verifier 交付前机械校验"
  （设计 §3.3）；检索子代理提示词移除"候选 ≤5 / never the full reference
  list"软约束，改指机械预算反馈（"候选 N/M，剩余 K"）；来源加权/引用规则
  段落去冗余；prompt.rs 测试同步更新。验证：orz-loop 281 / orz-assurance
  151+1 doctest / orz-host 207 / orz-tui 178 / orz-bin 42 通过，Python
  runtime 244 / assurance 1607+14 skipped 通过，仓库门禁 valid、0 错误。
  审计：
  [GAP-RETRIEVAL-MECH 步骤 6 实施审计](audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md)。
- 依赖顺序：1（B-1 闭合）→ 2/3/5 → 4（闭合）→ 6。
  1. （已完成）B-1 闭合：web_search citations 结构化透传 loop；
  2. （已完成）web_fetch 计数门禁 + 计数反馈 + `ORZ_WEB_FETCH_CANDIDATE_CAP` 接线；
  3. （已完成）机械预筛模块（候选池净化 + 排序标签）与结构化结果扩展；
  4. （已完成）browser_read 范围/模式参数（全文/预览/关键词提取）工具能力扩展；
  5. （已完成）输出级引用校验器与交付边界接线；
  6. （已完成）提示词相应缩短（计数/预筛/引用规则）与测试更新。
- 批次已闭合（2026-08-14）：DC 剩余两信号（`same_module_no_evidence` /
  `key_surface_unexamined`）未并入批次，转 P3 遗留小项独立跟踪。

### 3. CLASSICAL-EXEC-ASSISTANT（POC 已通；实施中）

- 入口：[设计](CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；POC：[`prototype/classical_console/README.md`](../prototype/classical_console/README.md)。
- 定位：给主模型套一个真正的操作台（而不是工作环境）——模型面只注册工具名/动作名，助理层按主模型输出逐层定位组件并执行，不理解动作语义；注册表路由 → 契约校验 → 目标解析 → ACAF 票据 → 执行 → verifier；承接 OPS-PROTOCOL 裁剪方向；不新增平行协议、不拥有决策权、执行层全部确定性。
- 威胁模型（2026-08-13 用户裁决）：只防幻觉与注入；助理层 = 单一策略执行点（围栏合并为策略表 + taint 动作组合禁令）；动作级策略沙盒成立，OS 级执行沙盒按动作另行挂载。
- 调研结论（2026-08-13）：Rhasspy 已归档（2025-10）、Rasa 维护模式、n8n 非真开源、Windmill AGPL——均不作底座；**Home Assistant 服务模型 + hassil（Apache-2.0、活跃）为首选成熟参考**；StackStorm/Node-RED/OVOS 备选。
- 集成形态（v0.3 用户裁决）：HA 助理层与 orz 深度融合，是 orz 的一部分；薄接缝改置 orz 本体 ↔ 底座（模型后端，Grok/Codex 等），不是 orz ↔ ConsoleClient；POC 的 stdio 协议仅为原型隔离，不作为生产接缝。
- 借鉴登记（2026-08-13）：DeepSeek Harness（v0.1 预览、MIT）只借设计本身、不引入其技术栈——PTC 程序化工具调用（模型输出程序化动作脚本，操作台逐行确定性执行、失败 fail-closed）+ Profile/Bundle 动作组合（按 Benchmark/ReadOnly/标准场景加载动作集）。
- fail-closed 返回契约（2026-08-13，已落地）：调研 HA 原项目——WebSocket 成功回 `result:{context,response}`、失败回 `error:{code,message}`（message 带校验路径），无失败点/上游结果，不足；错误信封扩展为 `step`（失败点枚举 protocol/intent/registry/contract/target/execute/verify/policy）+ `code` + `message` + `upstream`（解析后真实目标/票据 id/部分输出/verifier 摘要，无则 null），模型可独立排障；POC 已实现并纳入冒烟（21/21）。
  2026-08-15 全面检查修复：策略拒绝（权限/ACAF/taint/模式门）由执行器适配层
  归一化为 `step=policy` + `code=policy_denied`；响应契约强制必填（注册时校验
  并缓存 schema，任何输出过机械验证）；`exit_code=Some(0)` 成功契约。
  2026-08-15 P1-2 定案补登记：拒绝路径（权限/ACAF/模式门；taint 预留）返回
  结构化信号 `ToolResult.policy_denial = {source, code, reason}`，controller
  删除「稳定输出前缀」字符串判定（`console_policy_refusal` 退役）；ToolCompleted
  增可选 `policy_denial`（Schema/verifier/fixtures 先行）；实施随 S3 前置。
- 动作粒度与稳定性裁决（2026-08-13）：细粒度优先——粗按钮（run_terminal_cmd 什么都做）才是限制模型（参数幻觉面大）；细动作必须配套机械组合层（PTC/管道/意图）避免碎片化。负担=构建期线性成本（注册+schema+handler+探针/策略映射+测试），运行时近零；风险在边界漂移与变更连锁；护栏=动作契约按版本化 API 管理（新增优先、废弃走迁移期、参数向后兼容）、探针决定可见性、Profile/Bundle 分区、契约即测试。小样 2 收益量化裁决点已闭合（2026-08-15 用户裁决通过，独立判定一致）。
- 执行失败特殊反馈与日志可见性（v0.4，已落地）：错误信封恒带 `trace_id`；`step=execute` 失败附有界 trace 尾部；新增只读服务 `assistant.trace`（有界、读入审计）——主模型可结合日志与操作台覆盖未预录内容；POC 冒烟 28/28。
- 机械组合模型（v0.4 设计）：线性脚本模式（PTC）——步骤=注册动作实例 + `$ref` 数据引用，每步独立契约校验 + trace，任一步 fail-closed；无任意代码/隐式控制流，循环/条件暂不做；列入实施序列小样 3。
- 黑板动作栏（v0.5 用户提案，定为生产协作形态）：黑板拆三块——注册板块（助理层维护、当前轮动作投影、常驻按需读）、动作栏（模型写订单，无副作用）、结果栏（receipt+trace_id+fail-closed）；发放=机械单一出口（模型轮结束触发、消费一次、round 防重）；模型面不再出现执行/发送工具；stdio 仍为原型隔离。
- 进度（2026-08-13）：小样 1（控制台路由小样）已跑通——`prototype/classical_console/`（HA 式服务注册表 + hassil 意图 + stdio JSON 协议），服务模式（`workspace.read_file` / `workspace.list_dir` / `workspace.index` / `assistant.trace`）+ 意图模式（en/zh，槽位表由工作区索引动态生成），含 fail-closed 契约与执行日志，28/28 检查通过。
- 进度（2026-08-15）：小样 3（机械组合脚本模式）已实施——`workspace.run_script`
  服务（线性脚本 + `$ref` 数据引用，执行前静态校验引用存在/作用域/类型、名称唯一、
  禁嵌套脚本；每步独立 schema 校验 + trace；失败保留内层 step/code 并带
  script_step；上限 20 步/30s/4MiB）；固定语料 8 场景对照实验（baseline 逐轮串行
  20 轮 vs candidate 单脚本 8 轮，成功率均 100%），冒烟 87/87；结果工件
  `prototype/classical_console/sample3_result.json`；**已闭合（2026-08-15 用户
  裁决通过，独立判定一致）**。
- 进度（2026-08-15）：orz 内嵌集成 S1 已落地——`orz-loop/src/console.rs`
  操作台核心（ServiceRegistry + ActionSpec 契约 + issue_action 五步路由
  （注册表/契约/目标/执行/验证）+ fail-closed 信封（step/code/message/
  upstream/trace_id，execute 附有界 trace 尾部）+ 有界 Trace/TraceStore；
  执行经 ActionExecutor 抽象委托，生产实现由 controller 复用 run_host_tool
  的权限/ACAF/事件链，禁止绕过既有门）+ 黑板动作栏数据面
  （`blackboard::ActionBoard`：注册板块/动作栏单槽（单轮一单）/
  结果栏有界 50，随 plan epoch 快照归档与轮换）。**2026-08-15 全面检查修复**——
  执行器返回 ExecuteError（执行失败/策略拒绝，`step=policy` + `policy_denied`）；
  响应契约强制必填（注册时校验并缓存 schema，任何输出过机械验证）；
  `exit_code=Some(0)` 成功契约（None/非零按执行失败）；TraceStore 提交语义
  （发放收口 commit，失败事件满员滚动保底）；注册板块最小参数提示投影；
  S2 验收点显式登记（round/epoch 防重放、目标解析、ACAF 票据、策略表、
  step=policy）。新增 18 项测试（console 15 + blackboard 3）；orz-loop
  356 通过 / 0 失败。**2026-08-15 S2 已落地**——模型面投影（
   `blackboard_read` section=actions：注册板块/动作栏单槽/结果栏有界渲染，
   随 plan epoch 归档可读 + `blackboard_action_write` 写单按钮，pending 机械
  拒绝，round/plan_epoch/run_id 机械盖章，主车道专属三重守卫）+ 轮末机械
  发放（post-tool-batch 安全间隙、pending checkpoint 优先；round/plan_epoch/
  run_id 防重放与过期 → 注册表/契约 → 经 ControllerConsoleExecutor 委托
  run_host_tool（权限/ACAF/模式门/事件链）→ 响应 schema 验证 → 结果栏
  receipt + trace_id → TraceStore.commit；策略拒绝归一化 step=policy +
  policy_denied）；注册板块每轮机械刷新（基础动作集 6 项）。新增 8 项测试，
  orz-loop 364 通过 / 0 失败；实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`。
  assistant.trace 服务与 PTC/Profile 生产化随后续切片落地。
- 进度（2026-08-15）：**S3 前置（P1-2 结构化策略拒绝）已闭合**——
  `ToolResult.policy_denial = {source, code, reason}`（source ∈
  permission/acaf/retrieval_mode/taint）在 `run_host_tool` 边界五条拒绝
  路径接线（权限 deny/defer、ACAF 票据门、检索模式门 ×3）；console 适配层
  只按结构化信号映射 `step=policy`，`console_policy_refusal` 前缀判定退役；
    ToolCompleted 增可选 `policy_denial`（Schema/verifier/fixtures 先行，
    verifier 交叉规则=exit_code 非 0 + 工具命中已知拒绝路径）；内容碰撞回归
    （成功输出含旧前缀判成功）。**2026-08-15 全面审查修复（F1-F8）闭合**：
    拒绝事件补 `exit_code=1` + `status=error`（含 host 级拒绝）、verifier
    ACAF 家族补 `web_fetch`/`browser_read`、新增生产者事件→验证器对拍
    测试、`orz/` 登记为父仓库 git 子模块（SilverWhite/CLI
    `feat/fusion-architecture`，提交 a0c9ffc 已推送远端）+ 源码完整性清单
    接入仓库门禁。验证：orz-loop 367 通过 / 0 失败、acaf_e2e 21 通过、
    Python runtime 213 通过、仓库门禁 valid（含 orz 清单 1406 文件）。
    实施审计见
    `docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`。
- 进度（2026-08-15）：**S3（trace/PTC/Profile）已闭合**——
  `assistant.trace` 注册为控制台内部动作（`ActionKind::TraceRead`：按
  trace_id 有界取回、`not_found` fail-closed；读操作本身入 trace 与
  ToolStarted/ToolCompleted 事件面）；`workspace.run_script` 注册为控制台
  内部动作（`ActionKind::RunScript`：PTC 线性脚本生产化——`$ref` 静态/
  运行时校验、逐行契约校验 + trace、上限 8 步/30s/4MiB、禁嵌套、失败
  保留内层 step/code + `script_step`；内层步骤逐行经 `issue_action` 复用
  注册表/契约/目标/执行/验证五步链，仍走 `run_host_tool` 全链路）；注册
  板块升级为 Profile/Bundle ∩ 探针完整集（`ActionBundle` standard/
  read_only/benchmark + `registrations_for`，同轮探针快照同时驱动工具投影
  与注册板块）。验证：orz-loop 377 通过 / 0 失败（3 ignored live）、
  fmt/clippy 无新增告警、orz-host/tui/bin/assurance check 通过、仓库门禁
  valid（含 orz 清单 1406 文件，已重新生成）。实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md`。
  2026-08-16 全面审查收口（用户逐项裁决）：单订单步数上限 20→8
  （`MAX_SCRIPT_STEPS_PER_ORDER`）、`assistant.trace` 查无 id 定案
  `step=execute`+`not_found`、checkpoint 轮跳过注册板块刷新、注册不变式
  补齐（内部 target/kind 唯一/bundle 非空/嵌套按 kind）、最终超限信封补
  `script_step`；orz-loop 384 通过 / 0 失败；S4 登记项=单步超时（host 层
  进程树收口）与脚本消耗 tool-round 预算，详见审计 §6。**S4 已闭合
  （2026-08-16）**——单步超时下沉 host 层（`call_tool_with_timeout` 覆盖 +
  结构化 `timed_out` → `tool_timeout`/`script_timeout`+`script_step`）、
  脚本 tool-round 预算（预检 `budget_insufficient`/实际步数减计/下一轮预算
  块反映）、端到端测试（完整会话/checkpoint 板块保留/超时/预算边界）；
  orz-loop 392 通过 / 0 失败；实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`。
- 实施序列：
  1. 槽位表由工作区索引动态生成（POC 已闭合；生产接线复用 orz `project_doc_index` 缓存）；
  2. 编辑执行器 `workspace.search_replace`（小样 2，收益裁决点：编辑应用成功率 + 主模型工具轮数）——**已闭合（2026-08-15）**，测量与结果工件见 `prototype/classical_console/sample2_result.json`；
  3. 机械组合脚本模式（小样 3：线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）——**已闭合（2026-08-15）**，用户裁决通过 + 独立判定一致，测量与结果工件见 `prototype/classical_console/sample3_result.json`；
   4. orz 内嵌集成（HA 操作台作为 orz 组件接线；薄接缝在 orz ↔ 底座模型后端；黑板动作栏为生产协作接缝，POC stdio 仅原型隔离）——**S1 已落地（2026-08-15）**：操作台核心 + 黑板动作栏数据面；S1 全面检查修复已闭合（2026-08-15）；**S2 已落地（2026-08-15）**：模型面投影 + 轮末发放（含 round/epoch/run_id 防重放、step=policy 归一化、TraceStore.commit 收口），实施审计见 `docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`；**S3 前置（P1-2 结构化策略拒绝）已闭合（2026-08-15，全面审查修复 F1-F8 已登记）**，审计见 `docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`；**S3（assistant.trace 接线 + run_script PTC 生产化 + Profile/Bundle 加载）已闭合（2026-08-15）**，审计见 `docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md`（2026-08-16 审查收口 §6）；**S4 已闭合（2026-08-16）**：端到端测试 + 单步超时（host 层进程树收口）+ 脚本 tool-round 预算消耗 + 决策门材料，审计见 `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`；
  5. 小样全面达标后裁决正式组件（决策门）；不达标即撤——**已闭合（2026-08-16
     用户裁决「P0-C 可转正式组件」）**。

### 3a. PLAN-FIRST-BLACKBOARD（模型面重构；2026-08-15 用户定案）

- 入口：[设计](PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)（AUTH-PLAN-FIRST-BLACKBOARD /
  FUS-PLAN-FIRST-MODEL-SURFACE / FUS-PLAN-STEP-GATE / FUS-AGENTS-MD-PLAN-WRAPPER /
  FUS-PROMPT-DEPERSONALIZE / FUS-CONSOLE-DUAL-MODE / FUS-CONSOLE-POLICY-DENIAL）；
  权威：ADR-0010 §14.17（v1.17）。
- 定案（2026-08-15 用户裁决）：放弃「直接执行面永久移除」；双模式 console 默认 +
  direct 受控降级（3 连败助理层故障面 → 无工具询问轮 → 切换留痕；计划门约束
   console 订单，direct 为有记录的例外，`console_step_done` 需证据置 done）；
  结构化策略拒绝（P1-2）为 S3 前置。
- 阶段：A（模板去人格 + AGENTS.md 计划型机械包裹 + 首轮计划轮硬门）；B（注册板块=
  探针投影 + 工具栏刷新绑定黑板模型栏）；C（console 默认 + direct 受控降级）。
  明细与验收见设计 §9/§11；实施勾选见 TODO P0-C。
- 进度（2026-08-16）：**阶段 A 已闭合**——模板去人格（主/子代理/apply-patch
  模板 + ORCHESTRATOR_PROMPT_BODY，XOR 加密模板重生成，无人格关键词渲染测试）、
  AGENTS.md 计划型机械包裹（`<plan_first_framework>` 固定前缀，用户内容之前）、
  首轮计划轮硬门（`plan_write` v0.2 事件 + 结构化校验/一次重填/降级留痕 +
  黑板 plan epoch 落板；首轮工具面=blackboard_read+plan_write，其余机械拒绝；
  会话级门，ACP server/CLI run 生产接线）；实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md`。
- 进度（2026-08-16 审查收口）：计划轮不消耗 tool-round 预算（用户裁决）、
  compaction whitelist 窗口顺延至计划落板后首个执行轮、结构上限定稿
  （步骤 ≤32/动作 ≤8/证据 ≤16/总量 ≤32K，参照 AutoGPT/oh-my-loop/joyagent/
  编排工具/LangChain 成熟设计）、AGENTS.md 包裹改为系统提示词层无条件注入
  （canonical 常量入 orz-assurance，不依赖 AGENTS.md 存在）、P1 工具事件
  契约形状收敛、plan_write 面随开关收敛（子代理/grill 剔除）、P3 全部闭环
  （TUI 降级投影/同轮单次/参数 schema/上限/persona 测试）；ADR-0010
  §14.17⑯、审计 §7 登记。
- 进度（2026-08-16）：**阶段 B 已闭合**——注册板块=探针投影收口：controller
  单一探针源（`console_probe_source`，仅内存/随轮覆盖、run 起始复位）→
  `sync_console_registrations` 为派生唯一路径（Profile/Bundle ∩ 探针完整集），
  无探针轮次不再 bundle-only 刷新、沿用上一轮内容（移除静态基础集中间态）；
  工具栏刷新绑定黑板模型栏：`blackboard_read section=actions` 读取时由最近
  探针源派生并持久化（live 视图），归档 epoch 读保持快照（测试锁定）；工具
  投影与注册板块共用同一探针源（同源一致性测试锁定；审查收口：run 起始复位
  与归档读不派生单测）。实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md`。
- 进度（2026-08-16）：**阶段 C 已闭合**——console 默认 + direct 受控降级
  双模式：v0.2 事件 +2（`console_mode_transition` / `console_order_written`，
  action_write ToolCompleted 收敛通用形状）、双模式状态机（故障连败/询问轮/
  switch/stay/return）、步骤状态机（`pending → in_progress → done|failed`、
   步骤门 step_not_done、`console_step_done` 证据门）、模型面收敛（console 面=
  黑板读写+只读核查；direct 恢复工作工具投影并全链路盖章）；实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md`。

### 3b. ORZ-COMPACTION-REDESIGN（`implemented`；P0，S1-S4 已闭合 2026-08-14）

- 入口：[设计](CONTEXT_COMPACTION_DESIGN_2026-08-14.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；
  权威：ADR-0010 §3.6 / §14.10 / §14.14（v1.10 + v1.14）；实施审计：
  [GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md](audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md)。
- 决策门：2026-08-14 用户裁决定稿——有效窗口 384K、160K 普通触发 / 200K 兜底、工具调用记录每轮
  机械坍缩（零模型调用）、五段模板摘要（≤17K 字符、derived_unverified、冷却 2 模型轮、摘要链只进
  审计、滚动回查 marker）；推翻 2026-08-08 零模型摘要与仅最终答案间隙裁决；2026-08-14 用户
  放行实施；2026-08-14 审查修复裁决（v1.14）——守卫失败重试 3 次后强制压缩并报告、会话结束
  自动压缩治本、存档写失败显式重试报告、退化守卫 300 等效字符（CJK 折算 2）、黑板 edit 窗口
  滚动、冷却 3→2 模型轮。
- 实施切片：
  1. S1：D2-2 恢复超窗预估算截断 + D3-1 marker/白名单恢复保留（含测试）——**已闭合**；
  2. S2：工具调用记录机械坍缩（动作台账行 + 配对纪律 + 指针完整性 + 测试）——**已闭合**；
  3. S3：五段模板摘要接线（退化守卫 300 等效字符门、会话模型覆盖、17K 校验、重做/终止态、
     `context_compressed` 事件 Schema v0.2 + verifier/fixtures）——**已闭合**；
  4. S4：实施审计、ADR-0010 §14.10 补写、索引/BACKLOG/TODO 同步、设计文档状态更新——**已闭合**。
  5. S5：审查修复（守卫重试/强制压缩 + `guard_failed`、会话结束压缩 `session_end`、
     存档写失败 `archive_write_failed`、黑板窗口滚动、120s 超时、Top-40、契约扩展）——
     **已闭合**。
- 进度（2026-08-14）：S1-S5 全部闭合——恢复预检（`context_recovery_truncated` 事件 +
  完整侧车审计副本）、marker/白名单恢复保留、动作台账请求视图坍缩、五段模板摘要
  （160K/200K/2 模型轮/5K/0.6、重做 ≤3、summary_incomplete 终止态 + fallback 机械截断、
  `.gsa/compaction/` 存档 + 7 天 retention、TUI 投影、守卫强制报告、会话结束压缩、
  存档写失败显式报告、黑板窗口滚动）；orz-loop 305 / orz-host 208 / orz-tui 178 通过，
  工作区编译 exit 0，Python 事件校验 171 通过，仓库门禁 valid。
- v1.15 注（2026-08-14 用户裁决，实施未开始）：其中「黑板 edit 窗口随压缩滚动」机制
  已废止，黑板生命周期改按 plan epoch 轮换（见 6e）；压缩机制其余部分不变。

## P1 — 可并行审计 / 证据

### 4. FUS-COMPONENT-REGISTER（`partial`）

- 开放内容：65 组件全 `audit_required`，逐 crate 采用审计未开始（V11-IMPL-008）；不得从 crate 名/编译推断采用档位。
- 入口：[register](../upstream/fusion-component-register-v0.1.yaml)；[V1.1 复核](audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。

### 5. GAP-WINDOWS-EVIDENCE（`partial`）

- 开放内容：ORZ-WIN-PROC-001/002/003 仍为 `candidate`；需真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
- 入口：[incidents](incidents/windows/README.md)；[cases](cases/windows/README.md)。

### 6. IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- 开放内容：transport/retry/thinking 按主/子代理同构约束复核——**已闭合（2026-08-16）**：
  三实例共享同一 `DeepSeekTransport`（ModelConfig/RetryPolicy/thinking 单一来源、
  `REQUEST_MAX_TOKENS=160_000` 单一常量；请求级覆盖仅 `-p` 预检轮，文档化 F-07），
  证据与边界见变更记录。仍开放：DeepSeek live 通道与 Windows 实机晋级证据
  （ADR-0010 §11.7；当前仍为 offline / 构建时 evidence）。
- 入口：[DEEPSEEK_ADAPTER_CONTRACT](../architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)；[ADR-0007](../adr/ADR-0007-transport-retry-policy.md)；[ADR-0006](../adr/ADR-0006-credential-target-registry.md)。

### 6b. ORZ-CACHE-CONTEXT-COST（`approved`；P1，2026-08-14 登记）

- 定位：缓存与上下文成本收敛——保持 v1.8 探针可见性机制不动（工具集变化=真实状态
  变化，接受前缀 miss，第二轮自动恢复）；否决 per-window 探测 / 预热轮 / 工具层后置
  渲染 / 工具层 1K 压缩；短期仅 DeepSeek OpenAI 兼容面。
- 三项待实施：
  1. 请求 header 变化留痕——模型请求 header（system+tools 摘要 + config + 原因
     initial/change）变化时 journal 留痕；翻转可审计、miss 可归属。
  2. 探针准确性与稳定性——误判审计（假完整/假不完整）、翻转与 header 留痕事后核对、
     可选后端接线同步补翻转测试。
  3. 单轮工具结果注入预算 + 策略化读取——`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 默认
     50K、按模型轮累计、超限拒批并提示 offset 续读；提示词 grep/结构优先、证据关键
     文件才全文。
- 入口：[ADR-0010 §14.9](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [探针设计](TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md)；[TODO](../TODO.md)。
- 2026-08-15 实施闭合：① 请求 header 变化留痕——新增 v0.2
  `request_header_change` 事件（system+tools+config 三摘要 + `agent_role` +
  initial/change + `previous_header_sha256`；Schema/verifier/fixtures/TUI 同步）；
  ② 探针准确性——验证器翻转↔header 交叉核对 + `assurance/probe_accuracy_audit.py`
  （假完整/假不完整候选、门禁类 error 不计误判、旧 journal 兼容边界）；
  ③ 单轮注入预算——`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 默认 50K、按模型轮累计、
  超限无 ToolStarted 拒批 + `inject_tokens_used/budget` 字段 + offset/grep 提示 +
  提示词读取纪律；验证 orz-loop 337 / orz-assurance 151 / orz-tui 178 /
  orz-bin 全绿，Python 1888+14 skipped，仓库门禁 valid。审计：
  `docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md`。
- 2026-08-15 二次全面审查修复：payload 增 `change_kind`（机械「变化原因」，
  Schema/verifier 校验摘要差一致）；verifier 允许每车道多链 initial（子代理
  多 activation/主车道多 run 合法）；新增 `_verify_v02_inject_budget`；
  辅助模型请求（压缩摘要/预检）留痕边界登记；预算计数口径注记。详见审计 §7。

### 6c. ORZ-ORIENTATION-FORCED-TEMPLATE（`implemented`；P1，2026-08-15 闭合）

- 定位：中立问询升级为强制模板轮——触发点下一安全动作间隙明确暂停，模型填写问询
  模板后才恢复动作（Orientation 与 DC 两族共用，主车道）；目的=拉回注意力、防跑偏
  与钻牛角尖（强制表达、不验证诚实）；缓解必做（`progress_evidence` 存在性交叉
  校验、`gather_evidence` 必填缺失面）。
- 实施前置：ADR-0010 §4.2 正文修订（v1.13 已登记裁决）；事件/Schema/verifier/
  fixtures；测试；实施审计与索引同步。
- 2026-08-15 实施闭合：ADR-0010 §4.2 正文修订（v1.16）——「注入」→「触发点暂停并
  填写模板」；强制模板轮实现（checkpoint 轮无工具、模板字段/机械校验、一次错误反馈
  重填 + 降级兜底）；缓解必做（`progress_evidence` 与 journal 证据身份存在性交叉
  校验、`gather_evidence` 必填缺失面）；v0.2 `checkpoint_response` 事件
  （Schema/verifier/fixtures/TUI/枚举同步）；测试覆盖触发→暂停→填表→恢复、重填、
  降级、checkpoint 轮工具拒绝、主车道隔离与计数语义。实施审计见
  `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。
  2026-08-15 二次全面审查后复核修复：DC fire 增可选 `agent_role=main`
  （Schema/fixtures/生成器同步），验证器兼容历史 fire 并新增 outcome↔validation
  与 gather_evidence 条件交叉；响应角色收紧主车道；Rust 长度按 trim 后值；
  conformance 计数更名；orz-loop 333 / Python runtime 264 通过。
- 入口：[设计](ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.13](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [TODO](../TODO.md)。

### 6d. ORZ-SESSION-CONTEXT-MONITOR（`approved`；P1，2026-08-14 登记；2026-08-16 度量重定）

- 定位：会话压缩次数监测——度量=会话内压缩次数（`context_compressed`
  reason∈rhythm/fallback 计数、session_end 不计、一次一计）；≥2 次机械提醒、
  ≥3 次机械总结推荐（可配，默认待校准；2≈旧 384K、3≈旧 500K）；最简实现=阈值到达的
  最后一轮模型输出末尾机械附言（附次数）；headless 仅日志；TUI/journal 事件为
  beta 前可选；与压缩独立。
- 实施前置：压缩事件计数接线、阈值配置、机械附言注入点、测试、实施审计与索引同步；
  原 token 度量与 chars/2 中文估算校准项随 2026-08-16 用户裁决废止（理由：累计 token
  对模型不可见、阈值无质量边界，压缩次数为更直接的会话寿命代理）。
- 入口：[设计](SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.13](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [ADR-0010 §14.18](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [TODO](../TODO.md)。

### 6e. ORZ-BLACKBOARD-PLAN-EPOCH（`implemented`；P1，2026-08-14 实施闭合）

- 定位：黑板生命周期按 plan epoch 轮换，与压缩生命周期解耦——plan 区为单写者复写区
  （`plan_id`/`plan_epoch`）；仅新 plan epoch 批准触发原子轮换（归档旧 epoch 快照 →
  清 edits/tool_actions/exec 工作区并复写 plan → 写新 plan）；gate_log、白名单与
  检索分区不清；压缩不再清黑板（废止 v1.14「黑板 edit 窗口随压缩滚动」）；路径槽=
  本 epoch 增量（Top-40 + 5K + 指针）；marker 带 `plan_epoch`；`blackboard_read`
  跨 epoch 走归档。
- 决策依据：长任务压缩频繁，黑板随压缩擦除会破坏任务工作状态并动摇中立问询的
  task_position/计划锚点（2026-08-14 用户裁决）。
- 进度（2026-08-14）：S1-S5 全部闭合——plan 批准事件携带 `plan_epoch`（Schema/
  fixtures 同步）；`with_plan` 带 `plan_id`+`plan_epoch` 身份，同 plan_id 修订不清板、
  新 plan_id 原子轮换（归档旧 epoch → 清 edits/tool_actions/exec → 复写 plan）；
  `.gsa/blackboard/epoch-<n>.json` 快照（批准/修订持久化当前 epoch，轮换归档旧 epoch；
  写入有界重试、失败 warn 不阻断）；`blackboard_read` 增 `epoch` 参数跨 epoch 回查
  （缺失显式提示，不静默回退）；压缩不再清黑板（`context_compressed` marker 携带
  `plan_epoch`，路径槽溢出指针指向 epoch 快照）；archive dir 构造时装载最新 epoch
  快照（恢复入口）；retention 覆盖 `.gsa/blackboard` 7 天清扫。验证：orz-loop 310 /
  orz-host 209 / orz-tui 178 / orz-bin 全部通过；Python runtime journal 校验 157 +
  conformance 14 通过；仓库门禁 valid、0 错误。
- 补强（2026-08-15，全面复查 F1/F3，用户裁决）：`plan_epoch` 改为时间戳单调编号
  （unix 毫秒为基底，`next = max(now_ms, 磁盘现存 max + 1)`）——清扫后编号不复用、
  marker/`blackboard_read epoch` 引用跨窗口唯一；身份不变式强制（一一对应）：同
  plan_id 必须沿用同 plan_epoch、新 plan_id 必须严格大于当前 plan_epoch，
  `rotate_to_plan`/`try_with_plan` 返回错误、`with_plan` fail-fast、拒绝先于任何
  黑板变更；retention 对 `.gsa/blackboard` 清扫保留最高编号快照（恢复入口）。
  验证：orz-loop 312 / orz-host 210 / orz-tui 178 / orz-bin 全部通过；Python
  runtime 251 通过；仓库门禁 valid、0 错误。
- 复查遗留处理（2026-08-15 明确记录并全部闭合，v1.15⑨；来源：全面复查问题清单，
  F1/F3 已由 v1.15⑧ 补强处理，F8 随补强顺带修复）：
  - F2（P2，已处理）：epoch 归档写盘改为临时文件+自检解析+rename 原子提交，
    崩溃只可能留下 `.tmp`；`latest_epoch_snapshot` 从高到低回退到第一个可解析
    快照，半截文件不再阻断恢复。
  - F4（P3，已处理）：epoch 分配增加 `.claim-<n>` 原子占号（`create_new`），
    同毫秒并发进程只有一个能赢得编号，碰撞方递增重试；claim 计入下次分配扫描，
    崩溃不导致复用，retention 按年龄清扫过期 claim。
  - F5（P3，已处理）：路径槽溢出指针改从 controller 配置的归档目录单一来源
    取路径，不再由 `session_cwd/.gsa/blackboard` 重算，自定义归档目录不失真。
  - F6（P3，已处理）：`blackboard_read` 的 `epoch` 参数区分“缺失”与“非法”，
    0/负数/浮点等非法值显式报错（exit_code=1），不静默回退 live 视图。
  - F7（P3，已处理）：旧 epoch 归档写失败并入事件面——新增 v0.2
    `epoch_archive_write_failed`（rotated/current + attempts），builder 阶段
    排队、run 启动随 journal 写入，schema/fixtures/TUI 同步。
  - F9（P4，已处理）：`EpochSnapshot.rotated_at` 更名 `persisted_at`
    （serde alias 兼容旧归档），语义与实现一致。
  - F10（P4，已处理）：设计文档 §5 恢复措辞对齐 ADR ⑦/⑧（archive dir
    装载最新 epoch 快照）。
  - 验证（2026-08-15）：orz-assurance 151 / orz-loop 317 / orz-host 211 /
    orz-tui 178 / orz-bin 全部通过；Python runtime + assurance 1858 通过、
    14 skipped；仓库门禁 valid、0 错误。设计/ADR/BACKLOG/TODO/审计已同步。
- 入口：[设计](BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.15](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [实施审计](audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md)；
  [TODO](../TODO.md)。

### 6f. ORZ-LARGE-FILE-READ-CONTRACT（`implemented`；P1，2026-08-17 设计定案，同日实施闭合）

- 定位：大文件读取工具契约有界返回——读取超过粗门（默认 16KB、可配 8–32KB、
  env/TOML 口子）的文件返回读取句柄信封（path / size / encoding /
  content_sha256 / 可用范围 / 有界预览 ≤2–4KB / truncated / offset 续读指针），
  不返回全文；小文件保持全文一次返回（一次往返）；精门=单轮注入预算
  （默认 50K、`ORZ_MAX_INJECT_TOKENS_PER_ROUND`）为最终兜底。
- 决策依据：全文返回迫使模型全量接受（最坏情况中文 128KB ≈ 40K+ token ≈ 预算
  80–90%，仍撞注入层事后拒批）；语义适配留模型、助理层只提供机械原语；注入层
  事后拒批前移为契约层事先有界返回；黑板承担内容会膨胀 epoch 快照并产生过期
  副本（2026-08-17 用户裁决定稿）。
- 状态：**已实施（2026-08-17 本窗口闭合）**——GrokBuild `read_file` 文本路径
  超过粗门（默认 16KB、`ORZ_READ_FILE_COARSE_GATE_BYTES` 8–32KB，env/TOML 口子
  =`ReadFileParams.coarse_gate_bytes`）返回读取句柄信封（path/size/encoding/
  content_sha256/available_range/有界预览 ≤4KB/truncated/offset 续读指针）而非
  全文；小文件全文一次返回；信封 terminal-only；单行超长预算内截断并报 truncated；
  SKILL.md/`skills` 路径保持全量豁免；PDF/PPTX/图片不变；`FileTooLarge` 文本路径
  被取代保留为防御兜底。模型面契约提示（提示词读取纪律 + console 注册表描述 +
  工具描述）已同步；黑板/结果栏经信封只承载有界预览+指针（内容本体不上黑板）。
  测试：orz-tools read_file 199 / output 信封序列化 / orz-loop 440 / orz-host
  read_file e2e 2；clippy 无新增可归因告警；orz 子模块 172b14e；实施审计见
  `docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md`。
  **2026-08-17 全面检查修复（本窗口）**：P2-1 空窗口/越界 offset 语义
  （past-EOF truncated=false/offset=None、范围内空窗口 offset=start_line、
  最后一行行内截断 offset=None，渲染报实际行数）；P3-1 `[toolset.read_file]`
  配置节端到端接线（coarse_gate_bytes 经 orz-config 分层装载注入工具参数，
  优先于 env；AgentBuilder `with_read_file_params` 通路）；P3-2 concise 描述
  同步；P3-3 envelope 不追加 cursor rules 边界登记；P3-4 提示词措辞精确化。
  测试 orz-tools read_file 201 / output 84 / orz-agent 1 / orz-loop 440 /
  orz-host read_file e2e 5；clippy 无新增可归因告警；orz 7c4a99e + bd8d485；
  manifest 1401、门禁 valid。
- 入口：[设计 §11](CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) /
  [黑板设计 §4](PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) /
  [ADR-0010 §14.22](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) /
  [TODO](../TODO.md)。

### 6g. FUS-LEDGER-FOLD-STATE（`implemented`；P1，2026-08-18 设计定案，同日实施闭合）

- 定位：动作台账折叠从「每请求无状态重算」改为「折叠点状态化」——controller
  会话级持有 `fold_start`/`fold_cut`/`folded_ledger` 三态，请求视图 = preamble
  + 冻结台账 + `messages[fold_cut..]`（纯追加），推进之间前缀字节级稳定；
  折叠推进改低频机械触发（视图估算 ≥ `ORZ_FOLD_TRIGGER_TOKENS` 默认 128K，
  2026-08-18 定案；工具轮间隙执行）；压缩执行时旧台账归档进摘要存档、fold
  三态重置、摘要输入与主请求同源；恢复后 fold=None 重新累积（运行期状态
  不持久化）。
- 决策依据：DeepSeek 涨价后命中率目标 ≥90%；实测 67.4%（控制台
  8,845,056/4,279,939）；根因=`build_collapsed_request` 每轮滑动重写前缀
  （复刻模拟首次折叠重合率 1.5%）；对照 dsh「严格追加 + 显式 replace」纪律。
  **参数定案（2026-08-18 用户裁决，统一参数、不做跑分特化）**：折叠推进
  128K（MRCR 平台期边界 Flash 0.870；本仓库多文档读取 index 30.5K/ADR 43K/
  BACKLOG 27.8K 全量 ≈101K，允许连续读完关键文档集）＋压缩普通触发 160K→
  192K（Flash ≈0.81、压缩周期 ≈71 轮）＋兜底 200K→256K（Flash ≈0.76，
  超线即强制压缩）。384K 为 prompt 维度质量线，192K/256K 直接比较 <384K
  成立；旧「224K=384K−160K」「352K 缓冲」推导作废。384K 有效窗口出处=
  DeepSeek V4 技术报告 arXiv:2606.19348 Figure 9（MRCR-8-needle/Average
  MMR，SVG 逐点读取：Flash-Max 8K=0.910/16K=0.840/32K=0.870/64K=0.850/
  128K=0.870/256K=0.760/512K=0.600/1M=0.490，Pro-Max 对应 0.900/0.850/
  0.940/0.900/0.920/0.820/0.660/0.590；128K→256K 为下滑最快区段）；Max 档
  官方评估窗口 384K（论文 §5.3.1）；命中率估算 ≈95.5%（现状 72%、实测
  67.4%）、成本约现状 1/4。
- 状态：**已实施（2026-08-18 本窗口闭合；orz a5bea77）**——S1 fold 三态 +
  `build_request_view` + `advance_fold`（action_ledger.rs）；S2 loop-top 推进
  触发（视图估算 ≥128K、checkpoint 轮优先、零模型调用）；S3 压缩联动
  （摘要输入与主请求同源、drain 保留起点基于 fold_cut、压缩后三态重置）+
  摘要同源 + 恢复（每轮循环实例局部、恢复后 None 重新累积）；S4 参数接线
  （默认 128K + `ORZ_FOLD_TRIGGER_TOKENS`；压缩普通触发 160K→192K、兜底
  200K→256K）+ 测试（action_ledger 5 项 + orz-loop 循环级 2 项；orz-loop
  450 / orz-assurance / orz-tui 178 / orz-bin 全量通过、clippy 无新增告警、
  manifest 1401、仓库门禁 valid）+ 文档同步 + 实施审计
  `docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md`。
  实施状态偏离设计字面「controller 会话级字段」=每轮循环实例局部（随
  LoopOutcome 返回）：同一 controller 被主车道与嵌套检索子代理共用，共享
  字段会被子代理调度污染；语义等价（每次循环起始 None 重新累积）。未闭合
  计数不变（该设计轮按「实施前登记」口径未计入未闭合总数）。
  **2026-08-18 二次全面审查收口（orz 5274b39）**：① 设计 §3.5 第 1 步
  归档缺口补实现（冻结台账进摘要存档）；② 新增 v0.2 事件
  `ledger_fold_advance`（每窗口一次前缀重写可归因；Schema/verifier/
  fixtures/TUI 全链）；③ 折叠完整性回退覆盖全部被折叠轮；④ 死代码清理；
  ⑤ 压缩联动测试修正（首个 rhythm 走成功路径、两次归档断言）；⑥ 口径/文档
  修正（session_end 显式区分、审计计数 450→452）；⑦ fixture 生成器回填
  console 三事件与身份覆盖。验证：orz-loop 452 / orz-tui 178 / orz-assurance
  152、Python conformance 15 + journal validation 214 通过、clippy 无新增
  可归因告警、manifest 1401、仓库门禁 valid。详见实施审计 §6。
- 入口：[设计文档](LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md) /
  [ADR-0010 §14.26](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) /
  [TODO](../TODO.md)。

## P2 — 生产化决策门

### 7. IMPL-CONTROL-FABRIC（`partial`）

- 决策门：**2026-08-15 用户裁决 fail-closed 生产启用放行；2026-08-16
  翻转执行已闭合**——fail-closed 改为默认（未设置即强制；显式
  `0|false|no|off` 影子；非法值 exit 2），CLI run / ACP stdio / TUI 三个
  生产入口全部接线（ACP/TUI 此前未挂签名器客户端），核查清单 ⑦⑨⑩⑪ 收口
  （⑦ web_search 显式排除走 provider 原生搜索；⑨ host 稳定面不补绑定；
  ⑩ URL gate 与票据摘要规范化等价；⑪ 重定向逐跳 URL gate 覆盖），新增
  `orz-acaf-provision` 供应工具 + `scripts/orz_acaf_run.ps1` 启动链；审计见
  [`GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md`](audits/GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md)。
- Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
- Slice 4：Windows Sandbox backend（D-11）。
- 可选：conformance capture 票据场景；normalize_lexical 单源化（检索车道
  activation 绑定与 ACP 会话接线已随 Slice 2 / fail-closed 翻转完成，
  2026-08-16 收口）。
- 入口：[ADR-0011](../adr/ADR-0011-authenticated-control-and-action-fabric.md)；[fail-closed 审计](audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。

### 8. OPS-PROTOCOL（`pending`）

- 开放内容：v0.1 协议、Schema、Python/PowerShell 执行器已就位；生产接线待裁决（先验票，再由协议执行器执行）。
- 审查判定（2026-08-13，用户无异议）：平行执行层过重，不按原样生产接线。收敛方向——保留“删除安全”（回收站 + 缓存机械分类 + 容量 fail-closed）为 host-owned 工具；跨环境桥接保留为内部执行能力，不向模型暴露 op 信封；双执行器收敛为单一参考实现，生产走 Rust 工具面。裁剪设计待产出后登记。
- 入口：[协议](../protocol/structured-operation-protocol-v0.1.md)。

## P3 — 收尾 / 清理

### 9. EVIDENCE-LOCAL-BROWSER（`partial`）

- 开放内容：Python 路径（retrieval_workflow / evidence_store / pdf_evidence）按 ADR-0010 重新符合性审查或退役。
- 入口：[LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE](../存档/architecture/pre-adr-0010/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md)；[GAP_PDF_EVIDENCE_IMPL_AUDIT](audits/GAP_PDF_EVIDENCE_IMPL_AUDIT_2026-08-11.md)。

### 10. GATE-CHAIN（`partial`）

- 开放内容：分层 gate 链与融合 runtime 的最终接线随各切片审计复核（不单开大项）。
- 入口：[assurance](../assurance/)；[orz-assurance](../orz/crates/orz-assurance/)。

### 11. 遗留小项

- DC 硬信号 4/6：`same_module_no_evidence` / `key_surface_unexamined` 接线（建议并入 P0/检索机械控制批次）。
- prompt observed-scope 枚举补列（可选优化，P0-B 步骤 6 复核观察登记）：主提示词/检索提示词未列出合法 scope 枚举（`full_text_observed` / `partial_text_observed` / `metadata_only`），模型可能先踩一次 verifier 拒绝（`url_missing_observed_scope`）再修正；verifier 机械兜底已覆盖，暂不实施。
- V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
- V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留检查——复核并登记闭合或转 gap。
- orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`，顺序/负载相关、与本批无关）——复核并登记闭合或转 gap。
- 工作区收尾：见 P0 前置收尾。

## 条件触发（不占当前优先级）

- ORZ-RECOVERY-TOOL-OUTCOME：崩溃恢复工具结果词汇——恢复中断轮次补
  `TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN` 合成 Tool 消息 + "只重试只读/幂等
  操作、验证副作用或询问"指引。出现恢复面 400 或副作用未知证据时实施（单点修复，
  不建子系统）。入口：[调研附录候选 1](DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md)。
- ORZ-STAGNATION-TOOL-SIGNAL：停滞守卫补「同工具同参数」信号——出现「同参循环且
  输出持续变化」的具体证据时，在 stagnation guard 内加最小计数信号（同一工具连续
  N 次调用），不复刻 reminder 链。入口：[调研附录候选 2](DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md)。

## 变更记录

- 2026-08-18：FUS-TOOL-SCOPE-CONTRACT 后续两项闭合登记（P0-E 收尾；本窗口
  实施）——① list_dir 目录信封：`ListDirContent` 增 listed/ignored/truncated
  机械计数（ignored=未过滤走−可见走、同过滤语义、SCOPE_COUNT_CAP=200K 封顶；
  truncated=可见总数−实际渲染）+ 卡片尾部 `(scope: ...)` 脚注；legacy/codex
  面保持 None 不报；② grep 搜索信封：`files_searched` 机械来源收敛为 v1
  `rg --files` 探针、每次完成搜索都运行（含命中，摘要行内嵌 searched N files；
  错误路径 None）——`--stats` 跨版本位置差异污染流式面、`--json` 需重写输出
  契约，均不采用。orz 子模块 614bb3b；grep 99 / list_dir 60 / orz-tools
  全量 2761 通过、clippy 无新增告警；ADR-0010 §14.23 / 索引 / TODO P0-E /
  实施审计 / 操作台设计 §12 / 黑板设计 §4 同步。未闭合 29 → 27 项（P0-E
  后续项全部闭合）。
- 2026-08-18：FUS-LEDGER-FOLD-STATE 实施闭合登记（本窗口；用户指示实施）——
  S1 fold 三态 + 有状态视图 + 推进（action_ledger.rs）；S2 loop-top 推进触发
  （视图估算 ≥128K、checkpoint 轮优先）；S3 压缩联动（摘要输入同源、drain
  保留起点基于 fold_cut、压缩后三态重置）+ 恢复语义；S4 参数接线
  （`ORZ_FOLD_TRIGGER_TOKENS` 默认 128K；压缩普通触发 160K→192K、兜底
  200K→256K）+ 测试 + 文档同步 + 审计。orz 子模块 a5bea77；orz-loop 450 /
  orz-assurance / orz-tui 178 / orz-bin 全量通过、clippy 无新增告警、manifest
  1401、仓库门禁 valid；实施审计
  `docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md`。未闭合计数
  不变（设计轮按「实施前登记」口径未计入未闭合总数）。ADR-0010 §14.26 /
  BACKLOG 6g / TODO P1 / 索引同步。
- 2026-08-18：FUS-LEDGER-FOLD-STATE 二次全面审查收口登记（本窗口；用户指示
  处理全部审查发现）——① 设计 §3.5 第 1 步归档缺口补实现（冻结台账进摘要
  存档）；② 新增 v0.2 事件 `ledger_fold_advance`（推进留痕，Schema/verifier/
  fixtures/TUI 全链）；③ 折叠完整性回退覆盖全部被折叠轮（中途不完整轮不再
  被折叠）；④ 死代码 `collapsed_round_count` 删除；⑤ 压缩联动测试修正（脚本
  prompt_tokens 与视图量级一致，首个 rhythm 触发走成功路径，两次归档均含
  冻结台账）；⑥ 口径/文档修正（session_end 全量估算显式区分、设计 §3.1 补
  per-loop local 注记、审计计数 450→452）；⑦ fixture 生成器回填 console
  三事件与身份覆盖。orz 子模块 5274b39；orz-loop 452 / orz-tui 178 /
  orz-assurance 152、Python conformance 15 + journal validation 214 通过、
  clippy 无新增可归因告警、manifest 1401、仓库门禁 valid；ADR-0010 §14.26 /
  BACKLOG 6g / TODO P1 / 索引 / 设计文档 / 实施审计同步。
- 2026-08-18：FUS-LEDGER-FOLD-STATE 设计定案登记（用户裁决：先设计、不实施；
  纯文档）——动作台账折叠状态化：controller 会话级
  `fold_start`/`fold_cut`/`folded_ledger` 三态，请求视图 = preamble + 冻结台账
  + `messages[fold_cut..]`（纯追加），推进之间前缀字节级稳定；推进 = 视图估算
  ≥ `ORZ_FOLD_TRIGGER_TOKENS`（默认 128K，2026-08-18 定案）的机械低频触发；
  压缩时旧台账归档 + fold 重置 + 摘要输入同源；恢复后重新累积。**参数定案
  （用户裁决，统一参数、不做跑分特化）**：压缩普通触发 160K→192K、兜底
  200K→256K；384K 为 prompt 维度质量线直接比较（192K/256K < 384K），旧
  「224K=384K−160K」「352K 缓冲」推导作废。384K 出处复核=DeepSeek V4 技术
  报告 arXiv:2606.19348 Figure 9（MRCR-8-needle/Average MMR，SVG 逐点读取：
  Flash-Max 8K=0.910/16K=0.840/32K=0.870/64K=0.850/128K=0.870/256K=0.760/
  512K=0.600/1M=0.490，Pro-Max 0.900/0.850/0.940/0.900/0.920/0.820/0.660/
  0.590；128K→256K 为下滑最快区段）；Max 档官方评估窗口 384K（论文 §5.3.1），
  V4 输入上限 1M。命中率估算 ≈95.5%（现状 72%、实测 67.4%）、成本约现状
  1/4。ADR-0010 v1.26/§14.26、BACKLOG 6g、索引同步；设计文档
  `docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md`。未闭合计数不变。
- 2026-08-17：ORZ-LARGE-FILE-READ-CONTRACT 实施闭合（本窗口）——GrokBuild
  `read_file` 文本路径有界返回：超过粗门（默认 16KB、可配 8–32KB）返回读取句柄
  信封（path/size/encoding/content_sha256/available_range/有界预览 ≤4KB/
  truncated/offset）而非全文；小文件全文一次返回；信封 terminal-only；单行超长
  预算内截断（offset 指向下一行）；SKILL.md/`skills` 全量豁免；`FileTooLarge`
  文本路径被取代保留为防御兜底。模型面提示（提示词/工具描述/console 注册表）同步。
  测试 orz-tools read_file 199 / orz-loop 440 / orz-host read_file e2e 2；
  clippy 无新增可归因告警；orz 子模块 172b14e；manifest 1401、仓库门禁 valid；
  ADR-0010 §14.22 / 操作台设计 §11 / 黑板设计 §4 / TODO P1 / 索引同步。
  未闭合计数：该组 4 项此前未计入 P1 分组计数（口径遗漏，若计则 P1 13/总数 33）；
  本次全部闭合，P1 回 9 项、未闭合总数维持 29 项（补计与闭合相抵）。
- 2026-08-17：FUS-TOOL-SCOPE-CONTRACT 两项后续补记进计数（用户指示）——
  grep 面实施审计「边界与后续项」两条正式入账：① list_dir 补
  ignored/truncated 计数（读/搜/列三族统一）；② grep 命中路径
  `files_searched` 留空（后续 --json 面或 stats 位置收敛后再定）。
  P0-E 主项仍 0 项；未闭合 27 → 29（P0-E 后续 2 项）。BACKLOG 0a /
  TODO P0-E / 索引同步。
- 2026-08-17：大文件读取契约设计定案登记（用户裁决；纯文档、未实施）——读取
  工具契约有界返回：超过粗门（默认 16KB、可配 8–32KB）的文件返回读取句柄信封
  （path/size/encoding/content_sha256/可用范围/有界预览 ≤2–4KB/truncated/offset
  续读指针）而非全文；精门=50K 单轮注入预算兜底；小文件保持全文一次返回；语义
  适配留模型、助理层只提供机械原语；黑板/结果栏只放指针不放内容本体。ADR-0010
  v1.22/§14.22、FUS-LARGE-FILE-READ-CONTRACT `current-design`、BACKLOG 6f /
  TODO P1。未闭合计数不变。
- 2026-08-17：订单发放前拒绝入事件面项登记（用户指示处理）——冒烟重跑 ORD-000011
  （workspace.run_tests，arguments={}）写入后发放前被拒，失败只进结果栏 receipt +
  TraceStore（`consume_console_order` 未写 journal 事件），journal 无结构化拒绝记录。
  目标=发放前拒绝（order_stale / step_not_done / budget_insufficient / registry /
  contract / target / ACAF / policy / mode 门）统一入 v0.2 事件面（新增
  `console_order_rejected`，Schema/verifier/fixtures 先行）。P0-E 6 项、未闭合 33 项。
- 2026-08-17：grep 侦查纪律项登记（用户确认一并处理）——冒烟重跑中模型用不存在的
  目标字符串 grep 全树、空结果被过度泛化为「/app 无 C 源码」；工具行为正确（无匹配
  exit_code=1），属侦查策略/反馈解读问题。实施方向：提示词/计划框架侦查纪律 + 注册
  板块 grep 参数提示空结果语义 + 回归验证。P0-E 5 项、未闭合 32 项。
- 2026-08-17：grep 项复核更正登记（用户同意判断；纯文档/待办更正、无代码变更）——
  上述归因经 journal/trajectory/orz.txt 证据复核不成立：两次
  `doomgeneric_mips|frame\.bmp` grep 实际被 plan/console 门拒绝未执行（journal
  序列 24/38）；执行的 7 次 grep 全部无匹配（wall_ms 1–36ms），含 vm.js 实测存在的
  entryPoint/syscallNum/runElf 等字符串 → 系统性工具层空结果，疑搜索范围/路径解析
  异常；「工具行为正确」撤回。同轮按用户复核放弃 plan_write 特化示例方向，改为校验
  消息形状明确（错误消息写明 plan 对象形状）。P0-E 仍 3 项、未闭合仍 30 项。
- 2026-08-17：计划视图渲染步骤 ID 实施闭合——`blackboard_read section=plan`
  每步行首渲染 `step.id`（`- [状态] <step_id>: <目标> (actions: N; evidence: M)`，
  live 视图与归档 epoch 读同源）+ 系统提示词状态行当前步补 `[step_id]` +
  工具描述补取 id 提示；测试三层（epoch 渲染单测 / 工具级回达 / 跨 epoch
  归档读）；orz 子模块 0d1e01b（已推送 cli）；orz-loop 436 / orz-tui 178 /
  orz-assurance 152 / orz-bin 全量通过、clippy 无新增告警、manifest 1401、
  仓库门禁 valid。P0-E 4 项、未闭合 31 项。
- 2026-08-17：订单发放前拒绝入事件面实施闭合——v0.2 `console_order_rejected`
  （order_id / step / phase / code / reason / round / plan_epoch / run_id），
  发放前拒绝统一入事件面：phase=pre_issue（order_stale / step_not_done /
  budget_insufficient，step=protocol）+ phase=issue（registry / contract /
  target / ACAF / policy / mode 门，ACAF/模式/权限归一化 step=policy /
  code=policy_denied）；execute/verify 不入本事件（已执行订单经
  tool_started/tool_completed 留痕）。Schema/verifier/fixtures 先行（verifier
  交叉核对：拒绝须先有同 run 同 order_id 的 `console_order_written`、机械盖章
  一致、每订单至多一次拒绝、phase/step/code 一致性）；结果栏 receipt 保留为
  人类可读视图。orz 子模块 c67a452（已推送 cli；事件变体 + 三处 pre_issue
  路径 + 发放期 Err 分支 issue 路径发事件 + TUI 投影 + 测试断言）；
  orz-loop 436 / orz-tui 178 / orz-assurance 152 / orz-bin 全量通过、clippy
  与基线一致（lib 21 / lib test 26）、manifest 重生成 1401、仓库门禁 valid。
  P0-E 3 项、未闭合 30 项。
- 2026-08-17：ACAF 跑分决策登记（用户裁决）+ P0-E 下一步实施项对齐——① GAP-ACAF-
  HARNESS-PASSTHROUGH：跑分保持 ACAF 强制开启（容器内供应 manifest/keystore/signer，
  不接受影子模式），实施=适配器透传 + 容器供应 + 移除临时 `ORZ_ACAF_FAIL_CLOSED=0`
  覆盖 + 冒烟验证；② 两项 P1 观察升为实施项（plan_write 提示词/示例强化、
  `steps[].actions` 形状校验收紧）；③ 冒烟重跑定位新增步骤门模型面缺口——计划视图
  `section=plan` 不渲染步骤 `id`（`epoch.rs` 渲染仅 `[status] goal (actions; evidence)`），
  而订单 step_id 需精确绑定，模型靠猜测导致大量空转（轨迹 13/18/23 步、4 次 plan_write）；
  同时记录 grep 侦查低效（目标字符串不存在 → 空结果被泛化为「无源码」）。证据：
  `D:\tb-eval\jobs\2026-08-17__03-48-57`（reward 0.0、3 项 verifier 失败：
  test_vm_execution Timeout / test_frame_bmp_exists / _similar_to_reference FileNotFound）。
- 2026-08-17：GAP-CONSOLE-TOOLNAME-PATTERN 闭合——三个 console 面工具名点号改下划线
  （`blackboard.action_write`→`blackboard_action_write`、
  `console.step_done`→`console_step_done`、
  `console.return_to_console`→`console_return_to_console`）；同步 11 个 Rust 文件
  （91 处）+ Python verifier/schema/测试 + ADR-0010 §14.20 与设计文档；orz 子模块
  commit 0304b23、`orz_source_manifest.sha256` 重生成 1401 条目、仓库门禁 valid；
  orz-loop 434 / orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 + real_flag 2
  通过，clippy 无新增告警；Linux musl 二进制重建（rust:latest + aliyun 镜像）后
  make-doom-for-mips 冒烟重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57`——30m21s 跑满
  1740s 预算、`run_invalidated{wallclock}` 正常收尾，console 全链路（计划落板/修订/
  订单发放/上下文压缩）无 400（对比旧运行 `01-09-43` 400 即死）。GAP-ACAF-HARNESS-
  PASSTHROUGH 保持开放（正式跑分决策待定）。
- 2026-08-17：评测冒烟暴露问题登记（最优先）——正式跑分前最难错题试跑
  （make-doom-for-mips，flash + plan-first + console 默认 + headless）暴露两项阻断：
  ① GAP-ACAF-HARNESS-PASSTHROUGH：ACAF fail-closed 默认强制后 TB2 适配器不转发
  签发器配置，`orz --real` 拒启（`no signer client is configured`）；已临时以
  `ORZ_ACAF_FAIL_CLOSED=0`（影子模式）+ 适配器透传解阻，正式跑分决策待定。
  ② GAP-CONSOLE-TOOLNAME-PATTERN：console 面工具名点号违反 OpenAI 兼容工具名模式
  `^[a-zA-Z0-9_-]+$`，计划落板后下一轮 400 → run_failed（FakeProvider 不校验故单测
  未暴露）；修复=改名下划线 + 同步 verifier/schema/文档 + 重建重跑。另记录两项 P1
  观察（plan_write 字符串序列化→提示词强化；actions 空串→校验偏宽）。证据：
  `D:\tb-eval\jobs\2026-08-17__01-05-31` / `01-07-59` / `01-09-43`、
  `D:\tb-eval\gsa-volumes\b4-900s\a5da4937-2775-47ff-8202-98bf03a74780\runs\RUN-CLI-6a81eeed\events.jsonl`；
  本轮已重建 Linux 评测二进制（旧版备份 `orz-20260812.bak`）。处理窗口=新 Codex 窗口，
  详见 P0 0a。
- 2026-08-16：IMPL-DEEPSEEK-TRANSPORT 主/子代理同构复核闭合登记——
  transport/retry/thinking 三实例同构成立（`AgentLoopController::with_gateway`
  单 gateway 三实例克隆，controller.rs 2056-2064；`DeepSeekTransport::deepseek_v4`
  单 ModelConfig，transport.rs 92-107；`RetryPolicy::default()`，model.rs 47-70；
  `REQUEST_MAX_TOKENS=160_000` 单一常量，agent_loop.rs 49/1146-1148；主/子代理请求
  均 `thinking: None` → transport 默认 EnabledMax，agents/main.rs 50、
  agents/retrieval.rs 84；唯一请求级覆盖=`-p` plan gate，main.rs 630，F-07
  文档化例外；全仓无 per-agent 模型/重试 env）。边界登记：① 压缩摘要轮
  （SUMMARY_MAX_TOKENS=12_000）与 `-p` 预检轮为 loop 外辅助请求，header 留痕
  明确排除，非同构范畴；② DEEPSEEK_ADAPTER_CONTRACT §2.1 旧别名拒绝与 /models
  能力预检、§2.5 首事件语义未在 production transport 实现，属契约符合性另行
  跟踪。跑分决定（用户）：启用 plan-first（用作该设计的可使用性验证）；
  模型使用 `deepseek-v4-flash`（= 生产默认 `MAIN_AGENT_MODEL`，无需
  `ORZ_MAIN_AGENT_MODEL` 覆盖）。
- 2026-08-16：FUS-SESSION-CONTEXT-MONITOR 度量重定（用户裁决）——度量由会话累计
  模型可见输入 token（384K/500K）改为会话内压缩次数（`context_compressed`
  reason∈rhythm/fallback 计数、session_end 不计、一次一计）；阈值改次数制
  （≥2 提醒 / ≥3 推荐，可配、默认待校准）；chars/2 中文估算校准项废止；设计文档、
  ADR-0010 §14.18、索引、TODO 同步；实施未动（仍 pending）。
- 2026-08-15：推送惯例恢复登记——用户说明此前「Rust 修复不入 git、
  用户手动推送」惯例源于分类器类故障（已修复），恢复正常推送；父仓库
  main（`2ef2aa7`）与 orz 子模块分支 `feat/fusion-architecture`
  （`a0c9ffc`）均已推送远端。
- 2026-08-16：P0-C S3 全面审查收口登记——三层审查（设计/实现/符合性）
  后用户逐项裁决：单订单步数上限 20→8（`MAX_SCRIPT_STEPS_PER_ORDER`）；
  `assistant.trace` 查无 id 定案 `step=execute`+`not_found`；checkpoint 轮
  跳过注册板块刷新（`pending_checkpoint` 守卫）；注册不变式补齐（内部动作
  带 target 拒绝/内部动作类唯一/bundle 非空/嵌套按 kind 拒绝）；最终超限
  信封补 `script_step`+`action`；测试补齐 7 项（运行时 `$ref` 失败、形状/
  字段缺失、数组 items、tail>200、脚本内 TraceRead、9 步拒绝、注册不变式）；
  orz-loop 384 通过 / 0 失败；S4 登记项：30s 墙钟=总墙钟+单步受控（截止
  时间下沉 host 层、host 层进程树收口）、脚本按实际执行步数消耗 tool-round
  预算（预检/减计/错误码/预算块反映）；详见 S3 审计 §6。
- 2026-08-16：P0-C S4 实施闭合登记——单步超时下沉 host 层
  （`LoopHost::call_tool_with_timeout`：显式覆盖 = min(覆盖, 配置预算)，
  到期 `kill_active` 进程树收口；脚本每步传剩余截止；`ToolResult.timed_out`
  结构化信号 → 直接订单 `tool_timeout`、脚本归一化 `script_timeout`+
  `script_step`）；脚本 tool-round 预算（发放前预检 `budget_insufficient`
  零执行拒绝、实际执行步数减计、下一轮预算块机械反映）；端到端测试
  （FakeProvider 完整任务会话、checkpoint 轮板块保留、超时/预算边界）；
  orz-loop 392 通过 / 0 失败；决策门材料清单齐备；详见
  `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`。
- 2026-08-16：P0-C S4 全面审查收口登记（二次）——host-owned 同步工具
  （`project_doc_index`/`browser_read`/`pdf_read`/PDF 路由 `web_fetch`）
  不经 host timeout 包装为既有边界（配置预算=经注册表执行调用的硬上限）；
  脚本层每步完成后核对 30s 总截止（超时 `script_timeout`+`script_step`
  事后 fail-closed）；预算预检先静态校验脚本（不掩盖 `unknown_service`/
  契约错误）；小样 1 结果工件补齐（`sample1_result.json`，`smoke_test.py`
  90/90 复跑）；max=1 零剩余边界测试锁定；orz-loop 396 通过 / 0 失败；
  详见 S4 审计 §8。
- 2026-08-16：P0-C S4 超时语义复核裁决（用户复核 + Codex/Grok 成熟设计
  对照）——撤销上条「30s 总墙钟含进程时间」语义：脚本每步不传收缩剩余，
  由 host 每调用超时独立约束（配置预算，默认 5 分钟，进程树收口不变）；
  `MAX_SCRIPT_WALLCLOCK_SECONDS`/deadline/事后核对删除，墙钟+字节测试更名
  `run_script_enforces_byte_limits`；orz-loop 395 通过 / 0 失败；详见 S4
  审计 §8。
- 2026-08-15：P0-C S3 前置全面审查修复（F1-F8）闭合登记——拒绝事件补
  `exit_code=1` + `status=error`（含 host 级拒绝）、verifier ACAF 家族补
  `web_fetch`/`browser_read`、permission 家族补 host 路由检索工具、新增
  生产者事件→验证器对拍测试（`PolicyDenialProducerParityTests`）、
    `orz/` 源码完整性清单（`orz_source_manifest.sha256`，1406 文件）接入
    仓库门禁，并登记为父仓库 git 子模块（SilverWhite/CLI
    `feat/fusion-architecture`，提交 a0c9ffc 已推送远端）；验证：orz-loop
    367 / acaf_e2e 21 / Python runtime 213 通过，仓库门禁 valid；详见 S3
    前置审计 §7。
- 2026-08-15：P0-C S3 前置（P1-2 结构化策略拒绝）闭合登记——`ToolResult.
  policy_denial`（source/code/reason）接线五条拒绝路径（权限/ACAF/检索
  模式门 ×3）、console 适配层退役前缀判定、ToolCompleted 增可选
  `policy_denial`（Schema/verifier/fixtures 先行 + 交叉规则）、内容碰撞
  回归；orz-loop 366 / acaf_e2e 21 / Python runtime 209 通过，仓库门禁
  valid；实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`；
  同日本切片工具事故导致 `diagnostic_coverage.rs` 生产实现重建（按测试/
  调用面契约，非逐字节恢复），详见审计 §5。
- 2026-08-15：P0-C 内嵌集成 S2 落地登记——模型面投影（`blackboard_read`
  section=actions + `blackboard_action_write` 写单按钮，pending 机械拒绝、
  round/plan_epoch/run_id 机械盖章、主车道专属）与轮末机械发放
  （post-tool-batch 安全间隙、checkpoint 优先；round/plan_epoch/run_id
  防重放与过期（`order_stale`）→ 注册表/契约 → 经 ControllerConsoleExecutor
  委托 run_host_tool（权限/ACAF/模式门/事件链）→ 响应 schema 验证 → 结果栏
  receipt + trace_id → TraceStore.commit；策略拒绝归一化 step=policy +
  policy_denied）；注册板块每轮机械刷新（基础动作集 6 项）；新增 8 项测试，
  orz-loop 364 通过 / 0 失败；审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`。
- 2026-08-15：P0-C 内嵌集成 S1 全面检查修复登记——执行器返回 ExecuteError
  （执行失败/策略拒绝，`step=policy` + `policy_denied`）；响应契约强制必填
  （注册时校验并缓存 schema，任何输出过机械验证）；`exit_code=Some(0)`
  成功契约（None/非零按执行失败）；TraceStore commit 提交语义 + 失败事件
  满员滚动保底；注册板块最小参数提示投影；S2 验收点显式化（round/epoch
  防重放、真实目标解析、ACAF 票据、策略表、step=policy）；新增 18 项测试
  （console 15 + blackboard 3），orz-loop 356 通过 / 0 失败。
- 2026-08-15：P0-C 内嵌集成 S1 落地登记——orz-loop 新增 `console` 模块
  （ServiceRegistry/ActionSpec/issue_action/信封/Trace+TraceStore，执行经
  ActionExecutor 抽象委托）与黑板动作栏数据面（`blackboard::ActionBoard`
  注册板块/动作栏单槽/结果栏有界，随 plan epoch 归档轮换）；S2 模型面投影
  + 轮末发放待续。
- 2026-08-15：P0-C 小样 2 闭合登记——编辑执行器 `workspace.search_replace` 对照实验实施并测量（固定 10 场景语料；baseline 成功率 100%/平均 1.8 轮 vs candidate 100%/平均 1.0 轮，通过标准两项满足；POC smoke 57/57），用户裁决通过、独立判定一致；DSH B 项（文件观察策略收编为 search_replace 动作契约规则）随之落地；结果工件 `prototype/classical_console/sample2_result.json`，待办路由见 TODO P0-C。
- 2026-08-15：P0-C 小样 3 闭合登记——机械组合脚本模式 `workspace.run_script`
  （线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）对照实验实施并测量
  （固定 8 场景语料；baseline 成功率 100%/共 20 轮/平均 2.5 轮 vs candidate
  100%/共 8 轮/平均 1.0 轮，通过标准三项满足；POC smoke 87/87），用户裁决通过、
  独立判定一致；结果工件 `prototype/classical_console/sample3_result.json`，
  下一步=orz 内嵌集成，待办路由见 TODO P0-C。
- 2026-08-15：P0-C 全面审查 P1 修复登记——三项 P1 处理完成：4 MiB 上限改为
  最终响应累计（含未命名步骤与 `result`，错误码 `script_response_limit`）；
  `$ref` 运行期解析失败结构化（`invalid_reference`/`step=execute`，upstream 带
  `ref` 与 `script_step`）；baseline 人工模拟边界显式写入 README、语料描述与
  结果工件 provenance（含 `evidence_boundary`）；结果工件重生成（指标不变），
  POC smoke 90/90 通过。
- 2026-08-15：ACAF fail-closed 生产启用裁决登记——用户裁决放行（P2
  IMPL-CONTROL-FABRIC 决策门）；翻转执行与核查清单 ⑦⑨⑩⑪ 收口待实施，
  TODO/索引/ADR-0011 同步。
- 2026-08-14：黑板擦除机制重设计登记（用户裁决）——ADR-0010 v1.15（§14.15 补写）、
  新设计文档 `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`、压缩设计 v1.15 注记、
  P1 6e 登记；废止「压缩成功后清空黑板 edit 窗口」机制（设计层面，2026-08-14 实施闭合，
  见 `GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md`）；
  黑板按 plan epoch 轮换（归档/清工作区/复写 plan 原子提交；gate_log/白名单/检索分区
  豁免）；路径槽=本 epoch 增量、marker 带 plan_epoch、blackboard_read 跨 epoch 走归档；
  中立问询/DC 锚点跨压缩稳定。
- 2026-08-14：P0-D 二次复查处理登记（用户要求处理复查全部问题）——设计投影 §7
  复用边界对齐（orz-compaction 500 字符门表述移除、语义等价内联）、ADR §3.6/§14.14
  补写 session_end 160K 触发阈值、代码注释与 schema 冷却残留修正、终止态 marker
  占位 digest 改显式"（未生成）"、fixtures 生成器回写 P0-B step 5 手工修订（README/
  负样例/信封时间戳/LF 行尾）、chars/2 中文低估登记为 6d 实施前置校准项；
  ADR-0010 §14.14 条目 2、审计 §8；详见
  [GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md](audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md) §8。
- 2026-08-14：P0-D 压缩机制实施闭合登记（用户放行）——S1（D2-2 恢复预检截断 +
  `context_recovery_truncated` 事件 + D3-1 marker/白名单恢复保留）、S2（动作台账机械坍缩，
  请求视图）、S3（五段模板摘要接线：160K/200K/3 轮/5K/0.6、重做 ≤3、summary_incomplete
  终止态 + fallback 机械截断、`.gsa/compaction/` 存档 + 7 天 retention、
  `context_compressed` v0.2 payload + verifier/fixtures、TUI 投影）、S4（审计 + ADR-0010
  §14.10 补写 + 索引/TODO/设计文档同步）；FUS-COMPACTION-REDESIGN 转 `implemented`；
  实施审计 `docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md`。
- 2026-08-14：DSH 借鉴复核与两项设计确认登记——ADR-0010 v1.13（中立问询强制模板轮、
  会话累计上下文监测、崩溃恢复工具结果词汇条件项、DSH 借鉴复核结论）；新增设计文档
  `ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md` 与
  `SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md`；P1 登记 6c/6d；条件触发两候选；
  索引新增 FUS-ORIENTATION-FORCED-TEMPLATE / FUS-SESSION-CONTEXT-MONITOR /
  FUS-RECOVERY-TOOL-OUTCOME / FUS-DSH-BORROW-REVIEW。
- 2026-08-15：ORZ-ORIENTATION-FORCED-TEMPLATE 实施闭合（用户指示优先）——
  ADR-0010 §4.2 正文修订（v1.16）；强制模板轮实现（checkpoint 轮无工具、模板字段/
  机械校验、一次重填 + 降级兜底、pending 单槽与 Orientation 优先、主车道/检索车道
  边界）；缓解必做（`progress_evidence` 证据身份交叉校验、`gather_evidence` 必填
  缺失面）；v0.2 `checkpoint_response` 事件（Schema/verifier/fixtures/TUI 同步）；
  测试 orz-loop 332 通过（新增触发→暂停→恢复/重填/降级/工具拒绝/主车道隔离等用例）；
  FUS-ORIENTATION-FORCED-TEMPLATE 转 `implemented`；实施审计
  `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。
- 2026-08-14：P0-B 步骤 6 全面复核补记——索引 GAP-SOURCE-WEIGHTING-IMPL
  条目补"候选 ≤5 已机械取代"注记（P3 已处理）；审查观察登记：提示词未列
  observed scope 合法枚举（P3 可选优化，暂不实施）。详见
  [GAP-RETRIEVAL-MECH 步骤 6 实施审计](audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md) §7。
- 2026-08-14：P0-B 步骤 6 闭合登记——提示词相应缩短与测试更新实施完成
  （主提示词引用纪律缩减为"标记格式 + verifier 交付前机械校验"、检索提示词
  移除候选 ≤5 软约束改指机械预算反馈、来源加权/引用规则去冗余、prompt.rs
  测试同步）；P0-B 批次 1-6 全部闭合，FUS-RETRIEVAL-MECH 转 `implemented`，
  DC 剩余两信号转 P3 独立跟踪。详见
  [GAP-RETRIEVAL-MECH 步骤 6 实施审计](audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md)。
- 2026-08-14：压缩机制重设计定稿登记——ADR-0010 v1.10（§3.6 修订 + §14.10 裁决索引）、
  设计文档 `CONTEXT_COMPACTION_DESIGN_2026-08-14.md`、索引 FUS-COMPACTION-REDESIGN pending；
  参数=384K 有效窗口 / 160K 触发 / 200K 兜底 / 五段模板 17K / 冷却 3 步；推翻零模型摘要与
  仅最终答案间隙裁决；D2-2/D3-1 定为实施前置（S1）；实施待用户放行。
- 2026-08-14：P0-B 步骤 5 复核修复批次——审查发现的全部可处理问题已处理：
  `SRC-###` 改 run 级唯一分配（orientation-fire-run 重捕，5 commit →
  `SRC-001..005`）、URL 归一化复用共享 canonical 化、主车道 web 证据接入
  URL 绑定、全角冒号变体解析 + 围栏/行内代码块字面标记跳过、文档 `§`
  锚点不再参与 claim 上限、fixture message_block 对齐 producer、审计
  数字修正（orz-bin 42/14 ignored）、行界 TOCTOU 显式登记；验证重跑全绿
  （orz-loop 275 / orz-assurance 152 / orz-tui 178 / orz-bin 42、Python
  runtime 239 / assurance 1607+14 skipped、仓库门禁 valid）。详见
  [GAP-RETRIEVAL-MECH 步骤 5 实施审计](audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md) §7。
- 2026-08-14：P0-B 步骤 5 闭合登记——输出级引用校验器与交付边界接线实施
  完成（`[来源: ...]` 结构化解析 + ledger/path:line/URL/文档绑定 + §3.7.5
  上限 + 机械降级块 + `citation_validation` 事件 Schema/verifier/fixtures
  先行、TUI 投影、conformance 第 14 场景；主车道补 main_evidence 与
  run_source_ledgers）；orz-loop 268 / orz-assurance 151 / orz-tui 178 /
  orz-bin 42、Python runtime 239 / assurance 1621 通过，仓库门禁 valid；
  批次下一步为步骤 4（browser_read 范围/模式参数扩展，前置裁决已登记）。
- 2026-08-14：P0-B 步骤 4 前置裁决登记——用户裁决：browser_read 第二段计数域
  挂载在检索子代理 activation（复用 web_fetch per-activation 语义：activation
  累计、去重 URL 计数、continue 重入不重置、关闭清零）；主 Agent 不执行检索
  任务——主车道模型可见投影移除 browser_read，子代理投影从 host registry
  恢复（实现 + 单测，设计 §1.3 注更新）；步骤 4（browser_read 范围/模式参数
  扩展）按此实施。
- 2026-08-14：P0-B 步骤 4 闭合登记——browser_read 范围/模式参数扩展与第二段
  计数域复用实施完成（mode=full/preview/keywords + keywords 数组契约，
  preview 4K / keywords 16/64/3/160/12K 常量；browser_read 与 web_fetch 共用
  同一 activation 计数域与 cap，拒绝码 browser_read_candidate_*，
  tool_completed 计数字段覆盖 browser_read；证据按 mode 降级；Schema/
  verifier 先行，`_verify_v02_candidate_count` 覆盖两家族；conformance 14
  场景重捕，local-browser-read 携带计数字段；ACAF e2e browser_read 场景
  迁移到子代理车道，主车道直呼 fail-closed）；orz-host 205 / orz-loop 280 /
  orz-assurance 151 / orz-tui 178 / orz-bin 42、Python runtime 244 /
  assurance 1607+14 skipped、仓库门禁 valid；批次下一步为步骤 6（提示词
  相应缩短与测试更新）。详见
  [GAP-RETRIEVAL-MECH 步骤 4 实施审计](audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md)。
- 2026-08-14：P0-B 步骤 4 全面审查修复登记——候选门禁改为“决策先行、消费后置”
  （`candidate_gate` 只决策，`commit_candidate` 在权限/ACAF 票据通过后、
  ToolStarted 前提交；被权限/票据拒绝的调用不消耗预算、拒绝事件不携带计数）；
  keywords 摘录正文严格 ≤12K（分隔符/省略号计入）、提取输入按 100K 截断并打
  “input capped”页脚、工具定义补 `maxLength=64`、页脚 terms 改为实际输出词数；
  `ActivationState.web_fetch_candidates` 改名 `candidate_urls`；候选拒绝事件
  仅在车道内携带 target；新增门禁/提交拆分单测与 e2e“拒绝不消耗→重试计数=1”
  断言。登记：ADR-0010 §14.11、步骤 2/4 审计、设计 §1.1/§1.3 注。复核后：
  orz-host 207 / orz-loop 281 / orz-bin 42、Python runtime + assurance
  1851+14 skipped、仓库门禁 valid。
- 2026-08-14：ORZ-CACHE-CONTEXT-COST 登记（P1）——缓存与上下文成本收敛三项
  （请求 header 留痕、探针准确性、单轮注入预算 + 策略化读取），ADR-0010 v1.9、
  探针设计 §11 同步；否决方向一并登记（per-window 探测、预热轮、工具层后置渲染、
  工具层 1K 压缩）。
- 2026-08-14：P0-B 步骤 3 审查修复——verifier 允许“全净化空池”（空保留池
  须有 prefilter_log 移除记录）、`redirect_query_keys` 默认收窄（移除
  url/next/goto/target/continue）、redirect pattern 改 host 边界匹配、
  scheme-less 带端口引用解析回退、clippy 文档 lint 与审计计数口径修正；
  orz-assurance 151 / orz-loop 247 / Python 全量 1840 通过，门禁 valid。
- 2026-08-14：P0-B 步骤 3 闭合登记——机械预筛模块实施完成（canonical/host
  级去重、bad_url/login_wall/redirect_chain 移除、tier/weight + 词法相关性
  排序；`candidate_urls` 升级为预筛后保留池，`candidate_pool` +
  `prefilter_log` Schema/verifier 先行；配置种子 JSON + env 覆盖）；orz-assurance
  149 / orz-loop 246 / orz-host 200 / orz-tui 178 / orz-bin 全绿，Python 全量
  1838 通过，仓库门禁 valid；批次下一步为步骤 5（输出级引用校验器）。
- 2026-08-14：P0-B 步骤 2 闭合登记——web_fetch 候选机械计数门禁与计数反馈
  实施完成（`ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8、per-activation 计数域 +
  侧车持久化、无 ToolStarted 拒绝 + 熔断同面、`tool_completed` 计数字段
  Schema 先行、verifier 规则 + 9 测试、Rust +6 测试（含 continue 跨派发累计）
  与快照扩展）；审查修复——verifier 排除派发包装误报、黑板 exec 镜像与
  对话消息一致、cap-exceeded 缺字段加固、browser_read 计数域挂载面注记；
  批次下一步为步骤 3（机械预筛）。
- 2026-08-14：新增 AUTH-TODO 路由——建立根目录 [`TODO.md`](../TODO.md) 实施勾选清单（派生自本文件未闭合项），本文件仍为优先级/决策门权威。
- 2026-08-14：文档审查复核——FUS-TOOL-PROBE 小节头部由“实施中”改为已闭合（与索引 `implemented` 对齐）；CLASSICAL-EXEC-ASSISTANT 设计文档状态头、投影入口与 README 冻结版本表述联动修正。
- 2026-08-14：P0-B 步骤 1 审查闭环处理——旧来源加权审计 B-1/D-2/D-4 边界补
  闭合/取代注记、设计文档范围措辞更新、B-1 审计补进程内接缝与精确去重边界、
  verifier 补 ref 反向镜像负例测试 2 条（Python 全量 1821 通过）。
- 2026-08-13：P0-B 步骤 1（B-1 闭合）完成登记——web_search citations 结构化
  透传进 loop（ToolResult.structured 接缝 + 证据 candidate_urls + Schema/
  verifier 先行）；orz-loop/host/assurance/tui 测试全绿、Python 全量 1821 通过、
  仓库门禁 valid；批次下一步为步骤 2（web_fetch 计数门禁）。
- 2026-08-13：P0-A-2 闭合登记——v0.2 单一探针面扩展实施完成（23 个工作工具统一机械探针、面 A/C 撤销、投影切换为探针完整集∩声明集、`tool_availability_check` 事件/verifier/fixtures 同步、`LoopHost` 六项能力访问器接线）；orz-loop 236 / orz-host 199 / orz-tui 178 通过，Python verifier 与相关测试 395 通过，仓库门禁 valid；边界=orz-host 可选后端（lsp/memory/图像/视频/MCP）未接线，接线时翻转能力访问器。
- 2026-08-13：P0-A 步骤 7 完成登记——ADR-0010 v1.8 §3.5 修订登记（v0.2 单一探针面取代 v1.5「registry 全量 + 零可用性承诺」；A+C→B 定档合并登记，ADR §14.5 补复核注）；批次步骤 1-7 全部闭合；登记实现差距——代码仍为 v0.1 三面语义，v0.2 单一探针面扩展实施（P0-A-2）待续。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.5 两点获用户确认——注册板块常驻但内容按需读取（防上下文膨胀）；单轮一单先行（防并发写单竞态，反馈闭环驱动）。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.5 登记——黑板动作栏定为生产协作形态（注册板块/动作栏/结果栏；模型面=读板块+写订单，发放=机械单一出口；写订单无副作用，执行幻觉只能污染订单）；发放语义与配合规则登记。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.4 登记——执行失败特殊反馈（错误信封恒带 trace_id、step=execute 附有界 trace 尾部）+ 模型可见执行日志（只读 `assistant.trace` 服务）落地，POC 冒烟 28/28；机械组合模型（PTC 线性脚本模式）设计登记，列入实施序列小样 3。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 动作粒度裁决登记——细粒度优先（粗按钮才是限制模型），细动作必须配套机械组合层（PTC/管道/意图）；负担评估=构建期线性、运行时近零，风险在边界漂移与变更连锁；护栏=版本化动作契约 + 探针可见性 + Profile/Bundle 分区 + 契约即测试。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT fail-closed 返回契约定义并落地——调研 HA 原项目（成功 `result:{context,response}`；失败 `error:{code,message}`，message 带校验路径，无失败点/上游结果）；错误信封扩展为 step/code/message/upstream 四键；POC 冒烟升至 21/21。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.3 登记——集成形态改为 HA 助理层与 orz 深度融合（HA 是 orz 的一部分），薄接缝改置 orz 本体 ↔ 底座（Grok/Codex 等模型后端），POC stdio 协议仅原型隔离；登记 DeepSeek Harness 借鉴（PTC 程序化工具调用 + Profile/Bundle 动作组合，只借设计不引栈）；实施序列第 3 步改为 orz 内嵌集成。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 升入 P0（P0-C）——小样 1 跑通（20/20）、槽位表由工作区索引动态生成闭合（POC，生产复用 `project_doc_index` 缓存）；待办重排为 槽位表 → 编辑执行器 → orz 薄适配 → 决策门。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 小样 1 跑通登记——`prototype/classical_console/` 服务注册表 + hassil 意图第一条通路，17/17 检查通过；决策门保持（控制台路由小样达标后裁决正式组件）。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 威胁模型与策略执行点补充登记——只防幻觉与注入；助理层 = 单一策略执行点（围栏合并为策略表 + taint 组合禁令）；沙盒分层（策略层成立、OS 层按动作挂载）。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 设计草案登记——新建设计文档（v0.1）；同日升 v0.2 操作台模型（不需要理解层、模型面只注册名称），P2 探索项；决策门改为控制台路由小样，通过后再裁决正式组件。
- 2026-08-13：本轮判定登记——A+C→B 单一探针面定档；复杂度治理判定（LIF 不变量优先，砍冗余不砍不变量，ACAF Slice 3/4 暂缓，不做机制×不变量清单）。
- 2026-08-13：OPS-PROTOCOL 审查判定登记——用户无异议，平行执行层裁剪方向定案（保留删除安全为 host-owned 工具、桥接内部化、执行器单一化）；裁剪设计待产出。
- 2026-08-13：P0-A 步骤 6 完成登记——兜底消息中性化改造（权限门禁拒绝消息、连续拒绝熔断块、系统提示词、run_tests 管道错误与 DC 最小动作中性化，error 码与行为不变；新增系统提示词/熔断块中性词契约测试）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 7。
- 2026-08-13：步骤 6 全面复核裁决与修正——检索车道不适用本设计（回退检索分发拒绝消息与对应测试断言至原措辞）；事件 error 码/机器 reason 与明确事实性内容可进模型面；面 C 工具 schema 描述保留；deny 消息标点统一、中性词测试覆盖补齐 8 项；设计文档 §2/§3 登记适用范围裁决；orz-loop/host/tui 测试全绿、仓库门禁 valid。
- 2026-08-13：步骤 5 审查修复登记——调用即探针按车道隔离（`run_host_tool` 增 `probe_writeback` 标志，主/grill 车道才回写最小映射，检索车道失败只走 ToolCompleted(error) 审计，防主车道审计/事件流污染）；补检索车道零污染与跨 run 映射重置两项回归测试；设计文档补 §5 车道边界与 §8 run-start 首翻澄清；orz-loop/host/tui 测试全绿、仓库门禁 valid。
- 2026-08-13：P0-A 步骤 5 完成登记——最小上一轮映射与翻转事件（每模型请求前重算、仅翻转发事件、调用即探针回写、列表投影移入共享循环按轮重算、检索车道不探）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 6。
- 2026-08-13：步骤 3/4 全面审查清理——run_tests 无 runner 竞态调用改为 dispatch 前无 ToolStarted 中性拒绝（`missing_test_runner`）；补 workspace 不可读投影移除与竞态兜底测试；设计文档状态转 `approved`、删除 image_edit 探针行、登记面 A/C 同源约束与竞态兜底；controller/agent_loop 注释与新语义对齐；索引 FUS-TOOL-PROBE 转 `partial`。
- 2026-08-13：P0-A 步骤 4 完成登记——列表投影接线（面A + 面B完整集∩声明集 + 面C + 非工作工具；tool_probe 补面 A/C 常量与判定；ReadOnly 写探针过滤落地）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 5。
- 2026-08-13：P0-A 步骤 3 完成登记——`run_tests` 条件声明迁移至面 B 探针（controller 删除直接条件声明；探针完整才声明、不完整即移除；新增两条迁移语义测试）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 4。
- 2026-08-13：P0 决策登记——FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH 经用户裁决放行实施（ADR-0010 §3.5 修订按 P0-A 批次末第 7 步登记）；执行顺序裁定：P0-A 工具探针 → P0-B 检索机械控制 → P1 并行审计 → P2 核查收口/裁决/Slice 3/OPS 接线/Slice 4 → P3 收尾。
- 2026-08-13：P0-A 步骤 1-2 完成登记——面 B 探针模块与 `tool_availability_check` v0.2 事件升级（Schema/verifier/fixture/producer/TUI），真实 journals 重捕；批次下一步为步骤 3。
- 2026-08-13：审查处理登记——修复 Rust TUI incomplete 计数缺陷、README/设计示例一致性、TUI 状态段语义与判定词中性化、gate_decision 预留注释；登记步骤 4 交集语义与 orz-host 既有 flaky。
- 2026-08-13：建立统一待办；P0-P3 优先级按全量回查结果登记；设计/审计文档待办小节改为指针。
