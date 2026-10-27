# 372 批：0.9.14-3 Windows 出件换装（367～371 进体；仅 Win；核后停栈）

> 日期：2026-10-28
> 用户令：「请进行重建吧，本轮只做win平台包，版本号为0.9.14-3，同时请帮我进行换装，不过不用启动进程，我自己手动启动就好」。
> 基线：父仓 `5c045394`（批(367～371) 补：源摘要台账再生成）← `076e3c86`（批(367～371)）＋ orz `1b3efab5`（批(367～371) 落码）。
> 版本串：`0.9.14-3` ＝ orz 树内现值（365 批 bump 后**从未出件**）——本批为 **-3 首次出件换装**，非重出件（与 360「同号重出」性质不同，无版本划界豁免问题）；换装前在役件为 **0.9.14-2**（364 件，`5f3d0cf1…`）。
> 计数：66 不变（纯出件换装批）。

## §0 结论

在役 Windows 载体位＋装机位由 **0.9.14-2**（364 件，362/363 内容，带 R-370-00 崩溃缺陷）升至 **0.9.14-3**（orz `1b3efab5`：367 0ds M1-S1 state 分区＋368 纯摘要控制轮分流＋369 压缩摘要控制面工具化＋370 审查修复与断连根因修复＋371 审查发现全面处置）。三件 release→载体位→装机位 cmp 逐字节 **MATCH 6/6**；ACAF 重 provision exit 0（`binary_sha256=b42efd21…`＝新 signer，manifest 逐字段 diff 仅此一处）；载体清单 v0.9.14-3 重生成 3 entries；doctor 载体位 `12/0/0`、装机位 `10/2/0`（两 warn 环境性，沿 360/364 口径）；两侧 version 行均 `0.9.14-3`；在体特征 367/369/371 各批内容全数命中。**按用户令核后不停旧栈（旧栈已无进程）亦不启动新进程**——活体 bridge/child 核证与 `version_skew` 观察留待用户手动启动后。

## §1 源冻结与构建

- orz `1b3efab5`（18 文件，工作树干净、已推 `cli` 远端）＝367～371 全部落码；版本串 `0.9.14-3` 系 365 批所 bump，本批零改动（Cargo.toml/Cargo.lock 未触）。
- 构建：`cargo build -p orz-bin --release -j 2` exit 0（**4m51s**；日志 `D:\tb-eval\buildlogs\win_372_build.log`）；`--version`＝`orz 0.9.14-3 (windows/x86_64/release)`，build-info 同。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz.exe` | 60,393,472 | `9d96271545ca9e3745c2f3238e03265d84da872cd2f4265cfc1c6a3e988777ba` |
| `orz-signer.exe` | 6,740,480 | `b42efd21c7f3729c47a517e0f68eb07df3bc433b322561965ec76de677debc9b` |
| `orz-acaf-provision.exe` | 6,644,736 | `9825b1c60e560fe23434027c6d19b8fcbd8435d346a66973a97fda6f61288980` |

## §2 备份、换装、ACAF、清单

- 停栈核：换装前后均 **0 个 orz 进程**（用户已手动停旧栈；`tasklist` 实证）。
- 备份：两侧 `*.0.9.14-2-372-bak` 三件×2（内容＝364 件 `5f3d0cf1…`/`1097d43a…`/`49608f5a…`，两侧逐字节一致）；旧 manifest `acaf/signer-manifest.json.bak-20261028-372`（回滚路径同 352 §2：停栈→成组还原→重 provision）。
- 换装：release→载体位→装机位 cmp **MATCH 6/6**。
- ACAF：以载体位新 provision 件重 provision exit 0；`binary_sha256=b42efd21…` 与新 signer 逐位一致；新旧 manifest 逐字段 diff 仅 `binary_sha256` 一处；用户级 env 三指针不变（`ORZ_ACAF_KEYSTORE/MANIFEST/BINARY` 均指载体位；keystore 幂等装载，未新建）。
- 载体清单：`python scripts/generate_carrier_manifest.py D:\tb-eval\orz-windows --version 0.9.14-3` → 3 entries（`generated_at=2026-10-27T18:07:37Z`），三件哈希与部署件一致。

## §3 doctor 与在体特征

- doctor（各槽位目录内实跑，输出引至 `D:\tb-eval\buildlogs\doc_372_*.txt`）：载体位 `ok=12 warn=0 fail=0`；装机位 `ok=10 warn=2 fail=0`——两 warn 环境性（该目录无 run journal＝新工作区正常；装机位无 `carrier-manifest.json`，沿 341/346/352「装机位本不放清单」口径）。两侧 version 行均 `0.9.14-3`（build-info 同）。
- 流程教训（如实登记）：doctor 自检会把**扫描目录内**的临时输出文件计为「清单外新增」——首次载体位 doctor 因输出落位错误报 integrity warn，输出改引扫描目录外后复跑干净。后续换装批照此执行。
- 在体特征（部署件 `orz.exe` 字节面）：367 `state_read`×32／`state_write`×6；369 回执「压缩摘要已接收」×1；371 `tool_summary_participated`×7／「summary 必须是字符串」×12／「只支持 live 读取」×25——367/369/370/371 内容全数进体。

## §4 活体核证（按用户令不做）

- 用户令明示「不用启动进程，我自己手动启动就好」⇒ 本批**不启动** `orz web --real`／`orz --stdio`，364 §4 式的 banner／`/api/boot`／`version_skew`／会话投影核证**全部留待用户手动启动后**（首次启动时 bridge/child 应均为 0.9.14-3；如见 skew，按 352 §2 回滚：停栈→还原 `*.0.9.14-2-372-bak`→重 provision）。
- R-370-00 崩溃缺陷（`exec_policy` 管道臂字节切片 panic）已随本件修复；换装后 `-Pattern 'a|中文'` 形状命令的 370 §9.4 规避提示**作废**。

## §5 边界

- 未发行：0.9.14-3 未上 GitHub Release；下次发行候选版本号留用户裁决（树内 -3 已在役，「已发行版本号不重出」口径不受影响）。
- Linux 载体未动（仍 357 的 0.9.14；365～371 变更均未进 Linux 载体）——用户令「本轮只做 win 平台包」。
- orz 源自 `1b3efab5` 后零改动 ⇒ `orz_source_manifest.sha256` 无需再生成（371 补丁批口径延续）。
- 367～371 各批读数为各自工作树实测；本批仅出件，未重跑测试面（loop 948/0/3 等读数见 371 §10）。
- 仓库侧台账（本批档＋TODO/BACKLOG/INDEX 头部）随本批提交推送。

## §6 入口

- 批档：本档；上游 [`371 批档`](371_369_370_REVIEW_REMEDIATION_2026-10-28.md)（367～371 内容总账）／[`364 批档`](364_SWAP_V0914_DASH2_2026-10-27.md)（换装流程基线）。
- 构建日志：`D:\tb-eval\buildlogs\win_372_build.log`；doctor 输出：`D:\tb-eval\buildlogs\doc_372_carrier.txt`／`doc_372_install.txt`。
