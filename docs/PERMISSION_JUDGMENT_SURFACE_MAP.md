# ORZ 权限判定面单图（Permission Judgment Surface Map）

> **状态**：`reference`（判定面事实登记，权威版 v1.0，2026-09-07）。
> **上级**：OBS-PERMISSION-DUAL-IMPL 处置（α 方案，2026-09-07 用户裁决）/
> 设计与裁决记录
> [`PERMISSION_DUAL_IMPL_CONVERGENCE_DESIGN_2026-09-07`](PERMISSION_DUAL_IMPL_CONVERGENCE_DESIGN_2026-09-07.md)。
> **维护纪律**：任何权限判定面变更（新增/搬移/休眠/复活）必须同步本图；
> 本图与源码头注 ACTIVE/DORMANT/PARTIAL 标注一一对应，grep 双向可核。

## 1. 治理原则（用户裁决 2026-09-07）

1. **薄层边界是特意形态**：桥（orz-host PermissionBridge）独有判定段与
   上游血统引擎（orz-workspace 权限栈）的分工是架构意图，**不合并、不
   收敛**（β/γ 方向已否决：自研面不膨胀、不做深度融合）。
2. **自研面不膨胀**：ORZ 自研判定面维持现状；任何新增判定能力须显式
   立项裁决。
3. **休眠面冻结**：ORZ 生产不接线的血统能力冻结不演进、不新增接线；
   复活须显式立项。
4. **每类判定恰好一个演进 owner**（见 §4 唯一性声明）。

原则锚点：FUS-CORE（成熟组件优先、控制面边界由 ADR 裁决）、
FUS-DSH-BORROW-REVIEW（「orz 整体即薄层」）、ADR-0010 §14.60
（自研面不膨胀，α 批转录）。

## 2. 生产装配路径（判定面的总线）

```text
ORZ 生产入口（orz acp / CLI run / TUI）
  └ orz-host PermissionBridge（orz-host/src/permission.rs）
      ├ 桥独有判定段（薄层 owner，见 §3.2）
      └ spawn_permission_manager_with_hub ──→ orz-workspace 权限栈
          （manager actor：tokio spawn_local，读面 auto-allow、
           bash/edit 分析、提示、持久化；见 §3.3）
  呈现车道：ACP gateway（AcpPrompter）/ codex hub（PermissionHookTransport
  —— codex_permission.rs 实现、acp_server.rs/lib.rs 引用）
```

注：orz-host session.rs 的 17 处 `orz_workspace::` 消费为
folder_trust（信任门，非权限判定面），不在本图范围内，仅登记相邻关系。

## 3. 判定面权威登记

### 3.1 orz-tools 工具沙箱（读面单源，P0-0m）

| 判定面 | owner | 模式 | 备注 |
|---|---|---|---|
| read_file/grep/list_dir 三分判定 | `orz-tools resources::is_path_allowed_for_read` | ACTIVE | 技能根豁免→会话卷域→workspace→拒 |
| `.gsa` 窗口契约 | `resources::is_session_volume_window_path` | ACTIVE | 仅两窗口，双条件 |
| D1 卷根解析 | `resources::session_volume_canonical_root` | ACTIVE | 桥与工具沙箱共用 |

### 3.2 桥独有判定段（orz-host 薄层 owner）

| 判定面 | owner | 模式 | 备注 |
|---|---|---|---|
| 工具→AccessKind 分类 | bridge `access_kind` | ACTIVE | |
| 读面 scope（`.gsa` 镜像冻结 + cwd 外可读） | bridge `access_in_scope` | ACTIVE | `.gsa` 段 RETIRED-IN-PLACE（P0-0m D5），语义 owner=orz-tools |
| 策略门 ReadOnly / Benchmark{shell,network} / Interactive | bridge `request` | ACTIVE | MCP 前缀伪装防拒（design review D2-1） |
| 提示超时 fail-closed | bridge `PERMISSION_PROMPT_TIMEOUT` 300s | ACTIVE | orz-tui 倒计时共享单一来源 |

### 3.3 orz-workspace 引擎面（血统 owner；模块级分类 = S1 核证）

**ACTIVE**（ORZ 生产可达且参与判定）：

| 模块 | 证据（manager.rs 行号，生产区 <2368） |
|---|---|
| manager.rs（actor 本体，8,756 行） | spawn_local 装配 1290+ |
| types.rs | bridge 直用 + manager:26 |
| prompter.rs（bash/edit 提示链） | AcpPrompter 构造 1322 |
| hub_permission.rs（codex hub 车道） | PermissionHookTransport（codex_permission.rs:115 实现） |
| bash_command_splitting.rs | try_parse_shell 568 / 580 / 617 / 630 |
| exec_risk.rs | ambient_exec_risk_from_plan 1537 |
| shell_access.rs | is_safe_write_sink 582 / edit_target_protection 1572 |
| gate_preflight.rs | GatePreflight::evaluate 1588 |

**PARTIAL**（模块内活跃/休眠分居；标注到函数粒度）：

| 模块 | 活跃部分 | 休眠部分 |
|---|---|---|
| auto_mode.rs | `script_env_risk`/`EnvRisk`（manager:584，env 风险分析） | classifier/fast-path 家族（manager:1680 装配，ORZ auto mode 恒不启用） |
| policy.rs | `ShellWord`（manager:618，bash 词法） | `CompiledPolicy`/`evaluate_policy`（manager:1347，仅 permission_config Some；ORZ 传 None） |
| state.rs | `load_state_from_disk` 1292 + edit_policy 迁移 persist 1317 | 审批持久化 persist 1403/2130（ORZ remember_tool_approvals=false） |
| resolution.rs | `yolo_disabled_by_policy` 单点（manager:1238，读 orz_config requirements layers） | 规则解析引擎主体（消费方 discovery.rs 为 Grok Build 面，ORZ 不可达） |
| claude_settings.rs | `project_claude_settings_present`（folder_trust 391 → ORZ session trust 路径） | 权限规则/env 导入主体（rules 解析不触发；present 仅查文件存在） |
| prompter.rs（内注） | AcpPrompter 主链 | MCP 命名辅助家族（无 MCP 配置） |

**DORMANT**（ORZ 生产不可达）：

| 模块 | 证据 |
|---|---|
| rules.rs | `parse_permission_rule` 生产消费方仅 claude_settings 规则导入（ORZ 不可达）；manager:4704 / policy.rs:935/951 均测试区 |

### 3.4 相邻面（登记边界，非本 OBS 范围）

- **ACAF 票据门**（orz-loop acaf_flow）：控制/动作票据的签发验证与
  fail-closed——授权链第三面，与权限判定正交（权限允诺 ≠ 票据放行）。
- **folder_trust / trust**（orz-workspace，session.rs 消费）：会话信任门。
- **ork-tools 写面**（search_replace 写前核证 / 写域 deny-only）：写授权由
  助理层订单链承担，不在读面沙箱范围。

## 4. 唯一 owner 声明

| 判定类别 | 唯一演进 owner | 冻结/镜像 |
|---|---|---|
| 读面沙箱（workspace/会话卷/窗口） | orz-tools `resources` | permission.rs `.gsa` 段 = 冻结镜像（RETIRED-IN-PLACE） |
| 会话卷根解析 | orz-tools `session_volume_canonical_root` | 桥与沙箱共用同一函数 |
| 工具分类 / scope / 策略门 / 提示超时 | orz-host 桥判定段（薄层 owner） | 不下沉、不合并 |
| 读面 auto-allow / bash-edit 分析 / 交互呈现 | orz-workspace 引擎（血统 owner） | 休眠模块不演进（§3.3 PARTIAL/DORMANT） |
| deny_read_globs | manager 计算 → `DenyReadGlobs` 资源 → orz-tools 消费 | 单通道，无双判定 |
| ACAF 授权链 | orz-loop acaf_flow | 独立第三面（ADR-0011） |
