# ORZ-WIN-SBX-003 — AppContainer 子进程兼容面（二进制/句柄/环境/完整性，案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-02 `win-s4` high-nist 臂探针排障中，AppContainer 子进程连续暴露
  五类平台兼容约束（CLR/ctypes/whoami 不可用、管道句柄不可用、TEMP 重写、Low 完整性
  写限制、包目录归属错误），每类均有最小复现与可靠路径，合并沉淀为 AppContainer 兼容面
  案例候选。
- **来源证据**：`scripts/s4_vm_repro_v10.ps1`（管道二分）、`v11`（powershell 崩溃）、
  `v12`（python/cmd 可行）、`v13`（工作区写入）、`v14`（ctypes 导入矩阵）；
  `assurance/windows_sandbox.py`（use_std_handles、Low 标签、AC\Temp）；
  `_windows_high_nist/policy/enforcement_probe.py`（探针宿主与行为代理）。
- **能力**：在 Windows AppContainer（LOW IL + 包 SID）子进程内选择可用程序/IO/路径模型，
  并预判其文件与环境语义。

## 观察记录

1. **非原生程序不可用**：`powershell.exe`（Windows PowerShell 5.1 / .NET Framework CLR）
   在 AppContainer 进程内 DLL 初始化失败 `0xC0000142`；`python` 的 `_ctypes`
   （libffi-8.dll）导入失败（DLL 初始化失败）；`whoami.exe` 无输出（同类问题）。
   `cmd.exe` / `python.exe`（纯原生）正常。
   - batch-2 细分（2026-09-02）：上述 powershell 失败发生在“AppContainer 进程内
     再 spawn（孙进程）”路径；宿主以 CPAU+属性列表直接创建的 PS 直子进程可正常
     执行（enforcement-probe 19/19），见 `ORZ-WIN-SBX-004`。
   - 可靠路径：AppContainer 探针/宿主用原生程序；令牌类事实（AppContainer SID、完整性）
     由父侧 P/Invoke observation 断言，子进程内用行为代理（Medium 目录写不进 + Low 目录
     可写；LOCALAPPDATA 被 OS 重写为 `Packages\<app>\AC`）。
2. **匿名管道 std 句柄不可用**：AppContainer 子进程带 STARTF_USESTDHANDLES（匿名管道）
   启动 → `0xC0000142`（管道 DACL 只授创建者 SYSTEM，包令牌无权访问句柄）；不传句柄后
   正常。可靠路径：AppContainer 子进程跳过管道，结果写文件。
3. **LOW 完整性写限制**：LOW 子进程写默认（Medium）标签对象被拒（no-write-up）。
   可靠路径：工作区 `icacls /setintegritylevel Low /T /C`，并把探针/产物放入该目录。
4. **OS 重写 TEMP/LOCALAPPDATA**：AppContainer 子进程的 TEMP/TMP/LOCALAPPDATA 被系统
   重写到 `运行用户\AppData\Local\Packages\<app>\AC`；而 `CreateAppContainerProfile` 按
   **调用者**（此处 SYSTEM）profile 建包目录。可靠路径：sandbox 在运行用户 profile 下
   预创建 `Packages\<app>\AC\Temp` 并授包 SID `(OI)(CI)(M)`（注意 `Path("C:")` 是驱动器
   相对路径，须用 `Path("C:\\")`）。
5. **只能读授权路径**：AppContainer 读不了用户自建目录（如 `C:\s4`），只能读
   Program Files / System32（ALL APPLICATION PACKAGES）与已授包 SID 的路径。
   可靠路径：探针脚本复制进已授权的工作区再执行。

## 回归入口

- `assurance/windows_sandbox.py`：`use_std_handles` 条件、工作区 Low 标签、
  AC\Temp 预创建 + 包 SID 授权、`_grant_session_desktop_access(extra_sids=...)`。
- `_windows_high_nist/policy/enforcement_probe.py`：探针宿主（python）、文件重定向、
  行为代理断言。

## 边界

- 复现平台：Python 3.12.10 + Windows 11 25H2 build 26200。
- .NET Core（pwsh 7）在 AppContainer 内的兼容性未实测；whoami 失败的底层 DLL 未进一步
  定位（只确认"无输出"现象）。
- 工作区 Low 标签会降低该目录对 Medium/High 进程的写保护语义，仅限专用沙箱工作区使用。
