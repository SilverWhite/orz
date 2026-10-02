# 170 批：0.8.11 双平台载体重建进体——0cp S3／0cn S3 达成（2026-10-03）

> **用户令**：「请进行重建吧」（承接 164 批 0cn S2 落码＋168 批 0cp S2 落码＋169 批审查处置修正）。
> **本批**＝0.8.11 代窗口双平台载体重建（沿 117/144/147/155 批形态：源冻结→双平台重建换装→进体
> 字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。
> **源冻结**＝orz **`6493fdae`**（`7daf76c0` 164 批 0cn S2＋`99124f42` 168 批 0cp S2＋`e36dcacb`
> 169 修正批＋`6493fdae` bump 0.8.10→0.8.11；`cargo metadata --locked` exit 0、Cargo.toml＋Cargo.lock
> 恰两行）；未推送。
> **打包顺延**：本批不做 rel-stage 打包（0cp/0cn S4 真机核证未完、无发行压力；沿 155 批口径如实记）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（cargo 25m07s）**，退出码独立取（155 §2-A 方学注记落实）；**零 warning、零 rustc 闪退**；换装 `D:\tb-eval\orz-windows\` **MATCH 3/3**、`.0.8.10-bak` 链；`--build-info`＝`0.8.11 os=windows` exit 0；载体清单刷新（3 entries, 0.8.11） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261003-170`；provision **exit 0**；`binary_sha256=9f1e2889…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件（`f37556ab…`/`2aa80cb8…`）与 155 批账面逐位同＝未动（内层正典 again） |
| Linux musl | docker `rust:1.97-slim`（镜像重拉）apt 预检 **APT_OK**（零代理切换）＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001、`MSYS_NO_PATHCONV=1`、/target 缓存沿用）**cargo 42m08s exit 0（-j 1）**；`/out` 直写换装位（`.0.8.10-bak` 链），产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；Python ELF 解析＝三件 **ET_DYN（PIE）＋PT_INTERP=0（static-pie）**；alpine 3.20/bookworm 双冒烟 `version=0.8.11 os=linux` exit 0 |
| 进体字节判据 | **0cp 新面**：`域事件: 稳定确认` 0→1、`spike进入` 0→1、T̂ 注解「单轮工具周期会话自适应估计」0→1、`rli.notice.` 0→7（WIN）/0→1（LIN）、key 串 `streak_crossed`/`domain_spike_entry`/`migration_confirmed`/`coverage_gap` 在件内（1/1/1/5 WIN）；`rli_notice` 0→2（LIN）／WIN 0→0（imm64 内联拆分，见 §2-A）；**退役面**：`域迁移确认` 1→0（两平台）、`RLI提醒` 2→0（旧 pull-delta .rdata 字面量随直投退役消失；新直投字面量以 imm64 内联嵌入）；**0cn 面**：`预算：` 2→0（报告块预算行退役）、`墙钟约` 1→1（保留）；**回归面**：`/dev/null` WIN 3→3・LIN 46→46、`carrier-write` 1→1；**版本串** 0.8.11＝15（WIN）/10（LIN）、残留 0.8.10＝**0** |
| §2-A 字节面判读注记 | 短字面量（`\nRLI提醒: `／`rli_notice`／`lif_domain`）在 Windows 侧被 LLVM 以 `movabs imm64` 内联进多调用点（实据：`49 BF 0A 52 4C 49 E6 8F 90 E9`＝"\nRLI提" 指令流），连续字节搜索不可见——**功能面以件内 key 串＋key 形串＋冒烟整轮为证**，非缺失；判据口径据此如实记 |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `4ed531e8…` → **`c6a0812c…`**（注释同步：0.8.11＝源冻结 `6493fdae`；适配器 `6d55c26e…` 未动）；语法解析＋`--help` 加载 rc 0 |
| 冒烟 | Windows `--fake-provider -p hello` 整轮 rc=0（run `RUN-CLI-6ac00bd6`）；Linux alpine 3.20/bookworm `--build-info` 双绿 |
| 台账 | 本档；TODO（0cp・0cn S3 勾选、头部指针行）；BACKLOG（本批指针、计数行、P1 行）；索引 v4.139 → **v4.140**（头行）；第二卷 §1.123；orz 源清单随批再生 |

## §2-A 字节面判读注记（Windows 代码存储形态，如实记）

短 &str 常量在 Windows release 件中被 LLVM 以 `movabs reg, imm64` 形式内联进多份调用点副本，
字面量字节被指令编码拆开（`\nRLI提醒: ` 的实据形态 `49 BF 0A 52 4C 49 E6 8F 90 E9 …`＝
`movabs r15, "\nRLI提…"），连续子串搜索记 0——同串在 Linux 件 .rdata 呈连续形态（`rli_notice`=2）。
功能在件证据＝四类 key 串（`streak_crossed` 等）与键形 `rli.notice.`（WIN 7 处）＋两平台冒烟整轮。
**后续重建批的字节判据应以「key 串＋键形＋长文案」为面、避免以短 ASCII 串作跨平台判据。**

## §2 Windows 三件套（终态）

| 文件 | 尺寸 (B)（0.8.10→0.8.11） | SHA256 |
|---|---|---|
| `orz.exe` | 57,067,008（+13,312） | `ff22cbc144ecb812…` |
| `orz-signer.exe` | 6,740,480（Δ0） | `9f1e2889855a5e48…` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `32d494a5ecf48c25…` |

（换装基线＝0.8.10 三件 `615e3472…`/`b4b5d87b…`/`f23cc362…` 与 155 批账面逐位吻合。）

## §3 Linux musl 三件套（终态）

| 文件 | 尺寸 (B)（0.8.10→0.8.11） | SHA256 |
|---|---|---|
| `orz` | 115,516,016（+36,784） | `c6a0812c197eebcbc71126416e9f5b8aafa550a8e37463acacfb40c503981da1`（身份门新值） |
| `orz-signer` | 1,397,512（+304） | `7fee74b1c78e1bf7…` |
| `orz-acaf-provision` | 1,215,944（+16） | `6957066fa2ba0813…` |

## §6 边界与如实记

1. **未推送未发行**：orz 两提交（`e36dcacb`/`6493fdae`）与父仓各批均在本地；不建 Release；发布面仍停 v0.8.7。
2. **打包顺延**（见档头）——rel-stage 随发行批。
3. **Linux 载体未做 ACAF 重 provision**（沿 093/100/109/122/144/147/155 同口径）；Windows 在役目录已重 provision。
4. **`--version` 完整性核证通道**：Windows 无 tty 管道落 TUI 挂起（155 §6.4 已知形态）——完整性以构造性保证替代（manifest 由同批换装后文件生成、3 entries rc 0）＋`--build-info` rc 0；本批 `--help` 探测同挂起（已终止，属同族已知形态）。
5. **0cn S2 构建验证补跑闭合**：164 批登记「构建/测试验证随 S3 进体批补跑」——本批 Windows/Linux release 重建零告警 rc=0 即该补跑读数。
6. **0cp/0cn 不闭合**：S3 已达成（本批）；S4＝真机核证（0cp §5 七判据随真机收取、0cn 随真机读数），两项维持开放（计数 59 不变）。
7. **docker 镜像重拉**：`rust:1.97-slim`/`alpine:3.20`/`debian:bookworm-slim` 本机已不存在（前批清理），随批重拉——环境面如实记。

## §7 关联与关键词

[`164 批档`](164_0CN_S2_BUDGET_INJECTION_REMOVAL_2026-10-02.md)／[`168 批档`](168_0CP_S2_ACTION_SAMPLING_DIRECT_PUSH_IMPL_2026-10-03.md)／
[`169 批档`](169_0CP_S2_REVIEW_FIXES_2026-10-03.md)／[`155 批档`](155_CARRIER_REBUILD_V0810_0CK_0CL_S3_2026-10-02.md)（同形态先例）／
BACKLOG `0cp`/`0cn`／TODO `P1-0cp`/`P1-0cn`。

关键词：170 批、0.8.11 重建、源冻结 `6493fdae`、0cp S3 达成、0cn S3 达成、`c6a0812c…` 身份门、
ACAF 重 provision 9f1e2889、static-pie×3（Python ELF 解析口径）、alpine/bookworm 双冒烟 0.8.11、
imm64 内联字节面注记、打包顺延、未推送未发行、计数 59。

## §8 补记（落账后门禁）

- 机械门禁：落账后复跑 `check_repository.py`＝error_count **0**、`valid: true`（2026-10-03 实测）；父仓 orz 源清单随批再生（`generate_orz_source_manifest.py`，1,485 条）。
- **门禁拦截事件（如实记）**：首跑 `check_repository.py` 报 `ledger consistency: TODO 全部勾选但 BACKLOG 仍记开放——token: 0cp`——根因＝TODO `P1-0cp` 节把 `[x] S3 进体` 与 `[ ] S4 真机核证` 写在同一列表行（行首 `[x]` 掩盖行中 `[ ]`，勾选解析按行首取态）；处置＝S3/S4 拆为两行后门禁回绿。**教训**：勾选清单一行一事，复合状态不得共用列表行（门禁再次在 RLI 线拦下真实漂移——155 批 `RLI_ANNO` 拦截同族价值实证）。
