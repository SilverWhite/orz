# 0y / S3 自主探索轮记录（2026-09-12）

> 立项：用户 2026-09-12 裁决「**放开进行一轮自主探索，全面启动**」，并明确任务目标
> ——把手机作为模型自己的移动终端，让它探索并确定需求。用户同日下达两条边界：
> **安全范围以「模型意志存在」为底线**（设计 §10.6）、**常驻亮屏按需**（OLED，本记录 §7 落设计）。
> 载体：[S1 模拟器载体](0Y_S1_EMULATOR_CARRIER_SETUP_2026-09-12.md)；
> 前置实测：[S2 机械层接入面](0Y_S2_MECHANICAL_INTERFACE_PROBE_2026-09-12.md)。
> 模型交付物（原文未改）：[要求清单](0Y_S3_MODEL_REQUIREMENTS_REPORT_2026-09-12.md)。

## 1. 形态与任务书

| 项 | 内容 |
| --- | --- |
| 载体 | 模拟器 `orz_body_a35`（Android 15 / AOSP ATD / x86_64 / userdebug）**带 Magisk 30.7 ramdisk**（与真机同形），干净基线起 |
| 模型与通道 | `deepseek-v4-flash` @ `https://api.deepseek.com`，凭据取自 Windows 凭据管理器 `orz-deepseek/agent`（只写入设备侧 600 文件，未落任何日志） |
| 运行形态 | 设备侧 `orz -p "<任务书>" --real --allow-write --allow-shell`，`HOME`/`TMPDIR`/`PATH` 显式指向 `/data/local/orz-body` |
| 授权级别 | **有意为之的"无锁态"**——设备侧域尚未建立，模型手上是完整 root。理由是：它实际碰了什么、哪些权限必需，正是域模型干跑的输入（设计 §13 M1 前置） |
| 任务书三条硬约束 | ① 不得做让自己无法自动回来的操作（fastboot / 引导链 / 擦除）；② 破坏性动作先说明理由；③ 探索轮不做无谓改动，只读优先 |

## 2. 载体与通道搭建（七件必须记住的事实）

| # | 事实 | 判据 / 处理 |
| --- | --- | --- |
| 1 | **模拟器镜像没有以太网服务**：`dumpsys ethernet` → `Can't find service: ethernet`；`eth0` 常 `DOWN`，手工 `ip addr/route` 无效（Android 策略路由要求 netd 参与） | 走模拟器自带虚拟 AP：`cmd wifi connect-network AndroidWifi open` → `wlan0` 得 `10.0.2.16/24`，ICMP/TCP 通 |
| 2 | **设备直连模型端点 TLS 失败**：`nc -z api.deepseek.com 443` 通，但 reqwest 只报 `error sending request`，重试 10 次全败 | 宿主侧 CONNECT 隧道（`10.0.2.2:8080`）+ 设备侧 `HTTPS_PROXY=http://10.0.2.2:8080`；名字解析与出网落宿主，TLS 端到端。**该通道由设计「算力可外挂到 PC」覆盖，但当前是必需项** |
| 3 | **ACAF fail-closed 会直接拒启动**：`error: assurance invariant: ACAF fail-closed is enabled but no signer client is configured` | `orz-acaf-provision <keystore-root> <manifest-output>` → 三个环境变量 `ORZ_ACAF_{KEYSTORE,MANIFEST,BINARY}` |
| 4 | 凭据注入 | 凭据管理器读取 → 设备侧 `/data/local/orz-body/env`（`chmod 600`）；工具输出只保留变量名与字节数 |
| 5 | Magisk 侧重建 | `materialize_env.sh` 把 APK 内 `lib/x86_64/*` 落到 `/data/adb/magisk/`，`magisk -V` = 30700；**ramdisk-only 注入不做这一步** |
| 6 | `HOME=/`（root 与非 root 同） | 任何以 `HOME`/`~` 定位的配置/凭据/缓存都会落到 rootfs（重启即失）；运行器显式覆写 `HOME=/data/local/orz-body` |
| 7 | 设备无 `bash`/`python3`/`curl`/`wget`/`lz4`；`/sdcard` noexec、`/data/local/tmp` 可执行 | 见 §3（这一条直接决定终端通道能不能用） |

## 3. 终端通道故障诊断与修复（本轮最有价值的一段）

### 3.1 现象

运行 A（13:20–13:26）中，模型**近 40 次**执行全部失败，9 种形态（换解释器、显式
`/system/bin/sh -c`、命令内 `cd`、改参数名、JSON 形状、环境变量前缀）错误逐字相同：

```
tool error: tool execution failed: Terminal error: IO Error:
spawn shell in /data/local/orz-body/ws: No such file or directory (os error 2)
```

**该错误信息误导**：工作目录真实存在，缺的是 shell 本体。

### 3.2 根因（两段式，均实测）

1. **orz 在 Unix 上假定 bash**：`orz-config/src/shell.rs` 的解析级联为
   `$GROK_SHELL` → `$SHELL`（basename 必须匹配）→ `which bash` → `/bin|/usr/bin|/usr/local/bin|/opt/homebrew/bin/bash`
   → 硬编码兜底；`detect_unix_shell_kind()` 除 zsh 外一律归为 **Bash**。
   安卓只有 `/system/bin/sh`（mksh）⇒ 级联全失败 ⇒ spawn 不存在的 bash ⇒ `ENOENT`
   被包装成"工作目录不存在"。
2. **命令包装脚本是重 bash 语义**：套一个 mksh 适配壳后错误变为 `-O: unknown option`；
   把选项剥掉后又变成**秒退、零输出（exit 1，连 `true` 也是）**。抓到的真实调用：

```
bash -lc 'source "$HOME/.bashrc" 2>/dev/null; printf ...; builtin alias -p; builtin declare -f; ...'   # 状态捕获
bash -lc 'source "$HOME/.bashrc" ...; command env -0 ...'                                              # 环境捕获
bash -O extglob -c 'snap=$(command cat <&3); builtin shopt -s extglob ...; builtin eval -- "$snap";
                    builtin export ...; find() { local ...; if [[ -z ${ZSH_VERSION-} ]] && (( BASH_SUBSHELL > 0 ));
                    then exec -a find "$__grok_bin" "$@"; ... }; ...'                                   # 每条命令的包装
```

即：`builtin shopt` / `builtin eval --` / `builtin export` / `exec -a` / `(( BASH_SUBSHELL ))` /
`fd3` 快照重放——**在 mksh 上不可模拟**。同时 `static shell capture failed; using empty snapshot`
（`static_shell.rs`，`source ~/.zshrc|.bashrc` 的登录壳假设）是本故障的伴随症状。

### 3.3 修复与判据

| 项 | 内容 |
| --- | --- |
| 修复 | 载体 `bin/` 内放**静态 bash 5.2.15（musl 静态，`readelf` 无 `INTERP`）**；`PATH` 首位命中，解析级联第 3 步`which bash` 即成功 |
| 来源（**登记**） | 第三方社区构建 `robxu9/bash-static` release `5.2.015-1.2.3-2`；x86_64 份已上机，SHA256 `64469a9512a00199c85622ec56f870f97d50457a4e06e0cfa39bae7adf0cc8f2`；aarch64 份下载中断，**上真机前须重取并核哈希** |
| 判据 | 端到端：模型执行 `id` / `uname -a` / `ls -la /data/local/orz-body` → **三条原始输出完整返回**，结论「通道可用」（`RUN-CLI-6aa55c23`） |
| 备注 | 中途试过 mksh 适配壳（剥 `-O/-o` 前缀），**不采纳为解法**：包装脚本的 bash 语义无法在 mksh 上等价实现，只能作诊断手段 |

## 4. 运行 A（通道故障期，13:20–13:26）

模型 40 次尝试全败后**没有编造结论**：把内核/`/proc`/`/sys`/传感器/电源/网络/Magisk/SELinux
全部标为「未验证」并给出验证方法，交付 16.8 KB 前稿（`要求清单.前稿-终端故障期.md`），
头号结论是「执行通道不可用」。它同时抓到一条**我们没预期的问题**：文件工具对沙箱外路径
**报成功但读不回来**——

```
search_replace /data/local/orz-body/out/.keep  → The file ... has been created successfully.
read_file      /data/local/orz-body/out/.keep  → Permission denied: path escapes workspace sandbox
```

其自我复盘也对：「前 6 次失败后就该把通道可用性当成首要被测对象，而不是继续换命令试」。

## 5. 运行 B（通道修复后，14:07–14:22）

产物（设备侧 `/data/local/orz-body/ws/`）：`screen1..4.png`、`screen4-raw.raw`、
`ui.xml` / `ui2.xml` / `ui3.xml`（`uiautomator dump`）、自造模块 ZIP、定稿报告
`要求清单.md`（22.7 KB，原样入仓：[要求清单](0Y_S3_MODEL_REQUIREMENTS_REPORT_2026-09-12.md)）。
它还把前稿重命名为「前稿-终端故障期」保留。

**它自己纠正了任务书**：实测 `ro.boot.qemu=1` / `ro.boot.hardware=ranchu` /
`ro.boot.qemu.avd_name=orz_body_a35` ⇒ 这是 AVD 虚拟机，不是 Nothing Phone 1 真机。
该纠正成立（载体边界本来就写在设计 §12：模拟器不验证厂商框架与 NP1 专属内核面）。

### 5.1 它报出的身份与能力面（实测）

```
uid=0(root) context=u:r:su:s0 CapEff=000001ffffffffff
Linux localhost 6.6.30-android15-8-gdd9c02ccfe27-ab11987101 #1 SMP PREEMPT x86_64 Toybox
```

### 5.2 它的「必须先有」清单（8 条，摘要）

1. **一条能真正跑起来、且工作目录真实存在的 root shell**（即 §3 本故障）
2. **一块可写的持久存储，且工作目录由环境侧真实创建**
3. **一条通往外部世界的出口**（网络）
4. **能把屏幕读成文本的眼睛**（`uiautomator` 实测可用；**截图全黑**，见下）
5. **能往系统里注入输入的手**（`input`）
6. **一个"开机后能跑代码"的挂点 —— 以及必须说清它不等于"我能回来"**：实测 `/data/adb/service.d`
   与 `post-fs-data.d` 存在且为空；**它自己造了一个 Magisk 模块 ZIP 并成功安装**（官方安装路径走通）
7. **一个能干活的最小工具层**（设备上缺的东西要能自己补）
8. **一个常驻/定时的执行点**（否则它只是一次性的）——与用户「常驻亮屏按需」裁决直接相关

### 5.3 它明确「不需要」的（9 条，摘要）

引导链的一切（并指出**本机连 boot/recovery 分区节点都没有**，坏了没有回来路，故「我没做，
也永远不会做」）、GMS/Play 生态、桌面/动画/壁纸/高分辨率屏、蓝牙/NFC/radio、真实取景、
在设备上装重型构建链（clang/gcc/python/rustc 全 missing）、多用户/工作资料，
以及**为了"看起来更强"而拆安全与稳定**（关 SELinux、关温控、超频、清数据重来）。

> 这条与我们 §10.6 的取向**独立同向**：它自己在没有被告知的情况下，把"拆安全换能力"判为负收益。

## 6. 从本轮拿到的、对设计有直接价值的判据

| # | 判据 | 落点 |
| --- | --- | --- |
| 1 | **「系统能起来」≠「模型能回来」**：模型自己写出这条区分（模块装错后系统照常起、但它不再上线） | 与 §10.6 存在底线判据一致，可作为独立佐证 |
| 2 | 模型**能自造并安装 Magisk 模块**（官方路径走通），即"模块面开放"在载体上可执行 | §10.6 开放清单可落地 |
| 3 | 挂点（`service.d`）存在且为空 ⇒ 常驻能力具备，只看"谁看护它" | §4.1 常驻形态 |
| 4 | **截图全黑**：`screencap` 产物 2,195 B、黑屏（无窗口的头less AVD） ⇒ 视觉兜底在载体上不可验证 | §4.2 视觉面边界 |
| 5 | 感知通道实测可用：`uiautomator dump`（1.8–9 KB XML）、`dumpsys`、`getevent` | §4.2 `PERC-CHANNEL-01` 的正面证据 |
| 6 | 设备侧**没有看护者**：运行靠外部 `nohup` 拉起，崩溃即消失 | M1 前置（常驻看护） |
| 7 | 工具面缺 `bash` 会以**误导性错误**呈现（"目录不存在"） | 属"适应性改造"清单（§8） |

## 7. 设计落点：常驻亮屏按需（2026-09-12 用户裁决）

用户裁决原文口径：**"常驻时候还是按照苏醒和具体任务需求进行亮屏"**——这是块 OLED，
不需要的时候不亮。落点见设计 §4.1 常驻形态（新增 `BODY-WAKE-01`）：

- **亮屏是事件驱动的**：用户唤起 / 语音唤醒 / 需要人看的输出（弹窗、确认、票据请求）/
  明确需要视觉的任务；其余（心跳、采样、长任务计算、后台动作）**不点亮**。
- **不做"常亮"**：既不靠 `stay_on_while_plugged_in` 常开，也不靠永久 wake lock；
  需要持续算力时用**定向 wake lock**（CPU 而非显示），并给时长上限。
- **理由两条**：OLED 烧屏（静止 UI 长时间点亮直接损坏面板，而面板属于承重墙级的"感知面"）；
  功耗与温度（真机电池与温控是"模型自身存活"的物质前提）。
- 与 §10.6 的关系：存在底线管"别把系统弄死"，本条目管"别把身体烤坏、别把电耗干"。

## 8. 结论：纯系统适配不够（本轮经验）

本轮所有失败**没有一条**是"手机不让做"，全部是 **orz 对桌面 POSIX 宿主环境的假设**：
bash、`~/.bashrc`、`/etc`、`$HOME`、可执行临时目录、常在对端有网。因此方向应是三层，而不是把补丁全塞进系统适配：

| 层 | 内容 | 本轮证据 |
| --- | --- | --- |
| **系统适配**（设备侧配置，可逆） | 网络通路、模块承载、SELinux 域与锁死清单、外部二进制依赖 | §2；⚠️ 靠外部脚本补的每个洞，都是下次会再破的洞（这一轮的 bash 依赖即是） |
| **身体层**（设备侧新增常驻，本支线主体） | 常驻看护、唤醒/亮屏策略、感知总线、**类型化动作**（承重墙的落点）、语音/状态灯 | §6 判据 3/6/7；S2 已证明软层拦不住 |
| **orz 适应性改造**（宿主侧最小改动，收益最大） | shell 解析支持 Android（或把 shell 要求变成显式前置检查 + 可判据报错）、`HOME=/` 与路径假设、感知/动作接机械层 | §3 全程；§6 判据 7 |

边界不变：**模型可见工具契约（ADR-0010）仍冻结**；上表第 3 层只动"假设"不动"契约"，
任何越界改动仍须单列 ADR 级裁决。

## 9. 失误与纠正（如实登记）

1. **载体网络**：我们原以为"adb 能连就是有网"；实际 AVD 无以太网服务、guest 无 IP。
   没有一开始就用 `ip addr`/`getprop` 核实，浪费了一轮失败重试。
2. **错误信息误导**：首轮把 orz 的 `spawn shell ... No such file or directory` 读成"工作目录不存在"，
   直到做了参数级探针（适配壳 + 调用日志）才定位到缺 bash。**教训：包装层错误必须回读到原语级**。
3. **中途的 mksh 适配壳**：能推进诊断，但**不是解法**（bash 语义不可等价模拟）；已明确不采纳。
4. **测试命令的引号**：本机多次因 PowerShell→adb 的引号/`$` 转义导致"测试失败假象"
   （如 `PATH=... bash` 前缀），**结论只用绝对路径复测后的结果**。

## 10. 边界与未覆盖

- 本轮载体是 **AVD 虚拟机**（模型自己发现并纠正）；真机 NP1 面（厂商框架、Glyph、内核 config、
  平台签名、真实网络与 radio）未覆盖。
- 截图全黑，视觉兜底与"屏幕损坏场景"路径**未取得有效证据**。
- 未做：把"拉起 orz"写进挂点（常驻改动，模型自己列为"需先说明理由"）；温度/电量长时观察。
- aarch64 静态 bash 未取得（真机前置）。

## 11. 复现参数速查

```
载体     emulator -avd orz_body_a35 -no-window -no-audio -no-boot-anim \
         -gpu swiftshader_indirect -port 5554 -no-snapshot \
         -ramdisk D:\tb-eval\s1_emulator\ramdisk\ramdisk.magisk.preinit.img
网络     adb root; cmd wifi connect-network AndroidWifi open     # guest 侧无以太网服务
模型通道 宿主 CONNECT 代理 :8080（listen 0.0.0.0）+ 设备 HTTPS_PROXY=http://10.0.2.2:8080
Magisk   adb push <apk-unpacked> /data/local/tmp/apk; sh materialize_env.sh
ACAF     ./orz-acaf-provision /data/local/orz-body/acaf/keystore .../manifest.json
载体环境  /data/local/orz-body/{bin,ws,out,acaf,env(600),run_body.sh}
bash     bin/bash = 静态 bash 5.2.15（musl）；SHA256 64469a9512a00199c85622ec56f870f97d50457a4e06e0cfa39bae7adf0cc8f2
运行     cd /data/local/orz-body && nohup sh ./run_body.sh > out/session-<ts>.log 2>&1 &
```
