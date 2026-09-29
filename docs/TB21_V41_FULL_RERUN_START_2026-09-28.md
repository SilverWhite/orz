# TB 2.1 V4.1 整轮重跑起跑记录（0.8.4 载体，逐题串行；2026-09-28）

> 类型：轮次起跑记录（口径、身份、装置形态；结果与摩擦读数在本文滚动追加或另立收尾档）。
> 触发：2026-09-28 用户令——无合适狗粮轮题面，**直接进跑分**；**每道题单跑（串行）**；
> **尽可能留出宿主机资源，避免宿主机资源过少限制跑分**；提交与推送冻结（工作树改动不提交）。
> 同日用户令补充确认：**本轮一定按照官方口径跑，结果上传 Harbor**。
> 关联：[`排期与起跑前置`](TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md) §3.2 追加裁决
> （C6 撤回：整轮重跑 89 题，第 0 轮账面只作摩擦证据）／[`第 0 轮起跑记录`](audits/TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md)。

## 1. 口径（本轮裁定）

| 项 | 值 | 来源 |
|---|---|---|
| Harness / 数据集 | 官方 TB 2.1，pin `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a`（冻结清单逐题 sha256 同源） | 排期 §3、P2-15 |
| 模型 | `deepseek-v4-flash`（官方跑批固定名；代际靠时间戳＋事后分桶复扫核，不做前置阻断） | 排期 §2 |
| 试次 / 并发 | **k=1、每题一作业、`-n 1` 串行**（2026-09-28 用户令「每道题每道题单跑」） | 本档 |
| 墙钟 | 每题**官方 agent 超时**经 `--ak max_wallclock` 透传（2026-09-02 口径收口：官方值＝唯一评测墙钟；orz 优雅收尾避免超时孤儿） | FR-D02／第 0 轮执行器 |
| 上传 | `--upload --public`（Harbor，逐题作业） | 排期 §3＋本档用户令 |
| 定性 | k=1 筛查轮（不作榜单成绩；榜单需 ≥5 试次/题） | 排期 §3 |
| 载体 | orz **0.8.4**（源冻结 orz `9f12ecc2`，116 批双平台重建；Windows 本地换装位＋Linux 三件套 = `D:/tb-eval/orz-linux`） | 116 批档 |
| 范围 | **89 题全量重跑**（C6 撤回）；题序＝冻结 runner `run_official_2.1.sh` 官方批次划分 B1–B5（16/17/19/18/19） | 排期 §3.2 |

## 2. 代际身份（起跑时锁定值，逐题作业自动核验，不符即中止）

| 件 | SHA256 | 备注 |
|---|---|---|
| `orz-linux/orz` | `87941130b37da51e585d216df55c4e84e62d2571e448470bae20c86b5a01745e` | 0.8.4，116 批表逐位吻合（已实测核验） |
| `orz-linux/orz-signer` | `b1f639576aaa8253db1c51949027c843c5e056f911befa5a748cb63f2ea1fd94` | 同上 |
| `orz-linux/orz-acaf-provision` | `5bb027c4c88a6d3f128413b4f0ef28be7da95ff07a45e15132b3b9e608d1c27b` | 同上 |
| `tb_agents/orz.py` | `6d55c26ec9415d579961371973504f0d44de8027fe8318bbd8a0e835811b0d17` | 现行适配器（含 F1 试次隔离 `fdd161d4` 之后又并入 0ax `ORZ_WEB_SEARCH_LOCAL` 透传） |

执行器身份门：`scripts/run_r0_heavy_official.py` 的 `EXPECTED_CARRIER_SHA256` /
`EXPECTED_ADAPTER_SHA256` 已于本轮换装为上表锁定值（旧值 0.5.0 `393eee34…` /
适配器 `2737cfad…` 留注释备查）；`--allow-identity-drift` 未使用。

## 3. 起跑前旗标对账（`TB21_RUN_FLAG_CHECKLIST.md` 逐项勾选）

- [x] 已读清单，并对照现行适配器/启动脚本。
- [x] 本次实际生效的旗标组合：工具轮上限＝适配器 `_DEFAULT_MAX_TOOL_ROUNDS=999` 经
  `--max-tool-rounds` 传递（实际生效值待首题 journal 读数回填，勿以 999 假定）；
  检索面＝`ORZ_WEB_SEARCH_LOCAL=on`（`.env` 随 `--env-file` 进 agent 容器；本地分段 HTTP
  兜底车道，0bs ⑩ 三引擎小集）；浏览器评测＝**未开**（`eval_browser` 未传，容器内无浏览器
  为设计内边界，0ac 判据）；另有 `ORZ_TOOL_TIMEOUT_SECS=900`、`ORZ_STALL_TIMEOUT=360`
  为在役 `.env` 既有项（历轮官方口径同值，随 `.env` 透传）。
- [x] 出现「工具轮早停 / 检索面空 / 浏览器缺件」类读数时先按清单核对，不直接判镜像缺陷。

## 4. 装置与执行形态

- **执行器**：`scripts/run_official_v41_full.py`（逐题串行驱动器，本轮新增，未提交）——
  解析冻结 runner 的 BATCH1..BATCH5 权威题序 → 逐题调用
  `run_r0_heavy_official.py --tasks <题> --job-name official-v41-b<批>-<序>-<题名>`；
  身份门 → 预拉该题镜像（4 重试、digest 落 `preroll-images.log`）→
  `-k 1 -n 1 --upload --public -y` → 跑完 `docker rmi`（仅当后续题不共用该引用）。
- **断点续跑**：作业目录已有 `result.json` 即跳过；残目录（无 result）移入
  `jobs-official/_incomplete/` 取证保留后重跑；单题失败 90 s 后重试一次；
  整轮结束对失败清单自动补跑一遍（pass2）。
- **磁盘纪律**：峰值占用压到单题镜像级；宿主 D: 起跑余量 38.5 GB（≥排期 25 GB 线）。
- **宿主机资源纪律（2026-09-28 用户令）**：跑批窗口内不做构建/测试/其他重活；
  驱动器自身只做子进程等待。
- **窗口边界与监督形态（2026-09-28 二／三次用户令）**：本轮**只跑到 2026-09-29 09:00**
  （DeepSeek 计费高峰期不跑）——**不做硬截止准入**：整题整题跑（在跑的题按官方墙钟
  自然跑完，不题中截杀），到 09:00 后不起跑新题、存档当前进度收尾（`--deadline`
  简单起跑门）；**跑批形态＝「单跑单题、主会话逐题监督」**——每次只放一题
  （`--stop-after 1`），题毕通知主会话，核对用时／reward／上传／容器与磁盘后再放
  下一题（异常在题间被及时抓住）。
- **日志**：总账 `jobs-official/official-v41-full-round.log`；逐题
  `jobs-official/<作业名>-round.log` 与 `<作业名>-console.log`；产物 `jobs-official/<作业名>/`；
  轨迹卷 `gsa-volumes/<作业名>/`（k=1 单试次，试次隔离无兄弟可隔离）。
- 静默旁路读数件（`tb21_round_gate.py` / `tb21_friction_scan.py`）不接入跑批链，跑后人工读取。

## 5. 起跑快照（2026-09-28 18:32）

- 宿主 D: free **35.87 GiB** / 276.63 GiB（首题 pre 快照）；宿主内存 15.85 GB 总量。
- Docker：容器 0；镜像 4（= 构建工具链 3 件 debian/rust/alpine ＋首题已拉任务镜像 1）。
- Harbor：已登录（SilverWhite）。
- 首题（链路验证题）`official-v41-b1-01-build-pov-ray`：身份门 OK、预拉 OK
  （`alexgshaw/build-pov-ray:20251031`，digest `874baf49…`）、`max_wallclock=12000` 透传、
  2026-09-28 18:32:52 起跑。

## 6. 边界与不做项

- Docker VM 内存 ≈7.7 GB（宿主 16 GB 的一半，WSL2 默认分配）对 `memory_mb=8192` 的
  内存重题仍偏紧——沿第 0 轮先例以 `-n 1` 串行缓解；若单题 OOM 则记录后单独补跑，
  不为跑分临时改 Docker Desktop 系统配置。
- 提交与推送冻结（2026-09-28 用户令）：本档与两脚本改动留工作树，随下一批次入账。
- 不做：为跑分特化、旧数字并轨、并发重载、预改冻结 corpus runner（`run_official_2.1.sh` 未动）。
- 台账同步（BACKLOG `0b` 验证⑤ / 索引 / TODO）待轮次收尾批统一入账。

## 7. 逐题进度（滚动追加）

| 题 | 作业 | 起跑 | 结束 | reward | 备注 |
|---|---|---|---|---|---|
| b1-01 build-pov-ray | `official-v41-b1-01-build-pov-ray` | 09-28 18:32:52 | 09-28 19:05:28 | 0.0 | 链路验证题，四环节全绿；max_wallclock=12000；上传 `hub.harborframework.com/jobs/1e5089c7-6f4b-4487-be59-630d8fbc0668`（public）；32.9 min |
| b1-02 schemelike-metacircular-eval | 同名 | 09-28 20:0x（重跑） | 09-28 20:40:00 | **1.0** | 第 2 次试次（41 min）真完成：finished_at 盖章＋上传 `hub.harborframework.com/jobs/48949d8e-a0f1-48bf-a6dd-33840734793b`；第 1 次试次 19:06–19:25 被驱动器切换杀死（半程、骨架 result 遗留致误判跳过一次，`job_done` 已修复，骨架目录在 `_incomplete/` 取证） |
| b1-03 llm-inference-batching-scheduler | `official-v41-b1-03-llm-inference-batching-scheduler` | 09-28 19:28:00 | 09-28 19:55:19 | **1.0** | 27.2 min；上传在案；容器/镜像清理 rc=0 |
| b1-04 feal-linear-cryptanalysis | 同名 | 09-28 20:40 | 09-28 20:45:45 | **1.0** | 5 min；上传 `jobs/6e263f1e…`；清理 rc=0 |
| b1-05 dna-assembly | 同名 | 09-28 20:45 | 09-28 21:12:31 | **1.0** | 27 min；上传 `jobs/879bece4…`；清理 rc=0 |
| b1-06 feal-differential-cryptanalysis | 同名 | 09-28 21:12 | 09-28 21:22:21 | **1.0** | 10 min；上传 `jobs/294a94a7…` |
| b1-07 polyglot-c-py | 同名 | 09-28 21:22 | 09-28 21:32:42 | **1.0** | 10 min；上传 `jobs/8b793a79…` |
| b1-08 qemu-startup | 同名 | — | — | —（**挂起**） | **A2 装置侧失败×3，根因＝上游镜像源腐烂**：任务镜像 Debian 11 (bullseye) 的 `bullseye-security` 索引仍在，但索引指向的 `ca-certificates_20250419~deb12u1~deb11u1_all.deb` 已从池 404（索引/池不一致；bullseye LTS 2026-08 底到期迁移形态）⇒ agent 依赖安装步 `apt-get install` exit 100（秒级即终，orz 未起跑、判分未运行）；镜像内直接复现实证（`apt-get update` 绿、install 404）。**官方口径约束下不可修**（改镜像＝漂移不可比；轮中改适配器＝代际分叉）⇒ 三次失败试次均留证 `_incomplete/…-infra-failed{,2,3}-*`（前两试次已上传 `jobs/467d8928…` / `jobs/9476621b…`），已入 `deferred-tasks.txt` 延后清单，轮末/下窗重试（上游自愈或用户裁决例外）；若后续他题同形失败，逐题同法处置并汇总 |
| b1-09 sqlite-db-truncate | 同名 | 09-28 21:43 | 09-28 21:51:07 | **1.0** | 8 min；上传 `jobs/e85945fe…` |
| b1-10 vulnerable-secret | 同名 | 09-28 21:51 | 09-28 21:53:41 | **1.0** | 2 min；上传 `jobs/a9b7f4b2…` |
| b1-11 build-cython-ext | 同名 | 09-28 21:53 | 09-28 22:06:42 | **1.0** | 13 min；上传 `jobs/34a96d54…` |
| b1-12 configure-git-webserver | 同名 | 09-28 22:06 | 09-28 22:19:11 | **1.0** | 13 min；上传 `jobs/d002d61c…` |
| b1-13 fix-git | 同名 | 09-28 22:19 | 09-28 22:32:23 | **1.0** | 13 min；上传 `jobs/ec61223b…` |
| b1-14 headless-terminal | 同名 | 09-28 22:34（重跑） | 09-28 22:50:00 | **1.0** | **A3 网络瞬断×1 后重跑成功**：第 1 试次 agent bootstrap 步 `publicsuffix` 包 Fastly CDN 连接超时（与 A2 的 404 腐烂不同根因，b1-13 同期 apt 正常）⇒ 留证 `_incomplete/…-net-failed-*` 重跑，reward 1.0；上传 `jobs/acd63a2f…` |
| b1-15 merge-diff-arc-agi-task | 同名 | 09-28 22:50 | 09-28 23:03:12 | **1.0** | 13 min；上传 `jobs/b0a8be62…` |
| b1-16 git-multibranch | 同名 | 09-28 23:04 | 09-28 23:22:17 | 0.0 | **有效官方 0**：`AgentTimeoutError`（用满 900 s 官方墙钟后判分，未解出）⇒ 不重试；上传 `jobs/46020aed…` |

| b2-01 sam-cell-seg | 同名 | 09-28 23:23 | 09-28 23:48:30 | **1.0** | 内存重题 8192 MB：25 min 零异常，Docker VM 边界样本过关；上传 `jobs/f75d7599…` |
| b2-02 portfolio-optimization | 同名 | 09-28 23:48 | 09-29 00:02:43 | **1.0** | 14 min；上传 `jobs/dd8f2466…` |
| b2-03 video-processing | 同名 | 09-29 00:02 | 09-29 00:25:58 | **1.0** | 23 min；上传 `jobs/2dfa142f…` |
| b2-04 mcmc-sampling-stan | 同名 | 09-29 00:26 | 09-29 00:50:01 | **1.0** | 内存重题 8192 MB：24 min 零异常；上传 `jobs/4de29162…` |
| b2-05 path-tracing-reverse | 同名 | 09-29 00:50 | 09-29 01:09:10 | **1.0** | 19 min；上传 `jobs/239d8e22…` |
| b2-06 mteb-retrieve | 同名 | 09-29 01:09 | 09-29 01:28:18 | **1.0** | 19 min；上传 `jobs/07920214…` |
| b2-07 code-from-image | 同名 | 09-29 01:28 | 09-29 01:43:42 | **1.0** | 15 min；上传 `jobs/9badbf97…` |
| b2-08 break-filter-js-from-html | 同名 | 09-29 01:43 | 09-29 02:06:07 | **1.0** | 23 min；**撞满 1200 s 官方墙钟后判分仍通过**（有效试次）；上传 `jobs/fb88c8bc…` |
| b2-09 sanitize-git-repo | 同名 | 09-29 02:06 | 09-29 02:20:25 | **1.0** | 14 min；上传 `jobs/b60a27cd…` |
| b2-10 sparql-university | 同名 | 09-29 02:20 | 09-29 02:31:35 | **1.0** | 11 min；上传 `jobs/5447635c…` |
| b2-11 tune-mjcf | 同名 | 09-29 02:31 | 09-29 02:48:39 | 0.0 | 用满 900 s 官方墙钟未解出（有效官方 0）；上传 `jobs/e8d7515d…` |
| b2-12 git-leak-recovery | 同名 | 09-29 02:48 | 09-29 02:58:12 | **1.0** | 10 min；上传 `jobs/f60674ab…` |
| b2-13 cobol-modernization | 同名 | 09-29 02:58 | 09-29 03:12:12 | **1.0** | 14 min；上传 `jobs/1b6a3f4f…` |
| b2-14 fix-code-vulnerability | 同名 | 09-29 03:12 | 09-29 03:14:06 | 0.0 | **有效官方 0（模型流退化）**：journal `run_failed`＝`stream degeneration guard: reasoning_repetition`（400 字符重复跨度 20/20 命中，输出健康哨兵按设计斩流，209 事件后模型侧死亡）；非装置/基础设施失败 ⇒ k=1 如实记账不重试；哨兵触发计数 1（0d 判据线 ≤3）；上传 `jobs/f6c1f395…` |
| b2-15 gpt2-codegolf | 同名 | 09-29 03:14 | 09-29 03:32:21 | 0.0 | 用满 900 s 官方墙钟（与超时验证轮 r1–r3「检索成功不交付」模型习惯形态一致，有效官方 0）；上传 `jobs/901fdf85…` |
| b2-16 log-summary-date-ranges | 同名 | 09-29 03:32 | 09-29 03:35:25 | **1.0** | 3 min；上传 `jobs/e861af13…` |
| b2-17 openssl-selfsigned-cert | 同名 | 09-29 03:35 | 09-29 03:41:14 | **1.0** | 6 min；上传 `jobs/b4d63d01…` |
| b3-01 mteb-leaderboard | 同名 | 09-29 03:41 | 09-29 04:08:36 | **1.0** | 内存重题 8192 MB：27 min；历史未解池成员翻转；上传 `jobs/8926421a…` |
| b3-02 reshard-c4-data | 同名 | 09-29 04:08 | 09-29 04:31:42 | **1.0** | 23 min；上传 `jobs/ba8bdccf…` |
| b3-03 winning-avg-corewars | 同名 | 09-29 04:31 | 09-29 05:36:30 | 0.0 | 撞满 3600 s 官方墙钟未解出（有效官方 0）；上传 `jobs/3be4d78b…` |
| b3-04 caffe-cifar-10 | 同名 | 09-29 05:36 | 09-29 06:24:56 | **1.0** | 内存重题 8192 MB：48 min；内存重题累计 5/5 全过；上传 `jobs/96982db4…` |
| b3-05 rstan-to-pystan | 同名 | 09-29 06:47（重跑） | 09-29 07:13:16 | **1.0** | **A4 一次性中途暴死×1 后重跑成功**：第 1 试次 19m45s 处 journal 止于 `tool_started`（无终态事件、exit 1 非 137，OOM 证据不足）⇒ 留证 `_incomplete/…-midrun-exit1-*` 重跑，`run_finished` 正常、reward 1.0；上传 `jobs/adfa6353…`；内存重题 6/8 |
| b3-06 extract-moves-from-video | 同名 | 09-29 07:13 | 09-29 07:44:17 | 0.0 | 31 min 早收工判分未过（历史未解池成员，有效官方 0）；上传 `jobs/578725fb…` |
| b3-07 custom-memory-heap-crash | 同名 | 09-29 07:44 | 09-29 07:56:45 | **1.0** | 12 min；上传 `jobs/371839e4…` |
| b3-08 constraints-scheduling | 同名 | 09-29 07:57 | 09-29 08:04:10 | **1.0** | 7 min；上传 `jobs/3e915feb…` |
| b3-09 pytorch-model-recovery | 同名 | 09-29 08:04 | 09-29 08:24:36 | 0.0 | 撞满 900 s 官方墙钟（有效官方 0）；上传 `jobs/1b8757c2…` |
| b3-10 prove-plus-comm | 同名 | 09-29 08:24 | 09-29 08:28:27 | **1.0** | 4 min；上传 `jobs/ce093e21…` |
| b3-11 raman-fitting | 同名 | 09-29 08:28 | 09-29 08:40:13 | **1.0** | 12 min；**历史未解池成员翻转**（r3-unsolved20）；上传 `jobs/846419b1…` |
| b3-12 torch-pipeline-parallelism | 同名 | 09-29 08:41 | 09-29 08:56:53 | 0.0 | 内存重题 8192 MB：900 s 内完成判分未过（历史未解池成员，有效官方 0）；上传 `jobs/54383f7e…` |

## 8. 窗口收口存档（2026-09-29 08:57，计费高峰停跑）

- **停跑裁定**：08:56:53 b3-12 结束后不再放新题——下一题 agent 运行期必越 09:00
  计费高峰，与「只跑到 09:00」口径一致；无在途任务，装置静止（容器 0）。
- **本轮窗口总账**：**44 题完成＝1.0×35、0.0×9（命中率 79.5%）**＋b1-08 挂起
  （上游腐烂）＋**44 题未跑**（b3-13…b3-19＝7、B4＝18、B5＝19）。
  分批：B1＝13/15（2×0.0）、B2＝14/17（3×0.0）、B3 已跑 8/12（4×0.0）。
- **结构读数**：内存重题 8 道已跑 6 道＝4 过 2 未过（mcmc/mteb/caffe/rstan 过，
  gpt2-codegolf/torch-pipeline 未过）；**历史未解池翻转 4**（path-tracing-reverse／
  mteb-leaderboard／dna-assembly／raman-fitting 全部 1.0），未解池未翻转 4
  （gpt2-codegolf／tune-mjcf／extract-moves-from-video／torch-pipeline-parallelism）；
  哨兵触发 1（b2-14 模型流退化）；装置侧异常全部留证（A2 镜像源腐烂／A3 网络瞬断
  重跑过／A4 中途暴死重跑过）；44 个完成试次全部上传 Harbor（公开作业）。
- **对照**：R1 代同口径全轮 65.2%；本轮当前 79.5%（剩余题含约 13 道历史硬核，
  落点待 B4/B5）。参照线 90.6（官方公布值，非 k=1 同口径）。
- **续跑方式**：下个非高峰窗口重发同命令即可（`run_official_v41_full.py
  --stop-after 1 --deadline <新截止>`），断点自 b3-13 续；b1-08 在
  `deferred-tasks.txt` 保持挂起（上游自愈或用户裁决例外后移出重试）。
  **2026-09-29 用户令（口径改定）＝b3-13 断点不续，整轮 89 题直接全量重跑**
  （窗口 1 的 44 题 0.8.4 账面保留留档；全量重跑以 0cb S3 后新载体统一
  代际起跑，两段不并账；b1-08 挂起处置沿上）。
  **〔2026-09-29 本窗再裁决（取代上行全量重跑口径）＝外科重跑：0 分题中被写控
  拦截 ≥5 条者重跑（5 题）＋b3-13 断点续跑；全量重跑以成本否决。权威＝
  [`TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29`](TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md)〕**

- **B2 批收官（2026-09-29 03:41）**：17 题全部处置＝14×1.0、3×0.0（b2-11 tune-mjcf 撞墙钟、
  b2-14 fix-code-vulnerability 模型流退化哨兵斩流、b2-15 gpt2-codegolf 撞墙钟——均有效官方 0）。
  **全轮累计 32 题完成：1.0×27、0.0×5（命中率 84.4%）＋b1-08 挂起**；哨兵触发 1 次。
  03:41 起进入 B3 批（19 题，首题 b3-01 mteb-leaderboard 为内存重题 3600 s）。

- **B1 批收官（2026-09-28 23:22）**：16 题全部处置＝13×1.0、2×0.0（b1-01 build-pov-ray、
  b1-16 git-multibranch 均为有效官方 0）、1 题挂起（b1-08 上游腐烂，见 A2）；
  异常两起（A2 镜像源腐烂、A3 网络瞬断）均已定性与留证；完成试次 15/15 全部上传 Harbor。
  23:22 起进入 B2 批（17 题，首题 b2-01 sam-cell-seg 为内存重题 8192 MB——Docker VM
  内存边界首个实战样本）。

- **链路验证结论（2026-09-28 19:05）**：身份门→预拉→k=1/n=1→官方墙钟透传→判分
  （`result.json` 双层落盘）→Harbor 公开上传→轨迹卷 journal 宿主落盘→镜像清理（rmi rc=0）
  全链路绿。
- **异常 A1（2026-09-28 19:25–19:56，已修复）**：驱动器形态切换杀停时 b1-02 半程被杀；
  harbor 作业级 result.json 为**开跑即写的骨架**（`finished_at=None`、trials 空），
  `job_done` 只查文件存在 ⇒ 19:27 该题被误判已完成跳过（少跑一题）。修复＝完成判定改为
  内容校验（`finished_at` 非空且 trials 非空或存在逐试次件）；b1-02 重跑（第 2 次试次），
  骨架目录入 `_incomplete/` 取证。该缺陷对 b1-01/b1-03 无影响（两者经内容复核为真完成）。
- **监督形态（19:4x 起）**：单跑单题＋主会话逐题核对（用时/reward/上传/容器/磁盘），
  09:00 截止＝简单起跑门（不做硬截止准入，在跑题自然跑完，到点停新题存档）。
