# 设计-实现偏差全面审计（2026-08-06）

**日期**: 2026-08-06
**背景**: Aider Polyglot 摸底（92 题，82/92）暴露 P1-P9 问题；按用户指示，**修复前**全面确认设计与具体实现间的差异，为修复提供设计依据。
**方法**: 6 个独立审计代理并行（crate 矩阵 / orz-loop / orz-host / 双 TUI / orz-assurance / Python 侧 GAK 状态），全部判定带源码证据（file:line）；合成阶段对关键交叉点独立复核（见 §5）。
**参照系**:
- 当时设计基线: `存档/architecture/pre-adr-0010/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`（主）+ 同目录 v0.1（IP 定义原始出处）+ `存档/architecture/pre-adr-0010/IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`（既有偏差登记先例）+ `architecture/DESIGN_TO_IMPLEMENTATION_GAP_AUDIT_2026-08-02.md`（GAK-01~09 前次审计基线）；当前权威已转为 ADR-0010
- 当时约束文档: `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`、`存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md`、`protocol/PROTOCOL_DRAFT_v0.1.md` §9、`architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md`；当前裁决见 ADR-0010
- 实施记录（区分已记录偏差）: Slice #4/#7/#8/#12/#13/#16/#17（CLI_PROJECT_INDEX.md + docs/）

---

## 1. 总体结论

**无 P0（协议/安全级破坏性偏差）**。核心安全与完整性属性全部与设计一致且有测试锁定：依赖方向单向（Grok 底座零反向引用）、fail-closed 权限链、journal 哈希链（阻塞 send + 篡改检测）、deny 回放完整（P1 修复后无其他漏回放路径）、IP1 thinking:disabled 全路径覆盖、IP3b 4 判定点清零状态机（ADR-0005 全项）、快照 evidence 层语义、permit 逐条合规、Python reference-spec + 交叉验证（Slice #17）与声称一致。

**P1 ×2、P2 ×13、P3 记录类 ~30**。结构性问题集中在四类：
1. **产品层挂死风险**（LOOP-16，P1）：transport 无超时 × 非流式无限重试 × 零重试边界被静默越过
2. **跑分主缺口未修复**（LOOP-03/P3、LOOP-14/P7，P2）：工具可用性断链、MAX_TOOL_ROUNDS 无设计依据——均需**先定设计再修**
3. **设计承诺未兑现**（HOST-17/MAT-15 等，P2）：§3.2 桥接 9 依赖未接线、envelope 未实现、IP6 机制解释偏移无 ADR
4. **文档体系漂移**（~15 处，P3）：v0.2 多节过时、契约过期、裁决只存在于 commit/注释/Slice 记录而未见 ADR

---

## 2. 发现清单（按审计面）

### 2.1 Crate 矩阵（MAT-01~18）

| # | 设计条款 | 实现证据 | 判定 | 严重度 |
|---|---|---|---|---|
| MAT-01 | §2.2 删 orz-telemetry（14,960 死代码） | **未删**，裁剪后 ~4,851 行；被 orz-workspace/orz-mcp/orz-memory 真实使用（Cargo.toml:30,314） | 偏差但已有记录（Slice #13）；**无 ADR/设计更新** | P2 |
| MAT-02 | §2.2 删 orz-config-types | **未删**（~2,690 行），被 workspace/shared/memory 依赖 | 同上 | P2 |
| MAT-03 | §2.2 删 orz-http | 未删，16 行 stub，传递不可达（仅 orz-memory 引用，orz-memory 产品不可达） | 偏差但已有记录 | P3 |
| MAT-04 | §2.2 删 orz-announcements | 未删，15 行 stub 但 **LIVE**（orz-config-types 用 RemoteAnnouncement，Slice #13 判） | 偏差但已有记录 | P3 |
| MAT-05 | §2.2「删除（一次完成）」+ Phase 0 断裂预期「预期低」 | 删后编译断裂 → `61082d6` stub → `c4e8de3` 恢复——**设计低估 kept crates 耦合**（与 §2.1「不改代码」冲突） | **设计缺口**（可行性评估失实）+ 实现选裁剪保留 | P2 |
| MAT-06 | §1「零死代码 crate」纪律 | `orz-subagent-resolution` **完全死成员**（0 Cargo.toml 引用、0 src 使用）；Slice #13 记录称「按设计 §4 kept-provider 保留」但设计清单无此 crate | 偏差但已有记录（记录依据是宽松解释） | P2 |
| MAT-07 | §0「Crate 数 74 → ~42」 | 实际 **64** 成员；+20 = 4 待删保留 + 15 未枚举传递依赖 + orz-codex；设计自身算术不自洽（74−14+5=65≠42，另 ~20 个未列出） | 目标失实；设计矩阵未覆盖依赖闭包 | P2 |
| MAT-08 | §1「Grok 底座永不反向依赖自研」 | codegen/common/build/prod 全 Cargo.toml grep 0 命中 | **与设计一致** | — |
| MAT-09 | §1 单向链 orz-bin→tui→host→loop→assurance | 无反向边无环；tui/bin/codex 对 loop/assurance 跳层直连 | 与设计一致（方向合规，线性放宽） | P3 |
| MAT-10~13 | §2.3 行数估 | host 6,963 ✓ / assurance 6,480 ✓ / loop core 4,085 超估 ~2 倍 / tui 11,282 超 40-125%（无记录） | 位置一致；规模估偏差 | P3 |
| MAT-14 | §2.4 兜底 UI「Codex ratatui TUI ~20k 行」 | orz-codex 2,607 行重写落地（Slice #12 裁决记录齐备）；§2.3 表未列 orz-codex | 偏差但已有记录（表遗漏） | P3 |
| MAT-15 | §3.2 orz-host 依赖表 | **缺 9 声明依赖**（sandbox/mcp/chat-state/hooks/auth/secrets/config/models/tools-api）未接线；增 2（orz-paths/xai-tool-runtime 真实使用）；Cargo.toml NOTE「Phase 3 接线」已随 Phase 3 收官**过期** | 偏差（记录内容过期）；LoopHost 对应方法均为默认 stub | P2 |
| MAT-16 | §3.3 结构 5 文件 | 实际 10 文件（+keystore/permission/stdio/codex_*/tools，无 config.rs） | 偏差但已有记录（slice 齐备） | P3 |
| MAT-17 | §1「Telemetry 编译时 gate」 | 无初始化 ✓（零外发）；机制为 env 门控非编译时 gate | 效果一致、机制偏差 | P3 |
| MAT-18 | §2.1「保留不改代码」 | kept crates 有维护性修改（clippy 128→0、dunce、Windows 修复 d48b724），逐条记录 | 偏差但已有记录（语义未变） | P3 |

### 2.2 orz-loop（LOOP-01~16）

| # | 设计条款 | 实现证据 | 判定 | 严重度 |
|---|---|---|---|---|
| LOOP-01 | §3.4 LoopHost 9 方法 | 9 方法全在且签名一致；实现为超集 +2（`workspace_trust` host.rs:157、`on_text_delta` host.rs:192），**设计文档未同步** | 与设计一致（超集）；文档缺口 | P3 |
| LOOP-02 | IP1 thinking:disabled（v0.1「body 固定 {"thinking":{"type":"disabled"}}」） | transport.rs:162-164 唯一序列化点显式 disabled；流式/非流式/子代理/plan 变体全路径覆盖；reasoning_effort 恒 None（测试锁定） | **与设计一致**（32b319d 后状态） | — |
| LOOP-03 | IP2a TOOL_AVAILABILITY（v0.1「build() 末尾追加 context block 注入一次」） | controller.rs:294-297 headless **恒 `Some(true)`**（注释自认「every advertised tool is available」，与权限策略零关联）；:421 `tool_defs.clone()` 全量静态传递；block 每轮重建（非设计「会话开始注入一次」） | 偏差但已有记录（跑分 P3 未修复）+ **设计缺口**（IP2a 未定映射语义、未纳入 CN「真实工具状态」约束） | **P2** |
| LOOP-04 | IP2b ORIENTATION_CHECKPOINT（v0.1「注入下一轮 system prompt prefix」；v0.2 图「事件驱动 + cooldown」） | controller.rs:328-343 每轮事件记录；**无 pending_prefix 注入**（零命中）；cooldown 推迟仅存代码注释（orientation.rs:1-6），无 ADR | 与 Python canonical 一致；**设计文档内部矛盾**（v0.1 注入 vs v0.2 cooldown vs 实现 parity），取舍无记录 | P3 |
| LOOP-05 | IP2c INFO_SUFFICIENCY（§4.6.5 模板） | prompt.rs:18-23 与模板**逐字一致**；Role::User 注入；测试锁定 | 与设计一致 | — |
| LOOP-06 | IP3a IPG gate（调用 orz-tools 前） | tool.rs:37-46 → gates/ipg.rs:285；controller.rs:540 提升到**轮级、权限前**（早于设计要求，语义等价，注释记录） | 与设计一致 | — |
| LOOP-07 | IP3b 4 判定点（ADR-0005：>10/>10/>10/>8、顺序固定、触发即三实例清零） | inquiry.rs:47-52 阈值 ✓、:91-105 严格大于 + 顺序 ✓、controller.rs:776-819 触发即全清零 ✓、两检索子代理实例化 ✓、子代理语义动作（feed_semantic_action 不动 tool_calls）✓ | **与设计一致**（ADR-0005 全项） | — |
| LOOP-08 | IP3c sufficiency trigger（检索分发后） | controller.rs:626-652 检索分发完成后触发 ✓ | 与设计一致 | — |
| LOOP-09 | IP5 接线裁决（恢复路径「尚未接线」） | 提取/越界剔除/权限后 ToolStarted 前 track/evidence 层 ✓；**恢复路径已接线**（acp_server.rs:502 restore_snapshot + RST- run + snapshot_restored 事件，Slice #8）——**v0.2 §4 裁决段过时** | 与设计一致（含裁决）；文档陈旧 | P3 |
| LOOP-10 | §4.6.6 三事件（neutral_inquiry/counterexample_gate/retrieval_completion_check）+ evidence-only | 三事件全发出（event.rs:54-56，schema 逐字段匹配）；模型回答无控制流后果 ✓ | 与设计一致（schema 现 33 变体为后续扩展） | — |
| LOOP-11 | 反例询问两位置（仅一次 + 显式告知 + 草稿不进会话） | controller.rs:490-519 终答前拦截（once_only ✓、草稿 model_output 不入 messages ✓）；orz-bin main.rs:313-353 plan 写入前（once_only:false ✓）；block 文本逐字 ✓ | 与设计一致 | — |
| LOOP-12 | 子代理 completion check（逐行解析 yes/no/uncertain、事件带全文） | agents/retrieval.rs:66-76 + inquiry.rs:117-131；Python `RETRIEVAL_COMPLETION_CHECK` 逐字；失败路径不记录（注释 D8） | 与设计一致 | — |
| LOOP-13 | P1 教训「任何提前 return 审计回放」 | deny/defer 分支 push Tool 消息配对（controller.rs:999-1033，内容含「NOT available … do not retry」）；错误路径/子代理错误/截断/取消全路径无漏 | **与设计一致**（3e32ed7 修复完整）；教训未回流设计文档 | — |
| LOOP-14 | P7 MAX_TOOL_ROUNDS=8 | controller.rs:53 常量；**设计无任何条款定义工具轮次硬上限**（设计中的「8」全指问询判定点「轮次 > 8」，语义不同）；Python 参考实现实为 `max_turns: 20`/`max_tool_calls: 0`（canonical_cli.py:224-229）——**P7 记录「Python 移植」失实** | **设计缺口** + 偏差但已有记录（记录理由失实） | **P2** |
| LOOP-15 | Blackboard 5 分区 + MechanicalRelay 纯路由 | blackboard.rs:74-81 5 分区 ✓ 写者纪律 ✓；relay.rs:36-44 纯前缀匹配 ✓ | 与设计一致 | — |
| LOOP-16 | DEEPSEEK_ADAPTER_CONTRACT §2.5/§4「首事件 timeout、bounded retry、未收到首事件 vs 生成中断分开记录」；§2.3 transport「从 orz-sampler 移入」 | **无超时**（transport.rs:95-101 fork reqwest 零超时，注释自认 :398；非流式/无 token 流式均无）；**非流式 create 无限重试**（fork client.rs:558-596 backoff 默认无上限——与 08-02「显式零重试是 assurance 边界」记录及 Python `retry_policy: none` **冲突**，无 ADR 授权）；transport 为**重写**非移入（git 证实 orz-sampler 旧树无此文件，3d81e41 新建，注释 D3-2 记录） | 偏差但已有记录（P8 未处理）+ **契约冲突**（三处组合=挂死/无限重试循环，P8 实测 201s） | **P1** |

### 2.3 orz-host（HOST-01~18）

| # | 设计条款 | 实现证据 | 判定 | 严重度 |
|---|---|---|---|---|
| HOST-01 | §3.1 薄核心 ~8k 只做桥接 | 6,963 行（非测试 3,331）；无 loop/gate 逻辑混入；RunRecorder 属 §1 Journal Writer 职责 | 与设计一致 | — |
| HOST-02 | §3.3 结构（含 config.rs） | 无 config.rs（配置走构造参数 + orz-bin 组装）；Cargo.toml NOTE（P2-7）过期（同 MAT-15） | 偏差但已有记录（config 缺失无记录） | P3 |
| HOST-03 | IP4a bootstrap（trust, envelope, journal create） | trust ✓（fail-closed 测试锁定）；journal ✓；**envelope 未实现**（lib.rs:129-135 KNOWN GAP 注释 + 索引记录） | 基本一致；envelope 偏差但已有记录 | **P2** |
| HOST-04 | IP4b turn lifecycle | run-id 先于 bootstrap 预留、token 注册提前、Cancelled→成功响应、shutdown 全路径 ✓ | 与设计一致 | — |
| HOST-05 | IP4c session close（journal seal → receipt） | shutdown 全路径 ✓；**receipt 概念未实现**（grep 零命中，由 replay_journal/verifier 取代，无记录） | 一致（功能无损失） | P3 |
| HOST-06 | IP6「唯一需要修改 kept Grok 组件的注入点」+ v0.2 §6「keept crate 未修改」 | `spawn_permission_manager_with_hub` 唯一调用方=permission.rs:146；但 **kept permission 决策点零修改**（manager.rs/prompter.rs/policy.rs 自 fork 基线 diff 为空），IP6 执行全在 host 桥（yolo 恒 off/headless Ask→Deny/300s/P1 scope/ReadOnly 短路）；实现自述「走公共 seam 的桥」与设计文字实质偏移，**无 ADR**；v0.2 §6 与 orz-workspace 实际 21 文件改动（Windows 适配等，部分记录）矛盾 | **偏差且无记录**（安全属性当前靠「桥=唯一调用方」成立） | **P2** |
| HOST-07 | §2.4 双 TUI（同 host 双面、assurance 静默） | CodexAppServer 组合同一 Arc\<AcpServer\>（codex_app.rs:245-247）；assurance 完整静默运行；orz-bin 零 codex 引用 | 与设计一致 | — |
| HOST-08 | Slice #7 /stop 真取消 | 幂等 bool（acp_server.rs:258-277）、2s 窗口、token 提前、Cancelled→PromptResponse 成功响应（stdio.rs:98-102）✓ | 与记录一致 | — |
| HOST-09 | Slice #7 300s 倒计时 + §3.3 approval.rs | 功能一致（PERMISSION_PROMPT_TIMEOUT=300 pub；倒计时实现实际在 orz-tui）；**approval.rs 为 6 行 TODO stub** 与 §3.3 结构不符 | 偏差且无记录（stub） | P3 |
| HOST-10 | Slice #8 restore_snapshot（RST- 独立 run/scope/fail-closed/无权限流） | acp_server.rs:502-610 全项 ✓（计数独立、`..` 逃逸拒、运行中拒、shutdown） | 与记录一致 | — |
| HOST-11 | Slice #4 keystore + permit 三入口 | keystore.rs 32B/MAGIC/固定熵/KEY-id/schema const/DPAPI fail-closed ✓；三入口 Enforce+signer ✓ | 与记录一致 | — |
| HOST-12 | Slice #6 流式 text-delta | on_text_delta → agent_message_chunk（lib.rs:251-268）+ 120ms 守卫 ✓ | 与记录一致 | — |
| HOST-13 | Slice #16 sandbox read-only | ReadOnly 短路于 handle.request **前**（permission.rs:186-190）；三态 -32602；exit 2；`__` 恒拒 ✓ | 与记录一致 | — |
| HOST-14 | §1 Telemetry 硬无（编译时 gate） | 零 init/零外发 ✓（行为）；编译时 hard-disable feature 未实现（env 门控；orz-telemetry 仍在 workspace 待裁，索引记录） | 行为一致、机制偏差但已有记录 | P3 |
| HOST-15 | §1 Journal Writer（hash-chained, 阻塞 send） | recorder.rs:88-118 blocking_send + oneshot ack（never silently drop）✓；链延续/篡改检测 ✓ | 与设计一致 | — |
| HOST-16 | §1 Approval Prompter（ACP ApprovalRequest） | 桥 request → kept prompter；codex 面 PermissionHookTransport；headless 死网关 fail-closed ✓ | 与设计一致 | — |
| HOST-17 | §1「persist_turn → orz-chat-state」 | **静默 no-op**（host.rs:195-198 `Ok(())`，OrzHost 未覆写）——比显式 NotSupported 更隐蔽；Cargo.toml P2-7 NOTE 承诺 Phase 3 接线未兑现（同 MAT-15） | 偏差但已有记录（承诺过期） | **P2** |
| HOST-18 | 「28 工具」数 | 凭 Slice #12 记录，未编译验证 | 未验证 | P3 |

### 2.4 双 TUI（TUI-01~30，全部 P3）

| # | 设计条款 | 实现证据 | 判定 | 严重度 |
|---|---|---|---|---|
| TUI-01~03,05 | CLI_UI_INTERACTION_MODEL 菜单/工具栏/StatusBar/Find 弹窗化 | 冻结布局（Slice #5）与 SUP 演进一致 | 与设计一致（按 SUP 演进） | — |
| TUI-04,07 | 键盘建议（Ctrl+L、Alt+菜单、Alt+Left/Right、F5） | 未实现且无记录（Ctrl+Break→Ctrl+Z 有记录，其余无）；nav 栈无键盘路径（Slice #10 D3 部分记录） | 偏差但部分记录/无记录 | P3 |
| TUI-09~11 | CONTENT_PANE_CONVERSATION_RENDERING（冻结）3+1 处 | 折叠态显示「· success/· 错误」状态文字（projection.rs:188-213，违反反规则）；pin-to-top 未实现（widgets.rs:224-229）；`[展开]/[关闭]` 无键位绑定不可达；消息间空行缺失 | **偏差且无记录**（冻结文档） | P3 |
| TUI-12 | 「菜单含完整命令集」 | MenuBar.selected 从未设置；render_toolbar 静态文本忽略 enabled；命令发现由 HelpOverlay+斜杠命令承担 | 偏差但部分记录 | P3 |
| TUI-13 | §8 UI Preservation Contract | Process usage visibility（CPU/MEM）无实现；Plan/checklist 专属区域无（仅事件卡） | 设计缺口 | P3 |
| TUI-14~16 | §2.4/§1 ACP 通信 + 双 TUI 职责 | 进程内 duplex + acp_gateway ✓；journal 投影 33/33 事件全覆盖（bridge.rs:74-228，测试锁定）✓；orz-codex 无 assurance 面板断言 ✓（ASSURANCE_SURFACE 仍含「文件」微漂移） | 与设计一致 | — |
| TUI-17 | §2.4「Codex ratatui TUI ~20k」 | 自研 orz-codex 2,586 行（Slice #12 裁决记录齐备）；**§2.4 原文未修订** | 偏差但已有记录（文档陈旧） | P3 |
| TUI-18~28 | Slice #12/#10/#11 各项 | interrupt 非终局/EOF 不提升终局/单活跃 turn/-32001/审批穿透/持久化 grant/运行历史诚实标注/数值倒序/只读重放提示/D2-1~4/快照选择器/9 斜杠命令 + seq 守卫 | 与记录一致 | — |
| TUI-29~30 | --replay 链校验 + 事件投影 | ReplaySource require_terminal 链校验 + [验证] 卡 ✓；33/33 bridge 全覆盖 | 与设计一致 | — |

### 2.5 orz-assurance（ASR-01~11）

| # | 设计条款 | 实现证据 | 判定 | 严重度 |
|---|---|---|---|---|
| ASR-01 | PROTOCOL §9 五值决策（GAK-02） | lib.rs:24-52 五值枚举 ✓（构造级落地）；gate-decision schema 多 `'stop'`——**交叉复核：有 producer**（controller.rs:673 tool_rounds_limit 手写 json），契约已记录（PYTHON_REFERENCE_SPEC_CONTRACT:60,108「非豁免」）——ASR 原判「残留」有误；该 producer 绕过 GateDecision 类型（手写 json） | 与设计一致；`stop` 条目改为记录类：手写 json 不经类型 | P3 |
| ASR-02 | f86eb15 收紧（ACP probe 缺失→unavailable/block） | 代码落地（grok_tool_permission_observer.py:316-319）；**裁决仅存在于 commit 消息**，adr/docs 无记录 | 偏差但仅 commit 级记录 | P3 |
| ASR-03 | §1 Journal（hash-chained, 阻塞 send） | 33 变体 = schema 33 逐字段对齐；chain.rs canonical JSON Python parity + 双重篡改检测；recorder 阻塞 send + fsync + terminal 后拒收 ✓ | 与设计一致 | — |
| ASR-04 | §4.6.3 stagnation 阈值 + RestartRequested | 严格大于 10 ✓；RestartRequested 带 terminal_safe_restart_packet（4 项状态）✓；cooldown 归属 orz-loop（crate 侧 deferral 记录） | 与设计一致 | — |
| ASR-05 | IP5 快照（内容寻址 + 逃逸拒绝） | sha256 内容寻址 + BTreeMap 确定性；`..`/绝对路径 OutsideWorktree fail-closed ✓；git store→内容寻址替代**有记录**（session/mod.rs:16-21） | 与设计一致（一处已记录偏差） | — |
| ASR-06 | GAK-WIN-001 创建时分配 + WIN-PROC-003 取消阶梯（GAK-06） | Kill-On-Job-Close ✓；CREATE_SUSPENDED+Assign+Resume ✓（等价裁决有记录）；**取消阶梯 Rust 侧无任何实现且无归属记录**（job_object.rs 无 cancel API；全 crate grep CTRL_BREAK 零命中）——Python 侧已完整落地（windows_sandbox.py:1011-1070 4 阶段，见 PY-09），产品层（Rust）缺口 | containment 一致；**取消阶梯产品层偏差且无记录** | **P2** |
| ASR-07 | GAK-CRED-001 + DEEPSEEK_CREDENTIAL_HARDENING | RAII 零化/环境脱敏/泄露扫描/挂载审计 ✓；§2.4 有界文件扫描未移植（Python conformance 保留）；§2.1 WER 落点未验证 | 主体一致；§2.4 未移植无记录 | P3 |
| ASR-08 | permit（P1 一次性） | schema 逐字/HMAC-SHA256/O_EXCL claim/`[A-F0-9]` pattern/ttl 溢出防护全 ✓ | 与设计一致 | — |
| ASR-09 | §4.5 plan mode 状态机 | state_machine.rs 八状态与 Python R15 一致；artifact 节序强制；**无消费者接线**（全 workspace grep 零命中——待 loop 接线复核） | crate 语义一致；接线待验证 | — |
| ASR-10 | §2.3 热路径 4-7k + 纯度 | 6,480 行 ✓；8 模块无 Python 大块逻辑移植；gates/mod.rs:9-13 边界声明 ✓ | 与设计一致 | — |
| ASR-11 | §1 模块清单（未列 plan/） | 实际 8 模块含 plan/；设计正文 §4.5/§4.6.6 多处引用但 §1/§2.3/§5 清单未同步 | 偏差但已有记录（文档漂移） | P3 |

### 2.6 Python 侧 / GAK 状态（PY-01~13）

| # | 设计/审计条款 | 实现证据 | 判定 | 严重度 |
|---|---|---|---|---|
| PY-01 | GAK-01 真实 API 探针（P0 项） | 验证器/脚本/schema 齐备（deepseek_thinking_continuity.py 26 测试绿）；**真实 API 执行证据缺失**（.observed-runs 无真实记录，docs 明言「本次没有证明」） | 部分落地——执行缺口 | **P1** |
| PY-02 | DEEPSEEK_ADAPTER_CONTRACT §2.2「thinking 默认开启」 | Python 侧 disabled 修复已落地（cli.py:567-580）；**契约 §2.2 仍声明默认开启，过期未修订** | 实现已落地、契约过期（未记录裁决） | **P2** |
| PY-03 | GAK-01 验证器语义 vs thinking-disabled 新默认 | 验证器将 `reasoning_tokens==0` 判错（对 disabled 默认必失败）；controls 无 thinking_enabled 位；无裁决 | 设计缺口（未落地且无记录） | P3 |
| PY-04 | GAK-02 五值词汇统一 | 协议侧 `pass` vs assurance 侧 `allow`（instruction_provenance_gate.py:230-237 等）**词汇漂移无映射记录**；`not_applicable` 零产出路径（仅 schema/docstring） | 部分落地 | **P2** |
| PY-05 | f86eb15 收紧 | 同 ASR-02：代码落地、文档零记录（GENERAL_ASSURANCE_KERNEL_GAP_REGISTER 无 GAK 条目） | 无文档记录 | P3 |
| PY-06 | GAK-03 权限默认（契约须记录裁决） | 已落地（interactive 默认 fail-closed + auto_allow_once 仅 smoke + orz PermissionBridge 取代）；**契约无决策记录**（acceptance 未满足） | 已落地、决策记录缺失 | **P2** |
| PY-07 | GAK-04 idempotency_key | 已落地（instruction_gate.py:872-885 IDEM- key + ACT-IDEMPOTENCY-001 触发；evidence_kernel Action 字段）；以代码校验形式替代「proposal schema 文件」 | 已落地（字面项轻微偏离） | P3 |
| PY-08 | GAK-05 跨文件验证 + runner 接线 | verify_action_kernel 函数+schema+测试在（action_kernel_result.py）；**runner 接线未完成**（canonical_cli/integrated_run 零引用） | 部分落地（acceptance 未闭合） | **P2** |
| PY-09 | GAK-06 Win32 取消阶梯 | Python 侧完整 4 阶段（windows_sandbox.py:1011-1070，diags 分离记录，28 测试绿） | 已落地（conformance 层；产品层缺口见 ASR-06） | — |
| PY-10 | GAK-07/08/09 preflight 三项 | 全落地（退役别名/互斥检查/可选 /models + 防篡改守卫 + 测试） | 已落地 | — |
| PY-11 | reference-spec 契约（Phase 3 项 5） | 33/33 schema 文件（程序化核验）、good/bad fixtures 66+47、runtime/tests 114 + integration 46 绿、check_repository valid（journals 6/schemas 224）；全量 1620 本机至 62% 中止无失败 | 已落地 | — |
| PY-12 | Rust↔Python 交叉验证（Slice #17） | 注册表单一来源/按轨选 schema/12 字段投影对齐 chain.rs/Rust-parity 哈希形/6 journal 零 mismatch（本机重验通过） | 已落地与声称一致 | — |
| PY-13 | §2.3 Python 95k 保留范围 | canonical_cli 自声明 fixture-only；orz 对 assurance 全部注释级引用；CI 纯 Python | 已落地 | — |

---

## 3. 跑分问题 P1-P9 ↔ 设计条款映射（修复前依据）

| 跑分问题 | 状态 | 设计条款 | 判定 | **修复前需要** |
|---|---|---|---|---|
| P1 deny 回放缺失 | ✅ 已修复 3e32ed7 | 无显式条款（ACP 协议层约束） | 偏差已修；审计确认无其他漏回放路径（LOOP-13）；教训未回流设计 | 可选：设计补「协议完整性」条款 |
| P2 V4 thinking 默认开 | ✅ 已修复 32b319d | IP1（v0.1「body 固定 disabled」）——设计正确，曾未达 | 偏差已修（LOOP-02 全路径验证）；**DEEPSEEK_ADAPTER_CONTRACT §2.2 过期**（PY-02） | 修订契约 §2.2（P2） |
| P3 工具可用性三层断链 | ⬜ 未修复（deny 语义化已做 83764b1） | IP2a 仅规定 block 机制（v0.1）；未纳入 CN「真实工具状态」约束 | **设计缺口**（映射语义未定稿）+ 偏差（headless 恒 true 无依据，LOOP-03） | **先定 IP2a 映射定稿**（工具声明按策略过滤 + availability 反映策略），再改 controller.rs:294-297/421 |
| P4 偶发 400（thinking 回放） | ✅ 随 P2 消除 | 无条款（官方文档约束） | 防御性回放已实现（7274f24） | 无 |
| P5 harness 假 PASS | ✅ harness 侧已修（python -m pytest 归一化） | 评估设计文档无测试执行规范 | **设计缺口**：harness 在 %TEMP% 不在仓库、无版本控制、无设计条款 | 可选：评估基础设施治理（beta 测前） |
| P6 无测试反馈环 | ⬜ harness 缓解，系统级待裁决 | 无条款（自主 agent 验证闭环） | **设计缺口**（系统级） | beta 测设计裁决（工具内自测 or harness 级多轮） |
| P7 MAX_TOOL_ROUNDS=8 偏紧 | ⬜ 未处理 | 无条款（设计「8」全指问询轮次阈值） | **设计缺口** + P7 记录「Python 移植」**失实**（Python 实为 max_turns=20/max_tool_calls=0，LOOP-14） | **ADR 定稿语义/数值**（或参数化）后再调 |
| P8 非流式无超时 | ⬜ 未处理 | DEEPSEEK_ADAPTER_CONTRACT §2.5/§4：首事件 timeout、bounded retry、首事件 vs 中断分开记录 | **实现偏差**（契约明确未满足）+ 无限重试越零重试边界（LOOP-16，P1） | **直接修**（设计依据已有）：非流式加超时；重试 bounded 或显式零重试（与 08-02 记录对齐需 ADR 级确认） |
| P9 reasoning 输出侧丢弃 | ✅ 防御性关闭（随 P2） | 设计意图「不展示思维链」 | 与设计一致 | 无 |

---

## 4. 修复优先级建议

### 第一优先（跑分直接落点，P1/P2）

1. **LOOP-16（P1）— transport 超时与重试**：非流式 generate 加首事件/总超时（或 `-p` 改走 generate_stream + 强制 cancel token）；重试降级 bounded 或按契约显式 `retry_policy: none`（与 08-02「零重试边界」对齐，需 ADR 级确认重试策略归属）。设计依据已明确（DEEPSEEK_ADAPTER_CONTRACT §2.5/§4），无需先改设计。
2. **LOOP-03（P2）— 工具可用性三层闭合（P3 主缺口）**：**先定 IP2a 映射定稿**（v0.2 §4 补条款：工具声明 = 策略过滤后的真实可用集；availability = 运行时权限状态；deny = 不可用语义一次性告知），再实施 controller.rs:294-297/421 的过滤与映射。这是「修复前必须先有设计条款」的典型。
3. **LOOP-14（P2）— MAX_TOOL_ROUNDS**：ADR 定稿语义（anti-runaway backstop）与数值（8→16 或参数化），修正 P7 记录中「Python 口径移植」失实表述。
4. **PY-01（P1）— GAK-01 真实探针**：orz live 验证时以显式 thinking opt-in 执行真实探针产出 receipt，或显式关闭并记录裁决（与 PY-02/PY-03 一并处理）。

### 第二优先（结构性偏差，P2）

5. **HOST-06** — IP6 执行点解释变更补 ADR（桥=唯一调用方；kept permission 决策点零修改；未来新增调用方须审计）。
6. **HOST-17 / MAT-15** — §3.2 桥接裁决三选一：接线 orz-chat-state/hooks/auth/mcp；或显式放弃并修订设计 §3.2/§4（持久化由 journal 承担）；或 ADR 延迟。persist_turn 静默 no-op 至少改为显式 NotSupported。
7. **MAT-01/02/05/06/07** — crate 矩阵收敛：4 个「裁剪保留」决定升格 ADR 或修订 §2.2；orz-subagent-resolution 删成员或 ADR 显式保留；设计补「依赖闭包」节重核 ~64 目标。
8. **HOST-03** — envelope 校验在审批路径落地时兑现（已有记录）。
9. **ASR-06** — GAK-06 产品层（Rust）取消阶梯：记录归属（orz-sandbox 执行层 or job_object 加 cancel API——CTRL_BREAK 需 CREATE_NEW_PROCESS_GROUP），或确认 kept 底座承接后关单。
10. **PY-02/04/06/08** — 契约与词汇收尾：修订 §2.2；reference-spec §6 登记 `allow`≡`pass` 或统一；GAK-03 决策补记录；GAK-05 runner 接线或显式关闭。

### 第三优先（P3 记录类，随版本修订批量处理）

- v0.2 系列过时点：§2.2（裁剪保留）、§2.3（行数/orz-codex/transport 重写）、§2.4（自研兜底）、§3.2（依赖表）、§3.3（结构）、§3.4（+2 方法）、§4（IP5 恢复已接线、IP2a 定稿、IP6 解释、plan/ 入清单）、§6（kept 维护性修改）
- TUI-09~11（CONTENT_PANE 冻结规范 4 处）补 D 级记录或对齐实现；TUI-04/07 键盘未实现项在 Help/计划标注
- ASR-01 `stop` 记录（手写 json 绕过类型，可选类型化）；PY-03/05/07、ASR-02/07/11、HOST-02/05/09/14、MAT-03/04/09/11/12/14/16/17/18、LOOP-01/04/09 文档同步

---

## 5. 交叉验证与修正记录

1. **ASR-01 修正**：原判「schema `'stop'` 疑似旧版残留」——交叉复核：`'stop'` 有 producer（controller.rs:673 `tool_rounds_limit` 手写 `json!`），且 PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md:60,108 明确记录为有意纳入（「非豁免」）。修正为：schema 正确，条目性质改为「producer 为手写 json、绕过 GateDecision 五值类型」记录类。这也印证 LOOP-14：tool_rounds_limit 与问询阈值系统为两套并存机制。
2. **ASR-06 ↔ PY-09**：GAK-06 阶梯 conformance 层（Python windows_sandbox.py 4 阶段）已落地；产品层（Rust job_object）无实现——按审计口径「任一满足即算落地」契约满足，但产品层缺口须记录归属（P2 保持）。
3. **HOST-17 ↔ MAT-15**：同一事实两面（persist_turn 静默 no-op ↔ §3.2 缺 9 依赖 + NOTE 过期），合并为一条处置。
4. **HOST-06 ↔ MAT-18**：不矛盾——kept crates 有维护性修改（Windows 适配/clippy，记录齐备），但 permission 决策点本身零修改；v0.2 §6「keept crate 未修改」表述需修正为「决策点未修改、有维护性修改」。
5. **LOOP-14 独立复核**：我（主会话）先于代理发现 Python 侧无 `tool_rounds` 痕迹；LOOP 代理确认 Python 实为 max_turns=20/max_tool_calls=0——双向一致，P7 记录失实为确认结论。
6. **TUI-16 微漂移**：Slice #12 记录「文件 从面板串列表移除」vs 当前 widgets.rs:271 仍含——测试 payload 不含 edit_file_paths 故未触发，属潜在测试脆弱性（P3）。

---

## 6. 状态一览

| 类别 | 数量 | 明细 |
|---|---|---|
| P1 | 2 | LOOP-16（transport 超时+无限重试）、PY-01（GAK-01 真实探针执行缺口） |
| P2 | 13 | LOOP-03、LOOP-14、HOST-03、HOST-06、HOST-17/MAT-15、MAT-01/02/05、MAT-06、MAT-07、ASR-06、PY-02、PY-04、PY-06、PY-08 |
| P3 记录类 | ~30 | 见各面清单 |
| 与设计一致 | ~35 | LOOP-02/05/06/07/08/10/11/12/13/15、HOST-01/04/07/08/10/11/12/13/15/16、TUI-14~16/18~30、ASR-03/04/05/08/09/10、MAT-08/10/13、PY-09/10/11/12/13 |

**核心结论**：设计-实现总体一致率高的面（journal/权限/快照/permit/问询机制/双 TUI/交叉验证）均有测试锁定；问题集中在「设计文档未随实现演进」（~15 处过时点）、「实现未达设计承诺」（桥接/envelope/IP6 解释）、「跑分暴露的设计空白」（IP2a 映射、MAX_TOOL_ROUNDS、transport 超时）。P3/P7 修复必须先定设计条款；P8 有明确设计依据可直接修。

## 入口

- 跑分问题: `docs/POLYGLOT_BENCHMARK_FINDINGS_2026-08-06.md`
- 本审计: `docs/DESIGN_IMPLEMENTATION_DEVIATION_AUDIT_2026-08-06.md`
- 当时设计基线: `存档/architecture/pre-adr-0010/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` / 同目录 v0.1；当前权威已转为 ADR-0010
- 前次审计: `architecture/DESIGN_TO_IMPLEMENTATION_GAP_AUDIT_2026-08-02.md`（GAK-01~09）
- 实施记录: `CLI_PROJECT_INDEX.md`（Slice #1-17）/ `docs/CODEX_FALLBACK_TUI_SLICE_12_2026-08-06.md` / `docs/CODEX_FALLBACK_TUI_SLICE_16_2026-08-06.md` / `docs/CONFORMANCE_SUITE_SLICE_17_2026-08-06.md`
