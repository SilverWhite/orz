# 354 批：系统控制台设计（模型面：可发现／可组合／只读优先）

> 日期：2026-10-25
> 用户令：当前内部有没有什么其他你想修改的地方或者摩擦？我想做的是那种通用化的、特别方便模型使用和上手的"系统控制台"；按你的判断和建议做，先落一份设计文档。
> 范围：只读勘定 + 设计，不含落码。承接 353 批（回合原子性／终态声明／时间基准）。
> 计数：65 不变（设计件，未立项；落码分批见 §8）。

## §0 结论

摩擦的根不是"命令不够多"，是**每个面的形状不一样**：`read_file` 有 sha/range 信封，`grep` 只给 "at least"，`run_terminal_cmd` 只给裸 stdout，`blackboard_read` 有自己的折叠语义，`events.jsonl` 又是另一套。模型每换一个面就要重新学一遍"这货怎么返回"。

控制台按三层做：**capabilities（能力清单，可发现）→ inspect（只读诊断族，零审批）→ Envelope v1（统一结果信封，可组合）**。三层之外的一切（终端心跳、超时、断 run 注入、审批可见）都是"把状态作为机械事实重新给"，不要求生成式模型自己记住。

勘定已确认两处"我记得做了"的部分：**终端解码梯确实已做**（`util::encoding::decode_text`，截断/未截断两臂都走）；**grep 的 `total_matches` 也确实已算**。缺的是**上游代码页**与**面层渲染**——见 §1.1／§1.2。

## §1 现状勘定（证据面）

### §1.1 终端编码：梯子已做，坑在子进程代码页

- 已做：`orz-tools/src/computer/local/terminal.rs:483-513` 的 `to_result()` 对**截断臂与未截断臂**都调 `crate::util::encoding::decode_text`，标签经 `merge_encoding_labels` 合并；`maybe_truncate()`（:545-560）在解码后的文本上切片，不用 `from_utf8_lossy` 预摧毁字节（0bv 并件三已修，单测 `truncated_gbk_output_decodes_via_capture_ladder_not_lossy` 钉死）。
- 真实缺口：**PowerShell 5.1 的 stdout 代码页**。`python -c` 往 stdout 写 UTF-8，PowerShell 用 cp936 收下再转码，工具拿到的已是 GBK 字节流；解码梯"正确地"把它解成 `鎴戜滑` 这种乱码——**垃圾进、垃圾出，梯子无罪**。本会话至少 5 次因此改道"写 UTF-8 文件再 read_file"。
- 结论：351 的记忆没错；要补的是**启动器侧的代码页**（`chcp 65001` ＋ `[Console]::OutputEncoding/InputEncoding` ＋ `$OutputEncoding`），以及回执里的三读数。

### §1.2 grep：总数已算，面层只给 "at least"

- 已做：`orz-workspace/src/file_system/content.rs:28/157/219` 计算并回传 `total_matches`。
- 缺口：面层渲染成 `Found at least N matching lines`，**没有 total、没有下一页游标**；模型只能收窄 pattern 猜。`head_limit` 在 `console.rs:221` 有声明，但无 `next_cursor` 语义。
- 结论：不是实现缺口，是**面层没把已有数据交出来**。

### §1.3 读面沙箱：是摩擦，不是防线

- `read_file`/`grep` 限 workspace（本会话 3 次 `path escapes workspace sandbox` → `Copy-Item` 绕行 → 读 → 删）。
- 但 `run_terminal_cmd` 本来就能 `Get-Content C:\任意路径`——**读面沙箱只挡住模型的手，挡不住模型的脚**。用户裁决方向：撤销读面限制。

### §1.4 事件流：11 类事件、payload 异质、无诊断面

- `events.jsonl` 现有 11 种 `event_type`（run_preflight / tool_availability_check / run_started / prompt_submitted / request_header_change / face_fingerprint / model_output / permission_* / control_ticket_* / tool_started / tool_completed / write_control_review / mechanical_audit_update / run_finished…），payload 各写各的。
- 唯一窗口读数是 `face_fingerprint`（`message_count/total_chars/face_sha256`），排查时全靠手工解析。**没有"这次 run 的时间线"这一等公民。**

### §1.5 反馈：`tool_started` 之后是静默

- 无心跳、无"已运行 Ns 无输出"、无单工具超时回执；断 run 后侧车不落盘（353 §2），下一轮从旧现场起步。这是本会话最贵的一次事故。

### §1.6 其余

- 审批模式（YOLO / confirm-first）对模型不可见，导致"等你批准"这类错话术。
- `read_file outline=true` 只认 Markdown 标题，对 `.rs/.py` 无用。
- `[emoji 剥离] 1 处` 不说剥了哪个字符。

## §2 设计原则

1. **可发现**：能力先于命令——模型读一份 `capabilities` 就能上手，不必试错。
2. **可组合**：所有面共用一个 `Envelope`，分页/截断/编码/校验和同一套字段。
3. **只读优先**：诊断面零审批、零副作用；写面才走门禁。
4. **状态外部化**：不要求模型记住状态，每轮把"当前状态"作为机械事实重新给（幂等、可重放）——这是生成式模型唯一可靠的维持方式。

## §3 三层形状

### §3.1 capabilities（能力清单）

`orz capabilities --json`：每个工具/分区一条 `{name, kind(read|write|exec), params_schema, result_schema, needs_approval, readonly, one_line_usage}`。同时作为 `orz --help` 与模型面 `guide` 的单一来源（不再两处维护）。

### §3.2 inspect（只读诊断族，零审批）

| 命令 | 输出 |
|---|---|
| `orz inspect session <id>` | 轮次、上下文水位、压缩/清零历史、未回收工具调用、最近失败、租约/关闭态 |
| `orz inspect run <id>` | 单 run 时间线：模型轮 → 工具 → 审批 → 结果 → 耗时 → 终态（`--json` 可机读） |
| `orz inspect context` | 当前窗口构成：分块表、marker、字符/token、可回放指针、`face_sha256` |
| `orz inspect events <id> --json` | 结构化事件流（稳定 schema，供 pipeline） |

四个面全部只读：不写盘、不触发审批、不进模型面（除非模型主动调）。

### §3.3 Envelope v1（统一结果信封）

```json
{ "ok": true, "data": "...", "meta": {
    "chars": 12034, "lines": 312, "encoding": "utf-8",
    "truncated": false, "next_cursor": null,
    "sha256": "...", "path": "...", "took_ms": 42 } }
```

- `read_file` / `grep` / `run_terminal_cmd` / `blackboard_read` / `inspect` 一律走它；
- `next_cursor` 是唯一分页语义（现在是 read_file 的 offset、grep 的 head_limit、terminal 的截断三套）；
- 文本渲染保留（人可读），但 `--json` 必须给同一份数据。

## §4 专项设计

### §4.1 终端编码（354a）

- 启动器统一：`chcp 65001` ＋ `[Console]::OutputEncoding = [Text.Encoding]::UTF8` ＋ `[Console]::InputEncoding` ＋ `$OutputEncoding = [Text.Encoding]::UTF8`；`python` 调用加 `PYTHONIOENCODING=utf-8`。
- 回执三读数：`chars / lines / FFFD`（替换字符计数）——`FFFD > 0` 即判"解码可疑"，并在回执里提示"建议改用写文件+read_file"。
- 解码梯保留（它是兜底，不是主路径）。

### §4.2 grep 分页（354a）

- 面层渲染 `total_matches` 与 `next_cursor`（按 `(file,line)` 续页）；
- 命中数 > limit 时输出 `total: N, shown: M, next_cursor: <token>`；
- 与 Envelope 的 `next_cursor` 同一实现。

### §4.3 读面沙箱撤销（354c，用户已裁决方向）

- `read_file`/`grep`/`outline` 取消 workspace 限制；
- 写面（`search_replace`/`write_control_review`）**不动**；
- 敏感路径（`.ssh/`、`.aws/`、`*.pem`、`*.key`、keystore、`.env`）读前给一行提示（`[敏感面] 该路径疑似密钥材料`），**不拦截**；读取仍记 journal（审计面不变）；
- 理由：`run_terminal_cmd` 已可读任意路径，读面限制只增加摩擦、不增加安全。

### §4.4 时间轴：为什么不用 LIF/RLI，怎么放黑板

**LIF/RLI 不能当时间轴**：它是**特征与预测**（域标签、`T̂`、越线读数），不是"发生了什么"的记录。把推断混进记录，正是 353 删掉 RLI 推送的那条教训。时间轴必须由**机械层从 journal 投影**，零模型、零推断。

**形态**：

- 机械投影 `timeline(session)`：`轮次 → 工具 → 结果（exit/耗时/失败类）→ 审批 → 终态`，只读、可按 `run/round/tool` 过滤；
- 黑板常驻**只放索引行**（≤400 字符）：最近 N 轮一行摘要 + 未回收调用 + 一个锚点；明细按需 `inspect run`／`blackboard_read section=journal` 拉；
- **锚点必须 session-scoped**：现有 journal 指针只在本 run 有效（353 §1 已证），改为 `journal/<session>.jsonl` 或把锚点解析落到 `inspect`；
- 理由：常驻全量会吃窗口（现在的分块表已约 2K token/轮），索引行的信息密度最高。

### §4.5 机械探查与机械反馈（354a）

针对"生成式模型无持久内部状态"，只做三件机械事：

1. **心跳**：`tool_started` 后每 N 秒（默认 20s）若仍无输出，注入一行 `[工具心跳] <tool> 已运行 Ns，暂无输出`（只在无输出时，不刷屏）；
2. **单工具超时**：默认 300s（可配），到点回 `tool_completed(error=TIMED_OUT)`，附命令体首 200 字与已捕获输出尾部；
3. **断 run 对账**：下一轮开头注入 `[上次 run 中断] 未回收 <tool>(<call_id>)；结果未知，落盘副作用可能已发生`（353 P0-2）。

原则：**每轮重放状态，而不是要求模型记住状态**。

### §4.6 审批模式可见（354c）

每 run 注入一行：`[permission] mode=interactive-yolo source=default`（只读事实，不参与判定）。模型据此决定是否需要"等批准"话术。

### §4.7 read_file 代码 outline（354c）

`outline=true` 支持：Rust（`fn/struct/enum/impl/mod/trait`）、Python（`def/class`）、TS/JS（`function/class/interface`）、Markdown（标题）。返回 `L<line>: <kind> <name>`——这是"控制台"最该有的导航能力。

### §4.8 emoji 剥离可解释（354c）

回执改为：`[emoji 剥离] 2 处：U+1F600@L12、U+2705@L40`；或直接不剥、只提示。

## §5 Envelope 字段表

| 字段 | 类型 | 语义 |
|---|---|---|
| `ok` | bool | 工具是否成功（失败也带 data 与 error.code） |
| `data` | string/obj | 主体（文本或结构化） |
| `meta.chars/lines` | int | 交付文本的规模 |
| `meta.encoding` | str | 实际产出文本的解码阶段（`utf-8`/`gb18030`/`ladder:gbk→utf-8`） |
| `meta.truncated` | bool | 是否截断 |
| `meta.next_cursor` | str? | 唯一分页游标 |
| `meta.sha256` | str | 交付文本的哈希（编辑前核证用） |
| `meta.took_ms` | int | 工具自身耗时 |

## §6 验收

- `orz capabilities --json | jq '.tools[] | select(.readonly)'` → 只读面清单完整；
- `orz inspect run <id> --json` → 时间线含每轮工具/耗时/终态，且与 `events.jsonl` 逐条可对；
- `grep` 命中 > limit → 输出含 `total` 与 `next_cursor`，续页不重不漏；
- `run_terminal_cmd` 跑 `python -c "print('中文')"` → 回执 `FFFD=0`、正文为中文；
- 断 run 后下一轮首帧含 `[上次 run 中断]`；
- `read_file` 读工作区外路径 → 成功（不再 `escapes workspace sandbox`）。

## §7 不做／边界

- 不做"工具面无限扩张"：新增只读诊断走 `inspect` 子命令，不进模型工具面；`finish`/`ask` 待工具面版本化后再议。
- 不做时间轴的模型推断（只投影事实）。
- 不把 RLI 重新做成推送面。

## §7.5 机械层的复杂度预算（用户 2026-10-25 约束）

> 用户原话："orz 本身的目标就是降低模型的压力并且辅助模型更好的工作，只要这一补强不会过于繁杂就行。"

每条机械补强必须**至少满足一项**，否则不做：

- **A 减负**：减少模型必须记住或必须猜的东西（状态重放、total/游标、编码读数）；
- **B 暴露失败**：把原本静默的失败变成可见事实（心跳、超时、断 run 注入）。

同时受三条硬预算约束：**每工具结果至多追加 1 行**；**每 run 至多新增 1 类注入**（`[上次 run 中断]` 与心跳合并计）；**零新增模型调用、零新增模型工具**。超预算的补强一律退回设计。

## §7.6 子代理（scout）裁决

**本批不用 worker 子代理。** 理由：354a 的三处改动共享同一套契约（终端执行器 / grep 渲染 / 工具结果信封），并行写面会撕裂契约，且 `cargo` 的 target 目录是全局锁，并行编译互相排队；子代理的产出还要回灌主上下文，三处小补丁的交接成本高于收益。

**适用的形态是只读 scout，不是 worker**：一次性 fan-out、结果可压成表格的活（"扫全部输出解码点""把 11 类事件字段整理成表""逐文件核对 21 处时间戳替换"）。形态约束：只读、零写面、返回 ≤2K 的结构化表、必须带 `file:line` 证据。**不适用**：需要连续设计判断、需要改同一批文件、需要跑门禁的活。

多 agent 协作本批不做（用户已裁决：整体性优先）；scout 只在"取证面"出现，不在"执行面"出现。

## §8 分批

| 批 | 内容 | 目的 |
|---|---|---|
| **354a** | §4.1 编码（启动器代码页＋三读数）、§4.2 grep 分页、§4.5 机械探查/反馈 | 直接消掉本会话最大三块摩擦 |
| **354b** | §3.1 capabilities、§3.2 inspect 族、§3.3 Envelope v1 | 控制台骨架 |
| **354c** | §4.3 读面沙箱撤销、§4.6 审批可见、§4.7 代码 outline、§4.8 emoji 回执 | 收口 |

**354a 执行状态（2026-10-25）**：

- **T2 已落码**：终端回执三读数独立一行 `[output] chars=… lines=… FFFD=…`（`grok_build/bash/mod.rs`）；`exit: N` 头行保持原样。`cargo test -p orz-tools --lib bash` 237 passed。
- **T1 已落码**：`apply_child_env` 给所有子进程加 `PYTHONIOENCODING=utf-8` / `PYTHONUTF8=1` / `PYTHONLEGACYWINDOWSSTDIO=0`（`computer/local/terminal.rs`），放在基础层允许 login/request 覆盖。编译通过。
- **T3 改判**：真游标不是渲染问题。grep 在 `head_limit` 处**提前 kill rg**（`EXACT_FIT_PROBE_TIMEOUT` / early-kill 预算），因此既没有真 total，也没有可续读的位点；要做 `offset` 必须把读取预算抬到 `offset + limit`，并改 early-kill 记账与 streaming 路径。**移入 354b（与 Envelope v1 的 `next_cursor` 同批）**，不在 354a 硬做。
- **T4 拆分**：T4a＝工具结果里补"静默时长"（`took=…s`，让模型知道那段时间过了多久）；T4b＝`tool_started` 心跳 journal 行与断 run 注入（跨 host/loop）。T4a 可独立落，T4b 与 353 的 P0-2 同批。
- **T4a 已落码**：`format_default_prompt` 增 `elapsed: Option<Duration>`，三读数同行补 `took=…s`（守住"每工具至多追加 1 行"预算）；`BashTool::run` 传 `run_started_at.elapsed()`。`cargo test -p orz-tools --lib bash` **238 passed / 0 failed**（新增 `default_prompt_reports_took_when_elapsed_is_known`）。
- **T4b 状态更新**：断 run 事实注入已随 353 P0-2 落码（`RepairMode::Sidecar` → `[上次 run 中断]` 结果）；`tool_started` 心跳 journal 行仍未做。

**354 落码批（2026-10-25，[`354A_B_C 落码批档`](audits/354A_B_C_SYSTEM_CONSOLE_IMPL_2026-10-25.md)）**：T4b＋354b＋354c 全落——`tool_heartbeat` 心跳全链（默认 20s，`ORZ_TOOL_HEARTBEAT_SECS`，无法官族同 prompt_submitted 姿态，Python 合规 17/17）；grep 真游标（`offset`＋读取预算 offset+limit＋`[page]` trailer，模型面 total 预算截断时为 ≥ 下界——精确 total 属 workspace 服务端路径）；inspect 四面＋`orz capabilities`（Envelope v1 首批公民）；读面沙箱撤销＋`[敏感面]` 提示（会话卷窗口契约不动）；outline 四族语言；emoji `U+码点@行`；审批 `[permission]` 行。附带：P0-3 wrapper 收口（0dq）、350 批遗留 parity 断管修复。边界：Envelope 对模型工具面的整面字段统一登记 0dr 剩余（read_file/terminal/blackboard_read 已有等价 meta，信息等价）；载体未重建（用户令）。

**355 批勘误与补全（2026-10-25，[`355 批档`](audits/355_353_354_REVIEW_REMEDIATION_2026-10-25.md)）**：审查发现 T1 落码平台错位（env 变量在 `#[cfg(unix)]` 内、Windows 零增量；启动器代码页前置零落码）——355 批重做（`shell.rs` 启动器前置＋`PYTHONLEGACYWINDOWSSTDIO` 补 Windows 臂＋FFFD 失明判据扩展）；capabilities 补全 14 条＋`coverage` 如实（loop 侧无 host 单源部分策展，剩余登记 0dr）；inspect events 游标改绝对行号、context_view 改数字序、grep offset 越界改显式报错、敏感面 pattern 补全。
