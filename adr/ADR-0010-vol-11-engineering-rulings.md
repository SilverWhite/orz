# ADR-0010 分卷 11：§11 配套工程裁决

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 11（§11 配套工程裁决）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

## 11. 配套工程裁决

本节原有七个开放问题及 v1.1 六组补充裁决均已关闭。子系统设计可以细化字段、API 和迁移步骤，但
不得重新打开这些产品语义。

### 11.1 Orientation 只采用轮次触发

不增加 semantic action 辅助触发器。当前唯一常规触发是 session-level 7 个已完成对话轮；pre-handoff
等显式生命周期检查可以使用独立 trigger reason，但不参与七轮计数。理由是 semantic action 需要依赖
模型、工具或启发式分类器解释，无法形成稳定、可重放的机械事实，并会变相恢复已否决的 tool count。

### 11.2 Inquiry 拆为显式事件类型

采用独立的 `orientation_checkpoint`、`diagnostic_coverage_checkpoint`、
`information_sufficiency_assessment`、`retrieval_parent_disposition` 与 `retrieval_close_record` event type 和
payload Schema，不采用单个 `neutral_inquiry` 加 `oneOf`。前两者是中立问询；assessment 和 close record
是机械记录；parent disposition 是结构化生命周期命令。旧 `neutral_inquiry` 和
`retrieval_completion_check` 只用于 v0.1 journal replay，不得由新 writer 产生。

### 11.3 子代理工具、预算、并发和写权限

1. 两个检索子代理可以双并发；并发上限为两个 active retrieval session，项目文档/工作区检索和外部
   来源检索各占一席，不动态生成同角色副本。
2. 子代理使用与主 Agent 相同的模型、模型参数、具体工具 registry、单轮预算和 session 总预算默认值；
   默认工具轮预算均为 120；每个 session 独立计账，主 Agent 不与子代理争用同一个递减余额。
3. 工具一致不等于文件写权限无限。子代理只可写自身 blackboard 检索分区、当前任务检索文档和检索
   记录存档；除此之外的文件写入一律拒绝。
4. 写域通过 session capability receipt 冻结，使用解析后的绝对路径/对象 identity 校验；所有 shell、
   MCP 和脚本间接写入仍走同一门禁，并拒绝通过 symlink、junction、reparse point 或路径穿越逃逸。
5. 内部/外部检索角色各一个 active instance，允许双并发；全局 `web_search` concurrency 固定为 1。

### 11.4 子代理不设置专属资源限制

不为子代理额外设计内存、磁盘、保留期、模型降级或更短 wallclock 上限；它们继承主 Agent 的 session、
archive、compaction、retention、cancel 和宿主安全策略。这里的“不设限”是“不设置子代理专属限额”，
不表示绕过操作系统资源约束、用户取消、全局故障保护或与主 Agent 相同的预算终止语义。

### 11.5 融合组件来源登记

旧 CN 文档 §5.2 的“Grok 完整拥有 model loop/session/tool dispatch”矩阵与本 ADR 的融合 control plane
冲突，**不得原样恢复**。保留“逐 component 记录来源、采用档位和修改边界”的治理方法，并在下一轮
实现审计中建立以下成对制品：

- `upstream/fusion-component-register-v0.1.schema.json`：机械 Schema；
- `upstream/fusion-component-register-v0.1.yaml`：机器可读 current inventory；它是 ADR 投影，不独立改变设计。

截至 ADR v1.1 冻结，这两个文件均不存在，状态必须写 `not_started / audit_required`，不能根据 crate 名、
上游来源或编译通过推定 `inherited`、`modified` 或 `local`。下一轮审计至少逐 crate/component 回看当前
代码、Cargo dependency/feature、公开 seam、调用入口、实际可达性、上游 revision、local diff、license、
测试证据和升级责任人；tool/workspace/VCS、sandbox/permission primitives、ACP/MCP、provider client、
compaction、host/loop/assurance/UI 等能力群都必须覆盖。

每个 component entry 至少包含：稳定 component ID、能力、代码路径、origin kind/project/version/revision/
license、采用档位（direct reuse/thin adapter/controlled fork/local replacement/self-build）、ownership
分类（inherited/modified/replaced/local）、继承/修改/替换路径集合、公开 seam、冲突证据、设计记录、
验证引用、升级责任人与当前状态。三个路径集合必须互斥，lock/candidate 文件只通过引用关联，不复制
版本事实。进入受控 fork、局部替换或自研档位时，register entry 必须引用 §2.2 要求的设计记录。

### 11.6 Journal 版本迁移与 replay

1. 已写 journal 和已发布 Schema 不原地修改；新事件体系进入 `run-event-v0.2.schema.json` 及版本化
   payload Schema，新 writer 只产生当前版本。
2. 同一 hash chain 不混写 envelope 版本。旧 journal 保持字节不变，不通过“迁移”重算历史 digest。
3. offline replay/verifier 通过版本 registry 永久保留所有进入仓库或证据索引的历史版本；不设置按日期
   到期的 replay 兼容期。
4. 产品热路径只向当前版本追加。需要恢复旧 session 时，先只读 replay，再生成 migration receipt 和
   新版本 snapshot/journal；新 journal 记录旧 terminal/event digest，形成可追溯跨版本链接。
5. 旧版本 live-resume adapter 保留到所有被标记为 resumable 的旧 session 已迁移或显式关闭；纯历史
   evidence 之后只由 offline replay 支持，避免永久扩大产品热路径。

### 11.7 Windows 案例首批裁决与目录

正式路由冻结为：

```text
architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md
docs/incidents/windows/                 # 追加式事故与未闭合限制
docs/cases/windows/                     # 审计通过的脱敏精选案例
regression/windows/                     # 自动 fixture、脚本与人工复核入口
.observed-runs/windows/                 # git-ignored 原始运行；案例只引用 digest/manifest
```

首批审计采用保守晋级：

- `GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md` 的 `tool_timeout`、`task_cancel`、`parent_exit` 具有明确
  fixture、result/verification Schema、baseline/candidate 对照、digest 和零残留检查，分别晋级为
  `ORZ-WIN-PROC-001`、`ORZ-WIN-PROC-002`、`ORZ-WIN-PROC-003` 精选案例候选；整理时不得合并为一个
  模糊的“Job Object 已通过”案例。
- `GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md` 保留在事故路由，待 raw artifact identity 和结构化
  verification 补齐后再晋级，不能只凭修复说明与测试总数进入案例库。
- Windows native sandbox 的 raw TCP 残余保留为 open limitation/incident；host firewall 补偿不能被
  重写为 AppContainer 自身完成网络隔离。
- credential hardening 当前主要是 offline implementation evidence，不能晋级为 Windows Credential
  Manager 实机兼容案例，直至真实 credential-read 路径完成脱敏验证。2026-08-11 方向修正后
  web_search 读取路径复用主 DeepSeek key 通道（零化 + `redacted()` 唯一序列化出口，当前生产
  接线=构建时 `tracing::info!(redacted)`），DeepSeek 实机 live 测试已跑通
  （`live_deepseek_web_search_roundtrip`）；Windows 实机晋级仍待精选案例路由（2026-08-11 登记）。

### 11.8 受信控制与动作授权面（ADR-0011）

Authenticated Control and Action Fabric（ACAF）的决策权威由 [`ADR-0011`](ADR-0011-authenticated-control-and-action-fabric.md)
承担，详细设计见 [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。
ACAF 派生自 §2.4（permission hard gate）、§3.2/§3.8（子代理写域与受控 `run_tests`）、
§4.1/§4.2（机制分层与 7 轮机械计数）、§5.3/§5.4（Schema 迁移纪律与 journal/snapshot）与
§11.3（写域解析验证），**不改变本 ADR 任何既有条款**，不得反向削弱本 ADR 的机制分层、
机械判定语义或权威层级。ACAF 为 GAP-DENIAL-POLICY-REVISION 提供必接消费面，并为
GAP-RUN-TESTS RT-001/002/003 建立执行器前置依赖；其实施切片独立于 §9 Phase C 排期，
仅共享"先扩展 Schema/fixture 再修改 producer"的迁移纪律。

