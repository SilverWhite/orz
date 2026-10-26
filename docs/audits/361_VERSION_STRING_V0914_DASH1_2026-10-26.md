# 361 批：版本串登记 0.9.14-1 换装（360 重出件划界收口）

> 日期：2026-10-26
> 用户令：「请将本版本登记为0.9.14-1吧，还是做好区分，orz侧这一轮确实没有做好区分」⇒ 版本串本身升 **0.9.14-1**（`--version`/build-info/桥探针可机械核验），取代 360 的「同号重出」姿态。
> 基线：父仓 HEAD `55c3aeb4`（批(360)）＋ orz `867314a7`（bump 0.9.14-1：Cargo.toml＋Cargo.lock 恰两行，沿 357 bump 口径；内容＝`568ea733` 358/359 全量）。
> 计数：65 不变（纯版本登记换装，不新增项）。

## §0 结论

在役 Windows 载体位＋装机位升至 **0.9.14-1**（`867314a7`＝`568ea733`＋版本串 bump）：`--version`＝`orz 0.9.14-1 (windows/x86_64/release)`、build-info 同；三件 release→载体位→装机位 cmp 逐字节 **MATCH 3/3**；ACAF 重 provision（`binary_sha256=f3a0cb9b…`＝新 signer）；载体清单 v0.9.14-1 重生成 3 entries；doctor 载体位 `12/0/0`、装机位 `10/2/0`（version 行均 `0.9.14-1`）；活体 bridge/child 均 `0.9.14-1`、`version_skew=false`、五会话投影连续。自此本地在役件与 GitHub Release v0.9.14 资产（`95faff3a`）**版本串即异**，区分机械可核。

## §1 版本串单源核证

- `bridge_version` 由 orz-bin 启动 web 时覆写（`orz-bin/src/main.rs:1502` `config.bridge_version = env!("CARGO_PKG_VERSION")`）；child 探针取装机位件 `--version`；`orz-web`/`orz-host` 自身 crate 版本为 `0.1.0`（不可见面）——bump orz-bin 一处即三方一致，无假性 skew。
- 版本序注记：semver 序 `0.9.14-1` < `0.9.14`（pre-release 序）；本项目版本面仅作字符串标识与等值比较（bridge/child skew 比较），无排序依赖。`-1` 后缀沿 346 先例语义（再发行划界），此处前移到版本串本身。

## §2 构建、备份、换装、ACAF

- 构建：`cargo build -p orz-bin --release -j 2` exit 0（**1m13s** 增量，仅 orz-bin 重编译；日志 `D:\tb-eval\buildlogs\win_361_build.log`）。
- 停栈：taskkill 在役 360 web 桥（含其子进程；signer 已随退）；复检 0 orz 进程。
- 备份：两侧 `*.0.9.14-361-bak` 三件（内容＝360 重出件 `d9fd7387…`/`2956ca23…`/`90b80f9b…`，哈希见 360 批档 §1）；旧 manifest `acaf/signer-manifest.json.bak-20261026-361`。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz.exe` | 59,778,048 | `e2271c484b4a29223b215e76f6556363d2ef95f96dd1c9ddfa40daabf0b886f9` |
| `orz-signer.exe` | 6,740,480 | `f3a0cb9bb76563e0774452669d75e84fae2f850666e9ece1d294045204298665` |
| `orz-acaf-provision.exe` | 6,644,736 | `a757b85d8c410e0d9c0cd26ca2411e98cd1d160fd46740071a30b57044a1362e` |

- 换装：release→载体位→装机位 cmp **6/6**。
- ACAF：重 provision exit 0，`binary_sha256=f3a0cb9b…` 与新 signer 逐位一致；新旧 manifest 逐字段 diff 仅 `binary_sha256` 一处；三指针不变。
- 载体清单：`--version 0.9.14-1` → 3 entries，三件哈希与部署件一致。

## §3 doctor 与活体

- doctor（各槽位目录内实跑）：载体位 `ok=12 warn=0 fail=0`；装机位 `ok=10 warn=2 fail=0`（两 warn 环境性，沿 360 §3 口径）；两侧 version 行均 `0.9.14-1`。
- 活体：目标工作区 web 桥（装机位件，`--real`；日志 `D:\tb-eval\buildlogs\orz_361_web_stdout.log`／`orz_361_web_stderr.log`，`127.0.0.1:21487`）——banner `[permission] mode=interactive-yolo (auto-approve, source=default; benchmark axes off)` 逐字核收；`/api/boot` bridge `0.9.14-1`／child `0.9.14-1`／`child_probe=ok`／`version_skew=false`；五会话投影连续（`51da03ea`/`54d12bcc`/`d24eadd6` 活跃，`073b8892`/`c23e0aaa` 已关闭，legacy 全 false）。旧标签页 token 失效，新 URL token `00516496…`。

## §4 边界

- 本批取代 360 批档 §5 的「下次发行前须 bump 0.9.15」：在役件已登记 **0.9.14-1**；下次发行可发 v0.9.15（新内容）或 v0.9.14-1 tag（沿 346 后缀先例发行本件）——留用户裁决，两途均不再有同号歧义。
- 未推送、未发行；Linux 载体未动（仍 357 的 0.9.14，358/359/361 变更均未进 Linux 载体）。
- `0.9.14-1` 命名与已发行 tag `v0.9.14` 不冲突（tag 面尚无 `v0.9.14-1`）；若未来发行本件，GitHub Release tag `v0.9.14-1` 可用。
