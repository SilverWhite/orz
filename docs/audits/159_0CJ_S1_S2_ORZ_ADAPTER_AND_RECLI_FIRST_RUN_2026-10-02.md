# 159 批：0cj S1 挑题冻结（recli）＋S2 orz 适配器落地＋recli 真机首跑发射（2026-10-02）

> **日期**：2026-10-02；**用户令**：「目前RLI已经不是影子了」「就按照你的判定，跑recli吧」；中途「请先暂停」「暂时不写报告档」（另一处并发编辑工作区文档，本批落账后补）「现在可以开始落文档了」。
> **形态**：主会话直接执行（S1 台账＋S2 rig 侧新写适配器＋真机首跑发射）；**未提交、未推送**；计数不变 60。
> **边界**：零 orz 源码改动（只观测不改造）；harness 侧改动全在 rig clone（`D:/tb-eval/scbench/`，不在本仓）。

---

## §1 用户裁决与口径勘误（RLI 非影子）

- **RLI 口径勘误（用户令）**：RLI **已不是影子**——0bf（2026-09-22「RLI 生产化与域建模试验」）已将语义反转为**缺省常开**（kill-switch＝显式 `ORZ_LIF_RLI_SHADOW=0/off/false/no`）；「影子」仅存于该 env 名与内部结构名。158 批档 §2.2＋BACKLOG 主卷两处＋第二卷 §1.110 的「RLI 影子」措辞已同批修正为「0bf 起常开生产化」。
- **S1 首题冻结（用户令「就按照你的判定，跑recli吧」）**：**recli**（Hard×8 checkpoint＝Hard 池最长 RLI 跑道；CLI 框架全家桶：层级命令分派/参数校验/YAML 配置继承/别名/输出格式化/文件缓存/SQLite 持久化/容器编排/版本升级/系统要求检查；test_deps 仅 pyyaml）。**冻结口径＝首题先行**，3–5 题全 manifest 留首跑读数后随用户一并裁决（与 P2-15 纪律的偏差如实登记：冻结范围＝1 题）。

## §2 S2 适配器（rig 侧新写）

### 2.1 落点与 pin

- rig＝`D:/tb-eval/scbench/`：harness clone `slop-code-bench`（pin `31ceea3a`）＋题目 clone `scb-problems`（pin `38d627ec`）；`SCBENCH_PROBLEMS_PATH` 直指题目 clone（绕开 harness 托管目录，pin 完全本地可控）。
- 环境：Docker Desktop（引擎 29.6.2）／Python 3.12.0／uv 0.10.6／orz 三件套 `D:/tb-eval/orz-linux/`（0.8.10：orz＋orz-signer＋orz-acaf-provision）／key＝`D:/tb-eval/.env`（TB 同源）。

### 2.2 新增件（harness 侧四件）

| 件 | 内容 |
|---|---|
| `src/slop_code/agent_runner/agents/orz/agent.py` | `OrzAgentConfig`（type `orz`：binary/binary_dir〔env `ORZ_ORZ_BINARY_DIR` 兜底〕/max_tool_rounds/timeout/extra_env）＋`OrzAgent`（Agent 六方法实现＋ACP 驱动 `_AcpProcess`）＋`register_agent("orz", ...)` |
| `agents/__init__.py` | orz 导入与注册（AgentConfigType 判别联合自动收编） |
| `configs/agents/orz.yaml` | `type: orz / binary_dir: D:/tb-eval/orz-linux / max_tool_rounds: 999 / timeout: 3600 / cost_limits 全 0（=不设限）` |
| `configs/models/deepseek-v4-flash.yaml` ＋ `configs/providers.yaml` | deepseek provider（env `ORZ_DEEPSEEK_API_KEY`）＋模型条目（pricing 全 0＝不引 harness 成本账，读数以 journal 为准） |

### 2.3 驱动形态（对 RLI 观测关键）

- **容器内常驻 `orz --stdio --real`**（ACP 新行分隔 JSON-RPC）：`-p` 每次新会话不可续（bootstrap_session 每 run 新 journal），`--real` 为全局 argv 扫描旗标、缺省走 fake 传输（冒烟一轮 fake 输出实锤后修正）；`initialize → session/new（cwd=容器 workdir）→ 逐 checkpoint session/prompt`——**同会话＝同黑板＝同上下文**，RLI（0bf 常开、会话级跨 run 累积「全 run 无窗口」）天然覆盖全题长程。
- 权限出口：自动应答 `session/request_permission`（选 allow 项；无 allow 选项时仍回第一项防挂死）＝与 claude_code `bypassPermissions` 同姿势；未知 client 请求回 method-not-found。
- **per-checkpoint watchdog 3600s**：超时发 `session/cancel` → 等 `StopReason::Cancelled`（180s grace）→ 杀管道。0.8.10 载体已无 `ORZ_MAX_WALLCLOCK` 自限（148 批清退），客户端 watchdog 即唯一墙钟。
- usage 记账＝checkpoint 结束后解析最新 `.gsa/runs/RUN-*/events.jsonl` 的 `model_output` 计数；产物面＝`acp_transcript.jsonl`＋`acp-wire.log`（全量 wire 双向实时落盘，诊断用）＋`orz-acp.stderr.log`（`RUST_LOG=debug`）＋journal tar 拷贝。
- 容器供应链＝TB 适配器同形：docker cp 三件套 → chmod → **ACAF 容器内 provision（fail-closed ENFORCED）** → `orz trust` 兜底；env 白名单＝key／`ORZ_ALLOW_WRITE/SHELL/NETWORK`／ACAF 四件／`ORZ_MAX_TOOL_ROUNDS`／`RUST_LOG=debug`。

### 2.4 harness 上游 Windows 缺陷修正四处（rig clone 内，随本批登记）

1. `docker_runtime/models.py resolve_host_user`：`os.getuid()` 非 posix 直接 AttributeError → 非 posix 且无 `HUID/HGID` 时返回 None（容器默认用户）。
2. `docker_runtime/streaming.py _container_workdir`：Windows 下 `str(Path("/workspace"))`＝`"\workspace"` 被 daemon 400 → `as_posix()`。
3. `docker_runtime/exec.py _container_workdir`：同款第二处（**评测路径**踩中——checkpoint_1 首评 `docker run --workdir '\workspace'` 125 秒败＝`infrastructure_failure`，修后 checkpoint 2+ 评测即恢复；checkpoint_1 分数候跑毕独立复评 `slop-code eval`）。
4. `execution/placeholders.py` 静态资产容器路径：同款第三处（`"\static\..."`；recli 无资产未踩，多数据资产题必踩，预修）。
   另：无模板 agent 的镜像链＝`config.image` 回退 `slop-code:python3.12`（带标签派生基镜像），须先 `slop-code docker build-base` 构建（首次因 apt 网络抖动失败，重试成功，1.18GB）。

## §3 冒烟与首跑

### 3.1 ACP 冒烟（`smoke_orz_acp.py`，裸容器）

握手＋session/new＋真实小任务往返全绿：10.4s、182 条 update、`smoke.txt` 实际写入、4 模型轮、ACAF 票据/权限/orientation/counterexample 机构事件全在。**第一跑曾得 `(fake) 已收到请求`**＝缺 `--real`，修正后复跑全绿。

### 3.2 recli 首跑（run5，2026-10-02 19:07 发射，进行中）

- **run4（19:56 前）停摆事故**：checkpoint_1 在 initial_round 注入后、第 2 轮模型请求发出前停摆 45 min（journal 止于 face_fingerprint round 2；线程全 futex/pipe_read 停、容器无 TCP、stderr 无传输错误）——判定为**满盘时期引擎不稳的一次性 hang**：debug 追踪重跑（run5）同点顺利越过（round 11+ 在跑、LIF 域迁移事件在册、wire 流式正常）。
- **run5 中期读数（20:0x 核）**：checkpoint_1 完成＝16 模型轮／22 工具调用并已转 checkpoint_2（RUN-65f2858a-1）——**run id 共享会话前缀 `65f2858a`＝同会话跨 checkpoint 设计实证**；checkpoint_1 评测记录 `infrastructure_failure`（评测路径 workdir 缺陷第二处，§2.4-3，已修；分数候跑毕对 snapshot 独立复评）。
- run5 终局读数与 0cj S3 正式摩擦台账＝首跑完成后落档（追记本档或随 160 批）。

## §4 环境事故登记（Docker 满盘连锁，2026-10-02）

- 连锁时间线：run1 `Container not created`（惰性容器未拉起，修复＝`_ensure_container_running`）→ run2 `os.getuid` AttributeError → run3 workdir 400 → run4 发射成功但 checkpoint_1 停摆 → 深挖发现 **C:／D: 双盘 100% 满**（C: 1.2MB／D: 12MB 空闲）。
- 根因链：满盘 → `wsl --update` 报 `0x80070070`（盘满）实锤 → 动态 vhdx 无法增长 → bootstrap mkfs `No such device` → 引擎无法启动；早前「vhdx 损坏」判断实为同一根因（期间一次非正常 kill 加剧）。
- 处置：**用户清理磁盘**（C: 18G／D: 99G 空闲）＋重启 OS；18GB 旧数据盘改名留档 `D:/DockerData/DockerDesktopWSL/disk/docker_data.vhdx.corrupt-bak-20261002`（**未删除**，引擎健康确认后可删）；Docker Desktop 重建数据盘后引擎恢复（29.6.2）。
- **教训登记**：Docker 数据盘所在卷满＝vhdx 静默损坏与引擎挂死的根源；后续 TB／SCBench 跑批前先核盘（拟入跑批前检查清单）。

## §5 台账

- TODO：`P1-0cj` S1 勾选（首题冻结 recli，冻结口径注记）＋S2 勾选（适配器＋冒烟）；S3 行补首跑发射注记；计数行本批指针改写（计数不变 60）。
- BACKLOG：本批记录指针＋计数行本批指针＋`0cj` 节 S1/S2 状态行。
- BACKLOG 第二卷：§1.111（本批流水）。
- 索引：头行 v4.127 → **v4.128**。
- 机械门禁：落账后复跑 `check_repository.py`（结果见本节补记）。

## §6 关键词

159 批、0cj S1、recli 冻结、S2 适配器、orz --stdio --real、ACP 多轮、同会话跨 checkpoint、
RLI 会话级跨 run、watchdog cancel、ACAF 容器内 provision、Windows 三缺陷修正、Docker 满盘事故、
vhdx 留档、smoke 全绿、首跑发射。
