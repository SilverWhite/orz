# 0bm 编辑面实施记录与摩擦台账（狗粮长轮 · RUN-CLI-6ab6275c）

- 日期：2026-09-25
- 性质：**0bm 狗粮长轮（本会话＝真机轮本体）S2 实施增量**落账；载体 0.6.13
- 仓库：`D:\CLI\orz` @ 9d9ffef6（run 起始 HEAD；按用户令：**未提交 / 未推送 / 未重建**）
- 关联并入件：072 §7 七件、0bj 摩擦六件、0bp 检索形态 B 四子件、0bq 进程收口、orz web GBK 并入件
- 一句话结论：编辑面簇（⑤CRLF 逐行保真 / ④大小上限 / ⑥写路径收敛去名字特判 / ⑦回退窗口）与 0bj②⑤⑥ 已落码、全绿；③/0bq、0bp、①② 未竟（勘定结论随附 §5）。

## 1. 处置总览

| 并入件 | 状态 | 备注 |
|---|---|---|
| ⑤ CRLF 逐行保真 | 已实施＋测试全绿 | 逐行记录行尾；替换/插入继承替换起点行行尾；回退序 前→后→主调→LF |
| ④ 编辑面大小上限 | 已实施 | 默认 16 MiB；`ORZ_EDIT_MAX_FILE_BYTES` 逃生；fail fast 于哈希/解码前 |
| ⑥ 写路径收敛／去名字特判 | 已实施 | 新建 `util/write_face.rs` 单点；loop 侧三处结构触发 |
| ⑦ 回退窗口 | 已实施 | `.gsa/rollback/…` 原始字节快照，保留 5；成功面告知行带指针 |
| 0bj ② 块表说明行 | 改（钉子待补） | 「指针/回放仅已压缩·已截断块具备」 |
| 0bj ⑤ 320K 软化 | 改（钉子待补） | 去「现在就压」祈使；500K mandatory 硬形保留 |
| 0bj ⑥ model_stop | 已实施（勘定更正） | 早已实施（0bh 5e153de1）；本轮＝首读＋补钉＋ADR 转录核对 |
| 0bm ③ spawn sink per-dispatch | 未竟 | 勘定落点见 §5 |
| 0bq 进程收口 | 未竟 | 与 ③ 共登记面 |
| 0bp 四子件 | 未竟 | 规格已录（§5） |
| ① ADR 分卷 / ② run_agent_loop | 未启动 | 优先级最低 |

## 2. 已实施明细（含落点）

### 2.1 ⑤ CRLF 逐行保真
- `search_replace/helpers.rs`：`LineEnding`、`line_endings`、`prevailing_ending`、`push_lf_text_rendered`、`splice_with_line_endings`（+128 行）。
- `search_replace/mod.rs`：锚点路径（窗首行行尾继承替换起点、0bl① 四种换行形状、空文件零字节直写）＋经典路径（splice 替换表）；写回仍按原 BOM 形态编码。
- 语义定稿：**逐行记录行尾**；未触碰行的行尾保持原样；被替换区段消耗掉的行尾随内容消失；替换/插入的新换行继承「替换起点行」的行尾（回退序：前一行→后一行→主调行尾→LF）；终结换行沿 0bl① 既有形状。
- 测试：7 个新钉（锚点混排/插入继承/末空行；经典混排/多行替换/插入归一/局部继承）；旧钉 `crlf_mixed_line_endings` 按新语义改写——旧期望「含 CRLF 即整文件 CRLF 化」正是本件要改的行为（line4 的 LF 现保留；REPLACED 后随 line3 的 CRLF）。`search_replace` 全系 119 passed。

### 2.2 ④ 编辑面大小上限
- `util/write_face.rs`：`DEFAULT_EDIT_MAX_FILE_BYTES`=16 MiB；env `ORZ_EDIT_MAX_FILE_BYTES`（合法正整数覆盖；非法/0 回退默认）；`check_edit_size` 拒绝文案含实际大小、上限与逃生名。
- 接入：search_replace 三路径（锚点/经典/新建覆盖）＋ hashline 既有路径——读入后、sha/解码前 fail fast。

### 2.3 ⑥ 写路径收敛与去名字特判
- `util/write_face.rs`：`decode_for_edit`（UTF-16 fail-closed 门＋固定解码链＋`had_bom`）、`attach_notice_line`（仅当不以换行收尾时补分隔换行）。
- 两编辑工具 `attach_emoji_strip_notice` 均委派同一实现；hashline 既有路径改用 `decode_for_edit`；新建覆盖面**刻意不设** UTF-16 门（old 内容整体废弃，沿 0bl 口径）。
- loop 侧三处去 `search_replace` 名字特判（**结构触发**）：
  1. `console_exec.rs` 写订单发放前：按 `expected_anchor` 在场核证（原名判删除）；
  2. `console.rs` 发放后 diff 分支：终端/运行类先行（`workspace_delta`），其后 `exit_code==0` 且参数可导出 diff（`old/new` 在场）即回显；
  3. `host_exec/tool_run.rs` direct 面：同「expected_anchor 在场」结构触发。
- 死码清理：删除 `utf16_rejected_output` 薄壳（两路径改走 `decode_for_edit` 后无用）。

### 2.4 ⑦ 回退窗口
- 快照：`.gsa/rollback/<fnv1a32(路径)8>/<millis13>-<call8>.bak`；**原始字节**（BOM/行尾/编码保真——回退即原样写回）；每文件保留最近 5 条；同毫秒递增毫秒位避让（保名序＝时序）。
- 失败不静默：`RollbackOutcome::{Stored/Failed/Skipped}`；`Failed` 随告知行如实携带原因。
- 告知行：`[回退窗口] 编辑前内容已存 {pointer}（回退＝read_file 读取该文件，将内容原样写回 {file_path}）`。
- 附着口径：仅成功面（EditsApplied）；新建/空文件＝`Skipped` 无行；concise 变体经 `run_search_replace` 共用同一写路径，其尾部 swap 保留两字段内容，故同样携带（测试断言同步放宽为 `starts_with`+`contains`）。

### 2.5 0bj ②⑤⑥
- ② `model_face.rs` 块表说明行：改为「指针列与原文回放路径仅『已压缩/已截断』块具备（原文块与残段逐字仍在窗口内、无指针）」。
- ⑤ `context_scale.rs` 320K 块：去「现在就压：」祈使 → 「…是否现在压缩、压缩哪些块由你判断（压缩交给你自选、可延后；{target_tier_advice()}）——若决定压缩：{guide}」；`mandatory_compression_window_block`（500K）「请你现在压缩」硬形保留。
- ⑥ 勘定更正：model_stop 中性终态**早已实施**（0bh 5e153de1：`[RUN_END]` 解析、controller.rs 落 run_finished 五字段、schema、guide 行 1817、钉子 5084）——0bj⑥ 立项描述「未实施」为误判。

## 3. 验证证据（命令与终态）

- `cargo fmt -p orz-tools` / `-p orz-loop`：干净。
- `cargo test -p orz-tools --lib`：**2939 passed / 0 failed / 6 ignored**（终态；期间一次红＝自测钉 `rollback_retention_keeps_last_five`，修复见 F2）。
- `cargo test -p orz-tools --lib implementations::grok_build::search_replace`：119 passed。
- `cargo test -p orz-loop --lib anchor`：20 passed / 0 failed（含 `console_anchor_*` 全组、direct 面 `direct_search_replace_*`、`search_replace_anchor_mismatch_is_typed_getput_failure` 等——⑥ 改动定向验证）。
- 已知 flaky：`implementations::lsp::tests::e2e_restart_replay_requeues_pending_diagnostics` 全量并行下失败、**单跑通过**（F3）。
- 工具链：Rust 2024（env 读写须 `unsafe` 块；见 F9）。

## 4. 摩擦台账（本会话实录）

- **F1 写路径告知行 vs concise 精确断言**：`grok_build_concise::search_replace` 两测试 `assert_eq!` 全串断言被回退告知行打破（concise 工具经 `run_search_replace` 共用写路径）。处置：改 `starts_with`＋`contains("[回退窗口] …")`。根因＝同层测试对输出全串的强耦合。
- **F2 快照名序陷阱（自测钉出）**：同毫秒后缀避让（`…-1.bak`）在字典序下反排到 `….bak` 之前，破坏「名序＝时序」→ 保留窗口裁错。处置：改为**递增毫秒位**避让；测试 `rollback_retention_keeps_last_five` 钉住。
- **F3 lsp e2e 并行 flaky**：全量 2936→2939 两轮之间该测试两态；单跑恒过。列为已知 flaky。
- **F4 320K 模型面硬窗口实读**：本会话实读 **322,223tk**（软 192K/256K 均先行触发）；窗口「现在就压：」祈使句即 0bj⑤ 的实锤来源（本会话内为旧文案实读）。
- **F5 分块表「说明行」措辞误导实锤**：本会话表中说明行为「可按块回放（指针见上）」，而当轮块 #1-8 **并无指针列**（指针以独立行出现）；即 0bj② 所指缺口，本会话内为旧版实读（修复待 S3 生效）。另观察：说明行在「表尾」与「模型面通知」重复出现两次。
- **F6 PS 5.1 控制台回显乱码**：`Select-Object` 管道下中文（含本件新增的「[回退窗口] …」文案）呈 GBK 乱码——与 orz web GBK 并入件同族，本会话多次目击。
- **F7 资源软提醒触发**：commit 4.75 GiB ≥ available 3.95 GiB 提示出现，为设计内软档、无阻断（狗粮轮实录）。
- **F8 [RUN_END] 未自述**：本会话（0.6.13 载体）未以 `[RUN_END]` 收尾——model_stop 为后续实施，S3 重建后为验证点。
- **F9 Rust 2024 env 测试摩擦**：`set_var/remove_var` 须 `unsafe` 块＋进程级锁；write_face 测试按此落（`WRITE_FACE_ENV_LOCK`）。
- **F10 同形文段歧义往返**：锚点/经典两路径的同形解码块导致一次 `search_replace` 替换命中多处→重读后补足上下文重试（费 1 往返）——「同形文段」对编辑面的天然摩擦。
- **F11 rustfmt 版本噪声**：本机 `cargo fmt -p orz-tools` 产生两处与仓库既有档不符的噪声（`read_file/mod.rs` 的 `panic!` 折行、`emoji_strip_ranges.rs` 尾空行）——已 `git checkout --` 复原；提示本机 rustfmt 与仓库档案存在版本差。
- **F12 压缩窗口摘要消费时机不可见**：模型两度投递「9-10」摘要未见即折，第三度（明示「9-12」）方折——窗口与摘要的握手时点对模型不可观测（观测性缺口）。
- **F13 报告自身被 emoji 剥离（真机轮内实锤）**：本报告初稿的状态符号（对勾 U+2705／沙漏 U+23F3）在写盘时被机械剥离（0bl 行为本例实证；工具输出随附告知行「[emoji 剥离] 11 处（写入内容 L13–L23 行）」）。处置：状态格改用文字标记。

## 5. 未竟与移交（勘定结论）

- **③ spawn sink per-dispatch**：勘定落点＝工具 spawn 经 `orz-tools/util/spawn.rs` 汇聚；`xai-tty-utils/process_scope.rs` prepare/register/enroll/spawn/kill_all/kill_active:31-236；杀点 `orz-host/lib.rs:1322`（超时）、`host_exec/tool_run.rs:3742`（取消臂）、`controller.rs:6192`；spawn 点 `terminal.rs:2673-2757`。**方案未定稿**（sink 全局可见性保留方式待决）。
- **0bq 进程收口**：出身登记面 `.gsa/process_trees/`＋收尾扫净游离（与 ③ 共用登记面）；未落码。
- **0bp 四子件**：共 SERP 预算／降级序 browser→HTTP(`ORZ_WEB_SEARCH_LOCAL=1`)→provider／时限引擎页 10s·整体 30s·拉起不吃预算／S1→S2 钉；`client.rs`/浏览器车道实现面未勘。
- **①②**：ADR 分卷／`run_agent_loop` 深挖未启动。
- **0bj 残余**：②⑤ 钉子未补（`model_face.rs`/`context_scale.rs` 测试模块）；0bj③ 工具描述落点、0bj④「设计档模板」位置（docs 无 TEMPLATE 档，待查）未决。

## 6. S3（重建后）验证清单

1. 块表说明行新文案生效（0bj②）；2. 320K 软文案生效（0bj⑤）；3. `[RUN_END]` 自述与 run_finished 五字段（0bh）；4. 回退指针行随编辑成功面出现（⑦）；5. 大小上限拒绝文案（④，可用 `ORZ_EDIT_MAX_FILE_BYTES` 小值实测）。

## 7. 附：本轮改动清单（未提交）

```
M crates/codegen/orz-tools/src/implementations/grok_build/search_replace/helpers.rs
M crates/codegen/orz-tools/src/implementations/grok_build/search_replace/mod.rs
M crates/codegen/orz-tools/src/implementations/grok_build_concise/search_replace.rs
M crates/codegen/orz-tools/src/implementations/grok_build_hashline/edit/mod.rs
M crates/codegen/orz-tools/src/util/mod.rs
?? crates/codegen/orz-tools/src/util/write_face.rs（新增，含 5 单元测试）
M crates/orz-loop/src/console.rs
M crates/orz-loop/src/console_exec.rs
M crates/orz-loop/src/context_scale.rs
M crates/orz-loop/src/host_exec/tool_run.rs
M crates/orz-loop/src/model_face.rs
```

（`read_file/mod.rs`、`emoji_strip_ranges.rs` 的 rustfmt 噪声已复原，不在清单；详见 §4 F11。）
