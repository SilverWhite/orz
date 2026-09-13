# 0ac S3 实现审记（0AC_S3_IMPLEMENTATION_AUDIT_2026-09-14）

> 状态：`current`；范围＝**只审与登记**（用户 2026-09-14 指令边界：不改实现代码；发现问题按项目状态语义登记为**缺口/待裁决**，不擅自动手修）。
> 被审对象：orz `ac5d6375`（S3-a 法官面）+ orz `4c892951`（S3①②：检索族探针扩面 / 本地分段检索前端 / `cause` 自描述 / 法官镜像同步），父仓 `d7642d26`（manifest 重算）与 `859a5480`/`e719de5b`（S3-a 入账）。
> 设计基线：[`IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13`](../IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN_2026-09-13.md) §9/§10.3；账本：TODO `P0-0ac` ①–⑧ / BACKLOG `0ac`。
> **结论一行**：设计面 0 问题级缺陷（2 条口径注记）；实现面 **1×P0**（本地路径核心解析器 panic，3 自测红；dev/release 均 `panic="abort"`）+ **1×P1**（`cargo fmt --check` 非 0，16 处）+ **1×P1 符合性缺口**（S3① 投递侧未落：三事件写点 / M1–M3 / A/B 记录）。

## 1. 事实基线

| 项 | 事实 | 证据 |
|---|---|---|
| S3-a（法官五族） | `ac5d6375`：`immediate_feedback` 五族 + 7 单测；`REQUIRE_RETRIEVAL_FAMILY_PROBE=false` 开关（F-007 裁决(a)） | `orz-assurance` 226/0/0 全绿 |
| S3①②（生产者+镜像） | `4c892951`：13 文件 `+1395/-57`（`local_segmented.rs` +875、`projection.rs` +153、`client.rs` +99、`host_exec.rs` +73、`orz-host` +87、`families.rs` +42、`immediate_feedback.rs` +50、`tool_probe.rs` +32、`controller.rs` +19、`registry/types.rs` +12 …） | `git show --stat 4c892951` |
| 开关面 | `ORZ_WEB_SEARCH_LOCAL`（默认关）/ `ORZ_RETRIEVAL_DEADLINE_MS`（默认 10 000）/ `ORZ_RETRIEVAL_OVERALL_DEADLINE_MS`（默认 30 000）/ `ORZ_RETRIEVAL_ENGINES`（默认 `bing_cn` 单引擎）/ `ORZ_RETRIEVAL_SEGMENT_PAGES`（默认 3） | `local_segmented.rs:29–44`、`client.rs::with_local_segmented` |
| 测试基线（本轮实测） | `orz-tools --lib` **2819 passed / 49 failed**（3 = S3 新增红，46 = 既有环境性红）；`orz-loop --lib` **771/0/3**（须清 `ORZ_MAX_WALLCLOCK`，F-012）；`orz-assurance --lib` **226/0/0** | §5 命令与输出摘录 |
| 门禁 | `python scripts/check_repository.py` → `error_count: 0` / `valid: true` | §5 |
| 格式门 | `cargo fmt --all -- --check` → **退出 1**，16 处 diff（4 个本批文件） | §5 |

## 2. 设计合理性

1. **判据形态合理**：§9.4 的「每引擎独立钟（默认 10 s）+ 整体兜底（默认 30 s）」与本需求口径「10 s = 请求发出后等首个结果的上限、不是检索任务总时限」自洽；`per_engine_deadline_is_independent_of_the_whole_budget` / `overall_deadline_bounds_the_chain` 两个测试把两个钟的语义钉死，实现与设计一一对应。**无问题级缺陷。**
2. **`cause` 与身份面分离合理**：`cause` 只在失败形状（`exit_code != 0` 或 `status == "error"`）写，且壳码集合由 schema `not.enum` 机械拒绝；`cause` 无 `failure_target`、`failure_target` 无 `cause` 两种形状都合法（与既有合法语料不冲突，F-007 已钉三类 fixture）。设计上「单事件自描述」与身份面互不依赖，方向正确。
3. **口径注记 A（非缺陷，文档面）**：设计 §10.3 ② ③ 的字面是「`retrieval_family` 探针 run 起始一次（**不多不少**）」，而 F-007 裁决(a) 落成「present ⇒ 校验；absent 不判」，靠开关 `REQUIRE_RETRIEVAL_FAMILY_PROBE=false` 兜住历史语料。差异**已显式登记**（F-007 + `immediate_feedback.rs` 模块文档头 + 开关常量），生产者落地后新 run 均带探针 ⇒ 实际执法面等于「存在即校验」；但设计稿 §10.3 未随裁决回写（**文档口径滞后**，登记 P3，不动计数）。
4. **口径注记 B（非缺陷）**：设计 §10.3① 「全部带开关，默认关闭直至复验」覆盖生产者面；**探针事件本身无开关**（每 run 无条件 +1 条 `tool_availability_check`，`gate_decision` 恒 `pass`）。该行为已在 F-011 被观察到（改写了既有流式测试的期望序列并同批修正）。读数事件无阻断语义，风险低，登记为口径注记。

## 3. 实现合理性

### 3.1 P0-1｜本地分段检索的核心解析器 panic（3 自测红）

- **位置**：`crates/codegen/orz-tools/src/implementations/web_search/local_segmented.rs:339:30`，`decode_html_entities` 内 `rest[..rest.len().min(12)].find(';')`——**按字节切片且未对齐字符边界**。
- **触发条件**：待解码串里出现 `&`，且其后 12 字节内跨到多字节字符边界（CJK 站点标题/摘要极常见，SERP 全文来自真实 HTML）。
- **实测现场**（本轮实做）：`panicked at crates\codegen\orz-tools\src\implementations\web_search\local_segmented.rs:339:30: end byte index 12 is not a char boundary; it is inside '方' (bytes 11..14 of string)`；红的三例＝`parse_bing_serp_reads_current_structure_and_skips_ads` / `search_returns_hits_with_stable_shape` / `empty_serp_is_empty_result_and_chain_falls_through`（`6 passed; 3 failed`）。
- **放大器**：`Cargo.toml:311`（`[profile.release]`）与 `:352`（`[profile.dev]`）均 `panic = "abort"` ⇒ 同输入在**生产形态是进程直接中止**（test profile 用 unwind 才表现为测试失败）。
- **影响面**：开关 `ORZ_WEB_SEARCH_LOCAL` 默认关 ⇒ 未污染任何既有 run；但它是本地路径 SERP 提取的**唯一入口**（`parse_bing_serp` → `strip_tags` → `decode_html_entities`），一旦 S4 复验打开开关，真实 SERP 大概率首轮即命中 ⇒ **该路径当前不可用于实机复验**。
- **登记**：缺口（P0），见 §6-G1。

### 3.2 P1-1｜落码批未过 fmt 机械门

`cargo fmt --all -- --check` 退出 1，16 处 diff：`local_segmented.rs` ×9（:57/:134/:261/:280/:382/:606/:701/:722/:798）、`host_exec.rs` ×2（:399/:430）、`retrieval/projection.rs` ×3（:125/:1202/:1211）、`tool_probe.rs` ×2（:625/:1419）。与 F-009（S3-a 首版未过机械门就交付）**同族复发**：同一工作面两次落码均未先过 `fmt`。登记：缺口（P1），见 §6-G2。另（收尾段追加）：`cargo clippy -p orz-tools --lib` 对本批新文件报 2 处风格警告（`local_segmented.rs:110/111`）；`orz-assurance`/`orz-loop` 侧输出为 S3-a 前批与既有文件警告，未见 `error:` 行。

### 3.3 P1-2｜S3① 投递侧整体缺席（符合性缺口）

`rg` 全 crates 扫描：`retrieval_progress` / `retrieval_result_segment` / `result_delivered` 三个新事件名**只出现在 `orz-assurance` 法官读取面**（`immediate_feedback.rs`、`families.rs`、`journal/mod.rs` 文档头），产品码（`orz-loop`/`orz-host`/`orz-tools`）**零命中**；`EventType` 枚举亦无对应变体（`ResultDelivered|RetrievalProgress|RetrievalResultSegment` 全 crates 零命中）⇒ **三个新事件此刻没有任何写点**。同时设计 §10.3① / TODO 0ac ⑤ 的投递策略（I1–I3）与 D7 机制（M1–M3）无落码；A/B 记录只在 `local_segmented.rs:189` 有注释（`EngineAttempt` 自称"A/B 记录面"），无落盘面。

需要说明的口径：**提交信息未声称**这三件（`4c892951` 只声称探针扩面 / 本地检索前端 / `cause` 自描述 / 镜像同步），问题出在**账本侧读法**——父仓 `d7642d26` 记为「0ac S3①② 12 文件」，与设计 §10.3 的 S3 范围（①生产者全部 + ②法官规则 + ③回归钉子）并列时容易被读成「S3 已闭合」。登记：缺口（P1 符合性），见 §6-G3。

### 3.4 观察（不登记为缺陷）

- `ToolError::ExecutionFailedCaused` 归 `CODE_EXECUTION_FAILED` 壳码系（不新增稳定码），真实原因走 `tool_completed.cause`——与 S2 契约「稳定码族不变、cause 自描述」一致（`host_exec.rs:369–430`）。
- `failure_cause` 的回退链（工具 `details.cause` → 载荷既有 `cause` → 宿主码/`command_exit_<n>`/合成超时）全部**限失败形状**、与法官族 4 同一谓词，边界干净。
- 46 例既有红（`grok_build::grep` 20 / `opencode::glob` 13 / `opencode::grep` 13）失败形态＝测试内进程启动失败（`exit -1` / `header missing:`），涉事文件最后一次改动分别为 2026-09-13（`23e7a378`，仅测试断言消息）、2026-08-17、2026-08-13，**与 0ac S3 无共同修改面** ⇒ 判为既有环境性红（本机 spawn 面），本轮只登记事实、不作根因定论（与 F-013 的 grep 空返是两条独立的装置侧事实）。

## 4. 设计与实现的符合性矩阵

| 来源 | 条目 | 判定 | 证据 / 说明 |
|---|---|---|---|
| TODO ① / §10.3②③ | 探针扩 `retrieval_family`，run 起始一次并写 journal | ✅ | `controller.rs:3247+` 在 `run_started` 前写第二探针事件；`projection.rs:retrieval_family_payload` 三成员读数取 registry 声明面、`gate_decision` 恒 `pass`；形状钉子测试在位 |
| TODO ② / §9.4 | 检索统一截止（默认 10 000 ms，每引擎独立 + 整体 30 s 兜底） | ✅ | `local_segmented.rs:29–44`、`search()`/`fetch_gate` 两处 `budget = per_engine.min(remaining)`；两根钟各有测试 |
| TODO ③ / §10.3① | 稳定码 + `tool_completed.cause` 自描述 | ✅ | `Cause` 闭枚举 + `ExecutionFailedCaused` 桥接 + `failure_cause()` 写点；壳码由 schema `not.enum` 拒绝 |
| TODO ④ / §10.3② | 法官五族 + 回归钉子 | ◐ | 五族已**注册**（`families.rs:1591–1595`）且 Python 镜像在位（`assurance/run_event_journal_validation.py:3876/4148`）；但本批自测 3 例红（§3.1）⇒ 钉子自身未过 |
| TODO ⑤ / §10.3① | 投递策略 I1–I3 + D7 机制 M1–M3 | ❌ | 产品码零写点（§3.3） |
| TODO ⑥ | 检索子代理提前收口 | ❌ | 本批无落码 |
| TODO ⑦ | `web_search` 信号量独立短截止 + 排队即时回报 | ❌ | 本批无落码 |
| TODO ⑧ | S1→S2→S3→S4 轨 | ◐ | S1/S2 已入档；S3 部分；S4 未开 |
| §9.6② | 代理形态下引擎交由模型自选（`engine ∈ {auto,google,bing,duckduckgo}`） | ⏳ | 只落了 `ORZ_RETRIEVAL_ENGINES` 静态引擎链（含 `duckduckgo`/`google` 模板），无模型侧 `engine` 参数面（设计自称"代理形态下的实施路径"，留待复验后裁决） |

## 5. 机械核证命令与输出摘录（本轮实做，原样摘录）

```
$ cd D:\CLI\orz; cargo test -p orz-tools --lib
test result: FAILED. 2819 passed; 49 failed; 6 ignored; 0 measured; 0 filtered out; finished in 32.57s

$ cargo test -p orz-tools --lib local_segmented
test result: FAILED. 6 passed; 3 failed; 0 ignored; 0 measured; 2865 filtered out; finished in 0.10s

$ cargo test -p orz-tools --lib parse_bing_serp_reads_current_structure_and_skips_ads
thread '...' panicked at crates\codegen\orz-tools\src\implementations\web_search\local_segmented.rs:339:30:
end byte index 12 is not a char boundary; it is inside '方' (bytes 11..14 of string)

$ Select-String -Path Cargo.toml -Pattern "panic"
Cargo.toml:311:panic = "abort"        # [profile.release]
Cargo.toml:352:panic = "abort"        # [profile.dev]

$ cargo test -p orz-assurance --lib
test result: ok. 226 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.08s

$ cargo test -p orz-loop --lib        # 常驻环境含 ORZ_MAX_WALLCLOCK=3600
test result: FAILED. 770 passed; 1 failed; 3 ignored   # blackboard::tests::blackboard_read_serves_session_section（F-012 同形）
$ Remove-Item Env:ORZ_MAX_WALLCLOCK; cargo test -p orz-loop --lib
test result: ok. 771 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out; finished in 9.97s

$ python scripts/check_repository.py
"error_count": 0,  "valid": true        # EXIT=0

$ cargo fmt --all -- --check
Diff in ...\local_segmented.rs:57 / :134 / :261 / :280 / :382 / :606 / :701 / :722 / :798
Diff in ...\orz-loop\src\host_exec.rs:399 / :430
Diff in ...\orz-loop\src\retrieval\projection.rs:125 / :1202 / :1211
Diff in ...\orz-loop\src\tool_probe.rs:625 / :1419
FMT_EXIT=1

$ cargo clippy -p orz-tools --lib --message-format short      # 收尾段追加
--> ...\web_search\local_segmented.rs:110:9 / :111:9          # 本批新文件 2 处风格警告（无 error 行）
$ cargo clippy -p orz-loop --lib --message-format short
# orz-assurance: immediate_feedback.rs:151/299/367 + lif/mod.rs:65（S3-a 前批/既有）；orz-loop: acaf.rs:326/495、action_ledger.rs:347（既有）；无 error 行

$ rg -n "retrieval_progress|retrieval_result_segment|result_delivered" D:\CLI\orz\crates --glob "*.rs"
（产品码零命中；仅 orz-assurance 法官读取面命中）
$ rg -n "ResultDelivered|RetrievalProgress|RetrievalResultSegment" D:\CLI\orz\crates --glob "*.rs"
（零命中 ⇒ EventType 无对应变体）
```

> 注：本工作区 `grep` 工具已知空返（F-008/F-013，本轮复现 3 次），以上检索一律改用 `rg.exe`（`B:\Zcode\resources\tools\ripgrep\rg.exe`）或 `Select-String`；输出编码统一 `[Console]::OutputEncoding=UTF8`。

## 6. 状态登记与裁决点

- **G1（缺口，P0）**：本地分段检索核心解析器字符边界 panic（§3.1）⇒ 该路径在实机复验前必须先过自测；当前 `ORZ_WEB_SEARCH_LOCAL` 默认关是唯一护栏。
- **G2（缺口，P1）**：S3①② 落码批未过 `cargo fmt` 门（§3.2，F-009 同族复发）。
- **G3（缺口，P1 符合性）**：S3① 投递侧未落（三事件写点 / M1–M3 / A/B 记录 / TODO ⑤⑥⑦，§3.3、§4）⇒ **0ac 仍在开放项**，S3 不得按"已闭合"读。
- **裁决点（待用户裁决，本轮不动手）**：① 修复批次划分（G1+G2 同批 vs 拆轮）；② S3① 投递侧是否拆分为独立子阶段（建议语义：`S3①-a 检索侧已落` / `S3①-b 投递侧未落`）并更新 TODO ⑤⑥⑦ 的状态行；③ `REQUIRE_RETRIEVAL_FAMILY_PROBE` 是否在 G1/G2/G3 关闭后翻转（当前 false）。
- **登记位置**：本审计文档；摩擦台账 `F-014`/`F-015`（RUN-CLI-6aa6c9b3）；TODO `P0-0ac` 段审记行；BACKLOG `0ac` 块审记行；索引 `CLI_PROJECT_INDEX.md` v3.18（版本头 + `GAP-MECH-IMMEDIATE-FEEDBACK` 路由行）。

## 7. 边界与未做项（诚实声明）

- **未改任何实现代码**（含未跑修复）；未动 `orz` 子模块任何文件；父仓改动仅审计文档 + 账本（FRICTION_LEDGER / TODO / BACKLOG / 索引）。
- **已跑（收尾段追加）**：`cargo clippy -p orz-tools -p orz-loop --lib`——本批新文件 2 处风格警告（§3.2/§5），无 error；**未跑**：Linux/musl 构建（本轮无跨平台改动面无需求）；S4 实机复验（待放行）；载体重建、推送（用户边界明示）。
- **既有红边界**：46 例 grep/glob 族红为环境性既有事实（§3.4），本轮只登记不归因、不修复。
- 核证时点：2026-09-14 00:20–00:40（CST），工作区 `orz` HEAD=`4c892951` 干净、父仓 `main` 干净（审计前）。
