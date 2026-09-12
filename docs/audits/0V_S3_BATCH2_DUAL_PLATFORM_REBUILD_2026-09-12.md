# 0v 第二批 S3 双平台重建记录（2026-09-12）

> 入口：BACKLOG **0v** / TODO P0-0v——0v 第二批（引擎链软备忘 + 0v-A 引擎级
> 取证面）S1+S2 落码（orz `ee4ef617` / `b1e9ac65`）落在 0.4.2 冻结基线之后，
> 本批 S3 = 版本 bump **0.4.2 → 0.4.3** + 双平台三件套重建 + 冒烟 + 符号核证
> + manifest 重算，供 S4 实机复验（dna-assembly 复跑对照 + 0v-B 定向探针）。
> 基线：orz `f9fb70e4`（本批版本 bump，两文件两行）；父仓库构建前 HEAD
> `da8572d5`，两侧工作树净。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0v-s3-20260912\`（本地，
> gitignore `_windows_high_nist/**/evidence-*` 覆盖，不入库）。
> staging：`_windows_high_nist\staging-0v-s3-20260912\`（同 gitignore 覆盖）。
> **声明**：本次重建只刷新载体目录 `D:\tb-eval\orz-windows` / `orz-linux`；
> 0.4.0 正式发布资产未被触碰；0.4.2 载体三件已就地备份为
> `D:\tb-eval\orz-linux\*-0.4.2.bak`（Linux 侧；Windows 侧 0.4.2 哈希见
> [0.4.2 重建记录](0V_S4_REFRESH_REBUILD_2026-09-11.md) §1 可复验）。

## 0. 版本 bump（源冻结基线）

- 提交：orz `f9fb70e4`，两文件两行（`crates/orz-bin/Cargo.toml` + `Cargo.lock`）。
- 理由：0.4.2 基线之后落了 0v 第二批 S1+S2（软备忘语义 + 0v-A 取证面 +
  复审钉字测试），不 bump 会让「0.4.2」同时指向两个不同制品；口径同
  0.3.1 / 0.3.2 / 0.4.1 / 0.4.2 过程验证版本。

## 1. Windows 三件套（宿主 release）

- 构建：`cargo build --release -p orz-bin`（PROTOC=`orz\bin\protoc.exe`，
  10m51s），`CARGO_EXIT=0`，编译行 `Compiling orz-bin v0.4.3`。
- 产物（`orz\target\release` → staging → `D:\tb-eval\orz-windows` 同步，
  SHA256 逐对吻合）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 52,419,072 | `e0d03e95aa630f2d72a4043e449ecd1987834217cfb9fc6d772447c9f0da90a8` |
| orz-signer.exe | 6,738,432 | `755226a4ff6aaa62646916646612a3ada24ec01c1548c0e56e0d3a06d42c7ee0` |
| orz-acaf-provision.exe | 6,642,176 | `a0ca1f175846a4b2dfd6f5f627b1beb2eaab0c204a7b29f7e75fca99342ef179` |

- 与上批（0.4.2）差异：`orz.exe` 52,254,208 → **52,419,072 B**（+164,864 B，
  软备忘 + 取证面 + O-2 上限信封全量表代码增量）；`orz-signer.exe`
  6,742,528 → 6,738,432 B（−4,096 B，不含改动面的同源码重链接非确定性段）；
  `orz-acaf-provision.exe` 字节数不变。
- staging `SHA256SUMS.txt` 三件 + 载体 `sha256sum -c` 逐对 **OK**。
- 冒烟（`windows-smoke.txt`）：orz-acaf-provision usage exit 1；orz-signer
  `signer manifest not found` fatal exit 1——与上批形态一致。

## 2. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` + `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约：`D:/CLI:/orz` + `-w /orz/orz` + `/target` 全量重编
  + `/out` 直出载体），`BUILD_EXIT=0`，编译 **20m39s**（`-j 1`），日志
  `D:\tb-eval\orz-linux\build-20260912-043.log`，编译行
  `Compiling orz-bin v0.4.3`。
- **操作沉淀**：首次启动即败（`docker: the working directory 'B:/Git/orz/orz'
  is invalid`）——Git Bash 的 MSYS 路径转换把 `-w /orz/orz` 改写为主机路径；
  `MSYS_NO_PATHCONV=1` 前缀重跑即过。属宿主侧调用层工件，非装置缺陷，登记
  供后续批次直接采用。上批登记的「容器未挂 CARGO_HOME → 依赖图全量重编」
  观察保持成立（本批仍全量 222 crates；耗时 20m39s < 上批 28m43s，差异未隔离，
  只作登记）。
- 产物（`D:\tb-eval\orz-linux` 直出 + `.0.4.2.bak` 备份留存）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 109,540,832 | `eb7241343efb7b572cd7c2aeaad5fbb1de75b554551ecb286a9a1a60e84c5e9f` |
| orz-signer | 1,396,424 | `e1b84ac559bfe4213d3c2f9b40162d1720f1ca3a48aecd5b5498d9b14a27aaf3` |
| orz-acaf-provision | 1,215,056 | `6092b85bd835f51e32f2472af5df4262c0bb04f110b3c48cebf803f142cc4cc3` |

- 与上批（0.4.2）差异：`orz` 109,387,800 → **109,540,832 B**（+153,032 B，
  软备忘 + 取证面代码增量）；`orz-signer` +192 B；`orz-acaf-provision`
  +240 B（后两者不含改动面，同源码重链接非确定性段，非语义变化）。
- 静态核验（`linux-elf-static-check.txt`，`elf-phdr.py`）：三件均
  `e_type=0x3`（ET_DYN）+ `e_machine=0x3e`（x86-64）+ **PT_INTERP=0**
  （musl static-pie），`static-pie OK`。
- **双向加载冒烟全绿**（预期 exit 1 形态三件一致）：
  - bookworm-slim（glibc 环境，`linux-elf-smoke.txt`）：provision usage
    exit 1 / signer manifest 缺失 fatal exit 1 / `orz --version` 无 TTY
    `tui io error` exit 1；
  - alpine（musl 原生环境，`alpine-elf-smoke.txt`）：三件形态逐条相同。
  - 边界：Windows 宿主不能直接执行 ELF（`Exec format error` 126），「宿主」
    第二环境由 alpine 承担——静态 musl ELF 的「任意 Linux 环境可加载」语义
    由 glibc/musl 两个发行版环境共同核证。

## 3. 接线符号核证（双平台 orz 二进制字节串命中）

证据：`binary-symbols-win.txt` / `binary-symbols-linux.txt`（扫描器
`scan-binary-symbols.py` 随证据留存，本批新增 0v 第二批标记 6 项）。

| 标记 | Windows orz.exe | Linux orz |
|---|---|---|
| `[车道:本地浏览器检索\|推荐首选]` | 2 | 2 |
| `[车道:原生检索]` | 2 | 2 |
| `browser_launch_result` | 3 | 3 |
| `browser_control` | 75 | 59 |
| `retrieval_enabled` | 5 | 8 |
| `wait_load` | 4 | 7 |
| `setmkt=en-US` | 1 | 1 |
| `low_quality` | 23 | 23 |
| `engine_attempts` | 14 | 14 |
| `[INITIAL_ROUND_INQUIRY v0.1]` | 1 | 1 |
| `[/INITIAL_ROUND_INQUIRY]` | 1 | 1 |
| `开局问询（一次性，非强制模板，不打断动作）` | 1 | 1 |
| `post_tool_batch_gap` | 7 | 7 |
| `template_sha256_initial_round` | 1 | 1 |
| **`not_attempted`** | **1** | **2** |
| **链成功固定 reason（not attempted in this call…earlier engine）** | **1** | **1** |
| **ceiling reason（session engine-navigation ceiling）** | **1** | **1** |
| **收紧后 all_engines_failed 文案（all three SERP engines were attempted and failed）** | **1** | **1** |
| **`serp-attempts`（取证面路径/日志串）** | **48** | **15** |
| `0.4.3` | 15 | 4 |

- 新批标记双平台**全命中**：软备忘（`not_attempted` + 固定 reason + 收紧文案
  + ceiling reason）与 0v-A 取证面（`serp-attempts`）接线面完整。
- `ordered_engines`：Linux 命中 1（debuginfo 路径段）、Windows 0（release exe
  不含路径串，debuginfo 落 PDB）——函数名非字节串合约，软备忘语义由
  `not_attempted` + 两条 reason 串双平台核证，非缺陷。
- `initial_round_inquiry`（第 34 族法官名）双平台 0——与上批同形，家族名不
  链接进 orz 主二进制（执法 CLI 侧资源），非缺陷。
- `low_quality` 11 → 23、`engine_attempts` 2 → 14：S1+S2 新增字符串（脱敏/
  取证/信封路径）所致，双平台计数一致。

## 4. 仓库门禁

- `python scripts/check_repository.py` → `valid: true`、`error_count: 0`、
  `orz_source_manifest_files: 1441`。
- manifest 重算差异面 = 恰两行（`Cargo.lock`、`crates/orz-bin/Cargo.toml`），
  与 bump 改动面逐条对应；顺序按既定纪律「先 orz 提交 → 再生成 manifest」。
- 构建产物仅落 gitignore 覆盖的 `target/`、证据/staging 目录与载体目录；
  构建后 orz 子模块工作树净（HEAD `f9fb70e4`）。

## 5. 结论与边界

- **S3 闭合**：双平台三件套 + manifest 重建完成，ELF 静态核验 + 双向加载
  冒烟绿（bookworm glibc + alpine musl），接线符号双平台全命中，载体
  `D:\tb-eval\orz-windows` / `orz-linux` 已刷新为 0.4.3（0.4.2 三件已备份）。
- **0v S4 实机复验待放行**（唯一剩余步骤）：(a) dna-assembly 复跑对照
  （软备忘语义 + 取证面，判据 11/12）+ (b) 定向探针 0v-B（判据 1/5/7 与
  9/10）。执行器沿用 `scripts/run_0x_0v_s4_live_verify.py`（含 Chromium
  只读挂载）与 journal 分析器 `scripts/analyze_0x_0v_s4_journal.py`；取证面
  新增离线复核件 `runs/<run>/serp-attempts/*.json`。
- 未验证：本批只做重建与加载冒烟 + 字节串核证；软备忘/取证面的实机行为
  仍须 S4 复跑取证。Windows 侧只做确定性退出码冒烟（未复做 TUI 起手观察）。
