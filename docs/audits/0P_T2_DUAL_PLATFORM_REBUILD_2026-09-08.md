# 0p T2 双平台重建 + VM 同步 + manifest 重刷 + DryRun + enforcement-probe 审计（2026-09-08）

> 排期入口：`docs/BLACKBOARD_SELF_HISTORY_AND_GSA_TWO_STAGE_GATE_DESIGN_2026-09-07.md` §5 第 4 条（T2）。
> 基线：orz `7b00bbc9`（0.3.2 bump；S1 `928dceb3`+`fd46d4f9`、S2 `7d7d89e7`+`542c35d5` 已含）。
> 用户裁决（2026-09-08）：VM 侧只跑 high-nist 臂即可，不要求三臂严格可比（工程项目非研究）；
> C 盘清理谨慎执行（经用户逐项裁决）。
> 证据：`_windows_high_nist/evidence-t2-032-20260908/`。

## 1. 事实链

### 1.1 Windows 三件套 0.3.2（宿主）

- `cargo build --release -p orz-bin`（PROTOC=`orz\bin\protoc.exe`），构建日志确认 `orz-bin v0.3.2`。
- staging-0.3.2 哈希锁定（SHA256SUMS.txt）：
  - orz.exe `57c4ec0298a4059a4a4222b1490debf87e83f2891e2e07478a00a5c6b25cde58`
  - orz-signer.exe `cd405bfbb7b301c829b9ea41528f88c0397f35aefa1c51cc01792d4fc27f7f3c`
  - orz-acaf-provision.exe `dd70be420c117fd7bf9bb54150ecbe30efe7c0fc602b0f1441d849441f795c1e`
- `D:\tb-eval\orz-windows\` 三件同步刷新（`s4_copy_to_vm.ps1` 源路径）。

### 1.2 Linux musl 三件套 0.3.2（Docker rust:1.97-slim）

- `build_orz_aliyun.sh` 重建，`BUILD_EXIT=0`（fail-loud：退出码不再经管道吞掉），日志
  `orz-bin v0.3.2` 编译行；产物 `D:\tb-eval\orz-linux\` 三件 2026-09-08 01:16。
- 接线符号命中：`session_volume_notice`（S2 两段门通知信封）在卷。
- 静态性：三件 PT_INTERP=0、ld-linux 字符串=0（static-pie）。
- bookworm 容器冒烟：三件加载执行，`orz-acaf-provision` usage / `orz-signer` manifest 缺失
  fatal / `orz --version` 无 TTY tui io error——全部预期形态。

### 1.3 VM 同步 + acaf manifest 重刷（win-s4）

- `vm-acaf-reprovision` op：三件 0.3.2 换装 `C:\Program Files\orz` + `C:\s4\tools`
  （哈希逐对吻合 §1.1，旧件备份 `backup/*.pre-acaf-20260908_*.bin`）；
  AgentUser 沙箱内 provision 重刷 `C:\workspace\acaf`——`MANIFEST_SIGNER_HASH_MATCH=True`
  （binary_sha256=cd405bfb…）、keystore 三件清点、机器 env 四键回读全对、
  `ACAF_PROVISION_OK=True`、SBX outcome=compliant。
- `stage sync`：runner/probe/harness/orz.exe 9/9 任务同步 `SYNC_ALL_OK=True`。

### 1.4 验收门

- **vm-agent DryRun 全对**（high-nist 臂 tb2.1 三题，RunTag `dryrun-t2-032-final`）：逐题
  官方 timeout 900/900/3600、env-file 注入、allowlist `221.204.163.76`、errors 空、
  `AGENT_RUN_OK=True`、EXIT=0（173ade6 假绿修复后的真绿通道）。
- **enforcement-probe**：control 臂 exit 0（failed=0，顺带）；**high-nist 臂 19/19 全绿**
  （home_frozen/appcontainer_token/network_blocked/metadata_blocked/probe_file_cleaned 等
  全 true，exit 0，ResultPath 墙内落盘 + 证据回拷）。按用户裁决 non-admin 臂不要求
  （见 §2.3 排障记录：曾在残留加固态下 9/1，非 0.3.2 回归）。

## 2. 排障记录（时间序）

1. **构建脚本 apt 源瞬时故障**：aliyun plain-HTTP 502 → HTTPS 化仍索引失败 → 探测三镜像
   均间歇故障 → 定稿 USTC HTTPS + `apt-get update` 5 次重试环（`build_orz_aliyun.sh` 已改，
   沉淀为脚本内注释）。另：`bash -c | tee | tail` 管道吞退出码曾致首轮假绿，改为
   重定向+`BUILD_EXIT=$?` fail-loud。
2. **Docker Desktop 崩溃**（`error waiting for container: unexpected EOF`）：重启 Docker
   Desktop 后恢复。
3. **C 盘空间不足**：VM 启动/还原需 9.5GB VMRS，原仅 3.9GB 可用。经用户裁决清理：
   `.codex\visualizations`（6.9GB，codex 生成可视化产物）删除 + 回收站清空 + 用户 Temp
   清理 → 13GB 可用。DISM 组件清理暂缓（已够用）。检查点 VMRS 文件（快照内存）确认
   不可删后保留。
4. **non-admin 臂 9/1**（`home_write_succeeded=false`）：VM 恢复自昨日 chunk2 high-nist
   跑批后的加固残留态。非 0.3.2 回归（该检查不经过 orz 二进制）。按用户裁决不再追打。
5. **high-nist 臂 0xC0000142**（AppContainer 子进程 DLL 初始化失败，观测面 8/8 全过）：
   两轮复现确定性 → 检查点还原后仍复现 → **根因 = 检查点还原后漏跑 `stage setup`**
   （该阶段建 AgentUser + `ORZ_WINDOWS_RUN_USER*` 机器环境 + SeBatchLogonRight，还原被
   回滚；日志旁证 `RunUser=M` 残缺形态）。补跑 setup 后 high-nist 19/19 全绿。
   附带发现：检查点还原会把 `C:\s4` 一并回滚到基线版本（旧沙箱 CLI 报
   `StringSidToSidW` AttributeError），正规顺序 restore→sync→setup→arms 必须完整走。
6. **假绿教训重演**：本轮两次构建/执行退出码被 shell 管道形态掩盖，均已改为 fail-loud。

## 3. 结论与边界

- **T2 验收达成**：双平台三件套 0.3.2 + VM 换装 + manifest 重刷 + DryRun 全对 +
  enforcement-probe high-nist 19/19（control 顺带绿）。0p 排期下一项 = S4 重跑
  train-fasttext（RunTag `tf-selfhistory-032`，判据表设计 §4）。
- 边界登记：non-admin 臂按用户裁决不跑；DISM 清理未做（C 盘余量 13GB，S4 前如再触
  空间墙再议）；`.codex\visualizations` 删除不可逆（用户裁决）；staging-0.3.2 为本地件
  不入 git（既有 gitignore 纪律）。
- 正式闭合登记（BACKLOG/索引/manifest 重算/门禁）按排期随 S5 收口统一落账。
