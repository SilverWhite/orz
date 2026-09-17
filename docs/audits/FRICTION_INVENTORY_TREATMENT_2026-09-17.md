# 摩擦盘点处理批报告（`FRICTION_INVENTORY_2026-09-17` 处理批；run `RUN-CLI-6aac0af5`）

> 日期：2026-09-17；run＝**`RUN-CLI-6aac0af5`**（本地 23:44 起；载体 orz **0.6.1** 三件套 `D:\tb-eval\orz-windows\`）。
> 题面＝用户指定原文逐字（`.tmp-friction-task.txt`，207 字符）：先查 `CLI_PROJECT_INDEX.md` 路由 → 回查文档 → 处理 [`FRICTION_INVENTORY_2026-09-17`](FRICTION_INVENTORY_2026-09-17.md) 盘点摩擦项 → 按项目惯例落报告文档 → 期间新摩擦一并登记。
> 起步口径：cwd＝`D:\CLI`（显式；非 orz 子模块）、`ORZ_MAX_WALLCLOCK=0`（无墙钟）、`--real --allow-write --allow-shell --allow-network`；**不 commit、不 push**。前置＝同题面前次 run `RUN-CLI-6aabf5eb` 因 FR-N01（字节切片 panic）中止，0.6.1 修复后本批为题面重启的处理批。
> 树态：orz 工作树含 0am 批（挂起，留 O2 裁决）未提交改动，本批改动叠加其上（**主会话复核注**：FR-C04 批已随 orz `26dcce1b` 提交，不在未提交面；`blackboard.rs`/`controller.rs` 为 0am＋本批两批同文件，提交切分须知悉）。
> 口径纪律：用户 §11 批注已定「不处理 4（A01/A02/A05/A09）／需处理 30＋注记级 1」；本批只做**可安全落地的小修与复核销项**，B 类（O2 门）／FR-C07·D02（S4 门）／E 类（解冻门）只登记不实施。

---

## 0. 处置速览

| 条目 | 处置 | 落点／证据 |
|---|---|---|
| FR-A08 TER 断链 ×7 | **处置完成**：9 md 入 `存档/ter-review-2026-09-04/`；两 TER 文档 7 链接改写；存档 README 补条目；残留链接 0；门禁无新增错误 | 存档目录＋`TER_REVIEW_HANDLING`/`TER_M1_M2` 两档 |
| FR-A07 台账无可读摘要 | **复核＋修复**：台账行含目标/结果指针/最终回复；「目标=（无）」根因＝`TARGET_FIELDS` 缺 harness 参数名 ⇒ 补 `file_path`/`target_file`＋新钉子；模块 22/0 | `orz/crates/orz-loop/src/action_ledger.rs` |
| 注记级 字面双副本 | **去重落地**：新增唯一常量 `BLACKBOARD_WRITE_TOOL_NAME`；controller 注册处＋agent_loop 两处过滤改用；注释同步 | `blackboard.rs:20`、`controller.rs:3147-3152`、`agent_loop.rs:1076/2966` |
| FR-E01 turn_count 硬编码 | **形态复核 ⇒ 销项候选**：已由 orz `928dceb3` 修复（真实会话轮＋4 断言） | `controller.rs` 相关在码 |
| FR-E02 deny 信封不入事件面 | **形态复核 ⇒ 销项候选**：0p S2 已落 `tool_completed.policy_denial{source,code,reason}`＋测试 | `controller.rs:4287` 测试在码 |
| FR-C08 阶梯/fold 死面 | **形态复核 ⇒ 销项候选**：两族实现已退役（`61982a56`/`501447c0`）＋退役钉子；正式销项留账本批 | `agent_loop.rs:6058` 钉子 |
| FR-A03 序列化失败 ×2 | **取证收口，不修码**：文案非 orz 产物（全源/两代二进制/脚本/journal-conformance 全 0 命中）；0am 拒绝面事件链完整（seq 930-932）⇒ 留观察（下次复现留现场） | 见 §1.5 |
| FR-A04 陈旧 todo 残留 | **留复核**：原文不在 journal（不可重建）；本 run 未复现；属 grok-build 工具面 | 见 §1.6 |
| FR-A06 压缩交互 | **形态复核**：v8 无机械兜底、`model_participated` 如实落账；本 run 压缩 4/4 为模型产出摘要（`mode=model_summary`） | 见 §1.7 |
| FR-A10 并行抖动 | **读数固定（串行口径）**：清 env 串行 **332/0/5** vs 并行 **329/3/5**；并行抖动具名 3 条；套件读数固定串行 | 见 §1.8 |
| FR-A11 grok_home 4 失败 | **根因定位＝环境归因（降级）**：launcher env `GROK_HOME` 预置所致；清后全绿 **332/0/5**；随 env 报告收录 | 见 §1.9 |
| FR-A12 语料口径漂移 | **口径注落地**：TODO＋BACKLOG 的 S3 行补「滚动集、以实测 runs 计数为准（2026-09-17 实测 134）」 | `TODO.md:855`、`BACKLOG:970` |
| FR-A13 WIP 跨会话无状态行 | **示范落地**：`.tmp-wip-status.txt`（交接口状态行）＋本报告树态节 | 主仓根 |
| FR-A14 启动器未脚本化 | **脚本落地**：`scripts/dogfood_launch.ps1`（cwd 断言＋载体三件套＋ACAF provision＋env 装配＋清单输出；`-DryRun` 自测通过×2，含负例） | `scripts/dogfood_launch.ps1` |
| FR-D01 旗标契约漂移 | **清单落地**：`scripts/TB21_RUN_FLAG_CHECKLIST.md`（三类旗标现状复读＋起跑前打勾＋归因二选一登记） | `scripts/` |
| FR-D03 Docker 代理摩擦 | **配方固化**：`scripts/DOCKER_PROXY_RECIPE.md`（字节级还原口径 8 步＋假绿陷阱） | `scripts/` |
| 新摩擦（本批） | N-2 测试读数被 launcher env 污染；N-3 本机 rustc 崩溃与处置；N-4 `.ps1` 编码面 | 见 §2 |

---

## 1. 处置明细

### 1.1 FR-A08（TER 断链 ×7）——处置完成
- 事实：`TER_REVIEW_HANDLING_2026-09-04.md` 与 `TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md` 共 7 处链接指向 `../../.ter_review_2026-09-04/`；该目录被 `.gitignore` 排除 ⇒ 克隆面常驻 7 条门禁错。
- 处置：9 个 md 全量入 `存档/ter-review-2026-09-04/`（内容落库、可随仓分发）；两档 7 链接改写为 `../../存档/ter-review-2026-09-04/…`；`存档/README.md` 补条目。
- 验证：全仓无残留 `.ter_review` 链接；门禁错误数回到基线 1（唯一 orz submodule dirty）。

### 1.2 FR-A07（折叠台账无可读摘要）——复核＋修复
- 复核（v8 后形态）：台账行格式已在 v8 收口为 `[seq] 轮次N: 工具 目标=… 结果=sha256:… 最终回复=…`（单行、字段 300 字符上限）——**原立案形态已大改**，需按现行面处置。
- 修复：实测 `read_file`/`search_replace`/`blackboard_write` 行常现「目标=（无）」；根因＝`TARGET_FIELDS` 未覆盖 harness 参数名 ⇒ 补 `file_path`/`target_file`；新增钉子 `target_of_call_covers_harness_argument_names`。
- 读数：`action_ledger` 模块 **22/0**、`fmt` 干净、全量 **791/0/3**（§4）。

### 1.3 注记级（`blackboard_write` 字面双副本）——去重落地
- 新增唯一字面常量 `pub(crate) const BLACKBOARD_WRITE_TOOL_NAME`（`blackboard.rs:20`，含说明注）。
- 生产判等面改引常量：controller 注册处（既有注释互指处）＋agent_loop 两处过滤（压缩窗口装配与窗口调用筛分）；agent_loop 注释同步更新（原「同字面」注记改「经常量单点」）。
- 未收拢面（如实登记，留「重名面收敛·顺手批」）：本 crate 另有注册表/分类/探针/投影面字面若干（`tool.rs:82/180`、`tool_probe.rs:59/394`、`host_exec/tool_run.rs:1119`、`retrieval/projection.rs` 等），兄弟 crate 2 处（`orz-assurance/src/journal/families.rs:56`、`orz-host/src/permission.rs:593`）——**本批只收拢「已注释互指的两份副本」及其同族过滤面**，全库收敛另批评估。

### 1.4 FR-E01 / FR-E02 / FR-C08——形态复核 ⇒ 销项候选（证据在码）
- **FR-E01（W2 D-2 `run_finished.turn_count` 硬编码 1）**：已由 orz `928dceb3`（0p S1 批）修复——`with_session_turn` 注入真实会话轮；断言 ×4 在码。登记行号漂移注记属实（`controller.rs:3541`→现址）。
- **FR-E02（W2 D-3 deny 信封不入事件面）**：0p S2 已落 `tool_completed.policy_denial{source,code,reason}`（设计 C）；测试在码（`controller.rs:4287` 区）。拒绝原因已可审计。
- **FR-C08（0AE-C4 阶梯/fold 死面）**：阶梯族随 D2 下线删除（`61982a56`）、有状态折叠族退役（`501447c0`）；`agent_loop.rs:6058` 区有「禁事件」钉子兜底 ⇒ **大概率已消解**，正式销项留账本批复核。

### 1.5 FR-A03（序列化失败 ×2）——取证收口：非 orz 产物，不修码
- 扫描面（0 命中）：orz 全源（1457 个 `.rs`）；orz git 全史（`-S "serialize tool result"` 空）；0.6.0 与 0.6.1 两代载体全部 exe（逐字节 `find`）；`orz-mcp` 等其他文案面；`scripts/`＋`D:\tb-eval` 全部脚本面。
- 0am journal 复核：`invalid type: null` 全 journal 仅 1 处（模型自述报告文本）；被指认的拒绝面实况＝`search_replace` old==new（`seq 930/931`：`exit_code=1`、`cause=command_exit_1`、`failure_target=channels.rs`；`seq 932` 审计行「未应用（exit 1）」）——**事件链完整、语义正常**；模型面孤立错误行不在 journal（渲染层文案，非事件面产物）。
- 结论：代码面无定位面、无可修点；**登记为「来源不在 orz 产物、留观察」**，建议下次复现时抓取现场（模型输入字节/宿主日志）。
- 引用：前次崩溃 run `RUN-CLI-6aabf5eb` 的 journal 亦含同向局部取证（`seq 618` 区：全仓扫描 0 命中、0am `search_replace` 50 次调用中的拒绝面 call_id），与本结论互证。

### 1.6 FR-A04（陈旧 TodoWrite 残留）——留复核（边界口径）
- 复核：陈旧 todo 原文**不在本 run journal**（模型输入不落全文，不可重建）；本 run 未复现该现象；`TodoWrite` 属 grok-build 工具面（非 orz 框架面）。
- 处置：维持「留复核」不盲修；若再复现，收取「跨会话 todo 原文＋注入位置」现场后再定。

### 1.7 FR-A06（压缩交互）——形态复核：机制面已换代，本 run 参与实测
- 现行形态（v8 后）：窗口轮只暴露 `blackboard_write`；收口**不置机械兜底**——模型产出语义摘要或如实落账 `model_participated=false`（「模型面总量只由模型自压与 H1/T1 管」，设计 §4）。
- 本 run 实况：`context_compressed` **×4**（`trigger 230551→198879`／`281950→179745`／`267925→206767`／第四笔 seq1253 前最后阶梯读数 291410`→192132`），全部 `mode=model_summary`、`reason=model_selected` ⇒ **参与成立（4/4）**；块降幅 8/26/26/20 轮。**主会话复核注**：报告初稿写 ×3——第四笔发生在报告落笔后的收尾段（seq 1253，16:17:59），已按 journal 更正。
- 建议去向维持设计面（「摘要产出动作化」或「工具结果尾徽标」属机制改动，留设计批评估），不作本批实施。

### 1.8 FR-A10（orz-host 并行抖动）——读数固定：串行口径 + 具名抖动面
- 本机读数（清 env）：**串行 337＝332/0/5**（49.05s）vs **并行 329/3/5**（13.19s）。
- 并行抖动具名 3 条（较 0ai「多出 4 条未逐一复验」前进一步）：`process_tree::tests::sweep_reaps_a_matching_orphan_and_kills_it`、`tests::call_tool_timeout_kills_process_tree`、`tests::run_tests_output_scrubbed_of_secrets`（进程树/Job/超时族，负载敏感）。
- 口径决议（建议随账本批收录）：**套件读数固定串行**（`--test-threads=1`）；并行读数只作压力参考。

### 1.9 FR-A11（grok_home 4 条失败）——根因＝环境归因（建议降级）
- 隔离实验：4 条单独运行 ⇒ **3 真败＋1 锁毒化级联**（`env_already_set_is_respected_verbatim` 单独运行通过）。
- 归因实验：**清 `GROK_HOME` 后 3 条全过**；`GROK_HOME` 单独预置⇒败、`GROK_AGENT` 单独预置⇒过 ⇒ 根因＝本机 launcher env 预置 `GROK_HOME=D:\tb-eval\orz-windows\grok-home`。
- 读数对照：未清 env 串行 328/4/5；清 env（含 `GROK_HOME`/`GROK_AGENT`/`ORZ_*`）**332/0/5 全绿**。
- 建议：条目从「独立立项候选」**降级为「测试环境口径项」**——处置＝测试口令先清 `GROK_HOME`（或让 grok_home 测试对预置 env 免疫）；非产品缺陷。

### 1.10 FR-A12（语料口径漂移）——口径注落地（轻量）
- 实测复核：`D:\tb-eval\jobs-official` 现 **134** 个 `events.jsonl`（与 0am 报告 §1 一致）。
- 落地：`TODO.md` S3 行与 `BACKLOG_AND_PRIORITIES.md` 0am §开放内容 S3 段各补口径注——「滚动集、以回放器实测 runs 计数为准（2026-09-17 实测 134；含 final-smoke/arm-dryrun 追加；『102-run』为立项时点快照）」。设计两档（RLI/LIF）原文不含 102 计数，未动。

### 1.11 FR-A13（WIP 跨会话无状态行）——示范落地
- 新落 `.tmp-wip-status.txt`（主仓根；`.tmp` 前缀门禁忽略）：交接口状态行＝run 号／两仓树态（0am 批＋FR-C04 批＋本批）／读数／下会话接手点（先读何档、勿混提、测试前清 env 清单）。
- 即本批自身即为「WIP 在树（三批同树未提交）」的活例——状态行对下一会话直接可用。

### 1.12 FR-A14（狗粮启动器未脚本化）——脚本落地＋自测
- 新落 `scripts/dogfood_launch.ps1`：① `cwd` 断言（必须 `D:\CLI`、拒绝 orz 子模块——M-1 形态硬拦）；② 载体三件套存在性＋sha256 记录；③ ACAF（manifest 复用或现 provision，`-Shadow` 可切 fail-closed=0）；④ env 装配（无墙钟、权限三键、PROTOC、GROK_HOME）；⑤ 启动前列装配清单；⑥ 日志 `.tmp-dogfood-*.log`。
- 自测：`-DryRun` 正例通过（装配清单打印完整）；负例 `-Workspace D:\CLI\orz` 正确拒绝（断言原文「不得为 orz 子模块」）。**未实跑完整 run**（本批自身即运行中的 run，避免嵌套启动）。
- 实现注记：Windows PowerShell 5.1 读 `.ps1` 按 ANSI ⇒ 文件须 **UTF-8 BOM**（本次即踩即修；或脚本保持 ASCII）。

### 1.13 FR-D01（适配器旗标契约漂移）——起跑前对账清单落地
- 新落 `scripts/TB21_RUN_FLAG_CHECKLIST.md`：三类旗标（`--max-tool-rounds`／`--retrieval-mode`／`eval_browser`）的**现状复读**（`tb_agents/orz.py`：`_DEFAULT_MAX_TOOL_ROUNDS=999` 经 `--max-tool-rounds` 传参；`eval_browser` 现可由 `--ak eval_browser=true`/`ORZ_EVAL_BROWSER=1` 开）+ 起跑前打勾 + 「装置缺件／镜像缺陷」二选一归因登记。
- 边界：只固化对账流程，不改装置侧代码；FR-D02（同属装置侧）修复 `419b1c8f` 待实机复验随 S4。

### 1.14 FR-D03（Docker 代理切换摩擦）——配方固化
- 新落 `scripts/DOCKER_PROXY_RECIPE.md`：症状/根因链（`ProxyHTTPMode: manual`→死 Clash→`http.docker.internal:3128` 中继；`update` 假绿陷阱）＋**字节级还原 8 步配方**（备份→记账→切 `disabled`→重启→**install 实包验证**→构建→还原→复读核对）。
- 边界：先例固化（052/054）；脚本化（switch/restore＋重启面）留后续批；Harbor portproxy 面另档（`setup_harbor_proxy.ps1`）。

### 1.15 其余条目（本批只登记不实施，理由随账本口径）
- **B 类 6（FR-B01–B06）**：O2 裁决门未开（0am 线挂起），维持登记。
- **FR-C01/C02/C03/C04/C05/C06**：随 0ac S4／账本批（C04 采②已落码未提交；C05 已销项闭合——均见盘点档 §11）。
- **FR-C07／FR-D02**：S4 前置（D02 修复待实机复验）。
- **FR-D04**：边界登记（非缺陷），维持。
- **E 类 4（FR-E01–E04）**：冻结门；E01/E02 本批完成形态复核并转销项候选（§1.4），E03/E04 维持。
- **不处理 4（FR-A01/A02/A05/A09）**：按 §11 用户裁决维持（A05 归类勘误留档）。**注**：本批 N-3（rustc 崩溃）属 A05 同族资源面，按「真机若反复撞到须翻案重议」的口径如实登记。

---

## 2. 新摩擦登记（编号接盘点档 N 类 `FR-N01` 之后；本批期间发现）

| ID | 摩擦 | 证据/读数 | 建议去向 |
|---|---|---|---|
| **FR-N02** | **测试读数被 launcher env 污染**：狗粮启动 env（`ORZ_REAL=1`、`ORZ_ALLOW_*=1`、`ORZ_ACAF_FAIL_CLOSED=1`、`ORZ_ACAF_MANIFEST/KEYSTORE/BINARY`、`ORZ_MAX_WALLCLOCK=0`、`GROK_HOME`、`GROK_AGENT=1`）被测试进程继承 ⇒ `cargo test -p orz-loop` 批量红（panic：`ACAF fail-closed is enabled but no signer client is configured`，例 `blackboard.rs:3272` 区）；清 `ORZ_*` 后 **791/0/3 全绿** | 本机实测（本批 §4）；同族见 FR-A11 归因 | 测试运行口令明示「先清 `ORZ_ACAF_*` 与 `GROK_HOME`」；或测试初始化忽略 launcher env。**影响面：任何「既有红灯」判断都须先核 env** |
| **FR-N03** | **本机 rustc 崩溃（资源族）**：`cargo test -p orz-host --no-run` 默认并行下 rustc 崩溃 ×2（`0xC0000409 STATUS_STACK_BUFFER_OVERRUN`；涉 `orz-loop`、`orz-mcp` lib 编译，同参数复现）；`RUST_MIN_STACK=134217728`＋`CARGO_BUILD_JOBS=2` 后通过（2m03s） | 本批实测 | 重构建/测试口令固定该 env；与 FR-A05（宿主资源限，用户裁决「不处理」）同族——若真机反复撞到按 A05 口径翻案重议 |
| **FR-N04** | **`.ps1` 编码面（小）**：Windows PowerShell 5.1 按 ANSI 读 `.ps1`——含中文且无 BOM 时解析失败（`字符串缺少终止符`；本批 `dogfood_launch.ps1` 首版即踩，BOM 后双例自测通过） | 本批实测 | 脚本批约定：`.ps1` 带 UTF-8 BOM，或保持纯 ASCII |

## 3. 树态、混存与提交切分注意

- **orz 树**（未提交）＝0am 批（`Cargo.lock`/`Cargo.toml`、`lif/*`、`acp_server.rs`、`prompt.rs` 等）＋**本批 4 文件**（`action_ledger.rs`：target 提取修复＋钉；`blackboard.rs`：新增常量；`controller.rs`/`agent_loop.rs`：三处判等改常量＋注释同步）。**主会话复核注**：FR-C04 批已随 orz `26dcce1b` 提交入档，不在未提交面——初稿「三批同树」系沿 §11 时点口径未更新，现存未提交＝**0am＋本批两批**。
- **`blackboard.rs` 与 `controller.rs` 均为两批同文件**（0am 面＋本批常量）——提交切分须逐 hunk；`action_ledger.rs`、`agent_loop.rs` 为本批独占文件（可按文件切分）。
- **主仓**＝本批文档/脚本（§5）＋既有 `.tmp-*` 历史运行日志（未跟踪、门禁忽略；本批新增 `.tmp-wip-status.txt`）。
- **提交（2026-09-18 用户令「更新账本文档并提交推送」）**：orz **`07405e61`**（本批 4 文件；与 0am 批 hunk 级分离，0am 批保持未提交随 O2 裁决；单独工作树复核 orz-loop **786/0/3**＋fmt 干净〔按 ORZ-BUILD-MOUNT-001 补父仓 runtime 挂载〕）；父仓同批落文档/脚本/案例库与账本（0ak 闭合 36 → 35）。

## 4. 读数汇总（本批终态）

| 面 | 读数 | 口径 |
|---|---|---|
| 门禁 `check_repository` | **error_count=1**（唯一＝orz submodule dirty） | 报告落档前曾实测 3（=1＋本报告未落档时的 2 条引用断链）；落档后复测 1 |
| orz-loop 全量测试 | **791 passed / 0 failed / 3 ignored** | 清 `ORZ_*` env；含本批 1 新钉 |
| `action_ledger` 模块 | **22/0** | FR-A07 修复后 |
| orz-loop `cargo fmt` | 干净（`FMT=0`） | 本批末次 |
| orz-host 全量测试 | **串行 332/0/5**（49.05s）／并行 329/3/5（13.19s） | 清 `ORZ_*`＋`GROK_HOME`/`GROK_AGENT`；`RUST_MIN_STACK=134217728` |
| 压缩交互 | `context_compressed` **×4**，全 `model_summary`（参与 4/4） | `trigger 230551→198879／281950→179745／267925→206767／291410→192132（降幅 8/26/26/20 轮）` |
| 载体 | orz **0.6.1**（sha256 `5C991621E0F4…` 见启动器 DryRun 清单） | `D:\tb-eval\orz-windows\` |

## 5. 交付文件清单（本批）

**新增**
- `docs/audits/FRICTION_INVENTORY_TREATMENT_2026-09-17.md`（本报告）
- `scripts/dogfood_launch.ps1`（FR-A14）
- `scripts/TB21_RUN_FLAG_CHECKLIST.md`（FR-D01）
- `scripts/DOCKER_PROXY_RECIPE.md`（FR-D03）
- `.tmp-wip-status.txt`（FR-A13 示范；门禁忽略）
- `存档/ter-review-2026-09-04/`（FR-A08 归档；9 个 md）

**修改**
- `TODO.md`（S3 口径注）、`docs/BACKLOG_AND_PRIORITIES.md`（S3 口径注）
- `docs/audits/TER_REVIEW_HANDLING_2026-09-04.md`、`docs/audits/TER_M1_M2_COMPREHENSIVE_REVIEW_2026-09-04.md`（链接改写）
- `存档/README.md`（条目）
- orz：`crates/orz-loop/src/action_ledger.rs`、`blackboard.rs`、`controller.rs`、`agent_loop.rs`

## 6. 未决与下一步

1. 盘点档逐行处置批注（随本报告落档，见盘点档 §12）。
2. 正式销项与降级裁决随账本批：FR-E01/E02/C08（销项候选）、FR-A11（环境归因降级）、FR-A10 口径决议（串行读数）。
3. 0am 批合回、FR-C04/C05 状态维持既有裁决（§11）。
4. 下批候选（2026-09-18 更新）：**重名面全库收敛已立项排期 0ao**（§1.3 清单）、**压缩摘要动作化设计批已立项排期 0ap**（FR-A06 设计面，用户令「纳入排期」）；D03 脚本化、FR-A03 下次复现现场收取仍留候选。

> 维护口径：本报告与盘点档 §12 批注互为对应；全部处置明细以代码/文件在树为准（未提交态）。后续批引用本次读数时以 §4 口径为先（清 env＋串行）。
