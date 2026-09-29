# 写入管控保底化修订（0bw v2 / 0cb）设计档：灾难硬边界保底 ＋ 一般写动作归还审批组件

> **状态**：`design v2.1`（2026-09-29 定稿 v2.0＝0cb S1；**同日 v2.1＝S2 全面审查处理批**——
> 用户三项裁决：① 契约面 **schema 升 v0.3**＋legacy 回放豁免（跨代际回放兼容）；② 实现
> 过严臂进一步收窄（规则 2 PhysicalDrive/mkfs 目标位、规则 1 动词集 `ri` 补齐＋盘符相对根
> 补全、规则 4 蜂巢目标位）；③ 钉覆盖补全＋证据引用修正）。**权威链**：本档修订
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
（根级递归删除／raw 设备与卷毁写／引导固件与安全机制翻转／注册表蜂巢删除／载体自毁），
一般性写动作（装 /usr、清 C 盘、删过期补丁、编辑 /etc、`>/dev/null`）全部放行，
交回审批组件按权限模式处置。

## §1 最终规则面（封闭枚举；恰好 5 条 block 规则）

| # | 规则 id | 分类 | 触发形状（封闭：动词表 × 目标形状） | 例 |
|---|---|---|---|---|
| 1 | `catastrophic-recursive-delete` | **block** | 删除动词（`rm`/`rmdir`/`rd`/`del /s`/`Remove-Item`/`erase`/`ri`〔v2.1 补齐——Remove-Item 的 PowerShell 别名〕）＋ 目标 ＝ **卷根**（`/`、`C:\`、`D:\`…）或**根本性树根**（Win：`C:\Windows`、`C:\Program Files`、`C:\Program Files (x86)`、`C:\ProgramData`；Linux：`/boot`、`/etc`、`/usr`、`/bin`、`/sbin`、`/lib`、`/lib32`、`/lib64`、`/var`、`/dev`、`/proc`、`/sys`）＋ 递归旗（`-r`/`-rf`/`-R`/`--recursive`/`/s`/`-Recurse`）。盘符相对根形态（`\Windows`、`/usr`——`has_root` 无盘符）按 `%SystemDrive%`（env 缺失回退 `C:`）补全盘符后比对（v2.1 绕过面闭合） | `rm -rf /`；`Remove-Item C:\Windows -Recurse`；`rm -rf /usr`；`rm -rf \Windows` |
| 2 | `raw-device-write` | **block** | `dd`＋`of=` 落**块设备**（`/dev/[sv]d*`、`/dev/vd*`〔virtio，v2.1 登记增补〕、`/dev/nvme*`、`/dev/mmcblk*`、`/dev/mapper*`〔LVM/设备映射器，v2.1 登记增补〕、`\\.\PhysicalDrive*`；**`/dev/null` 显式豁免**）；`mkfs*`＋目标词落上述块设备形态（v2.1 收窄——镜像文件构建〔`mkfs.ext4 disk.img`〕放行）；`format`；`diskpart`（脚本形态）；卷影删除（`vssadmin delete`／`wbadmin delete`）。**目标位判定**（v2.1 收窄）：PhysicalDrive 词元仅在写侧目标位（`of=` 值／`mkfs*` 目标词）命中——读侧 `dd if=\\.\PhysicalDrive0`（备份/取证）与查询类提及放行 | `dd if=x of=/dev/sda`；`mkfs.ext4 /dev/sdb` |
| 3 | `boot-firmware-flip` | **block** | `bcdedit`；Defender 偏好域（`Set-MpPreference` 等）；防火墙 profile set；`sc/net stop windefend\|mpssvc`；`Set-ExecutionPolicy`；审计清除（`wevtutil cl`／`Clear-EventLog`）；`fltmc unload` | 沿 v1 `safety-mechanism-flip` 全集 |
| 4 | `registry-hive-delete` | **block** | `reg delete\|add\|import` 落 `HKLM`/`HKCR`/`HKU`——**目标位判定**（v2.1 收窄）：`reg <sub> <target>` 第三词元前缀比对，数据值中的蜂巢提及（`/d hklm-…`）不误拦 | 沿 v1 |
| 5 | `carrier-write` | **block** | `{cwd}/.gsa`、orz 安装目录（含 ⊆cwd 降级规则）、三件套、`grok-home/` | 沿 v1 C1/C2/C3（用户底线③本体） |

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

## §3 确定性论证（回答「拦截没有膨胀吧？」）

- **封闭枚举**：5 条 block 规则＝「封闭动词表 × 封闭目标形状」的组合，无启发式、
  无「疑似危险」判据、无路径前缀宽扫；表外**恒 allow**（沿 v1 默认）。
- **防膨胀钉（先红后绿）**：① deny 形状表恰 N 条断言（新增表项必改测试）；
  ② **正向放行集 fixture**（必须保持放行，一行回归即红）：`make install`、
  `apt-get install`、`pip install`、`echo x > /dev/null`、`2>/dev/null`、`+O/dev/null`、
  `Remove-Item C:\Windows\SoftwareDistribution\Download\old`（文件级）、
  `cat /proc/cpuinfo`、`ln -s /etc /w`（非递归删除，不拦）、`dd if=x of=/dev/null`；
  ③ 负向集：五规则各 ≥2 命中例（`rm -rf /`／`rd /s C:\`／`Remove-Item C:\Windows -Recurse`／
  `dd of=/dev/sda`／`bcdedit`／`reg delete HKLM`／写 `.gsa`）。
  **v2.1 补全**：行数钉扩展至全部封闭表（`DESTRUCTIVE_VERBS`〔规则 5 block 面〕/
  `DELETE_VERBS`/`ELEVATION_PROGRAMS`/`WRAPPER_WORDS`/`CONTENT_WORDS`/载体面三表）；
  收窄后放行负测补口（raw 读侧/镜像文件/reg 数据值/盘符相对根子目录）。
- **不再有车道问题**：保底形状在任何车道都无碍正常任务（没有正经任务会递归删卷根），
  无需基准/真机分档开关；v1 §2.3「无运行时开关、表项变更＝代码变更」原则维持。

## §4 三落地点与消费面变化

- **工具面 `search_replace`**：`check_write_target` 收窄为**载体集**（C1/C2/C3）；
  写系统树不再拒（0bw v1 的工具面系统核心拒臂退役）。
- **命令面 `run_terminal_cmd`**：按 §1 规则面重写 `review_command_with`；
  留痕形态不变（block 文案/warn 头行随 journal）。
- **undo 面**：目标域收敛**改为 cwd＋载体集**（v1 的「cwd＋系统核心」中的系统核心臂退役；
  cwd 边界与 C1 判定不变）。
- L3（Landlock/CFA）：allow 集随 deny 表收窄同步（核心集除外→仅灾难防护所需最小集），
  实施批同步本档；本批不动 L3 落码面。

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
- **台账**：0bw 条目由本档修订注记（不退役——架构与载体自保护仍有效）；ADR-0010 转录随落账批。
