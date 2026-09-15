# 0.5.2 载体重建与 0ai 狗粮考核测试放行（2026-09-16）

> 入口：TODO **P1-0ai**（前置：载体重建）/ BACKLOG **0ai**·**0ah**（批序 ② 狗粮考核
> 测试放行）。用户 2026-09-16 指示：「请进行重建吧，随后放行 0ai 的狗粮测试，
> 不设置墙钟」，并给定 0ai 题面原文（逐字入 `task.txt`，见 §6）。
> 源冻结基线：orz **`a580eb08`**（`feat/fusion-architecture`；＝ 0ah S1 五连批
> `61982a56` ＋ 版本 bump `c0871f1b` ＋ orz-tui 构建断裂修复 `a580eb08`，
> 0.5.1 冻结 `dbb42b1d` 以来共 10 提交）。产物：Windows x86_64 三件套 + Linux
> x86_64 musl static-pie 三件套 + 载体换装（`D:\tb-eval`）。**本批不含 GitHub
> Release / 推送**（用户未指示；0.5.1 先例中的「提交推送发行」为当时单项指
> 示，不沿用）。证据目录：`D:\tb-eval\evidence-052-20260916\`（本地件，不入
> 库）；构建日志 `D:\tb-eval\windows-build-0.5.2.log`、
> `D:\tb-eval\orz-linux\build-20260916-0.5.2.log`。

## 0. 源冻结基线与版本 bump

- bump：orz `c0871f1b`，两文件两行（`crates/orz-bin/Cargo.toml` + `Cargo.lock`，
  0.5.1 → **0.5.2**）；父仓 `orz_source_manifest.sha256` 同批重算（§5）。
- 基线内容（`dbb42b1d..a580eb08`，10 提交）：0ac S3-b/①-b 投递侧（`f03b2a4f`/
  `1deeba75`）、①-a 检索补强 G1–G4（`7e151ed1`）、0ae D0–D4（`f0040557`）、
  0af 契约对账（`8512fc71`）、四联批审查修复（`183fbb08`）、压缩收口窄边沿
  （`1f303cf4`）、0ah S1 五连批 squash（`61982a56`）、bump（`c0871f1b`）、tui
  修复（`a580eb08`）。
- **`GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP` 随本载体一并进载体**：修复提交
  `ea777918` 实测为 `61982a56` 祖先（`git merge-base --is-ancestor` 通过），
  其条目「载体重建待放行」随本批闭合；S4 实机复验语义不变。

## 1. 载体重建拦下的构建断裂（orz-tui，`a580eb08`）

`cargo build --release -p orz-tui`（随 `-p orz-bin` 依赖图进入构建）报
**E0004 non-exhaustive match**：`orz-tui/src/bridge.rs:81` `run_event_to_tui`
未覆盖 0ac S3①-b 新增三事件族 `RetrievalProgress` / `RetrievalResultSegment` /
`ResultDelivered`。**断裂自 `f03b2a4f`（2026-09-15）引入即存在**，此前未被发
现的原因：orz-tui 不在任何常驻测试门内（回归面为 orz-loop / orz-host /
orz-assurance / orz-tools，均不依赖 orz-tui），且 0.5.1 载体先于 `f03b2a4f`
冻结。定性：同 **ORZ-PLATFORM-TARGET-001** 族（「一处窗口全绿 ≠ 全目标可
编」）——本次是「事件面扩枚举后非 exhaustive match 无门禁拦截」的姊妹形态。
修法（机械，`a580eb08`）：三族降级 `TuiEvent::Unknown`（沿同函数
`BudgetCueInjected` 与已退役 DC 族「journal-only 中性事实无 TUI 投影」先例，
行为面＝TUI 回放显示事件名字符串，无新增语义）。**遗留观察项（不动计数）**：
`EventType` 扩族时 orz-tui match 无编译期/门禁防线，编译断裂只能在「构建进
载体」时暴露——建议后续与候选 3（families.rs 拆分）或账本机械化批次同车评估
「workspace 全成员 `cargo check` 进门禁」。

## 2. Windows 三件套（宿主 release）

- 构建：`cargo build --release -p orz-bin`（`PROTOC=orz\bin\protoc.exe`），
  `Compiling orz-bin v0.5.2` → `Finished release profile [optimized] target(s)
  in 29.88s`，`CARGO_EXIT=0`；warning 仅既有两项 `dead_code`
  （`xai-tty-utils process_alive` / `orz-host register_live_call_job`），无 error。
- 产物（`orz\target\release` → `D:\tb-eval\orz-windows` 换装，SHA256 逐对
  MATCH）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 53,624,320 | `7f14eb177c80c30ae9164d823f3a3f7a778233260c74351d8e9ea2d12969347c` |
| orz-signer.exe | 6,742,528 | `5b363acac0f80c5e6c1373df7ce72193621ad5363813cf99ef9640a78b366497` |
| orz-acaf-provision.exe | 6,642,176 | `96cacd99c59f678f85afd771c75bafc31a0d350b5335dd06bdbd1582cc7810e7` |

- 与 0.5.1 差异：`orz.exe` 52,837,376 → **53,624,320 B**（+786,944，0ac ①-b
  ＋0ae＋0ah S1 等代码增量）；signer/provision 大小不变、哈希重建更新。
- 冒烟（换装后原位复跑）：provision usage exit 1；signer manifest 缺失 fatal
  exit 1——与 0.5.1 形态一致。
- 字面量核证（`grep -a -c -o`）：0ah S1 六标记全在——`ORZ_SLIDER_WINDOW_TOKENS`
  1、`ORZ_SLIDER_RESIDENT_TOKENS` 1、`context_scale` 39、`model_summary` 9、
  `model_selected` 7、`hard_950k_intercepted` 7；**退役标记为 0**——
  `ORZ_FOLD_TRIGGER_TOKENS` 0、`attention_ladder` 0（D2 下线与旧 env 退役一致）。

## 3. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` + `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约 + `MSYS_NO_PATHCONV=1`）容器内构建，
  `Compiling orz-bin v0.5.2 (/orz/orz/crates/orz-bin)` → `Finished release
  profile [optimized] target(s) in 31m 44s`，`LINUX_BUILD_EXIT=0`
  （`build-20260916-0.5.2.log`；`/target` 沿用 0.5.1 缓存）。
- **网络摩擦与处置（登记，编号留台账）**：首轮 apt 五连败
  （`Unable to locate package ripgrep` 等）。根因链＝Docker Desktop
  `ProxyHTTPMode: manual` + `OverrideProxyHTTP(S): http://127.0.0.1:7890`
  指向**当前未运行的 Clash**（宿主 7890 无监听、注册表系统代理亦无值）——
  容器流量经 `http.docker.internal:3128` 中继强制走死上游（清容器 env 无效，
  拦截在 Desktop 层）。处置：备份 `settings-store.json` →
  `ProxyHTTPMode` 临时切 `disabled` → 重启 Docker Desktop（F-022 冷启动同形，
  本次 2 次均 ~5–10 s 就绪）→ 容器直连 apt 验证通过 → 构建完成后**字节级还
  原配置并再次重启**（还原后读数 `manual / http://127.0.0.1:7890` 与原值一
  致）。宿主直连 `https://deb.debian.org` 200（无代理路径本来就通）。
- 产物（构建脚本直写 `D:\tb-eval\orz-linux`）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 110,986,272 | `9aa88dd1053bfb178e09b599f2f0b37b1b6174e17b8ee4679906ce012c790e18` |
| orz-signer | 1,397,864 | `bb6f575b8ac0ad8da8c9af80c6eac96878ba9bb135e5335f0bc8d2e569b46d94` |
| orz-acaf-provision | 1,216,664 | `13626a1c6f9adebda203e1e15dcf871790d333582ddbf24e77ca5be207b3031e` |

- 与 0.5.1 差异：`orz` 110,099,784 → **110,986,272 B**（+886,488）；signer
  1,397,552 → 1,397,864；provision 1,216,344 → 1,216,664。
- 静态核验（Python 解析 ELF 头，bookworm-slim 无 binutils）：三件均
  `e_type=3`（ET_DYN）+ `e_machine=62`（x86-64）+ **PT_INTERP=0**（musl
  static-pie）。
- **双向加载冒烟全绿**（预期 exit 1 形态三件一致）：`debian:bookworm-slim`
  与 `alpine:3.20` 两侧 orz / orz-signer / orz-acaf-provision 均 exit 1。
- 字面量核证（Linux `orz`）：`ORZ_SLIDER_WINDOW_TOKENS` 1、`context_scale`
  52、`model_summary` 13、`hard_950k_intercepted` 7、
  `ORZ_FOLD_TRIGGER_TOKENS` 0、`attention_ladder` 0——与 Windows 面一致。

## 4. 载体换装（`D:/tb-eval`）

- **windows**（`D:\tb-eval\orz-windows`）：`cp old → *.0.5.1-bak` 后三件
  0.5.2 覆盖换装，post-swap `sha256sum` 与构建产物逐对一致（§2 哈希即证）；
  留档 `*.0.5.0-bak`（0.5.0 原件）与 `*.0.5.1-bak`（0.5.1 原件）。
- **linux**（`D:\tb-eval\orz-linux`）：**换装瑕疵与更正（如实登记）**——构
  建脚本直写 `/out`，0.5.1 三件在构建落位时即被覆盖； initially 以 0.5.2 现
  物误标 `*.0.5.1-bak`，随即发现并以
  `_windows_high_nist/staging-0ac-s3-20260914/linux/` 真件替换（sha256 与
  0.5.1 审计逐条一致：`149ab446…` / `97978a52…` / `9ee21fcc…`）。教训：脚本
  直写载体的批次，`.bak` 必须在构建前预留。

## 5. manifest 重算与门禁

- `python scripts/generate_orz_source_manifest.py` → **wrote 1450 entries**；
  差异面恰 **3 行**（`Cargo.lock` / `crates/orz-bin/Cargo.toml` /
  `crates/orz-tui/src/bridge.rs`）——与两个冻结提交（bump + tui 修复）完全对应。
- `python scripts/check_repository.py` → **`"valid": true` / `error_count: 0`**。

## 6. 0ai 狗粮考核测试放行（批序 ②）

- **放行配置**：题＝0ai 重文件拆分（`orz-loop/src/host_exec.rs`，基线 9,184
  行）；执行者＝orz 自身（0.5.2 载体）；**隔离工作区**＝
  `D:\tb-eval\dogfood-0ai-20260916\`（父仓 + orz 子模块整链克隆，冻结基线
  `a580eb08`）；题面 `task.txt`＝用户 2026-09-16 指定原文逐字：
  > 请先查看CLI_PROJECT_INDEX.md路由，随后回查所需文档。当前需要处理的任务为0ai任务，请直接进行，完成后暂不提交/推送。0ai任务结束后，请按照项目惯例落报告文档，如在任务过程中发现框架内部存在的摩擦项，请一同记录在报告文档中。
- **无墙钟**：`ORZ_MAX_WALLCLOCK=0` 显式置 0（`parse_max_wallclock`：0 =
  unbounded；兼中和宿主可能的常驻值）。stall 哨兵维持默认（生成期输出健康
  哨兵，非墙钟）。
- 启动形态：`cd <workspace> && ORZ_MAX_WALLCLOCK=0 orz.exe -p "$(cat
  task.txt)" --real --allow-write --allow-shell --allow-network > run.log 2>&1`
  （凭据＝Windows 凭据管理器 `orz-deepseek/agent` 既有通道，实测在册；
  ACAF 无 env＝影子模式，与既往狗粮 run 一致）。
- **考核口径（2026-09-16 用户裁决，不动）**：单轮/少样本如实标注、不作架构
  结论；判据读数（四件套机械读数＋驱逐增幅＋存档三键＋本地增长）与 0ai 产
  出（S1 切分图 → S2 机械搬移 → S3 回归核验）**待 run 完成后收口**；产出暂
  不提交/推送，经 S3 复核后按正常批次合回主仓。run_id 与 journal 路径落工作
  区 `.gsa/runs/`，随收尾批登记。
