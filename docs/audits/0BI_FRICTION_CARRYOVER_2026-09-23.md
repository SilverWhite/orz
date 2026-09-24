# 0BI — 0bh 轮摩擦与承接大杂项（2026-09-23）

> 会话 RUN-CLI-6ab3dbe5（载体 orz 0.6.11，`D:\tb-eval\orz-windows\orz.exe`；工作区 `D:\CLI`；
> orz 子模块 HEAD a46c7c02 / 父仓 HEAD b5e084ed，worktree 含本 run 未提交改动）。
> 用户令：完成 0bi 后**不必提交/推送/重建**；按项目惯例落本报告文档；过程摩擦一并记录。

## §0 结论速览

- **落码四件＋钉子**：① 编辑面 BOM 保真、⑤ `search_replace` 锚点行窗、⑩ 反例门收窄（含测试同步）、⑪ 写入面 emoji 机械剥离（含逃逸开关与告知行）。
- **裁定七件**：⑧ S3 真机首读（读数见 §4）；② 批量注入纪律＝模型侧运行纪律（工具侧不留兜底）；④ 报告自核（本档 §9 执行）；⑥/⑦ 延后（理由见 §2）；③ 构造点收敛＝延后；⑨ 外部客户端草稿/终稿重复＝用户口径「先接着用」，记录不阻断。
- **红判据（本轮口径）全绿**：`cargo test -p orz-tools --lib` 2919 passed / 0 failed；`cargo test -p orz-loop --lib -- --test-threads=1` 839 passed / 0 failed。release 全量串行档未跑（见 §7-1）。
- 摩擦 **9 项**（§5）；改动 **11 个跟踪文件（+730/-25）＋2 个新文件（未跟踪）**（§8）。

## §1 范围与基线

- 任务：BACKLOG/TODO `0bi`＝0bh 轮摩擦项与承接杂项，共十一件＋触发式观察项；分 S1 勘定／S2 落码与钉子／S3 真机首读／S4 收口（本档）。
- 基线：载体 0.6.11（源冻结 orz 917fadfb；071 双平台重建产物）；orz 子模块 a46c7c02 起工作；父仓 b5e084ed。
- 交集档：`docs/WRITE_FACE_EMOJI_STRIPPING_DESIGN_2026-09-23.md`（⑪ 设计档 v1.0／验收 7 条）、`docs/audits/0BH_INTERFACE_CLOSEOUT_R2_2026-09-22.md`（上游摩擦源）、`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`（§4.5／§14.38 写门／§14.76）。
- 纪律：不提交／不推送／不重建；一切改动留在 worktree。

## §2 十一件对账（逐件）

### ① 编辑面 BOM 保真 —— 落码＋钉子

- 实现（写侧 BOM 保真的**单一编码点**）：
  - `orz-tools/src/util/encoding.rs`：`decode_text`（已有；剥 `EF BB BF` 并标记 label `utf-8-sig`）、新增 `label_had_bom(label)`（`label == "utf-8-sig"`）、新增 `encode_text_preserving_bom(text, had_bom)`（had_bom 时前置 `EF BB BF`）。
  - `grok_build/search_replace/mod.rs` 两写点接入：新建路径（读处捕获标签→写处 preserving）与替换路径（读 `(old_text, encoding_label)`→`write_bytes = encode_text_preserving_bom(...)`）。
- 钉子（4 条）：`bom_marked_file_keeps_bom_after_edit`（BOM 保持）、`bom_is_not_introduced_without_one`（无 BOM 不引入）、`anchor_edit_preserves_crlf_and_bom`（与 ⑤ 组合）、`ps1_with_bom_remains_parseable_by_powershell`（cfg(windows)；真跑 powershell：`ReadAllBytes` 判前三字节 BOM＋`[scriptblock]::Create` 解析通过）。
- 背景：0bh §5 #1 实锤——`scripts/build_orz.ps1` 原带 BOM，经编辑工具写回后丢失，PS 5.1 按本地代码页误读中文，报解析错。本修复＝「写回与读入同形」。

### ② 批量注入纪律 —— 裁定（无代码）

- 口径：插入计数自检＋幂等折叠，属**模型侧运行纪律**；工具侧**不留兜底**。
- 理由：工具侧无法一般性判别「重复注入」的调用意图；冒然去重会破坏合法的重复插入（如同一文本多处替换）。留待复发时按证据再评估。

### ③ 测试构造点收敛（helper）—— 延后

- 读数：`read_file` 56 处 / `grok_build_hashline` 12 / `concise` 1 / `cursor_rules` 1 / `tool_io` 1（≈71 处）。
- 延后理由：纯测试面重构、无行为收益，且大面积机械替换本身有误伤风险；建议随下次触碰这些测试面时顺手收敛。

### ④ 报告自核 —— 本档 §9 执行

- 数字口径＝本 run worktree `git diff --stat` 与各测试命令尾部汇总行**直录**；命令全文见 `.gsa/runs/RUN-CLI-6ab3dbe5/events.jsonl`；跨档链接均为仓内既有文件。

### ⑤ `search_replace` 锚点行窗 —— 落码＋钉子

- 入参：`SearchReplaceInput.anchor: Option<SearchReplaceAnchor{sha256,start_line,end_line}>`（serde default＋skip_serializing_if；schemars 字段描述承载用法）。
- 语义：sha256 对**文件原始字节**核验（口径与 read 工具 anchor 同源：`hex_string(Sha256::digest(bytes))`，`pdf_evidence.rs` 既有 `hex_string`）；不符→`InvalidInput`「anchor mismatch … re-read」；行窗 1 基闭区间，窗内容整体替换为 `new_string`；BOM/CRLF 逻辑与 `handle_replacement` 同（复用 ① 单一编码点）。
- 分派序＝anchor → 新建（空 old_string）→ 常规替换；anchor 模式下 `old_string` 必须为空（非空即拒），两模式互斥避免歧义。
- 构造点补 `anchor: None` ×6（`mod.rs` ×3：make_input / replace_all_mode / crlf_replace_all；`grok_build_concise/search_replace.rs` ×2；`orz-workspace/src/permission/types.rs` ×1）。
- 与既有 `expected_anchor` 写门（orz-loop 层，ADR-0010 §14.38）**并存不冲突**：前者＝编辑工具入参级锚点；后者＝loop 写门级防呆。
- 钉子 4 条：`anchor_replaces_line_window` / `anchor_sha256_mismatch_rejected` / `anchor_mode_guards_old_string_and_window` / `anchor_edit_preserves_crlf_and_bom`。

### ⑥ 重复读增量窗口 —— 延后

- 需求：同 path 复读时给「上次窗口＋差集」的短读面。
- 阻塞：需会话内读历史状态（read_file 目前无该原子位），落点未定；且 read_file 已具 sha256/size/mtime anchor 与 handle 信封（0bh 面），暂以「先读 anchor、按需再读」纪律覆盖。

### ⑦ 调用终态机械告知 —— 延后（可选面）

- 候选实现点：orz-host `call_tool_inner` / `process_tree.rs`（`.gsa/process_trees/` 登记）检出存活后代后随结果附一行。
- 理由：bash 三个描述模板已有 Call-scope lifecycle 行（0bh ② 落地），机械增强属增益非缺口；排期另议。

### ⑧ S3 真机首读 —— 读数记录（见 §4）

### ⑨ 外部客户端（ACP）草稿/终稿重复 —— 记录不阻断

- 用户口径：先接着用。候选解法备查：车道门判定前缓冲增量（终稿才发）或替换语义（同 call_id 覆盖）。本轮不动码。

### ⑩ 反例门收窄 —— 落码＋测试同步

- `orz-loop/src/agent_loop.rs`：新增 `answer_gate_has_substance(svc, tool_rounds)`＝工具轮>0 ∨ 黑板有编辑记录 ∨ plan 存在且任一步非 Done/Failed；门条件改为 `!counterexample_fired && profile.counterexample_gate && answer_gate_has_substance(...)`。
- 效果：纯文本短聊（无执行事实、无未完成 plan）不再要求反例自检；带执行事实或未完成 plan 的运行仍触发。
- 测试同步：`controller.rs` 两旧用例补 `.with_plan(...)`、新增 `counterexample_gate_skipped_for_text_only_short_run`；`host_exec/tool_run.rs` 流式 deltas 用例补 plan；`delivery.rs`/`orientation.rs`/`tool_probe.rs` 相关面同步调整。`cargo test -p orz-loop --lib counterexample`＝3 passed。
- 触发式观察项（0bh §5 竞态类）：本轮未复发，按 0bh 口径维持不动、不升级。

### ⑪ 写入面 emoji 机械剥离 —— 落码＋钉子

- 咽喉点定案：**编辑族输入解码点（`run_search_replace`）**——在役写文件工具仅 search_replace（RETAINED_GROK_BUILD_TOOLS 口径）；`LocalFs::write_file`（收 bytes、二进制/编码混淆）被否。只剥 `new_string`；`old_string` 不剥（保持与既有文件字节匹配）。
- 剥离集＝Emoji_Presentation ∪ Extended_Pictographic ∪ 序列残件（ZWJ、VS15/16、keycap、肤色、区域指示符、标签字符）；**U+2605 ★ 例外保留**；`★ → ⇒ § ± × ÷ ∑ ≠ ∞` 等非 emoji 符号保留（验收）。数据源＝Unicode 15.1 `emoji-data.txt`（sha256=d7aef489…，下载于 `.tmp-0bi/`）；范围表 `util/emoji_strip_ranges.rs`（≈81 区间、有序无重叠，带单测）。
- 语义：最大 strip 段整条净化（ZWJ 序列不留残件）；keycap 基字符（`0-9#*` ＋可选 VS16 ＋ `20E3`）随序列；空白折叠**仅两侧均为水平空白**时折为单空格，否则原样删除（不跨行、不引入新空白）。
- 逃逸：`ORZ_WRITE_KEEP_EMOJI=1|true`（夹具仓用）——逃生用例证字节不变、无告知行。
- 告知行：`[emoji 剥离] N 处（L12/L45/L46）`（行号去重、上限 20），**成功写入后**尾附 `EditsApplied` 两个 prompt 字段（concise 字段亦随行）。
- 文件：新增 `util/emoji_strip.rs`（纯函数＋告知行＋逃逸解析＋单测 10 条）＋`util/emoji_strip_ranges.rs`（范围表）＋`util/mod.rs` 注册；接点 `run_search_replace` 入口（`mut input`＋剥离块＋`attach_emoji_strip_notice`）。
- 钉子：`emoji_stripped_from_written_content_with_notice`（含告知行 `L1/L2` 断言）、`emoji_escape_hatch_keeps_writes_byte_identical`、emoji_strip 单测（ZWJ/VS/肤色/旗帜/keycap/★保留/空白折叠/行号/逃逸/范围表）。

## §3 关键设计裁决（S1 定案）

1. ⑪ 咽喉点＝search_replace 输入解码点（非 FileSystem 写原语）：单一实现点＋逃逸可达＋读取/对话面不动。
2. ⑤ anchor 形状＝`{sha256, start_line, end_line}` 1 基闭区间＋`old_string` 必须为空（两模式互斥，避免歧义）；sha 口径与 read 工具 anchor 严格同源。
3. ⑩ 语义＝「执行事实或未完成 plan」才要求反例自检；跑分/纯文本口径不特殊处理。
4. ② 不留工具侧兜底（见 §2-②）。
5. ④ 报告数字以命令尾部汇总行直录为准，不手抄加工。
6. ⑪ 告知面取 B：随工具结果一行事实（模型免纪律；设计档验收 7 条之一）。

## §4 S3 真机读数（本会话；载体 0.6.11）

- read_file `outline=true` ✓（章节目录面可用）。
- 大文件 read handle 信封 ✓（size/encoding/sha256/mtime/lines/preview/truncated/continue 全带；本轮多次用于 120KB 级文件）。
- 上下文分块表 ✓：已压缩/已截断块带「台账 [seq] | 指针 rX·bY·sZ | 原文回放」三列；未压缩块无指针列（观察项，见 §5-b）。
- 黑板 `section=guide` 零徽章 ✓。
- 资源软档新梯 ✓：192K/256K 软提醒、320K 硬提醒均真触发（本会话 320K 硬提醒至少 2 次）。
- grep 尾注两态 ✓：已见「at least N」态。
- 未及读：journal anchor 三态、`run_finished` 新字段、投递计数（delivered/deferred/headroom）——留作下轮首读项。

## §5 摩擦项清单（本轮执行摩擦）

- (a) `orz.exe --version` 未打印版本、直入 TUI 并挂起约 3 分钟——已 `taskkill /PID 9580 /F` 清理；结论：该入口的版本核验需改途径（改用 `orz\\Cargo.toml`/`--build-info` 类旁路），避免再触发。
- (b) 分块表说明行称「可按块回放（指针见上）」，但**未压缩/未截断块无指针列**——口径应注明「指针仅已压缩/已截断块具备」。
- (c) 编辑锚点核对：文件一经变动（含本 run 自身写入）须重读后再改——属防呆设计、按流程走；高频场景下增加往返成本，记录备查。
- (d) PowerShell 5.1 下 `cd /d D:\CLI && …`（cmd 惯用式）直接失败——需改用 `Set-Location`；建议在工具侧描述或文档中备注。
- (e) 一条命令输出 GBK 乱码——PS 5.1 输出编码面需显式设定 `[Console]::OutputEncoding`。
- (f) 本会话压缩/截断频仍（10+ 次，含多次硬截断），工作现场反复重建——大仓＋长任务组合下的常态成本，记录备查。
- (g) 一次 `search_replace` 未应用（blackboard.rs，exit 1）——重试成功后继续；疑似锚点/并发面，未能复现细节。
- (h) 「ps1 解析」钉子初版语义误判：用 `[IO.File]::ReadAllText` 后取 `$t[0]` 判 BOM，而该 API 会**剥掉 BOM**→测试假红；v2 改为 `ReadAllBytes` 判前三字节 `EF BB BF` ＋ `[scriptblock]::Create` 解析，绿。教训：BOM 断言必须落在**字节面**。
- (i) `git diff` 提示 CRLF→LF（worktree 行尾面）：新写文件以 LF 入工作区、提交时 git 归一——非缺陷，备注。

## §6 验证记录（命令与结果）

| # | 命令 | 结果 |
|---|------|------|
| 1 | `cargo check -p orz-tools --tests` | ✓ Finished（57.20s，0 错） |
| 2 | `cargo check -p orz-workspace` | ✓ Finished（1m59s，0 错） |
| 3 | `cargo test -p orz-tools --lib -- emoji` | 14 passed / 0 failed |
| 4 | `cargo test -p orz-tools --lib -- anchor` | 62 passed / 0 failed（含 4 条新用例） |
| 5 | `cargo test -p orz-tools --lib -- bom` | 首跑 16 passed / 1 failed（ps1 初版，见 §5-h）→ 修正后 `-- ps1` 单跑 ok |
| 6 | `cargo test -p orz-tools --lib` | **2919 passed / 0 failed / 6 ignored**（44.65s） |
| 7 | `cargo test -p orz-loop --lib -- --test-threads=1` | **839 passed / 0 failed / 3 ignored**（126.32s） |
| 8 | `cargo test -p orz-loop --lib counterexample` | 3 passed / 0 failed（⑩ 面） |
| 9 | `git -C orz diff --stat` | 11 files changed, 730 insertions(+), 25 deletions(-) |

## §7 遗留与下一步

1. **release 全量串行档（红判据入口）—— 2026-09-24 已补跑（见 §10）**：`scripts/run_orz_tests.ps1 test --release -p orz-assurance -p orz-loop --lib -- --test-threads=1`
   → exit 0／警告 0：orz-assurance **275/0**、orz-loop **839/0/3**。本轮 debug 全量档（§6#6/#7）与 release 全量串行档**双档全绿**。
2. ⑪ 告知行**真机首读**（需载体升级后）：改写含 emoji 的文件，核对工具结果尾行与落盘字节。
3. ⑧ 未读三项（§4 末）：journal anchor 三态、`run_finished` 新字段、投递计数。
4. ③⑥⑦ 若纳入下轮：③ 随触面顺手收敛；⑥ 需先落 read_file 会话读历史原子位；⑦ 待在 orz-host call_tool 面做存活后代探测。
5. TODO/BACKLOG/索引的 0bi 状态行、orz pin/源码清单同步——留待提交轮（本 run 按令不提交/不推送/不重建）。
6. 触发式项（0bh 竞态类）：维持观察。

## §8 改动文件清单（本 run worktree）

跟踪文件（`git diff --stat` 11 files, +730/-25；括号为 --stat 变更行数）：

- `orz/crates/codegen/orz-tools/src/implementations/grok_build/search_replace/mod.rs`（582）——⑤⑪① 主体＋钉子
- `orz/crates/codegen/orz-tools/src/util/encoding.rs`（25）——① 三助手
- `orz/crates/codegen/orz-tools/src/util/mod.rs`（2）——⑪ 模块注册
- `orz/crates/codegen/orz-tools/src/implementations/grok_build_concise/search_replace.rs`（2）——⑤ 构造点
- `orz/crates/codegen/orz-workspace/src/permission/types.rs`（1）——⑤ 构造点
- `orz/crates/orz-loop/src/agent_loop.rs`（33）——⑩ 主体
- `orz/crates/orz-loop/src/controller.rs`（78）——⑩ 测试同步
- `orz/crates/orz-loop/src/delivery.rs`（10）／`orientation.rs`（4）／`host_exec/tool_run.rs`（9）／`tool_probe.rs`（9）——⑩ 关联测试面

新增（未跟踪，提交时需 `git add`）：

- `orz/crates/codegen/orz-tools/src/util/emoji_strip.rs`——⑪ 剥离原语＋告知行＋单测
- `orz/crates/codegen/orz-tools/src/util/emoji_strip_ranges.rs`——⑪ 范围表（生成源 Unicode 15.1）

## §9 勘误与备注（含 ④ 报告自核）

- 勘误 1：§5-h（ps1 初版测试假红 → v2 字节面修正）。
- 勘误 2：⑤ 早期勘定稿曾写「构造点约 9 处」，落码实测为 **6 处**（mod.rs 3／concise 2／permission 1），以本档为准。
- ④ 自核：本档所有数字取自本会话工具输出尾部汇总行，可直接对 `.gsa/runs/RUN-CLI-6ab3dbe5/events.jsonl` 回溯；引用档（`docs/WRITE_FACE_EMOJI_STRIPPING_DESIGN_2026-09-23.md`、`docs/audits/0BH_INTERFACE_CLOSEOUT_R2_2026-09-22.md`、`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`、`scripts/run_orz_tests.ps1`）均为仓内既有文件、相对路径可达。

## §10 主会话补记（2026-09-24）

- **release 全量串行档补跑（本档 §7-1 待补项／0bi ④ 红判据入口）**：用户令「本轮还是应该做一轮release全量补跑」。命令
  ＝`scripts/run_orz_tests.ps1 test --release -p orz-assurance -p orz-loop --lib -- --test-threads=1`（auto 档判 `-j 2`，commit 使用 82.2%）。
  读数＝**exit 0／警告 0**；`Finished release` **7m01s**；**orz-assurance 275 passed / 0 failed**（2.96 s）；
  **orz-loop 839 passed / 0 failed / 3 ignored**（116.16 s）。⇒ 本项 debug 全量档（§6 #6/#7）与 release 全量串行档**双档全绿**。
- **同批落台账（不提交／不推送／不重建）**：① **0bi 轮达成**入账（BACKLOG／TODO／索引三方同步）；② **0bj 立项（47 → 48）**
  ＝0bi 轮摩擦与承接大杂项二轮（四件＝`orz.exe --version` 挂起〔版本核验改旁路〕／块表说明行口径／PS 5.1 `cd /d` 失败／输出 GBK 乱码；
  并入＝硬打断提示层柔和化＋中性终态与结束自述实施；承接＝release 档·⑪ 告知行首读·⑧ 三项未读）；③ **案例库同族追加**
  ＝`ORZ-PS1-BOM-001` ⑨「BOM 断言必须落字节面」（§5-h 的泛化沉淀，不入计数）。
- **门禁**：`python scripts/check_repository.py` → `error_count = 1`，唯一红＝「orz submodule working tree is dirty」（本 run 改动按令未提交，预期态）；
  台账钉子 `python -m unittest -q assurance.tests.test_ledger_consistency_nails` → **15 tests OK**。
- **缓存面实测（主会话自 journal 复算；用户 2026-09-24 问询「0bi 任务的缓存情况是机制导致的还是怎么样」）**：
  本 run **256 个模型轮**，输入 token 合计 **43.9M**，其中 **cache hit 36,815,616（83.9%）／miss 7,090,664（16.1%）**；
  平均每轮 prompt ≈ **171.5K**、平均 miss ≈ **27.7K**；逐轮命中率 **中位 92.9%／p10 31.4%**。
  ① **静态前缀干净**——`request_header_change` 全程**只 1 次**（`reason=initial`；`system_sha256 = e3b0c442…`＝空 system；`tools_sha256` 恒定）
  ⇒ miss **不是**头部/工具面漂移造成；② **12 次压缩是主因**——压缩重写消息清单 ⇒ 共享前缀塌回静态头，
  紧随其后的 1–3 轮命中率掉到 **3–30%**，随后 2–4 轮内回温到 **90%+**；③ **30 个低命中轮（<50%）全部可归因**（2026-09-24 更正：早前只扫 `context_compressed`、漏了 T1 事实行）：本 run 的**窗口改写事件＝21 次**＝**12 次压缩 ＋ 9 次 T1 硬截断**（越线 seq 1103／1616／1790／1960／2128／2304／2472／2688／2848，每次移出 7–8 块 ≈315–364K 估算）；每个事件**后 1 轮** miss 合计 **2.89M（占全部 miss 41%）**、**后 3 轮**合计 **4.56M（64%）**。⇒ 早前记为「不与压缩相邻、成因不明」的 9 个 spike 实为 **T1 截断轮**。
  ④ 归因改进候选（未立项）：逐轮记录**模型面前缀指纹**（face prefix digest），把「压缩重写 / 截断重写 / provider 淘汰」三源从推断变为可对账读数。
  ⑤ **浅压缩根因＝区间指令的解析脆弱＋静默兜底（2026-09-24 勘定，实现面偏差）**：`context_scale::extract_block_selection` 要求摘要行是**裸区间**（`压缩块: 1-62`，行内不得有他字）；模型写的是**带注解区间**（`压缩块: 1-62（全部已闭合块；工作现场保留）`）⇒ 末段 `parse::<u32>()` 失败 ⇒ 返回 `None` ⇒ 走缺省兜底「只压最旧一个已闭合块」。本 run 12 次压缩实测对照：**6 次被静默丢弃**（seq 2233→仅块 55／2444→63／2567→70／2636→71／3027→85／3037→86，各 ≈30–59K 估算）、5 次裸区间按声明执行（`1-3`／`4-7`／`17-22`／`23-26`／`1-79`）、1 次（seq 1007 声明 `1-3`）实得第 8 块（块号对应关系待单独核）。⇒ 模型 12 次压缩里 6 次实得削减 ≤16%（7.5%／7.9%／6.6%／13.5%／9.1%／15.6%）；**失败是静默的**（模型只看到压缩成功、看不到区间被丢弃）。
  压缩存档显示模型常写**累计区间**（如 `压缩块: 1-62`），而机械层只压**仍为原文的已闭合块**（`closed && Live`）⇒ 区间内多数块已被前次压缩或 T1 截断处理时，**实得只剩最旧一块（≈30K 估算）**；未指定区间的缺省选择本就＝最旧单块 ⇒ 浅压缩＝**区间语义与实得块之间的落差**，非模型单方面的判断失误。
- **0bk 单独立项（2026-09-24 用户令）**：§10-⑤ 查明为**实现面解析偏差**后，用户裁「**压缩下限这个设计就不加也不留**」「**模型做的是对的但实现是错的**」「**机械层的压缩是明确设计**，这一部分成本没关系……不用改」⇒ **解析修正单独立项 `0bk`**（只修解析＋失败如实回报；**不加**下限／连号抑制／工作点标定，**不动**机械层压缩产物），未闭合计数 48 → 49。
- **账单对账（2026-09-24，用户报控制台「后半段（今天）14,270,973 token／¥6.16」后主会话复算）**：
  **逐位吻合**——本 run 跨天，9/24 00:00（HKT）之后 **61 轮**，输入 **13,519,706**（hit 10,569,600 ／ miss 2,950,106，未命中 21.8%）＋输出 **completion 751,267**
  ＝ **14,270,973**，与用户控制台读数一致（⇒ 控制台 token 口径＝输入＋completion；**reasoning 653,508 另列**、按输出计费）。⇒ 计费账无隐藏重付或漏记，journal 与 provider 口径一致。
- **成本结构**：后半输入成本以 miss 为主导（在 hit:miss 单价 h≈0.10 时 miss 占输入成本 **74%**，h≈0.20 时 **58%**）；而后半 miss 中
  **1,581,527（54%）来自该半段 6 次压缩的重写税**（每次压缩后 3 轮 miss：387,103／233,684／288,273／154,936／328,364／190,167）。
  **低效压缩实锤**：该半段 6 次压缩中 #1/#2/#3/#5 只砍 **30–57K**（如 457,995→427,610、362,140→305,570），且 #5→#6 相隔 **10 个 seq**（3027／3037）连号压缩 ⇒ 两次重写税 518K。
  ⇒ 对照 [`v8 成本重算`](../CONTEXT_SLIDER_V8_COST_RECOMPUTATION_2026-09-16.md) 的定稿档前提（**≤2 次重写税 ⇒ miss ≈778K/会话**），本轮 **12 次压缩／miss 7.09M ≈ 9×**；
  成本超模的根因是**压缩频次与深度**（7 次 `model_selected` ＋ 5 次 `context_scale_window`），不是缓存机制失效（静态前缀全程只 1 次变更、常态轮命中 93–99%）。
