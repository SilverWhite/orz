# ORZ-WIN-SBX-004 — AppContainer 内再 spawn：孙进程兼容面（直子进程 vs 孙进程，案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-02 P0-0l-⑥ batch-2 high-nist 主载
  （`probe-unsigned-ps1-run`）实机复现：AppContainer 墙内**由 AppContainer
  进程再 spawn** 的 powershell.exe 孙进程 DLL 初始化失败；与墙探针（PS 作为
  宿主直子进程）19/19 正常的差异，指向“由谁 spawn”这一此前未细分的维度。
- **来源证据**：
  `_windows_high_nist/formal-2026-09-02/evidence-task-high-nist/task-outcome-probe-unsigned-ps1-run.json`
  （attempt=blocked，rc=0xC0000142）与同目录 verify/task-baseline JSON；
  `enforcement-probe-high-nist.json`（19/19，PS 直子进程路径正常）；
  `_windows_high_nist/S4_PROGRESS_2026-09-02.md` §14.2；关联
  `docs/cases/windows/ORZ-WIN-SBX-003-appcontainer-child-compat.md`。
- **能力**：预判 orz/agent 在 AppContainer 墙内 spawn 命令树（含 PowerShell）
  的兼容边界——直子进程可用不代表孙进程可用。

## 观察记录

1. **孙进程 powershell.exe 失败**：python（AppContainer 子进程）经
   `subprocess.run` spawn `powershell.exe -NoProfile -NonInteractive
   -ExecutionPolicy Bypass -File <ws>\unsigned_probe.ps1` → 退出码
   `0xC0000142`（STATUS_DLL_INIT_FAILED），stdout/stderr 为空，marker 未落盘。
   同一探针在 control 臂 success（证明脚本/调用面本身无误）。
2. **直子进程正常**：enforcement-probe 以 powershell.exe 作为沙箱直子进程
   （宿主 CPAU + 属性列表创建）执行 19/19 断言全过；SBX-003 观察 1 的
   “powershell 不可用”应细化为“AppContainer 进程内再 spawn 的孙进程路径”。
3. **孙进程 python 可行**：pip 探针的 `python -m pip` 孙进程能启动（其崩溃
   在 pip import 期，见 ORZ-WIN-PIP-001），说明 AppContainer 孙进程并非全灭，
   问题具 DLL/镜像特异性（Windows PowerShell 5.1 依赖 .NET Framework CLR）。
4. **影响面**：orz 在墙内以“主进程 → 命令子进程”形态跑 PowerShell 工具链时
   将撞同型失败；§7 判据 3/5 与 §8 假设 6 的模型侧核对须区分直子/孙进程路径。

## 回归入口

- `_windows_high_nist/tasks/probe-unsigned-ps1-run/probe.py`（0xC0000142 →
  blocked 归类）。
- `assurance/windows_sandbox.py`（直子进程 spawn 属性）；墙探针直子进程路径。

## 边界

- 复现平台：Windows PowerShell 5.1（.NET Framework）+ Windows 11 25H2
  build 26200；AppContainer 空 capability + LOW IL。
- 未测：pwsh 7（.NET Core）在孙进程路径的兼容性、非空 capability、direct
  COM/DLL 加载细节。
- 候选修复方向（归 ⑦/模型侧核对）：任务经 sandbox 直子进程执行（如墙探针
  模式），或登记为边界并在 agent 侧任务选型避开 PowerShell 孙进程依赖。
