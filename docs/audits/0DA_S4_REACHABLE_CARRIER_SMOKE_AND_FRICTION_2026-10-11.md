# 0da 处理轮：S4 可达件执行＋载体活体冒烟＋卫生观测与摩擦（2026-10-11 轮）

> **轮次**：`RUN-CLI-6acac9b4`（真机 dogfood 轮；启动 2026-10-11 07:26:44＋08〔journal 记 2026-10-10T23:26:44Z〕；载体 **0.8.16**；`mode=benchmark`，allow write/shell/network；检索族关；任务文件 `.tmp-0da-task.txt`；启动器 `scripts/dogfood_launch.ps1`，墙钟缺省 `MAX_WALLCLOCK=0`）。
> **任务**：处理 `0da`（MODEL-CONTEXT-CONTROL-R2）并「直接做完」；遇需裁决项**自裁并记录**；完成后**不提交/不推送/不重建**；按项目惯例落报告并记录摩擦。
> **性质**：处理轮报告——**零源码改动**、**零台账改动**（TODO/BACKLOG/索引不动，随落账批同步）；执行面＝状态核证＋载体活体冒烟（探针 A/B）＋S4 样本-1＋S4 观测协议预注册＋终局演示（探针 Z，预注册）。撰写者＝轮内代理会话。
> **一句话结论**：0da 既有交付（S1–S3）**核证属实**；唯一剩余项 **S4 全量**因 0cy 卫生前置（S1 待放行）**如实顺延**至 0cz／0cy 联动轮；本轮把不依赖 0cy 的可达件**全部执行**并留痕。
> **状态**：未提交、未推送、未重建。

## §0 摘要

1. **状态核证**：S1〔240〕＋S2〔241〕＋S2 审查处置〔242〕＋S3〔243〕全达成；D6 更正注记已补；唯一剩余＝**S4 狗粮长轮实测**（并入 0cz S4／0cy 联动轮；判据＝D7 五观察项＋0cz 既有判据）。
2. **载体核证（活体）**：`D:\tb-eval\orz-windows\orz.exe`＝0.8.16 windows x86_64、57,531,392B、SHA256 `76904567…`（与 243 账**逐位一致**）；signer／provision／`carrier-manifest.json` v0.8.16 全对齐；本 run 载体即此件；orz 子仓 `316107f6` 清洁。
3. **活体冒烟（探针 A/B）**：0da 用户面（`blackboard_write op=clear`／`section=all` 门）**首次经在役二进制活体调用**——A＝`section=all+op=append` 如期拒绝；B＝`op=clear`（空分区）成功返回「移出 0 条」信封——**与设计/钉一致**。
4. **S4 样本-1（本 run）**：`context_manage` 在面、提醒面已触发；**调用＝0（自然零使用）**；无压缩/截断/守卫事件——低压样本，不替代 S4 全量。
5. **卫生观测（0cy 面）**：`.gsa/ledger/current.md`＝2.80MB 旧累积（mtime 9/28；无本 run 行）；会话档案模板文案与实况**错位**——0cy 设计输入（0cw 同族面现场实证）。
6. **自裁 6 条（S4-1…S4-6，§5）＋摩擦 6 条（F1…F6，§6）**。
7. **终局演示（探针 Z，预注册）**：本报告落盘后，以 `context_manage mode=clear` 在本轮自身上下文执行一次「清零→凭交接/板面续程」活体演示；结果补记于 §7.2。

## §1 状态核证（索引→批档→源码→载体）

- **路由链**：`CLI_PROJECT_INDEX.md`（v4.218）§6 `MODEL-CONTEXT-CONTROL-R2` 条目 → 批档 [`239`](239_0DA_MODEL_CONTEXT_CONTROL_R2_2026-10-11.md)／[`240`](240_0DA_S1_DESIGN_2026-10-11.md)／[`241`](241_0DA_S2_CONTEXT_MANAGE_R2_IMPL_2026-10-11.md)／[`242`](242_0DA_S2_REVIEW_DISPOSAL_2026-10-11.md)／[`243`](243_CARRIER_REBUILD_V0816_0CZ_0DA_2026-10-11.md) → TODO `P1-0da`／BACKLOG `0da` 节。
- **交付面核对**：① S1 设计稿 [`设计稿`](../MODEL_CONTEXT_CONTROL_R2_DESIGN_2026-10-11.md) 在位；② S2 落码在 orz 子树（`39116a81` 合批＋`316107f6` bump）；③ D6 更正注记已在 238 批档尾部（「勘误注记」节，2026-10-11 补记）**核在**；④ 243 重建批记录源冻结与双平台换装全链。
- **源码面**：orz 子仓工作树清洁、HEAD＝`316107f6`（＝243 源冻结）；父仓 HEAD＝`dce03a03`（245 补记），工作树唯一未跟踪件 `_hdr_new.txt`（见 F3）。
- **载体面（活体）**：三件套尺寸/哈希与 `carrier-manifest.json` 逐项对齐 243 账；`.0.8.15-bak` 链在侧（`orz.exe 57,135,104B`，与账面 +396,288 差一致）。读数原文见 §8。
- **结论**：S1–S3 达成的**账面与实物一致**；0da 无其他在册未竟码面/文档面项。

## §2 载体活体冒烟——replace 族（探针 A/B）

- **动机**：S3 判据为**字节面**（字符串在二进制中 0→N）；S2 判据为**测试面**（dev 环境钉组）；0da 的用户面（`op=clear`／`section=all` 组合门）在**在役载体二进制的活体调用**上未被直接触过。本轮以零破坏探针补此空档。
- **探针 A（组合门）**：`blackboard_write(section=all, op=append)` ⇒ **如期拒绝**，回执原文：「invalid blackboard_write section=all：all 仅与 op=clear 组合合法（all 不是通配写目标；写入请指定 plan|notes|findings）」——零副作用。
- **探针 B（清空路径）**：`blackboard_write(op=clear, section=notes)`（目标分区为空）⇒ **成功**，回执原文：「已清空黑板 notes（移出 0 条；旧条目全量留档于 journal 与会话卷，可回查）；live 水位【0.3M/10M】。清空后该分区读面为（无）。」——与设计 §6.1（清空为非占位；journal/会话卷留痕）一致；清空后读面指认「（无）」实际回读核在。
- **边界**：`mode=clear`／`mode=clear_all`／`mode=compress` 不在本组；自清零仅以终局演示（探针 Z）单列；`section=all` 的**正向**路径（`all+clear`）未在自身上执行（会触及在用的 plan 分区）——留存为联动轮/后续可选补项。

## §3 S4 观察样本-1（本 run 读数，低压）

| 考察项（0cz/0da S4 面） | 本 run 读数 |
|---|---|
| 工具在面 | ✓（`tool_availability_check`：complete 含 `context_manage`） |
| 提醒面 | ✓ 已触发（上下文水位通知＋权限提示多次；模型响应＝**延后**〔自评无需〕） |
| 用不用（使用率） | **0 次**（54 个 `tool_started` 中 `context_manage`＝0；窗口全程 ≈0.26–0.30M/10M 无压力） |
| 时机／用得对不对 | 无样本（未使用即读数；提醒到达后模型显式判断「无必要」） |
| 清零后再触兜底 | 无样本（本 run 无清零，亦无 H1/T1/700K 触发） |
| 二次清零／keep 使用 | 无样本 |
| replace 族使用分布 | 自然使用 **0**；探针使用 2（A 拒绝／B 清空；**非自然使用**，已标注，见 §2） |

- **读法**：样本-1＝**低压轮**；不能作为 S4 判据的达成或否定证据；用途＝「工具可用性＋提醒触发＋模型自评路径」的现场证据。S4 全量应以联动轮（受控长轮）为准。
- **§3.2 S4 观测协议预注册（联动轮执行用）**：

| # | 观察项 | 数据源 | 采集方法 |
|---|---|---|---|
| ① | 任务漂移 | 清零后各轮 model_output／plan／handover 引用 | 清零后轮次中任务指涉与 handover 关键词覆盖率（人工判读） |
| ② | 清零后再触兜底 | events（压缩/截断/守卫事件）＋estimate 读数 | 清零后越 H1／T1 时 estimate 必须回落（D1 修复判据） |
| ③ | 二次清零形态 | journal `ContextCompressed{mode=clear}` 计数与边界值 | 单调守卫（边界倒退⇒中性 exit 0）实证 |
| ④ | keep 使用形态 | 调用参数 `keep_recent_rounds` 分布 | 参数取值直方＋保留窗四方一致抽查 |
| ⑤ | replace 族四操作分布 | 工具调用计数（`mode=clear`／`clear_all`／`compress`／`op=clear`） | 逐 run 计数与时机标注 |

（数据源根：`.gsa/runs/<RUN>/events.jsonl`、`.gsa/compaction/`（blocks、clear.md）、journal、会话卷。）

## §4 卫生观测（0cy 前置面证据）

- **ledger 实况**：`.gsa/ledger/current.md` ＝ 2,801,888 B（2,475,363 字符）；mtime **2026-09-28 06:44:40**；内容为**旧轮次**机械行累积（样例行：轮 38／114／136）；**检索 `6acac9b4`／`0cz`／`0cw`／`245`＝0 命中**；`0da` 134 命中皆为哈希/ID 偶然子串（已逐样核查）。
- **本 run 实际写入面**：`.gsa/runs/RUN-CLI-6acac9b4/events.jsonl`、`.gsa/session/terminal/call_*.log`、`.gsa/process_trees/call_*.json`（今日足迹 40＋件）；`ledger/current.md` 今日**零写入**。
- **模板文案错位**：轮内「上下文分块」提示称机械行「外挂存档于 `D:\CLI\.gsa\ledger\current.md`（本会话内固定）」——与上实况不符（该文件为旧累积、未重置、无本 run 行）；模型若按文案检索将命中**旧轮次**数据。→ 文案/机制错位，**0cy S1/S2 设计输入**。
- **活区残留**：`archives/`＝58 件、`orientation/`＝5 件、`activations/`＝5 件在活区（0ct 读全开下可读）——0cw 污染同族面在 `D:\CLI` 的现场复现；**0cy 价值再证**。
- **模型触碰面自审（如实记）**：本轮为**审计目的**读取 `ledger/current.md` 头/尾行共约 6 行＋`0da` 样例 3 行（跨会话机械行）；`conversations/orientation/activations` **未读内容**（仅目录计数）；历史 `.tmp-*` 任务文件读取 9 件（措辞惯例对照，工作区面）。在无 0cy 防护下，跨会话面「可读即可触」——本项自身即证据。
- **板面卫生**：本 run 黑板为**会话级空起点**（goal/plan/notes/findings 起点皆空）、全程会话级增删——板面卫生良好；污染面集中在**文件面**（ledger 等）。

## §5 裁决记录（本轮自裁；授权＝任务书「遇需裁决项可直接衡量并裁决，记录即可」）

| # | 事项 | 裁决 | 理由 |
|---|---|---|---|
| S4-1 | S4 全量可否本轮执行 | **不可**；执行可达件（样本-1＋冒烟＋协议预注册＋终局演示） | 联动轮构成不全（0cz S4 同待）＋0cy 前置未达；不 fake、不越界 |
| S4-2 | 破坏性面 | 除探针 Z（终局、预注册的自身清零演示）外，一切破坏性动作不做（clear_all／二次清零／keep>0／子轮等） | 现场保护＋科学有效性；Z 标注为**机制演示**、不主张 S4 判据 |
| S4-3 | 是否跑新子轮做测量 | **不跑** | 成本/单样本/0cy 未达 → 收益低、有效性存疑；选项登记 §7.1 |
| S4-4 | 0cy 范围 | 不代做 0cy（S1/S2 不动）；本轮观测作为 **0cy 设计输入**（§4/F1/F2） | 范围纪律（0cy 为独立在册项，S1 待放行） |
| S4-5 | 台账 | TODO/BACKLOG/索引不动；报告即留痕，随落账批同步 | 0bw X5 先例＋用户令「报告即可」 |
| S4-6 | 探针边界 | 仅零破坏探针（A/B/Z）；全部如实标注；Z 预注册 | 审计洁净 |

## §6 摩擦记录

- **F1（机制/文案错位·0cy 面）**：会话档案模板称机械行外挂于 `ledger/current.md`「本会话内固定」，实况＝旧累积未重置、无本 run 行（§4 详）。
- **F2（活区残留·0cy 面）**：`ledger/current.md` 2.80MB 旧累积仍在活区；`archives`/`orientation`/`activations` 可读（0cw 同族、0ct 读全开下即为污染通道）。
- **F3（工作区残留）**：`_hdr_new.txt`（238 批索引头临时件）未跟踪残留于工作区根（244 批落账未清；本轮未动）。
- **F4（环境·编码族）**：PS 5.1 无显式编码读 UTF-8 无 BOM 文件呈 GBK 乱码（本轮回读 ledger/run_started 时复现 ×2；`-Encoding UTF8` 重读即净）——0bw F7 同族。
- **F5（基线注记精度）**：run 基线注记称 worktree「含未提交改动」；实测唯一未跟踪件＝`_hdr_new.txt`（无 M 类条目）。
- **F6（依赖阻塞）**：「直接做完 0da」受 0cy 前置阻塞，全量不可达（详见 §7.1）。

## §7 未竟与后续

- **§7.1 S4 全量（唯一未竟）**：并入 0cz S4／0cy 联动轮；**前置＝0cy S1 放行→S2/S3 落地**；建议序（沿既定批序）：0cy S1（收货时机／archives 布局／契约面／净室隔离裁决）→ 0cy S2/S3 → 联动轮（0cz 判据＋0da D7 判据＋0cy 真机核证并入）。**本轮的样本-1 与协议预注册（§3.2）可直接服务该轮。** 遗留可选：`all+clear` 正向路径活体补项；子轮测量选项（S4-3 否决理由在案）。
- **§7.2 终局演示（探针 Z）结果**：**已执行** `context_manage(mode=clear, handover=…)`（`keep_recent_rounds` 缺省；handover＝本轮交接摘要）。**回执（逐字）**：「清零已生效：边界＝第 41 轮；已移出 1-4（共 40 轮 ≈283603tk token，模型面估算）。交接摘要已写入黑板（notes）。清零存档：`.gsa/compaction/blocks/6acac9b4-clear.md`（边界前全部轮次逐字留档；read_file offset/limit 分页）。用 blackboard_read 回读工作计划与结论。清零后当前读数 ≈3099（内容估算，未含常驻框架段与静态开销）。」
- **§7.2-b 盘面核验（清零后实测）**：① 存档在位＝`6acac9b4-clear.md` 331,451 B／2,114 行（首部＝「# ORZ 清零现场留档（轮 1–40）／边界: 第 41 轮／消息区间: [0, 107)／run=RUN-CLI-6acac9b4／生成 2026-10-10T23:39:27Z」；正文自消息 [0] 会话起点逐字，尾至 [106]〔findings 写入〕）；② 块档 `6acac9b4-block-0001…0004.md` 在位（86,311／57,964／44,647／52,408 B；mtime 均＝清零时点 7:39:27 AM）；③ 交接摘要回读核在（notes【交接摘要】在册）；④ marker／分块表刷新：块#1–6 相继转「已清零」（含清零后新物化之 #5、#6），#5 现场可见「残段（仍在增长）≈16023tk·26-26」→「已清零 ≈39395tk·26-27」之态移；⑤ 水位：清零后 ≈3,099（内容估算）；清零前窗读 ≈0.26–0.30M/10M（移出 ≈283.6Ktk，量级对账一致）。
- **§7.2-c 四环对照（对照 §7 预注册）＋新观察（如实记录，不下结论）**：**交接→落盘→marker→水位重置四环全部实到**。新观察：a) 工件命名＝`<session>-clear.md`（与块档 `<session>-block-NNNN.md` 同族、同目录）；b) 清零后**渐进物化**：回执即时读数「已移出 1-4（共 40 轮 ≈283603tk）」之后，块#5、#6 于后续轮次刷新中相继现身（至本补记时点：块#1–6＝29 轮／≈209,473tk 已物化）——余下轮次之块物化待续，全量口径与已物化口径之差随物化推进自然对账（无异常初判）；c) 「残段」现场形态与「工作现场/残段不参与压缩截断」语义首次可见（见上）；d) 清零后任务续接凭「handover＋黑板＋盘面」完成（本 §7.2 补记与收束序即活体样本）。**判据说明**：本节＝**机制活体演示**（`mode=clear` 清零路径首执行真机实证）；**不主张 S4 判据**——S4 全量仍按 §7.1 并入联动轮。
- **§7.3 台账同步**：随落账批（计数不变 59；`0da` S4 状态行注记「可达件已执行＋全量待联动轮」）。

## §8 附：读数原文（节选）

```
=== 载体 ===
orz-build-info: version=0.8.16 os=windows arch=x86_64 profile=release
orz.exe      57,531,392B  sha256=76904567f4ad8fae742168bff99b29630dfa5c29b6a0f6475fde5b70c3e55b0a
orz-signer.exe  6,740,480B sha256=1919b56fa7c9d37ba78ac1c8c71ba43cfc37d97dd84c85f6b77c76478df13d39
orz-acaf-provision.exe 6,640,128B sha256=446d8cc773e18144735d50b965ed02895f46d9b77fc498bbc2a6a5235ecfb211
carrier-manifest.json: kind=orz-carrier-manifest version=0.8.16 generated_at=2026-10-11T06:50:00Z
（.0.8.15-bak 链在侧：orz.exe 57,135,104B；与 243 账 +396,288 一致）

=== 探针回执 ===
A: invalid blackboard_write section=all：all 仅与 op=clear 组合合法（all 不是通配写目标；写入请指定 plan|notes|findings）
B: 已清空黑板 notes（移出 0 条；旧条目全量留档于 journal 与会话卷，可回查）；live 水位【0.3M/10M】。清空后该分区读面为（无）。

=== 本 run 工具直方（tool_started；至撰写前快照）===
总数 54：run_terminal_cmd 26／read_file 12／grep 9／blackboard_read 4／blackboard_write 3／context_manage 0

=== ledger 检索 ===
current.md 2,801,888B mtime 2026-09-28 06:44:40；检索 6acac9b4=0／0cz=0／0cw=0／245=0；0da=134（偶然子串）

=== git 状态 ===
父仓 HEAD dce03a03；工作树 ?? _hdr_new.txt＋?? 本报告（终稿核验两件）；orz 子仓 clean@316107f6
```

关键词：0da、MODEL-CONTEXT-CONTROL-R2、S4 可达件、RUN-CLI-6acac9b4、0.8.16 活体冒烟、op=clear、section=all、context_manage 零使用、0cy 卫生前置、ledger 活区残留、探针 Z、摩擦、未提交。
