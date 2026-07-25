# ADR-0003：保障内核保持 runtime-neutral，Grok 仅作为参考适配

- 状态：Accepted
- 日期：2026-07-24
- 决策范围：通用 runtime、Assurance Kernel、profile 与参考实现的所有权边界
- 替代：`PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md` 中“Grok 是唯一产品底座”的裁决

## 1. 背景

仓库已经围绕 Grok Build 完成 ACP、workspace trust、Windows 子进程树、compaction provenance、
DeepSeek transport 与真实单轮会话的多项 conformance。它们证明 Grok 是当前证据最完整的参考框架和
首个适配目标，但不能推出 Grok 必须成为产品唯一、永久或不可替换的 runtime。

把“已验证最多”写成“唯一底座”会产生三类问题：

1. profile 会把实现名称误当成所需能力，削弱可替换性；
2. Grok 的上游限制可能被错误提升为 Assurance Kernel 的固有边界；
3. 新 runtime 即使通过同一套门禁，也会被文档上的确定性绑定排除。

本裁决只修改 CLI 工程架构，不产生或修改任何 LIF/FEP 科学 claim。

## 2. 决策

1. General Assurance Kernel 必须保持 **runtime-neutral**。它只依赖结构化 adapter contract、
   capability envelope、receipt 和独立 verifier。
2. Grok Build 是当前的 **reference runtime** 与证据最完整的 adapter candidate，不是强制底座。
3. profile 可以要求能力、隔离强度、事件完整性和验收 fixture；不得要求某个 runtime family。
4. runtime 由用户或 profile 在已通过门禁的候选中选择。没有合格候选时 fail closed，不因 Grok
   不可用而静默降级，也不因 Grok 可用而跳过门禁。
5. 任一 runtime（包括 Grok）都必须通过同一类 trust、sandbox、identity、retention、
   anti-injection、capability inheritance 和 audit completeness 验证，才能被某个 profile 接受。
6. 本仓库不自建第二套通用 model/tool/session runtime。`runtime-neutral` 表示可通过窄 adapter
   替换外部 runtime，不表示同时维护多个完整 runtime。

## 3. 所有权

| 层 | 负责 | 不负责 |
|---|---|---|
| runtime | model/tool loop、session、permission、基础 sandbox、provider 与通用工具 | 宣布自己满足本项目的保障门禁 |
| runtime adapter | 能力发现、typed event 映射、取消、terminal 与 runtime-specific receipt | 改写 profile 的 hard gate |
| General Assurance Kernel | trust、effective envelope、sandbox selection、credential/network permit、retention、provenance、audit verifier | 实现通用模型循环 |
| profile | 任务特有的能力要求、证据规则与验收门槛 | 全局绑定 runtime family |
| `lif-research` profile | SourceRouter、EvidenceKernel、ValidatorBridge、claim boundary、oracle isolation | 把 LIF/FEP 理论冒充 runtime 控制算法 |

## 4. 选择规则

runtime 选择顺序不是固定产品名单，而是：

1. 读取 profile 的 required capabilities 和 acceptance gate；
2. 对候选 adapter 生成带证据状态的 capability receipt；
3. 验证 workspace trust、sandbox、session identity 与 retention 前置条件；
4. 只在全部 hard gate 通过后生成 Effective Security Envelope；
5. 记录实际选择及其证据，不把 reference role 提升为 required role。

Grok 的 observed fixture 可以复用为参考基线；其他 runtime 必须产生自己的 observed fixture，
不能继承 Grok 的通过状态。

## 5. 后果

### 正面

- 工程不再被单一上游实现锁定；
- Grok 已有证据继续保留，不需要否定或重跑；
- profile 的安全要求可以跨 runtime 比较；
- “参考框架”和“已通过的执行框架”成为不同状态。

### 代价

- 每个新 adapter 都需要独立 fixture 和 verifier；
- runtime 选择与 capability discovery 成为显式前置阶段；
- 旧文档中的“唯一底座”“Grok owns production runtime”必须标为历史裁决或改写。

## 6. 验收

- profile registry 不存在 `required_runtime_family` 或等价全局绑定字段；
- `lif-research` 将 Grok 标为 `reference_only`；
- Effective Security Envelope 记录实际 runtime、adapter、binary/capability digest 和证据状态；
- 未观测或不合格 runtime 不得因 reference 身份被放行；
- 仓库机械检查包含“强制 runtime 绑定”负例。
