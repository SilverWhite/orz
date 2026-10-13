# 251 批 第三轮 SCBench 官方口径轮首题——cfgpipe 全题 6 档（2026-10-11）

> **批次**：251（前档＝[`250 批档`](250_0CY_S4_PROBE_RESET_EQUIVALENCE_2026-10-11.md)）。
> **日期**：2026-10-11。**用户令**：「直接起第三轮真正官方口径的sc bench吧，依旧按照官方口径来并
> 使用harbor run，先跑第一题看看，每个checkpoint都额外检查一下重置是否严格符合官方口径，这一轮做轮询」。
> **性质**：零源码零改动——真机评测轮（在役载体 0.9.0；评测侧本地起跑器 `run_r3_scb.py`，不入父仓/orz 仓）。
> **结论先行**：第一题 `cfgpipe`（官方字母序 p01）**全 6 checkpoint 完成、core 全 1.0**
> （core_mean 1.0；strict 0.892–0.952）；**逐 checkpoint 重置严格性核收全过
> （reset_conformance=true）**——会话 6/6 全新、journal 6/6 全新、ck2–ck6 起点 0cy 收卷行
> `swept=[上一档 s8]` 逐档恰等且 `errors=[]`；作业已 `--upload --public` 入榜
>（hub `6ed74c19-8263-4dd7-8f3b-8579e3d27150`）；余额 221.42 → 215.11 CNY（全题 6.31 CNY）。

---

## §1 口径与代际门

- 沿 0cw S3 官方形态（[`226/227/228`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md) 批定案）：k=1／零重试／每 checkpoint 全新 `orz --stdio --real` 进程＋全新 ACP 会话（agent 默认，不传 continuous）／官方 7200s 档帽零覆盖／逐题 `--upload --public`／低谷窗＋晨间缓冲停放门／余额守卫 30 CNY／暂停权 `R3_PAUSE`。数据集 pin 不变（`gabeorlanski/slopcodebench@aec29354…`）、模型 `deepseek-flash`。
- **代际门（本户新增硬校验）**：起跑前对三件套逐个 SHA256 对 248 批真值——`orz`＝`1ce40857…`（身份门）／`orz-signer`＝`7cf5dbe2…`／`orz-acaf-provision`＝`2ee04153…`，不符即拒绝起跑；容器内 install 自检回读 `build-info version=0.9.0 os=linux`。
- **逐 checkpoint 重置严格性核收（本户新增）**：每档采集 ① `session_id` 全新（step_result.json）；② journal run 全新（`RUN-<s8>-0` 目录名）；③ orz stderr 的 0cy 收卷行（ACP 车道 tracing 格式 `gsa session hygiene sweep swept=[…] shared_moved_to=… errors=[]`），ck≥2 断言 swept ∋ 上一档 journal s8——250 批探针结论（容器重置≡会话重置≡台账重置）在评测容器内的逐档实证。

## §2 判分读数（6/6 档）

| ck | 会话（s8） | 轮数 | 用时 | core | strict | isolated | erosion | verbosity |
|---|---|---|---|---|---|---|---|---|
| 1 | `d9fbb5fa` | 25 | 392.8s | **1.0** | 0.892 | 0.892 | 0.244 | 0.370 |
| 2 | `e2290011` | 41 | 654.6s | **1.0** | 0.912 | 0.935 | 0.390 | 0.423 |
| 3 | `4f6e3536` | 40 | 543.0s | **1.0** | 0.944 | 1.000 | 0.393 | 0.352 |
| 4 | `b319daab` | 53 | 1077.7s | **1.0** | 0.942 | 0.933 | 0.561 | 0.549 |
| 5 | `41fbeac6` | 48 | 481.8s | **1.0** | 0.952 | 0.980 | 0.571 | 0.549 |
| 6 | `692abf7a` | 76 | 661.2s | **1.0** | 0.940 | 0.862 | 0.592 | 0.543 |

- 全档 `stop_reason=end_turn`（无超时／无看门狗击杀／零 request_permission——yolo 面正常）；作业墙钟 4277s（10:36:04→11:47:21，≈71 min；0cw 同题 5002s）。
- 对照：0cw 轮同题（后被污染更正、成绩作废）6 档 45/48/56/73/66/55 轮、core 全 1.0——本轮轮数更省（25–76）。erosion 轴 0.24→0.59、verbosity 0.35→0.55 随档缓升（`erosion_increase_rate`＝1.0），core/strict/isolated 无退化形态；该轴的跨题解读留全目收官后的轮报告，不在此题预判。
- 花销：全题 6.31 CNY（0.75 CNY 另计＝§4①中止轮）。

## §3 重置严格性核收（reset_conformance＝true）

| ck | session 全新 | journal 全新 | 起点 0cy 收卷行 | 断言 |
|---|---|---|---|---|
| 1 | ✓ `d9fbb5fa` | ✓ RUN-d9fbb5fa-0 | 无（净容器，正确） | — |
| 2 | ✓ `e2290011` | ✓ RUN-e2290011-0 | `swept=["d9fbb5fa"]` shared→d9fbb5fa errors=[] | ✓ 恰等 ck1 |
| 3 | ✓ `4f6e3536` | ✓ RUN-4f6e3536-0 | `swept=["e2290011"]` | ✓ 恰等 ck2 |
| 4 | ✓ `b319daab` | ✓ RUN-b319daab-0 | `swept=["4f6e3536"]` | ✓ 恰等 ck3 |
| 5 | ✓ `41fbeac6` | ✓ RUN-41fbeac6-0 | `swept=["b319daab"]` | ✓ 恰等 ck4 |
| 6 | ✓ `692abf7a` | ✓ RUN-692abf7a-0 | `swept=["41fbeac6"]` | ✓ 恰等 ck5 |

每档起点收卷集合**恰为一个**会话＝上一档；容器卷跨档存续（harbor 所有）而会话足迹不跨档——官方「agent session data reset between checkpoints」在本载体上由 0cy 启动侧收卷机械达成。行集落 `jobs-r3/r3_reset_ledger.jsonl`（6 行）；题级行 `r3_problem_ledger.jsonl`；档级行 `r3_checkpoint_ledger.jsonl`。

## §4 摩擦与处置（如实记）

1. **首起外壳被回收**：10:19 首次起跑用 ZCode 会话内后台壳承载，10:35 壳被回收（exit 127），harbor 孤儿化但容器存活续跑——立即 `docker rm -f` 止血（中止轮耗 0.75 CNY、无结果无上传），残留 job 目录归档 `jobs-r3/.aborted-20261011T1050-cfgpipe/`；改用 PowerShell `Start-Process` **脱离会话独立进程**重启后全程稳定。**教训登记：长时评测跑批一律脱离会话进程承载，输出重定向到文件。**
2. **收卷行双格式**：0cy 收卷行在 CLI 车道为 eprintln 格式、ACP 车道为 tracing 格式（带 ANSI 色码、字段名 `shared_moved_to`）——起跑器首版正则只认 CLI 格式，已修为 ANSI 剥离＋双格式兼容（250 批探针未覆盖 ACP 车道格式，本轮实测补齐）。
3. **起跑器收集函数一处 `NameError`（`jd` 未定义）**：harbor 正常收官后起跑器在题级收集时崩溃（异常入未重定向 void，收官行 0 题）——**原始证据零损失**（per-step 目录、stderr、journal、reward 全在盘面），修正后用独立后处理重建核收台账并回填题级行；checkpoint 台账 6 行在崩前已落盘。同轮去重（核收跑两遍）并修正 wallclock 4477→4277s。
4. harbor 每 step 收官会清空容器日志同步目录（活 stderr 截断、per-step 副本落 `steps/checkpoint_N/agent/`）——活面读数须以 per-step 副本为准（§3 行集即取自副本）。

## §5 边界与下一步

1. 本批只跑第一题（用户令「先跑第一题看看」）；**余 35 题未跑**，是否全目续跑待用户裁决（起跑器 `--problems N` 支持续跑：已落终态题自动跳过，未完成残留拒绝覆盖）。
2. 逐题 `--upload --public`＝事实入榜通道已走通（hub 计数 +1）；榜单聚合面（官方 leaderboard 快照留证）留全目轮报告。
3. 0cw 轮冻结结论不变（污染更正在案）；本轮为 0.9.0＋0cy 面的**第三轮官方口径轮**，全目成绩判定留轮报告（V3）。
4. 零提交零推送；评测侧本地件（`run_r3_scb.py`、jobs-r3/）不入父仓不入 orz 仓。

## §6 关联与关键词

[`250 批档`](250_0CY_S4_PROBE_RESET_EQUIVALENCE_2026-10-11.md)（探针前置）／
[`235 批档`](235_0CW_CONTAMINATION_AUDIT_0CY_GSA_HYGIENE_2026-10-11.md)（0cw 污染审计＋0cy 立项）／
[`226 批档`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md)（官方重置口径勘定）／
[`248 批档`](248_SUBMIT_PUSH_AND_RELEASE_V090_2026-10-11.md)（0.9.0 载体发行）／
[`0cy 设计稿`](../GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md)。

关键词：251 批、第三轮 SCBench、官方口径、harbor run、cfgpipe、全 6 档 core 1.0、逐 checkpoint
重置严格性核收、reset_conformance true、0cy 收卷行逐档恰等、代际门三哈希、hub 入榜、
余额 6.31 CNY、计数 59 不变、索引 v4.223。
