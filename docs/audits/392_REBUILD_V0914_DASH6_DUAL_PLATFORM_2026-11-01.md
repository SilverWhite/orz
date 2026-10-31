# 392 批：0.9.14-6 双版本（双平台）载体重建出件——0dx 进体（2026-11-01）

> **用户令**：「请进行重建吧，本轮做双版本重建」——按 357 双平台先例口径（「Linux 和 win 同版本代码即可，统一命名」）执行：Windows release 件＋Linux musl 件同版本代号 **0.9.14-6**、同源码出件。本轮**仅重建出件暂存**：不换装（装机位在役仍 0.9.14-5，沿 385 停栈口径待下一换装批）、不发行（无 GitHub Release 指令）、不推送。
> **范围**：0dx 全量进体＝390（S2 落码五件，orz `40b51786` 前身 `b8f9ebc4`）＋391（审查处置，orz `40b51786`）编入载体；版本 bump 0.9.14-6（orz `ffba18bb`：`crates/orz-bin/Cargo.toml`＋`Cargo.lock` 恰两行）；父仓源清单钉源（`orz_source_manifest.sha256` 重生成 1504 条）。
> **计数**：71 不变（0dx 保持开放：S4 真机核证随其后；换装后活体判据沿批档 §5）。

## §0 结论

双平台同源出件 **0.9.14-6**：Windows `cargo build -p orz-bin --release -j 2` exit 0（**4m51s 增量**——0.9.14-5 出件后 `target/release` 缓存在位，仅变更 crate 重编，非 clean 全量；384 的 30m02s 系依赖树冷全量口径，本批如实登记差异）、零警告；`orz.exe --version` → `orz 0.9.14-6 (windows/x86_64/release)`、`--build-info` 同版本；三件 cmp 逐字节 MATCH 3/3、carrier-manifest 3 entries、在体字面量核证 0dx 面全在（§2）。Linux musl 三件 Docker 重建 exit 0（复原脚本首轮 protoc 缺件失败补齐后过，§3），`orz 0.9.14-6 (linux/x86_64/release)`、static-PIE 形态核验。Docker Desktop 代理配置按 052 处置字节级还原（§4）。

## §1 版本与源码门

- orz 子模块 `ffba18bb`（bump）＝`40b51786`（391 审查处置）→`ffba18bb`；双平台构建均自 `ffba18bb` 工作树。
- 父仓钉源：`python scripts/generate_orz_source_manifest.py` → 1504 条（384 钉源 1506 条基数上 390/391 文件面增删）。
- 测试基线＝391 批读数（同一源码态：loop 982/0/3、host 459/0/6、tui 186/9 环境口径、assurance 311/0、bin 六套件绿、pytest 109/0）。bump 仅版本串两行，不重跑（沿 357 §1 口径）。

## §2 Windows 重建与出件

构建 `cargo build -p orz-bin --release -j 2` exit 0（日志 `D:\tb-eval\buildlogs\win_392_build.log`；4m51s 增量、零警告）。`--version` → `orz 0.9.14-6 (windows/x86_64/release)`；`--build-info` → `version=0.9.14-6 os=windows arch=x86_64 profile=release`；signer 无 manifest fatal exit 1（357 同形态，预期）。

| 产物 | 字节 | 较 0.9.14-5 | SHA-256 |
|---|---:|---|---|
| `orz.exe` | 68,577,792 | +186,368 | `1daa8706ed752598ba6273ff1320ae8b4e5957cc2b55ce8a218da0dd37562a03` |
| `orz-signer.exe` | 6,740,480 | ±0 | `cd5cb7ecbc234cd98fa234dc8cb95d1e8f378dc8551fe7f9fa6b312498985acf` |
| `orz-acaf-provision.exe` | 6,644,736 | ±0 | `df2c28712e2658258c313a826a54c609ae1e6242923f29cfc51fe7e12627f242` |

- 出件暂存 `D:\tb-eval\orz-windows-0.9.14-6\`；`cmp` target/release ↔ 暂存 **MATCH 3/3**。
- 载体清单：`generate_carrier_manifest.py <暂存> --version 0.9.14-6` → **3 entries**。
- 在体字面量核证（`grep -a -c -o` 等价计数，384 先例口径）：0dx 面——`era_boundary`×2、`/checkpoint`×12、`/btw`×11、`unprefixed_queued_input`×1、`era_supplement_intent`×7、`fold_refused_same_era`×7、`0.9.14-6`×5；391 新面 `fold_refused_same_era` 在体＝审查处置随件确认（`EraRunState`/`production_slider_tokens` 计 0 系符号/路径字面量不入终件，合理阴性）。运维注记：出件首轮 cp 因 Git Bash `\$f` 转义吞分隔符把三件误写为兄弟文件 `orz-windows-0.9.14-6$f`（cmp 假 MATCH）——发觉后删除垃圾件、正斜杠路径重做，暂存目录现三件＋manifest 干净（机械失误如实登记，未进任何账面）。

## §3 Linux musl 重建与出件

**前置注记（如实登记）**：原构建脚本 `D:\tb-eval\build_orz_aliyun_trixie.sh` 已随 0du 目录清理灭失——本批按 052/346/357 三批档在案口径**复原脚本**（ruz:1.97-slim trixie 容器、apt 阿里云源、musl-tools、musl target、三件直写宿主暂存；脚本随 orz 仓入库 `build_orz_aliyun_trixie.sh`）。Docker Desktop 冷启动就绪（engine 29.6.2）。

- **代理处置（052 同款先例）**：daemon 代理 `manual/http://127.0.0.1:7890` 指向未运行的 Clash——Docker Hub 拉取拒连；`settings-store.json` 备份 `.bak-20261101-392` → `ProxyHTTPMode=disabled` → 重启 → **镜像改走 daocloud 透明代理源直拉成功**（`docker.m.daocloud.io/library/rust:1.97-slim` → 本地 tag `rust:1.97-slim`；rustc 1.97.1 与 346/357 同基）。构建完成后配置**字节级还原＋再次重启**（§4）。
- 构建调用（ORZ-BUILD-MOUNT-001＋`MSYS_NO_PATHCONV=1`）：`docker run --rm` 四挂载（源码 `/orz/orz`、出件 `/orz/out`→`D:\tb-eval\orz-linux`、target/cargo 缓存外挂 `D:\tb-eval\orz-linux-{target,cargo}`——与宿主 Windows 构建的 `target/` 完全隔离）；日志 `D:\tb-eval\buildlogs\lin_392_build.log`。
- **首轮失败如实登记**：`orz-tools-api` build.rs 缺 `protoc`（仓内只 vendor Windows 版 `bin/protoc.exe`，容器不可执行；原脚本应有 apt 件随灭失丢失）——复原脚本补 `protobuf-compiler` 后二轮 exit 0（crate 缓存续用，约 20 分钟级）。

| 产物 | 字节 | SHA-256 |
|---|---:|---|
| `orz` | 98,521,240 | `d82c3d9430f38a18dda75790f1674f9872431a6fd5faacc1eac2142df091e75c` |
| `orz-signer` | 1,401,392 | `43f1a11fab8a1b5fb254f4bea4600b10d5d4158f0ac2102ad5a891dfb55e2ccb` |
| `orz-acaf-provision` | 1,217,680 | `e69f2cd669994c82bef429f8521c67ec7ee59a9426e42da8b689c466fcd3a39c` |

- 版本核验：slim 容器内 `--version` → `orz 0.9.14-6 (linux/x86_64/release)`、`--build-info` 同版本；Alpine 3.20 冒烟 `--version` 同值；ELF 三件 `e_type=3`（PIE）＋ **`PT_INTERP=0`（static-PIE，346/357 同形态，宿主 Python 逐头解析实测）**。
- 在体字面量核证：`era_boundary`×7、`/checkpoint`×12、`/btw`×9、`0.9.14-6`×5、`era_supplement_intent`×7、`fold_refused_same_era`×7——0dx 面进体确认。
- Linux 侧不重 provision（沿 320/336 口径）；Linux 暂存目录仅三件，载体清单随换装/发行批统一重生成。

## §4 Docker 代理还原（052 处置收尾）

构建收尾后：`settings-store.json` 自 `.bak-20261101-392` 字节级还原（sha256 复核 `2e3ac271…`）→ Docker Desktop 再次重启 → 读数与原值一致（`manual / http://127.0.0.1:7890`）。

## §5 换装与活体判据（下一批预留）

本批不换装。下一换装批（沿 385 口径）判据预留：Windows 三位 cmp、ACAF 重 provision（新 signer `cd5cb7ec…` 逐位）、载体清单 doctor、目标工作区 web 桥 `/api/boot` bridge/child 均 0.9.14-6 且 `version_skew=false`、0dx 活体面（`/checkpoint` 切换回执、`era_boundary` journal 事件、`/btw` 排队）真机核证——与 0dx S4 真机核证（逐时代协议触达数对历史衰减曲线）合并执行。

## §6 入口

- 设计权威 [`CONTEXT_CHECKPOINT_ERA_DESIGN v1.1`](../CONTEXT_CHECKPOINT_ERA_DESIGN_2026-10-30.md)／390 批档／[391 批档](391_0DX_REVIEW_REMEDIATION_2026-11-01.md)。
- 构建：`orz ffba18bb`（bump）／复原脚本 `orz/build_orz_aliyun_trixie.sh`／日志 `D:\tb-eval\buildlogs\{win,lin}_392_build.log`／暂存 `D:\tb-eval\orz-windows-0.9.14-6\`＋`D:\tb-eval\orz-linux\`。
- 台账：TODO 392 行／BACKLOG 本批记录指针／索引头 v4.352。
