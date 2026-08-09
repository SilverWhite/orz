# orz — Local Assurance-First CLI Agent

ORZ 是一个本地优先、保障优先的 CLI Agent。项目采用融合架构：尽量复用 Grok Build 等成熟组件，同时由 ORZ 自己拥有 Agent control plane、循环编排、保障事件、权限边界和可验证状态链。

> 当前设计：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) v1.1，`accepted / frozen`。
>
> 当前实现：`partial`。现有 Rust workspace 尚未完全符合冻结设计，已知差距集中登记在 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md) 和冻结审计中。
>
> 历史 README：[`存档/readme/README_FULL_2026-08-09.md`](存档/readme/README_FULL_2026-08-09.md)。其中的 Phase/Slice、测试数量和旧冻结叙述只用于回溯。

## 核心定位

- **成熟优先的融合架构**：工具、workspace、sandbox、MCP、chat-state 等能力优先采用成熟实现；出现底层冲突时，由 ORZ 融合 control plane 统一裁决。
- **同构 Agent runtime**：一个主 Agent、一个内部检索子代理和一个外部检索子代理复用同一模型、thinking、transport、工具、上下文、压缩和单会话预算；角色合同与写权限不同。
- **保障内建**：journal、gate、permission、snapshot、compaction、recovery 和 verifier 是 runtime 的组成部分，不是外部包装器。
- **显式检索**：检索模式只能显式选择 `local_browser`、`framework_fallback` 或 `off`，禁止失败后隐式切换。
- **实现不反向定义设计**：代码、Schema 或测试无法表达 ADR-0010 时，登记为差距并扩展实现，不削弱设计以迎合现状。

## 当前状态

| 层面 | 状态 | 说明 |
|---|---|---|
| 自然语言设计 | `accepted / frozen` | ADR-0010 是唯一当前自然语言设计权威 |
| Rust production workspace | `partial` | `orz/` 已有可运行实现，但仍存在已登记符合性差距 |
| Python assurance | `reference` | conformance、Schema authority、fixture、审计和窄兼容路径 |
| Component adoption register | `pending` | 逐 crate/component 所有权需按当前代码重新审计 |
| Windows incident/case evidence | `pending` | 目录已建立，首批结构化证据尚未闭环 |

实现状态不能用 Phase 完成、测试全绿或单次评测结果替代。当前差距见 [`CLI_PROJECT_INDEX.md` §3.1](CLI_PROJECT_INDEX.md#31-已登记实现差距) 和 [`ADR-0010 冻结审计`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md)。

## 快速开始

### Rust Agent

要求：Windows 为当前主要开发环境；Rust toolchain 由 [`orz/rust-toolchain.toml`](orz/rust-toolchain.toml) 固定。

```powershell
Set-Location D:\CLI\orz
cargo build -p orz-bin
```

使用离线 fake provider 运行一个 headless prompt：

```powershell
cargo run -p orz-bin -- --fake-provider -p "hello"
```

启动 TUI：

```powershell
cargo run -p orz-bin -- --fake-provider
```

运行 plan mode：

```powershell
cargo run -p orz-bin -- --fake-provider --plan -p "inspect this workspace"
```

其他生产入口：

- `--real -p "<prompt>"`：使用真实 DeepSeek transport；凭据缺失时 fail-closed，不回退 fake provider。
- `--stdio`：启动 ACP stdio server。
- `--replay <events.jsonl>`：只读回放 journal。
- `--allow-write`：仅显式授予 headless 本地文件编辑；不会自动放开 Bash 或网络。
- `--max-wallclock <seconds>`：设置模型不可见的整轮 wallclock 上限。

真实 transport 的凭据约束见 [`ADR-0006`](adr/ADR-0006-credential-target-registry.md)，transport/retry 边界见 [`ADR-0007`](adr/ADR-0007-transport-retry-policy.md)。

### Python assurance reference

Python 路径不是 production Agent runtime。它用于 conformance、fixture、Schema 和离线验证：

```powershell
Set-Location D:\CLI
python -m pip install -e ".[dev]"
python gsa.py doctor --quick
```

详细入口见 [`assurance/README.md`](assurance/README.md) 和 [`PYTHON_REFERENCE_SPEC_CONTRACT`](architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md)。

## 架构概览

| 组件 | 当前职责 |
|---|---|
| `orz-loop` | 共享 Agent loop、模型/工具轮、检索角色调度和运行时守卫接线 |
| `orz-host` | ACP/session host、工具执行、权限桥和 runtime integration |
| `orz-assurance` | journal、gate、snapshot、credential、permit、recovery 和验证机制 |
| `orz-bin` | TUI、headless prompt、plan、ACP stdio 和 replay 入口 |
| `orz-tui` | 默认终端工作台和 journal projection |
| `orz-codex` | 默认不启用的 Codex-style fallback surface；不扩展为完整 IDE |
| `assurance/` | Python reference/conformance、Schema authority 和离线验证 |

主 Agent 和两个检索子代理必须复用共享 runtime。子代理只能编辑自身 blackboard 检索分区、当前任务检索文档和检索记录存档；具体合同见 ADR-0010。

## 仓库结构

| 路径 | 内容 |
|---|---|
| [`orz/`](orz/) | Rust production workspace |
| [`assurance/`](assurance/) | Python reference/conformance 实现 |
| [`runtime/`](runtime/) | run-event、manifest、fixture 和 runtime Schema |
| [`protocol/`](protocol/) | 协议草案、原因码和 gate matrix |
| [`regression/`](regression/) | 案例、fixture 和覆盖矩阵 |
| [`evaluation/`](evaluation/) | 评测协议和结果 Schema |
| [`adr/`](adr/) | 架构裁决记录 |
| [`architecture/current/`](architecture/current/) | ADR-0010 的当前派生投影入口 |
| [`docs/audits/`](docs/audits/) | 当前设计冻结与补充审计 |
| [`docs/incidents/`](docs/incidents/) | 产品事故记录入口 |
| [`docs/cases/`](docs/cases/) | 精选兼容性案例入口 |
| [`存档/`](存档/) | 已退出当前基线的历史材料 |

## 设计与审查入口

建议按以下顺序回查：

1. [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)：查稳定 ID、当前状态、关键词和精准入口；文件开头固定了写入格式与维护纪律。
2. [`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)：核对当前自然语言设计。
3. [`architecture/current/README.md`](architecture/current/README.md)：查无新增语义的当前设计投影。
4. [`ADR-0010 v1.1 补充复核`](docs/audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)：查裁决来源和待补工程内容。
5. [`ADR-0010 冻结与归档审计`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md)：查冻结时实现偏差。
6. Schema、当前源码和可复现测试：核对机器表达与实际实现，不用它们反向改写设计。

历史材料统一从 [`存档/README.md`](存档/README.md) 进入。归档材料只作来源回溯和演进审计，不独立产生当前实现需求。

## 已知符合性差距

冻结审计至少登记了以下未闭合项：

- Rust tool round budget 仍需从旧值迁移到 ADR-0010 冻结值，并重写相关测试。
- 旧 inquiry producer 仍需拆分为 Orientation、Information Sufficiency、Counterexample 和 Runtime Stagnation Guard。
- 检索子代理仍需从一次调用、零工具特殊路径迁移到共享 Agent runtime。
- run-event Schema 仍需完整表达机械充分性、parent disposition、contract revision 和关闭 CAS。
- Windows incident/case 需要真实 provenance、脱敏、分类和回归门槛。
- 逐 crate/component 采用矩阵需要按当前代码重新审计。

这些差距的 canonical ID 和入口维护在 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)，不得在 README 中继续扩展实施流水。

## 安全与证据边界

- 真实 provider 凭据不得写入仓库、prompt、journal 或调试输出；真实 transport 不得静默回退 fake。
- 工具、wallclock、停滞和权限路径必须 fail-closed，并保留可验证终态。
- 外部来源必须记录全文可见性；metadata、摘要或片段不得冒充全文证据。
- 历史回归案例不得直接作为被测 Agent 的答案提示。
- 本仓库不是 LIF/FEP 科学 claim 的事实源，不替代相应 INDEX、MAP、R、JSON、日志或原始产物。
- Global Review Mode 的 activation receipt 只激活审查义务，不等于审查结论。

## 开发检查

按改动范围选择最小充分验证；高风险 runtime 变更再扩大到 workspace 级：

```powershell
Set-Location D:\CLI\orz
cargo fmt --check
cargo check -p orz-loop -p orz-host -p orz-assurance -p orz-bin
cargo test -p orz-loop -p orz-host -p orz-assurance
```

文档、Schema 和 Python reference 变更还应运行仓库检查及相关测试；不得用旧测试锁定已被 ADR-0010 废止的逻辑。

## License

Apache License 2.0。见 [`LICENSE`](LICENSE) 和 [`NOTICE`](NOTICE)。
