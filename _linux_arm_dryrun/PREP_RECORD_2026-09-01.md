# Linux ARM 干跑准备记录（2026-09-01）

> 归属：TODO P0-0l ①（WINDOWS-HIGH-NIST-MAX-FRICTION，Linux arm 方法学干跑）。
> 状态：**准备完成**（2026-09-01）；arm64 三件套已产出并核验，enforcement-probe
> 与任务载体已就绪；实机 dry run 待用户放行（暂不跑测试）。

## 1. 载体确认

- Docker Desktop 宿主 amd64；QEMU/binfmt 模拟 arm64 容器可用
  （`docker run --platform linux/arm64/v8 arm64v8/debian:bookworm-slim uname -m`
  → `aarch64`，实测通过）。
- arm64 三件套目标：`aarch64-unknown-linux-musl` 静态构建；
  构建脚本 `build/build_orz_arm.sh`（基于 ORZ-BUILD-MOUNT-001 契约）。
- ripgrep 打包：不本地 `cargo install`（aarch64-musl 下 jemalloc C 编译失败，
  `atomic_memory_order_*` undeclared），改由 orz-tools build.rs 自动下载官方
  ripgrep 15.0.0 `aarch64-unknown-linux-gnu` 资产（该资产为 musl 静态，
  build.rs asset triple 映射已确认支持）。
- **最终方案：x86 rust 容器 + zig cc 交叉编译 + rust-lld 链接**：
  - zig cc（0.14.1）编译全部 C 依赖（aws-lc/zstd/ring/sqlite 等）为 aarch64-musl；
    wrapper 把 rust triple `--target=aarch64-unknown-linux-musl` 翻译为 zig
    原生 `-target aarch64-linux-musl`（zig 不接受 rust triple 格式）。
  - 链接用 rust-lld（wrapper 剥离 `-Wl,` 前缀、过滤 `-nostartfiles`/
    `-nodefaultlibs`、显式 `-flavor gnu`）——zig cc 链接会注入自己的 crt1.o
    与 rust self-contained crt 冲突（duplicate _start），故 zig 只编译不链接。
  - 耗时 7m37s（release，-j 8），远快于 QEMU 模拟（3h+ 仍未完成）。

## 2. 严格策略层（已实测）

- enforcement-probe 三臂断言在 arm64 容器内全部通过：
  - control：工作区可写、/usr/local 可写。
  - non-root：uid≠0、无 sudo、系统路径 EPERM、工作区可写。
  - high-nist：只读目标卷 EROFS、home 冻结、NoNewPrivs=1、cap drop 无 NET_RAW。
- 探针语义修正记录：
  - 「只读 OS」断言从「写 /」改为「写注入的只读目标卷」——high-nist 臂非
    全局 read_only（install 需写 /usr/local/bin、/etc/orz-acaf），断言必须
    锁定目标卷而非整个 rootfs。
  - home_frozen 显式注入 PROBE_HOME，避免「无 HOME」误判。

## 3. 任务载体（12 目录，harbor is_valid_dir 全通过）

- 摩擦探针：probe-write-system（系统路径写）、probe-write-workspace（工作区写）、
  probe-egress（网络出站）。
- 真实任务子集：log-summary-date-ranges（纯文件/文本任务移植，arm64 种子数据
  确定性生成）。
- 每主题 × 三臂 = 12 个独立任务目录（harbor 本地任务=目录名）。
- high-nist 臂 verifier 用 separate 模式（硬化边界外），artifacts 传递
  result/summary 文件。

## 4. 记账与分析

- `analysis/accounting.py`：对照 BoundaryBench 官方 not_applicable（7 题）与
  adapted-verifier（5 题）名单 + OS 错误文本→机械层通道映射
  （err/slow/deny 口径，对应设计 §7 判据 1 的干跑先行版）。
- 已在真实 job 数据（dependency-graph-smoke）上验证运行。

## 5. 已登记的约束与口径调整

- **Harbor 原生 egress allowlist 本机不可用**：Docker Desktop VM 内核
  `/proc/config.gz` 缺 `CONFIG_NFT_FIB_INET`（有 `CONFIG_NFT_FIB=m` /
  `CONFIG_NFT_FIB_IPV4=m`，无 `NFT_FIB_IPV6`），Harbor 内核探针保守返回
  False → `_enable_egress_control=False`，allowlist sidecar 不注入。
  影响：Linux 干跑不伪造 egress 墙——probe-egress high-nist 臂预期记为
  `reachable`（基线），restrict-egress 留待 Windows 主载（host firewall/WFP）。
- **ACAF keystore 属主桥接**：non-root/high-nist 臂 orz 以 agent 用户运行，
  install 阶段 root 创建的 0600 keystore 需可读——新增 `tb_agents.orz_strict`
  适配器，install 后 chown keystore/manifest 给 agent（文件仍 0600，仅换属主，
  ACAF 校验完整）。

## 6. 下一步（构建完成后）

1. 核验三件套为 aarch64 ELF 且静态（readelf/file + bookworm 冒烟）。
2. `run/run_enforcement_probe.ps1` 三臂先验墙。
3. control 臂 dry run（4 任务）→ 记账。
4. non-root / high-nist 臂 dry run → 记账 + 三臂 delta 归因。
5. 方法学结论回写设计 §10.1 + BACKLOG/TODO 登记。

## 7. 干跑执行记录（2026-09-01，全部完成）

- enforcement-probe 三臂先验墙：全部通过（control 全开放 / non-root 降权 +
  禁 sudo + 系统路径 EPERM / high-nist 只读卷 EROFS + home 冻结 + NoNewPrivs +
  无 NET_RAW）。
- control 臂：4/4 reward=1.0，2m21s，0 异常。
- non-root 臂：4/4 reward=1.0，2m19s，0 异常；OS 通道记账 epErm×1。
- high-nist 臂：4/4 reward=1.0，1m23s，0 异常；OS 通道记账 eroFS×1。
- 真实任务 log-summary-date-ranges 三臂输出一致（6 行计数
  45/21/33/24/33/21，与种子数据精确一致）。
- 记账汇总：`analysis/summary-{control,nonroot,highnist}.json`。
- 干跑期修复 6 项（Harbor docker_image 忽略 Dockerfile / 指令引用不存在文件 /
  non-root 缺 agent 用户 / OrzStrict stdout None / cap_drop 下 apt 与 chown /
  分离 verifier workdir / write-system /app tmpfs 属主），详见 README
  「已知坑与修复」「干跑期修复记录」。

## 8. 方法学结论（供设计 §10.1 回写）

1. **管线正确性成立**：BoundaryBench 三臂格（control/non-root/high-nist）
   移植到现有 Harbor 管线端到端可跑，enforcement-probe 先验墙 + 摩擦探针 +
   真实任务 + 记账全链路通。
2. **策略墙以原生 OS 错误形态真实呈现**：non-root 系统路径写入 EPERM、
   high-nist 只读卷 EROFS，均被 agent 捕获并记账（epErm / eroFS 通道）；
   合法工作区路径零误伤（probe-write-workspace 三臂全过、真实任务三臂同分）。
3. **Harbor 承载限制**：egress allowlist 本机不可用（内核缺 NFT_FIB_INET，
   记 reachable 基线，restrict-egress 留 Windows 主载）；分离 verifier 用
   `workdir=/tests` 对齐；cap_drop 需保留 CHOWN 以完成 ACAF keystore 属主
   桥接（0600 语义不变）。
4. **Windows 主载校准**：Linux 侧验证的是管线与记账口径；Windows 原生摩擦
   （ACL/AppContainer/Firewall/WFP）仍需按 §7 判据在硬化 VM 上逐项核对。
