# GAK-SBX-001 Windows Native Sandbox audit（2026-07-27）

## 裁决

新增 Windows Native Strict Sandbox 的 development 纵向切片，包括 AppContainer 探针、Job Object 进程 containment
和独立 verifier。合同层与 **live probe** 均已落地。

2026-07-27 本机 live probe（Windows 11 10.0.26200，非 elevated）观测到：
AppContainer token、Job Object、非管理员、System32 写阻断、workspace/temp 写、HKLM 写阻断均 **pass**；
**原始 Win32 TCP 出站（1.1.1.1:443）仍通** → `network_connect_blocked=false` → outcome **noncompliant**。
因此 `windows_native_strict` selection 仍 fail-closed，**不得**标为 production-ready 或关闭 GAK-SBX-001。

## 问题背景

GAK-SBX-001 登记为"阻断"级别：默认物理 sandbox 未完全闭环。
P2 Docker branch 已 observed compliant，但 Windows native strict backend 只停留在 Job Object 进程 containment，
缺少 restricted token/AppContainer、文件系统/注册表和完整网络边界。无 Docker 的 Windows 用户无法进入 strict 模式。

本轮目标不是立即推出生产级 native sandbox，而是先建立完整的验收链，使 Windows native backend 具备：
- 可测量（有 schema 定义的 observation 结构）
- 可验证（有独立 verifier 做交叉检查）
- 可比较（与 Docker candidate 使用同一 selection 框架）
- 可追溯（profile、digest、limitations 闭环）

## 新增组件

### 1. Windows Native Sandbox Profile

**`windows-native-sandbox-profile-v0.1.json`** + **`.schema.json`**：

冻结 AppContainer-based strict sandbox 的期望配置：
- identity: `appcontainer_disposable`，`elevation: never`
- filesystem: `workspace_only_writable`，禁止写入 system32/program_files/windows/host_home/credential_store
- network: `mode: none`，appcontainer_capabilities 为空（无 internetClient/internetServer）
- process: no-shell，job_object_kill_on_close，creation via `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`
- resources: 256 MiB memory, 30s wall time, 64 PID limit
- lifecycle: `cleanup_required`，`disposable_profile`

### 2. Windows Native Sandbox Observation

**`windows-native-sandbox-observation-v0.1.schema.json`**：

冻结 probe observation 的数据结构，包含三层：
- **appcontainer**：`sid_derived`（是否成功获得 AppContainer SID）、`profile_created`/`profile_deleted`（管理员级 profile 生命周期）、`capabilities`（必须为空列表）
- **job_object**：`created`/`assigned`/`kill_on_close`/`memory_limit_bytes`
- **checks**：7 项 boolean 检查——`non_admin`、`system32_write_blocked`、`workspace_write_succeeded`、`temp_write_succeeded`、`network_connect_blocked`、`registry_protected_blocked`、`probe_file_cleaned`

若 `outcome: compliant`，则所有 7 项 checks 必须为 `true`（schema 级 enforced via `allOf`/`if`/`then`）。

### 3. Windows Sandbox 实现模块

**`assurance/windows_sandbox.py`**（~520 行）：

核心函数：

- **`run_windows_native_sandbox_probe(workspace)`**：主探针入口，完整流程：
  1. 加载并验证 profile
  2. 验证 disposable workspace marker
  3. 调用 `DeriveAppContainerSidFromAppContainerName` 获取 AppContainer SID（不需要管理员）
  4. 尝试 `CreateAppContainerProfile`（需要管理员）→ 失败时标记 degraded，仍继续
  5. 创建 kill-on-close Job Object 并设置 memory limit
  6. 使用 `InitializeProcThreadAttributeList` + `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` 设置 AppContainer
  7. 通过 `CreateProcessW` + `EXTENDED_STARTUPINFO_PRESENT` 启动 no-shell Python 探针进程
  8. `AssignProcessToJobObject` 关联 Job Object
  9. 等待进程退出（带 timeout），超时则 kill Job Object
  10. 解析探针输出的 JSON 检查结果
  11. 清理：删除 AppContainer profile（如已创建）、释放 SID、清理 workspace probe file
  12. 构建 observation 并执行独立 verifier 交叉检查

- **`windows_native_candidate_from_observation(observation)`**：从 observation 构建 backend candidate，与 Docker 的 `docker_candidate_from_observation` 对等

cTypes Win32 API 绑定：
- `DeriveAppContainerSidFromAppContainerName`（userenv.dll）
- `CreateAppContainerProfile` / `DeleteAppContainerProfile`（userenv.dll）
- `FreeSid`（advapi32.dll）
- `CreateJobObjectW` / `SetInformationJobObject` / `AssignProcessToJobObject`（kernel32.dll）
- `InitializeProcThreadAttributeList` / `UpdateProcThreadAttribute` / `DeleteProcThreadAttributeList`（kernel32.dll）
- `CreateProcessW`（kernel32.dll）
- `CreatePipe` / `SetHandleInformation` / `ReadFile`（kernel32.dll）
- `WaitForSingleObject` / `GetExitCodeProcess`（kernel32.dll）

探针脚本（`PROBE_SCRIPT`）在 AppContainer 内执行 7 项检查：
1. non_admin：通过 `CheckTokenMembership` 验证不在 Administrators 组
2. system32_write_blocked：尝试写入 `%SystemRoot%\System32`（AppContainer 应被阻止）
3. workspace_write_succeeded：通过 `P2_WORKSPACE` 环境变量获取 workspace 路径并写入
4. temp_write_succeeded：使用 `tempfile.NamedTemporaryFile` 测试临时文件创建
5. network_connect_blocked：尝试连接 `1.1.1.1:443`（无 internetClient 能力应失败）
6. registry_protected_blocked：尝试创建 `HKLM\SOFTWARE\_p2_w32_probe_del` 注册表项
7. probe_file_cleaned：由宿主在清理阶段填充

### 4. Sandbox Verifier 扩展

**`sandbox_verifier.py`** 新增 `verify_windows_native_observation()`：

独立验证 observer 层，检查：
- profile 和 observation schema 合规
- profile digest 匹配
- AppContainer SID 已 derive 或 profile 已创建
- 若 profile 已创建，必须已删除（`profile_created → profile_deleted`）
- AppContainer capabilities 必须为空（网络隔离）
- Job Object 已创建且 assigned
- Job Object memory limit 匹配 profile
- Job Object 为 kill-on-close
- `shell_used` 必须为 false
- 7 项 checks 的 key 集合完整
- 除 `probe_file_cleaned` 外的所有 checks 均为 true（compliant 前提）
- 若 `outcome: compliant` 但 controls 不满足 → 拒绝（防止 overstated compliance）

### 5. Backend Selection 集成

**`sandbox.py`** 中 `windows_native_strict_candidate()` 更新为产生合理的 noncompliant candidate：
- 非 Windows 平台：`unavailable` + `platform_not_windows`
- Windows 平台：`available` + `noncompliant` + `appcontainer_probe_not_yet_run`

在 probe 运行并通过后，可调用 `windows_native_candidate_from_observation()` 产生 compliant candidate，
与 Docker candidate 一起传入 `build_sandbox_selection_receipt()`，由同一 selection 框架处理。

## 文件清单

### 新增文件
| 文件 | 用途 |
|---|---|
| `assurance/windows-native-sandbox-profile-v0.1.json` | AppContainer + Job Object 沙箱 profile 数据 |
| `assurance/windows-native-sandbox-profile-v0.1.schema.json` | Profile schema contract |
| `assurance/windows-native-sandbox-observation-v0.1.schema.json` | Observation schema contract |
| `assurance/windows_sandbox.py` | 核心模块：AppContainer/Job Object 探针 + candidate builder |
| `assurance/tests/test_windows_sandbox.py` | 15 个测试（schema、verifier、candidate） |
| `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md` | 本审计文档 |

### 修改文件
| 文件 | 变更 |
|---|---|
| `assurance/sandbox.py` | 更新 `windows_native_strict_candidate()`，使用实际 reason code |
| `assurance/sandbox_verifier.py` | 新增 `verify_windows_native_observation()`（~50 行） |
| `assurance/__init__.py` | 新增 3 个导出符号 |

## 测试覆盖

### Schema 层（2 个测试）
- profile JSON 通过自身 schema 验证
- forbidden_paths 不足 5 个时 schema 拒绝

### Observation Schema 层（2 个测试）
- 合法 observation JSON 通过 schema 验证
- `outcome: compliant` 但不满足所有 checks 时 schema 拒绝（via `allOf`/`if`/`then`）

### Verifier 层（7 个测试）
- `sid_derived=false` 且 `profile_created=false` → 拒绝
- `job_object.created=false` → 拒绝
- `job_object.assigned=false` → 拒绝
- `shell_used=true` → 拒绝
- `checks.network_connect_blocked=false` → 拒绝
- `profile_created=true` 且 `profile_deleted=false` → 拒绝（未清理资源）
- `capabilities=["internetClient"]` → 拒绝（网络未隔离）
- `outcome: compliant` 但 controls 不满足 → 拒绝（overstated）

### Candidate 层（3 个测试）
- compliant observation → compliant candidate
- noncompliant observation → noncompliant candidate（含 rejection_reasons）
- `windows_native_strict_candidate()` 在 Windows 上返回 `noncompliant` + `windows_native_live_observation_required`
- Live probe 测试：`WindowsNativeLiveProbeTests`（仅 Windows）

## Live probe 实测（2026-07-27，Windows 11 10.0.26200，非 elevated）

### 运行方式

```text
python scripts/run_windows_native_sandbox_probe.py
# 或：python -m pytest assurance/tests/test_windows_sandbox.py -k LiveProbe
```

### 观测摘要

| 控制项 | 结果 |
|---|---|
| `CreateAppContainerProfile` | 成功（无需 elevation） |
| `TokenIsAppContainer`（挂起创建后、Resume 前） | true（fail-closed 若为 false） |
| Job Object create/assign/kill-on-close | true |
| `non_admin` | true |
| `system32_write_blocked` | true |
| `workspace_write_succeeded` | true（icacls `(OI)(CI)(M)`） |
| `temp_write_succeeded` | true |
| `registry_protected_blocked` | true |
| `probe_file_cleaned` | true |
| `network_connect_blocked`（Win32 TcpClient → 1.1.1.1:443） | **false** |
| `outcome` | **noncompliant** |
| selection `windows_native_strict` | 不 allow |

### 同日探针修复

1. **UTF-8 BOM**：`Out-File -Encoding utf8` 导致主机 JSON 解析失败并默认为全 false；改为无 BOM 写入并容忍 BOM 读取。
2. **ACL**：`(RX,W)` 不足；改为 `(OI)(CI)(M)`。
3. **GAK-WIN-001**：`CREATE_SUSPENDED` → TokenIsAppContainer → Job assign → ResumeThread。
4. **Verifier**：noncompliant + `require_compliant=false` 时结构 valid，仅 `controls_compliant=false`。

### 网络残留

空 capability AppContainer 下 raw TCP 仍可能出站；HTTP 高层 API 往往超时。探针使用 TcpClient 作为诚实下界，不把 HTTP 超时写成“已隔离”。与 Docker `network=none` 不等价。

## 与现有架构的关系

| 组件 | 关系 |
|---|---|
| Docker sandbox（sandbox.py） | 共享 profile → observation → candidate → selection 四层模式 |
| sandbox_verifier.py | 共用 `verify_sandbox_selection_receipt()`，各自独立 observation verifier |
| assurance/__init__.py | 与 Docker 探针并列导出，由 selection receipt 统一路由 |
| gap register（GAK-SBX-001） | live 已跑；因网络残留保持阻断 |

## 评估与限制

### 优点
- schema → implementation → verifier → **live probe** 闭环
- AppContainer + Job 的进程/文件/注册表隔离已在本机 observed
- 静态 candidate 与 noncompliant live observation 均 fail-closed

### 已知限制
- **网络未闭环**：见上表；strict selection 不得 allow
- **子进程继承**未单独测
- **development exploration，不是生产 sandbox**

### 关闭 GAK-SBX-001 仍需
1. ~~本机执行 live probe~~（已完成）
2. observation `compliant`（当前卡在 `network_connect_blocked`）
3. compliant candidate + selection `allow`
4. 额外网络隔离机制（WFP/防火墙/等价），可能需要 elevation
