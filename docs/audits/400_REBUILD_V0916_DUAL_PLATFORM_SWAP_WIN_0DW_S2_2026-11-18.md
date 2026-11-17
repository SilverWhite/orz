# 400 批：0.9.16 双平台载体重建出件＋Windows 换装进体——0dw S2（2026-11-18）

> 用户令：「请进行重建并做换装吧」。性质＝载体重建＋换装批（0dw S2 进体）：源冻结 orz `5d3bec4d`（399 处置 `851d5341`＋版本 bump `a3b85c48`＋构建脚本头注修正 `5d3bec4d`；398/399 全部落码进体）。**0dw S2 自此达成**，S3 真机核证待用户手动首启；未闭合计数 73 不变（0dw 开放至 S3 读数）。未推送、未发行。

## §1 版本 bump

`0.9.15` → `0.9.16`：orz `a3b85c48` 恰两行（`crates/orz-bin/Cargo.toml`＋`Cargo.lock`，`env!("CARGO_PKG_VERSION")` 单源扇出——`--build-info`／banner／capability `orz_version`／桥 `bridge_version`／**0dw preflight `carrier_version` 章**同串）。

## §2 Windows 构建（exit 0）

`vcvars64`＋`cargo build -p orz-bin --release -j 2`（三 bin 同 crate：`orz`／`orz-signer`／`orz-acaf-provision`）：8m36s exit 0（398/399 触碰 orz-loop/assurance/host ⇒ 重编级联，较 392 增量 4m51s 为久，如实记）。日志 `D:\tb-eval\buildlogs\win_0.9.16_build.log`。

## §3 Windows 换装（394 程序全步）

1. **停栈双证**：`tasklist` ＋ `Get-Process` 均零 orz 进程（换装前后同证）。
2. **备份**：装机位三件 `*.0.9.15-400-bak`＋ACAF `signer-manifest.json.bak-20261118-400`（备份 cmp 逐位一致；回滚＝停栈→还原→重 provision）。
3. **复制＋MATCH 3/3**：暂存 `D:\tb-eval\orz-windows-0.9.16\` → 装机位 `%LOCALAPPDATA%\orz\bin`，cmp 三件全 MATCH。
   - orz.exe 68,623,360 `179f46d609645a6bfc2c804b3199e5cb5edc61dce2cdfe6e07faad54d98a23c5`
   - orz-signer.exe 6,740,480 `56aaa228015040f03b61ec9f41eabe0332b67df9fdd790ed91d782c03cccc00c`
   - orz-acaf-provision.exe 6,644,736 `e6912d97fc6d05a2a290f383c91f36176602e4dae55f89d9b4330eba76fce8a5`
   - 暂存载体清单 `carrier-manifest.json` 3 entries 0.9.16（装机位本不放清单口径，394 同）。
4. **ACAF 重 provision** exit 0：`orz-acaf-provision <keystore> <manifest>` 绑**新 signer** `56aaa228…`（manifest `binary_sha256` 逐位一致）；keystore 两件（`installation-key.dpapi`／`.json`）前后 sha256 逐位未动。
5. **doctor** 装机位实跑（`doc_0.9.16_install.txt`）：exit 0，**ok=10 warn=2 fail=0**（两 warn＝runs 空＋无清单，与 394 同构环境性）。
6. **版本串三面**：`--version`／`--build-info`＝`0.9.16 (windows/x86_64/release)`。

## §4 0dw S1 进体核证（字面量面；活体判据留 S3）

- `model_call_usage` ×2（EventType serde 枚举表）；`call_seq` ×7（恰 8 字节 movabs 立数形态）。
- `usage_complete`／`carrier_version`／`binary_sha256` 无连续明文＝**MSVC 拆立数构造**（`movabs "usage_co"`＋栈长字段 `0x0e`＝len 14；`movabs "carrier_"`；`movabs "binary_s"`）——键长逐位吻合，398/399 代码在体实锤（连续明文缺失系编译器将 9+ 字节键拆栈上重叠立数，非缺码；`face_estimate_tokens` ×21 命中多为 `model_face_estimate_tokens` 子串，据实注记）。
- 活体两判据（preflight 五键入 journal、逐调用 `model_call_usage` 落账与 CJK 偏差读数）＝**S3 真机首启**核证，随用户手动首启窗。

## §5 Linux 构建（exit 0；两次实证修正）

- 首轮失败×2：①Docker Desktop 未启动（拉起）；②**容器代理透传死路**——Docker Desktop `ProxyHTTPMode="manual"` 指向宿主 Clash `127.0.0.1:7890`（未运行，netstat 无监听），apt 全线 `connecting to 127.0.0.1:7890: connectex refused`。处置沿 **052 先例**：`manual → disabled`（settings-store.json 备份 `.bak-400`）＋Docker Desktop 重启 → 构建成功 → **settings-store.json 字节级还原**（sha256 逐位一致）＋再次重启。另：阿里云镜像 SSL 持续中断（`unexpected eof`，非瞬态）——代理关闭后 debian 直连 apt 正常，脚本内阿里云换源步骤本次无害通过。
- 第二轮失败×1：**复原脚本头注挂载形态笔误**——头注写子目录挂载 `D:\CLI\orz:/orz/orz`，而 orz-assurance `include_str!("../../../runtime/*.json")` 需**父仓挂载** `D:\CLI:/orz`（392 批档明载「首轮挂子目录失败实证修正」；本次复踩复证）。修正挂载后 exit 0（3m53s 增量）。脚本头注已修正入库（orz `5d3bec4d`）。
- 出件（`D:\tb-eval\orz-linux\`，清单 3 entries 0.9.16）：
  - orz 98,571,480 `038ab87b06e92344954158652e50ae78236037d6dd2753c722615c6071731131`
  - orz-signer 1,401,224 `82acefaf0f84f97ca01d713c6c8707ae996985a7def618563ef4d72342516d24`
  - orz-acaf-provision 1,217,488 `d6736ecb9944a43f250c6275705b7ac1648adf71f8b9161113405690bbc8655a`
- 核证：ELF 三件 `e_type=3`＋`PT_INTERP` 无＝static-PIE（392 口径）；冒烟＝alpine:3.20（musl 侧）＋rust:1.97-slim（glibc 侧）双容器 `orz 0.9.16 (linux/x86_64/release)`。**bookworm-slim 腿未跑**（本地无镜像＋代理禁用下 Docker Hub 直拉不通；392 双容器口径以「musl＋glibc 两形态」保住，据实注记）。**Linux 未换装**（Linux 载体部署面不在本机；出件暂存随下批/发行取用）。

## §6 父仓随批件

- `orz_source_manifest.sha256` 重算 **1507 条**（9 行变更＝399 触碰的 orz 文件＋bump 两件）。
- `check_repository.py`：error_count **39** 全数 ledger-slimming 既有类（超长＝旧批行 TODO:25/26、行龄＝2026-10-18~27 旧行自然越 21 天界）；本批唯一新引入（399 行 1562>1200）已随批裁至 1065——397/398 的「既有类登记不修」先例不动。

## §7 边界与不做清单（本批）

- **不推送、不发行**（随用户令）；GitHub Release 与包构建未动。
- **Linux 不换装**（部署面不在本机）；Linux 件暂存随发行批/后续部署取用。
- **S3 真机判据不在本批**：用户手动首启后核证＝①run_preflight 五键（`carrier_version`＝0.9.16＋`binary_sha256`＝`179f46d6…` 载体清单同口径）②逐调用 `model_call_usage` 落账且 CJK 估算 vs 实测偏差可复算③阶梯触发与实测对齐读数入档④0dy/0dz/0dx/0dt 合并首启窗同车判据。
- Docker Desktop 代理已按 052 先例字节级还原（manual/7890 形态照旧）——下次容器构建若 Clash 仍未运行需同款处置。
- 换装判据中「web 桥 skew＝false／0dx 活体面」等活体项属 S3 真机窗，不在本批机械面。

## §8 入口

- 前批：[399 处置档](399_0DW_S1_REVIEW_REMEDIATION_2026-11-18.md)／[398 批档](398_0DW_S1_RULER_TOKENIZER_PERCALL_USAGE_PREFLIGHT_2026-11-17.md)／0dw 立项档 [`0DW_*_REGISTRATION`](0DW_CONTEXT_FACE_SCALE_RULER_ALIGNMENT_REGISTRATION_2026-10-30.md)。
- 子仓：`5d3bec4d`（头注修正）← `a3b85c48`（bump）← `851d5341`（399）← `e5103563`（398）。
- 台账：TODO 400 行／BACKLOG 本批记录指针／索引头 v4.360。
