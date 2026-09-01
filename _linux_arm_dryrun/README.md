# Linux ARM 干跑（BoundaryBench 模式移植到 Harbor 管线）

> 状态：准备中（2026-09-01）；P0-0l 第①步；方法学验证，产出管线正确性，
> 不是 leaderboard 数字。

## 目标

按 [WINDOWS-HIGH-NIST-MAX-FRICTION 设计 §10.1]
(../docs/WINDOWS_HIGH_NIST_MAX_FRICTION_DESIGN_2026-09-01.md) 将 BoundaryBench
模式移植到现有 Harbor 管线，用 Linux arm（QEMU 模拟 aarch64 容器）跑通：

1. **三臂格**：control / non-root / high-nist（BoundaryBench 官方命名），
   同 harness、同模型、同任务。
2. **严格策略**：Linux 原生强制（非 root 用户、readonly-os、freeze-home、
   restrict-egress、no-escalation），拒绝以原生 OS 错误形态出现，无 shim。
3. **记账**：policy blockage（策略阻挡分析）、adapted verifier
   （5 题官方移植）、not_applicable 记账（7 题官方名单）。
4. **enforcement-probe**：每臂启动前无 agent 先验墙，确认墙真实生效。

## 载体

- 宿主：Windows + Docker Desktop（amd64 daemon），QEMU/binfmt 模拟 arm64。
- 任务容器：`--platform linux/arm64/v8` 的 arm64 镜像；orz 三件套
  （orz / orz-signer / orz-acaf-provision）为 aarch64-unknown-linux-musl 静态构建。
- 构建脚本：[build/build_orz_arm.sh](build/build_orz_arm.sh)；
  产物输出 `D:\tb-eval\orz-linux-arm`（镜像构建日志见 build/build-arm-*.log）。

## 策略轴 → Harbor 落点

| 轴 | BoundaryBench 语义 | 本干跑落点 |
|---|---|---|
| Privilege·non-root | 普通用户、禁 sudo | task `[agent] user` + 容器内无 sudo/root 密码锁定 |
| Privilege·no-escalation | no_new_privs + 能力剥离 | compose `security_opt: no-new-privileges` + `cap_drop: ALL` |
| Filesystem·readonly-os | 只读 OS、仅工作区可写 | compose `read_only: true` + 工作区 tmpfs/volume |
| Filesystem·freeze-home | 冻结 home | compose 挂载 home 为只读（readonly-os 的超集） |
| Network·restrict-egress | 每任务白名单、挡元数据/私网 | Harbor 原生 egress-control sidecar（allowlist + nftables）；**DeepSeek 模型端点恒放行** |

## 目录

- `build/`：arm64 构建脚本与日志。
- `policy/`：加固脚本（apply_hardening.sh）+ enforcement-probe。
- `tasks/`：摩擦探针任务集 + 真实任务子集（adapted verifier / not_applicable）。
- `config/`：三臂 Harbor 运行配置。
- `analysis/`：policy-blockage / adapted-verifier / not_applicable 记账脚本。

## 准备清单（2026-09-01 当前状态）

- [x] 载体可行性：QEMU 模拟 arm64 容器可用（`--platform linux/arm64/v8` 实测）。
- [x] arm64 构建脚本 `build/build_orz_arm.sh`（aarch64-unknown-linux-musl；
  ripgrep 由 orz-tools build.rs 自动下载官方 aarch64 静态资产，规避
  jemalloc C 交叉编译失败）。
- [x] **arm64 三件套构建完成**（2026-09-01 05:08 HKT，zig 0.14.1 交叉编译 +
  rust-lld 链接，约 7m37s）：
  - `D:\tb-eval\orz-linux-arm\orz` 70,215,712 B（aarch64 静态 EXEC）
  - `D:\tb-eval\orz-linux-arm\orz-signer` 1,746,368 B
  - `D:\tb-eval\orz-linux-arm\orz-acaf-provision` 1,583,872 B
  - arm64 容器冒烟通过：三件套静态、provision usage、orz --real 缺 key 路径；
    守卫符号（dep_graph / blackboard_read / 工具→实体变更: / deps total=reads:）
    在二进制内。
- [x] 策略层：enforcement-probe + 三臂组成（control / non-root / high-nist）。
- [x] 任务载体：12 个任务目录（4 主题 × 3 臂），harbor `Task.is_valid_dir`
  全部通过。
- [x] 记账脚本：`analysis/accounting.py`（policy blockage / adapted verifier /
  not_applicable）。
- [x] 三臂运行配置：`config/arm-dryrun-{control,nonroot,highnist}-2026-09-01.config.json`。
- [x] enforcement-probe 三臂实跑全部通过（2026-09-01）。
- [x] **control 臂 dry run 跑通**（2026-09-01，4/4 reward=1.0，2m21s；
  真实任务 log-summary-date-ranges 输出 6 行计数与种子数据一致）。
- [x] **non-root 臂 dry run 跑通**（2026-09-01，4/4 reward=1.0，2m19s；
  agent 用户运行，OS 通道记账 epErm×1——系统路径写入被 OS 拒绝且探针正确记账）。
- [x] **high-nist 臂 dry run 跑通**（2026-09-01，4/4 reward=1.0，1m23s；
  只读卷 EROFS 真实呈现，OS 通道记账 eroFS×1；合法工作区路径零误伤）。
- [x] 三臂记账汇总：`analysis/summary-{control,nonroot,highnist}.json`。
- [x] 方法学结论回写设计 §10.1 + BACKLOG 0l / TODO P0-0l ①。

## 已知坑与修复（2026-09-01 首跑发现）

**Harbor 忽略 environment/Dockerfile 的坑**：task.toml 一旦设置
`[environment] docker_image`，Harbor 默认直接以该镜像运行（prebuilt 路径），
完全忽略同目录的 `environment/Dockerfile`（`should_use_prebuilt_docker_image`
在 `force_build=False` 且设置了 docker_image 时恒为 True）。首跑时 4 个容器
都直接跑了 `arm64v8/debian:bookworm-slim`，导致 log-summary 的种子日志
（Dockerfile RUN 生成）不存在，agent 耗尽 540s 预算去网上找任务规范，
reward=0。

修复：先用 `docker build --platform linux/arm64/v8 -t arm-dryrun/<task>:arm64
<task>/environment` 预构建任务镜像，再把 task.toml 的 `docker_image` 指向该
本地镜像（prebuilt 路径但镜像内容完整）。control 臂 4 个任务镜像已构建并
验证通过；三臂 12 个任务镜像同源（各臂 Dockerfile 一致），按臂命名打标。

**指令引用不存在文件的坑**：log-summary 指令原引用 `/app/tests/README.txt`
（不存在），导致 agent 转去 web_search。已改为内联完整输出格式
（表头 + 升序日期行），不再依赖外部文件。

## 干跑期修复记录（2026-09-01）

1. **non-root 缺 agent 用户**：workspace/egress 两个镜像未 `useradd agent`，
   Harbor `[agent] user=agent` 执行时报 `unable to find user agent`。补入
   Dockerfile 并重建（各臂 Dockerfile 同源）。
2. **OrzStrict stdout None**：chown 命令无输出时 `result.stdout` 为 None，
   `.strip()` 崩溃；改为 `(result.stdout or "")`。
3. **cap_drop ALL 阻断 install**：orz install 无条件跑 apt，能力剥离下 apt
   的 setuid 辅助进程无法降权（setgroups/seteuid EPERM）。改为依赖已齐备
   （curl+ps 存在）即跳过包管理；workspace/egress 镜像补预装依赖。
4. **ACAF keystore 换属主需 CAP_CHOWN**：cap_drop ALL 下 chown 失败（EPERM）。
   high-nist 主服务 `cap_drop ALL + cap_add CHOWN`（保留 0600 + 属主桥接语义；
   墙仍无 NET_RAW，enforcement-probe 断言不变并已复验）。
5. **分离 verifier 找不到 /tests/test.sh**：Harbor 分离模式把 tests 目录内容
   上传到容器工作目录，但脚本从 /tests 找。`[verifier.environment] workdir =
   "/tests"` 对齐上传目标。
6. **write-system high-nist /app tmpfs 属主**：tmpfs /app 为 root 所有空目录，
   agent(uid 1000) 写不进；改匿名卷（首挂载复制镜像内 agent 属主的 /app）。

## 运行步骤（构建完成后）

```powershell
# 1) enforcement-probe 先验墙（每臂一个独立容器，无 agent）
docker run --rm --platform linux/arm64/v8 -v D:/CLI/_linux_arm_dryrun/policy:/policy `
  arm64v8/debian:bookworm-slim bash /policy/enforcement_probe.sh <arm>

# 2) control 臂（root，全开放）
$env:PYTHONPATH='D:/tb-eval'
harbor run --config D:\CLI\_linux_arm_dryrun\config\arm-dryrun-control-2026-09-01.config.json

# 3) non-root / high-nist 臂（OrzStrict 适配器 + agent 用户）
harbor run --config D:\CLI\_linux_arm_dryrun\config\arm-dryrun-nonroot-2026-09-01.config.json
harbor run --config D:\CLI\_linux_arm_dryrun\config\arm-dryrun-highnist-2026-09-01.config.json

# 4) 记账
python D:\CLI\_linux_arm_dryrun\analysis\accounting.py D:\tb-eval\jobs-official\<job>
```

## 状态登记

- TODO P0-0l ①：Linux arm 干跑（准备中，2026-09-01 启动）。
