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

### F-009 | 2026-09-13 | RUN-CLI-6aa6b63d | 模型习惯（自报）| fixed
**S3-a 首版未过机械门就交付**：新建 `crates/orz-assurance/src/journal/immediate_feedback.rs` 首版带 1 个 dead-code 警告（`payload_int` 未被使用）与 2 处 rustfmt diff（`cargo fmt --check` 非 0 退出）。证据：`warning: function payload_int is never used --> ...immediate_feedback.rs:73`；`Diff in ...immediate_feedback.rs:134` / `:176`。**代价**：1 轮清理。处置：删除未用函数 + `cargo fmt -p orz-assurance`（`--check` 退出 0、全库 226 测全绿）——已修。

### F-010 | 2026-09-13 | RUN-CLI-6aa6b63d | 设计内门（S3 范围只落一块）| open
**S3 三块中本轮只闭合第 ② 块的 S3-a 切片**：设计 §10.3 = ① 生产者（本地分段检索前端/投递策略/M1–M3/探针扩面 + 开关 + A/B）、② 法官规则（五条）、③ 回归钉子。本轮落 ② 的五族 Rust 实现（`immediate_feedback`：`retrieval_dedupe` / `result_delivered_accounting` / `retrieval_family_probe` / `failure_cause_shape` / `first_result_deadline`）+ 7 项单测（含 fixture 与全语料扫查）；**未注册进 `ALL_FAMILIES`**（与现有 41 族 parity 交叉核对无交集），Python 法官镜像与两族清单同步未做；① 整块未动。证据：`git status --porcelain` 仅新增该文件；模块文档头显式标注未注册原因。处置：事实留痕；注册 + Python 镜像 + 生产者三件待放行。

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

### F-013 | 2026-09-13 | RUN-CLI-6aa6bd3f | 装置侧（工具：`grep` 空返，F-008 第 4 次复现）| open（根因未定）
**`grep` 工具在本工作区再次空返，本轮未修**：对 `D:\CLI\docs`（其中 `FRICTION_LEDGER.md` 确有 3 处 `F-007`）以 `pattern=F-007` 调用 → `tool 'grep' completed with no output (exit_code=Some(0))`；紧邻的 `B:\Zcode\resources\tools\ripgrep\rg.exe -n --no-heading 'F-007' docs` 同工作区立即命中。定位进展（本轮事实）：① 工具描述串「Search file contents with regular expressions (ripgrep).」全仓唯一命中 `crates/codegen/orz-tools/src/implementations/grok_build/grep/mod.rs:269`；② rg 调用点 `mod.rs:1049` `rg_path()` → `:1051` `Command::new(rg_exec)`，参数形状 `--heading --with-filename --line-number --color=never --max-columns 1000 --max-columns-preview [-l|-c] -e <pattern> <workdir> --max-filesize 5M`（`:1052`–`:1125`），stdout/stderr 皆 pipe（`:1126`）+ `crate::util::detach_command`（`:1128`）+ `stdin(Stdio::null())`（`:1129`）；③ `rg_path()` 解析在 `grep/ripgrep.rs`（`bundle_rg` 分支落 `~/.grok/vendor/`，非 bundle 分支先读 `RG_BIN_PATH`，否则视为 PATH 上的 `rg`）。未复刻同一参数形状以区分「rg 真无命中」与「wrapper 丢输出」——根因未定，按「如定位简单才一并修」的限定条件本轮未改代码、未留钉子。**代价**：本轮 5 次工具调用（1 复现 + 4 定位）后让位于收尾（账本 + 提交）。

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

### F-015 | 2026-09-14 | RUN-CLI-6aa6c9b3 | 装置侧（工具：`grep` 空返，F-008 第 5 次复现）| open（根因未定）
**`grep` 工具继续空返**：本会话审记段复现 3 次（审计文档 §5 已注），收尾段再复现 3 次（本轮实做，原文）：`pattern=RUN-CLI-`（`D:\CLI\docs\FRICTION_LEDGER.md`）、`pattern=F-01[0-9]|run|Run|RUN`（同文件）、`pattern=0ac|GAP-MECH-IMMEDIATE-FEEDBACK|IMMEDIATE_RESULT`（`D:\CLI\CLI_PROJECT_INDEX.md`）三次均返回 `tool 'grep' completed with no output (exit_code=Some(0))`；三份目标该时刻确有大量命中（同刻 `Select-String` 命中 5 行、`rg.exe` 命中即返回）。**代价**：审记全段检索改道 `rg.exe`（`B:\Zcode\resources\tools\ripgrep\rg.exe`）+ `Select-String`，每轮额外 1–2 次工具调用。**处置**：事实留痕；根因线索见 F-013（未复刻参数形状以区分 wrapper 丢输出）。

> **本轮机械核证留痕（RUN-CLI-6aa6c9b3，输出摘录）**
> - `cargo test -p orz-tools --lib` → `test result: FAILED. 2819 passed; 49 failed; 6 ignored; 0 measured; 0 filtered out; finished in 32.57s`；`cargo test -p orz-tools --lib local_segmented` → `test result: FAILED. 6 passed; 3 failed; 0 ignored; 2865 filtered out; finished in 0.10s`，panic 原文 `panicked at crates\codegen\orz-tools\src\implementations\web_search\local_segmented.rs:339:30: end byte index 12 is not a char boundary; it is inside '方' (bytes 11..14 of string)`。
> - `Select-String -Path Cargo.toml -Pattern "panic"` → `Cargo.toml:311:panic = "abort"`（release）/ `:352:panic = "abort"`（dev）。
> - `cargo fmt --all -- --check` → `FMT_EXIT=1`，16 处 diff（分布同上）；`cargo clippy -p orz-tools --lib --message-format short` → `--> ...local_segmented.rs:110:9` / `:111:9`（本批新文件 2 处；无 `error:` 行）。
> - `cargo test -p orz-assurance --lib` → `226 passed; 0 failed; 0 measured`；`cargo test -p orz-loop --lib`（清常驻 `ORZ_MAX_WALLCLOCK` 后）→ `771 passed; 0 failed; 3 ignored`，不清空 → `770 passed; 1 failed`（`blackboard::tests::blackboard_read_serves_session_section`，F-012 同形）。
> - `python scripts/check_repository.py` → `"error_count": 0` / `"valid": true`（EXIT=0）。
> - `rg -n "retrieval_progress|retrieval_result_segment|result_delivered" crates --glob "*.rs"` → 产品码零命中（仅 `orz-assurance` 法官读取面）；`rg -n "ResultDelivered|RetrievalProgress|RetrievalResultSegment"` → 零命中。
> - `grep`（工具）空返原文见 F-015；对照：`Select-String -Path CLI_PROJECT_INDEX.md -Pattern "0ac"` → 5 行命中（L3/L5/L8/L59/L331，L331 = `GAP-MECH-IMMEDIATE-FEEDBACK` 路由行）、`rg.exe -n "RUN-CLI" D:\CLI\.gsa\ledger\current.md` → 命中 8 行（L35/L108/L110/L116–L120/L147）。
> - `git -C orz log --oneline -3` → `4c892951`（S3①/②）/ `ac5d6375`（S3-a）/ `ea777918`；`git -C orz status --porcelain` → 空（工作区干净）；父仓 `git status --short` → 仅 `?? docs/audits/0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md`。

## 统计

| 日期 | run | 摩擦条目 | 立案候选 | 已修 | 观察 |
|---|---|---|---|---|---|
| 2026-09-13 | RUN-CLI-6aa6a868 / 6aa6ac42 | F-001…F-006 | F-001②/F-002②（待立案） | F-001①/F-002①/F-006 | F-003（并入 0ac）/F-004/F-005 |
| 2026-09-13 | RUN-CLI-6aa6b63d（0ac S3-a 落码） | F-007…F-010 | F-007（判据口径待裁决）/F-008 | F-009 | F-007/F-008/F-010 |
| 2026-09-13 | RUN-CLI-6aa6bd3f（0ac S3② 裁决(a) 落地 / S3① 生产者面） | F-011…F-013 | F-013（根因待定，待立案） | F-011/F-012 | F-007 已按裁决(a) 翻宽口径落地；F-008 第 4 次复现未修 |
| 2026-09-14 | RUN-CLI-6aa6c9b3（0ac S3 实现审记） | F-014…F-015 | F-014（G1–G3 修复批次划分待裁决）/F-015（F-008 同族，根因待定） | — | F-008 第 5 次复现；G1–G3 详见 [0ac S3 审记](audits/0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md) |
