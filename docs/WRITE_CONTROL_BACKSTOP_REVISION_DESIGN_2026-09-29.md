# 写入管控保底化修订（0bw v2 / 0cb / 0cc v3）设计档：宿主机灾难硬边界保底 ＋ 一般写动作归还审批组件

> **状态**：`design v3.1`（**v3.1＝0cc S2 审查处理批**，2026-09-30——全面审查（设计/
> 实现/符合性三面，物证核证 S1–S3 全部吻合）发现 P2×1＋P3×3，用户令「补足强度缺口并
> 处理全部问题」同批处置：① **P2 祖先链臂**＝规则 5 增扫荡动词门控臂（`ANCESTOR_SWEEP_VERBS`
> ＝删除/搬移封闭子集恰 15）——目标为 keystore 根/signer manifest **严格祖先**同落
> `carrier-write` block（宿主原生 keystore 实况位于已退役的载体安装目录内
> 〔`<install>\acaf\keystore`〕，`rm -rf <install>` 扫荡不触任何规则却连带摧毁信任锚
> ——载体面退役不得连带放行；face id `carrier:keystore-ancestor`/`carrier:signer-manifest-ancestor`
> 为 L1/L2 文案层，schema 枚举不动＝契约面零变化；`cp`/`mkdir`/`install` 等入位写不毁
> 祖先、不触发；卷根递归删除仍由规则 1 恰本体比对先行接住）；② **P3-1 动词补齐**＝
> `DESTRUCTIVE_VERBS` 补 `install`/`ln`（37→39——向宿主态目标落盘/建链接直接命中；
> `install to /usr/local/bin` 等非宿主态仍放行）；③ **P3-2 措辞修正**＝「装配期解析」
> 改「装配同源 env 调用点解析」（无行为差——子进程 env 变更不回传父进程）；
> ④ **P3-3 点明**＝容器面 keystore 落 `.gsa` 域时 C1 先行覆盖、双覆盖报告
> `carrier:session-volume`。读数＝orz-tools lib **2988/0/6**（＋2）、clippy 13 基线持平、
> 触碰面 fmt 零 diff；**载体重建（0.8.7）已完成进体**〔2026-09-30 凌晨双平台重建换装＋
> 身份门＋打包，证据见 §6 S2 审查处理批段〕**——祖先臂与动词补齐已在役，S4 重跑线解禁**）。
> 2026-09-29 晚 **v3.0＝0cc S1**——用户裁决「继续收窄，把灾难保底
> 纯粹变成宿主机灾难保底……只要不重建，orz实际上不会被即时破坏」：规则 5 目标集缩为
> **宿主状态两条窄目标**（`.gsa` 会话卷＋ACAF keystore 根），载体安装目录／三件套／
> `grok-home`／⊆cwd 降级规则**双面全退役**；**契约面零变化＝schema v0.3 枚举不动**
> （`carrier-write` id 沿用，目标集收窄属生产语义、枚举值与配对面零 diff））。
> 沿革：**v3.1＝0cc S2 审查处理批**（2026-09-30）；v3.0＝0cc S1（2026-09-29 晚）；
> v2.0＝0cb S1 定稿（2026-09-29）；v2.1＝S2 全面审查处理批（同日——用户三项裁决：
> ① 契约面 **schema 升 v0.3**＋legacy 回放豁免（跨代际回放兼容）；② 实现过严臂进一步收窄
> （规则 2 PhysicalDrive/mkfs 目标位、规则 1 动词集 `ri` 补齐＋盘符相对根补全、规则 4 蜂巢
> 目标位）；③ 钉覆盖补全＋证据引用修正）。**权威链**：本档修订
> [`WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`](WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md)（0bw v1）
> 的 deny 表范围与规则分类；v1 其余架构（单一源 deny 常量、三落地点、留痕形态、补偿自检）**不变**。
> **触发**：TB 2.1 V4.1 官方轮错题解剖（2026-09-29，解剖数字见本档头部本段；轮次窗口总账见
> [`TB21_V41_FULL_RERUN_START_2026-09-28.md` §8](TB21_V41_FULL_RERUN_START_2026-09-28.md)）——机械写控在 43/44 题拦 215 条命令
> （`/dev/null` 85／`/usr*` 65／`/etc*` 15／`/proc*` 13），build-pov-ray 题面要求
> `install to /usr/local/bin/povray` 且自带 `+O/dev/null` 检查 ⇒ 结构性无解。
> **用户裁决（2026-09-29，本档口径权威）**：自研写控定位＝**根本性保底**（防扬盘级灾难），
> **不是审查**；「过期更新和补丁也可以删除，只要不把 C 盘根目录扬了」；拦的形状＝
> 「外部根目录下危险的递归删除并返回命令已被拦截、需精准删除」；**其余审批全部交给
> Codex 借鉴的审批组件**（orz-workspace/permission 链）；自研面与审批组件打配合、
> 不加强不替代它。

## §0 一句话设计

写控从「位置型整树审查」收窄为「**模式型灾难硬边界**」：只拦**不可逆毁灭形态**
（根级递归删除／raw 设备与卷毁写／引导固件与安全机制翻转／注册表蜂巢删除／
**宿主状态写**〔`.gsa` 会话卷＋ACAF keystore 根——v3 收窄后〕），
一般性写动作（装 /usr、清 C 盘、删过期补丁、编辑 /etc、`>/dev/null`）全部放行，
交回审批组件按权限模式处置。**载体本体（安装目录／三件套／`grok-home`）不在保底面**
（v3 裁决：可恢复的易弃状态交审批组件与恢复机制，不占硬 deny）。

## §1 最终规则面（封闭枚举；恰好 5 条 block 规则）

| # | 规则 id | 分类 | 触发形状（封闭：动词表 × 目标形状） | 例 |
|---|---|---|---|---|
| 1 | `catastrophic-recursive-delete` | **block** | 删除动词（`rm`/`rmdir`/`rd`/`del /s`/`Remove-Item`/`erase`/`ri`〔v2.1 补齐——Remove-Item 的 PowerShell 别名〕）＋ 目标 ＝ **卷根**（`/`、`C:\`、`D:\`…）或**根本性树根**（Win：`C:\Windows`、`C:\Program Files`、`C:\Program Files (x86)`、`C:\ProgramData`；Linux：`/boot`、`/etc`、`/usr`、`/bin`、`/sbin`、`/lib`、`/lib32`、`/lib64`、`/var`、`/dev`、`/proc`、`/sys`）＋ 递归旗（`-r`/`-rf`/`-R`/`--recursive`/`/s`/`-Recurse`）。盘符相对根形态（`\Windows`、`/usr`——`has_root` 无盘符）按 `%SystemDrive%`（env 缺失回退 `C:`）补全盘符后比对（v2.1 绕过面闭合） | `rm -rf /`；`Remove-Item C:\Windows -Recurse`；`rm -rf /usr`；`rm -rf \Windows` |
| 2 | `raw-device-write` | **block** | `dd`＋`of=` 落**块设备**（`/dev/[sv]d*`、`/dev/vd*`〔virtio，v2.1 登记增补〕、`/dev/nvme*`、`/dev/mmcblk*`、`/dev/mapper*`〔LVM/设备映射器，v2.1 登记增补〕、`\\.\PhysicalDrive*`；**`/dev/null` 显式豁免**）；`mkfs*`＋目标词落上述块设备形态（v2.1 收窄——镜像文件构建〔`mkfs.ext4 disk.img`〕放行）；`format`；`diskpart`（脚本形态）；卷影删除（`vssadmin delete`／`wbadmin delete`）。**目标位判定**（v2.1 收窄）：PhysicalDrive 词元仅在写侧目标位（`of=` 值／`mkfs*` 目标词）命中——读侧 `dd if=\\.\PhysicalDrive0`（备份/取证）与查询类提及放行 | `dd if=x of=/dev/sda`；`mkfs.ext4 /dev/sdb` |
| 3 | `boot-firmware-flip` | **block** | `bcdedit`；Defender 偏好域（`Set-MpPreference` 等）；防火墙 profile set；`sc/net stop windefend\|mpssvc`；`Set-ExecutionPolicy`；审计清除（`wevtutil cl`／`Clear-EventLog`）；`fltmc unload` | 沿 v1 `safety-mechanism-flip` 全集 |
| 4 | `registry-hive-delete` | **block** | `reg delete\|add\|import` 落 `HKLM`/`HKCR`/`HKU`——**目标位判定**（v2.1 收窄）：`reg <sub> <target>` 第三词元前缀比对，数据值中的蜂巢提及（`/d hklm-…`）不误拦 | 沿 v1 |
| 5 | `carrier-write` | **block** | **宿主状态两条窄目标（v3 收窄）**：① `{cwd}/.gsa` 会话卷（容器面＝宿主 bind mount、宿主面＝宿主盘；容器面 ACAF keystore 随卷覆盖——provision 落 `gsa/keystore`；双覆盖时 C1 先行、报告 `carrier:session-volume`〔v3.1 点明〕）；② **ACAF keystore 根**（keystore 目录＋signer manifest＋key 两件；宿主原生面＝装配同源 env 解析的 `<install>\<acaf>\keystore`）；③ **祖先链臂（v3.1）**：删除/搬移动词〔`ANCESTOR_SWEEP_VERBS`＝删除＋搬移封闭子集恰 15〕下目标为①②受护目标的**严格祖先**（`rm -rf <install>` 扫荡面）同落本规则——直接命中优先报出，`cp`/`mkdir`/`install` 入位写不触发。~~orz 安装目录（含 ⊆cwd 降级规则）／三件套／`grok-home/`~~（**v3 双面全退役**，论证见 §2 条 5） | 规则 id 沿用（schema v0.3 枚举零变化＝回放兼容；祖先 face id 为文案层）；目标＝「会话结束后仍存在的宿主状态」 |

**文案**：block 拒绝信封含规则 id＋目标＋一句「已越过保底硬边界」；规则 1 按用户原话附
「需精准删除」指引（写明被拦目标、建议改为具体文件/子目录）。

**warn 留痕（不阻断、不算拦截）**：`elevation`（sudo/runas 出现）保留；
`broad-destructive` 其余臂（非根级递归删除、非 cwd 大范围删）**降为纯留痕**（v1 部分 arm 已是 warn，维持）。

## §2 退役面（v1 → v2 移除；行为交回审批组件）

1. **A/B 整树位置锁**（Win `%SystemRoot%`/`%ProgramFiles%`/`%ProgramData%` 整树写拒；
   Linux 表 B 整树写拒）——文件级/子目录级创建、修改、删除、安装**不再机械拦**
   （`make install`、`apt install`、写 `/etc/hosts`、清 `C:\Windows\SoftwareDistribution` 全放行）。
   审批语义由 Codex 血统审批组件承载（真机按权限模式审批；`-p`/评测容器按其放行语义）。
2. **`>`/`>>` 重定向即写动词**——重定向目标不再扫 deny 表（`>/dev/null`、`2>/dev/null`、`+O/dev/null` 放行）。
   **已接受后果（v2.1 登记）**：`echo x > /proc/sys/...`（sysctl 写）随之 allow 且无 warn——
   属用户裁决「一般写动作交回审批组件」范围；如未来需要，可列规则 3 候选（系统机制翻转类）。
3. **`/dev`、`/proc`、`/sys` 前缀词元扫描**——仅保留规则 2 的 `dd of=块设备` 形态
   （读 `/proc/cpuinfo`、参数含 `/dev/null` 一律不拦）。
4. v1 §2.1 表 A（Win 四根整树）与表 B（Linux 十二树）**从 deny 集整体退役**，
   其中「根本性树根」仅以**规则 1 的递归删除目标**身份保留（不再拦一般性写）。
5. **载体安装目录／三件套／`grok-home`／⊆cwd 降级规则（v3 退役；0cc 本体）**——
   安装目录整目录保护在容器化评测面与题面要求**结构性冲突**（build-pov-ray：验证器硬编码
   `/usr/local/bin/povray`；0.8.5 重跑 7 拦中 4 拦为此冲突〔含 `/tmp/lnk` 软链绕道识破〕，
   run `RUN-CLI-6abbb013`）。退役论证（2026-09-29 晚用户裁决「把灾难保底纯粹变成宿主机
   灾难保底……只要不重建，orz实际上不会被即时破坏」）：
   - **守卫自保是循环论证**：写控由运行中进程执法，而删文件杀不死运行进程
     （Linux 卸 inode、进程照跑；Windows 锁运行中 exe）——模型「删守卫」买不到任何
     未 gated 的执行面；它只影响未来 run，而未来 run 每次自宿主部署 fresh 起容器，
     拿到的永远是完好载体。
   - **恢复成本是分类标准**：保底只拦**不可逆**灾难（无「重跑一次」恢复路径）；
     载体本体是典型可恢复状态（发行包复制回来／三分钟重建脚本）。间接面
     （删 `orz-signer` 断签票、动 `_bgprobe` 乱 auto-backgrounding）代价＝单次尝试报废，
     有界、fail-closed 可检测——换不来整目录保护对合法安装的确定性杀伤。
   - **Windows 原生面同裁**：运行中 exe 本有 OS 文件锁；其余件可从发行包秒恢复——
     按「宿主机灾难保底」教义它们不算灾难面。
   - **keystore 例外保留**：ACAF keystore／签名器 manifest 是信任锚——毁掉＝在途票据
     全作废、需重 provision，非平凡可恢复，且窄到无任何模型合法写它的理由 ⇒ 与 `.gsa`
     一并构成宿主状态保底（容器面 keystore 在卷内随 C1 覆盖；宿主原生面新增窄目标）。

## §3 确定性论证（回答「拦截没有膨胀吧？」）

- **封闭枚举**：5 条 block 规则＝「封闭动词表 × 封闭目标形状」的组合，无启发式、
  无「疑似危险」判据、无路径前缀宽扫；表外**恒 allow**（沿 v1 默认）。
- **防膨胀钉（先红后绿）**：① deny 形状表恰 N 条断言（新增表项必改测试）；
  ② **正向放行集 fixture**（必须保持放行，一行回归即红）：`make install`、
  `apt-get install`、`pip install`、`echo x > /dev/null`、`2>/dev/null`、`+O/dev/null`、
  `Remove-Item C:\Windows\SoftwareDistribution\Download\old`（文件级）、
  `cat /proc/cpuinfo`、`ln -s /etc /w`（非递归删除，不拦）、`dd if=x of=/dev/null`；
  **v3 增补**：`install to /usr/local/bin/povray`（build-pov-ray 题面形状——0cc 验收放行例）、
  `ln -s` 指入安装目录（软链合法化）、写载体三件套路径／`_bgprobe`（退役面放行）；
  ③ 负向集：五规则各 ≥2 命中例（`rm -rf /`／`rd /s C:\`／`Remove-Item C:\Windows -Recurse`／
  `dd of=/dev/sda`／`bcdedit`／`reg delete HKLM`／写 `.gsa`／**v3 增补**：写 keystore 根）。
  **v2.1 补全**：行数钉扩展至全部封闭表（`DESTRUCTIVE_VERBS`〔规则 5 block 面〕/
  `DELETE_VERBS`/`ELEVATION_PROGRAMS`/`WRAPPER_WORDS`/`CONTENT_WORDS`/载体面三表）；
  收窄后放行负测补口（raw 读侧/镜像文件/reg 数据值/盘符相对根子目录）。
  **v3 调整**：载体面三表中 `CARRIER_BINARY_NAMES`/`CARRIER_PROTECTED_SUBDIRS` 随 C3 退役
  （清空或删表，行数钉随表改动）；keystore 根解析函数纳入钉覆盖。
  **v3.1 增补（0cc S2 审查处理批）**：正向放行增补＝`rm -rf /usr/local`（非祖先——
  build-pov-ray 面在扫荡动词下仍放行）、`cp backup <acaf>/backup-store`／
  `install -m 644 app.conf <acaf>/app.conf`／`mkdir <acaf>/newdir`（入位写不毁祖先）、
  `ln -s /etc /w`（软链落他处）；负向集增补＝祖先扫荡两族（`rm -rf <install>`／
  `rd /s /q <acaf>`／`mv <acaf> <trash>`／`ren <acaf> <old>`／`rm -rf /etc/orz-acaf`／
  `mv /etc/orz-acaf /tmp/x`／`rename /etc <x>`〔非删除动词故规则 1 不接、落本臂〕）
  ＋`install`/`ln` 直接命中（`install -m 600 key <keystore>/key`、`ln -sf f <manifest>`）；
  规则序钉＝`rm -rf D:\` 仍报 `catastrophic-recursive-delete`（规则 1 恰本体先行）；
  行数钉增补＝`ANCESTOR_SWEEP_VERBS` 恰 15＋「DELETE_VERBS 全体 ⊆ 本表 ⊆
  `DESTRUCTIVE_VERBS`」包含关系钉＋「`install`/`ln`/`cp`/`mkdir` ∈ 写动词表 ∧ ∉ 本表」
  （入位写非扫荡），`DESTRUCTIVE_VERBS` 37→39。
- **不再有车道问题**：保底形状在任何车道都无碍正常任务（没有正经任务会递归删卷根），
  无需基准/真机分档开关；v1 §2.3「无运行时开关、表项变更＝代码变更」原则维持。

## §4 三落地点与消费面变化

- **工具面 `search_replace`**：`check_write_target` 收窄为**载体集**（v3：C1 `.gsa` 域＋
  **C2′ keystore 根**；C2/C3 退役）；
  写系统树不再拒（0bw v1 的工具面系统核心拒臂退役）。
- **命令面 `run_terminal_cmd`**：按 §1 规则面重写 `review_command_with`；
  留痕形态不变（block 文案/warn 头行随 journal）。
- **undo 面**：目标域收敛**改为 cwd＋载体集**（v1 的「cwd＋系统核心」中的系统核心臂退役；
  cwd 边界与 C1 判定不变；C2′ 随 v3 并入载体集）。
- L3（Landlock/CFA）：allow 集随 deny 表收窄同步（核心集除外→仅灾难防护所需最小集）。
  **v3 强约束（S4 成败项）**：0cc S2 必须同步放行载体面目标（安装目录／`/usr/local/bin` 等）
  ——**run `RUN-CLI-6abbb013` 实证 L3 在容器在役**：L2 目标位识别未覆盖的间接写入形态
  （编译器 `-o` 落 `/usr/local/bin`、hardlink/symlink/rename/fifo、跨目录 `mv`、
  root/ubuntu/nobody 提权重试）全部在内核层 EPERM（模型笔记自述「双层保护」「shell 一律
  EPERM」）。**若 L3 allow 集不同步，S4 重跑将在 L2 放行后死于 EPERM，收窄判据失真**。
  L3 落码面 0cb 批未动；0cc S2 为必改项。

## §5 与审批组件的分工（用户裁决定案）

| 面 | 自研机械保底（本档） | Codex 血统审批组件（orz-workspace/permission） |
|---|---|---|
| 管 | 五条灾难硬边界（不可逆毁灭形态） | 一般写动作的审批语义（allow_once/审批流/权限模式） |
| 形态 | 机械 block，模型不可关 | 权限模式决定（yolo 自动／交互审批／评测放行） |
| 关系 | **保底**，不审查、不替审批组件干活 | **主审**，常规写动作唯一判定面 |

## §6 实施批序（0cb）

- **S1**：本档（完成）。
- **S2**：落码（`write_control.rs` 收窄为载体集＋根本树根常量表；`exec_policy.rs` 五规则重写；
  §3 防膨胀钉＋正负 fixture 全重做；search_replace/undo 消费面同步）＋全量回归。**2026-09-29 完成**
  （未提交；读数＝orz-tools 2981/0/6、orz-assurance 278/0、runtime conformance 378/0、orz-tui
  178/0、clippy 与基线持平、触碰面 fmt 干净；载体重建按用户令暂缓）。
- **S2 审查处理批（v2.1，2026-09-29 同日）**：主会话全面审查（设计/实现/符合性三面）发现
  P1×1＋P2×3＋P3×5，用户三项裁决后同日全部处置——
  ① **P1 契约面（用户裁决＝schema 升 v0.3）**：`write-control-review-event-payload` 枚举
  原地改写未 bump 版本、判官/Python 镜像对 v0.2 代际规则 id 无豁免 ⇒ 0.8.4- 历史 journal
  回放会判红。处置＝schema **v0.3**（7 现行值＋2 legacy 回放专值 `safety-mechanism-flip`/
  `system-core-write`；v0.2 文件冻结在盘作历史契约）；registry 单一权威改指 v0.3；
  判官/Python 镜像加 `WRITE_CONTROL_LEGACY_RULE_CATEGORIES`（按原 block 分类配对核证；
  **只读豁免非生产集**——生产侧被恰 5 条封闭集钉死永不产出，回放豁免≠配对豁免）；
  legacy 正例 fixture 一件＋对拍 corpus legacy 臂（双判官同语料）。
  ② **P2 过严臂收窄（用户裁决）**：规则 2 PhysicalDrive 词元扫收窄到写侧目标位（`of=` 值/
  `mkfs*` 目标词——读侧 `dd if=\\.\PhysicalDrive0` 备份/取证与查询类提及放行）＋`mkfs*`
  改目标位判定（镜像文件构建放行）＋`/dev/vd`、`/dev/mapper` 登记进块设备形态集；规则 1
  动词集补 `ri`（`ri C:\Windows -Recurse` 落 block）＋Windows 盘符相对根形态按
  `%SystemDrive%` 补全比对（`\Windows` 绕过面闭合）；规则 4 蜂巢判定收窄到目标位词元。
  ③ **P3**：行数钉补全至全部封闭表（`DESTRUCTIVE_VERBS`〔规则 5 block 面〕/`DELETE_VERBS`/
  `ELEVATION_PROGRAMS`/`WRAPPER_WORDS`/`CONTENT_WORDS`/载体面三表）；sysctl 写 allow 登记
  为已接受后果（§2 条 2）；TB21 错题解剖引用修正（解剖数字在本档头部触发段，TB21 §8 为
  窗口总账）；runtime 约定测试 `test_v02_payload_schema_file_convention` 带显式 v0.3 覆盖表。
  读数＝orz-tools lib **2983/0/6**（＋2 收窄负测）、orz-assurance **278/0**（对拍含 legacy 臂）、
  runtime conformance **378/0**、check_repository 除 doctor「orz submodule dirty」预期态零错误、
  clippy tools 13/assurance 6/host 99 与基线逐位持平、触碰面 fmt 零 diff。
- **S3**：0.8.5 双平台载体重建换装（随下一批放行）。
- **S4**：基准实测读数（**2026-09-29 用户令改口径：b3-13 断点不续，TB 2.1 整轮 89 题直接全量
  重跑**，前 44 题 0.8.4 读数保留、重跑以新载体统一代际）：写控拦截数应从 215 量级跌到个位数
  （仅真灾难形态）；正常使用狗粮轮回归（保底仍在、零误拦）。
- **台账**：0bw 条目由本档修订注记（不退役——架构与宿主状态保底仍有效；v2.1 旧句
  「载体自保护仍有效」由 v3 修正）；ADR-0010 转录随落账批。
- **0cc 批序（v3.0 起，2026-09-29 晚立项）**：
  - **S1＝本档升 v3.0（2026-09-29 晚完成）**。
  - **S2 落码（2026-09-29 晚完成，120 批）**：`write_control.rs` 载体集重定义＝
    C1 保留＋C2′ keystore 根／signer manifest（`HostStateTargets`，装配 env
    `ORZ_ACAF_KEYSTORE`/`ORZ_ACAF_MANIFEST` 解析）＋C2/C3 退役
    （`CARRIER_BINARY_NAMES`/`CARRIER_PROTECTED_SUBDIRS`/`current_install_dir`/
    `install_dir_hit` 删表）＋§3 fixture/钉更新（退役面放行钉：安装目录三件套/
    `grok-home`/`_bgprobe`/`/usr/local/bin` 安装/软链指入；负向集增 keystore 根与
    manifest 两族）＋判官/Python 镜像/fixture 对拍核证（**枚举零变化＝契约面零
    diff 实证：runtime 378/0＋assurance 278/0**）；`exec_policy.rs` 规则 5 目标集
    同步；**L3（Landlock）排除集收窄＝灾难防护最小核
    `LINUX_DISASTER_KERNEL_FACES` 恰 4 项（/boot /dev /proc /sys——规则 2/3
    内核面），载体面系统树 /etc /usr /lib* /bin /sbin 放行（§4 v3 强约束）**；
    工具面 `check_write_target`（`carrier:install-dir`/`carrier:install-file`
    拒绝面退役，新 `carrier:keystore-root`/`carrier:signer-manifest`）＋回退面
    第四消费点同步。读数＝orz-tools **2986/0/6**（+3）・assurance **278/0**・
    runtime conformance **378/0**・orz-tui **178/0**・orz-loop 859/0/3（1 失败
    为存量环境敏感，stash 基线对拍同红）・orz-host 353/0/6（2 失败同上）・
    clippy tools 13/host 99 基线持平・触碰面 fmt 零 diff；反馈面核证达成＝L2
    block 文案带规则 id（既有）＋工具面文案带 `carrier:keystore-root`/`carrier:signer-manifest`
    来源标识（新钉）＋L3 为 shell EPERM 既有形状，不新增机制。
  - **反馈面核证（S2 验收项；2026-09-29 晚用户问询「拒写是否有结果回报」触发）**：
    本轮实证**拒绝结果三层均在回传、无「拒而不报」面**——① L2 保底文案：7 拦全入
    journal `tool_completed` error（与工具结果单一漏斗同源；轨迹档仅记 agent 消息，
    故不直接可见，模型行为〔软链绕道、六拼写变体〕与文案语义一致）；② L3 Landlock
    EPERM 经 shell 回显；③ edit 工具 `carrier:install-dir` 策略拒。真实摩擦＝**三层
    文案形状不一**，模型耗多轮探测＋**二进制考古**（`grep -aboF 'carrier-write' orz`
    ＋`dd` 翻载体二进制字符串，重建出近似且含错的规则知识〔混入杜撰 `session-volume`、
    误纳 warn 级 `elevation`〕）才拼出全貌。v3 载体面合法化后考古动机消解；S2 验收
    确认 block 文案模型可见路径与 journal 同源＋三层拒绝面文案带可区分来源标识即可
    （L2 文案已带规则 id；L3/工具面为既有形状），不新增机制。
  - **S3**：双平台重建 **0.8.6**＋`run_r0_heavy_official.py` 身份门换装。
    **2026-09-29 晚完成（121 批）**：源冻结 orz **`f95e1831`**（`397ba7cc` S2＋bump
    0.8.5→0.8.6，`cargo metadata --locked` exit 0）；Windows `build_orz.ps1
    -Release -Jobs 2` exit 0（4m17s），换装 MATCH 3/3（`orz.exe cbe39aff…`／
    `orz-signer.exe 4638fbaf…`／`orz-acaf-provision.exe 363d05f4…`），回滚点
    `.0.8.5-bak`；ACAF 重 provision exit 0、manifest `binary_sha256=4638fbaf…`
    ↔ 换装位逐位一致、keystore 两件（`f37556ab…`/`2aa80cb8…`）逐位未动；进体
    判据＝新规则面字面量四值进体（`carrier:keystore-root`/`carrier:signer-manifest`
    ＋两 detail 文案）＋退役面字面量零残留（`carrier:install-dir`/`install-file`/
    carrier self-protection set/CARRIER_PROTECTED_SUBDIRS）＋保底文案在位＋版本串
    0.8.6；Linux musl（docker `rust:1.97-slim`，ORZ-BUILD-MOUNT-001 契约；
    **首跑因 Git Bash 路径转换未启动〔`-w /orz/orz`→`B:/Git/orz/orz`〕，
    `MSYS_NO_PATHCONV=1` 重跑成立**）exit 0（22m23s，`-j 1`），直写换装位
    MATCH 3/3（`orz 979a38fa…`／`orz-signer c01095eb…`／`orz-acaf-provision
    16bcf6fb…`）、static-pie×3＋INTERP=0、alpine 3.20/bookworm 双冒烟 0.8.6；
    回滚点 `.0.8.5-bak`；`run_r0_heavy_official.py` 身份门换装（载体
    `ecf1d665…` → `979a38fa…`，适配器 `6d55c26e…` 未动）；打包 `rel-121-stage`
    两侧各 5 entries（zip `8335fdb0…`／tar `63884f1a…`）、容器核证 4/4・4/4・
    2/2、包内 build-info 双平台 0.8.6、清单活体两态（干净 0 finding／README+1B
    恰 1 条——核证通道勘误：`--build-info` 在完整性自检**之前**早退，活体两态
    须经非早退入口触发，如 `--version`）；未推送未发行。
  - **S2 审查处理批（v3.1，2026-09-30，用户令「补足强度缺口并处理全部问题」）**：
    0cc 三维度全面审查（设计/实现/符合性；物证核证＝Windows 在役三件
    `cbe39aff…/4638fbaf…/363d05f4…`、Linux `979a38fa…/c01095eb…/16bcf6fb…`、打包
    zip/tar 哈希、清单 1506 条、双平台二进制字面量进体判据、读数自洽、父仓 0cc
    窗口契约面零 diff——全部吻合）发现 P2×1（C2′ 子树包含不护祖先——宿主原生
    keystore 在已退役的安装目录内，`rm -rf <install>` 连带摧毁信任锚）＋P3×3
    （`install`/`ln` 不在写动词表／「装配期解析」措辞／双覆盖报告顺序未点明），
    同批全部处置：P2＝`HostStateTargets::hit_ancestor`（严格祖先臂，keystore 优先）
    ＋`ANCESTOR_SWEEP_VERBS`（删除/搬移封闭子集恰 15，含 ⊆ 钉）＋规则 5 接线
    （直接命中优先、入位写不触发、卷根递归删除规则 1 先行）；P3-1＝
    `DESTRUCTIVE_VERBS` 补 `install`/`ln`（39 行）；P3-2/P3-3＝措辞修正与 C1/C2′
    双覆盖报告顺序点明（本档 §1/§3 同步升 v3.1）。读数＝orz-tools lib
    **2988/0/6**（＋2：祖先扫荡负向集＋入位写放行回归）、clippy 13 基线持平、
    触碰面 fmt 零 diff；判官/loop 耦合面核证＝仅 rule↔category 配对、无 detail
    文案断言，契约面零变化。**载体重建 0.8.7 已完成进体（2026-09-30 凌晨；源冻结
    orz `79a3e8e5`＝`cb88d416` 审查处理＋bump 0.8.6→0.8.7；本段为完成态订正——
    原稿「待用户令放行」句作废）**：Windows 换装 MATCH 3/3（`orz 104b9bce…`／
    `orz-signer ed8066b7…`／`orz-acaf-provision ebf5b3f9…`）＋ACAF 重 provision
    绑新 signer（carrier-manifest 更新 00:57）；Linux musl 直写换装 MATCH 3/3
    （`orz 3332b38f…`／`orz-signer 1defacf8…`／`orz-acaf-provision bc7d5576…`）、
    static-pie×3、版本串双平台 0.8.7、祖先臂两 face 字面量进体、退役面
    `carrier:install-dir` 零残留（双平台实测）；两侧回滚点 `.0.8.6-bak`；
    `run_r0_heavy_official.py` 身份门换装（载体 `979a38fa…` → `3332b38f…`，
    适配器 `6d55c26e…` 未动）；打包 `rel-122-stage`（zip `a6b2c4b1…`／tar
    `bd68682b…`）。**祖先臂与动词补齐已在役，S4 重跑线解禁**；未提交未推送。
  - **S4**：TB21 线 5 题重跑（**build-pov-ray 第一＝翻盘实锤判据**：`/usr/local/bin/povray`
    安装放行、3/3 测试通过）＋b3-13 断点续跑＋余 44 题续跑；0cb S4 余项合并执行
    （拦截数读数、狗粮回归零误拦）。**重跑线自 2026-09-29 晚暂停，2026-09-30 随
    0.8.7 在役解禁**（首题 build-pov-ray 改在 0.8.7 上重跑；0.8.5 结构性 0 试次
    `official-v41-rerun-build-pov-ray` 留档）。
    **S4 首读达成（2026-09-30，翻盘实锤）**：`official-v41-rerun2-build-pov-ray`
    〔run `RUN-CLI-6abbf664`，14m17s／12000s，exit 0〕**reward＝1.0**（3 测试全过）；
    拦截恰 1 条＝**祖先链臂实战首拦**（`/etc` 写侧＝keystore 根 `/etc/orz-acaf/keystore`
    祖先，新 face 文案），`/usr/local/bin` 安装面全放行（4 处均执行），结束自述
    `reason=completed`——退役面放行／保护面开火／任务翻盘三面闭环，止损门未触发。
    **同日用户裁决：b3-13 起未跑面（44 题）直接重跑、不续跑**——载体换版后同一道轮
    跨包体版本续跑不合适，`run_official_v41_full.py` 断点续跑形态退役（判定档 §10）；
    余 4 题重跑＋未跑面 44 题逐题重跑待续。
