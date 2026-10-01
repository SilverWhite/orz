# 147 批：0.8.9 双平台载体重建进体——0cf/0cg 进体并闭合（2026-10-01）

> **用户令**：「请进行重建吧」（承接同日 146 批 0ce 闭合＋0cf/0cg S1/S2 落码）。
> **本批**＝0.8.9 代窗口双平台载体重建（沿 117/121/144 批形态：源冻结→Windows 重建换装＋ACAF
> 重 provision→Linux musl docker 直写→进体判据→**实弹探针**→身份门换装→打包核证→落账）。
> **源冻结**＝orz **`e87b0630`**（`be4f90ff` 批落码＋`e87b0630` argv_scrub 实弹修复＋`6295b3dc` bump）。
> **0cf/0cg 随进体判据达成闭合：60 → 58**（两批序 S4＝闭合，判据＝新面在役载体可达）。
> **未推送未发行**（沿 113/116/121/144 惯例，随推送批一并；发布面仍停 v0.8.7）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结 | orz `6295b3dc`（bump 0.8.8→0.8.9，Cargo.toml＋Cargo.lock 两行）＋`e87b0630`（argv_scrub 实弹修复，见 §2-A）；`cargo metadata --locked` exit 0；未推送 |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（5m19s 初建＋29s 修复后增量重链）**；唯一警告＝orz-host unused import（既有面）；换装 `D:\tb-eval\orz-windows\` **MATCH 3/3**、回滚点 `.0.8.8-bak` 链；`--build-info`＝`0.8.9 os=windows`（exit 0）；**载体清单刷新至 0.8.9**（原安装根清单滞留 0.8.7＝144 批既有缺口，本批顺手闭合，如实记 §6.4） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261001-147`；provision **exit 0**；`binary_sha256=fc38e48d…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件（`f37556ab…`/`2aa80cb8…`）前后逐位未动（内层正典 again 成立） |
| Linux musl | docker `rust:1.97-slim`＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001；**首挂失败一次如实记**＝脚本未按契约挂 `/build.sh`，修正后 exit 0；**实弹修复后二建 exit 0**）；`/out` 直写换装位（`.0.8.8-bak` 链），产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；Python ELF 解析＝三件 **ET_DYN（PIE）＋PT_INTERP=0（static-pie）**；alpine 3.20/bookworm 双冒烟 `version=0.8.9 os=linux` exit 0 |
| argv_scrub 实弹探针 | **初建载体探针失败（如实记＝§2-A）**：ps 仍见完整 argv ⇒ `/proc/self/mem` 通道在该容器环境自进程读写不可靠；**重设计为直接裸指针写**（setproctitle 标准形态）后复测全绿：ps cmdline＝`/v/orz --stdio`（`--max-wallclock 99999` 消失）、外部宿主注入 `ORZ_MAX_WALLCLOCK=77777` 启动后 `/proc/<pid>/environ` 读数 **0**、原始 cmdline 字节仅剩 `/v/orz --stdio` |
| 进体判据 | Windows 件：版本串 `0.8.9` 10→**11**/残留 `0.8.8` 1→**0**；`WALLCLOCK_ELAPSED/LIMIT/REMAINING_ROUNDS` 1/2/1→**0/0/0**；0cf 简注 `framework usage manual…` 0→**5**、`section=guide` 0→**5**；保留面 `F6_BUDGET_CUE` 2→2、`TOOL_ROUND_BUDGET` 5→5；五规则 id＋保底文案逐位不变。Linux 件：版本串 7→**8**/残留 60→59；墙钟三串 1/2/1→**0/0/0**；0cf 两串 0→**5/5**；`/proc/self/stat` 0→**1**（argv_scrub 进体）；0ch 面 `/dev/null` 46→46、`/dev/zero` 1→1、`/dev/` 族 75→75（零回归）；`Orchestrator Mode` 两代均 0（DCE——0ce 的字符串级判据主场缺位，如实记 §6.3） |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `874df6ca…` → **`4e35f410…`**（注释同步：0.8.9＝源冻结 `e87b0630`；适配器 `6d55c26e…` 未动）；语法解析＋`--help` 加载通过（中间态 `00b37193…` 被 §2-A 修复取代，如实记） |
| 打包 | `rel-147-stage` 两侧各 **6 files**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`；0.8.9 版 README 新增 0.8.9 节）；包内件与在役载体逐位一致；容器核证（alpine 3.20）zip **6 files**・tar **6**、包内 build-info（tar 侧 in-container）＝**0.8.9 exit 0**；**清单活体两态**＝解压态干净 **0 finding**／README+1B **恰 1 条**＝`长度不符（清单 49309 ≠ 实际 49310）` ✓ |
| 台账 | 本档；BACKLOG（指针行/计数行 **58**/P2 总览行/P2 开放项行去 0cf・0cg/两节闭合 bullet）；TODO（计数行 58/P2 路由行/P2-0cf・0cg 四勾）；第二卷 §1.99；索引 v4.115 → **v4.116**（头行＋§8 implemented 增 0cf/0cg）；源清单重生成；门禁见 §6 |

## §1 源冻结与提交

| 项 | 值 |
|---|---|
| orz 提交 1 | `be4f90ff`（146 批 0ce/0cf/0cg 批落码；初稿记「37 文件 +181/−14,109」——**148 批勘误：实际 commit 统计＝41 files +389/−14,114**，初稿数字为 fmt 触碰面随批与漂移对齐并入前的快照，未随提交终态回改） |
| orz 提交 2 | `6295b3dc` `chore(release): bump version 0.8.8 -> 0.8.9` |
| orz 提交 3 | `e87b0630` `fix(0cg ②③ argv_scrub 实弹修复)`（本批内发现并修复，见 §2-A） |
| 校验 | `cargo metadata --locked` exit 0（bump 后） |
| 推送 | **未推送**（orz 分支 ahead：`be4f90ff`＋`6295b3dc`＋`e87b0630`） |

## §2-A argv_scrub 实弹修复（本批核心事件，如实记）

初建 0.8.9 Linux 载体的 `scrub_linux` 走 `/proc/self/mem` 通道（stat 定界＋seek 读写回）。
**容器内实弹探针失败**：`ps ax` 仍见完整 `--max-wallclock 99999`（fail-soft 静默未达）。诊断：
最小独立 musl 探针（同解析＋同零化逻辑）证实**直接裸指针写自身 argv 区有效**（写入后读回一致、
ps 面旗标消失），而 `/proc/self/mem` 自进程读写在本环境（WSL2 内核 6.6.87 容器）不可靠。**重设计**：
`scrub_linux` 改为 stat 定界后直接 `slice::from_raw_parts_mut` 原位零化（setproctitle 标准形态，
不再经 `/proc/self/mem`）；纯函数四钉不变；orz `e87b0630` 提交；两平台重建、换装、门、包全部随新
哈希重做（中间态身份门 `00b37193…` 被取代）。修复后复测见 §0 实弹探针行——**0cg ②③ 的产品级
实弹闭环成立**。

## §2 Windows 重建与换装（修复后终态）

| 文件 | 尺寸 (B)（0.8.8→0.8.9） | SHA256 |
|---|---|---|
| `orz.exe` | 57,116,160（−1,536） | `fadb4f134ce9870a…`（修复后终态；初建态 `e0b27606…` 被取代） |
| `orz-signer.exe` | 6,740,480（Δ0） | `fc38e48d29de9f8c…` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `5f31bf0d5eb645de…` |

（现役 0.8.8 三件换装前哈希 `82c37eaa…`/`f651f7ba…`/`3c520cdf…` 与 144 批账面逐位吻合＝换装
基线实证。signer/provision 代码未随 0cg 变化 ⇒ 哈希与 0.8.8 不同源但批内两次构建一致。）

## §3 Linux musl 三件套（修复后终态）

| 文件 | 尺寸 (B)（0.8.8→0.8.9） | SHA256 |
|---|---|---|
| `orz` | 115,559,728（−1,248） | `4e35f4101f26a8e6c91ab14dbb871dc715ce983f1cf7c6d0edd76396710d2385`（身份门新值） |
| `orz-signer` | 1,397,344（−4,248） | `27f94fe90167dcd0…` |
| `orz-acaf-provision` | 1,215,936（−4,248） | `bc119e2db33e6360…` |

alpine 3.20 `file`/`readelf` 缺位 ⇒ static-pie 判据以 **Python ELF 头解析**定案：三件 `e_type=3
(ET_DYN/PIE)`＋`PT_INTERP=0`（144 批先例为 file 输出，本批改解析法，等价口径，如实记）。

## §4 实弹探针（0cg ②③ 产品级证据）

容器（alpine 3.20）内以在役新件起 `orz --stdio --max-wallclock 99999`（外层宿主注入
`ORZ_MAX_WALLCLOCK=77777`）：3 秒后取证——

| 通道 | 读数 | 判定 |
|---|---|---|
| `ps ax` cmdline | `/v/orz --stdio` | flag＋value 消失 ✅ |
| cmdline 含 `max-wallclock` 计数 | **0** | ✅ |
| `/proc/<pid>/environ` 含 `ORZ_MAX` | **0**（宿主注入的 77777 启动后被 `remove_var`＋environ 区零化） | ③ env 收口实弹 ✅ |
| 原始 cmdline 字节 | `/v/orz --stdio` | argv 区零化留痕正确 ✅ |

## §5 打包（rel-147-stage）

- `win/`：`orz.exe`＋`orz-signer.exe`＋`orz-acaf-provision.exe`＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`（0.8.9 版 README＝新增 0.8.9 节＋0.8.8 改注中间载体）。
- `lin/`：同构（musl 三件）。
- 归档：`orz-0.8.9-windows-x86_64.zip`（6 files）/`orz-0.8.9-linux-x86_64.tar.gz`（6 files），均平铺。
- 根 `SHA256SUMS`：zip `813acfb6…`／tar `f5955cbf…`。
- 容器核证（alpine 3.20）：zip 6 files・tar 6 files・tar 侧 in-container `--build-info`＝`0.8.9` exit 0。
- 清单活体两态（`--version` 通道、容器无 tty 的 exit 1 为 tui io error 已知形态，只看 finding）：解压态 **0 finding**；README+1B **恰 1 条** `长度不符（清单 49309 ≠ 实际 49310）`。

## §6 边界与如实记

1. **未推送未发行**：父仓与 orz 提交均在本地；GitHub Release 不建；发布面仍停 v0.8.7。
2. **Linux 载体未做 ACAF 重 provision**（沿 093/100/109/122/144 同口径）；Windows 在役目录已重 provision。
3. **0ce 的字符串级判据主场缺位**：`Orchestrator Mode`（`ORCHESTRATOR_PROMPT_BODY`）在 0.8.8/0.8.9
   两代 Windows/Linux 件均为 0——该常量在 0.8.8 即被链接器 DCE（预设表在二进制内无可达引用），
   0ce 的进体证据主体＝**源删除＋工作区编译＋orz-agent 302 测试绿**（146 批），字节面只有
   「无回归」意义，如实记。
4. **安装根载体清单 144 批遗留缺口闭合**：Windows 安装根 `carrier-manifest.json` 在 144 批换装后
   未刷新（滞留 0.8.7 三件哈希，`--version` 通道自 0.8.8 起一直告警）；本批两侧均刷新至 0.8.9 并
   验证 0 finding（Linux 侧 `orz-linux/carrier-manifest.json` 同批刷新）。
5. **方法学三则**：① static-pie 判据本批改 Python ELF 头解析（alpine 无 file/readelf；等价口径）；
   ② 容器内执行 Windows 卷挂载的二进制须 `chmod +x`（挂载执行位不保留）＋tar 解包到容器内路径
   （挂载卷直跑受限）；③ `--version` 在无 tty 管道会落 TUI（裸 orz 兜底）——核证一律以 stderr 重
   定向＋`grep carrier-integrity` 计 finding 为准。
6. **0ch S4 仍未跑**：`caffe-cifar-10`／`git-multibranch` 两题（0.8.9 已在役＝重跑线解禁）。
7. **0cf/0cg 闭合（60 → 58）**：两批序 S4＝闭合（无独立实机步骤），判据＝进体判据达成＋实弹探针
   （0cg）／字节判据（0cf）在在役载体成立；`pending` → `implemented`。

## §7 关联与关键词

[`146 批档`](146_0CE_0CF_0CG_DEADCODE_GUIDE_WALLCLOCK_2026-10-01.md)／
[`144 批档`](144_CARRIER_REBUILD_V088_0CH_S3_2026-10-01.md)（同形态先例）／
[`117 重建档`](117_CARRIER_REBUILD_V085_0CB_S2_2026-09-29.md)／
BACKLOG `0cf`/`0cg`／TODO `P2-0cf`/`P2-0cg`。

关键词：147 批、0.8.9 重建、源冻结 `e87b0630`、argv_scrub 实弹修复、`/proc/self/mem` 不可靠、
直接指针写、setproctitle 形态、实弹探针 ps/environ 双绿、`4e35f410…` 身份门、rel-147-stage、
static-pie×3（ELF 解析口径）、清单活体两态、安装根清单 144 遗留缺口闭合、0cf/0cg 闭合 60 → 58、
0.8.9 顺延缺位补齐、未推送未发行。
