# 0ac S3 修复报告（0AC_S3_FIX_REPORT_2026-09-14）

> 状态：`current`；run_id：`RUN-CLI-6aa6d379`（CST 2026-09-14 01:00–01:25 落定）。
> 范围 = [`0AC_S3 实现审记`](0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md) 的 **G1/G2/G3** 与摩擦台账 **F-008…F-015** 中**当前可直接处理**的问题：动手修 + 机械核证 + 账本/文档回写。
> 边界（用户 2026-09-14 指令）：**不推送、不重建载体**；结构性/待裁决项（G3 投递侧、开关翻转）只登记不动手。
> 修复提交：orz `96d2b263`（父 `4c892951` → `96d2b263`，分支 `feat/fusion-architecture`，5 文件 +96/−49）；父仓提交见 §6。

## 0. 结果一行

修 4 件：**G1**（P0，本地分段检索解析器字符边界 panic）、**G2**（P1，`cargo fmt` 门未过）、**F-012**（会话面测试受宿主常驻环境变量影响）、**F-016**（PATH 上 `rg` 首解为悬空 WinGet 垫片 ⇒ 46 例 spawn 面红 + `grep` 工具空返）。

核证（本轮实做）：`orz-tools --lib` **2819→2869 passed / 49→0 failed**；`fmt --check` 退出 0（16 处 diff → 0）；`orz-loop --lib` **771/0/3**（**在常驻 `ORZ_MAX_WALLCLOCK=3600` 场景下复跑，用例层面亦 ok**）；`orz-assurance --lib` 226/0/0；父仓门禁 `valid: true / error_count: 0`。

**G3 维持 open**（三事件产品码写点仍零命中，转新摩擦项 **F-017** 立案候选）；审记文档开关口径已勘误（**F-018**）；设计稿 §10.3 的文档滞后（审记 §2.3 口径注记 A，P3）已回写闭合。

## 1. 修复清单

| # | 来源 | 等级 | 处置 | 落点（orz `96d2b263`） |
|---|---|---|---|---|
| G1 | 审记 §3.1 / §6-G1 | P0 | **已修** | `crates/codegen/orz-tools/src/implementations/web_search/local_segmented.rs`（`decode_html_entities` 去字节切片 + 命名实体 + 钉子测试） |
| G2 | 审记 §3.2 / §6-G2 | P1 | **已修** | 同上 + `orz-loop/src/host_exec.rs` / `retrieval/projection.rs` / `tool_probe.rs`（`cargo fmt` 16 处 → 0）+ 新文件 clippy 2 处 → 0 |
| F-012 | 台账 F-012 | 装置侧 | **已修** | `crates/orz-loop/src/blackboard.rs`（会话面测试期望改同源解析） |
| F-016 | 台账新增 | 装置侧 | **已修**（绕行） | 工作区 PATH：`C:\Users\1\.local\bin\rg.exe`（真 ripgrep 14.1.1，位于悬空垫片之前） |
| G3 | 审记 §3.3 / §6-G3 | P1 符合性 | **未修（维持 open）** | 无写点事实本轮复核不变；→ F-017 |
| 审记 §2.3（P3 文档滞后） | 审记 §2.3 | P3 | **已闭合** | 设计稿新增 §10.5 回写；审记新增 §8 勘误 |

## 2. 逐件细节

### 2.1 G1｜`decode_html_entities` 字符边界 panic（P0）

- **修前形态**：`let Some(semi) = rest[..rest.len().min(12)].find(';') else { … }` —— 对 `&` 起点后 **12 字节窗口做字节切片**；窗口末端落进多字节字符（CJK 标题/摘要极常见）即 `panicked … end byte index 12 is not a char boundary; it is inside '方' (bytes 11..14 of string)`（3 例红：`parse_bing_serp_reads_current_structure_and_skips_ads` / `search_returns_hits_with_stable_shape` / `empty_serp_is_empty_result_and_chain_falls_through`）。
- **修后形态**：`let Some(semi) = rest.find(';').filter(|index| *index < rest.len().min(12)) else { … }` —— 整体查找 `;`（ASCII）+ 窗口谓词收窄，**无切片、语义等价**；`&` 与 `;` 之间的实体名解析路径不变。
- **顺带**：补 SERP 正文常见命名实体 `mdash / ndash / hellip / lsquo / rsquo / ldquo / rdquo`（原先落到 `None` 分支原样保留）。
- **钉子测试**：`decode_html_entities_keeps_multibyte_window_boundary_intact`（3 断言：`"A &mdash; 官方站点"`、`"B &hellip; 中文"`、未闭合 `"未闭合 &amp"`）。
- **前后对照（实测）**：`cargo test -p orz-tools --lib local_segmented` → 修前 `6 passed; 3 failed` ⇒ 修后 `10 passed; 0 failed`（新增 1 例钉子，其余为实体面测试）。

### 2.2 G2｜`cargo fmt` 门（P1）+ 新文件 clippy

- 16 处 diff 分布（审记 §3.2 原位记录）：`local_segmented.rs` ×9、`host_exec.rs` ×2、`retrieval/projection.rs` ×3、`tool_probe.rs` ×2。
- 处置：应用 `cargo fmt`（本批 4 文件），`cargo fmt --all -- --check` **退出 0**；`cargo clippy -p orz-tools --lib` 中 `local_segmented.rs` 命中 **2 → 0**（`--message-format short` 过滤该文件 0 行）。
- 残留（事实）：`clippy` 其余 3 条警告在**既有文件**（非本批新增），另有依赖 crate `xai-tty-utils` 2 条（`process_alive` 未使用 / 可折叠 `if`）——均为审记前既存，不属 G2 修复面。

### 2.3 F-012｜会话面测试与宿主常驻环境变量（装置侧）

- 现象（F-012 原始记录）：宿主 shell 常驻 `ORZ_MAX_WALLCLOCK=3600` 时 `blackboard::tests::blackboard_read_serves_session_section` 失败（770/1/3），清空该变量即 771/0/3，其间不改代码。
- 修复：该用例的期望墙钟改为**与产品同源解析**（`controller::main_wallclock_limit_secs_override()`），不再硬编码「未设即默认」的隐式前提。
- 核证（本轮实做，先设后清）：
  - `ORZ_MAX_WALLCLOCK=3600` 常驻 + 单用例 `--exact` → `ok`（`1 passed; 0 failed; 773 filtered out`）；
  - 同环境全量 `orz-loop --lib` → `771 passed; 0 failed; 3 ignored`；
  - 清空该变量再跑单用例 → `ok`（等价性对照）。

### 2.4 F-016｜PATH 上 `rg` 首解为悬空 WinGet 垫片（装置侧，新登记）

- **事实链**：
  1. `where rg` 三个候选：`C:\Users\1\.local\bin\rg.exe` → `C:\Users\1\AppData\Local\Microsoft\WinGet\Links\rg.exe` → `B:\Zcode\resources\tools\ripgrep\rg.exe`；
  2. WinGet Links 项是**符号链接**（`Length=0`，写于 2026-08-04），Target = `…\WinGet\Packages\BurntSushi.ripgrep.MSVC_…\ripgrep-15.2.0-x86_64-pc-windows-msvc\rg.exe`，而该**包目录缺失**（`PKG_DIR_MISSING`）；对该路径 spawn 直接报 `程序"rg.exe"无法运行: No application is associated with the specified file for this operation`；
  3. 修前（本会话折叠段、01:00 前）：`orz-tools --lib` **2819 passed / 49 failed**（其中 3 例 = G1；余 46 例 = spawn 面），与此同时 harness 的 `grep` 工具连续 5 轮共 5 次空返（F-008/F-013/F-015）；
  4. 修复动作：在 **PATH 更早位置**（`C:\Users\1\.local\bin\rg.exe`）补入真 ripgrep **14.1.1**（5,400,984 B，2026-09-14 01:00）⇒ `where rg` 首解改为该项，`rg --version` 正常。
- **复现实验（本轮实做，判定性）**：不改任何代码，仅把**坏垫片目录前置**到 PATH 后重跑——
  - 全量：`test result: FAILED. 2822 passed; 47 failed` （对照：正常 PATH `2869 passed; 0 failed`）；
  - 模块级：`grok_build::grep::tests`（49 例）→ `29 passed; 20 failed`（对照：`49 passed; 0 failed`），与审记 §3.4 点名的 `grok_build::grep` **20 例**吻合（另 26 例 `opencode::glob` 13 / `opencode::grep` 13 未单独复跑，同属 PATH 解析面）。
- **同批效果**：修后 `orz-tools --lib` **0 failed**（其间代码改动仅 G1/fmt/F-012 三处，均不触及 spawn 面）⇒ 审记 §3.4「46 例既有环境红、本机 spawn 面」获得直接成因。
- **同轮对照**：本轮 harness `grep` 工具 3 次调用全部正常返回（命中 15 / 12 / 59 行）——F-008/F-013/F-015 的「空返」本轮未复现（登记为根因候选，见台账补注）。
- **残留**：WinGet 垫片本体**未修**（需重装该包；本轮只改道），若 PATH 相对顺序变化会复发 ⇒ 记 F-019（观察）。

### 2.5 G3｜投递侧未落（未修，维持 open）

- 本轮复核（命令见 §3）：`retrieval_progress|retrieval_result_segment|result_delivered` 全 crates **29 命中，全部在 `orz-assurance`**（`journal/families.rs`、`journal/immediate_feedback.rs`、`journal/mod.rs`）；`ResultDelivered|RetrievalProgress|RetrievalResultSegment` **0 命中** ⇒ `EventType` 无对应变体、产品码（`orz-loop`/`orz-host`/`orz-tools`）零写点；TODO ⑤⑥⑦（I1–I3 / M1–M3 / 子代理收口 / semaphore 截止）本批无落码。与审记 §3.3 一致。
- 处置：**不动手**（结构性缺口，待用户裁决是否拆 `S3①-a 检索侧 / S3①-b 投递侧`）⇒ 台账 **F-017** 立案候选。
- 附：审记 §6 裁决点③「`REQUIRE_RETRIEVAL_FAMILY_PROBE` 是否翻转」**实际已失效**——该开关及其执法分支在 `4c892951` 已被移除，实现面只剩 2 处**文档注释**提到该名字（`immediate_feedback.rs:41` / `:50`），详见 §2.6。

### 2.6 文档面（P3 闭合 + 勘误）

- 设计稿新增 **§10.5 回写（2026-09-14）**：① 法官规则③「探针 run 起始一次（不多不少）」的终态 = F-007 裁决(a) 的**宽口径**（present ⇒ 校验；absent 不判），且执法开关已在 `4c892951` **移除**（现无开关，仅文档注释留痕）；② 口径注记 B（探针事件本身无开关）回写；③ 生产者面状态（三事件写点未落）。
- 审记文档新增 **§8 修复回写与勘误**：G1/G2/F-012 修复状态、F-016 复现实验、G3 维持 open、§2.3/§6-③ 的开关口径勘误。
- 账本/索引：TODO `P0-0ac`、BACKLOG `0ac` 增修复行；`CLI_PROJECT_INDEX.md` v3.18 → **v3.19**（本报告路由 + 0ac 路由行状态 + 摩擦台账路由行）。

## 3. 机械核证（本轮实做，命令 + 原样输出摘录）

```
$ cd D:\CLI\orz; cargo fmt --all -- --check
FMT_CHECK_EXIT=0                                    # 修前：FMT_EXIT=1（16 处 diff）

$ cargo clippy -p orz-tools --lib --message-format short   # 过滤 local_segmented
0 hits                                              # 修前：local_segmented.rs:110 / :111 两处
（全量：orz-tools (lib) generated 3 warnings —— 均在既有文件；xai-tty-utils 2 条既有）

$ cargo test -p orz-tools --lib
test result: ok. 2869 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out; finished in 44.98s
                                                    # 修前：FAILED. 2819 passed; 49 failed; 6 ignored

$ cargo test -p orz-tools --lib local_segmented
test implementations::web_search::local_segmented::tests::decode_html_entities_keeps_multibyte_window_boundary_intact ... ok
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 2865 filtered out; finished in 0.11s
                                                    # 修前：6 passed; 3 failed

$ $env:ORZ_MAX_WALLCLOCK="3600"; $env:ORZ_ACAF_FAIL_CLOSED="0"; cargo test -p orz-loop --lib blackboard::tests::blackboard_read_serves_session_section -- --exact
test blackboard::tests::blackboard_read_serves_session_section ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 773 filtered out; finished in 0.43s
                                                    # 修前同场景：FAILED（770 passed; 1 failed; 3 ignored）

$ cargo test -p orz-loop --lib                       # ORZ_ACAF_FAIL_CLOSED=0
test result: ok. 771 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 10.15s

$ cargo test -p orz-assurance --lib
test result: ok. 226 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.08s

$ cargo test -p orz-tools --lib                      # F-016 复现实验：PATH 前置坏垫片目录
test result: FAILED. 2822 passed; 47 failed; 6 ignored; 0 measured; 0 filtered out; finished in 45.18s
$ cargo test -p orz-tools --lib grok_build::grep::tests        # 同上实验
test result: FAILED. 29 passed; 20 failed; 0 ignored; 0 measured; 2826 filtered out; finished in 0.13s
$ cargo test -p orz-tools --lib grok_build::grep::tests        # 正常 PATH 对照
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.37s

$ rg -n "retrieval_progress|retrieval_result_segment|result_delivered" crates --glob "*.rs"   # G3
B_hits=29   （全部落 crates\orz-assurance\src\journal\{families.rs,immediate_feedback.rs,mod.rs}）
$ rg -n "ResultDelivered|RetrievalProgress|RetrievalResultSegment" crates --glob "*.rs"
A_hits=0
$ rg -n "REQUIRE_RETRIEVAL_FAMILY_PROBE" crates --glob "*.rs"
2 hits —— immediate_feedback.rs:41 / :50（均为文档注释；开关与执法分支已移除）

$ python scripts/generate_orz_source_manifest.py
wrote 1448 entries to orz_source_manifest.sha256    # 差异面恰 5 行（本批 5 文件）
$ python scripts/check_repository.py
"error_count": 0,  "valid": true                    # EXIT=0

$ git -C orz log --oneline -1   →  96d2b263 fix(0ac S3): 审记 G1/G2 修复…
$ git -C orz status --porcelain →  （空，工作区干净）
```

## 4. 本轮新摩擦项（→ 台账 F-016…F-020）

| # | 归因类 | 一句话 | 处置 |
|---|---|---|---|
| F-016 | 装置侧（工具/工作区） | PATH 上 `rg` 首解 = 悬空 WinGet 垫片 ⇒ 46 例 spawn 面测试红 + harness `grep` 工具空返（F-008/013/015 根因候选） | 已修（绕行：PATH 前置真 rg 14.1.1）；复现实验 47 红 |
| F-017 | 设计内门（S3 范围） | S3① 投递侧未落：三事件产品码零写点（G3） | open（立案候选，待裁决拆子阶段） |
| F-018 | 装置侧（文档/实现漂移） | 审记把 `REQUIRE_RETRIEVAL_FAMILY_PROBE` 的当前态记为 `false`（实为已移除，仅剩 2 处注释） | 已闭合（审记 §8 勘误 + 设计稿 §10.5 回写） |
| F-019 | 装置侧（观察） | WinGet `rg` 垫片本体仍悬空（包缺失），当前靠 PATH 顺序绕开；顺序一变 spawn 面红复发 | 观察（未修，需重装包） |
| F-020 | 装置侧（文档回写） | 报告 §6/§0 的部分回写条目先于动作写成「已办」，收尾核证发现未落地（索引路由 2 处 + 父仓提交行） | 已修（收尾补齐 + 哈希回写） |

## 5. 未做项与边界（诚实声明）

- **未推送**（`orz` 与父仓均未 `git push`）；**未重建载体**（Windows release / Linux musl 均未跑）；未跑 S4 实机复验（`ORZ_WEB_SEARCH_LOCAL` 仍默认关）。
- **G3 未动手**（结构性，待裁决）；**WinGet 垫片本体未修**（需重装包，属用户环境）。
- `opencode::glob` / `opencode::grep` 各 13 例**未单独复跑**（归入 §2.4 同一复现实验组，未逐模块取证）。
- 本轮**未跑** Linux/musl 构建与跨平台测试（无跨平台改动面）；未新增/修改任何 schema 或 fixture。

## 6. 账本与索引回写

| 文档 | 动作 |
|---|---|
| 本报告 | 新建（`docs/audits/0AC_S3_FIX_REPORT_2026-09-14.md`） |
| [`0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14`](0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14.md) | 头部追记 + 新增 §8（修复回写与勘误） |
| [`FRICTION_LEDGER`](../FRICTION_LEDGER.md) | F-008/F-012/F-013/F-014/F-015 补注；新增 F-016…F-019；本轮核证留痕；统计行 |
| [`设计稿`](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md) | 新增 §10.5 回写（P3 闭合） |
| [`TODO.md`](../../TODO.md) | `P0-0ac` 增修复行 |
| [`BACKLOG_AND_PRIORITIES`](../BACKLOG_AND_PRIORITIES.md) | `0ac` 小节标题状态 + 增修复行 |
| [`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) | v3.18 → v3.19（版本头 + `GAP-MECH-IMMEDIATE-FEEDBACK` 路由行 + 本报告路由 + 摩擦台账路由行） |
