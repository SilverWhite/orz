# 336 批：0.9.8 适用版换装＋0dn S2 真机十条（2026-10-22）

> **用户令**：「请进行实际适用版本的换装并做0dn S2吧」。**本批**＝把 335 构建的 0.9.8 按 S2 真机暴露的两处 Web 缺陷修正后重出、双平台换装（Windows 装机位/载体位＋Linux 载体位/评测挂载位），并在换装后的 0.9.8 上跑满 0dn S2 十条真机场景。**计数 64 不变**；0dn 仍为 S3 词表待续；本批未发行，**337 批已发行 v0.9.8**。
>
> **与 335 的关系（如实）**：335 的 0.9.8 三件套（WIN `e3df4308…`／LIN `8f61631f…`）被本批重出件取代；版本号仍为 `0.9.8`，因为当时 0.9.8 尚未发行，且 330 验收口径要求「十条在 0.9.8 载体上全绿」。旧件在两侧均有 `.0.9.8-bak`；新哈希见 §1。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源码修正 | **两处 Web 缺陷**（S2 实测）：① 版本偏斜横幅被 WS open 的「桥接已连接」flash 顶掉（4 s 消失）；② `<1 s` 极短回合整轮落在 1 s 跟随轮询之间 ⇒ journal 尾从未挂上，用户卡/工具行/终态事实全缺。修法均在 `orz-web/assets/app/main.js`，零 Rust 语义改动 |
| Windows 重建 | `cargo build -p orz-bin --release -j 2` **exit 0，38.13 s，0 warning**；`orz.exe` **59,261,952 `7cf16614…`**／signer **6,740,480 `54114c37…`**／provision **6,640,128 `a9523418…`** |
| Windows 换装 | 装机位＋载体位 **MATCH 3/3**；两侧 `.0.9.8-bak`（前件 `e3df4308…`／`49ea0702…`／`10917f3b…`）；`--version`／`--build-info` 均 `0.9.8 windows/x86_64/release` |
| ACAF | 载体 `acaf` 重 provision **exit 0**；旧 manifest 备份 `signer-manifest.json.bak-20261022-336`；新 `binary_sha256=54114c37…` ↔ signer 逐位一致 |
| Windows 清单 | `D:/tb-eval/orz-windows/carrier-manifest.json` **v0.9.8 / 3 entries**（551 B）；载体 `orz doctor` **ok=12 warn=0 fail=0** |
| Linux 重建 | docker `rust:1.97-slim`＋trixie 脚本 **exit 0**；cargo `Finished … 3m 43s`；`orz` **118,384,560 `bfcc3f0a…`**；signer/provision 与 335 同哈希（本批只改 Web 资产） |
| Linux 形态 | `e_type=3 (DYN)`、`e_machine=62`、**无 PT_INTERP**（static-pie）；alpine 3.20 `RUN-CLI-6ad9f82f` rc0；bookworm-slim `RUN-CLI-6ad9f839` rc0；bookworm `doctor` **ok=10 warn=2 fail=0**（warn＝无 Chrome＋新工作区无 run journal） |
| Linux 清单 | `D:/tb-eval/orz-linux/carrier-manifest.json` **v0.9.8 / 3 entries**；carrier 复验 3 件通过 |
| **0dn S2** | **十条全 PASS**（75 条断言，0 失败；总墙钟 148.2 s；同日复跑 `s2-098-run2` 仍 10/10、75 断言逐项过、143.3 s）；每场景真 Chrome（headless=new）＋`orz web`＋fake 替身；读数 `D:/tb-eval/s2-098/results.json`，截图同目录 |
| 门禁 | 见 §7；计数 64 不变；**已推送；337 批发行 v0.9.8**（父仓 `main` 已推送；orz `cli/feat/fusion-architecture`＝`cfed6191`） |

## §1 换装（双平台）

**为什么重出**：S2 在 335 的 0.9.8 上实测到 §2 两处缺陷；先用 335 原始件跑出 8/10，修码后重出并换装，S2 才十条全绿。重出仍为 0.9.8（当时未发行；337 批已发行）。

**Windows**

| 文件 | 335 件（`.0.9.8-bak`） | 336 件 | 变化 |
|---|---|---|---|
| `orz.exe` | 59,261,952 `e3df4308…` | **59,261,952 `7cf16614…`** | Δ0 B（Web 资产变） |
| `orz-signer.exe` | 6,740,480 `49ea0702…` | **6,740,480 `54114c37…`** | Δ0 B（重编译） |
| `orz-acaf-provision.exe` | 6,640,128 `10917f3b…` | **6,640,128 `a9523418…`** | Δ0 B（重编译） |

- 构建：`cmd /c "cargo build -p orz-bin --release -j 2 > D:\tb-eval\buildlogs\win_336_build.log 2>&1"`；日志 `Finished … in 38.13s`，0 warning。
- 换装：`C:\Users\1\AppData\Local\orz\bin` 与 `D:\tb-eval\orz-windows` 各先留 `.0.9.8-bak` 再覆盖；两侧 `orz.exe` 哈希 **`7cf16614…` 逐位一致**。
- ACAF：`orz-acaf-provision.exe D:\tb-eval\orz-windows\acaf\keystore D:\tb-eval\orz-windows\acaf\signer-manifest.json` exit 0；`binary_sha256=54114c37…`。
- 清单：`generate_carrier_manifest.py D:/tb-eval/orz-windows --version 0.9.8` → 3 entries（首轮误把 `.0.9.8-bak-336` 入册 6 条；备份改名 `.0.9.8-bak` 后重生成，沿 0bw 排除口径）。
- 核证：装机位 `orz 0.9.8 (windows/x86_64/release)`；载体 `doctor` `ok=12 warn=0 fail=0`。

**Linux**

| 文件 | 335 件（`.0.9.8-bak`） | 336 件 |
|---|---|---|
| `orz` | 118,384,560 `8f61631f…` | **118,384,560 `bfcc3f0a…`** |
| `orz-signer` | 1,397,896 `e649aa6c…` | 1,397,896 `e649aa6c…`（同哈希） |
| `orz-acaf-provision` | 1,216,344 `83e72a9a…` | 1,216,344 `83e72a9a…`（同哈希） |

- 构建命令同 335（`rust:1.97-slim`、`/target` 暖缓存、直写 `/out`）；日志 `D:/tb-eval/buildlogs/lin_336_build.log`。
- 换装位即 `D:/tb-eval/orz-linux`（评测容器挂载的载体位）；构建前已留 `.0.9.8-bak`。
- 冒烟：alpine `--version`/`--build-info` = `0.9.8 os=linux` rc0，临时 provision `binary_sha256=e649aa6c…`，`-p` run `RUN-CLI-6ad9f82f` rc0；bookworm `RUN-CLI-6ad9f839` rc0，`doctor` 含 `carrier: 3 件复验通过`。
- Linux 不重 provision 载体 acaf（沿 320/335 口径）；容器内临时 provision 仅冒烟用。

## §2 本批代码修正（S2 实测驱动）

**336-F1 版本偏斜横幅被连接 flash 顶掉（P2，Web 可见面）**

- 现象：`/api/boot` 已返回 `version_skew=true`、`bridge_version=0.9.8`、`child_version=0.9.9`，系统行也落屏；但 `acp.on('open')` 的 `flash('桥接已连接')` 把 `connBanner` 文案覆盖为「桥接已连接」，并在 4 s 后隐藏。334 §3.1 要求的持久横幅实际只在 boot 后短暂可见。
- 修法：`main.js` 抽 `renderVersionSkewBanner()`（从 `state.bridgeVersion/childVersion/childProbe` 重绘、清掉 flash 自动隐藏 timer），`applyBootFacts` 与 `acp.on('open')` 都调用；无偏斜时不动。
- 证据：`D:/tb-eval/s2-098/version-skew/version-skew.png`（红条常驻）＋`result.json` 的 `persistent skew banner` 断言。

**336-F2 极短 run 不挂 journal 尾（P1，事实面缺失）**

- 现象：`<1 s` 的 fake 回合（无工具/后台任务场景）在 `submitPrompt` 的 1 s 跟随轮询第一次触发前就结束；`syncLiveTail` 从未运行 ⇒ 实时窗只剩 ACP 流式模型文本，用户卡、工具行、`tool_running`、终态系统行全缺。S2 `background` 场景首跑实测 `content.items` 只有 3 条、`backgroundTasks=0`，而磁盘 journal 已有 `tool_running`。
- 修法：`submitPrompt` 的成功返回与 catch 路径各补一次 `refreshExplorerData().catch(() => {})`；让 `syncLiveTail` 在 prompt 结束后仍挂一次，`journal_tail` 从 offset 0 重放全量事实。
- 证据：修后 `background` 场景 `content.items` 含用户卡/`tool:run_terminal_cmd`/系统行，`backgroundTasks≥1`；全套 S2 十条绿。

## §3 S2 套件（新增 `scripts/s2_real_machine.py`）

- **形态**：单 Python 脚本，复用仓内 `assurance/browser_retrieval.py::BrowserCDPClient` 驱动**真 Chrome**（`--headless=new`；不引 Playwright），每个场景独立工作区＋独立 `orz web` 端口＋独立 Chrome profile。
- **替身**：`orz web` 无 `--fake-provider` 参数；脚本用桥/子进程继承的 `ORZ_FAKE_SCENARIO`（审批/后台）与缺省 fake 文本（其余），等价 330 的 `--fake-provider` 口径，审计如实登记。
- **断言源**：页面 DOM／`#connBanner`／`#modalLayer`，页面内 `import('/assets/app/state.js')` 的投影状态，`/api/boot`、`/api/runs`、`/api/recall`、`/api/delivery`、`/api/context`，以及磁盘 `.gsa` 事实（run journal 终态、`.closed`/`.deleted` 墓碑、侧车）。
- **无头观察面**：headless Chrome 默认节流 rAF，`scheduleRender` 不跑；脚本对每个 tab 调 `Page.bringToFront`＋`Emulation.setFocusEmulationEnabled`，并带 `--disable-background-timer-throttling/--disable-renderer-backgrounding`。这是观察面适配，不是产品改动。
- **产物**：`D:/tb-eval/s2-098/<场景>/{result.json,server.log,*.png}`＋`results.json`／`SUMMARY.md`；每次载体重建后重跑：
  `python scripts/s2_real_machine.py --out D:/tb-eval/s2-098`

## §4 十条读数

| # | 场景 | 关键断言 | 结果 |
|---|---|---|---|
| 1 | 刷新重连 | reload 后同一 session；系统行「已重连到…（子进程保活）」；服务端日志 `reused=true`；第二轮可继续 | **PASS** 12.1 s / 8 断言 |
| 2 | 双标签 409 | 第二标签横幅「连不上本地桥…另一个 orz 窗口」；裸 WS 升级 **409**＋body `workspace_occupied` 四要素 | **PASS** 19.3 s / 8 |
| 3 | 撤回后重发 | `/rewind` 列出 `#1`；确认后原文回输入框；`/api/recall` 有 `recall_log`；重发产生新 run | **PASS** 11.6 s / 6 |
| 4 | 删除后不复活 | 关闭 → `.closed`；`DELETE /api/sessions` 200；`.deleted` 墓碑；侧车删除；旧 id prompt 得「会话不存在或已被删除（session）」四要素；新输入落新会话；侧车不复活 | **PASS** 13.1 s / 12 |
| 5 | 保活过期提示 | grace=2 s；断线 19 s 后 reaper 记 `reason="grace"`；新页系统行「保活窗口已过期…下一条输入将新建会话」；新 session id 不同 | **PASS** 34.9 s / 6 |
| 6 | 交付面板 | 工具栏「交付状态」出「交付事实」；`status=completed（完成）`、`terminal_source=short_text`；API 与面板一致 | **PASS** 10.7 s / 9 |
| 7 | 审批并发 | 真实 `browser_control script` 取得真实权限弹窗；再经本尊 `dialogs.showPermission` 送第二条，出现「上一条权限请求已被本次请求顶掉…本页累计顶掉 1 次」；run 收尾 | **PASS** 10.8 s / 5（形态偏差见 §6） |
| 8 | 上下文读数 | 「上下文」弹窗「估算占用」「chars/2」「消息数」「最近压缩」；`/api/context` `chars>0`、`estimate_method` 含 `chars/2`、`estimated_tokens>0` 且 ≤ chars/2 | **PASS** 10.9 s / 9 |
| 9 | 后台可见 | `run_terminal_cmd is_background` → `tool_running`；`backgroundTasks≥1`；属性弹窗「后台任务」「累计转入后台」「仅模型可见」 | **PASS** 11.3 s / 5 |
| 10 | 版本偏斜 | 运行中把桥 exe 换名，用 `0.9.8→0.9.9` 字节补丁探针占位；`/api/boot` `bridge_version=0.9.8`、`child_version=0.9.9`、`child_probe=ok`、`version_skew=true`；持久红条＋系统行 | **PASS** 13.5 s / 7 |

合计 **75 条断言全过**；总墙钟 **148.2 s**（含 Chrome/server 启停）。截图 **24 张**，`results.json` 含每场景 checks 与产物路径。

## §5 产物与复跑

- 读数根：`D:/tb-eval/s2-098/results.json`（`meta.binary_sha256=7cf16614…`，`meta.version=orz 0.9.8`）＋`SUMMARY.md`；同日复跑 `D:/tb-eval/s2-098-run2/`：**10/10 PASS、75 断言、143.3 s**，二进制哈希逐位同。
- 每场景：`result.json`（checks/error/seconds/artifacts）＋`server.log`＋`.png`（`final` 与场景命名图）。
- 复跑：`python scripts/s2_real_machine.py --out D:/tb-eval/s2-098`（默认跑十条；`--only refresh,delivery` 可单跑；`--keep` 保留 Chrome profile）。
- 构建日志：`D:/tb-eval/buildlogs/win_336_build.log`、`lin_336_build.log`；Linux 冒烟：`D:/tb-eval/smoke-336-lin`、`smoke-336-lin-bookworm`。

## §6 如实记与偏差

1. **审批并发的第二条请求是页面内本尊 `dialogs.showPermission` 送出的**，不是 agent 侧第二个并发请求：权限门默认 yolo，且真实工具批按顺序执行，第一条待批时第二条不会同时在途。场景先用真实 `browser_control script` 证明 agent→client 权限链路，再用同一 UI 模块验证 333 的「顶掉」代码路径；审计不把第二条写成 agent 发起。
2. **版本偏斜探针**：运行中的 Windows exe 被锁定，先改名再放字节补丁的 0.9.8→0.9.9 同名件；`/api/boot` 是真实磁盘探针读数，不是注入值。探针件在场景结束后还原。
3. **删除场景经 HTTP 维护面发起**（`DELETE /api/sessions`），与 Web 探索器「删除」同一桥路由；CLI 维护子进程不向在跑 child 发 `_orz/session_state`，所以页面不会自动出「已删除」系统行——本场景以墓碑/侧车/旧 id 拒绝/新会话四证代替，属可接受的观测面差异，登记为残余（非本批代码缺陷）。
4. **保活场景**用 `ORZ_WEB_KEEPALIVE_SECS=2`＋19 s 等待（reaper tick 15 s）；读数只证明「过期提示＋新会话」，不证明 30 min 默认窗口。
5. **S2 跑在重出的 0.9.8 上**（同版本字符串、不同哈希），不是 335 的原始字节；335 的读数仍作为历史证据保留，新哈希以本档为准。
6. **Linux 冒烟**只做 CLI/`-p`，不做 Web/CDP（容器无 Chrome；S2 真机面在 Windows 桌面）。
7. **0dn S3 未做**：`landed/shipped/verified` 词表与旧条目回填仍待；本批只把 S2 从 `pending` 推进到「十条全绿」。

## §7 台账与边界

- 本档：`docs/audits/336_0DN_S2_REAL_MACHINE_2026-10-22.md`。
- 套件：`scripts/s2_real_machine.py`（新增；父仓）。
- orz 子仓：`orz/crates/orz-web/assets/app/main.js` 两处修正；提交见 §8。
- 载体：WIN `D:/tb-eval/orz-windows`（3 entries，`7cf16614…`）；LIN `D:/tb-eval/orz-linux`（3 entries，`bfcc3f0a…`）；装机位 `C:/Users/1/AppData/Local/orz/bin` 三件已换。
- 335 档 §2 的 WIN 哈希与 §3 的 LIN 哈希已被本批取代；335 档补一行指针（历史读数不删）。
- **已推送、未做**：父仓 `main` 与 orz `cli/feat/fusion-architecture`（`c3faec70..cfed6191`）已推送，远端 ref 回读一致；`v0.9.8` tag／GitHub Release／安装包**未做**（用户令「安装包先不发行」延续）；0dn S3；Linux 评测宿主以外环境。
- 计数 **64 不变**；0dn 状态仍为 `pending`（S1＋S2 达成，S3 待续）。

## §8 提交

- orz：`cfed6191`（`fix(336): persistent version-skew banner + fast-run journal catch-up`；仅 `main.js`，+26/−5）。
- 父仓：`261e018c`（`feat(336): 0.9.8 适用版换装＋0dn S2 真机十条（10/10 PASS）`；本档＋套件＋索引/TODO/BACKLOG/335 指针＋源清单 1494 条＋子模块指针）。
- 已推送：父仓 `main`、orz `cli/feat/fusion-architecture`（`c3faec70..cfed6191`）；本批未建 tag／Release，**337 批已建 tag `v0.9.8` 并发行**。
