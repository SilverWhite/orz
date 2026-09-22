# 0bd 总结档 — 摩擦大杂项轮（FR 批处置 · 繁杂狗粮轮）

> **日期**：2026-09-22（会话跨 21 日深夜—22 日凌晨）。**任务令**：处理 0bd；不提交／不推送／不重建；按项目惯例落总结档；摩擦随报告登记。
> **本轮载体**：0.6.8（源冻结 orz `f76e5e32`，含 0bf 码；RLI 缺省常开）。**本会话即该轮 S3 长会话真机狗粮轮**：`run=RUN-CLI-6ab17325`（journal `.gsa/runs/RUN-CLI-6ab17325/events.jsonl`）；同轮并收 0bc S3 三项资源面与 0be 投递链／侧车覆盖（用户令「长会话真机狗粮轮依靠实际任务，用 0bd 作为任务」）。
> **范围（十四件）**：① 构建前置自检＋自动降并行／② 测试入口默认清理 env／③ `--` 吞参＋shell 工具说明／④ 两条 HEAD 警告清零／⑤⑧ 启动器 RLI 开关＋回显／⑥ 设计 §11② 勘误（**已随 0bc 落地，不在本轮实做**）／⑦ RLI 数值判定式提醒／⑨ 会话 env 假红→测试夹具自清／⑩ 探针收编进仓／⑪ 冻结语料纪律／⑫ 宿主两条既存红定位／⑬ 繁杂度投递链与侧车真机覆盖／⑭ 题面编码。

## §1 结论摘要

- **十三件（①–⑤、⑦–⑭）全部落码并验证**；⑥ 已于 0bc 落地。测试侧：orz-loop 830/0/3（清 env 与污染 env **双绿**）、orz-host 322/0/5（串行；并行 320/2 属既有**并行负载敏感**红，本轮完成定位）、orz-assurance 272/0、orz-tools 关联子集 86/0＋181/0、xai-tty-utils 32/0；探针 example 编译＋运行双通（skill 表产出）。
- **S3 读数齐**：RLI 面两触发真机首现、网格补点 42→112、域迁移确认两度；资源面 `host_resource_snapshot`×22（其中 **`commit_notification` 真机首现 ×2**——0bc S3 ② 项达成；软提示送达 1 次）；CPU 去顶档样本 38.4%；进程上限默认 32。
- **⑫ 定位完成**（详见 §5）：两"既存红"＝并行负载/计时敏感，非功能缺陷；串行全绿。处置待后。
- 工作树改动**未提交、未推送、未重建**（任务令）；本档为交付物。

## §2 处置明细（逐件）

**① 构建前置自检＋自动降并行（缓解 F14）**：新增 `scripts/build_orz.ps1`。自检 commit/物理余量后选并行档：`auto`＝使用率 ≥88% 或 commit 余 <4 GiB→`-j 1`；≥72% 或 <12 GiB→`-j 2`；否则 `min(cores,8)`。本机读数（物理 15.8 GiB／余 5.7 GiB；commit 28.9 GiB／余 5.5 GiB＝81%）→ 判 2（与既存 `-j 2` 纪律一致）。含 `-Release/-Jobs/-Check/-Clean/-DryRun` 与 cargo 参数透传；`--` 容错同 ③。
**② 测试入口默认清理 env**：`scripts/run_orz_tests.ps1` 清 **7 键**（`ORZ_ACAF_FAIL_CLOSED/MANIFEST/KEYSTORE/BINARY`、`GROK_HOME`、`GROK_AGENT`；RLI 键未设故未清）并**显式回显**已清键集——不再依赖"记得用脚本"的记忆面。
**③ `--` 吞参＋shell 工具说明**：(a) 运行侧——本机实测 `& <script> … -- <args>` 会**吞掉裸 `--`**，而 `-File` 调用与引号 `'--'` 透传；脚本接口加启发式：检测到测试参数（如 `--nocapture`/`--test-threads`）前无分隔符时**补回 `--`** 并回显（实跑取证：提示「检测到被吞的前导 --…已补回」，`--nocapture` 正确抵达）。(b) 文档侧——`orz-tools` `internal_auto_bg` 描述模板尾部（`is_windows` 条件）新增 PowerShell 规则：传参用 `powershell -File <script.ps1> <args>`；`& <script> … -- <args>` 吞裸 `--`；CJK 乱码先设 `[Console]::OutputEncoding = [System.Text.Encoding]::UTF8`。**不代改模型命令**（遵「不做」面）。
**④ 两条 HEAD 警告清零**：(a) `xai-tty-utils/src/lib.rs` 补 `pub use resource_job::process_alive;`——0z S2R 引入的 kill-face 探针未 re-export 致 `dead_code`；`cargo check` 0 警告。(b) `orz-tools/src/types/resources.rs:2847` 测试元组 `journal`→`_journal`（+两行注释）——`unused variable` 消除；该测试（`windows_trailing_dot_gsa_spelling_denied`）跑绿。
**⑤⑧ 启动器 RLI 开关＋回显**：`scripts/dogfood_launch.ps1` 增 `-RliOff`（写 `ORZ_LIF_RLI_SHADOW=0`）与缺省**显式写 1**（0bf 常开语义）；装配清单增 `rli=` 行回显。
**⑦ RLI 数值判定式提醒（核证）**：落点＝0bf ③ 已实现的两触发（域迁移确认／持续越线 k=5），pull-delta 一次性、只给读数与特征、非硬提示。**本会话真机首现**（详见 §3：持续越线 slow×5 与域迁移确认×3 条）。用户口径「模型面只留两触发」——**未新增第三触发**；「预测达阈」措辞歧义**登记待裁决**（若原意含独立触发，与 0bf 校准冲突，需用户裁定）。
**⑨ 会话 env 假红→夹具自清（本轮核心）**：根因＝`orz-loop/src/controller.rs` `initial_acaf_fail_closed()` 的 `cfg(test)` 分支**读会话 env**：dogfood 载体把 `ORZ_ACAF_FAIL_CLOSED=1` 沿进程树下沉 ⇒ 单测进程无 signer ⇒ run 起点拒（`"ACAF fail-closed is enabled but no signer client is configured … refusing to start the run"`）。实测：污染对照下 orz-loop 全量 **614 过/216 败**。修复（两处）：(a) `controller.rs` `cfg(test)` 分支**恒 `false`**（不再读 env；显式 `.with_acaf_fail_closed(true)` 入口保留；生产 resolver 不动；注释登记 216 假红）；(b) `orz-host/src/grok_home.rs` `EnvVarGuard` 增 `cleared()`（先 `remove_var` 再捕获），3 条安装目录测试改用（会话 `GROK_HOME` 下沉致 `EnvRespected` 假红）。**复跑：污染 830/0/3 ✔；清 env 830/0/3 ✔**（不再两边分叉）。
**⑩ 探针收编**：`D:\tb-eval\rli-forecast-probe`（仓外，手改需外部脚本；0be 轮已有编码粘连摩擦）→ 收编为 **`orz/crates/orz-assurance/examples/rli_forecast_probe.rs`**（源 sha256 `db2ec197…`；头部注记来源与用法）。编译 exit 0；**运行 exit 0**：对 `.gsa/runs` 产出 skill 表（例：Slow h=5 skill=+0.340；Stall h=2 +0.392；Deny 通道 n=2088）＋JSON 产物。对拍一律对**冻结副本**（见 ⑪）。
**⑪ 冻结语料纪律**：新增 `scripts/freeze_rli_corpus.ps1`——`.gsa/runs` → 冻结副本 ＋ `MANIFEST.json`（sha256/字节/行数）＋ README；默认 `D:\tb-eval\analysis\frozen\rli-corpus-<stamp>`。live 语料随会话漂移，对拍/回归以冻结件为准。
**⑫ 宿主两条既存红定位**：见 §5（定位完成；处置待后）。
**⑬ 繁杂度投递链与侧车真机覆盖**：RLI 面已带 0be ④ 读数（分通道 horizon＋短视锚点＋λ̂＋**繁杂度行**，渲染测试含 `繁杂度:` 锚；面内受 1 KiB 截断所限，见 §6-g）；`user_notice` 事件本会话未出现（未达自校准阈值；登记）。**侧车（`conversations/<id>.json`）本会话未 drain**（未落盘），本轮以黑板 rli 面代偿核证，落盘核记入遗留（§7）。
**⑭ 题面编码**：`dogfood_launch.ps1` 读取端改**显式 UTF-8**（`-Encoding UTF8`）并回显 BOM 探测位——PS 5.1 ANSI（GB2312）误读无 BOM UTF-8 题面（0be 轮实测 `璇峰厛鏌ョ湅…`）不再复现（本会话题面 299B 带 BOM，回显核过）。与 ⑧ 同文件同批（用户口径：机械层修复、不涉模型命令改写）。

## §3 S3 真机读数（本会话＝长会话狗粮轮）

**RLI 面（kill switch 缺省路径＝常开）**——两时刻读数：

| 项 | 早段（≈r29） | 后段（≈r92） |
|---|---|---|
| 步数 / 采样（网格补点） | 229 / 271（42） | 361 / 473（112） |
| T̂ | 6.8 s | 11.0 s |
| 自判域 | normal（r55 入域） | normal（r150 入域；域迁移 low_progress→normal） |
| 近提醒 | 持续越线 slow×5（u=3.02≥θ=0.84） | 域迁移确认 ×2（r111 失配 0.12；r150 失配 0.06） |

- **两触发真机首现**（⑦ 核证）：持续越线（StreakCrossed，k=5）与域迁移确认（MigrationConfirmed，稳定 3 轮）各现；pull-delta 一次性、只读数、无「该做什么」措辞。k=5 与 τ=3 为 0bf 校准值（源头 `orz-assurance/src/lif/rli.rs`：`RliNoticeKind`/队列/push；`controller` 渲染）。「预测达阈」独立触发**未新增**（登记待裁决，见 §6-m）。
- **kill switch 缺省路径**：本会话 env `ORZ_LIF_RLI_SHADOW` 未设 → `RLI on`（0bf 缺省常开语义真机验证）。

**资源面（0bc S3 三项 + 通知设施）**：

- `host_resource_snapshot`×22：`run_start`×1、`tier_change`×19（watch⇄soft⇄hard；commit_free 低至 0.96 GiB）、**`commit_notification`×2（真机首现）**——payload：`commit_notification_bytes=4719947776`（4.40 GiB 线）、`commit_used_bytes=4.65/4.62 GiB`、`limit_flags=512`、`violation_flags=512`、`tier="unknown"`。伴**软提示送达 1 次**（模型面收到「[资源软提示] …已达 commit 临限通知线（4.40 GiB），动作照常执行」；每 run 一次去重生效）。
- **CPU 去顶档**：无 `cpu_rate_limit` 生效（0bc 默认去顶）；安装期样本 **38.4%**（3 s 均值；并行编译吞吐正常，无节流停滞）。重编译墙钟样本：orz-tools 测试编译 8m11s、orz-host 测试编译 7m48s、orz-assurance 51.95s（-j 2）。
- **进程上限**：默认 `2×cores+8`＝**32**（12 核）；本会话无 `ORZ_JOB_ACTIVE_PROCESS_LIMIT` 覆盖、无上限拒绝事件。
- 其它：journal **1612 行**（event_type 直方图：tool_started/completed、control_ticket_issued/consumed、mechanical_audit_update 等）；`context_compressed`×6；会话内长会话开销面正常。

**0be 投递链/侧车（⑬ 部分）**：RLI 面已现 0be ④ 读数形态（分通道 horizon＋短视锚点 `p1(1T̂)`＋`λ̂`＋**繁杂度**行；渲染测试含 `繁杂度:` 锚点）。`user_notice` 事件未出现（未达自校准阈值档）。**侧车 `conversations/<会话>.json` 本会话未 drain、未落盘**——真机覆盖改以黑板 rli 面代偿；落盘核记入遗留（§7）。

## §4 测试与验证汇总

| 对象 | 模式 | 结果 |
|---|---|---|
| xai-tty-utils | `check` 警告面 | **0 警告**（修前 1） |
| xai-tty-utils | 全量 | **32/0** |
| orz-tools | `types::resources` 子集 | **86/0** |
| orz-tools | `bash` 模块子集 | **181/0**（含描述渲染与 PowerShell 注相关面） |
| orz-loop | 全量・清 env（wrapper） | **830/0/3 ✔** |
| orz-loop | 全量・**污染 env**（修前→修后） | 614/216 ✗ → **830/0/3 ✔** |
| orz-host | 全量・清 env・单线程 | **322/0/5 ✔**（含两条"既存红"全过） |
| orz-host | 全量・并行 ×2 | 320/2/5（§5 定位） |
| orz-host | `grok_home` 污染子集 | **4/0 ✔**（修前 3 条假红） |
| orz-assurance | 全量・清 env | **272/0 ✔** |
| 探针 example | 编译＋运行 | exit 0 ＋ skill 表/JSON（§2-⑩） |
| wrapper ③ 取证 | `&` 调用＋`--nocapture` | 补回提示 ＋ 参数抵达 ✔ |

## §5 ⑫ 宿主两条既存红定位（本轮＝定位，处置待后）

**复现矩阵**（清 env）：

| 运行 | 命令 | 结果 |
|---|---|---|
| 串行 | `cargo test -p orz-host --lib -- --test-threads=1` | 322/0/5（全绿，含两红） |
| 并行 #1 | 默认并行 | 320/2：`call_tool_timeout_kills_process_tree` ＋ `run_tests_output_scrubbed_of_secrets` |
| 并行 #2 | 默认并行 | 320/2：`call_tool_timeout_kills_process_tree` ＋ `run_terminal_cmd_truncation_carries_output_object` |

**失败形态**：
- `call_tool_timeout_kills_process_tree`（两式）：(a) 续跑活性调用 `echo orz-alive` 撞宿主 2 s 工具预算 → `Timeout("… TIMED OUT after 2s wall-clock budget …")`；(b) python 悬挂脚本**快速 exit 1**（并行负载下孙子进程 spawn 失败路径）→ 首调用未走超时路径即断言失败。
- `run_tests_output_scrubbed_of_secrets`：对话尾为空（`result.output` 无 `[REDACTED_SECRET]`）——并行负载下 runner 输出缺失/丢失的竞态。
- `run_terminal_cmd_truncation_carries_output_object`（浮动次红）：30K 输出未触发默认 8K 截断——并行负载下产出/截断路径竞态。

**结论**：两条"既存红"（及一条浮动次红）均为**并行负载/计时敏感**，非功能缺陷——串行档全绿、复现实证。与 0bf 报告「host 316/6 环境红」与 0bc 记录相互印证。**处置建议（待后，不阻断）**：(i) host 套件默认以 `--test-threads=1` 档作为"红判据"入口（wrapper 文档化）；(ii) 如需并行档全绿：`call_tool_timeout` 续跑调用改显式更长覆盖（保留语义）、另两条 red 查 runner/截断竞态（需要单独设计）；(iii) 不因环境敏感而放宽断言强度。

## §6 摩擦登记（a–n）

- **a. 题面编码（已修，残留建议）**：PS 5.1 `Get-Content` 默认 ANSI（GB2312）读**无 BOM** UTF-8 题面 → 乱码进载体（0be 轮实测；本会话题面带 BOM 无恙）。修复＝读取端 `-Encoding UTF8` ＋ BOM 回显（⑭，与 ⑧ 同文件）。残留：写入端仍可能产出无 BOM 题面——建议后续写入端强制 BOM（登记）。
- **b. PS 5.1 `&` 调用吞裸 `--`（已修＋文档化）**：`& <script> … -- <args>` 会把裸 `--` 吞掉（`-File` 调用与引号 `'--'` 透传）。修复＝脚本启发式补回＋shell 工具描述 PowerShell 注（③）。残留：对**其他**直接 `&` 调 cargo 的脚本同样适用（后续新脚本沿用同容错）。
- **c. 会话 env 假红（已修；本轮最大项）**：dogfood 载体下沉 `ORZ_ACAF_FAIL_CLOSED=1`＋`GROK_HOME` 等 ⇒ orz-loop 全量 **216/830 假红**＋host grok_home 3 条假红。修复＝⑨ 两处（cfg(test) 不染 env＋夹具 `cleared()`）。教训：**测试语义不得继承会话 env**。
- **d. `initial_acaf_fail_closed` 测试分支语义与注释不符（已按注释意图收口）**：原 cfg(test) 读 env（=1 即 enforce），与"逻辑测试默认 shadow"注释相悖——本批收口为空值恒 shadow，显式入口保留。
- **e. 宿主并行负载敏感红（定位完成；处置待后）**：§5。串行全绿；并行恒现 1 条＋浮动 1 条。
- **f. orz-host `reclaim.rs:478` `write_file` 测试层 `dead_code` 警告（沿线发现；未处置）**：0bd ④ 口径外（HEAD 预置双警告＝xai-tty＋orz-tools 已清零），本批测试编译暴露第 3 条；登记待后续清算。
- **g. RLI 面 1 KiB 上限致「繁杂度」行读面不可见（读面口径）**：渲染测试断言在（`繁杂度:` 锚）但 live「now」面超限即截断，繁杂度/短视读数常落窗外。建议后续调整面内优先级或折叠策略（与 0bf「压缩窗口开销口径」同域）。
- **h. 侧车未 drain（本轮不能核落盘）**：`conversations/<会话>.json` 无本 run 文件（未会话结束/未 drain）；`user_notice` 未触发（未达自校准阈值档）。真机覆盖以黑板 rli 面代偿；落盘核记入 §7。
- **i. PowerShell 中文输出编码（已文档化）**：非 UTF-8 控制台致 CJK 乱码——已进 shell 工具描述（③）；本会话所有命令先设 `[Console]::OutputEncoding=UTF8` 作业已固化。
- **j. 探针仓外（已收编）**：原 `D:\tb-eval\rli-forecast-probe`（手改需外部脚本；0be 轮曾发生非 ASCII 注释编码粘连）→ example 形态进仓（⑩）。
- **k. live 语料漂移（已落纪律）**：`.gsa/runs` 随会话增长；对拍/回归改用冻结副本＋MANIFEST（⑪）。
- **l. `commit_notification` 事件形状（已登记）**：`tier="unknown"`、`readings` 四字段（verifier 允许集已含；与 0bc 定案一致）。
- **m. 「预测达阈」措辞歧义（待裁决）**：若用户原意含"预测达阈"独立触发，则与 0bf「模型面只留两触发」校准冲突——本轮按两触发核证、未新增第三触发（登记）。
- **n. 测试面并行档与构建面降并行的口径差（观察）**：① 的自动降并行只覆盖**构建**脚本；**测试**入口（②）仍为固定 `-j`。建议后续把「auto 档」同样引到测试入口（登记，不阻断）。

## §7 遗留与移交

- **⑦「预测达阈」歧义**：待用户裁决（未新增第三触发）。
- **⑫ 处置面**：本轮只交付定位（§5）；处置建议三条待后续批。
- **⑬ 侧车落盘核**：本会话未 drain；随会话归档自然覆盖（或下轮会话开档时核 `conversations/*.json` 的 rli 字段）。
- **承接 0bf 遗留**：压缩窗口取值开销口径／双迁移通知去留／权重与锚标定（**探针已可产出 skill 表供标定**：本批运行读数示例 Slow h=5 skill +0.340、Stall h=2 +0.392——可直接作标定输入）。
- **本批不提交／不推送／不重建**（任务令）：工作树改动与本文档留在本地，随下一批统一处置。

## §8 附录（证据与指针）

- **本批修改/新增（父仓）**：`scripts/dogfood_launch.ps1`、`scripts/run_orz_tests.ps1`、`scripts/build_orz.ps1`（新）、`scripts/freeze_rli_corpus.ps1`（新）、`docs/audits/0BD_FRICTION_MISC_2026-09-22.md`（本档）。
- **本批修改/新增（orz 子模块）**：`crates/codegen/xai-tty-utils/src/lib.rs`；`crates/codegen/orz-tools/src/{implementations/grok_build/bash/mod.rs, types/resources.rs}`；`crates/orz-loop/src/controller.rs`；`crates/orz-host/src/grok_home.rs`；`crates/orz-assurance/examples/rli_forecast_probe.rs`（新）。
- **测试日志与产物**：`D:\CLI\.tmp-0bd-orztools-norun.log`、`.tmp-0bd-t1a.log`（wrapper 取证）、`.tmp-0bd-loopfull-polluted.log`／`…-polluted2.log`（216→0）、`.tmp-0bd-hostfull.log`（串行 322/0/5）、`.tmp-0bd-hostfull-par{,2}.log`（并行 320/2）、`.tmp-0bd-assur.log`（272/0）、`.tmp-0bd-probe-out.json`（探针产物）。
- **读数指针**：journal `.gsa/runs/RUN-CLI-6ab17325/events.jsonl`（1612 行；`host_resource_snapshot`×22、`commit_notification`×2、`context_compressed`×6）；黑板 rli 面（now/recent 两读）；`.tmp-0bd-task.txt`（题面，299B，带 BOM）。

---

## §9 提交批（2026-09-22 用户放行：「现在的话请先进一轮提交和推送吧」）

| 项 | 结果 |
|---|---|
| orz 提交 | **`56d328ee`**（0bd 六件：`xai-tty-utils/src/lib.rs`；`orz-tools/src/{implementations/grok_build/bash/mod.rs, types/resources.rs}`；`orz-loop/src/controller.rs`；`orz-host/src/grok_home.rs`；**新增** `orz-assurance/examples/rli_forecast_probe.rs`），推 `cli`（`f76e5e32..56d328ee`） |
| 父仓提交 | 本档（新增 §9）＋ `scripts/dogfood_launch.ps1`（②③⑤⑧⑭）／`scripts/run_orz_tests.ps1`（②③）＋ 新增 `scripts/build_orz.ps1`（①）／`scripts/freeze_rli_corpus.ps1`（⑪）＋ 0be／0bf 合并转录闭合与 0bg 立项的台账三方同步 ＋ `adr/ADR-0010` §14.75／v1.77 ＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算 **1463** 条（差异 6 行＝5 改 1 增），推 `origin main` |
| 载体 | **不重建**——0.6.8 在役件仍为 `f76e5e32` 冻结源（RLI 中间过渡版，不发行；0.6.7 不补发）；本批源码**尚未进载体**（下次重建才带上） |
| 预检 | **免**（默认口径：纯逻辑面且不换装载体） |
| 门禁 | 提交前唯一 `error_count=1`＝「orz submodule working tree is dirty」（预期态）；提交后 `valid: true` |

**§7 与本节的边界**：§7「本批不提交／不推送／不重建」是**轮内纪律**（0bd 任务令原文
「任务完成后请不必进行提交/推送/重建，按照项目惯例落一份报告文档即可」）；提交与推送
由用户 2026-09-22 **单独放行**，故不构成 §7 的破例——「载体未重建」一条两节一致。

**同轮并批的台账面（随本批入仓）**：① **0be／0bf 合并转录闭合**——ADR-0010
**§14.75／v1.77** 一条转录覆盖「RLI 在线自适应＋生产化与域建模」（不为中间过渡语义
重复转录），0bf 的 S3 长会话轮＝**本轮 0bd 轮**，⇒ 计数 **46 → 44**；② **0bg 立项**
——RLI 收尾与摩擦大杂项轮＝本轮摩擦 a–n 三分类后**可处理九件**并轮（按 0bd 同形真机
狗粮轮）⇒ **44 → 45**；③ **三项 RLI 设计裁决**随批登记——第三 kind＝**掩盖缺口**
`CoverageGap`（分母＝域枚举四值、就绪门＝已完成段 ≥3、越线门＝驻留 ≥ 该域已完段中位；
随报 `g`＋未覆盖域列表＋驻留比＋域模型累积失配）、提醒**注解随报**（全采纳，理由＝
压缩后模型可能再也看不到释义）、**双迁移通知**＝模型面只报 RLI 确认式（LIF **本轮
暂不退役**，即时计数改由机械层记录、不发模型）。

关键词：0bd、摩擦大杂项轮、提交批、`56d328ee`、会话 env 假红、
`initial_acaf_fail_closed`、`EnvVarGuard::cleared`、探针收编、
`rli_forecast_probe`、0be／0bf 合并转录、0bg 立项、载体未重建。
