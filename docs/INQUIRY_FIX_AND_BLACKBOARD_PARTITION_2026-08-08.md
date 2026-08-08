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
| **极简状态行**（常驻 system prompt） | 当前 plan 步骤 + 编辑计数（2-3 行） | 最小 |
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
   - 压缩后目标：**~100K**（间隔 ≥20 轮，前缀缓存账才划算——每次压缩 = 缓存一次大 miss ~200K tokens）
   - 压缩范围：**过程性内容**（工具结果全文 → 摘要）；**精确段保留**（编辑动作区、plan、最近 K 轮消息原文）——压缩不引入不确定性
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

## 7. 关联

- 评测上下文：`docs/TERMINAL_BENCH_2_EVAL_2026-08-08.md`（TB 适配 + 成绩）
- 设计文档：`architecture/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.5（黑板）/§4.6（inquiry）
- 阈值：`adr/ADR-0005-neutral-inquiry-thresholds-finalized.md`
- 工具轮上限：`adr/ADR-0008`（ORZ_MAX_TOOL_ROUNDS 默认 40）
- memory `fusion-phase-tracking.md`（2026-08-08 条目）
