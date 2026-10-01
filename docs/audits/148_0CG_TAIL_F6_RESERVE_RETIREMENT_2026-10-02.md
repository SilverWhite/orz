# 148 批：审查处置——F6 push 档与 0au 尾部派发保留量随 0cg 尾巴退役（2026-10-02）

> **缘起**＝对 0ce/0cf/0cg（146/147 批）的三维度全面审查（设计合理性／实现合理性／设计与实现
> 符合性），查出一处 **P2 符合性缺口**：0cg ②③ 的 argv/env 收口把两个账面声称「保留/不动」的
> 机械面断了供流；另 P3×3。**用户裁决**：「墙钟本身都已经退役了，F6/保留量不涉及其他部分的话，
> 就进行记录，一起退役即可」。
> **本批**＝P2 处置（F6/保留量/供流线退役）＋P3-1 勘误＋P3-2 注释对齐＋P3-3 登记。
> **计数 58 → 57**（0au 随裁决退役闭合，`pending` → `withdrawn`）；orz 批提交 `e1c373ec`
> （9 文件 +48/−737）；**零重建零 bump**（0.8.9 在役不动，源态前移随下一代重建进体）；
> **契约面零变化**；**父仓未提交未推送**（沿惯例待令）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| P2 缘起 | 0cg 退役 argv→env 桥并启动期 `remove_var` 后，env `ORZ_MAX_WALLCLOCK` 恒缺席 ⇒ **F6 push**（`f6_push_limit_secs ← main_wallclock_limit_secs_override() ← env`）与 **0au 尾部派发保留量**（`agent_loop.rs:4268 ← 同一读源`）恒 None/永不触发；而 146 批档与 orz `be4f90ff` 提交信息均声称「F6 push 档（默认 off）与 retrieval 派发保留量（机械面）不动／保留」——**结构保留、功能断供**，账实缺口未记录。字节级进体判据（`F6_BUDGET_CUE 2→2`）只证字符串在体，未证供流活性 |
| 前提核验 | 「不涉及其他部分」成立：F6 消费面＝agent_loop 每轮调用＋prompt 注入块注册臂＋3 测试；保留量消费面＝agent_loop 预扫描/保留臂/post-batch 重述＋batch_close 判定族＋5 测试；供流线＝`parse_main_wallclock_limit_secs`/`main_wallclock_limit_secs_override`/`run_wallclock_limit_secs` 字段/`with_run_wallclock_limit_secs` builder（零调用方）/`run_elapsed_wallclock_secs`（唯一消费者即 F6）。**契约面零触碰**：`budget_cue_injected` 事件枚举（orz-assurance）／`budget-cue-injected-event-payload-v0.2.schema.json`／`verify_budget_cue_injected` 校验族全部保留服务历史 journal；**D2 单批墙钟**（`BatchCloseKind::WallclockBound`）、**子代理档位墙钟**（180/300/450、`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS`）、**0z 硬门**（`run_invalidated{status: wallclock}`）零触碰 |
| P2 处置 | 三面同退役（orz `e1c373ec`）：① F6 面＝`F6_BUDGET_CUE_PREFIX`/`f6_budget_cue_block`/`f6_push_cue_for_remaining`＋注入块注册臂＋controller 开关/上限/跨阈记账/`maybe_push_f6_budget_cue`/run 复位臂/agent_loop 调用点＋3 测试；② 0au 面＝`CLOSE_ROUND_MARGIN_SECS`/`RUN_TAIL_RESERVE_SECS`/`WALLCLOCK_RESERVED_CAUSE`/`wallclock_reserved`＋agent_loop 预扫描/保留臂/`reserved_retrievals`/post-batch 重述＋5 测试；③ 供流线＝`parse_main_wallclock_limit_secs`/`main_wallclock_limit_secs_override`/`run_wallclock_limit_secs`/`with_run_wallclock_limit_secs`/`run_elapsed_wallclock_secs`。退役注记四处（prompt 0cg 注记扩写／controller 退役块／batch_close 退役注记／agent_loop 派发点注记） |
| P3-1 | 147 批档 §1 `be4f90ff` 统计「37 文件 +181/−14,109」与实际 commit（41 files +389/−14,114）不符——初稿为 fmt 触碰面随批与漂移对齐并入前快照；**本批勘误**（原位注记，不改读数本体） |
| P3-2 | 0ce 文档级漂移对齐：orz-tools `agents_md_tracker.rs`×2（AgentBuilder 归因）／`bridge.rs` slash_skills（PromptContext）／grok_build `task/types.rs`×2（AgentDefinition）＋orz-config-types `GoalRoleModel`（AgentDefinition）——幽灵类型引用改指现役机制（tool bridge／agent-type definition）并注 0ce 退役；纯注释，机制零变 |
| P3-3 | 登记（不改动）：`main.rs` run_tui 入口（裸 `orz` 兜底）在 scrub 之后重读 `std::env::args()`，Linux 上被擦 token 读回空串——该路径不解析 `--max-wallclock`、`parse_run_root` 按旗标匹配不受空串影响，今日无害；若未来有 scrub 后的位置敏感 argv 解析，须用本地 `args` 副本（main.rs:177 形态） |
| 读数 | `cargo check --workspace` 绿；orz-loop lib **847/0**（总 850＝858−恰 8 退役测试）＋integration **2/0**；orz-bin 全靶 **60/0**（16+12ig／2／15／24／2／1 逐靶同 146 账面）；clippy 触碰三 crate 差分 **100→99 恰 −1**（随删 0au 测试内 `assert_eq!(…, false)` 字面）；触碰文件 fmt 零新增 diff |
| 台账 | 本档；147 批档 §1 勘误；BACKLOG（指针行／计数行 57／P1 总览行／开放项行去 0au／0au 条目闭合／0cg 条目尾巴 bullet／0ce 条目注记 bullet）；TODO（计数行 57／P1 路由行去 0au／P1-0au 闭合）；第二卷 §1.100；索引 v4.116 → **v4.117**（头行＋0au 条目 `pending` → `withdrawn`＋§8 0cg 条目补尾巴）；[`LIF_DYNAMICS 设计档 §0`](../LIF_DYNAMICS_PROJECTION_AND_ROUND_BUDGET_DESIGN_2026-09-16.md) 同族注记；源清单重生成 **1,485** 条；门禁 `valid: true` |

## §1 P2 三维度审查结论（本批处置依据）

- **0ce／0cf**：设计、实现、符合性三维全过（保留面六边消费实证、死符号代码级零残留、
  preserve_order 落边修复正确；0cf 简注位置/措辞/双钉与进体字节判据逐项吻合）。
- **0cg**：设计合理（撤除依据充分）、实现正确（argv_scrub 直接指针写方案核验通过：
  `/proc/self/stat` 字段 48–51 映射与 `fields[N-3]` 索引算术正确、SAFETY 注记成立、fail-soft
  边界明确；实弹探针双绿独立复验；身份门 `4e35f410…` 与两平台字节判据逐项复现）——唯
  「保留面」声明的供流断供未记录（本批 P2）。
- **0cg 的「保留面」字节级验证教训**：`F6_BUDGET_CUE 2→2` 证字符串在体，不证可达——进体判据
  对「保留面」宜加供流活性核对（本批已随退役消除该面，教训留档）。

## §2 验证读数与如实记

| 面 | 读数 |
|---|---|
| 编译 | Windows `cargo check --workspace` 绿（PROTOC 显式直连）；rustc `0xc0000409` 闪退一次、重跑通过（146 批同款环境面） |
| 测试 | orz-loop lib **847/0**（3 ignored；总 850＝基线 858−恰 8 退役测试：prompt 1＋tool_run 2＋batch_close 3＋agent_loop 2）；integration **2/0**；orz-bin **60/0** 逐靶同 146 账面（含 stdio_e2e 事件计数不变＝最小流本就零 F6/零保留事件） |
| 预存失败（非本批） | `retrieval::dispatch::tests::user_cancel_closes_pending_activations_before_run_cancelled` 在全量 lib 首跑失败（30ms 取消竞态：cancel 先于激活收口，close_reasons 0≠1）；**stash 差分实证＝基线态（`be4f90ff`）同形失败，先于本批**；同批全量复跑自愈通过（负载敏感族，146 批 orz-host 先例形态）。不占计数、另记观察 |
| clippy | 触碰三 crate（orz-loop/orz-tools/orz-config-types）`--all-targets` 差分：基线 100 → 现态 99，唯一差值＝随删 0au 测试的 `used assert_eq! with a literal bool`；**零新增** |
| fmt | 触碰文件（9 个）零新增 diff（agent_loop.rs 尾空行随本批恰修）；**预存漂移如实记**＝提交态即有 fmt 漂移散布于未触碰面（acaf.rs×2／compact.rs×5／console.rs×9／console_exec.rs×11／delivery/gateway/activation/disposition 等约 47 处，形态＝链式调用拆行与块尾空行）——非本批引入、不占计数，留待触碰批顺带收口 |
| 环境 | D 盘耗尽一次（277G 满 100%，git index.lock 写入失败伴随）——清 `target/debug/incremental` 12G 续行（146 批先例）；protoc dotslash 包装失效以 `PROTOC=D:\CLI\orz\bin\protoc.exe` 直连（146 批先例） |
| 源清单 | `generate_orz_source_manifest.py` → **1,485** 条（纯删改不改文件集）；门禁 `check_repository.py` **`valid: true`** |

## §3 边界与余额

1. **未提交未推送（父仓）**：沿惯例待令；orz 侧 `e1c373ec` 单提交（`be4f90ff`＋`6295b3dc`＋
   `e87b0630` 之上，分支 ahead 四提交）；0.8.8/0.8.9 发布面仍未推送未发行（停 v0.8.7）。
2. **0.8.9 在役载体不受影响**：F6/保留量在 0.8.9 已是断供死配置，退役仅清源码——进体判据
   （字节级）在下一重建窗口自然达成（`F6_BUDGET_CUE` 2→0 等），零急迫性。
3. **0au 闭合口径**：`pending` → `withdrawn`（用户裁决退役不实施），计数 58 → 57；其 S1 曾入树
   （0.8.8/0.8.9 载体在役含该机制）、S4 从未实机验证——历史如实记于 BACKLOG 0au 条目。
4. **0ch S4 仍未跑**：`caffe-cifar-10`／`git-multibranch` 两题（0.8.9 在役＝重跑线解禁）。

## §4 关联与关键词

[`146 批档`](146_0CE_0CF_0CG_DEADCODE_GUIDE_WALLCLOCK_2026-10-01.md)／
[`147 批档`](147_CARRIER_REBUILD_V089_0CF_0CG_2026-10-01.md)（§1 本批勘误）／
[`0au 深挖档 §3`](0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md)／
[`LIF_DYNAMICS 设计档 §0`](../LIF_DYNAMICS_PROJECTION_AND_ROUND_BUDGET_DESIGN_2026-09-16.md)／
BACKLOG `0cg`/`0au`／TODO `P1-0au`。

关键词：148 批、审查处置、P2 断供、F6 push 退役、0au 尾部派发保留量退役、供流线清退、
ORZ_MAX_WALLCLOCK、budget_cue_injected 契约面保留、withdrawn、计数 58 → 57、
stash 差分预存失败、fmt 预存漂移、be4f90ff 统计勘误、orz `e1c373ec`、零重建零 bump、
未推送未发行。
