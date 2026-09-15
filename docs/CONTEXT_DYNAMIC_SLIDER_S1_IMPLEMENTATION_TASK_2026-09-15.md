# 动态上下文滑块 S1 实施批任务书（2026-09-15）

> **类型**：实施批任务书（执行权威；不含任何实施结果与判据读数）。
> **状态**：**已放行 · 已落码 · S1 修订批（v7 回改）亦已落码**（2026-09-15 用户「请直接进行 S1」放行并于当日落码，**回执见 §10**；同日 v7 裁定后用户令「开始进行批次 A S1 部分」，回改同日落码，**回执见 §10.7**）。**2026-09-15 v7 用户裁定**（压缩分工：机械＝结构化轨／模型＝语义轨；500K 纯提醒／900K 强制；提醒水位会话级；`mode=model_summary`＋原文定位指针）对本批实现构成的回改，清单与验收线见 **§10.6**（执行记录见 §10.7）；设计权威已同步为 [`滑块设计稿 v7.1`](CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md)（§3.4.1／§3.5.1／§8）。A/B、S2、尾批仍各自另行放行）。
> **上级**：BACKLOG **0ah** / TODO **P1-0ah**；索引 `AUTH-CONTEXT-DYNAMIC-SLIDER` /
> `GAP-DYNAMIC-CONTEXT-SLIDER`。
> **设计权威**：[`CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15`](CONTEXT_DYNAMIC_SLIDER_DESIGN_2026-09-15.md)（draft v6）。
> **定位**：本文件管**范围／落点／判据／禁做项**；设计语义（不变量、参数律、翻转口径）以设计稿为准——
> 二者冲突时先停下并回写本文件，不得就地改设计。
> **源冻结基线**：以放行时点 `orz` HEAD 为准（本文件落档时不早于 `1f303cf4`；父仓登记批见 §8）。
> **前置（均已达成）**：0ac ①-b 收尾批（orz `1deeba75`）、0ae D0–D4 落码（`f0040557`）＋审查修复批（`183fbb08` / `1f303cf4`）。

## 0. 用户裁决口径（本批依据，逐条可回查）

| # | 裁决（日期） | 出处 |
|---|---|---|
| R1 | **重付红线可改写**：2026-08-19 的「每窗折叠重付 ≤ ~15K 真实 token」不再作硬线，由四条新口径取代（结构性上限＝保留带＋新内容／相对成本 R≤×1.6／命中率 ≥90%／驱逐次数） | 设计稿 §3.3.1（v3） |
| R2 | **L 可加大、H 可放大**；设计定位＝**巨大/高压长任务**，优先保驻留带绝对深度 | 设计稿 §3.3（v3） |
| R3 | **D2 注意力阶梯整体下线**，只保留**实际上下文 500K / 900K 两级提醒**（须告知模型实际读数并提醒压缩）；**D3 保留**、开窗量尺改挂实际上下文＋模型自选；D0/D1/D4 保留 | 设计稿 §3.4（v4） |
| R4 | 参数**默认保守档 H=160K / L=64K**；对「模型会不会主动用」**不悲观**（低使用率须先核「档案里有没有内容」） | 设计稿 §2/§3.3/§4.3（v4） |
| R5 | **单对话、上传面无实际上限**；留存面收敛为 **A 机械压缩部分 ⊕ B 逐字本体**；两类**同步存档**（复用既有归档包原语 + 三键互标） | 设计稿 §3.5（v5） |
| R6 | **块轴**：全局键＝会话相对 LIF 轮、台账行键＝`[seq]`、时间双标、两法都做、**排期置尾** | 设计稿 §7 / DP-13（v6） |

## 1. 目标与非目标

**目标（S1 一个批内闭合）**：把「锯齿折叠」换成「常驻滑窗」，机械消灭实测死亡螺旋（末 34 分钟 8 次折叠、
4–5 分钟记忆周期、任务线断裂 1 次），并按 R3 完成 0ae 的线路归位。

**非目标（本批不做，违者即越界）**：

1. **S2 逐字分页档案**（manifest ＋ 重放）——gate 后置，凭 S1＋0ae 的 A/B 数据另裁（设计稿 §4）。
2. **块轴（LIF 时间/事件轴）**——R6 置尾，独立批（设计稿 §7）。
3. **不动 8 工具面**（本批零新增工具；S2 若采形态②才涉及用户主导显式例外）。
4. **不改 ADR-0010**、不动索引/BACKLOG/TODO 之外的账本面（登记见 §8）。
5. **不碰 `D:\AGI`**（用户 2026-09-13 红线）。

## 2. 落码清单（A1–A8；落点以设计稿 §3 为准）

| # | 项 | 要点 | 设计出处 |
|---|---|---|---|
| **A1** | 常驻滑窗内核 | 两参数 H/L；触发＝loop-top 视图估算 ≥H；动作＝自最旧驻留轮起收集**连续完整轮**至累计估算 ≥**H−L**，一次性移出视图；`L_start` 前移；指针消息 set-once 不退让 | §3.1/§3.2 |
| **A2** | 段边界整轮对齐 | **按 token 硬切否决**；驻留带恒含 ≥1 个完整轮结构（`fold_tail_rounds` 已退役，不得复活）；驱逐只在 loop-top 完整轮间隙执行 | §3.1 |
| **A3** | 台账续写语义照旧 | 移出的轮按既有摘要行格式追加 `.gsa/ledger/current.md`（原子追加、`[seq]` 续号、写失败禁用折叠）；**本批不加轴列** | §1/§3.2 |
| **A4** | 首次驱逐一次性固化提醒 | 复用 0ae D2 **128K 梯的文案语义**（「请将接线结论/关键读数固化至黑板 plan/notes」），每 run 恰一次。**前置：量尺对齐**——既有 128K 梯吃上一请求实测 `prompt_tokens`，本触发吃视图估算 chars/2，两尺不同，实施批须先对齐（建议「首次真实驱逐后置一次性提醒」）并在交接件写明取法 | §3.2 |
| **A5** | D2 下线 | `attention_ladder.rs` 的 128K/160K/300·500·600·700K/800K/920K 各级整体退役（含 `ORZ_LADDER_*` 的处置：保留 env 但不再生效须显式落账，或同批移除）；**A4 复用其文案，不复用其量尺** | §3.4 |
| **A6** | 500K / 900K 实际上下文提醒 | 刻度＝**模型侧上下文累积量估算**（非上传视图、非 provider 读数）；两级各一次/run；独立块、loop-top 安全间隙；**文案须含实际读数**；落 `mechanical_audit_update`（kind 改名，枚举与 Python 镜像同批对齐——0ag 先例）；900K 级升级语气并提示「未固化内容届时只能靠档案回读」 | §3.4 |
| **A7** | D3 改挂实际上下文量尺 | 开窗条件从「视图 920K」改为「实际上下文 500K/900K ＋ 模型自选」；窗口 ≤3 轮无工具轮、模型未完成则机械兜底、`model_participated` 如实落账（既有 `PendingCheckpoint::ModelCompression` 与 `finalize_model_compression_close` 统一出口沿用不动） | §3.4 |
| **A8** | 机械压缩兜底上调 ＋ 同步存档三键 | ① 机械梯随上限上调为兜底：rhythm 改 H＋缓冲（缓冲值实施批定）、硬兜底改挂**实际上下文**（建议 ≥950K 强制一次），并须与 R1 第 4 条（打断频率上限）合并核对；② 归档包按 §3.5 扩展（纳入 A 类台账摘要行与压缩摘要存档清单）＋ **三键互标**（会话相对 LIF 轮区间 / 台账 `[seq]` / journal run+sequence）＋增量归档时点 | §3.4/§3.5 |

## 3. 参数与配置

- **默认工作点**：H＝160K、L＝64K（保守档；`L/H=0.400 ⇒ R≈×1.54`，平均工作点 112K 仍在 128K 平台期内 ⇒ 本档**不需要质量读数**）。
- **env**：H 用 `ORZ_SLIDER_WINDOW_TOKENS`；L 的 env 名由实施批定名（建议 `ORZ_SLIDER_RESIDENT_TOKENS`）。
- **纪律**：**参数不得进指针文案**（改 env 即造成一次性前缀失效；配置固定性优先于精确表述）——同 §3.1 既有纪律。
- **档位出口**：成本超线 ⇒ 压 L/H；驻留带不足 ⇒ **同比例抬 H 与 L**（成本不变、工作点抬高）。192K/80K、256K/96K 档留候选，仅巨大/高压任务启用（启用时 DP-9 质量读数随之生效）。

## 4. 判据与读数口（放行后随 A/B 交）

机械读数**四件套为主**：缓存命中率（provider/journal）、每窗折叠重付（真实 token）、驱逐次数、重读 offset 分布；
外加下表新增口：

| 口 | 目标 |
|---|---|
| 尾部塌缩/死亡螺旋 | 0 次深谷（视图永不 <L）；基线＝末 34 分钟 8 次折叠 |
| 成本相对线 R | ≤×1.6（建议线），超线 ⇒ 压 L/H |
| 每窗重付 | 结构性上限＝保留带 L ＋ 驱逐后首请求新内容（15K 绝对线已由 R1 改写） |
| 命中率 | provider ≥90%（预期 92–95%，实测值须给出） |
| 驱逐次数 | 保守档 ≈+23%（次数 ∝ 1/(H−L)）⇒ 须确认该增幅可接受 |
| 提醒面 | 500K/900K 各发一次、文案含实际读数；`model_participated` 占比须统计 |
| 存档一致性 | 三键齐备率 100%、悬挂 0 |
| 本地增长 | journal / conversation sidecar / 台账文件 / 归档包 四类字节读数与增速 |
| 出处/基线正确率、交付量 | 不回归（0 失准；362 行＋7 编辑为基线） |

采样口径：**同题、每臂 ≥2 run**，长实现类 run ≥2h；**单 run 不足以落架构结论**（C6 撤回纪律）。
**使用率类判据（展开率、主动压缩率）**须先核设计稿 §4.3 的两个前提，否则记「不可判读」。

## 5. 不变量与回归钉子（本批必带）

1. 前缀字节稳定：两次驱逐之间视图＝上一视图＋纯追加；指针 set-once 不重渲染。
2. 整轮配对：轮缺工具结果 ⇒ 永不折叠；段边界整轮对齐。
3. **触发未执行零副作用**；preamble / `safe_fold_cut` 放弃折叠防线继承。
4. 400 修复纪律：retain 后移、守卫先于变更。
5. 恢复复位：滑动态为运行期内存态（`LedgerFoldState` 现 5 字段），恢复后复位、自冻结桥末端重新累积。
6. 机械压缩兜底在 rhythm 路径 NoOp 时不得停摆（`1f303cf4` 既有纪律：NoOp 不 rearm）。

## 6. 交接与恢复纪律（本批硬要求）

1. **先跑已写单测**：本 run 写完全部单测后**必须实跑**（狗粮 run `RUN-CLI-6aa7e0aa` 实测「362 行的 6 条单测全程未跑」，深审 §6-D 明确要求钉进下批任务书）。
2. **交接件**：范围、被打断现场（文件/行号级）、剩余件落点、动手前必核清单、**未核项如实标注**、恢复点。
3. **并发事实声明**：同工作区可能有并行会话（深审 §3-4 实证交叉污染），本批开工前先 `git status` 全量（禁 `| head` 截断）并在交接件写明并发事实。
4. **越界即停**：需要新工具面、需要改 ADR、需要动 S2/块轴时，停下报用户，不得自行扩张。

## 7. 批序与门

**① S1 实施批（本任务书） → ② A/B（同题每臂 ≥2 run，四件套读数） → ③ S2 裁决（形态①优先） → ④ 尾批＝块轴（设计稿 §7）**；
每一步独立放行、独立登记，**不得跳步合批**。

## 8. 登记归属（本批落码时同交）

1. **本批登记**（随落码批，非本文件）：BACKLOG/TODO **0ah** 勾选与状态、索引 `GAP-DYNAMIC-CONTEXT-SLIDER` 状态推进。
2. **翻转落账（三处，须显式、不得顺滑通过）**：① 2026-08-19 折叠桥截断裁决的 **15K 红线**（按 R1 改写 `FUS-LEDGER-FOLD-STATE` 验收线）；② **0ae D2 下线**（R3，对 0ae 全量裁决的落地改动）；③ 若采 S2，即重开外挂件 §3「不做 memory_get/memory_search」的相关面。
3. **尾批（块轴）的契约触碰另计**：台账行格式扩展（对 08-18 行契约的部分翻转）＋ `DomainSpike` 增 `round`——随尾批登记，不并入本批。

## 9. 风险与已知未知项

1. **成本模型是解析推导、非实测**（设计稿 §3.3 的 R 律）：A/B 必须以四件套实测覆盖，不得以推导值当结论。
2. **V4 塌陷带（384K–512K）**对 V4.1 适用性＝已知未知项（继承 0ae §9）。
3. **量尺二义性是本批最大实现陷阱**：视图估算（chars/2）／provider 实测 prompt／实际上下文估算三者在滑窗下彻底分离（A4/A6/A8 各挂一尺），实施批须逐处写明取尺并在交接件给对照。
4. **无上限单对话的本地增长**（journal/sidecar/只增不删台账）：先给读数与增速，再裁轮转（DP-12）；轮转不得破坏 journal 链与三键。

---

## 10. 落码回执（S1 实施批；2026-09-15 放行并当日落码）

> 本节为 §6.2 要求的**交接件**：范围／落点／量尺／读数／未核项五段齐全；**未核项如实标注，不得当已核**。

### 10.1 范围与落点（orz 子模块改动，**已提交 `61982a56`**）

| 项 | 落点（文件 → 机制） |
|---|---|
| **A1/A2** 常驻滑窗内核 | `orz-loop/src/compact.rs`：`slider_window_tokens`(H,160K)／`slider_resident_tokens`(L,64K)／`slider_rhythm_buffer_tokens`(32K)／`hard_context_tokens`(950K)＋`ORZ_SLIDER_WINDOW_TOKENS`・`ORZ_SLIDER_RESIDENT_TOKENS`・`ORZ_SLIDER_RHYTHM_BUFFER_TOKENS`・`ORZ_CONTEXT_SCALE_HARD_TOKENS` 解析（`parse_slider_tokens` 纯函数）；`action_ledger.rs`：`bridge_estimate_budget(L)`（真实 token → chars/2 估计口径），`bridge_cut`/`advance_fold`/`safe_fold_cut` 语义照旧＝自最旧驻留轮起收**连续完整轮**至剩余 ≤L、**段边界整轮对齐**、最新轮单独超预算仍保留（谷值物理下限）；`agent_loop.rs`：loop-top 触发改 `view_estimate ≥ H`、驱逐预算改 L、D4 机械段随推进冻结不变 |
| **A3** 台账续写语义 | **零改动**（`append_ledger_rows`／`external_row_line`／`tail_seq` 续号／写失败禁用折叠全部沿用；本批**不加轴列**） |
| **A4** 首次驱逐一次性提醒 | `agent_loop.rs` 推进**成功**分支（写失败回滚不计）＋`context_scale.rs::first_fold_reminder_block`（复用 0ae D2 128K 梯文案语义，`kind=context_scale`、`key=context_scale:first_fold`） |
| **A5** D2 下线 | 删 `orz-loop/src/attention_ladder.rs`（整档退役；`ORZ_LADDER_*` 五 env 随之消失）＋新增 `orz-loop/src/context_scale.rs`（`lib.rs` 模块表同步）；`mechanical_audit::KIND_ATTENTION_LADDER` 保留（`allow(dead_code)`，仅供历史 journal 回放，生产零写入） |
| **A6** 500K/900K 提醒 | `context_scale.rs`（`ContextScaleState`：每级恰一次/run、**不 rearm**；`reminder_block` 文案**必含实际读数**；900K 升级语气＋「未固化内容届时只能靠档案回读」）；`agent_loop.rs` loop-top 注入＋`prompt.rs` 注册 `[CONTEXT_SCALE` 为注入文本（**不写回持久化会话**，与「固定文本不是模型输出」纪律一致） |
| **A7** D3 改挂实际上下文 | 开窗条件＝**实际上下文估算 ≥500K/900K**（与提醒同轮）；`PendingCheckpoint::ModelCompression`（≤3 轮）与 `finalize_model_compression_close` 统一出口**沿用不动**；随提醒同块注入窗口任务（`reminder_with_window_block`）；reason 更名 `attention_920k_window` → **`context_scale_window`**（契约同步，见 10.1 末行） |
| **A8①** 机械兜底上调 | rhythm ＝ **H＋缓冲**（视图刻度）⇒ 滑窗下退化为「驱逐停滞兜底」（满足 R1 第 4 条打断频率上限）；**硬兜底 ＝ 实际上下文 ≥950K 强制一次**（新 `reason=context_scale`；`measured` 与 `force` 同尺——修掉「视图读数做基数、全量估算做被比较对象 ⇒ guard 恒拒压」的量尺错配） |
| **A8②** 同步存档三键 | `orz-host/src/acp_server.rs`：归档包升**信封**（`{"schema","conversation","archive_keys"}`，`conversation` 成员＝sidecar **原始字节**零变换）；`build_archive_keys` 给三键（LIF 会话轮跨度 `entry_round→round` ＋ `window_rounds` 临时键、台账 `[seq]` 跨度/行数、journal `RUN-<session8>-*` 的 run+sequence）＋A 类压缩清单；**增量归档**＝run 尾按实际上下文跨 500K 里程碑追加一次（水位文件 `.gsa/archives/<session8>.milestones.json` 单调幂等，会话关闭仍照旧打包）；`decode_archive_package` tolerant 兼容旧裸包 |
| **契约面（同批对齐）** | `runtime/mechanical-audit-update-event-payload-v0.2.schema.json`（kind 枚举 +`context_scale`）、`runtime/context-compressed-event-payload-v0.2.schema.json`（reason 枚举 +`context_scale`/`context_scale_window`＋描述）、`runtime/session-archive-event-payload-v0.2.schema.json`（+可选 `incremental` 与包结构描述）、`assurance/run_event_journal_validation.py`（两处镜像同步）、`orz-assurance/src/journal/families_s2c.rs`（Rust 法官 reason 枚举同步） |

### 10.2 量尺对照（设计 §9-3「量尺二义性＝本批最大实现陷阱」：逐处写明取尺）

| 触发 / 动作 | 取尺 | 代码位 |
|---|---|---|
| 滑窗驱逐（推进） | **视图估算**（折叠后上传视图 chars/2） | `agent_loop.rs` `view_estimate >= slider_window_tokens` |
| rhythm（机械摘要） | provider 实测上一请求 `prompt_tokens` | `rhythm_tokens()` ＝ H＋缓冲 |
| fallback（视图窗口兜底） | provider 实测（`safety_tokens` 256K） | 同上（默认档常态不可达，保留 S2 展开路径） |
| 500K/900K 提醒 ＋ D3 开窗 | **实际上下文估算**（全量会话 chars/2） | `context_scale.rs` ＋ `agent_loop.rs` loop-top |
| 硬兜底 ≥950K | **实际上下文估算**（同一把尺同时给 guard 基数与事件 `trigger_tokens`） | `hard_context_now` / `force_context_scale` |
| 增量归档里程碑 | **实际上下文估算**（同口径镜像；orz-loop 的 `estimate_messages_tokens` 为 `pub(crate)`，跨 crate 不可复用） | `acp_server.rs::estimate_conversation_tokens` |

### 10.3 判据读数（本批实跑，非推导）

- **orz-loop**：`cargo test -p orz-loop --lib` → **802 通过 / 0 失败 / 3 忽略**（新增：`context_scale` 模块 5、滑窗整轮对齐/驻留带/二次推进 1、A4 首次驱逐提醒 1、A6 900K 升级＋实际读数 1、A8① 硬兜底 1；0ae 原 D3 三条窗口钉子按新量尺重写后仍绿）。
- **orz-host**：`cargo test -p orz-host --lib acp_server` → **46 通过 / 0 失败**（新增 `archive_keys_and_incremental_milestones_follow_the_design`；`session_archive_single_gzip_package_and_event` 改走 tolerant 解码并加三键断言）。
- **orz-host 全量 lib 单跑**：322 通过 / **3 失败** / 5 忽略——三条（`codex_app::eof_mid_turn_keeps_turn_running_to_valid_journal`、`tests::call_tool_timeout_kills_process_tree`、`tests::run_terminal_cmd_truncation_carries_output_object`）**逐条单跑均通过**，属并行负载下的既有时序抖动，与本批改动面无交集（未触碰 process/terminal/codex_app 路径）。
- **clippy/fmt**：新增模块 `context_scale.rs` 零告警；`acp_server.rs` 与 `compact.rs` 本批新增行零告警（剩余告警逐条落在未触及行）；`cargo fmt -p orz-loop -p orz-host` 干净。
- **仓库门禁**：`python scripts/generate_orz_source_manifest.py` → **1450 条**；`python scripts/check_repository.py` → `valid: true` 的其余检查全过，唯一 error ＝「orz submodule working tree is dirty」（＝本批改动未提交所致，非内容错误）。

### 10.4 未核项（如实标注）

1. **成本律 R／命中率／驱逐次数／每窗重付** 四项机械读数**未测**——设计 §3.3 的 R 是解析推导，须由 ② A/B（同题每臂 ≥2 run、长实现类 ≥2h）实测，不得以推导值当结论。
2. **900K 提醒与 ≥950K 硬兜底的真实长会话路径未跑**：单测分别用真实阈值（1.9M 字符 prompt ⇒ 实际上下文 ≈950K）与测试缝隙（`with_hard_context_tokens(1_000)`）覆盖机制，**未在真机长 run 上验证**。
3. **增量归档未跑真机跨里程碑 run**（逻辑与幂等由单测覆盖）；`keys_digest` 未实现（判定为冗余：三键在包内、包 digest 在事件内）。
4. **`ORZ_LADDER_*` 五 env 随模块删除**：不再生效且**无兼容告警**（已在 10.1 与索引 `AUTH-CONTEXT-SOFT-GATE` 显式落账，不得当静默失效通过）。
5. **`attention_ladder` kind 保留在闭枚举**：仅为历史 journal 可校验（删值会让旧刊判 invalid）；新刊零写入由单测钉住（本 run 不得出现该 kind）。
6. **`decode_archive_package` 当前仅测试消费**（显式 `allow(dead_code)`）：生产回读点随 S2（若采形态①）接入。
7. **并发事实**：本批开工前 `git status` 全量显示父仓/子模块均干净；作业期间**邻线子代理在父仓提交过一次 `d2cc39b9`**（v3.35 登记批 5 文件，未含 orz 指针与本批代码——**主会话未授权该提交**，已如实登记）。本批 orz 改动全程**未提交**，恢复点＝orz HEAD `1f303cf4`。

### 10.5 剩余件与恢复点

- 恢复点（提交前）：orz 工作区 HEAD 仍 `1f303cf4`（本批改动在工作区）；父仓 HEAD 含 `d2cc39b9`。
- **提交后恢复点（2026-09-15 提交收尾）**：orz HEAD **`61982a56`**（13 文件 +4588/−850）、父仓 HEAD **`524518fe`**（12 文件）；两侧工作区干净，`orz_source_manifest.sha256` 重算 1450 条，**门禁 `valid: true`（`error_count: 0`）**——提交前复核读数：orz-loop 818/0/3、orz-host 325/0/5（`--test-threads=1`）、orz-assurance 229＋fixtures 全绿、`cargo fmt --all -- --check` 干净、`runtime/tests` 361/1（唯一失败＝既有无关红灯 `test_v02_all_51_event_types_covered`）。**§10.7–§10.10 各节首句的「代码仍未提交／orz HEAD 仍 `1f303cf4`」为提交前读数，已由本项闭合（内文不改写）。**
- 剩余件（批序 DP-6 不变）：**② A/B**（四件套读数＋存档一致性＋本地增长读数）→ **③ S2 裁决**（形态①优先）→ **④ 尾批＝块轴**（`LedgerFoldState` 轴表 5⇒6 ＋ `DomainSpike.round` ＋ 台账行扩列；契约触碰另计）。
- 翻转落账：①（08-19「≤~15K」红线改写：旧两条 `ORZ_FOLD_*` env 退役、四条新口径上位）与 ②（0ae D2 下线）**已随本批落账**（索引 `FUS-LEDGER-FOLD-STATE`／`AUTH-CONTEXT-SOFT-GATE`）；③ 随 S2 裁决。

### 10.6 v7 裁决回改清单（2026-09-15；设计稿 v7 §3.4.1／§3.5.1／§8）

> 本节登记 **v7 用户裁定**对本批已落码实现构成的回改——**§10.6 为范围与验收线清单**（**执行回执见 §10.7**：2026-09-15 用户令「开始进行批次 A S1 部分」放行并当日落码）；与 §10.1／§10.3 描述冲突处**以本节与 §10.7 为准**（§10 正文保留为实施记录）。

| # | v7 裁决 | S1 现状（已落码） | 回改要点 |
|---|---|---|---|
| V1 | **压缩分工（并存，非影子压缩）**：机械＝**结构化轨**（工具／命令／结果照常压缩**并照常作用于上下文**）；模型＝**语义轨**（机械压不了的语义由模型摘要替换） | 现状：机械压缩 `run_template_compact` 把结构化与语义一并吞掉，且语义侧无模型参与；500K 即强制开窗 | 机械轨**保留**对上下文的压缩作用（工具／命令／结果 → 台账摘要行＋指针＋`drain`＋compaction 存档）；**新增语义轨**：语义摘要替换被压区（`mode=model_summary`）；以 `kept_start=fold_cut` 为界，未折叠降级不得沿用「保留最近 2 轮」 |
| V2 | **500K ＝ 纯提醒**（不打断、不开窗、不强制，可延后） | 500K 越线即 `pending_checkpoint=ModelCompression`（打断＋开窗） | 500K 仅注入提醒；不开窗、不设 pending |
| V3 | **900K ＝ 必须压缩一次**（窗口内装载「滑块之外的携带内容」＋滑块） | 900K 开关窗但不装载待压区（模型看不到待压内容） | 窗口请求装载待压区原文（过大时分段）；产出模型摘要；≤3 轮兜底 |
| V4 | **950K 兜底 once＋冷却** | `hard_context_now` 无 once／无冷却，可逐轮重压 | 加一次性水位＋`min_rounds` 冷却；压不动则落 anomaly 并停手 |
| V5 | **提醒水位会话级**（与黑板同族、新会话独立） | `ContextScaleState` per-run（每个 prompt 重发） | 随会话状态持久化（sidecar 同类字段），prompt 起始注入 loop、fire 时回写 |
| V6 | **`mode=model_summary`** 契约扩展 | `mode∈{template_summary,mechanical}` | schema／Python 镜像／Rust 法官三处同步 |
| V7 | **四项原文定位指针**（compaction 路径＋digest／台账 `[seq]` 区间／journal run+sequence／sidecar 路径） | 只有 compaction 路径＋digest＋台账指针 | marker／摘要块补齐四项；逐字分页回读仍属 S2 |
| V8 | **成本判定线**：×1.6 降为参考线，改判「不高于 08-19 前形态」 | 判据表写 R ≤ ×1.6 | 判据与设计稿同步（§3.3／§3.3.2／§5），A/B 用 `view_estimate_after` 实测 |

**验收线（S1 修订批，须先写单测再实跑——§6.1 纪律）**：① **压缩分工**——机械轨照常 `drain` 工具／命令类内容并留台账摘要行＋指针（单测：机械压缩后 `messages` 估算下降）；**语义轨**须产出 `mode=model_summary` 事件并替换被压区（单测：语义摘要路径）；② 500K 提醒不设 pending（单测）；③ 900K 窗口请求含待压区（单测：窗口轮请求估算 ≥ 待压区估算）；④ 950K 兜底 once＋冷却（单测：压不动场景下触发数＝1＋anomaly）；⑤ 会话级水位跨 prompt 幂等（host 侧单测）；⑥ `mode=model_summary` 三处契约一致；⑦ 定位指针四项齐备（单测）。**§4 判据与 A/B 口径不变**；**零新增工具面**（模型自选压缩＝机械可识别摘要块）。

### 10.7 落码回执（S1 修订批 v7；2026-09-15 用户令「开始进行批次 A S1 部分」放行并当日落码）

> 本节为 §10.6 的执行回执（**代码仍未提交**（提交前读数；已由 §10.5 提交收尾闭合）；orz HEAD 仍 `1f303cf4`，改动在工作区）。与 §10.1／§10.6 描述冲突处以本节为准。

| # | v7 裁决 | 落点（实现事实） |
|---|---|---|
| V1 | **压缩分工（并存）** | `run_template_compact` 新增 `semantic: Option<SemanticCompaction>`——两轨共用 drain／marker／存档／事件路径：机械轨（`None`）照常作用于上下文（工具／命令／结果 → 台账摘要行＋指针＋`drain`＋compaction 存档＋事件）；**语义轨**由模型回复文本里的 `[SEMANTIC_SUMMARY]…[/SEMANTIC_SUMMARY]`（六段结构，≥2 段命中才识别）触发，机械层在下个安全间隙用它**替换被压区**（`mode=model_summary`）。识别点＝**响应处理**（工具轮文本不进 `messages`，只留 `model_output` 事件）。对象边界：`kept_start`＝`fold_state.fold_cut`；**未折叠降级路径改以驻留带 L 为界**（`compact_fallback_cut`：主车道非 session_end 用 `bridge_cut(bridge_estimate_budget(L))`，无可移出内容＝NoOp——不再沿用「保留最近 2 轮」；非主车道与 session_end 保持既有语义） |
| V2 | **500K ＝ 纯提醒** | `ContextScaleFire.opens_window`：刻度表**除最后一档外都是纯提醒**——500K 只注入提醒块（含实际读数＋自选压缩方法＝摘要块格式＋可延后语义），**不设 pending、不开窗、不缩工具面**；`mechanical_audit` 记 `opens_window=false` |
| V3 | **900K ＝ 必须压缩一次（窗口装载待压区）** | 最高档注入升级提醒 ＋ 开窗；**窗口轮请求绕过折叠**（`window_loads_pending_region` ⇒ 上传 `messages` 全量＝「滑块之外的携带内容＋滑块」）；窗口内「参与」判据加入**产出摘要块**（不再只认 `blackboard_write`）；窗口任务文案改指摘要块（原「标注可弃范围」在 S1 从未被消费，删除） |
| V4 | **950K once＋冷却＋anomaly** | `hard_force_at_round` ＋ `HARD_CONTEXT_COOLDOWN_ROUNDS=4`（loop 迭代轴）＋ `hard_context_stopped` 停手旗标：**压得下来 ⇒ 不停手不落 anomaly**；压不动分两类——机械压过仍在线上（残留＝语义层）⇒ anomaly＋强制开窗；机械无可压内容（NoOp）⇒ 只落 anomaly 停手（不开窗空转）。anomaly 值 `semantic_compaction_stalled`，`key=context_scale:hard_950k_stalled` |
| V5 | **提醒水位会话级（DP-16）** | `ContextScaleState::from_notified_keys` / `notified_keys`；controller `context_scale_notified` Mutex ＋ `with_context_scale_notified` / `mark_context_scale_notified`；host `StoredConversation.context_scale_notified`（`serde(default)` 向后兼容）prompt 起始注入、成功 run 回写（SUCCESS-ONLY，同 `fatigue_tiers_notified`） |
| V6 | **`mode=model_summary` 契约扩展** | 三处同步：`runtime/context-compressed-event-payload-v0.2.schema.json`（`mode` 增 `model_summary`、`reason` 增 **`model_selected`**、description 同步）；`assurance/run_event_journal_validation.py`（Python 镜像）；`orz/crates/orz-assurance/src/journal/families_s2c.rs`（Rust 法官）——`model_summary` 与 `mechanical` 同为无模型槽位失败的收口路径（不得 `summary_incomplete`） |
| V7 | **四项原文定位指针** | `summary::LocatorPointers` ＋ `render_marker_lines`：compaction 存档＋digest／台账 `[seq]` 区间／journal run＋sequence／conversation sidecar 路径，缺项如实「（无）」。**机械 v0.2／v0.3 marker 与语义 marker 共用**；台账区间来源＝`append_ledger_rows_range`（本窗口 epoch 已折叠行序号），journal 区间＝窗口 epoch 序列跨度，sidecar 路径＝`<session_cwd>/.gsa/conversations/<session8>.json`（口径同 host） |
| V8 | **成本判定线改口径** | 设计稿 §3.3.2／§5 已改（×1.6 降参考线，改判「不高于 08-19 前形态」）；无代码面，A/B 用 `ledger_fold_advance.view_estimate_after` 实测 |

**附带修复（同批，审查 P2 记录项）**：压缩窗口机械提示（`[模型参与压缩…]`，即 `window_dropped_calls_notice` / `window_remaining_notice` / `compression_window_block`）**注册进 `prompt::is_injected_block_text`**——修复前该前缀未登记，窗口轮提示会被写回持久化会话（0AE 遗留缺口）。

**验收线状态（§10.6 七条；全部先写单测再实跑）**：

| # | 验收线 | 钉子（单测） | 状态 |
|---|---|---|---|
| ① | 压缩分工（机械轨照常 drain＋语义轨 `model_summary` 替换被压区） | `agent_loop::tests::model_summary_block_replaces_the_region`（`mode=model_summary`／`reason=model_selected`／被压区原文移出请求）＋既有 v0.2/v0.3 机械压缩钉子 | ✅ |
| ② | 500K 提醒不设 pending／不开窗 | `agent_loop::tests::context_scale_500k_is_a_pure_reminder_without_a_window`（工具面不收缩、`opens_window=false`、无 `model_compression` 事件） | ✅ |
| ③ | 900K 窗口请求含待压区 | `agent_loop::tests::compression_window_request_loads_the_pending_region`（非窗口轮原文不可见／窗口轮原文可见且估算 ≥ 滑窗视图） | ✅ |
| ④ | 950K 兜底 once＋冷却（压不动 ＝ 触发数 1 ＋ anomaly） | `agent_loop::tests::hard_context_stall_records_one_anomaly_and_stops`（anomaly 恰 1、无 `context_compressed`、不开窗）＋ `hard_context_fallback_fires_on_the_actual_context_scale`（压得动 ⇒ 不停手不落 anomaly） | ✅ |
| ⑤ | 会话级水位跨 prompt 幂等（host 侧） | `agent_loop::tests::context_scale_watermark_is_session_level`（已提醒键不重发）＋ `acp_server::tests::cross_prompt_conversation_continues`（预置水位经 prompt 起始注入 → 成功回写原样往返）＋侧车往返/legacy 兼容钉子 | ✅ |
| ⑥ | `mode=model_summary` 三处契约一致 | `runtime/tests/test_run_event_journal_validation.py`（Python 镜像 4 钉）＋ `orz-assurance` `s1_revision_context_compressed_tests`（Rust 法官 3 钉；含未知 mode/reason 仍被拒） | ✅ |
| ⑦ | 定位指针四项齐备 | `summary::tests::compact_marker_carries_four_locator_pointers`（四项齐备＋缺项「（无）」）＋端到端：`model_summary_block_replaces_the_region` 断言 marker 带 journal run 与 sidecar 路径 | ✅ |

**实跑读数（2026-09-15，本批）**：

- `cargo test -p orz-loop --lib` → **811 passed / 0 failed / 3 ignored**；
- `cargo test -p orz-host --lib -- --test-threads=1` → **325 passed / 0 failed / 5 ignored**（并行跑时 `call_tool_timeout_kills_process_tree` 等进程树/终端类钉子偶发失败——单跑与串行跑均通过，F-1 环境抖动，与本批无关）；
- `cargo test -p orz-assurance` → **229 + 各 fixture 套件全绿**（含 Rust↔Python verdict parity）；
- `cargo fmt --all -- --check` 干净；`cargo clippy -p orz-loop -p orz-host --lib` **本批新增代码零告警**（余量为既有风格类告警）；
- `python -m pytest runtime/tests -q` → **361 passed / 1 failed**（唯一失败＝既有无关红灯 `test_v02_all_51_event_types_covered`，断言 55 vs registry 65；本批未修、与本批无关）；
- `python scripts/check_repository.py` → `valid: false`，**唯一 error ＝ `orz submodule working tree is dirty`**（本批未提交所致，非内容违规）；
- 历史 journal 抽样校验（`.gsa/runs/*/events.jsonl`，27 个）：**`context_compressed` 家族 0 违规**（其余失败为历史 run 未收尾／digest 类既有事实，与本批无关）。

**未核项（如实标注）**：

- **A/B 口径全部未跑**（同题每臂 ≥2 run、四件套读数、驱逐次数增幅确认、存档三键齐备率、本地增长四类读数）——属批序 ②，另行放行；
- 900K 窗口「过大时分段」**未实现**：本批按「窗口轮上传 `messages` 全量」落地（实际上下文估算 <1M，换算后 prompt ≈0.77×；若将来 S2 展开膨胀使窗口请求逼近 provider 上限，分段属另行裁决项）；
- 语义摘要块的**质量**（是否真保住任务线）只能由 A/B 质量/重读率读数回答，本批只保证机械通路与契约；
- **恢复面（**审查修正批 2026-09-15 更正：原表述写反，按实际实现保留**）**：压缩 marker（机械 v0.2／v0.3 与语义 `v0.3-语义` **共用** `[前文上下文已压缩` 前缀）**既是注入块文本、也是 restore-retained 块**——写回过滤器的判据是双条件 `is_injected_block_text(...) && !is_restore_retained_block(...)`（`orz-loop/src/controller.rs`），而 `is_restore_retained_block` 对 `CONTEXT_COMPRESSED_PREFIX` **恒真**（D3-1，`orz-loop/src/prompt.rs`；钉子 `prompt::tests::restore_retained_blocks_identified`）⇒ marker **会**写回 conversation sidecar、**会**随恢复回到模型上下文。故语义摘要正文除 `.gsa/compaction/<id>.md`（存档）与 `context_compressed` 事件的 id/digest 之外，**还随侧车跨恢复留存**——这比原表述更好，**按实现保留**（不改码）。
- 磁盘：本轮 `--all-targets` clippy 曾把 `D:` 写满（0 字节空闲），已删除 `orz/target/debug/incremental`（构建缓存，可重建）释放 ≈10.6 GB；源文件未受影响。

### 10.8 审查修正批（2026-09-15；对 §10.7 落码的全面审查 → 按用户「按建议处理」放行并当日落码）

> 用户令「请对当前实现的批次 A（S1 修订批 v7）实施全面检查（设计合理性／实现合理性／设计与实现符合性）；请对审查出的全部问题进行处理」。审查结论＝**六条主线接线与契约同步经复核成立**（机械轨仍真实作用于上下文、500K 不开窗、900K 绕过折叠装载全量、契约三处逐字一致、`attention_ladder` 零写入），并发现 **P1×2、P2×3、P3×5**；本节为全部发现的处理回执（**代码仍未提交**（提交前读数；已由 §10.5 提交收尾闭合），orz HEAD 仍 `1f303cf4`）。

**处理清单**（逐条对应审查发现）：

| # | 审查发现 | 处置（落点） |
|---|---|---|
| **P1①** | 恢复面表述写反（见 §10.7 未核项末条） | **改稿保留实现**：§10.7 该条按实际（marker 为 restore-retained、语义摘要随侧车跨恢复留存）更正；零代码改动 |
| **P1②** | **终止轮的语义摘要块被静默丢弃**：`pending_semantic` 此前只有 loop-top 一个消费点，终答／预算耗尽／IPG 截停三处 break 直接丢弃（无事件、无 marker） | **补 run 尾安全网**（`agent_loop.rs` run 尾、与 `finalize_model_compression_close` 同型）：模型已离场、无工具在途的安全间隙补一次同形语义压缩（压得动 ⇒ 落 `mode=model_summary`＋移出被压区＋存档；压不动 ⇒ 如实 NoOp，不虚构事件）。钉子 `final_answer_semantic_summary_is_consumed_at_the_run_tail`（**已实证**：临时停用该段 ⇒ 该钉断言 `context_compressed` 数 0 vs 1 失败，复原后绿）。边界：`?` 错误传播路径仍丢弃（与 0AC-A6 同类，维持挂账） |
| **P2③** | 四项定位指针过承诺：被压区 `drain` 后不在 `messages`／sidecar，compaction 存档按设计只存摘要与指针、台账行 300 字符上限 ⇒ **只有 journal 含逐字原文** | 文案与设计按实际收窄：`LocatorPointers::render_marker_lines` 逐项标注载体层级；语义 marker「结构化轨」段与存档正文改写（逐字原文权威载体＝journal；分段回读属 S2 未实现）；钉子 `compact_marker_carries_four_locator_pointers` 增四条载体断言 |
| **P2④** | 「增量归档与提醒同尺」名不副实：loop 侧量**含注入块**的运行期 `messages`，host 侧量**已过滤注入块**的 sidecar 会话 | 口径写明（`acp_server.rs::estimate_conversation_tokens` 注释＋设计稿 §3.5.1）：同口径≠读数等值，差值＝注入块累积量；里程碑只需单调＋幂等 |
| **P2⑤** | **900K 窗口轮无上限守卫**：`messages.clone()` 全量上传，实际上下文远超 900K 时该轮请求硬失败 | **fail-soft 降级**：新增 `window_upload_cap_tokens`（默认 **1.10M**，env `ORZ_CONTEXT_SCALE_WINDOW_CAP_TOKENS`；1M 真实窗口 ÷ 实测换算 0.77 ＋ 余量）＋ `context_scale::window_skipped_over_cap_block`——越上限则**不开窗**、注入降级块（如实报实际读数／上限／分段未实现，自选压缩通路仍在）、同迭代走机械强制压缩（`reason=context_scale`）、事实经 `mechanical_audit_update{key=context_scale:<档位>}` 的 `anomaly=window_upload_over_cap` 落账；**950K 档的强制开窗同受该守卫**（`window_open_skipped_over_cap` 入 summary）。钉子 `window_over_upload_cap_degrades_instead_of_opening_a_window`＋`context_scale::tests::over_cap_block_reports_the_degradation_and_keeps_the_self_selected_path` |
| **P3⑥** | 收尾／检索路径 `journal_seq=Some((0, seq))` 与注释「如实留（无）」矛盾 | 按事实改正：`0 → 当前 seq` 是**整 run 事件跨度**（run 自 `run_started`(seq=0) 连续编号），非占位值；注释同步（`controller.rs`／`retrieval/dispatch.rs`）；台账跨度确不可得 ⇒ `ledger_seq` 不设 |
| **P3⑦** | 950K「停手」范围与设计表格措辞不符（两类都停手；「压得动但仍在线上」实际不停手） | 设计稿 §3.4.1 档位表按实现改写（见该表 950K 行） |
| **P3⑧** | 账本行「门禁 `valid: true`（除…一条）」自相矛盾 | 三处（索引 v3.36／TODO 0ah／BACKLOG 0ah）改为如实：`valid: false`，**唯一 error＝「orz submodule working tree is dirty」**（本批未提交） |
| **P3⑨** | 语义轨事件 `retained_rounds` 报 `recent_tail_rounds`(=2)，与保留边界（驻留带）不符 | 语义轨按压缩后 `messages` 里仍在的完整轮数如实报（机械轨照旧报配置尾轮数）；schema `retained_rounds.description` 同步 |
| **P3⑩** | 台账 `[seq]` 区间在并行主车道会话下可能跨到他人行（读尾号→追加，非跨进程原子） | 注明边界（`action_ledger.rs::append_ledger_rows_range` 注释）；严格隔离（会话域／写锁）本批不动 |

**实跑读数（2026-09-15 修正批，全量重跑）**：

- `cargo test -p orz-loop --lib` → **814 passed / 0 failed / 3 ignored**（新增 3 钉：run 尾语义消费、越上限降级、降级块文案；`compact` 默认值断言扩 1 项）；
- `cargo test -p orz-host --lib -- --test-threads=1` → **325 passed / 0 failed / 5 ignored**；
- `cargo test -p orz-assurance` → **229 lib ＋ 各 fixture 套件全绿**（含 Rust↔Python parity）；
- `cargo fmt --all -- --check` 干净；`cargo clippy -p orz-loop -p orz-host --lib` **新增代码零告警**（复核：全量 52+12 条逐条落在未触及行）；
- `python -m pytest runtime/tests -q` → **361 passed / 1 failed**（唯一失败＝既有无关红灯 `test_v02_all_51_event_types_covered`：断言 55 vs registry 65，本批未触碰 `run-event-v0.2.schema.json`）；
- `python scripts/check_repository.py` → **`valid: false`，唯一 error ＝「orz submodule working tree is dirty」**（本批未提交）；manifest `--check` valid、1450 条。

**未核项（修正批）**：A/B 四件套读数、900K/950K 真机长会话、增量归档真机跨里程碑——均仍未跑（属批序 ②）；**>1.10M 实际上下文的真机行为**（降级路径）只有单测覆盖；「过大时分段」仍未实现（降级≠分段）。

### 10.9 落码回执（950K 失败处置：硬截留 ＋ 明确告知；用户 2026-09-15 裁定并当日落码）

> 用户口径：「压不动的话不进 NoOp 了，强硬只保留当前滑块，将其他的丢弃，并明确返回『上一轮上下文压缩失败，已机械截留』，让模型自己决定下一步，这样的话任务还能继续」。本节为该项的执行回执（**代码仍未提交**（提交前读数；已由 §10.5 提交收尾闭合）；orz HEAD 仍 `1f303cf4`）。**取代** §10.8 的 P2⑤ 行里「950K 档的强制开窗同受该守卫」与 v7 ④ 的「残留 ⇒ 强制开窗／无可压内容 ⇒ anomaly 停手」两分处置（设计稿 §3.4.1 表 950K 行／§5 判据／§8 已同步改写）。

| # | 项 | 落点（实现事实） |
|---|---|---|
| ① | **压不动 ⇒ 硬截留（只保留当前滑块）** | 机械层照常强制压一次（`force=true`、`reason=context_scale`）——`kept_start` 取驻留带预算 L 换算的完整轮切点（`bridge_cut(bridge_estimate_budget(L))`），即视图只剩「前置＋固定指针＋当前滑块」；被移出轮次的行入台账、逐字原文留 journal 与本地档案。**不进 NoOp、不永久停手**（删 `hard_context_stopped`） |
| ② | **明确返回「上一轮上下文压缩失败，已机械截留」** | 新增 `context_scale::TRUNCATION_FAILURE_HEADLINE` ＋ `compaction_failed_truncation_block(dropped_rounds, ledger_hint, actual_tokens)`（前缀 `[CONTEXT_SCALE` ⇒ 注入文本、不写回持久化会话）：**有截留**报「滑块之外的 N 轮已移出」＋台账/回读指引；**零截留**如实报「滑块之外已无可截留内容——溢出体量位于滑块内（设计上滑块不被压缩）」；末句给出低成本选项（按指针回读／避免重复整读超大文件／固化到黑板）＋「任务无需中止」 |
| ③ | **事实落账／不再强制开窗** | 新 key `context_scale:hard_950k_intercepted`，anomaly `hard_context_compaction_failed_truncated`（有截留）／`hard_context_compaction_failed_slider_bound`（零截留），summary 带 `dropped_rounds=`／`slider_only_view=true`／`window_opened=false`；**950K 档不再强制开窗**（`last_resort_block` 随之退役、生产零调用，保留留档）；有截留时每次都落账，零截留的重复发生只落一次（避免同一持久条件逐轮刷账） |
| ④ | **告知节流** | 告知块**每 run 一次**（`hard_emergency_noticed`）；**冷却仍在**（`HARD_CONTEXT_COOLDOWN_ROUNDS=4`）——冷却过后若又有新的滑块外轮次累积，仍会（且只）再截留一次：这是「不是逐轮重压」而不是「停手」 |
| ⑤ | **判据（新钉子，先写后跑）** | `hard_context_failure_intercepts_and_tells_the_model_without_stopping`（零截留形态：anomaly 恰 1、`dropped_rounds=0`、无窗口轮、注入块带 headline＋「已无可截留内容」＋「任务无需中止」、仍具注入块身份）＋ `hard_context_failure_with_dropped_rounds_reports_the_count`（有截留形态：小轮在前、巨型轮在后 ⇒ 移出 1 轮后仍在线之上，anomaly `..._truncated`＋`dropped_rounds=1`＋告知块报「滑块之外的 1 轮已移出」＋本迭代确有 `reason=context_scale` 的机械压缩）＋ `context_scale::tests::truncation_failure_block_states_the_fact_and_leaves_the_next_step_to_the_model`（两条事实形态的文案与注入前缀） |

**实跑读数（2026-09-15，本节）**：`cargo test -p orz-loop --lib` → **816 passed / 0 failed / 3 ignored**（+2 钉）；`cargo test -p orz-host --lib -- --test-threads=1` → **325/0/5**（**如实登记**：与 `pytest runtime/tests` 并行跑时 `tests::call_tool_timeout_kills_process_tree` 曾因 2s 墙钟预算负载抖动失败一次，逐条单跑 4.56s 绿、无争用全量串行复跑亦 325/0/5 绿——与 §10.3／§10.8 记载的 F-1 环境抖动同型，非本批代码面）；`cargo test -p orz-assurance` → **229** ＋ fixtures 全绿；`cargo fmt --all -- --check` 干净（本节首跑曾因格式差异失败，`cargo fmt --all` 后复跑干净并重跑单测仍 816/0/3）；`cargo clippy -p orz-loop -p orz-host --lib` **新增代码零告警**；`python -m pytest runtime/tests -q` → **361/1**（唯一失败＝既有无关红灯）；`python scripts/check_repository.py` → **`valid: false`**（唯一 error＝orz 未提交）；manifest `--check` valid。

**未核项（本节）**：① **真机长会话下的「告知 → 模型自行收敛」效果**（是否据此减少重读、是否把结论固化到黑板）只能由 ② A/B 的读数回答；② 零截留形态（溢出体量位于滑块内）在真机的出现频率未知——设计上滑块不被压缩，此时机械层到此为止，最终防线仍是 0z 本地资源门；③ 「>1.10M 降级」与本节截留的**叠加真机路径**未跑（单测分别覆盖）。

### 10.10 落码回执（窗口内溢出：超大工具结果指针化 ＋ 模型面措辞改「当前上下文窗口」＋ 告知不报容量；用户 2026-09-15 裁定并当日落码）

> 用户口径三条：① 「我同意你的建议，请按照这个方向再落一条」（＝把 §10.9 未核项②的「溢出位于滑块内 ⇒ 机械层到此为止」改为**窗口内也能机械消化**：按 OUTPUT-DEGENERATION-GUARD 先例把超大工具结果正文换成指针）；② 「模型知道什么是滑块吗，是否考虑将其换成『当前上下文窗口』？」；③ 「『当前实际上下文 ≈1.01M token（1010000 token）』这句不加吧？毕竟实际本地存储的容量是没有限制的」。本节为三条的执行回执（**代码仍未提交**（提交前读数；已由 §10.5 提交收尾闭合）；orz HEAD 仍 `1f303cf4`）。

| # | 裁定 | 落点（实现事实） |
|---|---|---|
| ① | **窗口内超大工具结果指针化** | `action_ledger::pointerize_oversized_tool_results(messages, target, per_result_cap, journal_path, run_id)`——截留（窗口外轮次移出）之后仍在线之上时启用：把超过 `OVERSIZED_TOOL_RESULT_CAP_TOKENS`（默认 **8K** 估计，依据＝实测读/计划轮 1–3K、终端轮 5–7K）的 `Role::Tool` 正文换成「**原文头部（≤400 字符）＋ 回读指针**」；指针指向 run journal `events.jsonl`（按 `call_id=` 检索）并声明「这是读取当时的快照、可能已陈旧，编辑/决策前以新鲜读取为准」（同 ADR-0010 §14.33 纪律）；**大者优先**直到估算 ≤ 硬线或没有候选。不变量：`role`/`tool_call_id` **不动**（配对不破坏）、**幂等**（已带 `[工具结果已机械指针化` 前缀者不再入选）、不动非工具消息 |
| ② | **模型面措辞＝「当前上下文窗口」** | 全部模型可见文案去掉「滑块／驻留带」：500K 纯提醒／900K 升级块／首次驱逐固化提醒／窗口任务块／窗口降级块／压缩失败告知块／S1 台账固定指针消息（`build_pointer_message`）——统一为「**当前上下文窗口**（之外／之内）」。内部注释与设计稿仍用「滑块／驻留带／滑窗」作为分区术语（两条词汇各归其位） |
| ③ | **告知不报容量读数** | `compaction_failed_truncation_block` **删除**「当前实际上下文 ≈x.xxM token（N token）」句（本地存量无上限，读数对该动作无指导意义）；事实改为**逐项如实**：窗口外 N 轮已移出（＋台账/回读指引）／窗口内 M 个超大结果正文已换成指针（＋`call_id` 指引）／两者皆无 ⇒「窗口外已无可截留、窗口内也没有值得指针化的超大结果——溢出体量位于前置/窗口框架内，机械层到此为止」。**500K/900K 提醒仍带自身读数**（A6 裁定未变） |
| ④ | **落账扩展** | 同一 key `context_scale:hard_950k_intercepted`，anomaly 三值：`hard_context_compaction_failed_truncated`（有轮次移出）／**新增** `hard_context_compaction_failed_result_pointerized`（仅窗口内指针化）／`hard_context_compaction_failed_slider_bound`（两者皆无）；summary 增 `pointerized_results=`／`freed_tokens=`（`dropped_rounds=`／`slider_only_view=`／`window_opened=` 保留） |
| ⑤ | **判据（新钉子，先写后跑）** | `action_ledger::tests::oversized_tool_results_are_pointerized_idempotently`（只替换超门槛者／释放读数／达线即停／头部＋`call_id`＋events.jsonl＋「新鲜读取」文案／小结果不动／配对未破坏／二次调用零动作）＋ `agent_loop::tests::in_window_oversized_result_is_pointerized_until_under_the_line`（单轮 20K 字符 ⇒ 窗口外无轮次可截留，指针化后 `actual_context_tokens_after < 硬线`；anomaly＝`..._result_pointerized`；`dropped_rounds=0`／`pointerized_results=1`；模型看到的该结果＝指针行、配对字段仍在；告知块带指针化事实、不带轮次事实、不带 `token` 读数）＋ `context_scale::tests::truncation_failure_block_states_the_facts_without_a_capacity_reading`（三种事实组合；断言**不含** `token` 与「滑块」）＋ 既有两钉按新文案更新 |

**实跑读数（2026-09-15，本节）**：`cargo test -p orz-loop --lib` → **818 passed / 0 failed / 3 ignored**（+2 钉）；`cargo test -p orz-host --lib -- --test-threads=1` → **325/0/5**；`cargo test -p orz-assurance` → **229** ＋ fixtures 全绿；`cargo fmt --all -- --check` 干净；`cargo clippy -p orz-loop -p orz-host --lib` **新增代码零告警**；`python -m pytest runtime/tests -q` → **361/1**（唯一失败＝既有无关红灯）；`python scripts/check_repository.py` → **`valid: false`**（唯一 error＝orz 未提交）；manifest `--check` valid。

**未核项（本节）**：① **8K 门槛／400 字符头部／「大者优先」策略**是工程取值（env 未开放，常量在 `action_ledger`），真机是否出现「头部过短，模型仍按记忆决策」或「门槛过低，正常轮被指针化」须由 ② A/B 核；② 指针化后模型**实际回读率**（是否真去 `read_file`／grep journal）只能由 A/B 统计；③ 与 500K/900K 提醒、>1.10M 降级的**叠加真机路径**未跑（单测分别覆盖）；④ 术语切换后**模型行为是否有差异**（「当前上下文窗口」是否比「滑块」更可执行）留 A/B 观察。
