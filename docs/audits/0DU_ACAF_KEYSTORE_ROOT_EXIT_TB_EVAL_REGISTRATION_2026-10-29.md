# 0du 立项登记：ACAF 密钥库根迁出测评目录（tb-eval 清理事故后续）（2026-10-29）

> **性质**：零代码登记批（计数 67 → 68）；**用户裁决**＝凭据（ACAF 信任锚）从测评目录迁出，修改进排期；用户自行重启在役栈接着使用。
> **来源**：2026-10-29 凌晨用户清理 `D:\tb-eval` 及相关缓存后，活体会话票据类动作全拒（`[票据拒绝] : signer_unreachable`），会话其余面运行正常；用户问"是否有东西绑在 terminal bench 组件下面"——勘定结果＝绑定的不是评测组件，而是**登记的 Windows 载体位及其上的 ACAF 密钥库根**。

## §1 事故现场（2026-10-29 实测）

- `D:\tb-eval` 清理后仅余 `scbench\jobs-0cw`；`orz-windows`（载体位）、`orz-windows-0.9.14-4`（暂存位）、`orz-linux`（Linux 载体位）、`buildlogs`、`s1_emulator` 等全部随删。
- 在役栈：会话 `224b39bd`（工作区 `C:\Users\1\OneDrive\Desktop\新建文件夹`），bridge `orz.exe web --real`（PID 21864）＋ child `orz.exe --stdio`（PID 21268），均自**装机位** `%LOCALAPPDATA%\orz\bin`（0.9.14-4，379 件）起；**无 signer 进程**。
- journal 坐实（`RUN-224b39bd-5` events.jsonl）：每条 `run_terminal_cmd` 配对 `control_ticket_rejected`（`ticket_kind=command_exec_v1`、`reject_code=signer_unreachable`、`detail=io error: 管道正在被关闭。 (os error 232)`）；`read_file`/`grep` 等读面工具与会话/检索不受影响——fail-closed 只覆盖持票据的动作面，即"运行正常"的机理。

## §2 根因链（勘定）

1. **用户级 env 三指针**（379 批设置、注册表持久）：`ORZ_ACAF_KEYSTORE=D:\tb-eval\orz-windows\acaf\keystore`、`ORZ_ACAF_MANIFEST=D:\tb-eval\orz-windows\acaf\signer-manifest.json`、`ORZ_ACAF_BINARY=D:\tb-eval\orz-windows\orz-signer.exe`——均指**载体位**。
2. 两个启动器缺省参数同指载体位：[`dogfood_launch.ps1`](../../scripts/dogfood_launch.ps1)／[`orz_acp_launch.ps1`](../../scripts/orz_acp_launch.ps1) 的 `$BinDir = 'D:\tb-eval\orz-windows'`、`$AcafRoot = 'D:\tb-eval\orz-windows\acaf'`。
3. 载体位删除 ⇒ 签名器按需拉起时密钥库根缺失、启动即退（0by 106 批同款：`installation key root is not a directory`）⇒ 管道关闭（os error 232）⇒ 每条票据 fail-closed 拒 `signer_unreachable`。**与 0by 的差别**：0by 是装配点落点错（新 bin 目录暴露），本次是在役密钥库根被运行期删除；0by S2 装配门只在**启动时**断言，运行期密钥库根消失无探测、无横幅——栈带着死票据道继续跑。
4. 运行时不受影响的面：装机位三件（0.9.14-4 完好，含全版本 `*.0.9.x-bak`）、工作区 `.gsa`（会话/journal/runs/侧车）、模型 API 面——均不在 tb-eval。

## §3 影响面盘点（按 git 记录；回答"仅凭据受影响？"）

- **对在跑系统：是**——唯一受影响的是 ACAF 信任锚（keystore＋manifest＋signer 二进制指针）的绑定路径；会话/journal/装机位均不依赖 tb-eval。
- **资产面不止凭据**（随删清单，按记录）：
  - 可复原（有 pin/权威/哈希）：载体位三件与载体清单（重建批例行，批档存全部哈希）、harbor 浅克隆（`scbench/harbor`，pin `31ceea3a`）、`0cr_official/manifest.json` 机器副本（权威＝196 批档 §3 表）、SCB 论文迁入件可按 272 批记录重迁。
  - **永久丢失（本地不入仓件）**：SCB 论文源件迁入副本（`scbench/scb-paper-2603.24755`，46 件；读数与结论已入 272 批档）、`s1_emulator` 证据根（0Y，691 MB／59 日志／16 引导；判据 1–7 结论已入 0Y 批档）、TB 驱动脚本、`0am_s3`／`probe-0ac-s1` 读数仓、jobs 取证、buildlogs、dogfood 隔离工作区、`orz-cache/cargo-target` 缓存。读数均已入档，无账面缺口。
- **scripts 内其余 tb-eval 硬编码**（使用时即断，属同族面、随 0du 分票处理）：`dogfood_launch.ps1`／`orz_acp_launch.ps1` 内 `protoc = D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`、`freeze_rli_corpus.ps1` 缺省冻结输出根、`gate_google_observe.ps1` 全套（.env／orz-linux／venv harbor／jobs-gate／gsa-volumes）、`__wtest.ps1`。

## §4 0du 迁移范围（排期；实施批待用户令）

1. **新根**：候选 `%LOCALAPPDATA%\orz\acaf`（装机位旁、随安装走，脱离测评目录）；`ORZ_ACAF_BINARY` 改指装机位 signer。
2. **用户级 env 三指针**改指新根（注册表持久面；改后**新起进程**生效，在跑栈须重启）。
3. **两启动器缺省**：`$AcafRoot` 改指新根；`$BinDir` 缺省改指装机位 `%LOCALAPPDATA%\orz\bin`（载体位语义移入重建批显式传参）。
4. **重 provision＋验证**：新根现场 provision（换装批先例＝常规操作，会话/journal 为 sha 链自证、不依赖密钥身份）；装配门＋doctor 复验。
5. **顺带评估**：运行期密钥库根消失的观测面（0by S4 边界；本次事故即该缺口的实机样本——栈不重启则票据道静默死亡）。
6. scripts 其余 tb-eval 硬编码逐票处理（§3 清单；`gate_google_observe` 所依赖的 `orz-linux` 载体位若还需使用须重建或改指）。

## §5 过渡解锁（用户自行执行，两法任选）

- **A（不动配置）**：旧位重 provision——`orz-acaf-provision.exe D:\tb-eval\orz-windows\acaf\keystore D:\tb-eval\orz-windows\acaf\signer-manifest.json`（目录现场新建），env 三指针不动，重启栈后票据恢复；凭据暂留测评目录，等 0du 实施批迁出。注意：缺省启动器缺省 `$BinDir` 仍指已删载体位，须传 `-BinDir %LOCALAPPDATA%\orz\bin` 或直启装机位。
- **B（顺手完成迁移主体）**：新根 provision＋改 env 三指针（PowerShell `[Environment]::SetEnvironmentVariable(…,'User')`），重启栈即生效；0du 实施批只剩脚本缺省与 scripts 残余硬编码。

## §6 S1 实施记录（2026-10-29，用户令「直接进行0du」）

- **启动器落码**（两件同形）：`$BinDir` 缺省 `D:\tb-eval\orz-windows` → `"$env:LOCALAPPDATA\orz\bin"`（装机位）；`$AcafRoot` 缺省 → `"$env:LOCALAPPDATA\orz\acaf"`（新根）；`PROTOC` 隐式缺省退役 → `-Protoc` 显式传参（缺省不设；原 protoc 25.3 随 tb-eval 清理删除，工作区内 orz 源码构建链如需 protoc 须显式装配或另行登记 rebuild-infra 缺口）；头部说明补 0du 注。
- **新根 provision**：`%LOCALAPPDATA%\orz\acaf\keystore` ＋ `signer-manifest.json`，exit 0；`binary_sha256=3802e2f2…` ＝ 装机位 signer（orz-signer.exe）实哈希逐位一致 ＝ 379 批账面值（同件）。provision 取 signer 的规则＝**工具同目录**（`signer_binary_path()` exe 邻接），自装机位调用即哈希装机位 signer，无需额外装配。
- **用户级 env 三指针已改**（注册表 User 域，读回核验）：`ORZ_ACAF_KEYSTORE=C:\Users\1\AppData\Local\orz\acaf\keystore`、`ORZ_ACAF_MANIFEST=…\signer-manifest.json`、`ORZ_ACAF_BINARY=C:\Users\1\AppData\Local\orz\bin\orz-signer.exe`。**只对新起进程生效**：在跑栈（bridge 21864/child 21268）仍持旧死指针，须停栈后**新终端**重启（旧终端 env 是旧的）。
- **验证全绿**：① signer 生产同款冒烟（宿主子进程 env 形＝`ORZ_SIGNER_MANIFEST`＋`ORZ_SIGNER_KEYSTORE_ROOT`，见 `orz-loop/src/acaf.rs` spawn）——新根 exit 0（manifest→自哈希→keystore 启动链全过、服务正常起停）／删除路径负对照 exit 1 fatal，与事故链逐环对齐；② `orz_acp_launch.ps1 -DryRun` 装配门过（新缺省、装机位三件、`sha256 18fd19de…`＝379 账面）；③ `dogfood_launch.ps1 -DryRun` 装配门过（`--build-info` 报 v0.9.14-4；`GROK_HOME` 顺带落到装机位 `grok-home`〔在册〕；PROTOC= 空＝新缺省）；④ `orz doctor`（装机位槽，新指针）`ok=10 warn=2 fail=0`——与 379 装机位基线逐字一致，两 warn 环境性（槽内无 run journal／无 carrier-manifest）。
- **§4.6 裁决落地**：用户令「跑分已经结束，丢了就丢了吧」——eval 线脚本（`gate_google_observe.ps1`／`freeze_rli_corpus.ps1`／`__wtest.ps1`）的 tb-eval 硬编码**不修**（评测域脚本本就以测评目录为家；依赖件已删、按用户裁决接受）。两启动器内 protoc 行已随 S1 处理（见上）。
- 活体复验（S2）＝用户停栈重启后票据道恢复（`run_terminal_cmd` 出正常回执）；复验通过前 0du 保持开放（计数 68 不变）。

## 入口

- BACKLOG `0du`（开放项计数 68）／TODO 0du 条目／索引头部 v4.342。
- 关联：`GAP-ACAF-SIGNER-UNREACHABLE`（0by，同错误签名先例）／379 批档（env 三指针设置处）／索引"不可触碰禁区"行（tb-eval 为已登记合法清理目标，本事故不违反）。
