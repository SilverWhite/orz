# ORZ-PLATFORM-TARGET-001 — 平台目标覆盖：生产目标 / 测试目标分开验证（案例候选）

- **状态**：`candidate`（2026-09-13 晋级候选；未宣称产品级闭环）
- **晋级裁决**：构建/评测环境类事故，具备明确复现命令、机械根因（Windows-only 符号 /
  字段 / 依赖进入非 Windows 目标）、逐处修复与两侧平台验证，晋级为精选案例候选
  （`harness_environment` 类）。
- **来源证据**：`docs/incidents/ORZ-PLATFORM-TARGET-001.md`
- **能力**：①**目标覆盖**——「Windows 面全绿」与「非 Windows 目标可编」分开验证，
  且生产目标与**测试目标**各自独立核对（`--all-targets` 语义）；②**平台门纪律**——
  Windows-only 符号 / 字段 / 依赖不得进入非 Windows 目标，必要时按平台给足语义分支
  （unix 走 POSIX 原生路径，非 unix 维持原判），而不是让一侧静默降级。
- **回归入口**：
  - 容器命令（按 `ORZ-BUILD-MOUNT-001` 形态：挂父仓 `-v D:/CLI:/orz`、工作目录
    `/orz/orz`、`RUSTUP_TOOLCHAIN` 定名；镜像 `rust:1.97-slim` +
    `protobuf-compiler` + `ripgrep` + `python3`）：
    `cargo check -p orz-host --all-targets`、`cargo test -p orz-tools --lib`。
  - 修复形态（回归时按此核对）：`orz/crates/codegen/orz-tools/src/computer/local/terminal.rs`
    `require_git_bash_backend` 的 unix 直放行分支、
    `orz/crates/codegen/orz-tools/src/implementations/grok_build/bash/mod.rs` 的同型守卫、
    `orz/crates/orz-host/src/process_tree.rs` `dead_pid()` 的 unix 分支（`sh -c exit 0` 取已死 pid）。
  - 两侧平台核对：Linux 侧编译通过 + Windows 侧语义不变（本批口径：xai-tty-utils 28/0、
    `cargo check -p orz-host` 干净、Windows 侧测试维持各自基线）。
- **验证记录**：
  - 2026-09-13 窗口内三批共 9 处：`0b2a8f5b`（xai-tty-utils 2 处，生产目标）、
    `1f13e5ec`（orz-host 1 处，生产目标，0z S3 重建实测暴露）、`23e7a378` + `ad8f72f8`
    （orz-tools 5 处 + orz-host 1 处，**测试目标**）。
  - 修复后容器 Linux 实测：两段门测试组 **7/7**、host 会话卷 **3/3**、loop 审计 **1/1**；
    全量 `cargo test -p orz-tools --lib` **2923 通过 / 17 失败 / 6 跳过**——17 项为
    `lsp::tests::*`×16 与 `opencode::glob::gitignore_respected`×1（容器缺 LSP 二进制 / git
    的形态），**未逐项归因、不得当作产品缺陷**（同 `ORZ-TOOL-BINARY-COMPAT-001` 归因纪律）。
- **复盘要点（纪律）**：① 生产目标编译通过不构成测试目标通过的证据，反之亦然；② Windows
  回归全绿不构成非 Windows 目标可编的证据；③ 该族缺陷只在「容器 Linux 实测 / 载体重建」
  窗口暴露，代价按窗口计（重建窗口最贵），故检查应前移到重建之前。
- **边界**：本案例覆盖**编译目标覆盖**，不覆盖运行期平台差异（运行期差异归
  `ORZ-WIN-*` 案例族）；容器实测是人工/批次入口，**不是 CI 常驻门**（是否常驻待裁决）；
  本条不主张生产目标在 Linux 有缺陷。
