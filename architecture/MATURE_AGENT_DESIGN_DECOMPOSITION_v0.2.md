# 成熟 Agent 设计深拆与职责收缩 v0.2

状态：架构审计与 implementation spike 重排；**不修改现有协议语义**，不接入真实模型，不迁入旧研究
`index/map/self-check`，不恢复云端范围。产品与术语定位见
[`PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`](PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md)。

## 1. 结论先行

当前仓库看起来“额外内容很多”，主要因为此前把一个高保证 Agent 拆成了可分别验证的事实：进程边界、权限、
事件、恢复、上下文、证据和评测。这个拆分本身有价值，但不表示每一层都要由本项目重新实现。

成熟项目的共同经验更接近三层所有权：

1. **Grok runtime**：模型循环、session、tool 执行、permission、sandbox、compaction、后台任务和结构化实时协议；
2. **恢复与审计 plumbing**：shadow Git、append-only receipt、独立 verifier；只采用成熟的不变量，不复制产品；
3. **LIF 专项科学保障层**：workspace trust、来源/证据/claim 门禁、脱敏、科学 validator、DeepSeek
   conformance、Windows 进程监督和评测隔离。它约束 Agent 如何服务 LIF 研究，不表示 LIF/FEP 理论参与
   Agent 控制算法。

因此 v0.2 的方向是收缩，不是继续堆功能。尤其是 Grok 已正式提供 ACP；生产集成不应长期围绕一次性
headless stdout 重建 tool loop。

## 2. 本轮新增的上游事实

### 2.1 Grok 已经拥有持续运行的结构化 Agent 协议

锁定 Grok Build revision `a881e6703f46b01d8c7d4a5437683546df30449d` 的
[`15-agent-mode.md`](https://github.com/xai-org/grok-build/blob/a881e6703f46b01d8c7d4a5437683546df30449d/crates/codegen/xai-grok-pager/docs/user-guide/15-agent-mode.md)
说明 `grok agent stdio` 是持久 JSON-RPC/ACP server，包含 `initialize`、`session/new`、`session/prompt`、
`session/update` 和交互 permission。`session/update` 明确区分 `tool_call` 与 `tool_call_update`。本地锁定 binary
随附文件 SHA-256 为 `9ec66e8510b30e31efe3eb8f0bce4638a8e58840ff57a8c34c864aa052631e3a`。

这直接改变 event bridge 的定位：

- ACP 是待验证的**实时 canonical source 候选**；
- session `events.jsonl`/`updates.jsonl`、trace/export 是 post-run cross-check；
- 当前 metadata bridge 是独立 digest/verifier 与 headless fallback，不应升级为平行 runtime protocol；
- 时间邻近仍不能用来补全跨来源因果关系。

### 2.2 Grok 已拥有后台任务与监控

同 revision 的
[`20-background-tasks.md`](https://github.com/xai-org/grok-build/blob/a881e6703f46b01d8c7d4a5437683546df30449d/crates/codegen/xai-grok-pager/docs/user-guide/20-background-tasks.md)
覆盖 background command、task ID、output poll/wait、kill、monitor、loop 和 scheduler。本地文件 SHA-256 为
`9d2c094e0e2f59b9b98c743efcbde2a4cd0fa14b2063d9911fe28f3ea42ea11b`。

所以本项目不建设通用后台任务管理器、调度器或云端队列。LIF 专项科学保障层中的 Windows supervisor 只验证
进程树收束、取消和 terminal receipt 是否与 Grok 的任务状态一致。

### 2.3 成熟设计强调“边界分离”，不是“大一统内核”

- [Codex app-server](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md) 将 thread、turn、item、
  notification、approval request 和 cancellation 分开；可借用 typed identity 与唯一 terminal 不变量，不复制服务端。
- [Gemini CLI checkpointing](https://github.com/google-gemini/gemini-cli/blob/acae7124bdd849e554eaa5e090199a0cf08cd782/docs/cli/checkpointing.md)
  在写入前使用独立 shadow Git，并把文件恢复与 conversation/tool-call 恢复关联。
- [OpenCode snapshot/revert](https://github.com/anomalyco/opencode/blob/cb562b2c6289c2eee707078f9ab644cbe1d3d8a9/packages/opencode/src/snapshot/index.ts)
  同样把独立 Git store、patch/restore 和 session/message 边界联系起来。
- [Gemini CLI policy engine](https://github.com/google-gemini/gemini-cli/blob/acae7124bdd849e554eaa5e090199a0cf08cd782/docs/reference/policy-engine.md)
  展示 allow/deny/ask、优先级和匹配解释；Grok 已有相应 permission owner，本项目只保留不可委托的 LIF hard gate。
- [Goose security inspector](https://github.com/block/goose/blob/65e1e3d508b9f85c53998564db04708c90031881/crates/goose/src/security/security_inspector.rs)
  提醒 security finding 与 authorization decision 必须分开。
- [Cline checkpoints](https://docs.cline.bot/core-workflows/checkpoints) 也使用与用户 Git history 分离的 shadow
  repository，并允许分别恢复 files、task 或两者。它证明 recovery UX 的可行性，不证明自动授权是安全的。

这些项目并未给出“恢复存在，所以危险操作可以自动批准”的充分推论。文件恢复无法撤销 secret 泄漏、网络写入、
远端删除或其他外部副作用。

## 3. 所有权矩阵

| 能力 | 正式 owner | LIF 专项科学保障层只做什么 | 不再做什么 |
|---|---|---|---|
| model/tool loop | Grok | DeepSeek 请求形状与两轮 continuity conformance | 自建生产 model adapter loop/action kernel |
| live session/event | Grok ACP | 记录能力协商、ID、digest、terminal 与缺失项；post-run 交叉核验 | 从 headless 文本推断 tool loop；建立第二套 session protocol |
| tool permission | Grok | 记录匹配结果；在外层施加 workspace trust、secret/network、claim promotion hard gate | 通用 policy engine、长期通配 grant DB |
| sandbox/tool runtime | Grok | Windows Job Object、防火墙与实际副作用 receipt | 复制 shell/file/search/browser runtime |
| background work | Grok | 验证 task ID、cancel、process-tree terminal 对齐 | scheduler、monitor service、remote queue |
| compaction | Grok | 记录输入 span/digest、boundary、summary=`derived_unverified` | 自建 compactor；从 summary 反推未记录事实 |
| audit receipt | LIF 专项科学保障层 | hash chain、redaction、source/completeness、独立 verifier | 把 receipt 冒充模型正确性证明 |
| file recovery | 独立 shadow Git plumbing | 把 commit ID、before/after aggregate、conflict plan 写入 receipt | 自建 blob format；自动改写用户 Git history |
| evidence/claim/evaluation | LIF 专项科学保障层 | SourceRouter、EvidenceKernel、ValidatorBridge、LeakScanner、EvaluationRunner | 委托给 Grok permission 或 prompt 自律 |
| cloud/mobile relay | deferred | 仅保留 ADR 中的未来重启条件 | 当前建设和持续运维 |

## 4. 必须保持的五个分离

### 4.1 审计 checkpoint 与恢复 checkpoint

现有 fixture checkpoint/delta 是 hash-only **audit receipt**：能证明两个观察时刻的差异，不能恢复。下一阶段若
shadow Git fixture 通过，只需在 audit receipt 引用独立 store 的 commit/tree ID；二者不能用同一个模糊的
“checkpoint 成功”状态代替。

### 4.2 恢复能力与操作授权

restore 是新的危险 action，必须固定目标 checkpoint 与 expected-current digest，检测人类/外部写入，冲突时
fail closed，并单独确认。恢复成功不改变原 permission 决策，也不删除后续审计事件。

### 4.3 security finding 与 permission decision

scanner 输出 finding ID、证据与置信度；permission 另记 allow/deny/ask 和依据。启发式 finding 不能伪装成
安全证明，permission 也不能抹掉 finding。

### 4.4 runtime event 与审计投影

ACP/session event 是 runtime observation；科学保障层 event bridge 是经过脱敏的审计投影。投影可验证自己的 sequence/hash，
但不能声称保留了被策略省略的 raw content 或跨来源全局顺序。

### 4.5 context continuity 与事实真实性

compaction summary 只用于继续工作。LIF 记录其来源边界和 `derived_unverified` 状态；它不能替代原 observation、
validator verdict 或 claim evidence。

## 5. v0.2 取舍

### Adopt / adapt

- **ACP-first**：先做 no-model `initialize` 能力探针，再用 fake provider 验证 `tool_call` →
  `tool_call_update` → terminal continuity，并与 session files 对账。
- **shadow Git recovery fixture**：采用独立 Git dir、容量上限、no-overwrite、conflict-only plan；不接触用户分支。
- **typed identity**：稳定记录 run/session/turn/item/tool-call ID，terminal 恰好一次。
- **narrow hard gates**：workspace trust、secret/network permit、evidence/claim promotion 仍在 LIF 专项科学保障层
  fail closed。

### Defer

- repo map、architect/editor 双模型、多 Agent orchestration；
- server/UI 级 event query、全文索引维护；
- cloud sandbox、remote queue 和持续云运维；
- 超过一次、带工具、带研究数据或可重试的真实 DeepSeek call。一次最小 development
  call 已获单独授权并具备 two-phase one-shot 入口，但仍被固定 credential 缺失阻断。

### Reject as default

- 自建生产 action kernel、model transport、session runtime、permission engine；
- 自建 content-addressed blob 备份层，除非 shadow Git 在固定 fixture 上被机械证明不满足需求；
- 用模型判断 `requires_approval` 作为 secret/network/destructive external action 的唯一 hard gate；
- 因可回滚而自动批准危险操作；
- 手工维护 Grok search index 或从最终文本补造 tool event；
- 默认云同步、raw private reasoning 永久明文落盘。

## 6. 修订后的 implementation spike

0. **Global Progress Sentinel（no-model fixture 已完成）**：从 task contract、计划和 journal 重建方向覆盖、
   未决 acceptance 与 verification debt；生成可去重 WARN 和结构化 disposition。详见
   [`GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`](GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md)。
1. **ACP capability probe（no-model，已完成）**：锁定 binary 的 `grok agent stdio` 已完成
   `initialize`，冻结 protocol/capability/x.ai extension 响应、进程 terminal、全出站阻断和泄漏扫描；未创建
   session 或模型 turn。详见
   [`../docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md`](../docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md)。
2. **ACP fake tool continuity（已完成 Windows fake-only 实测）**：disposable workspace + fake provider 的
   allow-once/cancel 双场景、structured tool ID、状态转换、permission response、terminal、session-file
   对账与独立 verifier 已通过；取消允许无 terminal 或唯一 failed terminal，但严格禁止 completed、第二次
   primary request 和 `tool_completed`。见
   [`../docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md`](../docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md)。
3. **shadow Git recovery fixture**：在现有 hash-only checkpoint 收据中引用 shadow commit/tree；生成显式
   files-only restore plan 和冲突报告，仍不自动 restore 真实 workspace。
4. **Windows child-tree containment（已完成 observed 矩阵）**：固定
   `run_terminal_command` timeout、background task cancel、parent exit 三场景及独立 verifier 已实现；
   candidate `0.2.111` 全部通过，baseline `0.2.106` 的 tool-timeout 暴露返回终态后未退出。start-to-assignment
   race 继续作为已知限制，不扩张 runtime。
5. **compaction provenance（manual 路径已完成）**：fake-only `/compact` 已验证 lifecycle、
   source span/digest、request/checkpoint 与 derived summary 边界；automatic threshold 仍未观测。

每一项都先用 fake/no-key fixture。任何 spike 失败只缩小 capability claim，不修改协议语义来迁就实现。

## 7. 对当前仓库的影响

- `ACTION_KERNEL_CONTRACT`、`MODEL_ADAPTER_LOOP_CONTRACT` 等早期文件保留为历史探索/fixture 规格，不再视为生产
  runtime roadmap；后续可用 ADR 单独做 supersede，不在本轮删除。
- 当前 event bridge、workspace trust、Windows supervisor、DeepSeek fake provider 与 checkpoint/delta 均保留；
  它们分别成为 verifier、hard gate 或 conformance fixture，而不是新的通用 Agent 平台。
- 旧研究工作区只在具体 LIF claim 任务需要时按文件读取并登记来源；本独立仓库不恢复其 index/map/self-check。
