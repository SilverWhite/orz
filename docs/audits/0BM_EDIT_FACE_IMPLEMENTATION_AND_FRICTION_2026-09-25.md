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

（补记 2026-09-25：上列清单已随后以 orz `b7dd241e` 提交、并随 bump `e22c0dba` 进载体 **0.6.14**，
见 [`076 重建档`](076_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md)。）

## 8. 主会话补记：缓存面实测与 0bk 修果对账（2026-09-25）

> 用户问询「上一轮狗粮轮任务的缓存情况如何」（同 0bi 轮的「缓存面实测」同形补记）。
> 口径＝主会话自本 run journal（`D:\CLI\.gsa\runs\RUN-CLI-6ab6275c\events.jsonl`）复算，
> 对照件＝0bi 轮（`RUN-CLI-6ab3dbe5`，**载体 0.6.11＝0bk 修前**）；复算脚本
> `.tmp-cache-survey.py`／`.tmp-cache-detail.py`／`.tmp-cache-compare.py`（门禁豁免件，未入仓）。

### 8.1 本 run 读数（载体 0.6.13）

- **139 个模型轮**；输入 **18,999,485**＝hit **17,504,384（92.1%）**＋miss **1,495,101（7.9%）**；
  输出 completion 238,585 ＋ reasoning 182,365 ＝ **420,950**；平均每轮 prompt **136.7K**、平均 miss **10.8K**。
- 逐轮命中率 **中位 97.5%／p10 90.3%／最低 4.4%**；≥95% 命中的轮 **117/139（84%）**；<50% 的轮 **10 个**。
- **窗口改写事件 4 次 ＝ 4 次压缩 ＋ 0 次硬截断**（`context_scale:hard_truncate` 零条）；
  压缩后 1–3 轮 miss 合计 **876,477（占全部 miss 58.6%）**；常态（非相邻）127 轮 miss 618,624（**4,871/轮**）。
- 静态前缀面：`request_header_change` 全程 **1 次**（`reason=initial`），与 0bi 轮同形 ⇒ miss 非头部/工具面漂移。

### 8.2 对照 0bi 轮（载体 0.6.11，0bk 修前）

| 读数 | 0bi（修前） | 0bm（本次） | 倍率 |
|---|---:|---:|---:|
| 模型轮 | 256 | 139 | 1.84 |
| 输入 token | 43,906,280 | 18,999,485 | 2.31 |
| hit | 36,815,616（83.9%） | 17,504,384（92.1%） | 2.10 |
| miss | 7,090,664（16.1%） | 1,495,101（7.9%） | **4.74** |
| 输出（completion＋reasoning） | 3,926,398 | 420,950 | 9.33 |
| 逐轮命中 中位／p10 | 92.9%／31.4% | 97.5%／90.3% | — |
| ≥95% 命中轮占比 | 39% | 84% | — |
| 压缩次数 | 12（6 次退化，只掉 ≤1 轮） | 4（全为模型自选，掉 12／26／27／27 轮） | — |
| 500K 硬截断 | 9（释放合计 2,998,977） | **0** | — |
| 窗口改写事件合计 | 21 | **4**（每 100 轮 8.2 → 2.9） | — |
| 改写后 3 轮 miss 占该轮 miss | 60.4% | 58.6% | — |
| 折算消耗（hit:miss:out ＝ 1:10:15） | 16.66M 单位／65.1K 每轮 | 3.88M 单位／**27.9K 每轮** | 4.30／**2.33** |

（复算口径注：本复算对 0bi 的「事件后 1 轮／3 轮」得 40.8%／60.4%，0bi 报告记 41%／64%，
差异仅来自相邻窗口的边界归属，方向与结论一致。）

### 8.3 归因：0bk 修果（实现面解析偏差修复）

- **载体旁证**：`区间说明`／`声明区间` 两串在 `orz.exe.0.6.11-bak` 中 **0／0 次**，自 **0.6.12** 起 **8／14 次**
  （`block_selection_unrecognized` 同为 0 → 14）⇒ 修件 `4f28b83a` 随 0.6.12 进载体，
  本 run 跑的 0.6.13（构建 15:26、起跑 15:48）**已含**。
- **行为实证**：本 run 4 份压缩档均带新对账行「**区间说明: 声明区间 X-Y ＝ 全部命中**」，
  单次掉 **12／26／27／27 轮**、释放 **62.6K–152.1K**；0bi 同类档记为
  「处理分块 **63**／轮 **195–195（1 轮）**／释放 ≈**33,744tk**」（＝缺省兜底的退化形态），且**无对账行**。
- **后果链**：退化压缩 ⇒ 窗口压不下去（0bi 水位爬到 【1.9M/10M】）⇒ 撞 **9 次** 500K 硬截断
  （释放合计 2,998,977，均为全额重写）；本 run 只触发 4 次深压缩，**零硬截断**。
- **单次改写代价两轮相近**（0bi 21 事件均摊 ≈204K，本 run 4 事件均摊 ≈219K）⇒ 省下的是**事件次数 21 → 4**，
  不是单次更便宜。
- **口径边界（不可混算）**：轮数 256 → 139、输出 3.93M → 0.42M 属**任务体量差异**（输出面与缓存无关），
  故**全程倍率含此分量，逐轮倍率才是修件的干净读数**；常态轮 miss 14.3K → 4.9K 亦主要反映输出尾巴大小。
- **残留口子**：本 run miss 的 **58.6% 仍来自 4 次压缩后的 1–3 轮**（≈219K/次）；
  下一级杠杆是「更少、更深的改写」，非缓存机制失效（常态轮命中 90–99%）。

### 8.4 计价几何校准与「压缩后第二针」缺口（同日补记）

- **价目与峰谷口径校准**（一手＝[`DeepSeek 定价页`](https://api-docs.deepseek.com/zh-cn/quick_start/pricing/)）：
  Flash 系列（2026-09-10 12:00 起）**空闲档＝命中 0.02 元／未命中 1 元／输出 4 元**每百万 tokens，
  高峰档为 2 倍；**高峰＝北京时间周一至周五 09:00–12:00、14:00–18:00 且排除中国法定节假日；
  周末与法定节假日全天为空闲档**（2026-09-19 官方补充说明：调休补班的周末同样按空闲档）。
  ⇒ **2026-09-25（中秋假期首日）全天低谷**，本 run（15:48–16:22）计**空闲档**，用户口径更正已采纳。
  以本价目复算 0bi 轮后半段（9/24 00:00 后 61 轮，hit 10,569,600／miss 2,950,106／completion 751,267）
  得 **¥6.17**，与用户控制台当时读数 **¥6.16** 逐位吻合 ⇒ 计价几何＝**命中:未命中 ＝ 1:50、未命中:输出 ＝ 1:4**。
  **修正读数**：本 run 全长 **¥2.80**（空闲档；前稿误按高峰 ×2 记 ¥5.60）、0bi 轮全长 **¥16.00**；
  ⇒ 全程少 **82%**、逐轮同价 ¥0.0625 → **¥0.0201（少 68%）**——用户「少了 2/3」的读法在逐轮口径逐位吻合。
  同批**核销待核**：0bi 后半段按「仅 completion 计输出价」得 ¥6.17（对账成功），把 reasoning 另计则得 ¥8.78
  （对账失败）⇒ **reasoning 不另行计价**（计入 completion），前稿的待核项关闭。
- **「压缩后第二针」缺口（本轮新发现）**：4 次压缩后的命中断层里，**第 1 轮** miss 合计 481,369（32.2%），
  **第 2 轮** miss 合计 **375,220（25.1%）**，第 3 轮仅 19,888（1.3%）；
  其中第 2 针由 **4 次压缩中的 3 次**造成（轮 84／97／126 合计 373,296，唯 1 次压缩后第 2 轮干净：轮 66 miss 仅 1,924）。
  实证指纹：轮 83（seq 757）与轮 84（seq 763）的 hit **同为 11,392 token**——即第 2 轮并未复用第 1 轮自身
  已落盘的前缀单元，而是再次塌回**静态头 11,392**（该常量即本 run 的命中地板），随后轮 85 恢复到 146,688。
  ⇒ 压缩后的第 1 针属 provider 机制（窗口头部改写 ⇒ 前缀在很早处分叉），**第 2 针不应发生**；
  候选来源＝模型面二次渲染（块表／水位／摘要行位置）与 provider 单元落盘时机，二者用现有读数**不可区分**。
  这一项正是 0bi 报告 §10-④ 未立项的「逐轮模型面前缀指纹」候选的实证价值所在；本轮**只记录、不立项**。

### 8.5 与 Codex＋DeepSeek 的机制差异勘定（同日，用户问询「Codex 接 DeepSeek 时命中没 orz 这么难看」）

- **前提更正（一手反例）**：Codex 的 agent 循环**本身**是 DeepSeek 前缀缓存的杀手——第三方项目
  [`cache-doctor`](https://github.com/JNMOS/cache-doctor) 的立项陈述即「把 Codex → DeepSeek 的缓存命中率
  从 **<20%** 拉到 **80%+**」，归因＝动态时间戳、工具重排、MCP 枚举非确定性让前缀**从 byte 0 失效**。
  ⇒ 「Codex 平」来自其**接入形态**（官方接入／前缀整形代理），不是机制天然更优；两种接入形态的
  Console 读数不可直接比对。
- **我们的命中率分解（本 run）**：干净轮 **96.4%**（尾巴仅 4.9K/轮）＋ 4 次压缩 ⇒ 整体 92.1%。
  ⇒ 与「Codex 侧常报 95%+」的差距**几乎全部落在「窗口改写」这一项**；不改写的轮次我们是 96–99%
  （≥95% 命中的轮占 84%）。**阶梯读数**：消掉「第二针」⇒ 94.0%；零改写 ⇒ 96.6%。
- **三条可核机制差异**：
  1. **投影层 vs 消息列表**：我们的请求视图＝前置＋固定指针＋分块表＋D4 机械段＋各分块＋主滑块＋尾部
     （`orz-loop/src/agent_loop.rs` v8 装配注释），设计不变量 I2/I6 只承诺「两次压缩之间前缀只追加」；
     Codex 侧是消息列表＋系统指令，压缩＝整段替换为一条摘要。⇒ 我们的前缀除受**内容改写**影响外，
     还受**非尾部段重渲**影响，前沿更多（第二针即此）。
  2. **压缩策略**：Codex 的 auto-compact 是模型自带阈值（未公开，>90% 窗口会被静默钳制，
     `--disable auto_compaction` 可关），实质是「快满才压一次」；我们＝192/224/256/288K 软提醒 →
     320K 硬提醒开窗 → 500K 硬截断，且 0bj⑤ 后**由模型自选**是否压、压哪些块
     ⇒ 本 run 模型在 260–340K 即主动压缩 4 次。
  3. **静态头厚度**：本 run `system` 为空（`system_sha256 = e3b0c442…`），命中地板只有 **11,392**（工具面）
     ⇒ 压缩后能保住的共享前缀只有这一段。
- **价格几何推论（DeepSeek 专属）**：命中价＝未命中价的 **1/50** ⇒ **一次压缩（≈130–150K miss ≈ ¥0.14）
  ≈ 把窗口放大 100K 并保持 70 轮**（100K×0.02 元/M×70）⇒ 在 DeepSeek 上「窗口做大、压缩更少」
  比「多压一次」便宜得多。
- **候选动作（只记录、待裁决，本轮不落码）**：① 「逐轮模型面前缀指纹」小件（0bi §10-④ 立项）＝
  逐轮记录 face 各段字节哈希与**首个分歧点**，把「第二针」定位到具体段落；② 加厚**不可压静态头**
  并把逐轮易变元数据（水位／估算／指针列）尽量移向尾部；③ 压缩触发点后移（软提醒起点上抬），
  硬截断线保持。
（补记 2026-09-28：候选①②经六轮狗粮存档只读复算升级立项 **`0bz` / `GAP-CONTEXT-FACE-TRANSIENT-FORK`**
（用户令「请将这一上下文压缩优化内容立项为明确待办项吧」；三源客户端证据＝恢复指纹／+2 hit 反低于 +1〔31,488→9,344〕／
干净 +2 三例 ⇒ 脸面瞬态重渲而非 provider 落盘时序；**候选③「压缩触发点后移」经用户裁决出局**——「不压第一针……
注意力质量和动作连续性……目前已经不能再割舍了」）；见 [`110 立项档`](110_CONTEXT_FACE_TRANSIENT_FORK_REGISTRATION_2026-09-28.md)。）
