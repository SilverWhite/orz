# Security Policy

## 报告渠道

安全漏洞请经**本仓** GitHub Security Advisories 私密报告：

https://github.com/SilverWhite/orz/security/advisories/new

请勿为安全报告开设公开 GitHub issue，也不要把未修复细节、凭据或用户数据贴到公开面。若私密入口暂不可用，可开一条**不含复现细节**的 issue 说明「需要安全报告私密通道」，等待维护者联系。

（历史文档曾指向 xAI 上游的 HackerOne 计划，那不是本仓渠道；已更正。）

## 安全模型概览（机械层）

orz 运行时安全由**机械层**承载（模型提议、机械层执行/校验）：

- **读面沙箱**：文件读取收窄于工作区＋注册技能根＋会话卷只读窗口（`.gsa/session/terminal/*.log`、`run_tests_output.txt`）；`.gsa` 读为「读全开放、只读不改」，两段门为确认性设计。
- **写入管控（0cc v3＋319 加固，2026-10-19）**：工具面只硬拒**宿主状态窄目标**（`.gsa` 会话卷及其祖先目录＋ACAF keystore 根／signer manifest 及其祖先链）＋`run_terminal_cmd` 命令面五条 block 规则（规则 1 已补工作区根／祖先／`.`／无约束通配／`find -delete` 等；维护 CLI 命令面 block）；其余写动作交回审批组件。
- **ACAF 票据门**：命令、写入与联网访问经权限桥与 ACAF 一次性票据（`orz-signer` 独立进程签发；未配置即 fail-closed）。
- **Linux 子进程 Landlock**：`run_terminal_cmd` 子进程写面由内核 Landlock ruleset 收窄（0bw①／0ch v4，已实施）。
- **journal**：会话事件逐条留痕，支持 `--replay` 回放复查；block／warn 事实随 tool 结果进入 journal。

## 写入管控（0cc v3＋319 加固）——能力与边界

**定位：高阻力＋强审计，非绝对保证。** 按「保证／阻力／审计」三档如实表述：

### 保证（机制可证）

- **工具面（`check_write_target`）只硬拒宿主状态窄目标**：
  - `.gsa` 会话卷**及其祖先目录本体**（`carrier:session-volume`／`carrier:session-volume-ancestor`）；
  - ACAF keystore 根（目录子树）与 signer manifest 本体，**及其祖先链**（`ORZ_ACAF_KEYSTORE`／`ORZ_ACAF_MANIFEST` 同源解析；未配置＝该窄目标不设防，与 ACAF「未配置即无票据」同形）。
- **命令面（`review_command`）block 规则恰五条封闭枚举**：
  1. `catastrophic-recursive-delete`——删除动词＋递归旗＋目标为卷根／根本树根本体（Windows：`C:\Windows`、`C:\Program Files`、`C:\Program Files (x86)`、`C:\ProgramData`；Linux：`/boot` `/etc` `/usr` `/bin` `/sbin` `/lib` `/lib32` `/lib64` `/var` `/dev` `/proc` `/sys`）；**319 起还覆盖**：工作区根（`rm -rf .`）、工作区祖先、含 `*`／`.*` 的无约束通配、受护根内通配（`rm -rf /usr/*`）、`find -delete`／`find -exec rm`、`xargs rm -rf`、`cd` 进入受护目录后的 `rm -rf *`，以及 `cmd /c`／`bash -c`／`powershell -Command` 内容位一层递归；
  2. `raw-device-write`——裸设备／卷毁写（`/dev/null` 显式豁免）；
  3. `boot-firmware-flip`——引导固件与安全机制翻转；
  4. `registry-hive-delete`——注册表蜂巢删除；
  5. `carrier-write`——宿主状态写：`.gsa`（含祖先链）＋keystore/manifest（含祖先链）。
- **维护 CLI 双门（319）**：命令面直接 block `orz archive`／`unarchive`／`delete-session`／`trust` 与 `orz rollback restore`（`carrier-write`，含 `cmd /c` 等内容位递归与 `ORZ_MAINTENANCE_TICKET` 引用）；同时 `orz-bin` 各入口要求桥注入的 `ORZ_MAINTENANCE_TICKET` 能力令牌（长度 ≥16，缺失/空白/过短 exit 2，fail-closed；桥 spawn 时同键注入）。TTY 交互确认（双保险）未落——余项 0dl。
- **不再宣称**：v2 的「系统核心路径硬拒」与「载体自保护集」已退役——工具面**不**再硬拒 systemroot／`Program Files`／`ProgramData`，也**不**再硬拒 `/boot` `/etc` `/usr` `/lib*` `/bin` `/sbin` `/dev` `/proc` `/sys` 的普通写入；orz 安装目录、三件套（`orz`/`orz-signer`/`orz-acaf-provision`）与 `grok-home` 也不再是工具面硬拒目标。一般性写动作（装 `/usr/local/bin`、编辑 `/etc` 下文件、`>/dev/null`、删过期补丁、写三件套所在目录）放行，交回审批组件。
- 规则 1 的树根身份仍保留：`rm -rf /etc`（目标恰为树根本体）仍 block；但 `/etc/foo`、`/usr/local/bin/x` 的普通写入/删除不受该规则保护。

### 阻力（高成本）

- 命令面审查是 **best-effort 词法闸**，不解析完整 shell 语法：变量拼接、命令替换、别名、脚本内命令、`.NET` 直调、`eval`、编码混淆等**不保证覆盖**；内容位递归最多 3 层。
- 规则 1 的 319 加固形态（`.`／通配／受护祖先／`find -delete`／`xargs rm`／`cd` 链／维护 CLI）以落码与测试为准；**最终读数与残余项以 319 修复批报告为准**。
- 维护子命令已有命令面 block＋桥令牌门；TTY 交互确认未落（0dl 余项），令牌泄漏/同用户进程环境读取仍属残余风险。
- Windows 8.3 短名、`subst`、junction、UNC 的未存在面不保证拦截；命令审查不构成沙箱。
- 权限默认 **yolo 自动放行**（写／命令／网络默认批准、不逐次弹窗）；交互审批面已接线，但普通写默认不逐次询问。该缺省是明确保留的高风险便利姿态：ACAF 票据不证明模型动作符合用户真实意图；处理不可信输入时请显式使用 `--no-yolo`／`ORZ_YOLO=0`，或在外部沙箱中运行。

### 审计（事后可查）

- block／warn 事实随 tool 结果进入 hash-chained journal；warn 在结果头部附 `[写入管控·提示]` 行；可经 `--replay` 复查。
- 命令被 block 时返回机械文案（含规则 id 与目标），模型面可见；keystore／manifest 臂不携带备份指引，`.gsa` 臂附备份/暂存指引。

## Linux Landlock（已实施，边界）

- **已实施**：0bw①（2026-09-27）＋0ch v4（2026-10-01）子进程写面守卫——`run_terminal_cmd` 子进程在 `pre_exec` 装 Landlock ruleset，只授权装配期计算的写面；`/boot` `/dev` `/proc` `/sys` 四树为内核面排除集（不授权），`/dev/null` 等 7 个安全节点做文件级放行；`/etc` `/usr` `/lib*` `/bin` `/sbin` 已随 0cc v3 收窄放行。
- **边界**：仅覆盖 `run_terminal_cmd` 子进程写面，不覆盖工具面文件写（那是 L1 `check_write_target` 的判定面）；内核 <5.13／Landlock 不可用／`/` 枚举失败时**跳过装挂（fail-open）**，只剩 L1/L2；非 Linux 平台无 Landlock。它不是「一切写入的沙箱」，也不替代审批组件。

## 已知残余绕过面（如实登记；0dl 余项）

以下为 318 审查确认、319 修复批后仍存在或需继续核证的形态（319 已落部分以修复批报告为准）：

1. **非删除动词的破坏**：`sed -i`／`perl -pi`／`python -c "shutil.rmtree(...)"`／`robocopy /MIR`／`rsync --delete` 等不属规则 1 删除动词集；工具面只保护 `.gsa` 与 keystore/manifest，工作区级毁灭不在写控承诺内。
2. **维护 CLI 令牌面**：命令面 block 与桥注入 `ORZ_MAINTENANCE_TICKET`（≥16）已落；TTY 双保险未落。令牌泄漏或同用户进程读取桥环境仍属残余风险（不防同用户恶意进程为既定边界）。
3. **`.gsa` shell 读面**：shell 读 `.gsa` 不走会话卷窗口门（读全开放为既定口径；「模型可读」与「窗口门」的差异如实登记）。
4. **词法审查旁路**：变量拼接、命令替换、脚本内命令、别名、`.NET`/解释器直调、编码混淆、Windows 短名／junction／UNC 未存在面等不保证覆盖。
5. **Landlock 边界**：仅子进程写面、内核 <5.13 或装挂失败即 fail-open；不是全系统沙箱。
6. **运行形态**：真机直跑、无沙箱、权限默认 yolo；不防已失陷的签发器、同用户恶意进程、内核级失陷与恶意模型服务。

上述 2–4 项的余项（维护 CLI TTY 双保险、`.gsa` shell 读面窗口门、词法旁路负向钉）与真机 S3/S4 登记在 BACKLOG `0dl`；在 319 报告收口前，不得把本文件读成「这些形态已全部拦截」。

## 调试开关

`ORZ_ACAF_FAIL_CLOSED=0` 可临时关闭安全层（降低防护等级），不推荐用于正式工作；`ORZ_GSA_HYGIENE=0` 只关收卷删源闸，不关打包闸。

## 报告建议包含

- orz 版本与平台（`orz --version`；OS 版本与内核版本，Landlock 面需后者）；
- 复现步骤与命令原文；
- 预期与实际行为；
- 涉及面（读面沙箱／写入管控规则 id／ACAF 票据链／journal 完整性／Landlock 装挂读数）；
- 是否使用 `ORZ_ACAF_FAIL_CLOSED=0` 等降低防护的开关。
