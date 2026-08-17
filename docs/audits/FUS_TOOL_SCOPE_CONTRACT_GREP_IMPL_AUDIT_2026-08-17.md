# FUS-TOOL-SCOPE-CONTRACT grep 面实施审计（2026-08-17）

- **范围**：grep 工具结局三型分型 + 空结果 `rg --files` 探针 +
  `hidden`/`no_ignore` 开关 + `GrepSearchOutput.files_searched` 信封字段 +
  build.rs 静态链接守卫 + 评测构建脚本静态 musl rg。涉及
  `orz-tools`（grep 工具）、`orz-loop`（console 注册表 schema）、
  `orz-tools/build.rs`、`D:\tb-eval\build_orz_*.sh`。
- **依据**：ADR-0010 §14.23（v1.23）/ FUS-TOOL-SCOPE-CONTRACT /
  BACKLOG 0a 项 5 / TODO P0-E / 案例 ORZ-TOOL-BINARY-COMPAT-001。
- **根因（容器实机复现）**：glibc 动态 rg（trixie `/usr/bin/rg`，要求
  GLIBC_2.39）经 `GROK_TOOLS_BUNDLE_RG_PATH` 打包进 musl orz；任务容器
  bookworm（glibc 2.36）加载失败 exit 1 + 空 stdout；`finalize_grep` 把
  exit 1 + 空 stdout（或 exit 2 + "No files were searched"）统一转成
  "No matches found" 并丢弃 stderr——7/7 grep 空结果、模型错误泛化「无源码」。

## 实施内容

1. **结局三型分型**（`finalize_grep`）：
   - 非零退出且 stderr 非空 → 显式报错（`Error calling tool: <stderr>
     (exit N, root: …)`），先于空结果判断；
   - `files_searched=Some(0)` → "Searched 0 files … Retry with
     --no-ignore/--hidden"（范围空，区别于真无匹配）；
   - 空 stdout 且 searched>0 → "No matches found in N files"；
   - exit 2 语法错误保持硬失败。
2. **机械来源定稿**：v1 用 `rg --files` 探针（仅空结果路径：非零退出 + 双流
   为空），与主搜索同过滤集（glob/type/deny/ignore/hidden/max-filesize），
   10K 计数截断并杀子进程；弃用 `--stats`——rg 15 输出到 stdout、旧版到
   stderr，跨版本位置差异会污染流式面与卡片一致性。
3. **参数面**：`GrepSearchInput` 新增 `hidden`/`no_ignore`（映射
   `--hidden`/`--no-ignore`，主搜索与探针同传）；console 注册表
   `workspace.grep` schema 同步。
4. **信封字段**：`GrepSearchOutput.files_searched: Option<u64>`
   （serde 兼容；命中路径 None、零范围 Some(0)、真无匹配 Some(N)）。
5. **构建守卫**：build.rs 对非 Windows 的覆盖路径做 ELF PT_INTERP 静态链接
   校验，动态二进制直接构建失败并提示（已验证：glibc 版检出 PT_INTERP、
   官方静态 musl 版无）。
6. **评测构建脚本**：`build_orz_aliyun.sh`/`build_orz.sh` 改
   `cargo install ripgrep 15.0.0 --target x86_64-unknown-linux-musl` 产静态
   rg 供覆盖，不再指向 `/usr/bin/rg`；apt 依赖补 `make`（cargo install
   编译 jemalloc 需要）。

## 测试证据

- grep 模块 42/42（新增：stderr 显式报错 / 零范围分型 / 带计数 no-match /
  隐藏目录探针 0 与 --hidden=1）；types 561/561；orz-loop console 68/68；
  planning 13/13；plan_first 9/9；clippy 无新增告警（orz-loop 基线告警不变）；
  orz-tools 全量 2747 过（1 个 LSP e2e 并行偶发、单跑通过，与本改动无关）。
- Linux musl 重建（rust:1.97-slim，10m04s）通过 build.rs 静态守卫；
  打包 rg 35.6MB、无 PT_INTERP。

## 冒烟回归（任务容器实机）

证据：`D:\tb-eval\jobs\2026-08-17__GREP-FIX-SMOKE\smoke-notes.md`。

- 二进制级：打包 rg `--version`=15.0.0；精确 orz 命令对 `/app/vm.js` 搜
  `syscallNum` 命中 20 处；`DG_DrawFrame` over /app 命中。
- 端到端：新 orz + ACAF 签发器 + 文件密钥库，`orz --real` 指令「搜索
  syscallNum 并报告」——模型报告命中 20 行（带行号）并 done、exit 0；
  journal grep tool_completed exit_code 0。对照旧二进制 7/7
  "No matches found"（`D:\tb-eval\jobs\2026-08-17__03-48-57`）。

## 边界与后续项

- `--files` 探针仅空结果路径运行；命中路径 `files_searched` 留空（后续
  `--json` 面或 stats 位置收敛后再定）。
- list_dir ignored/truncated 计数未实施（FUS-TOOL-SCOPE-CONTRACT 后续项）。
- 构建守卫只覆盖非 Windows 覆盖路径。

## 登记

ADR-0010 §14.23（v1.23）/ BACKLOG 0a 项 5 / TODO P0-E / CLI_PROJECT_INDEX /
操作台设计 §12 / 案例 ORZ-TOOL-BINARY-COMPAT-001；orz 子模块 b990b49。
