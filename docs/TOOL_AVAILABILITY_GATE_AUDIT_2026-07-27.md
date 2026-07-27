# Tool Availability Gate audit（2026-07-27）

## 裁决

新增工具可用性机械门禁（Tool Availability Gate），填补 Agent 框架中"机械组件可用性状态向模型回传"的缺口。
本轮修订新增三个独立机制，全部遵循 existing no-model/offline 边界，不调用真实模型、不联网、不读取 credential。

## 问题背景

问题源自对 DeepSeek 网页端与移动端 App 的实测观察，非本 CLI 自身的 web 界面。具体表现为：

- **快速模式（未开启搜索）**：模型无法主动探测搜索工具的可用状态。web/app 框架未将"搜索不可用"这一机械事实回传给模型，模型在信息缺失下进入猜测和逻辑循环，错误率极高。
- **手动开启搜索后**：模型直接使用检索功能，错误消失。
- **专家模式（明确关闭搜索）**：模型从上下文获知搜索已关闭，不进入猜测。

该问题暴露的是**模型上游框架层面的共性缺口**：无论 DeepSeek 还是其他 Agent runtime，若框架不将工具/组件的机械可用状态明确注入模型上下文，模型就无法区分"工具不可用"与"我还没尝试"——这正是魔改版 CLI 作为保障层需要机械介入的位置。CLI 在模型被调用前执行工具可用性探测，将状态明文回传，从根上消除这整类信息缺失导致的猜测。

## 新增组件

### 1. Tool Availability Gate (`assurance/tool_availability_gate.py`)

机械探测函数 `probe_tool_availability`：
- 接收 `tool_specs`（工具声明列表）和 `probe_registry`（运行时探针注册表）
- 对每个工具执行机械状态检查：`available` / `unavailable` / `unprobed` / `degraded`
- 生成标准化的 `tool_availability_report`（schema: `tool-availability-report-v0.1.schema.json`）
- 生成 gate receipt（schema: `tool-availability-gate-receipt-v0.1.schema.json`）
- 生成 `[TOOL_AVAILABILITY v0.1]` context block，内含明确的 AVAILABLE/UNAVAILABLE/DEGRADED/UNPROBED 列表

Context block 格式示例：

```text
[TOOL_AVAILABILITY v0.1]
AVAILABLE: bash_exec, file_read, file_write
UNAVAILABLE: search
[/TOOL_AVAILABILITY]

You MUST NOT guess whether an unavailable tool exists or what it would return.
```

Gate decision 逻辑：
- `allow`：所有工具已探测且无 degraded
- `defer`：存在 unprobed 工具
- `block`：存在 degraded 工具

### 2. Tool Belief Mismatch Detector (`evaluate_tool_belief_mismatch`)

独立检测模型对工具可用性的错误信念：
- 对照 `model_claimed_available` / `model_claimed_unavailable` 与实际 gate 报告
- 检测三种 mismatch 类型：
  - `wrong_belief_about_availability`：模型声称不可用的工具实际可用
  - `wrong_belief_about_unavailability`：模型声称可用的工具实际不可用
  - `tool_capability_inflation`：模型将 degraded 工具当作可用

### 3. Tool Belief Stagnation Guard (`evaluate_tool_belief_stagnation`)

基于公开输出的工具信念停滞检测：
- 检测不可用工具的名字被模型提及（`TOOL-BELIEF-UNAVAILABLE-TOOL-MENTIONED`）
- 检测工具使用语言模式（如"我将搜索"、"I will use the search tool"），标记为 `TOOL-BELIEF-CAPABILITY-GUESSING`
- 仅当显式提及不可用工具名时触发 restart；纯猜测仅记录警告不重启

## Orientation Checkpoint 扩展

### Schema 变更
- `orientation-checkpoint-v0.1.schema.json`:
  - `message_block` 正则放宽为 `^(\\[TOOL_AVAILABILITY|\\[ORIENTATION_CHECKPOINT)`，允许 infused 格式
  - `allowed_response_fields` enum 新增 `available_tools_acknowledged`
  - `forbidden_response_fields` enum 新增 `tool_belief_mismatch_statement`
  - 新增可选字段 `tool_availability_sha256`
- `orientation-checkpoint-verification-v0.1.schema.json`:
  - `allowed_fields_observed` enum 新增 `available_tools_acknowledged`
  - `forbidden_fields_observed` enum 新增 `tool_belief_mismatch_statement`

### 新增函数
- `build_tool_availability_infused_orientation_block`：将工具可用性 context block 与中性 orientation 问题组合
- `build_tool_availability_infused_orientation_checkpoint`：生成 infused checkpoint
- `build_orientation_checkpoint` 新增 `tool_availability_sha256` 参数（可选，向后兼容）

## Integration 扩展

### `orientation_runtime_integration.py`
- fixture schema 新增可选字段：`tool_specs`、`probe_registry`、`available_tool_names`、`unavailable_tool_names`
- 当 fixture 包含 `tool_specs` 时，自动运行 tool availability gate 并写入额外 artifact：
  - `tool-availability-report.json`
  - `tool-availability-gate-receipt.json`
  - `tool-belief-stagnation-receipt.json`
- 无 tool_specs 时保持向后兼容，不写入额外文件

### `orientation_runtime_journal.py`
- 事件序列新增可选事件 `tool_availability_check`（在 `run_started` 之前）
- 事件序列新增可选事件 `tool_belief_stagnation`（在 `runtime_stagnation_guard` 之后）
- 新增 payload schema：
  - `tool-availability-check-event-payload-v0.1.schema.json`
  - `tool-belief-stagnation-event-payload-v0.1.schema.json`
- verifier 支持可变事件计数（5-8 个事件）

## 文件清单

### 新增文件
| 文件 | 用途 |
|---|---|
| `assurance/tool_availability_gate.py` | 核心组件 |
| `assurance/tool_availability_report-v0.1.schema.json` | tool availability report schema |
| `assurance/tool_availability_gate_receipt-v0.1.schema.json` | gate receipt schema |
| `assurance/tool_availability_check_event_payload-v0.1.schema.json` | journal event payload schema |
| `assurance/tool_belief_stagnation_event_payload-v0.1.schema.json` | journal event payload schema |
| `assurance/tests/test_tool_availability_gate.py` | 26 个测试 |
| `docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md` | 本审计文档 |

### 修改文件
| 文件 | 变更内容 |
|---|---|
| `assurance/orientation_runtime_guard.py` | 新增 3 个函数、扩展 stagnation guard |
| `assurance/orientation-checkpoint-v0.1.schema.json` | 扩展字段枚举和 message_block 正则 |
| `assurance/orientation-checkpoint-verification-v0.1.schema.json` | 扩展字段枚举 |
| `assurance/orientation_runtime_integration.py` | 集成 tool availability gate |
| `assurance/orientation_runtime_journal.py` | 新增 tool availability 事件 |
| `assurance/orientation-stagnation-integration-fixture-v0.1.schema.json` | 新增 tool_specs 等可选字段 |
| `assurance/orientation-stagnation-integration-receipt-v0.1.schema.json` | 新增 tool availability artifacts 和 checks |
| `assurance/__init__.py` | 新增 7 个导出符号 |
| `assurance/tests/test_orientation_runtime_journal.py` | 适配新错误消息 |

## 测试覆盖

- **Tool Availability Gate**（8 个测试）：probe 分类、上下文块生成、gate receipt 决策、输入校验
- **Tool Belief Mismatch**（4 个测试）：不可用/可用不匹配、degraded 检测、正常通过
- **Tool Belief Stagnation**（5 个测试）：不可用工具提及、猜测检测、正常使用、无提及
- **Infused Orientation Checkpoint**（5 个测试）：内容包含、中性保持、acknowledgement 允许、mismatch 拒绝、sha256 绑定
- **已有测试回归**（4 个测试）：stagnation guard 仍正常工作、orientation checkpoint 无 tool_specs 时行为不变

## 边界与限制

- Tool availability probe 使用静态 fixture registry，不执行真实运行时 hook；生产环境需由 runtime adapter 维护 probe_registry
- Tool belief stagnation 的名词检测基于子串匹配，可能遗漏别名或变体
- Capability guessing 的检测使用关键词模式，可能对假设性讨论产生误报
- 当前仍为 offline/fake 模式，不调用真实模型、不发起网络请求

## Canonical CLI 接入（同日）

`assurance/canonical_cli.py` offline `run` 已在 source visibility gate 之前执行 tool availability
probe，写入 `tool-availability-report.json` / `tool-availability-gate-receipt.json`，并把摘要绑定进
answer packet 与 journal 事件 `tool_availability_check`。当前 probe_registry 固定为 offline fixture
（工具均标为不可用/未探测集合由 specs 驱动），不证明真实 runtime 工具状态。

## 下一步

后续适合接入：
- 真实 runtime adapter 的 tool registry 探针（替代 fixture registry）
- 将 tool availability context block 注入真实模型 system prompt 的接线
- 对已知不可用工具的模型幻觉进行更精确的 claim 级别检测
