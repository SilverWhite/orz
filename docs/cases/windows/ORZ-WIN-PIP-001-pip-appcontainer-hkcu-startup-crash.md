# ORZ-WIN-PIP-001 — pip 在 AppContainer 墙内 import 期崩溃（HKCU Shell Folders 缺失，案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-02 P0-0l-⑥ batch-2 high-nist 主载（`probe-pip-user-install`
  9/9 双臂批次之一）实机复现：pip 在 AppContainer 墙内任意命令 import 期即崩，
  与本次安装动作、`--no-cache-dir`/`--no-index` 均无关。
- **来源证据**：
  `_windows_high_nist/formal-2026-09-02/evidence-task-high-nist/task-outcome-probe-pip-user-install.json`
  （attempt=blocked + 完整 traceback）与同目录 verify/task-baseline JSON；
  `_windows_high_nist/tasks/probe-pip-user-install/`（本地 wheel + pip 调用）；
  `_windows_high_nist/S4_PROGRESS_2026-09-02.md` §14.2。
- **能力**：预判 pip（及同类 platformdirs/appdirs 依赖工具）在 AppContainer 冻结
  HKCU 墙内的可用性；为 ⑦ agent 侧任务选型提供 blocked 记账依据。

## 观察记录

1. **崩溃点**：`pip install` 子进程启动后，`pip._internal` import 期
   `USER_CACHE_DIR = appdirs.user_cache_dir("pip")` → `platformdirs.windows`
   `get_win_folder("CSIDL_LOCAL_APPDATA")` → `winreg.OpenKey(HKEY_CURRENT_USER,
   Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders)` →
   `FileNotFoundError: [WinError 2] 系统找不到指定的文件。`
2. **不可绕过**：`--no-cache-dir --no-index --no-deps --no-input --log <workspace>`
   全部带上仍崩溃——USER_CACHE_DIR 在模块加载期解析，命令行参数尚未生效。
3. **环境语义**：AppContainer（LOW IL + 空能力）+ HKCU 冻结下，当前进程的注册表
   视图不含 RunUser hive 的 Shell Folders 键（读不到路径）；同一安装包在 control
   臂 success（本地 wheel、9/9 批次证据）。机器侧记为 blocked（error_reported），
   与 denied（操作被显式拒绝）区分。

## 回归入口

- `_windows_high_nist/tasks/probe-pip-user-install/probe.py`（blocked 归类：
  USER_CACHE_DIR/appdirs/platformdirs/Shell Folders/FileNotFoundError 标记）。
- `_windows_high_nist/tasks/verify_task.py`（blocked 语义：
  expected ∈ {denied, blocked} 时要求 error_reported）。

## 边界

- 复现平台：Python 3.12.10（python.org，Program Files）+ pip 24.x；
  Windows 11 25H2 build 26200。
- 未测：非空 AppContainer capability、pwsh/pipx 等同族工具、AgentUser 非
  AppContainer（non-admin）臂——按用户裁决不补跑 non-admin 任务消融。
- 候选修复方向（归 ⑦，不在本切片实施）：向 AppContainer 子进程注入可用
  HKCU 视图/环境变量使 platformdirs 走环境路径；或把该轴登记为高摩擦已知
  边界并在 agent 侧任务选型避开。
