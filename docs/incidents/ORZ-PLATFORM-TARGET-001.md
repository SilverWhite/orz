# ORZ-PLATFORM-TARGET-001 — 平台目标覆盖缺口：Windows 面全绿 ≠ 非 Windows 目标可编（事故登记）

- **状态**：已归因 / 已处置（2026-09-13；同一窗口内三批 9 处全部修复）
- **分类**：`harness_environment`（构建与测试目标覆盖）
- **现象**：orz 工作区在 Windows 上编译、测试、打包全部正常，非 Windows（评测容器 /
  载体重建）面却在**三类不同位置**编译失败；每次都不是被日常回归拦下，而是在
  「容器 Linux 实测」或「载体重建」窗口才暴露：
  1. **2026-09-13 P1-4 处置批**（orz `0b2a8f5b`）：`xai-tty-utils` 两处——`lib.rs`
     re-export 列表无条件导入 `#[cfg(windows)]` 的 `duplicate_job_handle` /
     `image_fingerprint_from_handle`（E0432）；`sha2` 依赖误挂 windows 桶，而 Linux 侧
     指纹路径 `image_fingerprint_from_pid`（`/proc/<pid>/exe` sha256）需要它（E0432）。
  2. **2026-09-13 0z S3 载体重建**（orz `1f13e5ec`）：`orz-host` 无条件读取
     `SpawnObservation.job_handle_dup`，该字段本身 `#[cfg(windows)]`（E0609）——P1-4 批
     只修了 `xai-tty-utils` 两处，本处漏网，实测暴露时距 musl 重建只差一步。
  3. **2026-09-13 本轮**（orz `23e7a378` + `ad8f72f8`）：**测试目标**编不过——
     `orz-tools` 5 处（`computer/local/terminal.rs::require_git_bash_backend` 与
     `implementations/grok_build/bash/mod.rs::bash_streaming_progress_includes_final_drain`
     调用 `#[cfg(not(unix))]` 的 `orz-config::shell::{detect_windows_shell, WindowsShell}`）
     + `orz-host` 1 处（`process_tree.rs::dead_pid()` 只有 `#[cfg(windows)]` 实现，同文件
     多处 sweep 测试无门调用）。
- **根因（同族）**：Windows-only 符号 / 字段 / 依赖进入非 Windows 目标。两个放大因素：
  ① **目标面互不覆盖**——前两批是生产目标（`cargo check` / musl 打包），第三批是
  **测试目标**（`cargo test --lib`）；生产目标能编不代表测试目标能编，反之亦然，两侧
  都不是对方的代理指标；② **发现时机**——该检查不常在日常回归里，只在容器 Linux 实测 /
  载体重建时触发，代价按窗口计（重建窗口最贵：不修则整次重建必败）。
- **复现条件**：Linux 容器内按 `ORZ-BUILD-MOUNT-001` 形态挂父仓（`-v D:/CLI:/orz`、
  工作目录 `/orz/orz`）并定名 `RUSTUP_TOOLCHAIN`，然后 `cargo check -p orz-host
  --all-targets` 与 `cargo test -p orz-tools --lib`；修复前分别报 E0432 / E0609 /
  E0425 / E0433。
- **证据**：orz 提交 `0b2a8f5b` / `1f13e5ec` / `23e7a378` / `ad8f72f8`（提交信息含现象、
  修法与两侧平台验证）；索引条目 `GAP-ORZ-TEST-TARGET-UNIX-BUILD` 与
  `FUS-HOST-RESOURCE-SAFETY`；[`0Z S3 双平台重建记录`](../audits/0Z_S3_DUAL_PLATFORM_REBUILD_2026-09-13.md)。
- **处置（2026-09-13）**：
  1. 逐处修复：非 Windows 目标补 cfg 门并给足语义（unix 分支走 POSIX 原生路径——
     `sh -c exit 0` 取已死 pid；Git Bash 前置条件在 unix 不存在即放行），Windows 语义不变。
  2. 同批把 grep 两段门断言的失败信息补上 stdout/stderr（失败信息应自带证据）。
  3. **回归纪律**：非 Windows 目标编译（生产目标 + 测试目标）前移到载体重建之前，
     入口见案例文件的「回归入口」。
  4. 晋级案例候选：`../cases/harness_environment/ORZ-PLATFORM-TARGET-001-platform-target-coverage.md`。
- **避免复发**：重建前置清单加入「非 Windows 目标编译」一步（生产 + 测试两侧，`--all-targets`
  覆盖）；案例回归入口给出容器命令与两侧平台核对口径。
- **边界**：本条登记的是**目标覆盖**缺口，不主张 orz 生产目标在 Linux 有缺陷——0.5.0
  双平台载体构建正常；三批修复均未改变 Windows 语义，第三批改动全部落在测试目标内
  （生产目标零变化）。
