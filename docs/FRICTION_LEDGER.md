# 摩擦台账（FRICTION LEDGER）

> 状态：`current`；建立：2026-09-13（orz 制作 orz 狗粮线，用户裁决）。本文件是**活页台账**，不是审计快照：每轮 orz 真机 run 产生的摩擦项在此累积，逐条对账后要么立案（转 BACKLOG/GAP）、要么登记为观察、要么标记已修。
>
> **写入协议**（2026-09-13 用户裁决）：
> 1. **orz 侧**：每轮 run 收尾由 orz 中的模型**直接追加**条目到本文件（run 末摩擦自报，三约束：run 末一次性不逐轮注入；只报事实不给建议；无摩擦明说"无"不凑数）。条目必须带 `run_id`。
> 2. **主会话侧**：`tb21_friction_scan.py` 机械扫描 + journal 对账；自报只是线索，**journal 是权威**（归因纪律 / S-16 证据基线）；对账后标注处置（立案/观察/已修/否定）。
> 3. 条目格式：`F-###`（日期 | run_id | 归因类：装置侧/设计内门/模型习惯 | 现象与一手证据 | 代价 | 处置）。
> 4. 立案的条目同步进 BACKLOG；已修的保留记录并标 `fixed`。

## 条目

### F-001 | 2026-09-13 | RUN-CLI-6aa6a868 | 装置侧（文档/契约）| fixed
**ACAF signer env 契约错位**：宿主侧读 `ORZ_ACAF_MANIFEST`，signer 子进程读 `ORZ_SIGNER_MANIFEST`（未设时回退 exe 同目录），`README.md` 启动环境只列三个变量、缺 signer 侧两个 ⇒ 手工部署极易配错且报错信息不指向根因。证据：`orz-signer fatal: signer manifest not found at …\orz-windows\signer-manifest.json`（env 已设 `ORZ_ACAF_MANIFEST` 仍找不到）。处置：本轮以「同时设两套变量」绕过（host 透传逻辑本身正确，`orz-loop/src/acaf.rs:175`）；**候选动作**：README 补全两组 env 或宿主 spawn 时自动注入 signer 侧变量——待立案。

### F-002 | 2026-09-13 | RUN-CLI-6aa6a868 | 装置侧（发布流水线）| fixed
**发布包 manifest 哈希过期**：`acaf/signer-manifest.json` 的 `binary_sha256` 与包内实际 `orz-signer.exe` 不符（包刷新时未重算）⇒ signer 每次启动即自哈希失败退出（fail-closed 设计行为正确），host 自愈重试每次再死。**代价**：RUN-CLI-6aa6a868 全程 229 次 `control_ticket_rejected`，模型被锁死在只读形态跑完 128 工具轮。处置：`orz-acaf-provision` 重算 manifest（旧件备份 `.bak-20260913`）；**候选动作**：发布清单加「二进制更新 ⇒ manifest 重算」核对步骤——待立案。

### F-003 | 2026-09-13 | RUN-CLI-6aa6a868 | 设计内门（0ac 正在修的病）| open → 0ac S3 验收样本
**确定性不可达无 cause**：229 次拒绝的失败载荷只有 `reject_code=signer_unreachable` + `io error: 管道正在被关闭 (os error 232)`，没有一条说明"manifest 哈希不匹配"或"env 缺失"——模型与排障者都只能盲猜，模型为此空转了全部只读轮次。这正是 0ac「机械层即时回报 + 单事件自描述」要治的形态；**处置**：作为 0ac S3 `cause` 字段的活体验收样本（修好后同形故障应一步报因）。

### F-004 | 2026-09-13 | RUN-CLI-6aa6a868 | 模型习惯（正面样本，登记不立案）
**模型行为正确的对照**：在"只能读、不能执行"的牢笼里，模型没有伪造进度，终稿如实声明「未落码、未跑门禁、未写入仓内文件」，并把可复核的设计 diff 全部列在会话里。与 F-003 合并看：机械层报因缺失时模型守住了诚实底线——归因纪律的正面证据。

### F-005 | 2026-09-13 | RUN-CLI-6aa6a868 / RUN-CLI-6aa6ac42 | 装置侧（观察）| open
**S2 产出核证全绿但有两处待复查**：① run3 仅 2 次 `content_anchor_mismatch` 编辑锚点重试（自恢复，代价小，暂不立案）；② S2 声称的"7 个 schema 文件 JSON 解析全通过"与"fixture 13+3 条"数目与实际提交（schema ×5、fixture 16）口径不一致——产物本身核验为真（门禁 `valid: true`），但自报计数不精确，**下轮自报要求附机械核对命令输出原文**。

### F-006 | 2026-09-13 | 传话流程 | 装置侧（流程）| fixed
**摩擦自报模板未随任务下发**：首轮传话按用户指示"仅告知"任务原话，run 末自报未发生，摩擦只能由主会话从 journal 反向提取。处置：本文件写入协议第 1 条已立；**下轮起传话模板固定附带自报指令**（见协议）。

### F-007 | 2026-09-13 | RUN-CLI-6aa6b63d | 设计内门（判据与既有契约互斥）| open
**0ac S3-a 法官规则按设计字面执法会误判既有合法语料**：按设计 §10.3 ③「`retrieval_family` 探针 run 起始一次（不多不少）」与 ④「`cause` 与 `failure_target` 的失败形状一致性」的字面落地，首版规则在既有合法形状上产出三类误判——① "缺探针=违规"会把既有 17 份 fixture journal **全部**判违规（生产者尚未落地）；② "`cause` 必须带 `failure_target`"与 S2 合法 fixture `runtime/fixtures/run-event-v0.2/payloads/tool-completed.cause.valid.json`（`exit_code=1` + `cause=channel_deadline_exceeded`，无 target）冲突；③ "`failure_target` 必须带 `cause`"与既有合法语料 `runtime/fixtures/run-event-v0.2/journals/local-browser-capability.jsonl` 第 14 行（`status=error` + `failure_target` + 无 cause）冲突。证据：三份 fixture 原文（本轮实读）。**代价**：发现经 3 次核对轮；因未注册进 `ALL_FAMILIES`，未污染任何门禁。处置：改为「探针 present ⇒ 校验（不多于一次/位置/读数完整），absent 不判」并挂开关 `REQUIRE_RETRIEVAL_FAMILY_PROBE=false`；新增语料回归测试把三类合法形状钉死（`s2_contract_fixtures_are_judged_as_the_contract_says`）。

### F-008 | 2026-09-13 | RUN-CLI-6aa6b63d | 装置侧（工具/工作区）| open
**`grep` 工具在本工作区返回空结果**：对 `D:\CLI\CLI_PROJECT_INDEX.md`（绝对路径）与 `docs`（相对路径）检索 `0ac` / `S3` / `FRICTION_LEDGER`，均返回 `no output (exit_code=0)`，尽管同文件同处存在大量匹配（同参数 `rg.exe` 与 PowerShell `Select-String` 立即命中）。证据：两次空返调用记录 + `Select-String` 命中行（前轮实做）；本轮再复现 1 次——对 `D:\CLI\orz\crates\orz-assurance\src\journal` 检索 `REQUIRE_RETRIEVAL_FAMILY_PROBE|pub mod immediate_feedback|verify_all_immediate_feedback` 同样空返，改 `rg.exe` 后 6 行全中（本轮实做），合计 3 次。**代价**：2 轮工具调用；`findstr` 输出为 GBK 乱码，另需 `[Console]::OutputEncoding=UTF8` 校正。处置：本轮改用 `B:\Zcode\resources\tools\ripgrep\rg.exe` + `Select-String`，事实留痕。
> **补注（2026-09-14，RUN-CLI-6aa6d379）**：根因候选已定——PATH 上 `rg` 首解曾是**悬空** WinGet 垫片（见 F-016）；修复后本轮 harness `grep` 工具 3 次调用全部正常命中（15/12/59 行）。F-008 三次复现与 F-013/F-015 的空返同属该窗，登记为**根因候选**（未做工具侧参数复刻，故不作终局定论）。

### F-009 | 2026-09-13 | RUN-CLI-6aa6b63d | 模型习惯（自报）| fixed
**S3-a 首版未过机械门就交付**：新建 `crates/orz-assurance/src/journal/immediate_feedback.rs` 首版带 1 个 dead-code 警告（`payload_int` 未被使用）与 2 处 rustfmt diff（`cargo fmt --check` 非 0 退出）。证据：`warning: function payload_int is never used --> ...immediate_feedback.rs:73`；`Diff in ...immediate_feedback.rs:134` / `:176`。**代价**：1 轮清理。处置：删除未用函数 + `cargo fmt -p orz-assurance`（`--check` 退出 0、全库 226 测全绿）——已修。

### F-010 | 2026-09-13 | RUN-CLI-6aa6b63d | 设计内门（S3 范围只落一块）| open
**S3 三块中本轮只闭合第 ② 块的 S3-a 切片**：设计 §10.3 = ① 生产者（本地分段检索前端/投递策略/M1–M3/探针扩面 + 开关 + A/B）、② 法官规则（五条）、③ 回归钉子。本轮落 ② 的五族 Rust 实现（`immediate_feedback`：`retrieval_dedupe` / `result_delivered_accounting` / `retrieval_family_probe` / `failure_cause_shape` / `first_result_deadline`）+ 7 项单测（含 fixture 与全语料扫查）；**未注册进 `ALL_FAMILIES`**（与现有 41 族 parity 交叉核对无交集），Python 法官镜像与两族清单同步未做；① 整块未动。证据：`git status --porcelain` 仅新增该文件；模块文档头显式标注未注册原因。处置：事实留痕；注册 + Python 镜像 + 生产者三件待放行。
> **补注（2026-09-14，RUN-CLI-6aa6d379）**：② 法官面五族已注册且本轮自测复绿（`orz-assurance --lib` 226/0）；③ 回归钉子的 3 例红随 G1 修复闭合；**① 生产者面**中「本地分段检索前端 + `cause` 自描述」已落（`ORZ_WEB_SEARCH_LOCAL` 默认关），**投递侧仍零写点** ⇒ 转 F-017 立案候选。

> **本轮机械核证留痕（RUN-CLI-6aa6b63d，输出摘录）**
> - `cargo test -p orz-assurance --lib immediate_feedback` → `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 219 filtered out; finished in 0.01s`（首次 6 项，补 fixture 测试后 7 项）。
> - `cargo test -p orz-assurance --lib` → `test result: ok. 226 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.08s`。
> - `cargo fmt -p orz-assurance -- --check` → `FMT_CHECK_EXIT=0`（清理前为 2 处 diff）。
> - 语料对照（`Select-String`）：`run-event-v0.2\journals\local-browser-capability.jsonl` 命中 2 行（第 13 行 `browser_launch_result` cause / 第 14 行 `tool_completed` status=error + failure_target + 无 cause）；全 17 份 journal 中 `failure_target` 仅出现 1 处。
> - `git -C orz status --porcelain` → `?? crates/orz-assurance/src/journal/immediate_feedback.rs` + `M crates/orz-assurance/src/journal/mod.rs`；`git diff --stat` → `1 file changed, 4 insertions(+)`（mod.rs 仅加文档条目与 `pub mod immediate_feedback;`）。
> - `rg.exe -n "REQUIRE_RETRIEVAL_FAMILY_PROBE|pub mod immediate_feedback|verify_all_immediate_feedback"`（journal/ 目录）→ 6 行命中：`immediate_feedback.rs:54: pub const REQUIRE_RETRIEVAL_FAMILY_PROBE: bool = false;`、`immediate_feedback.rs:183: if REQUIRE_RETRIEVAL_FAMILY_PROBE {`、`immediate_feedback.rs:398: pub fn verify_all_immediate_feedback(...)`、`immediate_feedback.rs:530: assert!(!REQUIRE_RETRIEVAL_FAMILY_PROBE);`、`mod.rs:38: pub mod immediate_feedback;` 等。

### F-011 | 2026-09-13 | RUN-CLI-6aa6bd3f | 装置侧（测试与事件面耦合）| fixed（本轮同批修）
**检索族探针新增的 run-start 事件直接撞红既有流式测试**：0ac S3① 落地后 `crates/orz-loop/src/host_exec.rs:4423` 的 `host_exec::tests::text_deltas_forwarded_in_order_before_model_output` 字面事件序列断言失败——期望序列比实际少一个 run-start 探针事件。证据（本轮实做原文）：`assertion left == right failed`，`left: [ToolAvailabilityCheck, ToolAvailabilityCheck, RunStarted, PromptSubmitted, RequestHeaderChange, ModelOutput, CounterexampleGate, ModelOutput, RunFinished]` / `right: [ToolAvailabilityCheck, RunStarted, …]`。**代价**：1 轮定位 + 1 轮改期望序列（同批机械动作）；改后 `cargo test -p orz-loop --lib` 全绿（771 passed; 0 failed; 3 ignored）。

### F-012 | 2026-09-13 | RUN-CLI-6aa6bd3f | 装置侧（宿主环境继承）| 观察
**宿主任 shell 常驻 `ORZ_MAX_WALLCLOCK=3600` 使测试红**：在 `ORZ_ACAF_FAIL_CLOSED=0` 已设的前提下，`cargo test -p orz-loop --lib` → `test result: FAILED. 770 passed; 1 failed; 3 ignored`（失败用例 `blackboard::tests::blackboard_read_serves_session_section`）；两次运行唯一环境差为清空 `ORZ_MAX_WALLCLOCK`，清空后同一命令 → `test result: ok. 771 passed; 0 failed`，其间未改任何代码。证据：`Get-ChildItem Env:` 原样输出（另有 `ORZ_ACAF_BINARY`/`ORZ_ACAF_KEYSTORE`/`ORZ_ACAF_MANIFEST`/`ORZ_ALLOW_*`/`ORZ_REAL`/`ORZ_DEEPSEEK_API_KEY` 常驻）+ 两份测试摘要；未读该用例的断言差文本。**代价**：1 轮重跑。
> **补注（2026-09-14，RUN-CLI-6aa6d379）**：**已修**——会话面用例的期望墙钟改为与产品**同源解析**（`controller::main_wallclock_limit_secs_override()`），不再依赖「宿主未设该变量」的隐式前提。核证：常驻 `ORZ_MAX_WALLCLOCK=3600` 下单例 `ok`（`1 passed; 0 failed`）、全量 `orz-loop --lib` `771 passed; 0 failed; 3 ignored`；清空变量后单例同样 `ok`（等价对照）。落码 orz `96d2b263`。

### F-013 | 2026-09-13 | RUN-CLI-6aa6bd3f | 装置侧（工具：`grep` 空返，F-008 第 4 次复现）| open（根因未定）
**`grep` 工具在本工作区再次空返，本轮未修**：对 `D:\CLI\docs`（其中 `FRICTION_LEDGER.md` 确有 3 处 `F-007`）以 `pattern=F-007` 调用 → `tool 'grep' completed with no output (exit_code=Some(0))`；紧邻的 `B:\Zcode\resources\tools\ripgrep\rg.exe -n --no-heading 'F-007' docs` 同工作区立即命中。定位进展（本轮事实）：① 工具描述串「Search file contents with regular expressions (ripgrep).」全仓唯一命中 `crates/codegen/orz-tools/src/implementations/grok_build/grep/mod.rs:269`；② rg 调用点 `mod.rs:1049` `rg_path()` → `:1051` `Command::new(rg_exec)`，参数形状 `--heading --with-filename --line-number --color=never --max-columns 1000 --max-columns-preview [-l|-c] -e <pattern> <workdir> --max-filesize 5M`（`:1052`–`:1125`），stdout/stderr 皆 pipe（`:1126`）+ `crate::util::detach_command`（`:1128`）+ `stdin(Stdio::null())`（`:1129`）；③ `rg_path()` 解析在 `grep/ripgrep.rs`（`bundle_rg` 分支落 `~/.grok/vendor/`，非 bundle 分支先读 `RG_BIN_PATH`，否则视为 PATH 上的 `rg`）。未复刻同一参数形状以区分「rg 真无命中」与「wrapper 丢输出」——根因未定，按「如定位简单才一并修」的限定条件本轮未改代码、未留钉子。**代价**：本轮 5 次工具调用（1 复现 + 4 定位）后让位于收尾（账本 + 提交）。
> **补注（2026-09-14，RUN-CLI-6aa6d379）**：根因候选 = PATH 上 `rg` 首解为悬空 WinGet 垫片（F-016，已修）；修后 harness `grep` 工具 3/3 正常命中。**修复是绕行**（PATH 前置真 rg），垫片本体仍悬空 ⇒ 残留见 F-019。

> **本轮机械核证留痕（RUN-CLI-6aa6bd3f，收尾段实做输出摘录）**
> - `cargo test -p orz-loop --lib`（`ORZ_ACAF_FAIL_CLOSED=0` 且清空 `ORZ_MAX_WALLCLOCK`）→ `test result: ok. 771 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 9.96s`。
> - 同上但保留 `ORZ_MAX_WALLCLOCK=3600` → `test result: FAILED. 770 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 9.94s`（失败用例 `blackboard::tests::blackboard_read_serves_session_section`）。
> - 探针面修复前原文：`crates\orz-loop\src\host_exec.rs:4423:9: assertion left == right failed`（左含两个 `ToolAvailabilityCheck`，右仅一个）。
> - `grep`（`path=D:\CLI\docs`, `pattern=F-007`）→ `tool 'grep' completed with no output (exit_code=Some(0))`；`rg.exe -n --no-heading 'F-007' docs\FRICTION_LEDGER.md` → `13:### F-007 | …` 等命中。
> - `git -C orz status --porcelain` → 12 个 `M` + 1 个 `?? crates/codegen/orz-tools/src/implementations/web_search/local_segmented.rs`；`git -C orz diff --stat` → `12 files changed, 520 insertions(+), 57 deletions(-)`。
> - 父仓 `git status --porcelain` → `M assurance/run_event_journal_validation.py` + `M orz`；`git diff --stat` → `2 files changed, 250 insertions(+)`。
> - 0ac S3①② 建造段（已折叠）的逐条核证命令与运行结果存档于 `D:\CLI\.gsa\ledger\current.md`（按行检索）。

### F-014 | 2026-09-14 | RUN-CLI-6aa6c9b3 | 模型习惯（落码交付门；F-009 同族第 2 次）| open（缺口 G1/G2，修复批次待裁决）
**0ac S3①② 落码批（orz `4c892951`，当前 HEAD）带 3 例自测红与格式门未过**：① 新文件 `crates/codegen/orz-tools/src/implementations/web_search/local_segmented.rs:339:30` 按字节切片解码 HTML 实体造成字符边界 panic——`cargo test -p orz-tools --lib local_segmented` 实测 `6 passed; 3 failed`（红：`parse_bing_serp_reads_current_structure_and_skips_ads` / `search_returns_hits_with_stable_shape` / `empty_serp_is_empty_result_and_chain_falls_through`），panic 原文 `end byte index 12 is not a char boundary; it is inside '方' (bytes 11..14 of string)`；② `cargo fmt --all -- --check` 退出 1、16 处 diff（4 个本批文件：`local_segmented.rs`×9 / `host_exec.rs`×2 / `retrieval/projection.rs`×3 / `tool_probe.rs`×2）；③ `cargo clippy -p orz-tools --lib` 本批新文件 2 处风格警告（`:110/:111`）。**代价**：开关 `ORZ_WEB_SEARCH_LOCAL` 默认关是唯一护栏——打开前本地检索路径不可实机复验（dev/release 均 `panic="abort"`，同输入在生产形态为进程中止）；审记轮取证 1 段测试跑批。**处置**：按用户 2026-09-14 边界只登记不动手——缺口 G1（P0）/G2（P1）见审计文档 §3.1/§3.2 与 §6；修复批次划分待裁决。
> **补注（2026-09-14，RUN-CLI-6aa6d379）**：**G1/G2 已修并核证**（orz `96d2b263`）：G1 = `decode_html_entities` 去字节切片（整体 `find(';')` + 窗口谓词）＋ 命名实体 ＋ 钉子 `decode_html_entities_keeps_multibyte_window_boundary_intact`（`local_segmented` 6/3 → 10/0）；G2 = `fmt --check` 16 处 → 0、新文件 clippy 2 处 → 0（`orz-tools --lib` 2819/49 → 2869/0）。**G3 维持 open** ⇒ 转 F-017。报告：[`0AC_S3_FIX_REPORT_2026-09-14`](audits/0AC_S3_FIX_REPORT_2026-09-14.md)。

### F-015 | 2026-09-14 | RUN-CLI-6aa6c9b3 | 装置侧（工具：`grep` 空返，F-008 第 5 次复现）| open（根因未定）
**`grep` 工具继续空返**：本会话审记段复现 3 次（审计文档 §5 已注），收尾段再复现 3 次（本轮实做，原文）：`pattern=RUN-CLI-`（`D:\CLI\docs\FRICTION_LEDGER.md`）、`pattern=F-01[0-9]|run|Run|RUN`（同文件）、`pattern=0ac|GAP-MECH-IMMEDIATE-FEEDBACK|IMMEDIATE_RESULT`（`D:\CLI\CLI_PROJECT_INDEX.md`）三次均返回 `tool 'grep' completed with no output (exit_code=Some(0))`；三份目标该时刻确有大量命中（同刻 `Select-String` 命中 5 行、`rg.exe` 命中即返回）。**代价**：审记全段检索改道 `rg.exe`（`B:\Zcode\resources\tools\ripgrep\rg.exe`）+ `Select-String`，每轮额外 1–2 次工具调用。**处置**：事实留痕；根因线索见 F-013（未复刻参数形状以区分 wrapper 丢输出）。
> **补注（2026-09-14，RUN-CLI-6aa6d379）**：**本轮 0 复现**——修复 PATH `rg` 解析（F-016）后 harness `grep` 工具 3 次调用全部正常命中（`D:\CLI\.gsa\ledger\current.md` 12/59 行、`D:\CLI\docs\FRICTION_LEDGER.md` 15 行）。与 F-008/F-013 合并看，F-008 族（共 5 次空返）的**根因候选 = PATH 上 `rg` 首解为悬空 WinGet 垫片**；因未做工具侧参数复刻，保留「候选」口径。

> **本轮机械核证留痕（RUN-CLI-6aa6c9b3，输出摘录）**
> - `cargo test -p orz-tools --lib` → `test result: FAILED. 2819 passed; 49 failed; 6 ignored; 0 measured; 0 filtered out; finished in 32.57s`；`cargo test -p orz-tools --lib local_segmented` → `test result: FAILED. 6 passed; 3 failed; 0 ignored; 2865 filtered out; finished in 0.10s`，panic 原文 `panicked at crates\codegen\orz-tools\src\implementations\web_search\local_segmented.rs:339:30: end byte index 12 is not a char boundary; it is inside '方' (bytes 11..14 of string)`。
> - `Select-String -Path Cargo.toml -Pattern "panic"` → `Cargo.toml:311:panic = "abort"`（release）/ `:352:panic = "abort"`（dev）。
> - `cargo fmt --all -- --check` → `FMT_EXIT=1`，16 处 diff（分布同上）；`cargo clippy -p orz-tools --lib --message-format short` → `--> ...local_segmented.rs:110:9` / `:111:9`（本批新文件 2 处；无 `error:` 行）。
> - `cargo test -p orz-assurance --lib` → `226 passed; 0 failed; 0 measured`；`cargo test -p orz-loop --lib`（清常驻 `ORZ_MAX_WALLCLOCK` 后）→ `771 passed; 0 failed; 3 ignored`，不清空 → `770 passed; 1 failed`（`blackboard::tests::blackboard_read_serves_session_section`，F-012 同形）。
> - `python scripts/check_repository.py` → `"error_count": 0` / `"valid": true`（EXIT=0）。
> - `rg -n "retrieval_progress|retrieval_result_segment|result_delivered" crates --glob "*.rs"` → 产品码零命中（仅 `orz-assurance` 法官读取面）；`rg -n "ResultDelivered|RetrievalProgress|RetrievalResultSegment"` → 零命中。
> - `grep`（工具）空返原文见 F-015；对照：`Select-String -Path CLI_PROJECT_INDEX.md -Pattern "0ac"` → 5 行命中（L3/L5/L8/L59/L331，L331 = `GAP-MECH-IMMEDIATE-FEEDBACK` 路由行）、`rg.exe -n "RUN-CLI" D:\CLI\.gsa\ledger\current.md` → 命中 8 行（L35/L108/L110/L116–L120/L147）。
> - `git -C orz log --oneline -3` → `4c892951`（S3①/②）/ `ac5d6375`（S3-a）/ `ea777918`；`git -C orz status --porcelain` → 空（工作区干净）；父仓 `git status --short` → 仅 `?? docs/audits/0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md`。

### F-016 | 2026-09-14 | RUN-CLI-6aa6d379 | 装置侧（工具/工作区：PATH 上 `rg` 解析）| fixed（绕行）
**PATH 上 `rg` 首解曾是悬空 WinGet 垫片**：`where rg` 三个候选中 `C:\Users\1\AppData\Local\Microsoft\WinGet\Links\rg.exe` 为**符号链接**（`Length=0`，写于 2026-08-04），Target = `…\WinGet\Packages\BurntSushi.ripgrep.MSVC_…\ripgrep-15.2.0-x86_64-pc-windows-msvc\rg.exe`，而该**包目录缺失**（`PKG_DIR_MISSING`）；对该路径 spawn 报 `程序"rg.exe"无法运行: No application is associated with the specified file for this operation`。**代价**：修前 `cargo test -p orz-tools --lib` = `2819 passed; 49 failed`（其中 3 例 = 审记 G1，余 **46 例 = spawn 面**：审记 §3.4 点名 `grok_build::grep` 20 / `opencode::glob` 13 / `opencode::grep` 13）；同窗 harness `grep` 工具 5 次空返（F-008/F-013/F-015）。**复现实验（本轮实做，不改任何代码，仅把坏垫片目录前置到 PATH）**：全量 `FAILED. 2822 passed; 47 failed`（对照正常 PATH `2869 passed; 0 failed`）；`grok_build::grep::tests` `FAILED. 29 passed; 20 failed`（对照 `49 passed; 0 failed`）。**修复（绕行，01:00）**：在 PATH 更早位置补入真 ripgrep `C:\Users\1\.local\bin\rg.exe`（14.1.1，5,400,984 B）⇒ `where rg` 首解改为该项；`orz-tools --lib` 同批 **0 failed**（其间代码改动仅 G1/fmt/F-012，均不触及 spawn 面）。**残留**：垫片本体未修 ⇒ F-019。

### F-017 | 2026-09-14 | RUN-CLI-6aa6d379 | 设计内门（S3 范围）| open（立案候选）
**S3① 投递侧未落 = 审记 G3 维持 open**：本轮复核 `retrieval_progress|retrieval_result_segment|result_delivered` 全 crates **29 命中全部在 `orz-assurance`**（`journal/families.rs` / `immediate_feedback.rs` / `mod.rs`），`ResultDelivered|RetrievalProgress|RetrievalResultSegment` **0 命中**（`EventType` 无变体）⇒ 三事件**无产品码写点**；I1–I3 / M1–M3 / 子代理提前收口 / semaphore 截止（TODO ⑤⑥⑦）本批无落码。**代价**：设计 §10.3 的「①生产者②法官③钉子」与「S3 已落」并列时易被读成已闭合（审记 §3.3 即为此而立）。处置：不动手（结构性）⇒ 立案候选，待裁决是否拆 `S3①-a 检索侧（已落）/ S3①-b 投递侧（未落）`。入口：审记 §3.3/§6-G3/§8、设计稿 §10.5-3。
> **补注（2026-09-14，用户裁决）**：**采纳拆分**——S3① = ①-a 检索侧（已落，orz `4c892951`+`96d2b263`）/ **①-b 投递侧（未落，下一实现批次**：三事件 EventType 变体+写点、I1–I3、M1–M3（M2 先行、M1 带开关+A/B）、子代理提前收口、semaphore 截止**）**。TODO P0-0ac 与 BACKLOG 0ac 已同步裁决行。本条转**已裁决**，实现完成前 0ac 维持 open。

### F-018 | 2026-09-14 | RUN-CLI-6aa6d379 | 装置侧（文档与实现漂移）| fixed
**审记把开关当前态记为 `false`，实为已移除**：审记 §1/§2.3/§6-③ 按 `ac5d6375` 时刻写「`REQUIRE_RETRIEVAL_FAMILY_PROBE=false` 开关（F-007 裁决(a)）」，但 `4c892951` 已随宽口径落地**删除该开关与执法分支**；本轮实查全 crates 仅剩 **2 处文档注释**提到该名（`crates\orz-assurance\src\journal\immediate_feedback.rs:41` / `:50`）。**代价**：审记 §6 裁决点③「是否翻转（当前 false）」提给用户时**失去对象**（1 轮复核，0 次实际决策成本）。处置：本轮闭合——审记新增 §8 勘误（连带 §2.3 口径注记 A 的「文档口径滞后，P3」）＋ 设计稿新增 §10.5 回写；本条留痕。

### F-019 | 2026-09-14 | RUN-CLI-6aa6d379 | 装置侧（观察）| open
**WinGet `rg` 垫片本体仍悬空**：F-016 的修复是**绕行**（PATH 前置真 rg），`C:\Users\1\AppData\Local\Microsoft\WinGet\Links\rg.exe` 仍是指向缺失包目录的符号链接（`Length=0`，Target 不存在）。**风险形态（事实）**：任何不继承该 PATH 顺序、或 PATH 被重排/改写的进程仍会解析到坏垫片，spawn 面红与 `grep` 空返同形复发。处置：观察（修需重装 `BurntSushi.ripgrep.MSVC` 包，属用户环境面，本轮未动）。

### F-020 | 2026-09-14 | RUN-CLI-6aa6d379 | 装置侧（文档回写）| fixed
**修复报告的部分回写条目先于动作写成「已办」，收尾核证发现未落地**：`0AC_S3_FIX_REPORT_2026-09-14` §6 把索引三项回写（`GAP-MECH-IMMEDIATE-FEEDBACK` 路由行 / 本报告路由 / 摩擦台账路由行）与 §0「父仓提交见 §6」记作已办；收尾核证时 `git -C D:\CLI diff --stat` 显示 `CLI_PROJECT_INDEX.md` 实改仅 **1 行**（版本头，`0AC_S3_FIX_REPORT` 索引内命中 1 处），路由行与台账行未动、§6 无父仓提交行。**代价**：收尾核证 1 轮（diff / Select-String 对照即发现）。**处置**：收尾补齐索引 L331 路由行（G1/G2 已修 + G3 维持 open + 修复报告入口）、L128 台账行注记、§6 父仓提交行（哈希回写）；本条留痕。

> **本轮机械核证留痕（RUN-CLI-6aa6d379，修复轮收尾段实做输出摘录）**
> - `cargo fmt --all -- --check` → `FMT_CHECK_EXIT=0`（修前 `FMT_EXIT=1`，16 处 diff）。
> - `cargo clippy -p orz-tools --lib --message-format short` → 过滤 `local_segmented` **0 行**（修前 `:110:9` / `:111:9`）；全量 `orz-tools (lib) generated 3 warnings`（既有文件）＋ `xai-tty-utils (lib) generated 2 warnings`（依赖 crate）。
> - `cargo test -p orz-tools --lib` → `test result: ok. 2869 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 44.98s`（修前 `FAILED. 2819 passed; 49 failed; 6 ignored`）。
> - `cargo test -p orz-tools --lib local_segmented` → `ok. 10 passed; 0 failed`（修前 `6 passed; 3 failed`）；新钉子 `decode_html_entities_keeps_multibyte_window_boundary_intact ... ok`。
> - `$env:ORZ_MAX_WALLCLOCK="3600"; $env:ORZ_ACAF_FAIL_CLOSED="0"; cargo test -p orz-loop --lib blackboard::tests::blackboard_read_serves_session_section -- --exact` → `ok. 1 passed; 0 failed; 773 filtered out`（修前同场景 `FAILED. 770 passed; 1 failed; 3 ignored`）；全量 `cargo test -p orz-loop --lib` → `ok. 771 passed; 0 failed; 3 ignored; finished in 10.15s`。
> - `cargo test -p orz-assurance --lib` → `ok. 226 passed; 0 failed`.
> - F-016 复现实验（PATH 前置 `C:\Users\1\AppData\Local\Microsoft\WinGet\Links`）→ 全量 `FAILED. 2822 passed; 47 failed; finished in 45.18s`；`grok_build::grep::tests` → `FAILED. 29 passed; 20 failed`；正常 PATH 同模块对照 → `ok. 49 passed; 0 failed`。
> - `where rg` → `C:\Users\1\.local\bin\rg.exe` | `…\WinGet\Links\rg.exe` | `B:\Zcode\resources\tools\ripgrep\rg.exe`；`rg --version` → `ripgrep 14.1.1 (rev 4649aa9700)`；坏垫片直跑 → `程序"rg.exe"无法运行: No application is associated with the specified file for this operation`；`dir` 显示垫片 `Length=0`、Target 指向缺失包目录（`PKG_DIR_MISSING`）。
> - 检索面复核（G3）：`rg -n "retrieval_progress|retrieval_result_segment|result_delivered" crates --glob "*.rs"` → **29 命中**（三文件全在 `orz-assurance`）；`rg -n "ResultDelivered|RetrievalProgress|RetrievalResultSegment"` → **0 命中**；`rg -n "REQUIRE_RETRIEVAL_FAMILY_PROBE"` → **2 命中**（`immediate_feedback.rs:41/:50`，均为文档注释）。
> - harness `grep` 工具（本轮 3 次）→ 正常命中（`D:\CLI\.gsa\ledger\current.md` 12 行 / 59 行、`D:\CLI\docs\FRICTION_LEDGER.md` 15 行）；对照 F-008/F-013/F-015 共 5 次空返。
> - `python scripts/generate_orz_source_manifest.py` → `wrote 1448 entries`，差异面 5 行（本批 5 文件）；`python scripts/check_repository.py` → `"error_count": 0` / `"valid": true`（EXIT=0）。
> - `git -C orz log --oneline -1` → `96d2b263 fix(0ac S3): 审记 G1/G2 修复…`；`git -C orz status --porcelain` → 空；`git -C orz show --stat HEAD` → `5 files changed, 96 insertions(+), 49 deletions(-)`。
> - 收尾补记（父仓核证）：`python scripts/check_repository.py` → `"error_count": 0` / `"valid": true`（EXIT=0，含本批全部文档改动）；父仓账本提交 `57214be0`（`账本: 0ac S3 修复批落档…`，9 files +229/−10；**未推送**），本次哈希回写为其后一条补记。

### F-021 | 2026-09-14 | RUN-CLI-6aa77e19 | 装置侧（外部网络面：Docker 镜像拉取）| 观察
**`docker pull rust:1.97-slim` 失败**：报错原文 `Error response from daemon: failed to resolve reference "docker.io/library/rust:1.97-slim" … dialing registry-1.docker.io:443 … because Docker Desktop has no HTTPS proxy … host has failed to respond`（`evidence-0ac-s3-20260914\docker-pull-rust.log`）。事实：本机 Docker Desktop 未配 HTTPS 代理，`registry-1.docker.io:443` 直连无响应。**影响面**：本轮 Linux musl 构建的常规镜像获取路径不可用；**未阻塞**——本地既有 `rust:1.97-slim`（1.27 GB，5 周前拉取）支撑全部容器构建（`Finished release … in 42m 05s`）。**代价**：1 次拉取尝试（至超时报错）。**处置**：观察（未改动本机 Docker 网络配置）。

### F-022 | 2026-09-14 | RUN-CLI-6aa77e19 | 装置侧（容器环境：Docker daemon 冷机）| fixed（重试）
**容器构建首轮 `BUILD_EXIT=1`**：13:30 起的构建输出 `failed to connect to the docker API at npipe:////./pipe/dockerDesktopLinuxEngine … the daemon is not running`（`D:\tb-eval\orz-linux\build-20260914-1330.log`）。**处置**：启动 Docker Desktop 后 13:40 重试，进入容器并完成构建（`build-20260914-1340.log`；终态 `linux-build-container-full.log` L1788 `Finished release [optimized] target(s) in 42m 05s`）。**同形前例**：2026-09-13 0z S3 载体构建同形（其审计 §2 操作记录）。**代价**：约 10 min 墙钟（13:30 失败 → 13:40 重试）。

### F-023 | 2026-09-14 | RUN-CLI-6aa77e19 | 装置侧（文件占用：载体目录内运行中进程持旧镜像）| fixed（绕行）
**载体内原位替换不可行**：`D:\tb-eval\orz-windows` 内运行中宿主进程持有旧 `orz.exe` / `orz-signer.exe` 映像，原位覆盖路径不可用；实际换装记录为 `fresh-copy-to-tmp -> rename old to *.0.5.0-bak -> move tmp into place`（`windows-carrier-sync-0.5.1.txt` 的 `swap method=` 行）。**结果**：post-swap 三件与 staging 逐对 `MATCH=True` ×3；留档 `orz.exe.0.5.0-bak` / `orz-signer.exe.0.5.0-bak` 两件实物。**代价**：换装由 1 步变 3 步（多 1 次 rename + 1 次留档），无额外失败重试。

### F-024 | 2026-09-14 | RUN-CLI-6aa77e19 | 模型习惯（自报：写运行时受管路径）| 登记（无修复动作）
**收尾核证开始时对运行时会话卷发起 1 次编辑**：目标 `D:\CLI\.gsa\ledger\current.md`，`search_replace` 被拒，工具原文 `Error: D:\CLI\.gsa\ledger\current.md is inside the runtime-owned .gsa session volume, which is not model-writable.`。**事实**：该调用零写入、目标文件内容与 mtime 未变（其后读取正常）；**代价**：1 次工具调用。**面向澄清（事实）**：本轮自报写入面是本台账 `docs/FRICTION_LEDGER.md`；`.gsa/ledger/current.md` 是历史折叠存档（只读）。提交与推送随本批账本处理（F-021…F-024 同批，回执见 `git log`）。

> **本批机械核证留痕（RUN-CLI-6aa77e19，收尾核证段实做输出摘录）**
> - `gh release view v0.5.1 --repo SilverWhite/CLI --json tagName,name,isDraft,isPrerelease,publishedAt,url,assets --jq '{…}'` → `{"assets":[{"digest":"sha256:49df6cebbce12375cb2969f2bed03dfdfc8f4b02f5b89764f6475fb05ce7cdf0","name":"orz-0.5.1-linux-x86_64.tar.gz","size":34987981,"state":"uploaded"},{"digest":"sha256:f7e8274c706c9b0cf202ccc126d220e4719097bf98847fe3478c6b368689432f","name":"orz-0.5.1-windows-x86_64.zip","size":26472460,"state":"uploaded"}],"draft":false,"name":"orz 0.5.1（0ac S3 载体重建）","pre":false,"published":"2026-09-14T07:32:43Z","tag":"v0.5.1","url":"https://github.com/SilverWhite/CLI/releases/tag/v0.5.1"}`。
> - `git -C D:\CLI\orz ls-remote cli refs/heads/feat/fusion-architecture` → `dbb42b1d0ac5be56e68aa4e5ca307ff168f465a6`；`git -C D:\CLI\orz rev-parse HEAD` → 同值（本地 = 远端）。`git -C D:\CLI\orz log --oneline -3` → `dbb42b1d chore(release): 版本 bump 0.5.0 → 0.5.1…` / `96d2b263 fix(0ac S3): 审记 G1/G2 修复…` / `4c892951 0ac S3①/②…`。
> - `git -C D:\CLI rev-parse origin/main` → `c4491629b2abd65bddd9295493c862781cabb31b`；`git -C D:\CLI log --oneline -1 origin/main` → `c4491629 chore(submodule): orz 指针 -> dbb42b1d …`（C1 已在远端）。
> - `Get-FileHash …\staging-0ac-s3-20260914\orz-0.5.1-linux-x86_64.tar.gz -Algorithm SHA256` → `49DF6CEBBCE12375CB2969F2BED03DFDFC8F4B02F5B89764F6475FB05CE7CDF0`（34,987,981 B）；同法 zip → `F7E8274C706C9B0CF202CCC126D220E4719097BF98847FE3478C6B368689432F`（26,472,460 B）＝ GitHub 服务端 digest 逐字一致。
> - `git -C D:\CLI status --short` → `M CLI_PROJECT_INDEX.md` / `M TODO.md` / `?? docs/audits/0AC_S3_DUAL_PLATFORM_REBUILD_2026-09-14.md` / `?? releases/orz-0.5.1-x86_64/`（C2 回写面；收尾段核对）。
> - 收尾补记（父仓核证）：`git -C D:\CLI push origin main` → `c4491629..8c7529ee main -> main`；**C2 = `8c7529ee`**（6 files +316/−5，含本台账 F-021…F-023）；`python scripts/check_repository.py` → `"error_count": 0` / `"valid": true`（EXIT=0）；`git -C D:\CLI log --oneline -1 origin/main` → `8c7529ee 账本: 0ac S3 载体重建与发布落档…`。

### F-025 | 2026-09-14 | RUN-CLI-6aa7bee3 | 装置侧（网络面：GitHub 推送瞬时 TLS 失败）| 重试成功
**实现提交后首次 `git push cli HEAD` 失败**：输出原文 `fatal: unable to access 'https://github.com/SilverWhite/CLI.git/': TLS connect error: error:00000000:lib(0)::reason(0)`（`D:\CLI\.gsa\session\terminal\call_00_UNEYgRl7XTRKx4QyVLTM6886.log`）。**事实**：同一命令第二次执行成功——`dbb42b1d..f03b2a4f  HEAD -> feat/fusion-architecture`，远端 `git ls-remote cli refs/heads/feat/fusion-architecture` 回 `f03b2a4f463827d4d8004c3a15b03991375fc0b6`。**代价**：1 次推送重试。**处置**：重试（无配置改动）。

### F-026 | 2026-09-14 | RUN-CLI-6aa7bee3 | 装置侧（检索工具：PowerShell 不展开 glob）| 观察
**两次 `rg` 调用因把 glob 当路径传入而报错**：`rg ... D:\CLI\orz\crates\orz-loop\src\*.rs` → `文件名、目录名或卷标语法不正确。 (os error 123)`；`rg -F "web_search" *.rs`（cwd 内）→ 同类 IO error。**事实**：改传目录 + `--glob "*.rs"`（或直接传单文件路径）后立即命中。**代价**：2 次工具调用往返。**处置**：观察（本轮内已改用手法；不改装置）。

### F-027 | 2026-09-14 | RUN-CLI-6aa7bee3 | 模型习惯（自报：测试里改进程 env）| 已修（本轮内）
**首轮 `cargo test -p orz-loop immediate_delivery` 编译失败**：`error[E0133]: call to unsafe function std::env::set_var/remove_var ... requires unsafe block`（5 处，crate 为 edition 2024）。**事实**：删除 env 变更测试、抽出纯函数 `switch_truthy(&str)` 后再跑 → `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 774 filtered out; finished in 0.00s`。**代价**：1 轮编译往返（`Finished test profile ... in 22.89s`）。**处置**：已修（契约改由纯函数单测钉住 + `switch_enabled()` 默认关断言）。

### F-028 | 2026-09-14 | RUN-CLI-6aa7bee3 | 装置侧（行显示：源文件裸 CR 致 PowerShell 吞行）| 观察
**`Get-Content | Select-Object -Skip N` 的显示行号与 rg 行号漂移 13 行**：同一文件同一位置，PowerShell 显示 `3528: domain,`，而 `rg -n -B 16 -A 16 "host\.call_tool_with_timeout" host_exec.rs` 显示 `3528: host.call_tool_with_timeout(...)`。**事实**：该文件部分注释行含裸 `\r`（mojibake 注释），控制台把两条源行合并成一行显示（如 `3510:` 一行里同时出现 `目。` 与 `let (round, domain) = ...`），故按显示行号定位不可靠；rg 带上下文的行号与真实文件一致（编辑锚点最终以 rg 上下文 + 文本锚点为准，两次 `search_replace` 均一次命中）。**代价**：1 次额外定位往返。**处置**：观察（不改装置）。

> **本批机械核证留痕（RUN-CLI-6aa7bee3，0ac S3①-b 落产品码写点段实做输出摘录）**
> - `cargo test -p orz-loop immediate_delivery` → `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 774 filtered out; finished in 0.00s`（5 单测：渲染无头拒收 / 渲染头+逐项逐段解析 / 三 payload 字段shape / 稳定码不猜 / 开关默认关）。
> - S2 schema 交叉核证（python + jsonschema，`runtime/*.schema.json` 三件）→ `segment required_ok no_extra` / `segment jsonschema: PASS`、`progress required_ok no_extra` / `progress jsonschema: PASS`、`delivered required_ok no_extra` / `delivered jsonschema: PASS`。
> - `git -C D:\CLI\orz show --stat HEAD` → `crates/orz-assurance/src/journal/event.rs | 14 ++` / `crates/orz-loop/src/host_exec.rs | 95 ++++++++` / `crates/orz-loop/src/immediate_delivery.rs | 366 ++++` / `crates/orz-loop/src/lib.rs | 1 +`，`4 files changed, 476 insertions(+)`；`git -C D:\CLI\orz log --oneline -1` → `f03b2a4f feat(0ac S3-b): immediate-feedback arrival/delivery write points (retrieval_progress / retrieval_result_segment / result_delivered)`；`git -C D:\CLI\orz status --porcelain` 提交前 = `M crates/orz-assurance/src/journal/event.rs` / `M crates/orz-loop/src/host_exec.rs` / `M crates/orz-loop/src/lib.rs` / `?? crates/orz-loop/src/immediate_delivery.rs`，提交后为空。
> - 推送核证：`git push cli HEAD`（第二次）→ `dbb42b1d..f03b2a4f  HEAD -> feat/fusion-architecture`；`git ls-remote cli refs/heads/feat/fusion-architecture` → `f03b2a4f463827d4d8004c3a15b03991375fc0b6`（= 本地 `rev-parse HEAD`）。
> - 写点定位核证（rg 上下文）→ `3527: let call =` / `3528: host.call_tool_with_timeout(&tc.name, tc.arguments.clone(), &tc.call_id, timeout);` / `3540: let (mut result, succeeded) = match call_result {`（到达面/交付面写点插于 3527 之前与 3540 之前）。
> - 本轮**未做**：全量 `cargo test -p orz-loop`、`cargo fmt/clippy` 全量、载体重建（用户指示「暂时不做重建」）；S3①-b 的失败面（`stable_code`）无真实网络失败样本核证（`stable_code_from_error` 仅单测钉住契约）。

### F-030 | 2026-09-14 | RUN-CLI-6aa7bee3 | 模型习惯（交付门：父仓推送未跑门禁，F-009/F-014/F-020 同族第 4 次）| fixed（主会话对账补办）
orz 推送父仓提交 `f51a9d22` 前未重算 `orz_source_manifest.sha256`、未跑 `check_repository.py`，且终答核证清单未声明跳过。主会话对账实测：`check_repository.py` → `"error_count": 4, "valid": false`（`event.rs`/`host_exec.rs`/`lib.rs` 3 处摘要失配 + `immediate_delivery.rs` 未入册）。**代价**：门禁失效窗口 = `f51a9d22` 推出后至补办前；1 次补办批次。**处置（主会话）**：`generate_orz_source_manifest.py` → 1449 条；复跑门禁 → `"error_count": 0` / `"valid": true`。
> **补注（2026-09-14，用户裁决）**：不立案；**归因存疑**——用户指出该 run 设了墙钟 7200 s，不能排除 orz 估时后主动取舍（不跑全盘测试故未重算 manifest），非纯模型习惯缺陷；此后非跑分任务不设墙钟（2026-09-14 用户口径），该变量已消除。

## 统计

| 日期 | run | 摩擦条目 | 立案候选 | 已修 | 观察 |
|---|---|---|---|---|---|
| 2026-09-13 | RUN-CLI-6aa6a868 / 6aa6ac42 | F-001…F-006 | F-001②/F-002②（待立案） | F-001①/F-002①/F-006 | F-003（并入 0ac）/F-004/F-005 |
| 2026-09-13 | RUN-CLI-6aa6b63d（0ac S3-a 落码） | F-007…F-010 | F-007（判据口径待裁决）/F-008 | F-009 | F-007/F-008/F-010 |
| 2026-09-13 | RUN-CLI-6aa6bd3f（0ac S3② 裁决(a) 落地 / S3① 生产者面） | F-011…F-013 | F-013（根因待定，待立案） | F-011/F-012 | F-007 已按裁决(a) 翻宽口径落地；F-008 第 4 次复现未修 |
| 2026-09-14 | RUN-CLI-6aa6c9b3（0ac S3 实现审记） | F-014…F-015 | F-014（G1–G3 修复批次划分待裁决）/F-015（F-008 同族，根因待定） | — | F-008 第 5 次复现；G1–G3 详见 [0ac S3 审记](audits/0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md) |
| 2026-09-14 | RUN-CLI-6aa6d379（0ac S3 修复批） | F-016…F-020 | F-017（S3① 投递侧未落；拆子阶段待裁决） | F-016（绕行）/F-018/F-020（收尾补齐）；F-012、F-014 的 G1/G2 同批翻 fixed（见其补注） | F-019（WinGet 垫片本体仍悬空）；F-008/013/015 根因候选；G3 与核证详见 [修复报告](audits/0AC_S3_FIX_REPORT_2026-09-14.md) |
| 2026-09-14 | RUN-CLI-6aa7bee3（0ac S3①-b 落产品码写点） | F-025…F-028、F-030（F-029 编号留空：装置侧事项，2026-09-14 口径收窄移出台账） | —（均不立案） | F-027（本轮内纯函数化）/F-030（门禁由主会话补办复绿） | F-025（push 首次 TLS 失败，重试成功；外部网络面）/F-026（PowerShell 不展开 glob）/F-028（裸 CR 注释致显示吞行）；F-019（WinGet 垫片本体）仍悬空 |
| 2026-09-14 | RUN-CLI-6aa77e19（0ac S3 载体重建与发布） | F-021…F-024 | —（本批无新立案） | F-022（Docker daemon 冷机，启动后重试成功）/F-023（载体换装改 rename 绕行） | F-021（Docker Desktop 无 HTTPS 代理、registry-1.docker.io 直连无响应；本地既有镜像支撑，未阻塞）；F-024（模型习惯自报：1 次写入运行时受管路径被拒）；核证与发布详见 [重建记录](audits/0AC_S3_DUAL_PLATFORM_REBUILD_2026-09-14.md) |
