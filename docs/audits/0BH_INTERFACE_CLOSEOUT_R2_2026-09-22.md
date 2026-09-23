# 0BH — 交互面收口二轮（2026-09-22）

> 狗粮轮 `RUN-CLI-6ab2aae5`（载体 orz **0.6.10** 双平台已换装；本执行轮**不提交／不推送／不重建**）。
> 范围＝BACKLOG `0bh`（十六件：①提醒投递预算独立化／②调用级拆树告知面／③压缩回执机械摘要／④压缩带宽口径／⑤两条竞态设计／⑥构建争用面／⑦报告自核纪律／⑧0bg 遗留承接／⑨编辑回抄成本／⑩乱码降级告知／⑪grep 命中预算／⑫审计档按节定位／⑬重复读增量窗口／⑭黑板 guide 分区／⑮机械定位符＋journal anchor／⑯中性终态与结束自述）。
> 入口：`TODO.md` P1-0bh；设计档＝[`NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN`](../NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md)（⑯）、[`BLACKBOARD_GUIDE_AND_POINTER_DESIGN`](../BLACKBOARD_GUIDE_AND_POINTER_DESIGN_2026-09-22.md)（⑭⑮）。

## §0 结论速览

- **落码并验证 13 件**（①③④⑥⑩⑪⑫⑭⑮⑯ 全量；② 描述行面；⑦⑧ 以本报告本体承载体例）；
- **⑤ 设计定稿（无代码；§3）**；**S1 定案、S2 未落码**共 2 件（⑨；⑬）＋②的机械告知增强（可选面未落）——定案与落点见黑板 `notes`／`plan` 快照，本报告 §7 逐条承接；
- 编译器面：`cargo check -p orz-loop --tests` ✓、`-p orz-tools --tests` ✓；定向测试：`grok_build` 全系 1338/0 ✓（read_file 124／grep 51／hashline／concise）、`cursor_rules` 11/0 ✓、`tool_io` 5/0 ✓、`context_scale` 7/7 ✓、`model_face` 9/9 ✓、controller 钉子（⑯/①）✓；
- **本 run 载体 0.6.10 不含本轮新面** ⇒ ⑪⑫⑭⑮⑯ 的新语义（grep 尾注／read_file outline／guide 分区／anchor 回查／[RUN_END] 通道）**未真机可读**，S3 面记为「本 run 不可读＋静态核证」；下一载体轮（0bi 候选）做首读。
- 摩擦 11 条（§5）：最重一条＝**编辑工具写回 `.ps1` 丢失 UTF-8 BOM**（`build_orz.ps1` 原带 `EF BB BF`，编辑后 PS 5.1 按 GBK 误读致 CJK 解析错——已补 BOM 并以 dry-run 复核；建议工具侧对 `.ps1` 保留 BOM）；另新增 #10 批量注入两连击／#11 构造点密度。

## §1 范围与基线

- 会话：`RUN-CLI-6ab2aae5`（本报告数字取自本会话 worktree＋journal；事件数 **3213 行**＝收官刷新时点读数，`.gsa/runs/RUN-CLI-6ab2aae5/events.jsonl`）。
- 基线：子模块 `orz` 起点 `306b6c77b9f7`（worktree 含未提交改动）；父仓含 0bg／0.6.10 相关在途改动。
- 约束：不提交／不推送／不重建（用户令）；报告落本档；摩擦随轮记录。
- 载体：`orz --version` 面＝0.6.10（`D:\tb-eval\orz-windows`）。

## §2 十六件对账（逐件）

| # | 件 | 状态 | 落点／读数 |
|---|----|------|-----------|
| ① | 提醒投递预算独立化 | **落码✓** | `controller.rs attach_pull_delta` 重写：提醒段独立预算 `NOTICE_BUDGET=240` B 先行装配（≤2 条、域类≤1、**索引式投递标记**）；徽章让位并折叠为 `…(+k)`；未挂足时给「RLI提醒+N条暂存」短告知；总帽常态 256 B、退化 496 B。`rli.rs` 增 `mark_notices_delivered_at`／`record_notice_delivery_accounting`／`notice_delivery_stats`＋快照三字段（delivered/deferred/headroom）。钉子测试 `pull_delta_long_header_folds_badges_instead_of_swallowing_notices` ✓ |
| ② | 调用级拆树告知面 | **部分✓** | 门一 **A**（S1 裁决）：调用内后代随调用回收（Windows＝Job Object kill-on-close；Unix＝进程组信号）——`bash/mod.rs` 三个默认描述模板统一追加 "Call-scope lifecycle" 行（长活走受管后台）。「调用终态告知」机械增强未落码（可选面，承接 §7） |
| ③ | 压缩回执机械摘要 | **落码✓** | 压缩/截断回执带**定位符行**（`指针（回查：blackboard_read section=journal anchor=）: r<轮>·b<块>·s<seq>`；`model_face::render_pointer_line`）；块表行「 | 指针 …」列；被折轮次区间＋块号＋s=journal 起点＝「较早内容仅导航」形态 |
| ④ | 压缩带宽口径 | **落码✓** | `DEFAULT_LADDER=[192K,256K,320K,500K]`（[;4]；取消 224K/288K 档）；`COMPRESSION_TARGET_TIER_TOKENS=192_000`＋`target_tier_advice()`（`context_compress` Requested 响应与 H1 窗口块**共用**）；测试 `compression_target_tier_advice_is_shared_and_advisory` ✓；类型 6→4 全同步（compact/agent_loop/model_face/controller 测试梯） |
| ⑤ | 两条竞态设计 | **设计✓（无代码）** | 对象（复核）＝`orz/crates/orz-host/src/lib.rs` L~4030 `run_terminal_cmd_truncation_carries_output_object`（30K 输出→截断＋`output_object` 落盘＋模型面 `read_file` 指针）与 L~4389 `run_tests_output_scrubbed_of_secrets`（run_tests 输出脱敏：会话尾＋卷面 `run_tests_output.txt` 双判据）；裁决＝**不放宽断言**＋**串行档为红判据入口**（0bg ①b 已文档化）；升级路径＝§3／§7 |
| ⑥ | 构建争用面 | **落码✓** | `scripts/build_orz.ps1`：新增 `-WaitForHeadroom` 排队口径（15 s 节拍轮询、≤30 min、扫描 cargo/rustc/orz 重活进程；到线或超时如实打印后继续）＋软提示行（commit≥72% 或余<12 GiB 时提示降并行/排队，**不阻断**）；dry-run 实测：commit 83.1%／余 4.2 GiB → `jobs=2（auto）`＋软提示行 ✓ |
| ⑦ | 报告自核纪律 | **体例✓** | 本报告 §6 全数列出命令与结果；数字口径＝本会话 worktree＋journal 实测（§1） |
| ⑧ | 0bg 遗留承接口径 | **体例✓** | 本报告 §7 承接表（0bg §5 摩擦→0bh 件映射） |
| ⑨ | 编辑回抄成本 | **未落码** | S1 定案：`search_replace` 增 anchor 模式（sha256＋行窗，跳过全串回抄）；落点＝`grok_build/search_replace/mod.rs`（可引 `grok_build_hashline` 锚点形态）；承接 §7 |
| ⑩ | 乱码降级告知 | **落码✓** | `read_file/mod.rs`：解码链落 `utf-8-lossy:<p>%` 档时，`content` 与 `content_concise` 追加「[编码降级告知] …（重取建议：`run_terminal_cmd`／iconv／按字节工具）」；不阻断、不改写原文（纯读取）；`read_file` 测试 121/121 ✓（⑫ 后 124/124，见 §6） |
| ⑪ | grep 命中预算 | **落码✓** | `grok_build/grep/mod.rs` 三格式化函数（content／files／count）尾注两态：cut 分支省略计数改**行数口径**（`omitted=trimmed_lines.len()-cut_idx`，替代 `count_matches`；截断区无命中行时不再静默省略）＋收窄引导「… [{at least }{N} lines truncated; narrow the pattern or add a path/glob to see the rest] …」；「已截断但无字节切断」也出尾注「… [output truncated at the budget; …] …」；流式面（BodyStreamer）不动（尾注 terminal-only）。测试 51/51 ✓（含 2 新钉子） |
| ⑫ | 审计档按节定位 | **落码✓** | `ReadFileInput.outline: Option<bool>`（text-only；serde default＋schemars 文案）；`build_outline()`＝ATX 标题（`#`~`######`＋空白/行尾）→ `L<行号>: <标题>` 索引；cap 300＋显式省略；无标题如实告知（不产伪索引）；钩子在 `total_lines` 后、**coarse gate 前**（大文件也可先定位；返回 content=content_concise=索引）；`DESCRIPTION_FULL`＋concise 各补一行；5 文件 71 处构造点补 `outline: None`；read_file 124/124 ✓（＋3 钉子） |
| ⑬ | 重复读增量窗口 | **未落码** | S1 定案：同 path 重复读给「上次窗口＋差集」短读面（需会话内读历史状态，落点待定）；承接 §7 |
| ⑭ | 黑板 guide 分区 | **落码✓** | `blackboard_read section=guide`：pull 面、**零徽章**（排除增量头）、≤1 KiB；单源文案 `BOARD_GUIDE_BODY`＋digest8 随行（`AgentLoopController::render_board_guide`）；含指针回查法与 [RUN_END] 通道语法行；组合参数纪律＝显式文本错误 |
| ⑮ | 机械定位符＋journal anchor | **落码✓** | 指针 `r<轮>·b<块>·s<seq>`（`#sha8` 可选）三生成点：压缩回执／块表／（域时间线承接）；`blackboard_read section=journal anchor=<ptr>`：**只回机械字段摘要 ≤512 B**（seq/event_type/timestamp/payload 键/#sha8＋指针回显）；错误三态（不存在（越界附事件数）／属于其他 run 或已归档／已过期（sha8 不符）） |
| ⑯ | 中性终态与结束自述 | **落码✓** | 新模块 `model_stop.rs`（`[RUN_END]` 声明解析：intent{conclude,pause,other}／reason{completed,partial,blocked,awaiting_response,other}／summary／unrecognized；`ORZ_MODEL_STOP_AWAIT` 缺省开、`0` 关）；`run_finished` payload 新字段（intent/reason/summary≤4K/unrecognized≤500/await_channel；**无声明 ⇒ 旧形逐字一致**）＋schema 5 属性；钉子 `run_finished_carries_model_stop_declaration` ✓；子项＝删 `context_scale.rs` 两处「任务无需中止／继续即可」✓ |

## §3 关键设计裁决（S1 定案）

- **门一（拆树告知）＝A**：告知面写进工具描述（调用内后代随调用回收；长活走受管后台），**不做** breakaway／detach 特例。
- **门二（带宽口径）＝A**：只做目标档建议（192K）；较早内容只给「域位置＋轮号」导航；**不加**折叠下限、**不动** `max_reduction_ratio`。
- **⑯ 语法定案**：`[RUN_END]` 行内声明 `intent=<…>; reason=<…>; summary=<…>`；未识别取值入 `unrecognized`（机械记录、不驳回）；`ORZ_MODEL_STOP_AWAIT` 缺省开（跑分用显式 `0` 关＋起跑前断言＝承接 §7）。四原则：判断归模型／机械只记录与传话／回应归外部／无应答者不剥夺出口。
- **⑮ 指针语法**：`r<轮>·b<块>·s<seq>`（s＝journal 事件行号；可选 `#<sha8>` 内容锚）≤24 B；三生成点＝压缩回执／分块表／（域时间线承接）；回查 ≤512 B。
- **⑭ guide 形态**：pull 面、零徽章、≤1 KiB、单源文案＋digest8；内容＝黑板总览＋指针回查法＋[RUN_END] 通道。

- **⑤ 两条竞态口径（本批定稿，无代码）**：**不放宽断言强度**（两条均系产品判据：截断必带 `output_object`／输出必脱敏落卷）；并行档偶红按负载噪声重跑校验；**红判据入口＝串行档**（0bg ①b 已文档化）；若串行档复发，按 0bd 模式给续跑调用**显式超时覆盖**／serial 标注（消除抖动窗口，而非放宽判据）。
- **⑪ 尾注口径**：省略数一律**行数口径**＋收窄引导（narrow the pattern or add a path/glob…）；「已截断无字节切断」也出声；流式面（BodyStreamer）不注入（尾注 terminal-only）。
- **⑫ outline 口径**：text-only、整档索引、cap 300 显式省略；**先于 coarse gate**（先定位再按 offset/limit 读段）；无标题如实告知。

## §4 S3 真机读数（本会话；载体 0.6.10）

- **块表 v0.1（本会话逐轮实测）**：`[块#k] 轮次 a-b ≈Ntk | 状态 | 动作: … | 目标: … | 台账 [seq] a-b | run=…`；「已压缩」块带 `[seq]` 外挂台账区间（本会话 9176 起连续编号、随轮增长）＋按块回放路径；「原文／残段」块显标注 `[seq] —（原文未外挂）`——即 ⑮ 的改造对象（新列「指针 r·b·s」需下一载体）。
- **软提醒**：本 run 内 192K／224K／256K 档均真触发过（224 属 0.6.10 旧梯；新码已取消 224/288 档）；`[CONTEXT_SCALE 288K]` 提示行实测出现（0.6.10 口径）。
- **压缩 v0.4**：本会话实际执行多次分块压缩（`compaction-RUN-CLI-…` 序档；摘要＋机械摘要＋回放路径三件式实测工作正常）。
- **RLI 面**：提醒计数与水位线（`【x.xM/10M】`）实测在板；① 的独立预算改造未进载体 ⇒ 本 run 观察到的仍是旧行为。
- **⑪⑫⑭⑮⑯ 新面**：本 run **不可读**（载体不含）；静态面＝编译＋钉子测试；真机首读顺延下一载体轮。

## §5 摩擦项清单（本轮执行摩擦）

1. **编辑工具写回 `.ps1` 丢 UTF-8 BOM**（最重）：`scripts/build_orz.ps1` 编辑前带 `EF BB BF`（`git show HEAD` 核证），经编辑工具写回后无 BOM → PS 5.1 按 GBK 误读，中文注释/字符串致「数组索引表达式丢失」等解析错。**已修**（.NET `UTF8Encoding($true)` 重写＋`ParseFile` 复核＋dry-run 复核）；建议：编辑工具对 `.ps1`**保留原 BOM**（或在写入时按原编码回写）。
2. `impl` 内 const 引用须 `Self::` 前缀（`BOARD_GUIDE_BODY`；编译错一轮）。
3. `FakeProvider` 脚本耗尽：run 级测试每轮消耗一条脚本，`run_finished` 钉子需 2 条（首轮后仍有收束轮）。
4. `EventWriter` 无 `events_path()`（只有 `journal_dir()`）；anchor 解析路径＝`journal_dir()/events.jsonl`。
5. borrow 陷阱：journal 行扫描先 tuple 借 `(Value, &str)` 挂生命周期 → 改 index 取行再解析。
6. cargo 迭代成本：`check` 约 14–140 s/次、测试构建 2 m+（宿主 commit 偏紧时更慢）。
7. Windows commit 上限压力：构建窗口内出现「系统提交内存不足」通知线（≈4.06 GiB）；⑥ 的 dry-run 实测 commit 83.1%。
8. 大文件读取走 read handle 分页（`controller.rs` 360 KB、`tool_run.rs` 331 KB 等）。
9. 块表逐轮注入在窗口尾部（本会话实测；⑮ 改造后指针列将随块表逐轮注入，注意单块 ≈24 B×块数的常态开销）。
10. **批量结构注入两连击（PowerShell）**：为 5 文件 71 处构造点补 `outline: None`——首轮（相对路径＋内联多行 regex）`matched=0` 零命中＋路径异常（未写入、无害）；次轮（绝对路径＋`[char]13/10`＋`String.Replace`）注入成功但缩进模式（8/12/16/20 空格）互为子串 ⇒ 同一站点重复插入 2–3 份；以「连续同名行折叠」收口，`fmt8==outline8` 计数复核（56/56、12/12、1/1、1/1、1/1）。教训＝批量结构注入必须带**插入计数自检**与幂等折叠。
11. **构造点密度**：单字段新增（`outline: Option<bool>`）波及 5 文件 71 处字面量构造（read_file 56／hashline 12／concise 1／cursor_rules 1／tool_io 1）；观察项＝测试构造面宜收敛 helper（未立项）。

## §6 验证记录（命令与结果）

- `cargo check -p orz-loop --tests` ✓（多次；最后含 ⑭⑮ 改动）。
- `cargo check -p orz-tools --tests` ✓（②⑩ 之后；⑪⑫ 收官复跑 ✓，11.79 s）。
- `cargo test -p orz-loop --lib context_scale::` → **7 passed**。
- `cargo test -p orz-loop --lib model_face::tests` → **9 passed**。
- `cargo test -p orz-tools --lib implementations::grok_build` → **1338 passed / 0 failed**（收官；含 read_file／grep／hashline／concise）。
- `cargo test -p orz-tools --lib implementations::grok_build::read_file` → **124 passed**；`…::grep` → **51 passed**；`read_file_outline` 定向 → **3 passed**；`cursor_rules` → **11 passed**；`tool_io` → **5 passed**。
- 钉子：`run_finished_carries_model_stop_declaration` ✓；`pull_delta_long_header_folds_badges_instead_of_swallowing_notices` ✓；`blocks_are_numbered_from_the_conversation_start_and_stay_stable` ✓。
- `scripts/build_orz.ps1 -DryRun` ✓（commit 83.1%／余 4.2 GiB → jobs=2（auto）＋软提示行）。
- journal 事实面：本 run `events.jsonl` **3213 行**（收官刷新时点）；事件字段（schema_version/run_id/event_id/sequence/timestamp/event_type/previous_event_sha256/payload_schema/payload/payload_sha256）——⑮ anchor 的 `s` 口径＝`sequence`。

## §7 遗留与下一步

- **承接表（0bg §5 → 0bh）**：0bg 摩擦三分类中的「结构摩擦」已分别入 ④（带宽口径）／⑥（争用面）／⑫⑬（读面减负）；「执行摩擦」入 §5；「遗留承接」＝⑤ 本批**设计定稿**（§3），余下 ⑨⑬ 交下一轮。
- **本批未竟**（定案已入黑板 `notes`／`plan`）：⑨ `search_replace` anchor 模式；⑬ 重复读增量窗口；＋②「调用终态告知」机械增强（可选面，未落）。
- **⑤ 升级路径（预登记）**：若串行档复发，按 0bd 模式给续跑调用**显式超时覆盖**／serial 标注——不改断言、不改产品语义。
- **S3 首读顺延**：⑪⑫⑭⑮⑯ 新面真机首读＝下一载体轮（0bi 候选）；届时核对：grep 尾注两态、read_file outline（含 >64 KiB 大文件绕 gate）、guide 零徽章/≤1KiB、anchor 三态文案、`run_finished` 新字段落 journal、块表指针列。
- **索引／TODO 同步**：本档落 `docs/audits/`；索引与 TODO/BACKLOG 的同步按批例在提交轮一并处理（本轮用户令不提交）。

## §8 改动文件清单（本 run worktree）

| 文件 | 件 | 摘要 |
|------|----|------|
| `orz/crates/orz-loop/src/controller.rs` | ①③⑭⑮⑯ | attach_pull_delta 重写；run_finished 新字段装配；`render_board_guide`／`resolve_journal_anchor`；钉子测试 |
| `orz/crates/orz-loop/src/model_face.rs` | ③⑮ | `FaceMarkers.journal_seq`／`pointer_for`／`parse_marker_journal_range`；块表指针列；`render_pointer_line`；marker 带 `pointer_line` |
| `orz/crates/orz-loop/src/model_stop.rs` | ⑯ | 新模块（`[RUN_END]` 解析＋开关＋语法行） |
| `orz/crates/orz-loop/src/context_scale.rs` | ④⑯ | 阶梯 192/256＋目标档建议＋删两处判断句 |
| `orz/crates/orz-loop/src/compact.rs`／`agent_loop.rs`／`lib.rs` | ③④⑯ | 类型 6→4 同步；压缩/截断调用点 pointer_line；mod 声明 |
| `orz/crates/orz-loop/src/host_exec/tool_run.rs` | ⑭⑮ | `section=guide`／`section=journal anchor=` 分发与纪律；零徽章排除；帽调整 |
| `orz/crates/orz-assurance/src/lif/rli.rs` | ① | 提醒投递计数三件套＋快照三字段 |
| `orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs` | ② | 三个描述模板追加 Call-scope lifecycle 行 |
| `orz/crates/codegen/orz-tools/src/implementations/grok_build/grep/mod.rs` | ⑪ | 三格式化函数尾注两态（行数口径＋收窄引导／budget 尾注）；＋2 钉子；grep 51/51 ✓ |
| `orz/crates/codegen/orz-tools/src/implementations/grok_build/read_file/mod.rs` | ⑩⑫ | 乱码降级告知；`outline` 字段／`build_outline`／gate 前钩子／描述补行；＋3 钉子；构造点补字段；read_file 124/124 ✓ |
| `orz/crates/codegen/orz-tools/src/implementations/grok_build_concise/read_file.rs` | ⑫ | 描述补行＋构造点补 `outline: None` |
| `orz/crates/codegen/orz-tools/src/implementations/grok_build_hashline/read_file.rs`／`cursor_rules_on_read.rs`／`types/tool_io.rs` | ⑫ | 构造点补 `outline: None` |
| `runtime/run-finished-event-payload-v0.1.schema.json` | ⑯ | ＋intent/reason/summary/unrecognized/await_channel |
| `scripts/build_orz.ps1` | ⑥ | `-WaitForHeadroom`＋软提示；BOM 修复 |

## §9 勘误与备注

- 本报告 §2 表中「落码✓」＝源码落码＋编译/测试面通过；「部分✓」＝门一 A 裁决的告知面已入描述，机械增强未落；「未落码」＝S1 定案留档、S2 未动（承接 §7）。
- 数字口径：件数 16（含 ⑯ 子项）；未闭合计数不动（46 项口径以 BACKLOG 为准）。
- 本档不含提交／推送面操作（用户令）。
- **2026-09-22 收官刷新**：本档随 ⑪⑫ 落码与 ⑤ 设计定稿刷新（§0／§1／§2／§3／§4／§5／§6／§7／§8 同步）；「落码✓」11 → **13 件**；「未落码」5 → **2 件**（⑨⑬；⑤ 转设计✓、⑪⑫ 转落码✓）。
- 观察项（低优先）：⑫ `outline` 未为 `grok_build_hashline` 变体补描述行（共享同一输入结构；低优先观察）。
