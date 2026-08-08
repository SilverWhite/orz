# inquiry 触发修复 + 黑板分区设计补充（2026-08-08）

**设计补充 + 待办交接文档**——对 `architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.5/§4.6 的修订方向（v0.2 冻结文档暂不改，实施时随实施文档修订）。**新窗口按此继续**。

## 1. 背景：Terminal-Bench 长链诊断（实证）

dna-assembly 82 模型轮 journal 分析：**61 次 neutral_inquiry（74% 轮次）**、59/61 触发原因 = `output_repeats`。根因链：
- controller L642 原实现把**整个 messages 会话历史**做重复度测量（"measure over the conversation"）
- 长会话结构性重复（工具结果回放、固定短语）→ ngram 重复随会话增长
- `feed_output_repeats` 用 **max 累计**（历史峰值不清零）→ 触发清零后下轮对整个会话重测立即又超阈值 → **每轮连发，隐式冷却失效**

同时确认：黑板 5 区结构存在但**半死**——只有 gate_log 被写（orientation check id），plan/exec 区从未写、工具结果不进黑板（journal tool_completed exit_code null）、**system prompt 不渲染黑板（模型不可见）**；跨轮信息全靠全量会话回放（长链上下文线性膨胀）。

## 2. 已完成修复（orz，未提交）

**output_repeats 单轮测量**（controller.rs L638-663 重构）：
- 测量范围 = **仅当前轮 response.text**（原为整个会话历史）
- 会话级 stagnation 由 `runtime_stagnation_guard` 独立承担；inquiry 的 output 判定点只管单轮输出重复
- **判定点保留**——它针对 DeepSeek 输出重复崩溃防护（[DeepSeek-V3 issue 1550](https://github.com/deepseek-ai/DeepSeek-V3/issues/1550)；v4 社区有同类报告），只修测量语义不删判定点
- 回归测试 `long_silent_tool_loop_does_not_fire_inquiry_every_round`（20 轮静默工具循环 → inquiry ≤3；修复前 ~20 次）
- 验证：orz-loop 99 passed 全绿（含新测试）

## 3. 设计补充 A：inquiry 判定点语义确认

- `output_repeats` 判定点是**硬门控**，唯一针对 deepseek 输出重复缺陷——保留
- 测量 = 单轮输出（已修）；触发即全清零（隐式冷却）——修复后冷却语义恢复
- 阈值不变（output>10 严格大于，ADR-0005）

## 4. 设计补充 B：黑板分区修订（v0.2 §4.5 激活 + 修订）

**总原则**：框架唯一目的是让模型负担尽可能小；黑板从"半死结构"激活为模型的状态视图。**任何非确定性记录区不引入**（随记区否决——难控制 + 重复文本注入会冲淡重点、触发 output_repeats 判定）。

### 4.1 三区结构（单一主 agent 下）

| 区 | 写入方 | 内容 | 确定性 |
|---|---|---|---|
| **plan 区** | 控制器 | plan mode 状态机映射：goal、步骤 + 状态（pending/in-progress/completed）、当前步骤 | ✅ |
| **编辑动作区**（替代随记区） | 控制器（编辑类工具成功后） | `{文件} {行范围} 变动` + 时间戳，如 `1.py 200-379行变动`；行范围从 old_str/new_str 换行计数计算 | ✅ |
| **工具动作区** | 控制器（每工具轮） | 全部动作按类折叠（read/编辑/terminal/检索）+ **时间戳**，按时间戳取用/回看 | ✅ |

- 工具动作区**时间戳**：事件层已有纳秒时间戳（journal 全事件带 timestamp），天然排序
- **分类折叠**：按类聚合，折叠展开，每条带时间戳
- 编辑动作区写入时机 = 编辑工具（search_replace/write 类）**成功后**——"实际变动"才记

### 4.2 黑板外化三层（注意力稀释最小化）

| 层 | 机制 | 注意力成本 |
|---|---|---|
| **极简状态行**（plan mode 下常驻 system prompt） | 当前 plan 步骤（目标/步数/已完成/当前步/待办，2-3 行）。**编辑计数不入常驻行**——2026-08-08 实施裁决（ADR-0008 §2.3 缓存纪律）：每编辑轮变化会重造 17.7%→98% 前缀缓存回归；本轮增量由增量推送携带、累计经 blackboard_read 可取 | 最小 |
| **增量推送**（机械连接） | 每工具轮后，**本轮的编辑动作/工具结果摘要**作为工具结果消息的一部分回放（紧跟工具结果、语义连贯）——模型看到"本轮发生了什么"，非全量黑板 | 低（只推新增） |
| **按需取用** | `blackboard_read` 工具（分区 + 时间范围参数，orz-host 工具集 +1）——模型需要回看历史时主动调用 | 零（不调不注入） |

- **机械连接语义**：增量推送是**确定性触发**（工具轮后自动附加，不经模型决策）——对应融合架构 MechanicalRelay 的既有概念
- 全量黑板渲染（每轮 system prompt）**不做**——模型不调 blackboard_read 时零稀释

## 5. 待办清单（实施顺序——2026-08-08 定稿）

**分层实现（用户批准 A→B→C）**：
- **A 阶段（先做）**：记录补齐 + blackboard_read + 显式元认知提示 + **150K 显式压缩**
- **B 阶段（A 达标后）**：压缩常态化（A 的显式压缩升级为唯一推送）
- **C 阶段（暂不做）**：仅极简状态 + 全按需

### A 阶段实施项

1. **编辑动作区 + 工具动作区落地**：工具成功写编辑记录（文件/行范围/时间戳）→ journal 事件扩展（tool_completed payload 加 edit 记录字段）——**run-event schema Python authority 先行**（33 事件注册表 + fixtures + conformance 交叉验证，跨仓库同步面大）
2. **增量推送**：工具轮结果回放附编辑摘要（消息层，零 schema 改动）——建议先做（无 schema 风险、收益直接）
3. **blackboard_read 按需工具**（orz-host 工具集 +1，分区 + 时间范围参数）
4. **极简状态行**（prompt 组装，plan mode 映射）
5. **记录保留策略**：.gsa 记录树**默认保留 7 天**（可配置）——debug 价值独立成立，不依赖压缩设计
6. **150K 显式压缩**（2026-08-08 定稿参数）：
   - 触发：实际 prompt token 数 > **150K**（实测 Polyglot prompt 均值 238K、窗口 ≥300K——150K = 窗口 ~50%，平均任务触发 1-2 次、TB 长任务 2-3 次，频率可控）
   - **窗口余量安全触发**（2026-08-08 审查 D1-1 补）：实测 > **250K**（默认 safety_tokens）**无视 20 轮冷却**强制压缩——冷却同样推迟首次压缩，高起点任务可能在冷却届满前触窗；多一次缓存 miss 远小于窗口溢出 run 失败。参数化（`ContextCompactConfig.safety_tokens`）
   - 压缩后目标：**~100K**（间隔 ≥20 轮，前缀缓存账才划算——每次压缩 = 缓存一次大 miss ~200K tokens）
   - 压缩范围：**过程性内容**（工具结果全文 → 摘要）；**精确段保留**（编辑动作区、plan、最近 K 轮消息原文）——压缩不引入不确定性。**"摘要"= 确定性整轮丢弃 + marker 机械摘要行**（2026-08-08 审查裁决：LLM 摘要非确定，违反同条"不引入不确定性"；marker 附一行黑板累计——编辑处数 + 工具分类计数，零模型调用）
   - 显式注释：压缩段打标 **"前文上下文已压缩"**（历史替换非每轮注入，不触发 output_repeats）——同时是回查提示（模型知道前文不可靠 → blackboard_read）
   - 观测：压缩事件记入 journal（新增事件类型或 model 轮观测字段——实施时定，Python authority 先行）

### 判定点/设计约束确认（沿用）

- `output_repeats` 判定点保留（DeepSeek 输出重复防护，issue 1550）——单轮测量（已修）
- 非确定性记录区不引入（随记区否决——重复注入冲淡重点）
- 压缩触发显式化：模型必须知道"前文已压缩"（隐式逼迫不可靠——模型无元认知，不会意识到记忆模糊）

## 6. 当前 orz 未提交改动（累计 6 处）

| 文件 | 改动 |
|---|---|
| `orz-loop/src/controller.rs` | ① ORZ_MAX_TOOL_ROUNDS env 覆盖（SWE-bench）；② **output_repeats 单轮测量修复 + 回归测试**（本次） |
| `orz-loop/src/gateway/credentials.rs` | Linux env 凭据通道（ADR-0006 扩展） |
| `orz-host/src/session.rs` | keystore 非 Windows 降级 |
| `orz-config/src/managed_text/source.rs` | Linux cfg 参数名修复 |
| `xai-fast-worktree/src/api.rs` | Linux cfg 修复 |

## 7. A4/A5/A6 实施完成记录（2026-08-08 同日，未提交）

**A4 极简状态行（plan mode 映射）** — ① `AgentLoopController::with_plan(goal, steps)` 把已批准 plan artifact 映射进黑板 plan 区（goal=prompt、steps=plan sections，首步 InProgress 其余 Pending；步骤推进留待 model-generated plan 时代）；② `prompt::build_status_line` 渲染 `[任务状态 v0.1]` 块（目标/步数/已完成/当前步/待办，2-3 行）注入 system prompt（仅 plan 存在时，非 plan 运行零注入）；③ **编辑计数刻意不入常驻行**——每编辑轮变化会重造 2026-08-07 的 17.7%→98% 前缀缓存回归；本轮增量已由 `[本轮编辑]` 推送、总量经 blackboard_read 可取（实现裁决，已注释）；④ orz-bin `--plan` 路径 run_plan_phase 返回 artifact → with_plan 接线。测试 +5（渲染/状态跟踪/注入/无 plan 零注入/**跨轮字节稳定**——缓存纪律锁定）。

**A5 记录保留策略** — 新 `orz-host/src/retention.rs`：`.gsa` 默认 7 天（`ORZ_RETENTION_DAYS` 可配，0=禁用）；`runs/`（RUN-/RST- 目录，按 events.jsonl mtime 判龄，当前 run 按名保护）、`snapshots/`（manifest 按龄删 + **对象按引用 GC**——内容寻址对象跨快照共享，只删无任何剩余 manifest 引用**且老于 cutoff** 的对象）、`one_shot_permit/` 按龄删；**keystore 永不清理**（删安装密钥 = 静默作废全部 permit 签名）。bootstrap_session 统一入口执行，best-effort 不阻断。测试 +8（含 filetime 真实回拨 mtime——workspace 既有 dev 依赖）。

**A6 150K 显式压缩** — ① `ModelResponse.prompt_tokens`（transport 双路径提取 usage.prompt_tokens；fake 可脚本化）——**触发用真实测量值**而非估算；② 控制器 `ContextCompactConfig{150K/100K/20轮/250K 安全触发}` 可注入；③ `compact_messages`：按轮边界（assistant 带 tool_calls 的声明消息为轮起点）**整轮丢弃**旧轮——声明+工具回复+注入推送同生共死，幸存 tool_call_id 恒有配对声明（协议 D2-1 纪律）；preamble（原始 prompt）+ 最新 K 轮原文保留；估算 tokens=chars/2（CJK 保守向）；④ 压缩点插 `[前文上下文已压缩 v0.1]` 标记（已注册 is_injected_block_text，不进停滞守卫；显式告知=设计 §5「模型无元认知」裁决；**附一行黑板累计机械摘要**——编辑处数+工具分类计数，零模型调用）；⑤ **journal 新事件 `context_compressed`**（Python authority 先行：run-event enum 33→34 + 新 payload schema + 注册表 + fixtures 重生成 + conformance 7 passed + check_repository valid；Rust EventType + TUI bridge/projection 同步）。**§8 C.1/C.2 细化（同日用户裁决后实施，见 §8 C.4）**：参数改为 160K/90K；节奏压缩仅限最终答案间隙一次 + 250K 保命重置计数；压缩白名单工具（首工具批次写入、常驻 preamble 隔断压缩、16K 上限、明文存档 .gsa）。测试：orz-loop 107→**121**（compact 纯函数 2 + 触发/冷却/安全触发/rhythm gap 4 + whitelist 3 + A4 5 + transport/prompt 断言增补）、orz-host 81→**90**（retention 8 + session 1）、tui 174/codex 33 全绿、clippy 全目标零新增。

**审查闭环（三独立代理，2026-08-08）**：实现——**无 P1**；P2×1 修（retention_cutoff 病态 env 溢出 panic → checked_sub 钳制到 epoch + 回归测试）+ P3×2 修（损坏 kept manifest → 跳过对象 GC fail-safe + 测试；schema messages_kept 描述含 marker）+ P3×4 记录。符合性——**无 D1**（三条设计意图核心全部忠实落地；A4 编辑计数排除 = D2 合理改编——ADR-0008 §2.3 缓存纪律强于 §4.2 文字，已改 §4.2 补注；A6 摘要语义 = D2 可辩护偏离——整轮丢弃比 LLM 摘要更忠实「不引入不确定性」，已改 §5 补记；§7 数字失实修正：orz-host 实为 81→**90**）；D3 记录（状态行 4 行 vs 「2-3 行」措辞、keystore/session/state.json 范围边界、chars/2 校准注记、marker 措辞略宽、跨 turn 计数器迁移前瞻、plan 分区「状态机只实现初态」推迟注记）。设计合理性——**D1×1 修**（冷却同时推迟首次压缩 + 高起点任务可触窗 → safety_tokens 250K 无视冷却 + 测试）+ **D2×2 修**（对象 GC 无年龄门 + manifest 非原子写 = 并发会话竞态毁进行中快照 → 年龄门 + 解析失败 tracing::warn + 测试；marker 回查邀请 vs exec 无上限渲染 = 回查反噬压缩成果 → marker 机械摘要行 + exec 渲染上限最近 50 条含省略注记）+ **D2×1 记录**（A4 推进机制接入点前瞻：拆静态/动态，动态走尾部消息——今日设计成立）+ D3×12 记录（编辑计数剔除最优、per-turn 计数与消息生命周期一致无削弱、chars/2 方向正确、连续压缩 marker 累计语义无害、事件 7 计数可审计缓存账等）。

验证汇总：orz-assurance 95 / orz-loop 117 / orz-host 90 / orz-tui 174 / orz-codex 33 / orz-bin 4 / conformance 7 / runtime 114 / check_repository valid。**下一步**：TB medium 池补样本 → TB 全量 89 → SWE-bench 剩余 14 题。

## 8. 设计补充 C：压缩时机动作判定 + 压缩白名单（2026-08-08 用户决策，未实施）

**原则（用户明确）**：压缩**不引入不确定性**——机械压缩（确定性整轮丢弃 + marker 机械摘要行），非模型压缩（LLM 摘要否决：失真、不可恢复、有调用成本）。本节是 A6 的两项细化设计，**待用户确认后实施**。

### C.1 压缩时机：动作判定（定稿——用户 2026-08-08 决策）

**需求**：达到压缩阈值后先做**动作判定**——模型完成本轮全部动作、输出最后内容后再压缩；**不做任何中途压缩打断模型动作**。核心诉求 = **维持单轮次任务的流畅和稳定性**。

**定稿语义**（比 C.1 初稿更激进——用户最终裁决）：

| 机制 | 参数 | 触发条件 | 时机 |
|---|---|---|---|
| **节奏压缩**（最终答案前一次） | 触发 **160K** / 目标 **90K** / 冷却 ≥20 轮 | 上下文 ≥160K 且距上次压缩 ≥20 轮 | **仅在"最后的批次间动作处"**：候选答案轮（模型输出无工具调用的一轮 = 最后工具批次已完成）之后的 loop-top——即 gate 注入后、gate 回答（最终答案）前；机会恰好一次（gate 只注入一次） |
| **保命压缩**（窗口守卫） | **250K**，无视冷却 | 实测 >250K | 任何 loop-top（天然在批次间，不打断动作）；**压缩后重置压缩计数**（rounds_since_compact=0——后续节奏判定正常进行） |

- **阈值 160K**（用户指示，原设计 150K——数值已改）；**目标 90K**（用户指示"保留后 90k 上下文，降低回查压力"，原 100K——最终答案轮上下文更小，反例自查回查压力低）
- 工具循环期（模型动作中）：**不压缩**（保命线除外）——流畅性优先
- 工具批次执行中永不压缩（loop-top 天然保证批次完整性）
- gate 后模型继续工具轮的边角（罕见）：后续最终答案轮 break 前无节奏压缩点——保命线兜底
- 无工具调用的运行（纯文本问答）永不压缩 ✓（无后续请求受益）
- 语义结果：长任务 = 一次节奏压缩（最终答案前）+ 最多若干次保命压缩（若任务极长涨过 250K）
- **账目诚实注记（设计审查 2026-08-08）**：节奏压缩是 **token 负项**——miss 实为压缩后保留段（~85K，被丢 token 不进 miss），每次节奏压缩净 +50-60K 单位；它是**纯质量/稳定性决策**（买最终轮注意力 + gate 自查稳定性 + 对 gate 后续跑边角的预防性窗口保护），非成本优化，勿误读
- **冷却语义注记**：冷却在节奏路径的实际作用 = 挡短任务（<20 轮到终点的最终轮不压）与保命后 20 轮内到终点——保命后上下文 ~90K+增长大概率 <160K 阈值本身不触发，冷却无关；勿以"死参数"误删
- **边角注记**：早产候选（模型早期轮即无工具触发 gate、当时上下文小不压）消耗节奏机会——gate 后续跑的长任务最终间隙（无工具轮直接 break）无节奏点，仅保命线兜底

### C.2 压缩白名单（定稿——用户 2026-08-08 决策）

**需求**：任务背景信息和必须获取的信息在压缩时保留——压缩应尽可能避开这一部分；**压缩白名单**的写入**仅限于单一对话中的模型首轮动作**；模型被明确告知可将客观任务的所需信息写入白名单。**白名单常驻上下文、被压缩机制跳过**（用户裁决：不采用"压缩时再注入"——注入会强化印象、固化背景理解，不利于模型后续变动；背景内容并非绝对一成不变）。

**设计**：

1. **工具形态**：专用工具 `compaction_whitelist_add(content: string)`（orz-host 工具集 +1，与 blackboard_read 同模式：控制器特判、不落 host 分派）
2. **可用性**：窗口 = **首个工具批次 + 首轮**（用户裁决——覆盖"首轮纯文本、次轮才用工具了解项目"的边角；判定：模型响应轮次 ≤1 或 `tool_rounds==0` 执行中的批次）；窗口关闭后调用返回明确错误「白名单仅限首轮动作写入」
3. **权限分类**：`RiskClass::ReadOnly`——纯内存写、无外部副作用（与 blackboard_read 同类先例）；所有策略（Interactive/ReadOnly/Benchmark）下声明且自动放行；`modifies_files` false（不触发快照）；工具动作区分类 "other"
4. **内容上限**：累计 **16K 字符**（用户裁决——大项目背景够用；≈8K tokens ≈ 压缩后目标 90K 的 ~9%；可配置）；单条 content 为空拒绝
5. **常驻机制**（用户裁决——"直接隔断压缩机制"）：白名单消息在**写入时**插入 messages 的 **preamble 区**（原始 prompt 之后、首轮声明之前）——压缩机制的 preamble 恒保留规则**自动跳过它**（零特殊压缩逻辑）；写入后每轮请求都携带（位于缓存前缀内，hit 价 ≈ miss 的 1/10）；压缩时刻无"新鲜注入"——不强化印象
6. **写入窗口内多次写入**：追加条目（换行分隔）；窗口关闭后拒绝
7. **明文存档**（用户裁决）：写入时**机械追加** `.gsa/runs/{run_id}/whitelist.jsonl`（JSONL：时间戳+内容，明文）——**run 目录下，A5 按 run 整删自动覆盖 7 天保留（A5 零改动）**；运行时压缩从内存读（无 IO 依赖），存档 best-effort 不阻断（失败不阻断写入）
8. **模型告知**：工具 description 明示「仅首轮可用、写入内容不被压缩、存档于 .gsa 记录树」+ `BASE_SYSTEM_PROMPT` 加一行静态告知（缓存安全）
9. **语义**：「客观任务的所需信息」= 任务背景、关键路径/约束、验收标准等客观事实——主观决策留给模型；**不可更新**（用户裁决——写入仅限首轮；背景演化靠模型自己的理解，常驻呈现不强化）

**白名单与既有精确段的边界**：preamble（原始 prompt）+ plan 区（黑板）+ 编辑动作区（黑板）是框架侧精确段；白名单是**模型驱动的精确段**——互补关系。上下文结构 = 【原始 prompt + 白名单】＋【压缩段】＋【当前轮】。

### C.3 激进方向（记录，等数据后裁决）

- 候选：**工具结果头部保留**（对所有轮 Tool 消息只留头部 300-500 字符 + 「…全文见 blackboard_read」指针——协议安全：只改 content 不动 tool_call_id 配对；确定性框架内最自然的激进升级）
- 候选：**中期节奏压缩**（2026-08-08 用户裁决纳入数据闸门）——每 20-30 轮在工具间隙压缩一次（160K/min_rounds=20/loop-top/零新机制——纯配置）；设计审查账目：82 轮任务净省 ~15-20% tokens；质量风险 = 模型中途丢失工作状态（无元认知，marker+摘要+blackboard_read 只能缓解）——被「流畅性优先」裁决挡在门外，但账上为正，列入 TB 全量 89 后数据闸门评估（真实缓存账 + 任务完成率对比后裁决）
- 前提：**TB 全量 89 跑完拿真实缓存账**（journal 已记每轮 prompt_tokens + cache_hit/miss_tokens——压缩一次的实际 miss 成本 vs 省下的轮均增长）后再裁决是否实施
- 不激进的方向确认：触发降低（缓存账亏）、目标降低（下界=preamble）、冷却缩短（账目微妙）、preamble 压缩（任务定义丢失）

### C.4 实施状态（2026-08-08 已实施，未提交；审查闭环后数字修正）

1. **C.1 已实施**：`ContextCompactConfig{160K/90K/20轮/250K 保命}`——节奏压缩仅限最终答案间隙（`!last_round_had_tools && counterexample_fired` = 候选答案轮后、gate 回答前，恰好一次）；保命压缩（>250K）任何 loop-top 生效、重置 rounds_since_compact；工具轮后的间隙永不节奏压缩（动作流畅性）
2. **C.2 已实施**：`compaction_whitelist_add(content)`——窗口 = 首个工具批次（tool_rounds==0，覆盖"首轮纯文本→次轮了解项目"；与设计文字"响应轮 ≤1 或首个批次"结构等价——首个工具批次必然出现在响应轮 0 或 1，符合性 D2-1 核对通过）；ReadOnly 类（全策略声明+放行——**双层实现**：控制器 risk_class 管声明 + host 桥 access_kind 特判 `Read(None)` 管实际放行，审查 P1-1 修复，blackboard_read 同修）；上限 16K 字符（`with_whitelist_cap` 可配）；写入即插 preamble 区消息（压缩机制自动跳过=常驻隔断，零特殊逻辑）；机械追加存档 `{journal_dir}/whitelist.jsonl`（JSONL 明文，A5 按 run 整删覆盖 7 天，A5 零改动）；BASE_SYSTEM_PROMPT 静态告知（含负面排除：不写计划/步骤/推测/临时状态——计划由 plan mode 承载）；工具 description 含"先读后写"引导；工具动作区折叠 "other"；is_injected_block_text 注册
3. **C.3 仅记录**（中期节奏压缩候选已列入数据闸门评估，见下）

**验证**：orz-loop 117→**122**（新增测试函数 +14：A6 压缩 6 + 白名单 4 + A4 状态行 3 + prompt 2——符合性 D3-1 修正：此前 "+4" 表述失实）、host 81→**91**（retention 8 + session 1 + P1-1 桥测试 1）、assurance 95/tui 174/codex 33/bin 3 全绿、clippy 全目标零警告（run_host_tool 8 参 too_many_arguments allow 惯例）。IP2a 策略测试断言更新（ReadOnly 类声明集合 +compaction_whitelist_add——设计意图：全策略声明）。

**审查闭环（三代理，2026-08-08）**：实现——**P1×1 修**（access_kind 未特判 → 白名单/blackboard_read 落 Edit else 分支，headless/Benchmark 确定性拒绝 → `Read(None)` + 桥级三策略测试）+ P3×4 记录（跨 turn 窗口重开+session 级存储混合、上限检查锁内原子性理论、archive 同步 fs、估算脆弱性）+ 同批次追加存档覆盖已补。符合性——**无 D1**（10 条用户裁决全部忠实执行）；D2×2 可辩护（窗口判据等价吸收、白名单 User 消息形态合法恒在首 decl 前）；D3×7（C.4 数字失实已修、model.rs 过时注释已修、空 content 测试已补、拒绝语言英文与 description 一致、可配置面 builder-only、连续 User 消息真实 provider 覆盖留意）。设计合理性——**无 D1**；**核心发现：节奏压缩是 token 负项**（miss 实为 ~85K 非 ~160K——被丢 token 不进 miss；每次节奏压缩净 +50-60K 单位，纯质量/稳定性决策：买最终轮注意力 + gate 自查稳定性 + 对 gate 后续跑边角的预防性窗口保护）；中期压缩备选（160K/min_rounds=20/零新机制）账上为正（82 轮任务 ~15-20%）但被"流畅性优先"裁决挡在门外——**已列入 C.3 数据闸门评估候选（待用户裁决）**；白名单凭据样扫描建议（GAK-CRED-001 一致性，待用户裁决）；D3 若干（冷却在节奏路径的实际作用=挡短任务、早产候选消耗节奏机会边角、白名单过时事实永驻但 recency 可矛盾、纯文本运行实为 2 轮）。

## 9. 关联

- 评测上下文：`docs/TERMINAL_BENCH_2_EVAL_2026-08-08.md`（TB 适配 + 成绩）
- 设计文档：`architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.5（黑板）/§4.6（inquiry）
- 阈值：`adr/ADR-0005-neutral-inquiry-thresholds-finalized.md`
- 工具轮上限：`adr/ADR-0008`（ORZ_MAX_TOOL_ROUNDS 默认 40）
- memory `fusion-phase-tracking.md`（2026-08-08 条目）
