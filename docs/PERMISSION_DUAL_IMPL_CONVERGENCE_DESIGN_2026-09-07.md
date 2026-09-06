# 权限判定双实现收敛——摸底与方向讨论稿（2026-09-07）

> **状态**：讨论稿 / 立项候选（`reference` → 待用户裁决方向后转正式设计）。
> **上级**：OBS-PERMISSION-DUAL-IMPL（GLM 外部审查 (c)，2026-09-06 登记
> 「随终局治理视野排期」）；本稿为其推进的摸底与方向提案，**不含实施**。
> **触发**：2026-09-07 用户指示处理两项遗留观察；本项为其中之一。

## 1. 摸底结论（对登记表述的精确化）

OBS 登记原文为「移植层 manager.rs（8,756 行）与自研 permission.rs（1,331 行）
双处承担权限判定」。摸底后精确化：**这不是两套独立判定器，而是一套引擎 +
一个桥 + 一段桥独有判定**——

### 1.1 真实结构

```text
生产入口（orz acp / CLI run / TUI）
    └─ orz-host PermissionBridge（1,347 行，生产逻辑 ≈555 行 + 测试 ≈790 行）
         ├─ 桥独有判定段（自研，~200 行）：
         │    access_kind 工具→AccessKind 分类
         │    access_in_scope 读面 scope + `.gsa` 镜像（RETIRED-IN-PLACE）
         │    policy 门（Interactive / ReadOnly / Benchmark allow_shell/network）
         │    PERMISSION_PROMPT_TIMEOUT 300s 提示超时 fail-closed
         ├─ spawn_permission_manager_with_hub ──→ orz-workspace 权限栈（引擎）
         │    manager actor（会话状态 / Read,Grep auto-allow / denial 通道 /
         │    deny_read_globs 计算 / yolo / auto-mode / classifier …）
         │    resolution 4,395 · auto_mode 3,198 · shell_access 3,146 ·
         │    bash_command_splitting 1,887 · prompter 1,698 · policy 1,488 ·
         │    state 805 · exec_risk 829 · types 684 · claude_settings 564 ·
         │    hub_permission 497 · rules 302 · gate_preflight 197 …
         │    （合计 29,842 行）
         └─ 消费方：session.rs（17 处 orz_workspace::permission）、
              codex_permission.rs / acp_server / lib.rs（PermissionHookTransport）
```

### 1.2 ORZ 生产接线面 vs 休眠面

ORZ 生产经 bridge 装配引擎时**大量能力显式不接线**：
`managed rules = None`、`remember_tool_approvals = false`、headless Ask →
Deny、yolo / auto-mode / classifier / claude_settings 导入 / bash 命令拆分 /
allowlist 持久化均为 Grok Build 血统能力，ORZ 生产不启用。即：29,842 行栈中
生产活跃的仅是 manager actor 核心（prompt 传输、Read/Grep auto-allow、
denial 事件、deny_read_globs 下发）+ prompter 的呈现通道；其余为编译进依赖
链的休眠面——这正是 OBS「复杂度与审计盲区」的实体。

### 1.3 判定语义分布（收敛对象清单）

| 判定面 | 现 owner | 状态 |
|---|---|---|
| `.gsa` 窗口契约 | orz-tools 沙箱（P0-0m S1） | ✅ 已单源（镜像冻结） |
| D1 卷根解析 | `resources::session_volume_canonical_root` | ✅ 已单源 |
| deny_read_globs | manager 计算 → DenyReadGlobs 资源 → orz-tools 消费 | ✅ 单通道 |
| 读面 scope（cwd 外可读 + `.gsa` 不可见） | bridge `access_in_scope` | 桥独有，与工具沙箱分层 |
| 工具→AccessKind 分类 | bridge `access_kind` | 桥独有 |
| 策略门（ReadOnly/Benchmark/Interactive） | bridge `request` | 桥独有 |
| 提示超时 fail-closed | bridge（300s） | 桥独有 |
| Read/Grep auto-allow、拒绝规则 | workspace 引擎 | 引擎 |
| 交互呈现 | workspace prompter + hub/ACP 车道 | 引擎+车道 |
| ACAF 票据门 | orz-loop acaf_flow（独立第三面） | 不在本 OBS 范围 |

## 2. 收敛方向选项（待裁决）

### 方向 α——判定面单图 + 休眠面冻结标注（文档批，不动代码）

落一张「哪里判什么」的判定面单图（§1.3 扩展为权威登记），桥独有判定段与
引擎职责各写明唯一性声明；休眠模块在 workspace 栈源码处统一 DORMANT 标注
（不演进、不新增接线）。成本最小（一个文档批），消除审计盲区的认知部分；
不消除代码体量，OBS 保持开放但降级为「已管控」。

### 方向 β——桥内聚 + 休眠面裁剪（中等工程，设计先行）

把桥独有判定段（access_kind / access_in_scope / policy 门 / 超时）与引擎
的对应规则合并到单一 owner（方案细化时裁决：下沉引擎 or 上收桥）；对 ORZ
生产不接线的休眠模块以 feature gate / crate 拆分方式移出 ORZ 依赖链
（workspace 栈保留给 Grok Build 血统上游同步）。预计显著缩小审计面与编译
面；需要接口设计 + 全量回归 + 与上游血统策略（ORZ 是否长期跟随上游）对表。

### 方向 γ——自研单一权限内核（大工程，不推荐当下）

以 orz-host 为基础重写最小权限内核，彻底退役 workspace 权限栈。8.7k 行
行为等价迁移风险高，且与「ORZ 融合架构 = 成熟组件优先（FUS-CORE）」及上游
血统持续移植策略冲突。仅当上游血统策略裁决为「冻结上游、停止跟随」时再
评估。

### 推荐

**α 立即做（可并入本批后续文档批或 S4 收口）；β 登记为终局治理正式立项
（S1 设计定稿先行、实施单独放行）；γ 不排期、仅作上游策略裁决的联动项。**

## 3. 待用户裁决点

1. 方向选择：α / β（α+β 分期）/ γ。
2. 若 β：单一 owner 放桥（上收）还是放引擎（下沉）；休眠面裁剪形态
   （feature gate vs crate 拆分）。
3. OBS-PERMISSION-DUAL-IMPL 条目状态更新口径（α 后改「已管控」或维持
   `reference` 观察）。
