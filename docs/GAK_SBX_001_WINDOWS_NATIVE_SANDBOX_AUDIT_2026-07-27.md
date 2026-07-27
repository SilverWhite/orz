# GAK-SBX-001 Windows Native Sandbox audit（2026-07-27）

## 裁决

新增 Windows Native Strict Sandbox 的 development 纵向切片，包括 AppContainer 探针、Job Object 进程 containment
和独立 verifier。本轮实现已建立 schema/contract/fixture 层的完整 no-model 验收链：profile、observation、verifier 和 candidate selection。
但**尚未完成实际 AppContainer 进程 live probe**——当前只验证了 schema 合同质量和 verifier 的 fail-closed 行为。

在实际 live probe 通过前，Windows native strict backend 保持 `noncompliant`，reason 为 `appcontainer_probe_not_yet_run`。

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
- `windows_native_strict_candidate()` 在 Windows 上返回 `noncompliant` + `appcontainer_probe_not_yet_run`

## 与现有架构的关系

| 组件 | 关系 |
|---|---|
| Docker sandbox（sandbox.py） | 共享 profile → observation → candidate → selection 四层模式 |
| sandbox_verifier.py | 共用 `verify_sandbox_selection_receipt()`，各自独立 observation verifier |
| assurance/__init__.py | 与 Docker 探针并列导出，由 selection receipt 统一路由 |
| gap register（GAK-SBX-001） | 本轮只关闭 schema/contract/fixture 层；live probe 尚未通过 |

## 评估与限制

### 优点
- 建立了完整的 schema → implementation → verifier → test 闭环
- AppContainer + Job Object 的组合在理论上可达 filesystem/registry/process/network 四层隔离
- `DeriveAppContainerSidFromAppContainerName` 无需管理员即可获得 SID
- 与 Docker 后端使用同一 selection 框架，不增加 selection 层的复杂度
- 未通过 live probe 时保持 fail-closed（不出 compliant candidate）

### 已知限制
- **Live probe 未执行**：当前只验证了合同层和 verifier 的 fail-closed 行为。实际 AppContainer 进程创建尚需：
  - 验证 Python 解释器在空能力 AppContainer 内能正常初始化
  - 确认 AppContainer 的 filesystem/registry virtualization 与本探针的预期一致
  - 实测网络阻断（空 capabilities → 无法出站连接）
  - 验证 process start→Job Object assignment race 在本实现中的具体窗口
- **管理员依赖**：`CreateAppContainerProfile` 需要 Administrator 权限。无 elevation 时，probe 使用 derived SID（无持久化 profile）——可能需要额外的 set-up 使 AppContainer 正确隔离
- **Python 兼容性**：AppContainer 的空能力列表可能导致 Python 无法加载某些 DLL 或访问必要的 registry keys。探针脚本已刻意最小化，但主 Python 进程的初始化可能因 AppContainer 限制而失败
- **网络隔离粒度**：无 capabilities 的 AppContainer 应阻止所有出站连接，但回环（localhost）可能仍可用。docker-sandbox 使用 `network=none` 完全断网，AppContainer 的零能力列表尚未实测是否完全等价
- **子进程继承**：Job Object 的 kill-on-close 覆盖进程树，但 AppContainer 的子进程是否会继承 AppContainer 限制仍需实测
- **这是 development exploration，不是生产 sandbox**：与 Docker 探针一样，本探针只证明一次 disposable 运行的快照，不证明所有路径安全

### 下一步关闭 GAK-SBX-001 的条件
1. 在本机 Windows 上实际执行 `run_windows_native_sandbox_probe()`
2. 至少一个 observation 达到 `compliant` 并写入文档
3. `windows_native_candidate_from_observation()` 产生 compliant candidate
4. `build_sandbox_selection_receipt(requested_backend="windows_native_strict")` 返回 `allow`
5. 补写实际 observed run 的局限性（例如：哪个 checks 通过/失败、AppContainer 创建工作与否、是否需 elevation、Python 兼容问题）
