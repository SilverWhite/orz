# ORZ-TOOL-BINARY-COMPAT-001 — 打包 rg 与运行容器 glibc 不匹配，失败被工具面吞成空结果（事故登记）

- **状态**：已归因 / 已处置（2026-08-17；机械守卫与工具契约修复已落地；
  Linux musl 重建后容器冒烟回归已执行通过——见
  `D:\tb-eval\jobs\2026-08-17__GREP-FIX-SMOKE\smoke-notes.md`）
- **分类**：`harness_environment`（构建打包 × 运行环境）
- **现象**：TB2 冒烟重跑（make-doom-for-mips，
  `D:\tb-eval\jobs\2026-08-17__03-48-57`）中 7 次 grep 全部返回
  "No matches found"（wall 1–36ms），pattern 含 vm.js 实测存在的字符串
  （syscallNum ×20、entryPoint ×3、sectionsToLoad ×2、runElf ×2 等）；
  模型把浅层 list_dir 与系统性空 grep 叠加泛化为「/app 无 C 源码」错误转向，
  最终 wallclock 耗尽、reward 0.0。
- **归因过程（含错误归因更正）**：初判为「模型用不存在的字符串 grep、工具行为
  正确（exit 1 无匹配）」；复核 journal/trajectory/orz.txt 后撤回（两次
  `doomgeneric_mips|frame\.bmp` grep 实际被 plan/console 门拒绝未执行、其余
  7 次全空且含实测存在字符串）；最终容器实机复现定位机械根因。教训：命令/操作
  异常时，先排查环境与机械因素，再归因模型或命令纪律。
- **根因（三层叠加）**：
  1. 构建侧：`build_orz_aliyun.sh`/`build_orz.sh` 设
     `GROK_TOOLS_BUNDLE_RG_PATH=/usr/bin/rg`——rust:1.97-slim（trixie）的
     glibc 动态二进制，要求 GLIBC_2.39，被原样打包进 musl-static orz；
  2. 运行侧：任务容器 Debian bookworm glibc 2.36 无法加载——
     `version 'GLIBC_2.39' not found`，退出码恰为 1、stdout 空；
  3. 工具面：`finalize_grep` 将 exit 1 + 空 stdout（或 exit 2 +
     "No files were searched"）统一转成 "No matches found"，stderr（真实
     错误）被丢弃——「rg 从未运行」与「真无匹配」在工具面机械不可分。
- **复现条件与证据**：容器实机复现（`alexgshaw/make-doom-for-mips:20251031`，
  bookworm/glibc 2.36）：挂载 `rg-15.0.0-override.bin` 跑
  `--version`/精确 orz 命令 → 加载失败 exit 1 + 空 stdout；换官方静态 musl
  rg 15.0.0 同命令命中 20 处（`--stats`: 10 files searched）。journal：
  `D:\tb-eval\jobs\2026-08-17__03-48-57\make-doom-for-mips__Mw8ZnbT\agent\orz.txt`
  （7 次 `tool.grep` span exit_code=1、wall 1–36ms）。
- **处置**（2026-08-17）：
  1. 工具契约（FUS-TOOL-SCOPE-CONTRACT）：`finalize_grep` 结局三型——非零退出 +
     stderr 非空→显式报错；`files_searched=Some(0)`→"Searched 0 files … Retry
     with --no-ignore/--hidden"；空 stdout + searched>0→"No matches found in
     N files"；exit 2 硬失败保留。空结果路径 `rg --files` 探针（同过滤集、
     10K 截断）区分零范围与真无匹配；`GrepSearchInput`/console 注册表新增
     `hidden`/`no_ignore` 开关；`GrepSearchOutput.files_searched` 入信封。
  2. 构建守卫：build.rs 对非 Windows 覆盖路径做 ELF PT_INTERP 静态链接校验
     （动态即构建失败并提示）；两份构建脚本改 `cargo install ripgrep
     --version 15.0.0 --target x86_64-unknown-linux-musl` 产静态 rg。
  3. 登记：ADR-0010 §14.23（v1.23）、BACKLOG 0a、TODO P0-E、操作台设计 §12、
     CLI_PROJECT_INDEX。
  4. 晋级案例候选：`docs/cases/harness_environment/ORZ-TOOL-BINARY-COMPAT-001-bundled-rg-glibc.md`。
- **避免复发（归因纪律，用户裁决 2026-08-17 增加）**：命令/操作出现错误或异常
  空结果时，先额外排查环境与机械因素——二进制/运行时兼容（glibc/musl/架构）、
  工具包装是否吞掉错误流、路径/作用域解析、沙箱/权限/ignore 语义、构建打包来源；
  不得仅凭「工具 exit 1」就断言「工具行为正确」。此考虑写入案例库 README 与
  本案例复盘要点。
