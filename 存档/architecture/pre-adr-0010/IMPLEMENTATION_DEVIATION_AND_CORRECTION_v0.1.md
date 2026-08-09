# 实现偏差与修正路线 v0.2

> Archive metadata: original_path=`architecture/IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`; archived_at=`2026-08-09`; final_status=`evidence_only`; superseded_by=`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`; authority=`historical deviation record only`.


状态：2026-08-03 更新。v0.1 记录了 sidecar vs fork 架构偏差；v0.2 补充了中立询问机制的
设计修正（触发模型从线性管道改为事件驱动）和最终阈值。2026-08-03 更新 §4 技术债务——Phase 1
完成、Phase 2 编译阻塞已解除（方案 A）。

## 1. 架构偏差：当前实现是 sidecar，不是 fork

经过二进制签名验证（两者均为 X.AI LLC Authenticode `Valid`）、lock 文件矩阵审核
（`integration_mode: "prebuilt-headless-sidecar"`）和代码路径追踪，确认：

- GSA 使用 X.AI LLC 官方签名的 Grok 二进制（0.2.112 锁定 + 0.2.118 在 PATH）
- GSA 通过 ACP 协议（`grok agent stdio`）启动和控制 Grok 子进程
- GSA 在外部注入 model config、环境变量、Job Object containment
- GSA 在外部记录 journal、生成 receipt
- **Grok 二进制本身未做任何修改**

用户预期的"同人游戏/源码级 fork"模式（在 Grok 源码中直接注入 assurance 层，
编译为 GSA 二进制）与当前 sidecar 模式存在结构性偏差。

> **本文不判断哪种模式更优。** 以下只登记当前实际状态、每条路径的 gate 覆盖情况、
> 设计修正和实现差距。架构方向选择（深化 sidecar vs 转向 fork）待另行决定。

## 2. 各路径 Gate 覆盖矩阵

```
                              IPG    Tool     Orientation  Source    Journal/
                               Gate   Avail.   Checkpoint   Vis.Gate  Verifier
                              ────── ──────── ──────────── ───────── ────────
gsa run --runtime canonical   ✅      ✅       ✅           ✅        ✅
  (offline/fake adapter)
gsa run --runtime canonical   ✅      ✅       ✅           ✅        ✅
  (real DeepSeek)
gsa ask --retrieval off       ❌      ⚠️注1    ❌           ❌        ⚠️注2
gsa ask --retrieval subagent  ❌      ⚠️注1    ❌           ❌        ⚠️注2
  (子代理内部有 IPG + workspace_trust)
gsa chat (交互)               ❌      ❌       ❌           ❌        ⚠️注2
gsa alpha smoke               ❌      ❌       ❌           ❌        ⚠️注2
gsa alpha real-call           ❌      ❌       ❌           ❌        ⚠️注2
grok (直接启动)               ❌      ❌       ❌           ❌        ❌

注1: tool_availability context block 以文本注入 system prompt，未经过机械 probe/gate receipt。
注2: 有基本 receipt/journal，但未经 IPG/orientation/source visibility gate 验证。
```

**只有 `gsa run --runtime canonical` 一条路径跑完整 gate 链**，且该路径被明确标注为
conformance fixture（`canonical_cli.py` 文件头注释："不是 production agent runtime"）。

`gsa ask` 和 `gsa chat` 作为实际面向用户的入口，gate 覆盖严重不足。

## 3. 中立询问机制——设计已修正，实现待重做

详见 [`docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`](../../docs/design-inputs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md)
末尾的"设计修正与补充"章节（2026-08-02 追加）。

### 3.1 核心纠正

| 原理解（错误） | 纠正 |
|---------------|------|
| Orientation checkpoint 是 prompt 前的静态文本注入 | 是**动作间隙的中途引导**，类似旁边轻声提醒 |
| 所有 gate 在模型调用前一次性按顺序跑完 | gate 触发模型应为**事件驱动**，在运行时按条件注入 |
| 不存在信息收集确认机制 | 新增 `INFO_SUFFICIENCY_CHECK`：**检索后 + 正式回复前**触发 |
| 无思维链保护 | **只在动作间隙注入**，绝不打断 in-progress 思考 |
| 无跨轮防护 | 新增**轮次触发**：每 8 轮强制检查一次 |

### 3.2 最终阈值

两条防线互补：

| 防线 | 条件 | 受 cooldown |
|------|------|-------------|
| **轮次触发** | 每 8 轮对话后必定触发 | ❌ 不受 |
| **单轮 token** | 公开输出 ≥ 7000 token | ✅ 受 |
| **单轮动作数** | 累计 ≥ 10 actions | ✅ 受 |
| **单轮工具种类** | 不同 tool_id ≥ 7 种 | ✅ 受 |
| **检索后确认** | 任何检索动作完成后 | ❌ 不受 |

轮次触发解放了单轮阈值——单轮阈值按大型任务体量设计，不需要因"尽早发现"而压低。

### 3.3 信息收集确认（新增）

检索/子代理返回后、正式回复前注入：

```text
[INFO_SUFFICIENCY_CHECK v0.1]
已获取来源: N 项
覆盖范围: {categories}
全文可见: M/N
当前信息是否足够回答用户问题？如不足，还需哪些信息？
[/INFO_SUFFICIENCY_CHECK]
```

约束：只陈述机械可验证的事实（来源数量、类别、全文可见性），不暗示质疑当前结论。

### 3.4 实现差距

`build_orientation_checkpoint` 和 `verify_orientation_response` 的**格式和验证逻辑正确，可复用**。
需要重写的是触发和注入机制：

| 组件 | 设计 | 实现 |
|------|------|------|
| Checkpoint 格式 + 验证 | ✅ | ✅ 已实现 |
| Stagnation guard 检测逻辑 | ✅ | ✅ 已实现 |
| 动作间隙事件驱动触发 | ✅ 已设计 | ❌ 未实现 |
| 硬门控（token/action/tool 计数器） | ✅ 已设计 | ❌ 未实现 |
| 思维链保护（只在动作边界注入） | ✅ 已设计 | ❌ 未实现 |
| 信息收集确认（检索后触发） | ✅ 已设计 | ❌ 未实现 |
| 轮次触发（每 8 轮强制） | ✅ 已设计 | ❌ 未实现 |

## 4. 当前遗留的技术债务

1. **Architecture**: sidecar → fork 迁移 ✅ Phase 1 完成（scaffold + build），Phase 2 编译阻塞已解除（方案 A, 2026-08-03）
2. **Phase 2 Journal + Transport**: 待实现 — orz-shell 已可编译，下一步实现 journal 系统和 thinking:disabled 注入
3. **Orientation 触发层**: checkpoint 格式和验证正确，触发/注入需重写为事件驱动
4. **Gate 覆盖**: `gsa ask` 和 `gsa chat` 两条用户入口只跑部分 gate（fork 后在 Phase 3 解决）
5. **Thinking proxy daemon**: sidecar 模式下的外部 workaround。fork 后 Phase 2 在 HTTP 传输层源码解决
6. **命名歧义**: `GrokAcpSession`（实际是 GSA 的 ACP 会话管理器）、`grok_runtime_adapter`（实际是 GSA runtime controller）

## 5. 架构决策：转向 fork 模式

**2026-08-02 已决策。** 不再深化 sidecar。以 Grok 源码为模板，将 GSA assurance 层注入到源码中，
编译为 GSA 二进制。详见
[`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.1.md)。

迁移分 4 个 Phase，Python 项目保留为 reference spec + conformance test suite。
