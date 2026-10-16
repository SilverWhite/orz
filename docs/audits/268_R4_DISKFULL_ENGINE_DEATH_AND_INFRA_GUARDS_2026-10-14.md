# 268 批：r4 磁盘满→docker 引擎自杀事故定因＋起跑器三基建守卫补丁（2026-10-14）

> **性质**：评测侧运行事故定因与起跑器补丁批（`run_r4_scb.py` 本地件；零 orz 源码、零父仓源码）。
> **会话**：主责 sess_2c2067bc（本窗）；前窗 sess_a7c66a96（06:46 后查至 07:41 会话故障中止，docker 线索由其首查）。
> **状态**：定因完成、守卫已落码、R4_PAUSE 在位；**重挂待 C 盘腾挪＋docker 复活**（见 §5）。

## §1 结论先行

**p18 file_query_tool checkpoint_2 冻结事故根因＝C 盘满导致 Docker Desktop 引擎主动自杀**，
非模型流断流、非净室泄漏、非 orz 守卫缺失：

```
[04:53:22.200] com.docker.backend.exe.engines: engine linux/wsl shutdown requested
  (cancel cause: write ...\AppData\Local\Docker\log\vm\init.log:
   There is not enough space on the disk.)
[04:53:32.609] engine stopped
```

时间线（2026-10-14，local UTC+8）：04:31:29 p18 起跑 → 04:45 checkpoint_1 收官（55 轮 13.6min
全 1.0，核收全过）→ 04:46:24 checkpoint_2 起新档（session `c235b71d`，0cy 收卷行正确扫上一档
`2891b57a`）→ 04:52:16 容器仍活跃请求 api.deepseek.com（monitor.log gvisor 转发记录）→
**04:53:22.200 引擎停止请求（磁盘满）→ 04:53:22.688 orz 最后一行日志（随 VM 冻结）→
04:53:32 engine stopped → 04:53:33 UI 弹「Docker Desktop is unable to start」** → harbor
`docker exec` 读流悬挂 **113 分钟零输出** → 06:46:24 撞 7200s 档帽 `AgentTimeoutError` 击杀 →
checkpoint_2 无 step_result.json／无 gsa copy／verifier 无分 → 起跑器核收判
`sessions_new=False＋/tmp 面 null` → **净室硬停门自停**（窗口 7/8 核收过，余额 308.69 → 265.60）。

C 盘现状：**0 字节可用**（124.1GB 用满）；D 盘 13.4GB；B 盘 18.4GB。docker-desktop WSL 发行版
vhdx 在 `D:\DockerData\DockerDesktopWSL`（镜像不占 C）；pagefile.sys 12GB 在 C。

## §2 定因证据链（逐件）

1. **docker backend 日志**（`AppData/Local/Docker/log/host/com.docker.backend.exe.log`）：
   停止原因行（§1 引文）＋ `State:stopping`＋`IsPaused:false`——引擎是被**写日志失败触发的主动
   关闭**，非 Resource Saver 误停、非外部 kill。
2. **orz 侧日志**（`r4-p18…/steps/checkpoint_2/agent/orz-acp.stderr.log`，5.6MB）：最后一行
   h2 帧 04:53:22.688——引擎停止请求后 0.5s 仍在正常写日志，随后与 VM 一起冻结；wire 日志停在
   半句 `agent_message_chunk`，无 end_turn。**容器冻结实锤：任何容器内守卫（RLI 181s 停滞采样、
   TER idle-kill、output-health guard）随 VM 一起死亡，结构性不可能触发。**
3. **harbor 侧**（result.json step_results[1]）：`AgentTimeoutError: Agent execution timed out
   after 7200.0 seconds`，occurred_at 04:46:24＋7200s＝06:46:24 整；verifier 0.5s 空转无分。
   悬挂点＝`docker.py:_collect_buffered_output → asyncio stream read`，引擎死后 exec 管道无
   错误无数据，harbor 无宿主侧停滞看门狗，只能等档帽。
4. **余额曲线**（r4_balance_log.jsonl）：04:47 后消耗归零（266.18 → 265.6 后恒 265.6 至收官）——
   与容器冻结同步，佐证模型调用已死（非模型侧断流后重试失败形态）。
5. **前窗独立结论互证**：sess_a7c66a96 07:39 已判「checkpoint_2 时 Docker Desktop 挂了＋
   daemon 不可达」，与本窗 backend 日志定因一致。

## §3 守卫缺失裁决（用户令「orz 侧守卫缺失则记录并修复」）

- **orz 侧：无缺失，不立项**。四类守卫（RLI 看门狗／TER idle-kill／output-health／压缩面）全部
  容器内进程内实现，宿主 VM 死亡时随进程冻结，**该失效模式在容器内不可观测也不可救**——防线
  正确位置在宿主侧（见 §4 补丁）。旁注：健康容器内纯模型流停滞（provider 静默不断连）形态下，
  orz 按设计无流级超时（P9 禁外部固定触发规则；重试只对错误触发），退路即评测 7200s 档帽；
  如需收紧属设计裁决非缺陷，本批不动。
- **宿主侧（起跑器）：三处缺失，已全部补齐**（§4）。
- **净室硬停门本身：行为正确**——p18 核收失败确实是异常信号（档位残缺），门停下窗口止损正确；
  本质是「异常信号的正确上报」，定因后按 errored 同级（基建失败）处置即可。

## §4 起跑器补丁（`run_r4_scb.py`，已落码并实测）

1. **放题前磁盘水位门**（`disk_free_gb`／`infra_block_reason`）：C: ≥8GB、SCB 所在盘 ≥5GB，
   低水位不放新题。
2. **放题前 docker daemon 门**（`docker_alive`）：`docker version` 20s 探测，不可达不放新题。
3. **可自愈类挂等语义**（`wait_for_infra`，主循环 `wait_for_egress` 后接入）：磁盘/daemon 拦截
   **挂等恢复（5min 重探、2h 级日志）而非停窗**，R4_PAUSE 随时可中止——避免本晨险情（daemon
   死时逐题快败烧成 errored 终态连锁跳过）重演。
4. **档内静默看门狗**（`run_harbor` Popen 化）：job 目录全体文件 mtime 静默超
   `R4_STALL_KILL_SEC`（默认 1800s）即 `taskkill /F /T` 击杀 harbor 进程树，**补写 errored
   形态 result.json 终态标记**（无标记残留会让重挂在「未完成残留拒绝覆盖」卡死），本题落
   errored 待人工——基建失败不牵连全轮，不触发净室门。113min 冻结损失自此封顶 ≈30min。
5. 实测：py_compile 过；`infra_block_reason()` 现值如实报「C: 0.0GB＜8GB；docker daemon 不可达」。

## §5 用户裁决落账（2026-10-14 08:4x，三条全落）

1. **放题节奏**：晨间不放题，等下一低谷窗（12:00 午窗）；起跑器保持挂起等待（PID 21052），
   放题前基建门自检磁盘/daemon——C 盘清理由用户中午回来自行处理，届时达标即自动放题。
2. **D 盘事故备份删除**（用户明批）：`docker_data.vhdx.corrupt-bak-20261002`（18.3GB，10-02
   遗留）已删，**D: 13.4 → 30.44GB**。
3. **p18 整题重跑（选项 b）＋双口径记账**：harbor 侧事故件**全保留**——job 目录整体转存
   `jobs-r4/r4-p18-file_query_tool__incident_20261014/`（result.json/steps/hub b3e841f4 原位），
   正式名终态已清，起跑器下次放题时 p18 按未跑整题重跑（台账 append，事故行凭 finished_at
   10-14T06:46 与重跑行区分）。**算总成绩分两口（口径命名 10-16 用户定：①＝硬标准口径，②＝标准口径）**：①**硬标准口径（残缺；用户 08:5x 定义）＝
   checkpoint_1 按实分记、其余检查点一律记 0**——p18 数据集实为 **5 档**（attempt-3 完整跑
   实证：1.0/1.0/1.0/1.0/0.875），故 (1.0＋0×4)/5＝**0.2**（初算 0.5 系误以事故跑所见
   2 档为全量——档数为任务阶段动态推进非定值，10-16 用户指正勘定，本节与 §12 已随之勘正）；
   ②**标准口径（补跑）**＝p18 清终态整题重跑的结果（落账后另行统计）。

**处置流水补记**：C 盘安全清理（Docker 诊断 548MB＋DockerDesktopUpdates 357MB＋wsl-crashes
31MB＋pip 70MB）0 → 3.68GB，Docker Desktop 重启后部分缓存自愈回写，稳态 ≈2.3GB；daemon
29.6.2 复活（docker-desktop 发行版 Running）；`docker system df` 实测镜像不按题累积（18 题
仅 3 个 980MB 题面镜像残留），prune 清收死容器 3.25GB＋悬空镜像 2.1GB（vhdx 高水位 27GB
内部余量）；p18 尸体容器证据面均已在宿主 agent 目录，尸体 prune 清收；WSL swap 迁 D 已写入
.wslconfig（下次 WSL 重启生效，回收 %TEMP% 1.7GB）。C 盘完整账：Windows 19G＋pagefile 12G＋
Program Files/ProgramData（已装软件）＋用户可清项全部 ≈15G 内小头——**无可清大件，结构性
腾挪＝用户中午共同清理面**（pagefile 迁移／卸载等由用户定）。

## §6 当前成绩读数（硬标准口径〔残缺，用户定义版〕；**2026-10-16 勘误：本节为 10-14 结算快照，
其硬标准口径数值按误认 2 档计（p18＝0.5）——勘定值 p18＝0.2，全轮总账见 §12**）

- **进度**：18/36 题完成（p01–p18 全 completed rc=0），84 档入池（83 有分＋p18/ckpt2 记 0）；
  墙钟合计 28.0h，花费 95.63 CNY（均价 5.31/题）。
- **r4 自身**：题级 core_mean 均值 **0.8631**（满分题 7/18：cfgpipe／circuit_eval／env_manager／
  etl_pipeline／eve_route_planner／execution_server／file_backup）；档级 core 均值 **0.8914**、
  strict 0.8554、isolated 0.8509；满分档 61/84（72.6%）；零分档 2 个（eve_industry/checkpoint_5
  推理退化判停＋p18/checkpoint_2 缺档记 0）。摩擦面统计仍按 83 有分档池（缺档无摩擦数据，
  不造 0）：erosion 上升档 41/83（49%）均值 0.548；verbosity 上升档 36/83（43%）均值 0.368；
  档轮次均值 68。
  **勘误注（10-16）**：本节硬标准口径档池同样按 2 档误设（缺 p18/ckpt3–5 四个 0）——勘定后
  18 题段硬标准口径题级应为 **0.8464**（0.5→0.2，差 0.3/18）、档级 **0.8606**（87 档＝82 有分
  ＋p18 五档[1.0＋0×4]）。历史快照保留原值不入总账面，全轮总账见 §12。
- **对 0cw 同 18 题**：5 升 7 平 6 降；题均 r4 **0.8631** vs 0cw 0.8779（−0.0148，主要即 p18
  缺档记 0 所致）；升幅最大 env_manager（+0.350）、eve_route_planner（+0.333）；降幅最大
  p18（−0.500，事故缺档）、dynamic_buffer（−0.206）、eve_industry／eve_jump_planner（各
  −0.167，k=1 单试噪声域）。
- **标准口径（补跑）读数（2026-10-14 19:23 attempt-3 收官落账）**：p18 整题重跑 completed rc=0，
  **5 档全跑**（1.0/1.0/1.0/1.0/0.875），wall 79.1min，core_mean **0.975**，重置核收全过＋
  /tmp 面全过，hub=6a50a2e9；**标准口径题均 0.8895、档级 0.9052（87 档）**——回到 0cw
  （0.8779）之上，对 0cw 仍 5 升 7 平 6 降（p18 −0.025）；attempt-3 erosion 上升档 2。
  两口径并行的最终总账待 36 题收官时按本节口径定义统一结算。

## §7 偶发退化判停（eve_industry/checkpoint_5）：裁决、叙述修正与成因调查

**用户裁决（2026-10-14）**：守卫实现**没有问题，维持现状**；只改相关叙述与注释。理由＝
停滞本身无法确认重跑能否**确定性**跳出复读循环，自动重试只会空转空烧；恢复路径＝**人工介入
整体性判断**（不设自动重试是既定设计而非缺陷）。

**叙述修正（已落码，随下代载体生效；在役 0.9.0 二进制不动——代际门纪律）**：
- `agent_loop.rs` WARN「interrupted the model round (audit-only)」→「(round-terminal; no
  auto-retry by design — recovery is human adjudication)」，并补注释记载裁决与理由；
- `transport.rs` trip 文案「(audit context)」→「(trigger context for post-hoc review)」；
- 命中面（1–19 次「audit only, not tripping」）叙述**不动**——那部分本来就是纯审计、正确。

**现场证据（封存于 `jobs-r4/r4-p11-eve_industry/eve_industry__FdrdYjv/steps/checkpoint_5/agent/`）**：
单轮内退化——20:18:38 起跑、仅一次 prompt（6687 字符）、模型在第一轮 reasoning 通道对
`load_plan_config` 代码段 400 字符循环复读约 9 分钟，20:27:49 guard 滚动哈希 20/20 击发 →
ACP 回 Internal error → driver rc=1 → 该档 rounds=0／core 0.0（strict 0.863 系 verifier 分组
数据）；下一档 checkpoint_6 全新会话 131 轮 core 1.0＝瞬时事件。

**成因调查（用户提出 deepseek flash 1013 灰度/偶发降智假说）**：
- **机制成立**：`deepseek-flash` 是官方 API 滚动别名（[更新日志](https://api-docs.deepseek.com/zh-cn/updates)：
  2026-09-10 起「模型名改为 deepseek-flash 即可调用最新 V4.1 Flash」）——灰度轮换在该别名下
  无感发生；社区长期以思维链风格变化侦测灰度并报告偶发降智（[灰度侦测帖](https://linux.do/t/topic/2418802)、
  [正式版灰测讨论](https://linux.do/t/topic/2588431)、[指令遵循下降报告](https://linux.do/t/topic/2313174)）；
  「长上下文输出无限重复、不触发 EOS」是 DeepSeek 系已档案的缺陷类
  （[DeepSeek-V3 #1550](https://github.com/deepseek-ai/DeepSeek-V3/issues/1550)）。
- **官方更新日志 2026-10-10～15 无任何条目**——1013 灰度若存在即属不宣告轮换，无法从官方面证实。
- **本地量化证据（全轮 repetition 命中分布）**：命中是**跨三天全程背景噪声**（p01/p03/p05/p06/
  p07/p08/p09/p11/p12/p18 十余档均有，p06/ckpt5 曾 88 次命中未击发）；**trip 全轮仅 ckpt5 一例**；
  紧邻其后的 p12（20:58–22:46）命中 37 次＝基线水平，**无围绕 10-13 晚间的聚集峰**。
- **结论（用户裁决 2026-10-14，两级落定）**：①**假说降级为猜测**——1013 灰度/偶发降智
  **无法证明**：官方日志无条目、用户未订阅 DeepSeek 通知无故障邮件可查，仅作猜测记录不入
  结论面；②**可接受波动定调**：模型偶发复读/降智这类波动**应被接受为环境事实**，无论源头
  是否模型侧问题——守卫职责＝识别＋截停＋留痕＋供人工整体判断（实现无误、维持现状），不承担
  「治愈」波动。守卫叙述修正已落码（`agent_loop.rs`／`transport.rs`，随下代载体生效）；设计稿
  勘旧与裁决落档见
  [`DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md` §3.3 勘旧注＋§8](../DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)。
  判别实验（可选深挖，非必做）：ckpt5 原 prompt 多时段重放测非确定性＋命中密度时间序列对齐
  灰度报告窗口。深挖入口＝本节证据路径。

## §8 二次事故（同日 13:25）：起跑器进程静默死亡 → p18 补跑孤儿化 → 监督器防线

- **形态**：runner python（PID 17776）与其子 harbor 在 13:25 前后被终止。**成因已破案（用户
  提供关键线索「当时在隔壁窗口用 codex 刷机」）**：codex 会话日志（logs_2.sqlite）
  **13:22:43 执行 `Stop-Process -Id 8736,2444,17776,23004 -Force`**——刷机工作流清理残留
  python 进程时按 PID 精确点名，17776（隐藏窗口的脱离 runner）被当作陈年进程误杀；
  此前 12:55–13:17 的 `Stop-Process -Name fastboot` 等为刷机正常操作与本轮无关。
  非 04:53 磁盘满事故的延续，亦非系统不稳定——两例根因各自独立。
- **损失盘点（p18 补跑 attempt-2）**：checkpoint_1–4 全部干净完成（end_turn，30/72/57/103
  轮，core 全 1.0，strict 0.90–1.00），死于 checkpoint_5 起跑前；harbor 未收官 ⇒ 无 hub
  上传、无台账行，官方意义上该次不存在；耗 ¥6.02（265.6→259.58）沉没。注：本题档位数
  多于事故首跑所见（首跑 2 档、本跑 5 档未完）——档数为任务阶段动态推进，非定值。
- **处置**：孤儿容器 `docker rm -f`；残件整体转存
  `jobs-r4/r4-p18-file_query_tool__rerun2-orphaned_20261014/`（四档 reward/step_result 原位
  保留）；正式名清空，**attempt-3 于 18:00 晚窗照常进行**（标准口径取 attempt-3 结果）。
- **防线补强（已落码在役）**：新增监督器 `run_r4_supervised.py`（挂起形态替代直挂）——
  runner 异常退出（exit≠0）时自动归档在途未完成残件（衔接「未完成残留拒绝覆盖」语义）
  后重启（60s 退避、上限 5 次防循环）；正常退出（窗口收官/题单完成/R4_PAUSE）不重启。
  14:54 起监督器 PID 2612 ＋ runner PID 23172 双进程在役，等待 18:00 晚窗。
  **残余风险（如实）**：监督器对「点名杀 runner」自愈有效；若刷机侧再次执行宽匹配
  「杀全部 python」，监督器与 runner 会同死——届时需人工重挂（晚间收工核查一次即可）。
  运维约定：codex 刷机窗口与 r4 跑批共存期间，进程清理请避开 python 进程族或先置
  R4_PAUSE。
  **2026-10-15 修订（首版缺陷实损落账）**：首版监督器「rc=0 即随退」——10-15 08:21 停放门
  正常收官后监督器同退，**12:00 午窗整窗空转**（10 题 × 1–2 题/窗的机会损失）。修订语义：
  rc=0 后查 R4_PAUSE／stop_reason——暂停权或「本轮题单跑完」才随退，否则 60s 重挂
  （runner 自会窗口等待）。18:20 修订版挂起，18:24 p27 mvvault 照常放题。教训：监督器
  的「正常退出」判定必须区分「窗口收官」与「题单完成」，收官≠完成。

## §9 补记（10-15）：DeepSeek 官方故障通报、赔偿到账与成本口径修正

- **官方通报**（用户提供）：DeepSeek API 2026-10-15 **08:11–13:04**（北京时间）服务故障；
  赔偿＝**10.8–10.14 消耗全额返还**，以 credit 自动入账、后续扣费先走 credit。
- **余额异动定因**：204.13 → 668.83 的 **+464.70 = 赔偿 credit**（＝本账户 10.8–10.14 消耗，
  覆盖 r4 全部 10-14 前开销——含 p18 事故首跑与两次补跑沉没；**r4 截至 10-14 的净成本归零**）。
  此前「用户充值」误读更正。p27 起消耗先扣 credit。
- **p26 与故障窗关系（核验）**：收官 08:21（故障开始后 10 分钟），8 档全部 end_turn；末档
  checkpoint_8 于 08:10:02 出现一次 midstream 解码错误（故障宣告前 1 分钟），retry 1/1 即愈，
  无退化无连环失败——0.834 成绩按官方形态采信有效。
- **关联注记（不扩大）**：10-13 13:22 的 eve_industry/checkpoint_5 复读判停**不在**本次官方
  故障通报窗内，§7 的「灰度/降智＝不可证猜测」裁决不变；DeepSeek 服务侧不稳定性获得官方
  侧旁证，但两时段互不隶属。
- **监督器午窗空转教训**与 p27 重挂详见 §8 2026-10-15 修订段（18:24 p27 mvvault 已起跑）。

## §10 备案（10-16 用户令）：尾两题峰窗直放一次性覆写

- **裁决**：p35 trajectory_api／p36 xjq 于 10-16 **10:00 直放**（借上午峰＋午谷收官，下午高峰
  留作他用）——对峰窗/停放门纪律的一次性用户覆写，仅本挂生效。
- **实现**：`run_r4_scb.py` 增 `R4_FORCE_RELEASE=1` env 覆写（`in_offpeak_window` 直返 True，
  横幅明示「用户令一次性」，不进常驻纪律）；监督器经 `launch_r4_tail.ps1` 定时引爆
  （10:00:01 挂起，横幅核验覆写在役）。
- **执行**：10:06:07 p35 起跑（余额 616.44）；p36 随 p35 收官接力，全目收官预计当日午后。

## §12 全轮收官总账（2026-10-16 12:43，r4＝36/36 完成）

- **完成**：36/36 全 completed rc=0、逐题 hub 上传（37 个 hub job＝36 终态＋p18 事故件
  b3e841f4 并存可追溯）；全部题的重置核收与 /tmp 面断言通过（唯 p18 事故跑例外，已双口径
  处置）；零 errored。
- **双口径最终读数（硬标准口径已按 5 档事实勘定，10-16；档级＝36 题最终 196 档全池）**：
  | 口径 | 题级均值 | 满分题 | 档级均值 | 对 0cw 全轮 0.8235 |
  |---|---|---|---|---|
  | 硬标准口径（残缺；p18＝0.2＝[1.0＋0×4]/5） | **0.8631** | 16/36 | 0.8770（196 档，满分档 141） | 12 升 15 平 9 降 |
  | 标准口径（补跑；p18＝0.975） | **0.8846** | 16/36 | 0.8968（196 档，满分档 144） | 12 升 15 平 9 降 |
  - r4 对 0cw 全轮 **+0.0611**（标准口径），两口径均显著领先；
  - 勘误链：硬标准口径初算 0.5（误以事故跑所见 2 档为全量）→ 36 题总账首版 0.8714 →
    10-16 用户指正「检查点不止两个」→ 勘定 0.2／题均 0.8631（§5 定义勘定、§6 快照勘误注）。
- **成本**：台账累计消耗 **¥217.04**（含 p18 事故跑 ¥1.35＋attempt-3 补跑 ¥5.16 两行；
  attempt-2 孤儿 ¥6.02 无台账行、不计入本数，事件链合计另见成绩分析 §6＝¥12.53），累计墙钟 58.6h；
  其中 10-14 前开销 ¥464.70 已由 DeepSeek 赔偿 credit 全额返还（§9），10-15/16 消耗走 credit
  扣减，期末余额 602.19。
- **事故与防线回顾（三起三修）**：①04:53 磁盘满→docker 引擎自杀（根因＝C 盘满；根治＝swap
  迁 D＋pagefile 迁 D＋起跑器基建三守卫）；②13:22 codex 点名误杀 runner（根因＝刷机进程
  清理；防线＝监督器清残自愈＋运维约定）；③监督器首版「收官即随退」致午窗空转（修订＝
  区分收官/完成，10-16 实战验证随退语义）＋runner STATUS 完成语义修订（监督器识别依赖）。
- **官方记分口径**：以 Harbor hub 36 个 public job 为准（p18 按 attempt-3 job 6a50a2e9；
  事故 job b3e841f4 保留作证据面）。
- **批档收尾待办**：CLI_PROJECT_INDEX／BACKLOG／TODO 同步随 268 批关闭一并做（含 0dc S3
  r4 全跑完成态登记）。

## §13 附：本窗前情（重挂与急停，如实入档）

07:55:33 本窗曾按「续跑＝重挂同命令」重挂（PID 15692，代际门三件逐位过、低谷窗 07:55 开启、
逐题跳过推进中）；07:39 前窗留档「daemon 不可达」线索浮出后，07:58:45 **急停击杀**——末行日志
停在 [18/36] 跳过，p19 未放题、无目录残留、无台账污染；同刻落 R4_PAUSE。急停时点在 p19 放题
前 ≈10s，无损失。该重挂所用脚本无 §4 守卫（补丁在其后落码），重挂待 §5 1–3 完成后以新脚本进行。
