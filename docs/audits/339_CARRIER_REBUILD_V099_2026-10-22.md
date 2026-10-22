# 339 批：0.9.9 双平台载体重建与换装（2026-10-22）

> **用户令**：「直接重建吧」（承接 338 批「回档后会话回放顺序坍塌」修复；此前问「要我接着做 339（重建＋换装＋S2 复跑）还是先提交」）。**本批**＝把 338 的源码修复打成 **0.9.9** 载体：版本 bump→Windows release 构建→Linux musl 容器构建→ACAF 重 provision→双平台换装/清单→冒烟与「修复已进体」载体级核证。**未推送、未发行**；计数 64 不变。
>
> **未做**：0dn S2 真机十条（本沙箱禁命名管道，Chrome 起不来，见 §6）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 版本 | **0.9.9**（`orz/crates/orz-bin/Cargo.toml`＋`Cargo.lock` 恰两行 0.9.8→0.9.9） |
| Windows 构建 | `cargo build -p orz-bin --release -j 2` **exit 0，1m51s，0 warning**；三件齐出（`src/bin/` 自动发现 signer/provision） |
| Windows 三件 | `orz.exe` **59,272,192 `dd0071de…`**／signer **6,740,480 `1b5b74a1…`**／provision **6,640,128 `19b8591b…`** |
| Windows 换装 | 载体位＋装机位 **MATCH 3/3**（carrier==install==release）；旧件两侧 `.0.9.8-336-bak`；既有 `.0.9.8-bak`（335 件）保留 |
| ACAF | 载体 `acaf` 重 provision **exit 0**；旧 manifest 备份 `signer-manifest.json.bak-20261022-339`；`binary_sha256=1b5b74a1…` ↔ signer 逐位一致 |
| Linux 构建 | docker `rust:1.97-slim`＋`scripts/build_orz_aliyun_trixie.sh` **exit 0**；cargo `Finished release … 5m05s` |
| Linux 三件 | `orz` **118,404,816 `fad9420d…`**／signer **1,397,920 `114320c5…`**／provision **1,216,336 `c6f09b35…`** |
| Linux 形态 | `e_type=3 (DYN)`、`e_machine=62`、**无 PT_INTERP**（static-pie） |
| 冒烟 | WIN 载体位 `doctor ok=12 warn=0 fail=0`；WIN `-p` rc0 `RUN-CLI-6ada0e36`；alpine 3.20 rc0；bookworm-slim `doctor ok=10 warn=2 fail=0` |
| 修复已进体 | 新装机件对同一 `.gsa` 快照 `/api/runs`＝`RUN-23 … RUN-0`（旧 0.9.8＝`RUN-8, RUN-9, RUN-7, …`） |

## §1 版本与构建

- 版本 bump：`orz-bin` 0.9.8→0.9.9＋`Cargo.lock` 同名包同步（`git diff --stat`＝两行）。**不重出 0.9.8**（337 已发行同号，同号不同字节会破坏已发布资产的可追溯性）。
- Windows：`$env:PROTOC=orz/bin/protoc.exe; cargo build -p orz-bin --release -j 2`；日志 `D:/tb-eval/buildlogs/win_339_build.log`。增量（暖缓存）**1m51s**。
- Linux：`docker run --rm -v D:/CLI:/orz -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml -v D:/tb-eval/orz-target:/target -v D:/tb-eval/orz-linux:/out -v D:/CLI/scripts/build_orz_aliyun_trixie.sh:/build.sh -w /orz/orz rust:1.97-slim bash /build.sh`；日志 `D:/tb-eval/buildlogs/lin_339_build.log`；`Finished release … 5m05s`。

**构建环境注意（本批实测）**：`orz/target` 与 `D:\tb-eval` 在受限沙箱下不可写（低完整性写入限制），构建须在完整访问模式执行；Docker 引擎走命名管道，同样只在完整访问下可用。

## §2 Windows 换装与 ACAF

| 文件 | 335 件（既有 `.0.9.8-bak`） | 336 件（本批备份 `.0.9.8-336-bak`） | 0.9.9（本批） |
|---|---|---|---|
| `orz.exe` | 59,261,952 `e3df4308…` | 59,261,952 `7cf16614…` | **59,272,192 `dd0071de…`** |
| `orz-signer.exe` | 6,740,480 `49ea0702…` | 6,740,480 `54114c37…` | **6,740,480 `1b5b74a1…`** |
| `orz-acaf-provision.exe` | 6,640,128 `10917f3b…` | 6,640,128 `a9523418…` | **6,640,128 `19b8591b…`** |

- **先停栈**：换装时 `orz-signer.exe` 被在跑的 orz 栈占用（桥进程 PID 13824/29308 用装机位 `orz.exe`、signer PID 556 用载体位 `orz-signer.exe`，均为 20:11 启动的上一版）。按用户令停掉三进程后换装；**该栈即用户截图所用的 0.9.8 Web 工作台，重启后会以 0.9.9 起来**。
- 换装位：载体位 `D:/tb-eval/orz-windows`、装机位 `C:\Users\1\AppData\Local\orz\bin`；两侧 **MATCH 3/3**。
- **备份命名踩坑（已修正并留钉）**：`carrier_integrity` 的「未列文件」检测只豁免 `contains(".bak")`／`ends_with("-bak")`；首轮用 `orz.exe.0.9.8-bak-336` 备份被判「清单外新增」→ doctor `fail=1`。改名 `.0.9.8-336-bak` 后 `ok=12 warn=0 fail=0`。后续批备份请一律以 `-bak`／`.bak` 结尾。
- ACAF：`orz-acaf-provision.exe <acaf>/keystore <acaf>/signer-manifest.json` → exit 0；`binary_sha256=1b5b74a1…`；keystore 复用未覆盖。
- 载体清单：`carrier-manifest.json` 重生成（`kind/version=0.9.9/generated_at/3 entries`）**570 B `75daca1a…`**。

## §3 Linux 换装与冒烟

- 输出位即评测挂载载体位 `D:/tb-eval/orz-linux`（脚本直接 `cp` 三件到 `/out`）；清单重生成 **559 B `aff80d9d…`**。
- **注意**：脚本直写 `/out`，336 件的 Linux 三件在构建时被直接覆盖（`.0.9.8-bak` 保留的是 335 件）。336 的 Linux 字节仍可从 GitHub Release `v0.9.8` 资产（337 批上传）回取。
- 形态复验：`e_type=3 (DYN)`、`e_machine=62`、无 `PT_INTERP` ⇒ static-pie（与 335/336 同形）。
- 冒烟：
  - alpine 3.20：`--version`＝`orz 0.9.9 (linux/x86_64/release)` rc0；`--build-info` rc0。
  - debian bookworm-slim：`--build-info` rc0；`doctor`＝**ok=10 warn=2 fail=0**，其中 `[ok] carrier: 载体清单复验通过（3 件）`；warn＝容器无 Chrome／新工作区无 run journal。
  - 首轮 doctor 曾 `fail=1`（清单仍是 0.9.8 的 118,384,560），重生成清单后转绿——**清单必须先于冒烟刷新**。

## §4 修复已进体（载体级核证）

用 **0.9.9 装机件**在 `.gsa` 只读快照上起 `orz web`，读 `/api/runs`：

```text
RUN-c23e0aaa-23, RUN-c23e0aaa-22, …, RUN-c23e0aaa-1, RUN-c23e0aaa-0,
ARC-c23e0aaa-24, ARC-c23e0aaa-18, ARC-c23e0aaa-1
```

旧 0.9.8 同快照为 `RUN-8, RUN-9, RUN-7, …, RUN-1, RUN-10, RUN-0`（338 档 §2④）。⇒ 338 的服务端排序修复确已编入载体；前端 `orderedSessionRuns`/`latestSessionRun` 同车进体（`orz-web` assets 编译期内嵌）。

## §5 读数汇总

| 门 | 结果 |
|---|---|
| Windows `--version`／`--build-info` | `orz 0.9.9 (windows/x86_64/release)`／`version=0.9.9 os=windows arch=x86_64 profile=release` |
| Windows 载体位 `doctor` | `ok=12 warn=0 fail=0`（含 `carrier: 3 件复验通过`） |
| Windows 装机位 `doctor` | `ok=10 warn=2 fail=0`（无清单＝装机位口径；warn＝无 run journal、无 carrier-manifest） |
| Windows `-p` | rc0，`RUN-CLI-6ada0e36`（fake provider；须 `ORZ_ACAF_KEYSTORE/MANIFEST/BINARY` 三键，否则 fail-closed 拒跑） |
| Linux alpine／bookworm | 均 rc0；bookworm `doctor ok=10 warn=2 fail=0` |
| 载体清单 | WIN 570 B `75daca1a…`／LIN 559 B `aff80d9d…`，均 `version=0.9.9`、3 entries |

## §6 未做与残余

1. **0dn S2 真机十条未跑**：本沙箱禁止命名管道，`chrome --headless=new` 进程启动即退（338 批同因），故无法起 BrowserCDPClient 真机套件；如需跑，请在完整访问的非沙箱环境执行 `scripts/s2_real_machine.py`。
2. **未推送、未发行**：父仓/子仓未提交（工作树含 338＋339 两批改动）；README 发布面仍指 0.9.8；tag/Release 留 340。
3. **装机位无 acaf／清单**（沿 321 口径：装机位只放三件套，ACAF 面只在载体位）。
4. **Linux 336 件被覆盖**（见 §3），仅可从 v0.9.8 Release 资产回取。
5. **Windows 运行栈已停**：换装必须停栈；用户界面需重新启动一次才回到 0.9.9。

## §7 与 338 的关系

338 定性与修码（排序键失效），339 只做「打包与换装」，零新增语义改动。两批合起来才构成用户可见的「会话记录不再乱序」；338 档见 [`338 档`](338_SESSION_REPLAY_ORDER_AFTER_RESTORE_2026-10-22.md)。
