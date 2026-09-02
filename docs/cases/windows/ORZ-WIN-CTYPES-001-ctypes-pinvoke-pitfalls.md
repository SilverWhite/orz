# ORZ-WIN-CTYPES-001 — ctypes P/Invoke 常见坑清单（案例候选）

- **状态**：`candidate`（2026-09-02 晋级候选；未宣称产品级闭环）
- **晋级裁决**：2026-09-01 至 09-02 `win-s4` sandbox（`assurance/windows_sandbox.py`）
  与诊断脚本中，ctypes 调用 Win32 API 连续踩中 7 类机械性坑（函数名、argtypes、结构数组、
  句柄包装、指针生命周期、DLL 归属、Path 语义），均有最小复现，合并沉淀为案例候选。
- **来源证据**：`scripts/s4_vm_diag_token.ps1` / `token2`、`s4_vm_repro_203.ps1`、
  `s4_vm_repro_v4.ps1`、`s4_vm_repro_v7.ps1`；`assurance/windows_sandbox.py` 各 P/Invoke 段。
- **能力**：用 ctypes 在 Python 3.12 x64 上可靠调用 Win32 安全/进程/用户对象 API，
  避免函数名、签名、句柄与内存生命周期的常见错误。

## 观察记录

1. **函数名张冠李戴**：`StringSidToSidW` 不存在 → `ConvertStringSidToSidW`。
2. **DLL 归属易错**：`GetUserObjectSecurity`/`SetUserObjectSecurity` 在 **user32**；
   `LocalFree` 在 **kernel32**；`CreateAppContainerToken` 在 **kernelbase**；
   `DeriveAppContainerSidFromAppContainerName`/`CreateAppContainerProfile` 在 **userenv**。
3. **argtypes 参数数错**：`AllocateAndInitializeSid` 声明 10 参但调用 11 参（漏
   nSubAuthorityCount）→ 在 `_is_elevated` 与 `_set_token_integrity` 各出现一次。
4. **结构数组传址**：CreateRestrictedToken 把"Count+数组"的包装结构地址当
   `PSID_AND_ATTRIBUTES` 数组传 → 内核把 Count 当 SID 指针解引用 → `ERROR_NOACCESS(998)`；
   应直接构造并传 `SID_AND_ATTRIBUTES`/`LUID_AND_ATTRIBUTES` 数组。
5. **句柄双重包装**：`wintypes.HANDLE(handle)` 对已是 c_void_p 的值再包装 →
   TypeError；统一 `_as_handle` 归一化。
6. **指针生命周期**：`ctypes.cast(ctypes.c_char_p(env_bytes), c_void_p)` 的临时对象可能
   被 GC；保持 c_char_p/create_unicode_buffer 对象存活至调用结束（配合
   `CREATE_UNICODE_ENVIRONMENT`）。
7. **`Path("C:")` 是驱动器相对路径**：`Path("C:") / "Users"` 相对 CWD 解析，建目录会落到
   非预期位置；须用 `Path("C:\\")`（`SystemDrive + "\\"`）。

## 回归入口

- `assurance/windows_sandbox.py`：`_lookup_account_sid_string`、`_grant_session_desktop_access`、
  `_create_restricted_token`、spawn 环境块构造。

## 边界

- 复现平台：Python 3.12.10 x64 + Windows 11 25H2；32 位进程的指针/结构对齐不同。
- 行为类坑（AppContainer 内 ctypes 不可用）见 `ORZ-WIN-SBX-003`。
