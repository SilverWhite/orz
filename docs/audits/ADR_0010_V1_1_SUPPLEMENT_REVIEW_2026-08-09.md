# ADR-0010 v1.1 六项补写严格审查（2026-08-09）

## 1. 总结

本轮回看旧设计、Python assurance 路径和当前 Rust 实现后，ADR-0010 升级并重新冻结为 v1.1。裁决不是
“记得的都恢复、记不得的都删除”，而是按产品价值、机械可验证性、实现事实和职责边界逐条处置。

## 2. 六项裁决

| 组 | 严格判断 | 裁决 |
|---|---|---|
| 显式检索模式 | 能阻止来源层级和 receipt 因 runtime 自动 fallback 而漂移，且 Python 路径已有 allowlist/gate | 保留 `local_browser/framework_fallback/off`；未授权为 `off`，禁止隐式切换 |
| Diagnostic / Global Review / IDE | 三者分别解决 debug 路线锁死、局部审查冒充全局审查、生命周期证据丢失；职责互不重叠 | 全部保留；IDE 材料维持 evidence-only，不扩完整 IDE |
| D-1 / D-9 / LBR / UI | 产品价值成立，但旧实现或旧文案存在 grep-only、执行误称只读、参数过时、session 所有权过时等问题 | clause-split 保留并重裁，不逐字恢复 |
| Information Sufficiency / Close | 充分性判定交给模型会增加不确定性；但主 Agent 明确提出新检索需求时，关闭不能抢先提交 | assessment 完全机械；主 Agent 结构化 `close|continue(requirement_delta)`，continue 保持 active；只有 close commit 重置 live state |
| crate/component matrix | 旧矩阵假设 Grok 完整拥有 loop/session/tool，与融合 control plane 冲突；当前 register 也尚不存在 | 不恢复旧结论；下一轮逐 crate/component 审计后生成 register |
| model / rounds / denial | 当前代码确认 DeepSeek V4；旧轮次口径不完整；累计 10 次只是一次性 prompt，并非真正 ceiling | 默认 DeepSeek family/current V4；工具/问询/恢复后轮次计数；删除 total-denial 10，只留同类拒绝连续三轮熔断 |

## 3. 五项历史内容复核

### 3.1 FIX_PLAN D-1

“使用来源时就地绑定可定位证据”合理，必须保留；固定 `[来源: 路径:行号]` 加 grep 并不充分：标记可以
指向不存在或不支持 claim 的来源，文档行号也会漂移。当前 Rust `prompt.rs` 和 retrieval prompt 仍锁定
旧字符串，后续应改成 `source_id -> ledger identity/visibility/claim limit` 的结构化验证，渲染层再显示
人类可读定位。

### 3.2 FIX_PLAN D-9

`run_tests` 反馈环值得保留，当前实现已经有 fixed command、30 分钟 timeout、1 MiB 输出上限、32 KiB
上下文 tail、完整输出 artifact 和 Job Object 处理。但“只读权限”表述错误：运行测试可执行任意测试代码、
写文件、联网和派生进程。下一轮必须审计 permission category、sandbox/network、环境脱敏、workspace delta
和 hidden-output 泄露；不能仅因 `ToolDispatcher::is_file_edit("run_tests") == false` 就称安全。

### 3.3 Local Browser

保留显式状态/异常、PDF 验证、redirect 后 URL 复查、网页文本不具指令权、默认禁任意 JS、失败显式化
和来源可见性层级。旧单一路径、`task_timeout_seconds: 60`、固定 tab 数等属于过时参数，不进入冻结核心。
Python assurance 路径已有较完整实现与测试，但 Rust 同构检索子代理尚未接入，不能写成 production closed。

### 3.4 Toolbar / 只读 session 投影

当前 `orz-tui` 已实际实现 Toolbar、Find、双 Esc session/run-history 列表和 `.gsa/runs/` 只读扫描，因此
保留不是凭空恢复。应保留的是 presentation/projection，不是旧的 Grok-owned persistence：UI 不拥有
session 事实和 restore，数据源转为 ORZ 当前 journal/session index。按钮排列和手势可继续演进。

### 3.5 crate/component 采用矩阵

旧能力级借鉴结论仍有参考价值，但具体 ownership 已随融合架构变化。`upstream/fusion-component-register-*`
当前不存在；下一轮必须从当前代码可达性和 local diff 出发，不可从 crate 名或上游来源推断采用状态。

## 4. Information Sufficiency 与关闭语义

- assessment producer：controller/verifier；无模型调用。其后的 parent disposition 是独立的主 Agent
  生命周期控制输出，不是模型重新判定充分性。
- 输入：task/retrieval contract、result、source ledger、visibility gate。
- 输出：`sufficient/insufficient/indeterminate/not_applicable`、reason codes、counts、digest、version。
- 控制后果：assessment 状态本身不自动关闭或继续；主 Agent 必须提交结构化 disposition。`close` 才允许
  normal close commit；`continue(requirement_delta)` 修订合同并保持同一 activation active。
- 新需求：requirement delta 必须非空、可验证；扩大 source/tool/path/permission scope 时重新经过
  task-contract/capability gate。不得先关闭再重开，也不得把 continue 解释成子代理自审查。
- 缺失 disposition：进入 `awaiting_parent_disposition`，不关闭、不自动调用子代理、不从自由文本猜测。
- 去重：同 contract revision 由
  `(activation_id, contract_revision, result_digest, assessment_version)` 幂等；continue 后 revision 递增，
  新结果可重新 assessment；只有 close commit 后清空 activation live 去重状态。
- 持久性：journal、检索文档、ledger、result archive、assessment、disposition 和 close receipt 永久保留，
  不随 live reset 删除。

## 5. 当前落地差距，供下一轮审计

| ID | 当前观察 | 下一轮必须确认 |
|---|---|---|
| V11-IMPL-001 | 显式 retrieval mode 主要存在于 Python CLI/receipt/gate，Rust retrieval session 未接入 | mode authority、transition event、禁止 fallback 是否端到端成立 |
| V11-IMPL-002 | Python Diagnostic Coverage 已有 2→3→4→5；需与 v1.1 去主观 0.5、event identity 规则对照 | signal producer、去重、episode close、Rust 接线 |
| V11-IMPL-003 | Global Review receipt 已实现于 Python | activation 与真正审查结论是否仍严格分离 |
| V11-IMPL-004 | Rust D-1 仍是 prompt 字符串 + grep 测试 | source ledger binding、claim limit verifier、renderer |
| V11-IMPL-005 | `run_tests` 已落地但需安全复核 | permission、sandbox/network、hidden-output、写副作用 |
| V11-IMPL-006 | LBR Python 实现存在，Rust 子代理仍 `tools: Vec::new()`/single pass | 同构 session、真实工具、状态/失败事件、archive |
| V11-IMPL-007 | Toolbar/只读 run-history 已落地 | 数据源是否统一到当前 ORZ session ownership，旧路径假设是否残留 |
| V11-IMPL-008 | component register 不存在 | 逐 crate 来源、local diff、可达性、license、seam、owner |
| V11-IMPL-009 | 仍有 `neutral_inquiry`、free-form retrieval completion、旧 Schema | assessment → parent disposition → close commit 的 v0.2 Schema/producer/verifier/replay 迁移；检查 revision CAS、幂等与迟到 close 竞态 |
| V11-IMPL-010 | 当前 model ID 为 `deepseek-v4-flash`，符合 family/V4；三个 Agent runtime 尚不同构 | 单一 registry 与三实例相同配置是否真正成立 |
| V11-IMPL-011 | controller 仍是旧 40 工具轮和旧 inquiry counter | 120 工具轮、7 逻辑模型轮、recovery 持久计数 |
| V11-IMPL-012 | denial 仍按每 tool call 累积 3/10 | 改为同 denial key 连续三 tool-call rounds，删除 total 10 |

本审查只确定设计和下一轮审计范围，不修改 Rust producer、Schema 或测试；旧测试通过不能覆盖上述差距。
