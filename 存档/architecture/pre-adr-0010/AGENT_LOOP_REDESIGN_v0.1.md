# Agent Loop 重设计 v0.1

> Archive metadata: original_path=`architecture/AGENT_LOOP_REDESIGN_v0.1.md`; archived_at=`2026-08-09`; final_status=`transferred/superseded`; superseded_by=`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`; authority=`historical design input only`.

> **注意 (2026-08-03)**：本文档已被 [`INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.1.md`](INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.1.md) 整合。
> 整合后的关键变化：
> - Pro/Flash/Blackboard/MechanicalRelay/检索子代理 完整采纳
> - Agent Loop Controller 作为独立 `orz-loop` crate（不是注入 orz-shell）
> - 注入点 IP1-IP2 重新归类为 Agent Loop Controller 内置功能（不再称"注入"）
> - IP3/IP4/IP5 保留为对 kept Grok 组件的真正注入
> - IP6 提升为架构不变量
> - 本文档作为**Agent Loop 设计参考**保留，实施阶段以整合文档为准。

状态：2026-08-03。基于 Plan A 编译修复后的审查发现、D 项目黑板/蜂群传导的经验教训、
以及多款成熟 CLI 的架构调研，重新设计 orz 的 agent loop 架构。

## 1. 动机

### 1.1 Grok Sampler 的问题

Grok 的 `orz-sampler`（~10,000 行）将模型选择、goal 选择、memory dream 触发、recap 生成、
prompt 构建、tool call 采样等全部混杂在一个 crate 中。设计目标与 orz 根本不同：

- Grok 需要管理多模型切换、云端配置下发、遥测打点
- orz 需要的是干净的 agent loop + assurance 注入

Plan A 恢复了 sampler 的完整源码以通过编译，但长期来看必须替换。

### 1.2 从 D 项目学到的教训

D 项目（2026-03，4-LLM 管线 + 黑板通信）遇到了三个核心问题：

| 问题 | 表现 | 根因 |
|------|------|------|
| 格式规整性 | `<NEED: xxx>` 正则匹配不可靠 | 模型不保证输出特定文本格式 |
| 传导与检索冲突 | NEED 信号和 swarm_fetch 两套机制并存 | 双轨制互相干扰 |
| debug 困难 | 静默失败难以定位 | 无法区分"模型不听话"和"传导层有 bug" |

**核心教训**：机械传导不应依赖文本标签匹配。`swarm_fetch` 工具（走 function calling）比 `<NEED:>` 正则可靠得多——验证了传导应该走 API 层结构化输出的方向。

### 1.3 从成熟 CLI 学到的模式

| CLI | 可取模式 | orz 采纳方式 |
|-----|---------|------------|
| **Codex CLI** | 简单 while 循环（no complex sampler） | Agent Loop Controller |
| **Aider** | Architect/Editor 双模型分离 | Pro(规划) + Flash(执行) |
| **Goose** | streaming-first Provider trait | ModelGateway 流式响应 |
| **Claude Code** | permission pipeline + todo tracking | IPG + Orientation + GateDecision |
| **Gemini CLI** | ReAct loop + CoreToolScheduler | ToolDispatcher + MechanicalRelay |

## 2. 架构总览

### 2.1 核心原则

1. **不做 "Sampler"，做 "Agent Loop Controller"**——~1,000 行的薄层，职责清晰
2. **纯 function.name 机械路由**——不依赖正则/文本标签匹配（D 项目的教训）
3. **黑板分区写入/共享读取**——模型不直接对话，而是读写同一结构化状态
4. **Pro + Flash 对等设计**——配置相同，分工不同；Pro 保有审查和授权权
5. **检索子代理优先**——子代理可用时派遣；不可用时 Pro/Flash 自行检索（fallback）

### 2.2 组件全景

```
┌──────────────────────────────────────────────────────────────────┐
│                         Blackboard                                │
│                                                                   │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────┐│
│  │ PlanSection  │ │ ExecSection  │ │ InternalRet  │ │ExternalRet│
│  │ (Pro 写)     │ │ (Flash 写)   │ │ (内部子代理写)│ │(外部子代理)│
│  │              │ │              │ │              │ │          ││
│  │ · goal       │ │ · results[]  │ │ · proj_docs  │ │ · web_src ││
│  │ · steps[]    │ │ · obs[]      │ │ · src_ledger │ │ · src_led ││
│  │ · analysis   │ │ · errors[]   │ │ · response   │ │ · resp    ││
│  │ · decisions  │ │ · auth_req[] │ │              │ │          ││
│  │ · auth_grants│ │              │ │              │ │          ││
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────┘│
│                                                                   │
│  ┌──────────────────────────────────────────────────────────────┐│
│  │ GateLog (Assurance 写)                                       ││
│  │ · gate_decisions[]  · orientation_checks[]                   ││
│  └──────────────────────────────────────────────────────────────┘│
│                                                                   │
│  读取规则：全部区域对所有 Agent 可读（读取不分区）                  │
│  写入规则：每个区域仅一个写入者（写入分区）                         │
└──────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────────┐
│  Pro              │ │  Flash           │ │  检索子代理 (×2)      │
│  DeepSeek V4      │ │  DeepSeek V4     │ │  DeepSeek V4 Flash   │
│                   │ │                  │ │                      │
│  角色: 规划+审查  │ │  角色: 执行+决策 │ │  内部: ProjectDoc    │
│  工具: 全量       │ │  工具: 全量      │ │  外部: WebSearch处理  │
│  派遣子代理       │ │  派遣子代理      │ │                      │
│  最终审查         │ │  独立决策小方向  │ │  配置:               │
│  授权 Flash       │ │  请求授权        │ │  · 独立 API key      │
│                   │ │                  │ │  · 独立 namespace    │
│  配置与 Flash 相同│ │  配置与 Pro 相同 │ │  · 独立黑板区        │
└──────────────────┘ └──────────────────┘ └──────────────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │  Mechanical Relay (150-200行)  │
              │  function.name 精确匹配 → 路由  │
              │  不做正则 / 不做文本解析        │
              └───────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │  Assurance Gates               │
              │  IPG | ToolAvailability        │
              │  Orientation | SourceVisibility│
              │  Pro + Flash 同等适用           │
              └───────────────────────────────┘
```

## 3. Agent Loop Controller

### 3.1 替代 Grok Sampler 的五大组件

Grok Sampler 的 ~10,000 行被替换为五个各司其职的组件：

| 组件 | 行数（估计） | 职责 |
|------|------------|------|
| **PromptBuilder** | ~200 | system prompt + tool defs + injection points (IP2a/b/c) |
| **ModelGateway** | ~150 | 发送请求、流式响应、处理 thinking blocks (IP1)、返回 text + tool_calls |
| **ToolDispatcher** | ~250 | 预执行 IPG (IP3a)、执行、后执行 orientation counters (IP3b)、检索后 sufficiency trigger (IP3c) |
| **OrientationMonitor** | ~150 | should_fire() 检查、注入 checkpoint、冷却管理 |
| **JournalWriter** | ~100 | 记录事件 → hash chain → persist (IP4) |

合计 ~850 行，加上 MechanicalRelay 约 200 行，总计约 **1,050 行**。

### 3.2 主循环

```
while True:
    context = PromptBuilder.build(system_prompt, tool_defs, blackboard)
    response = ModelGateway.stream(context)
    
    if response.has_thinking:
        handle_thinking_blocks(IP1: disabled)  # 仅记录，不进入上下文
    
    if response.is_text_only:
        JournalWriter.record(run_finished)
        break
    
    for tool_call in response.tool_calls:
        route = MechanicalRelay.route(tool_call.function.name)
        
        match route:
            case DispatchToFlash:
                Flash.execute(tool_call)
            case DispatchToSubagent(kind):
                subagent = pick_subagent(kind)  # internal | external
                result = subagent.dispatch(tool_call)
                blackboard.write(subagent.section, result)
            case TriggerGate(gate_type):
                gate_result = assurance.evaluate(gate_type, tool_call)
                blackboard.gate_log.append(gate_result)
            case WritePlan:
                blackboard.plan_section.update(tool_call)
            case Passthrough:
                result = ToolDispatcher.execute(tool_call)
                blackboard.exec_section.append(result)
    
    if OrientationMonitor.should_fire():
        checkpoint = OrientationMonitor.build_checkpoint()
        pending_context_prefix = checkpoint.message_block
        OrientationMonitor.reset_cooldown()
```

## 4. 黑板设计

### 4.1 四个写入分区

| 分区 | 写入者 | 内容 | 读取者 |
|------|--------|------|--------|
| **PlanSection** | Pro | goal, steps[], analysis, decisions, auth_grants | Flash, 子代理, Assurance |
| **ExecSection** | Flash | results[], observations[], errors[], auth_requests[] | Pro, Assurance |
| **InternalRet** | 内部检索子代理 | project_docs, source_ledger, organized_response | Pro, Flash, 外部子代理 |
| **ExternalRet** | 外部检索子代理 | web_sources, source_ledger, organized_response | Pro, Flash, 内部子代理 |
| **GateLog** | Assurance | gate_decisions[], orientation_checks[] | Pro, Flash, 子代理 |

### 4.2 分区规则

```
写入分区：
  - Pro         → PlanSection（仅此）
  - Flash       → ExecSection（仅此）
  - 内部检索子代理 → InternalRet（仅此）
  - 外部检索子代理 → ExternalRet（仅此）
  - Assurance   → GateLog（仅此）

读取不分区：
  - 所有 Agent 可以读取任意分区
  - 读取不需要授权
  - 读取行为本身不记录（只有写入记录）

黑板生命周期：
  - 每个 turn 开始前清空上一轮的 ExecSection
  - PlanSection 跨 turn 保留（但可更新）
  - InternalRet/ExternalRet 跨 turn 保留（但可更新）
  - 会话结束时全部序列化到 journal
```

### 4.3 Rust 类型定义（草图）

```rust
/// 黑板 — 共享结构化状态
struct Blackboard {
    plan: PlanSection,
    execution: ExecSection,
    internal_retrieval: RetrievalSection,
    external_retrieval: RetrievalSection,
    gate_log: Vec<GateEntry>,
}

struct PlanSection {
    goal: String,
    steps: Vec<PlanStep>,
    analysis: String,
    decisions: Vec<Decision>,
    auth_grants: Vec<AuthGrant>,
}

struct ExecSection {
    current_step: Option<usize>,
    results: Vec<ToolResult>,
    observations: Vec<String>,
    errors: Vec<ExecError>,
    auth_requests: Vec<AuthRequest>,
}

struct RetrievalSection {
    query: String,
    sources: Vec<SourceLedgerEntry>,
    organized_response: String,
    opacity_notes: Vec<String>,
    provenance: RetrievalProvenance,  // delegated_retrieval | derived_unverified
}

struct PlanStep {
    id: usize,
    description: String,
    status: StepStatus,  // pending | dispatched | completed | failed
    assigned_to: AgentRole,  // Pro | Flash | InternalRetrieval | ExternalRetrieval
}

enum AgentRole {
    Pro,
    Flash,
    InternalRetrieval,
    ExternalRetrieval,
}
```

## 5. Mechanical Relay（机械传导层）

### 5.1 设计原则

基于 D 项目的教训，机械传导层**不做**：
- ❌ 正则匹配文本标签
- ❌ 解析模型自由文本输出
- ❌ 启发式推断意图

机械传导层**只做**：
- ✅ 匹配 `tool_call.function.name`
- ✅ API 层 `tool_choice: required` 保证模型必走结构化输出
- ✅ strict function calling（beta）保证参数 schema 合规

### 5.2 路由规则

```rust
/// 机械传导路由表 — 纯 function.name 匹配
impl MechanicalRelay {
    fn route(tool_call: &ToolCall) -> RelayAction {
        match tool_call.function.name.as_str() {
            // Pro → 黑板规划区
            "plan_step" | "update_goal" | "record_decision" => {
                RelayAction::WritePlan { entry: tool_call.arguments }
            }
            
            // Pro → 派遣给 Flash
            "delegate_execution" | "dispatch_to_flash" => {
                RelayAction::DispatchToFlash { task: tool_call.arguments }
            }
            
            // Pro/Flash → 派遣检索子代理
            "dispatch_internal_retrieval" => {
                RelayAction::DispatchToSubagent { kind: SubagentKind::Internal }
            }
            "dispatch_external_retrieval" => {
                RelayAction::DispatchToSubagent { kind: SubagentKind::External }
            }
            
            // Pro → 触发 assurance gate
            "verify_result" | "request_audit" => {
                RelayAction::TriggerGate { gate: GateType::from_tool_name(name) }
            }
            
            // Flash → 请求 Pro 授权
            "request_authorization" => {
                RelayAction::WriteExecSection { entry_type: "auth_request" }
            }
            
            // Flash → 写入执行区
            "record_result" | "report_observation" | "report_error" => {
                RelayAction::WriteExecSection { entry_type: "result" }
            }
            
            // 标准工具（Pro 和 Flash 均可调用）
            "read_file" | "write_file" | "edit_file" | "shell_cmd" 
            | "grep" | "glob" | "list_directory" => {
                RelayAction::Passthrough  // 直接执行
            }
            
            // 检索工具（Pro 和 Flash 均可调用 — fallback 模式）
            "web_search" | "web_fetch" => {
                RelayAction::Passthrough  // 子代理不可用时直接检索
            }
            
            // 未识别 → 透传（由 ToolDispatcher 兜底处理）
            _ => RelayAction::Passthrough,
        }
    }
}
```

## 6. Pro 与 Flash

### 6.1 对等设计

| | Pro | Flash |
|---|---|---|
| 模型 | DeepSeek V4 | DeepSeek V4 |
| 工具集 | 全量（相同） | 全量（相同） |
| 检索能力 | ✅（子代理优先） | ✅（子代理优先） |
| 派遣子代理 | ✅ | ✅ |
| 写黑板 | PlanSection | ExecSection |
| 角色 Prompt | "你是规划者..." | "你是执行者..." |
| 独立决策 | 大方向规划 | 小方向自主决策 |
| 授权 | 审查并授权 Flash 请求 | 请求 Pro 授权高风险动作 |
| Assurance gates | ✅ 全部适用 | ✅ 全部适用 |

### 6.2 角色 Prompt 差异

```
Pro System Prompt 关键段落：
  你是 orz Pro — 规划与审查 Agent。
  你的职责：
  1. 理解用户意图，制定执行计划
  2. 将具体执行步骤派遣给 Flash
  3. 审查 Flash 的执行结果和授权请求
  4. 派遣检索子代理获取内/外部信息
  5. 触发 assurance gate 验证关键决策
  6. 最终决定是否通过审查
  
  约束：
  - 不直接执行文件修改和 shell 命令（派遣给 Flash）
  - Flash 请求授权时必须即时响应
  - 所有计划写入黑板 PlanSection

Flash System Prompt 关键段落：
  你是 orz Flash — 执行 Agent。
  你的职责：
  1. 从黑板读取 Pro 的执行计划
  2. 执行具体的工具调用（文件、shell、检索）
  3. 遇到信息不足时自主检索（优先派遣子代理）
  4. 小方向决策自行判断，高风险动作请求 Pro 授权
  5. 所有执行结果写入黑板 ExecSection
  
  约束：
  - 可以自主检索，可以派遣子代理
  - 高风险动作（删除、外部写入、不可逆操作）必须先请求 Pro 授权
  - 不修改黑板 PlanSection
```

### 6.3 Flash 授权流程

```
Flash 遇到高风险动作:
  1. Flash 暂停执行
  2. Flash 写入 ExecSection.auth_requests[]
  3. MechanicalRelay 将请求路由到 Pro 审查队列
  4. Pro 即时审查请求：
     a. 通过 → 写入 PlanSection.auth_grants[]
     b. 拒绝 → 写入 PlanSection.decisions[]，附带拒绝原因
     c. 需要更多信息 → 派遣检索子代理 → 再审查
  5. Flash 读取审查结果，继续或跳过
```

## 7. 检索子代理

### 7.1 内部检索子代理

```
名称: deepseek-retrieval-subagent
凭据: Windows Credential Manager "deepseek-retrieval-subagent"
职责: 扫描项目内部文档，回答关于项目本身的问题

输入: 自然语言查询
工具: 无外部 API — 仅 ProjectDocIndex
  - assurance/**/*.py
  - docs/**/*.md
  - architecture/**/*.md
  - adr/**/*.md
  - protocol/**
  - regression/**
输出:
  - source_ledger: 引用的文件路径 + 摘要
  - organized_response: 结构化回答
  - 标记: delegated_retrieval / derived_unverified
黑板: InternalRet

派遣方: Pro 或 Flash
触发条件:
  - 需要了解项目规范/设计时
  - 需要查找项目内已有实现时
  - 主 Agent 不确定项目内某个约定时
```

### 7.2 外部检索子代理

```
名称: FEP-Agent/DeepSeek-Retrieval
凭据: Windows Credential Manager "FEP-Agent/DeepSeek-Retrieval"
职责: 接收原始 web search 结果，进行结构化处理

输入: 原始搜索摘录 (title/url/snippet)
      注意：子代理不执行搜索——搜索由派遣方执行
工具: 无 — 仅处理传入的搜索摘录
约束:
  - 模型严格限制为仅使用提供的摘录
  - 禁止使用训练知识
  - 禁止编造来源
  - 每条结论必须引用搜索结果编号
输出:
  - source_ledger: 基于真实 URL 的来源清单
  - organized_response: 结构化处理结果
  - 标记: delegated_retrieval / derived_unverified
黑板: ExternalRet

派遣方: Pro 或 Flash
触发条件:
  - 派遣方已通过 web_search 获取原始结果
  - 需要对大量搜索结果进行结构化整理
  - 需要构建带来源引用的分析报告
```

### 7.3 检索优先级

```
决策树（Pro 和 Flash 均适用）:

需要检索信息？
  │
  ├─ 是项目内部文档问题？
  │     ├─ 内部检索子代理可用？
  │     │     ├─ 是 → dispatch_internal_retrieval(query)
  │     │     └─ 否 → fallback: Agent 自行 grep/read_file
  │     │
  ├─ 需要网络搜索？
  │     ├─ 外部检索子代理可用？
  │     │     ├─ 是 → Agent 先 web_search → dispatch_external_retrieval(results)
  │     │     └─ 否 → fallback: Agent 自行处理搜索结果
  │     │
  │     └─ 注意：web_search 本身由 Agent 执行（不经过子代理）
  │           子代理只处理搜索结果的结构化整理
  │
  └─ 简单检索 → Agent 自行检索（不派遣子代理）
```

## 8. 与 Grok 组件的关系

### 8.1 保留的 Grok 组件

| 组件 | 理由 | 修改 |
|------|------|------|
| `orz-tools` | 工具实现成熟，相对独立 | 注入 IPG + orientation counters |
| `orz-workspace` | 文件系统/VCS 操作 | 注入 snapshot |
| `orz-sandbox` | Job Object 隔离 | 不改 |
| `orz-mcp` | MCP 协议支持 | 不改 |
| `orz-config` | 配置解析（需裁剪 remote-settings） | 轻改 |
| `orz-http` | HTTP 客户端基础设施 | 注入 thinking:disabled |
| `orz-markdown` | 终端渲染 | 不改 |
| `orz-secrets` | 凭据脱敏 | 不改 |
| Grok TUI (pager*) | 兜底 UI | 裁剪 voice/update/mermaid |

### 8.2 替换的 Grok 组件

| Grok 组件 | 替换为 | 原因 |
|-----------|--------|------|
| `orz-sampler` (~10,000行) | Agent Loop Controller (~1,050行) | 职责混杂，设计目标不同 |
| `orz-telemetry` (~13,600行) | Journal 系统（自研） | 不发送遥测，只本地记录 |
| `orz-plugin-marketplace` (~5,600行) | No-op stub | 不需要插件市场 |
| `orz-announcements` (~400行) | No-op stub | 不需要公告功能 |

### 8.3 自研的核心

| 组件 | 说明 |
|------|------|
| Agent Loop Controller | ~1,050 行，替代 Grok Sampler |
| Mechanical Relay | ~200 行，纯 function.name 路由 |
| Blackboard | ~300 行，结构化共享状态 |
| Journal 系统 | ~500 行，事件记录 + hash chain + 验证 |
| Assurance Gates | 从 Python 移植（IPG, ToolAvailability, Orientation, SourceVisibility） |
| orz-tui | Assurance workbench TUI |

## 9. 实施顺序

### Phase 2: Journal + Transport + 单 Agent Loop

目标：`orz -p "hello"` → 有效的 `events.jsonl`

1. 实现 journal::event (RunEvent 类型)
2. 实现 journal::chain (SHA-256 hash chain)
3. 实现 journal::recorder (Codex 模式 async channel)
4. 实现 journal::verifier (Gemini 模式 4 维 invariants)
5. 实现 Agent Loop Controller（单 Agent 版本，先不上双模型）
6. IP1: HTTP transport `thinking: disabled` 注入
7. IP4a/IP4c: session lifecycle journal events
8. 验证: journal hash chain 连续，Python verifier 可独立验证

### Phase 3: 双 Agent + 黑板 + 检索子代理 + Gates

目标：Pro + Flash + 检索子代理完整协作

1. 实现 Blackboard（4 分区）
2. 实现 Mechanical Relay（纯 function.name 路由）
3. 实现 Pro/Flash 双 Agent（对等配置，角色 Prompt 不同）
4. 实现 Flash 授权流程
5. 移植内部检索子代理（ProjectDocIndex → Rust）
6. 移植外部检索子代理（WebSearch 处理 → Rust）
7. 实现 IP2/IP3（prompt + tool dispatch 注入）
8. 实现 OrientationMonitor（事件驱动触发）
9. 验证: 完整双 Agent + 子代理 + gate 链通过

### Phase 4: TUI + Snapshot + Polish

目标：完整产品

1. 从 Python `assurance/tui/` 移植 TUI → `orz-tui`
2. 实现 session::snapshot（OpenCode 模式 shadow Git）
3. 实现 sandbox::job_object, credential, permit
4. Grok TUI 兜底裁剪（voice/update/mermaid）
5. Python 项目标记 `reference-spec`
6. 验证: orz 全功能通过 Python conformance suite

## 10. 从 Grok Sampler 砍掉的功能

| Grok Sampler 功能 | 处理方式 |
|-------------------|---------|
| 多模型切换 | → 配置驱动，Pro/Flash 各用 DeepSeek V4 |
| goal 选择 | → Pro 在 PlanSection 中规划 |
| memory_dream 触发 | → 保留但默认关闭（已有设计） |
| recap 生成 | → compaction 时由模型自行完成 |
| prompt_build | → PromptBuilder，注入 assurance blocks |
| 云端配置下发 | → 已删除 |
| 遥测打点 | → Journal 替代（本地，不外发） |
| 模型采样参数 | → ModelGateway 中简单配置 |

## 11. 参考

- [`FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md`](FORK_ARCHITECTURE_AND_DESIGN_LANGUAGE_v0.3.md) — 五源融合设计
- [`FORK_IMPLEMENTATION_DESIGN_v0.1.md`](FORK_IMPLEMENTATION_DESIGN_v0.1.md) — 注入点精确位置
- [`PHASE2_POST_PLANA_REVIEW_v0.1.md`](../../../architecture/PHASE2_POST_PLANA_REVIEW_v0.1.md) — 审查发现
- [`PHASE2_COMPILATION_STATUS_v0.2.md`](../../../architecture/PHASE2_COMPILATION_STATUS_v0.2.md) — 编译修复记录
- [`GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md`](../../../architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md) — 检索子代理设计与 GAK 登记
- [`IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`](IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md) — 偏差分析
- D 项目存档：`G:\我的云端硬盘\VSCode_Copilot_Archives\` — 黑板/蜂群传导/硬字段校验参考实现
- Codex CLI: `codex-rs/rollout/src/recorder.rs` — JournalRecorder 参考
- Aider: Architect/Editor 双模型分离 — Pro/Flash 角色分离参考
- Goose: Provider trait + 安全审查管线 — 工具调度参考
