# 279 批：0dd/0de 审查处置落账——文档面勘注勘误＋账面登记（orz `1ecad57c`）（2026-10-16）

> **性质**：**落账批**——对一次全面审查（已完结，对象＝未推送 275/276〔0dd 线〕＋
> 277/278〔0de 线〕四批）查出的全部问题做**文档面处置与账面登记**；orz 代码处置批
> 已先行落码（orz `1ecad57c`，父提交 `644f6b01`，五文件 +131/−5：`CONTRIBUTING.md`
> ＋`acp_server.rs`＋`agent_loop.rs`＋`dispatch.rs`＋`bridge.rs`）；父仓本批只动文档
> 与登记面。零跑批；**计数不变 61**。
> **结论先行**：0dd 设计档两处勘注（§1 触发道四→五〔on-demand 归档道〕＋§7 S3 漂移
> 窗口风险句）＋276 批档/TODO「1.5M 字符」勘误（实测 1,088,890 字符 ⇒ 侧车估算
> ≈544,448）＋0de 设计档两处勘注（D3 带工具轮 RUN_END 边界有界性＋D6 TUI 侧补齐
> 勾注）＋0cy 勘注指代措辞修正＋三不取项如实登记；orz 面六件随 `1ecad57c`（N1 前提
> 勘误／N2 断言补强／N3 锐化／TUI bridge 双字段／user_cancel 加固／注释与 PROTOC 注记）。

## §1 审查来源与范围

对象＝未推送四批（275 0dd S1 设计／276 0dd S2 落码／277 0de S1 勘定／278 0de S2 落码）
的全面审查（已完结）：查出 **P2×6＋观察项**。处置映射＝①0dd §1 枚举缺口〔五道勘定〕
②0dd §7 S3 漂移窗口无风险句〔风险注记〕③276 批档 N1 前提读数失真〔勘误〕④TODO
S2 行同源失真〔勘误〕⑤0de D3 边界分支未讨论〔勘注〕⑥orz-tui「估 ~0K」漏修面
〔已修＋登记〕⑦陈旧注释〔已改 2K，列名〕⑧user_cancel flaky 预存实锤＋加固〔登记〕
⑨N2 断言补强〔登记〕⑩N3 锐化〔登记〕⑪CONTRIBUTING PROTOC 注记〔登记〕⑫0cy 勘注
措辞〔修正〕；不取三项（§4）。

## §2 orz `1ecad57c` 逐项处置（审查发现 → 落码）

| # | 件 | 内容 |
|---|---|---|
| 1 | N1 前提注记勘误 | 测试注释「1.5M 字符」勘正——实测 1,088,890 字符 ⇒ 侧车估算 chars/2 ≈544,448、≥500K 余量 ~8.9%；字符数定长故前提确定性成立；零行为变更 |
| 2 | N2 断言补强 | 推迟票钉解包验 footprint 段在且 `counts.bytes>0`（照既有 gzip→envelope 模式）＋ARC `session_archive` payload 无 `incremental` 键（close 道恒非增量）——276 批「含足迹包」声称自此有测试实质 |
| 3 | N3 锐化 | 抑制臂与对照臂 `requests.len()==3` 显式断言（脚本 3 响应实测确定） |
| 4 | D6 同族收口 | orz-tui `bridge.rs` `ContextCompressed` 双字段兼容：`estimated_tokens_after` 缺位/0 回退 `target_tokens`（web `644f6b01` 条件镜像）；回退落解析层 ⇒ `projection.rs` 渲染面零改动；+1 bridge clear 形单测渲「估 ~100K」非 0K |
| 5 | user_cancel 加固 | 30ms 调度竞窗（旧固定 sleep＋`len==1`）改轮询 `ToolStarted{target:"internal_retrieval"}` 落 journal 后再 cancel；基线差分（worktree @`4749f6b1` 三跑同败）实锤**预存**、278 免责——可作 0aq「负载敏感测试」RS 家族证据引用（不新开项、计数不动） |
| 6 | 注记两件 | `agent_loop.rs` 钉陈旧注释「钉在 1K」→ 2K（代码同批兄弟钉均 2K；仅注释）；`CONTRIBUTING.md` 新增极小 `## Local builds` 节（裸 shell 需 `PROTOC=D:\CLI\orz\bin\protoc.exe`） |

## §3 父仓文档面处置逐项

1. **0dd 设计档 §1**：dated 勘注（原文逐字保留）——「触发入口共四道」漏列**第五道＝
   on-demand 归档**（`archive_session_on_demand`，orz `acp_server.rs:867`；经 orz-bin
   CLI `archive` 与 orz-web `POST /api/archives/{s8}` 到达，同产 session_archive/ARC）；
   属用户显式动作、合主权语义，非肇事面。
2. **0dd 设计档 §7**：S3 漂移窗口显式风险句——S3 载体重建放行前，0.9.0 在役载体仍含
   run 尾自动归档行为、真机用户仍会踩 266 同款摩擦；窗口期以本句为登记。
3. **276 批档 §2**：N1 前提读数就地勘误（原文保留，dated）——「1.5M 字符」系估算；
   实测 1,088,890 字符 ⇒ 侧车估算 ≈544,448（≥500K 余量 ~8.9%）；orz `4749f6b1` 提交
   信息同源失真按不改写历史纪律留档。
4. **TODO P1-0dd S2 行**：就地勘误「1.5M 字符长答」→「≈1.09M 字符（估算 ≈544K）」
   ＋行尾〔279 批勘误〕（沿 TODO 面就地更正先例）。
5. **0de 设计档 D3**：dated 边界勘注——「带工具轮消息声明 RUN_END 而 run 继续下一轮」
   分支未讨论就此登记；后果有界（告知块不持久化、模型可下轮重发摘要、审计零损失），
   与 D4 非可信门裁决不冲突。
6. **0de 设计档 §4/§6（D6）**：TUI 侧补齐 dated 勾注——`bridge.rs` 双字段兼容＝279
   补齐（orz `1ecad57c`），`projection.rs` 渲染面零改动裁决。
7. **0cy 设计稿 §2 勘注**：指代措辞就地修正「上行」→「上表『500K 增量』行」（纯指代
   修正、语义零变——所指为表格行且中间隔有内容）。

## §4 不取项与理由（如实登记）

1. **N1/N2 的 200ms sleep 落地窗**：审查判风险可接受，维持现状、不改。
2. **N4 自定义 SeqHost 与 SeqOutputsHost 近同构**：测试基建复用收益低、触碰面风险
   不值，维持现状。
3. **267 批档 F3 行 `projection.js:198` 旧行号**：沿既有行号漂移惯例，不随批再勘。

## §5 验证读数

- `cargo test -p orz-host --lib -- --test-threads=1`：**365/0**（+0 钉）。
- `cargo test -p orz-loop --lib`：**877/0**（3 ignored；`user_cancel` 隔离 5/5 过）。
- `cargo test -p orz-tui`：**170 过／9 败**（9 败与改前基线逐一相同＝acp_client×6＋
  runner×3 ACAF 预存；+1 新过＝bridge 单测）。
- clippy：loop **96=96**／host **114=114**／tui **106=106**（后两项改前基线实采）零新增。
- fmt：触碰件净。

## §6 登记面同步清单

[`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) 头行（本批 279 摘要＋orz 指针
`644f6b01→1ecad57c`；v4.251→**v4.252**）；[`TODO.md`](../../TODO.md) 计数行本批指针
＋P1 路由 0dd/0de 短指针＋P1-0dd/P1-0de 处置行＋S2 行勘误；
[`BACKLOG_AND_PRIORITIES.md`](../../docs/BACKLOG_AND_PRIORITIES.md) 本批指针行＋计数
行＋P1 表两格；[第二卷 §1.216](../BACKLOG_AND_PRIORITIES_2.md)；orz gitlink
`644f6b01→1ecad57c`＋`orz_source_manifest.sha256` 重生成（恰 4 行变更＝
acp_server／agent_loop／dispatch／bridge）。计数不变 61；不推送。

## §7 关键词

279 批、审查处置落账、0dd/0de、on-demand 第五道、S3 漂移窗口、1,088,890 字符勘误、
侧车估算 ≈544K、D3 边界有界性、D6 TUI 补齐、bridge 双字段兼容、user_cancel 加固、
0aq RS 家族、N2 footprint 断言、N3 len==3、PROTOC 注记、orz 1ecad57c、索引 v4.252、
计数 61 不变。
