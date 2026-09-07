# 权限判定面分布治理——摸底、裁决与 α 方案设计（2026-09-07）

> **状态**：α 方案设计定稿，**实施待用户确认**（本批零代码、零标注改动；
> 仅有本设计文档 + BACKLOG/TODO 裁决登记）。
> **上级**：OBS-PERMISSION-DUAL-IMPL（GLM 外部审查 (c)，2026-09-06 登记
> 「随终局治理视野排期」）。
> **触发**：2026-09-07 用户指示处理两项遗留观察 → 本项摸底 → 同日用户
> 裁决方向。
> **版本**：v1.0 讨论稿（α/β/γ 提案）→ v1.1 裁决定稿（α 定案、β/γ 否决、
> α 细化为实施设计）。

## 0. 用户裁决（2026-09-07，逐字口径登记）

1. **方向 = α**（判定面单图 + 休眠面冻结标注，文档批）。
2. **β / γ 否决**，理由（用户架构口径）：
   - 自研部分已经足够大，项目已近两个月，**自研面不能再膨胀**；
   - ORZ 设计本身**不考虑跟随 Grok 版本、不做深度融合，只做薄层**；
   - β / γ 两个方案的**融合度都太高**；
   - **现在的桥接就是特意选择的方式**——桥 = 薄层边界，是架构意图，
     不作为待收敛缺陷。
3. 设计哲学对表：上述口径与既有登记一致——FUS-CORE（成熟组件优先、
   控制面边界由 ADR 裁决）、FUS-DSH-BORROW-REVIEW（「orz 整体即薄层：
   单一二进制、底座可较简单切换」）。「自研面不膨胀」为本口径新增的
   显式表述，α 批落地时一并写入判定面单图作为治理原则。

### 0.1 风险重定性（裁决推论）

OBS 原登记前提（「双实现 = 复杂度与审计盲区风险，需收敛」）按裁决修正为：
**判定语义分布（桥独有判定段 + 上游血统引擎）是薄层架构的特意形态，
不是缺陷**；残余风险收窄为两项并交由 α 管控——
- 休眠面体量：workspace 栈 29,842 行中 ORZ 生产不接线的部分无显式标注，
  审计时无法机械区分「活跃判定」与「休眠血统」；
- 认知盲区：判定语义的 owner 分布没有一张权威单图，跨层追溯靠口口相传。
β/γ 所针对的「体量收敛」目标随裁决撤销——**α 落地即本 OBS 的终态**
（状态改「已管控」），不存在后续收敛工程。

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

### 1.3 判定语义分布（判定面单图底稿，α-S2 扩展为权威版）

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

## 2. 方向处置（v1.1 裁决后存档）

- **α（判定面单图 + 休眠面冻结标注）——定案**，细化为实施设计见 §3。
- **β（桥内聚 + 休眠面裁剪）——否决**：判定段合并与依赖链裁剪均属深度
  融合改造，与「薄层边界是特意形态、自研面不膨胀」裁决直接冲突；不再
  作为立项候选保留。
- **γ（自研单一权限内核）——否决**：自研面膨胀上限的直接违反；不排期、
  不作为任何上游策略下的联动项保留。

## 3. α 实施设计（待用户确认后实施；本节即确认对象）

α 是**纯文档 + 源码注释批**：零行为变更、零模块搬移/删除、零接口变更。
范围三项产出 + 一项登记收口，切为三个最小可验收步。

### S1 判定面可达性核证（标注的事实依据先行）

对 workspace 权限栈 15 模块逐一做调用可达性核证（从 ORZ 生产装配面
`PermissionBridge::spawn → spawn_permission_manager_with_hub` 与
`session.rs` 的 17 处 `orz_workspace::permission` 消费出发，grep +
调用链核对），产出三档分类表：
- **ACTIVE**——ORZ 生产路径可达（如 manager actor 核心 / prompter 呈现 /
  deny_read_globs 下发）；
- **DORMANT**——ORZ 生产不可达（预判：auto_mode、bash_command_splitting、
  claude_settings、allowlist 持久化、yolo/classifier 装配面；以核证为准，
  本设计不预写结论）；
- **PARTIAL**——模块内部分函数活跃（如 resolution/rules 的默认规则路径），
  标注到函数/区段粒度。
验收：分类表逐模块带证据（调用点 file:line），无「凭名册判断」。

### S2 判定面单图落盘（权威登记）

- 落点：`docs/PERMISSION_JUDGMENT_SURFACE_MAP.md`（`reference` 路由；
  不入 `architecture/current/`——单图是事实面登记，非 ADR 派生投影）。
- 内容 = 本稿 §1.3 表扩展为权威版，每行含：判定面 / owner 模块 / 车道与
  入口 / 消费方 / ACTIVE-DORMANT-PARTIAL 模式 / 单源状态 / 边界声明。
  覆盖六大块：orz-tools 工具沙箱（含 `.gsa` 窗口契约 + D1 单源）、桥独有
  判定段（薄层 owner）、workspace 引擎面、ACAF 票据门（orz-loop，登记为
  相邻第三面并声明范围边界）、deny_read_globs 单通道、权限呈现双车道
  （ACP gateway / codex hub）。
- 开头登记治理原则（用户裁决 2026-09-07）：**薄层边界特意形态、判定分布
  不合并、自研面不膨胀、休眠面复活须显式立项**；并引 FUS-CORE /
  FUS-DSH-BORROW-REVIEW 作既有原则锚点。
- 桥独有判定段与引擎各写唯一性声明（每类判定恰好一个演进 owner：
  桥段 owner = orz-host 薄层，引擎 owner = workspace 栈；`.gsa` owner =
  orz-tools 沙箱，permission.rs 段为冻结镜像——沿 P0-0m D5 口径）。
- CLI_PROJECT_INDEX 增加 OBS 条目对该单图的入口指针（召回路由变更，
  按 §0.5 检查流程走）。

### S3 休眠面 DORMANT 标注 + OBS 收口

- 按 S1 分类表，对 DORMANT 模块（PARTIAL 到函数/区段粒度）在源码模块头
  追加统一注记，模板：
  ```text
  DORMANT（ORZ 生产面休眠，OBS α 批 2026-09-07）：本模块属 orz-workspace
  上游血统权限栈；ORZ 生产装配面未接线——<S1 核证的接线事实一行>。
  冻结不演进、不新增 ORZ 接线；复活须显式立项（自研面不膨胀，薄层边界
  与判定面单图见 docs/PERMISSION_JUDGMENT_SURFACE_MAP.md）。
  ```
- ACTIVE 模块头部加一行 ACTIVE 声明（指向单图），双面可 grep 区分。
- OBS-PERMISSION-DUAL-IMPL 条目更新：状态 → 「已管控（薄层架构特意形态 +
  判定面单图登记 + 休眠面冻结标注）」，入口增单图与本设计；索引/BACKLOG/
  TODO 同步；manifest 重算、门禁 Exit 0。

### 验收口径（α 批整体）

1. S1 分类表覆盖 workspace permission 栈全部 15 模块，逐模块有调用点证据；
2. 单图六大块齐备且与 S1 表、P0-0m D5 口径、ACAF 范围声明无冲突；
3. DORMANT/ACTIVE 标注与分类表一一对应（grep 双向核验：标注数 = 表行数）；
4. 全程零行为变更（`cargo check`/既有测试面零变化、orz-host 249/0/4 保持）；
5. 门禁 Exit 0 + manifest 重算；索引/BACKLOG/TODO 三处同步。

## 4. 待确认点（确认后即实施；均为文档粒度小决策）

1. **单图落点**：`docs/PERMISSION_JUDGMENT_SURFACE_MAP.md`（推荐）或并入
   本设计文档作一章（少一个文件，但单图会随演化膨胀，独立文件利于维护）。
2. **PARTIAL 粒度**：模块头标注为主、函数/区段级标注为辅（推荐），还是
   一律只到模块级（省事但 ACTIVE/DORMANT 混居模块会失真）。
3. **DORMANT 注记语言**：与仓库既有注释惯例一致用中文（推荐）。
4. **「自研面不膨胀」原则是否随 α 批转录 ADR-0010 §14.x**（推荐：转录——
   这是用户裁决级口径，与既有 §14.55/§14.56 同级的治理原则，落为 §14.60；
   不转录则仅在单图登记）。
5. α 批实施时机：紧随确认（推荐，纯文档批约一个批次的量）或并入 P0-0m
   S3/S4 收口批顺带执行。
