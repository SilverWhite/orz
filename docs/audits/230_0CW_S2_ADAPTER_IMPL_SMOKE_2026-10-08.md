# 230 批：0cw S2 适配器落码＋冒烟——重置形态与连续模式双判据达成（2026-10-08）

> **用户令**：「请开始进行S2吧」（228 批 S1 后放行）。
> **性质**：实施＋冒烟批——落码面全在评测侧本地件（`D:/tb-eval/scbench/scb_agents/`，不入父仓、
> 不入 orz）；零 orz 源码、零父仓源码、零跑批（冒烟＝装置侧单题验证，非 S3）；计数不变 **55**；
> 未提交、未推送。批号说明：并发会话已按 228 §6 撞号登记将 gist 探针批转 **229**（台账已整合，
> 本批 228 条目完好），本批顺延 **230**。

---

## §1 落码（两件，评测侧本地件）

- **`scb_agents/orz_acp_driver.py`**（容器内驱动，stdlib-only py3.8+）：三模式——
  `step`（默认＝官方重置形态：每 checkpoint 全新 `orz --stdio --real` 进程＋全新 ACP 会话）／
  `daemon`（连续形态服务端：单进程单会话持有，unix socket `/tmp/orz-acpd-<sha1(cwd)12>.sock`）／
  `client`（连续形态步驱动：连 daemon、缺则 `start_new_session` 自启）。软看门狗按 228 §2：
  deadline−grace 发 `session/cancel` 等 `Cancelled`，grace 后杀进程（防挂死 orz 滞留容器污染
  后续档；exit 124=timeout、0=ok、1=error）。权限请求自动 allow（0cr 同形）；journal 发现按
  墙钟 mtime 取 prompt 起点后最新 `RUN-*`；wire transcript＋`step_result.json`＋journal 副本
  落 `--logs-dir`。**零密钥纪律：env 值不落任何工件**。
- **`scb_agents/orz_multistep.py`**（harbor agent，`BaseInstalledAgent` 子类，import path
  `scb_agents.orz_multistep:OrzMultiStep` 接入）：install＝三件上传（`--ak
  orz_binary=D:/tb-eval/orz-linux/orz`，signer/provision 取同目录）＋容器内 ACAF provision
  （fail-closed ENFORCED）＋`orz trust`（best-effort）＋安装自检＋`orz --build-info` 版本捕获
  （`0.8.15 os=linux`）；`capabilities=AgentCapabilities(atif=True)`（resume=False ⇒ harbor
  默认每步全新 run）。run＝指令经上传文件传递（绕 argv 引号/长度面）＋driver exec（env 白名单：
  key、ACAF 四件＋fail-closed、ALLOW_WRITE/SHELL 恒开、ALLOW_NETWORK 随 harbor network policy
  PUBLIC、`ORZ_MAIN_AGENT_MODEL=deepseek-flash`、`ORZ_MAX_TOOL_ROUNDS=999`〔0cr 同形〕、
  `RUST_LOG=debug`）；检索门不传 env＝缺省 off（0cr 同形）。`--ak continuous=true` 切连续形态
  （0cr 复现开关，226 批登记落点）。`populate_context_post_run`＝journal→ATIF 轨迹（TB2.1 适配器
  映射）＋`AgentContext.metadata` 步证据（cost/token 轴按 228 §2 如实 null）。

## §2 冒烟读数（dag_execution 3 档×2 轮；harbor v0.23.0、载体 0.8.15 `75515440…`、k=1）

### 2.1 重置模式（默认；job `0cw-s2-smoke-reset`，completed 1 / errored 0）

| 档 | session id | journal RUN | stop | 轮数 | 墙钟 | core | strict |
|---|---|---|---|---|---|---|---|
| ck1 | `f25e9227…` | `RUN-f25e9227-0` | end_turn | 50 | 664s | 0.417 | 0.636 |
| ck2 | `3e8e955e…`（焕新✓） | `RUN-3e8e955e-0`（焕新✓） | end_turn | 78 | 720s | 0.600 | 0.659 |
| ck3 | `06fe6a8b…`（焕新✓） | `RUN-06fe6a8b-0`（焕新✓） | end_turn | 103 | 1135s | 0.000 | 0.549 |

- **判据 1 达成**：逐档 session id＋journal RUN 全焕新；**工作区跨档持久**＝ck2 core 0.417→0.600
  （建立在 ck1 产物上）＋各档 strict/regression 读数连续。ck3 core 坍缩与 0cr 同题形态一致
  （「8/12→2/5→0/3 单调坍缩」同族）＝模型表现面、非装置面，摩擦主线如实记。
- 官方指标族全流程落盘（core/strict/isolated/erosion/verbosity 逐档 reward.json）。

### 2.2 连续模式（`--ak continuous=true`；job `0cw-s2-smoke-cont`，completed 1 / errored 0）

| 档 | session id | journal RUN | stop | 轮数 | 墙钟 | core | strict |
|---|---|---|---|---|---|---|---|
| ck1 | `27567c00…` | `RUN-27567c00-0` | end_turn | 52 | 728s | 0.833 | 0.939 |
| ck2 | **同上（恒一✓）** | `RUN-27567c00-1` | end_turn | 28 | 374s | 0.800 | 0.927 |
| ck3 | **同上（恒一✓）** | `RUN-27567c00-2` | end_turn | 16 | 275s | 0.000 | 0.765 |

- **判据 2 达成**：daemon 自启（`/tmp/orz-acpd-<sha1>.sock` 在证）＋单会话跨全部档＝0cr 连续
  形态精确复现；journal 仍每 prompt 一 RUN（后缀 `-0/-1/-2` 递增）＝0cr journal 形状。轮数
  52→28→16 递减＝上下文携带的直接形状（k=1 冒烟观察、不作结论）。

### 2.3 判据 3/4

- **判据 3（2h 帽接线）达成**：`step_result.json` `timeout_cfg=7200 / grace_cfg=180` 逐档在证＋
  install 读数（`continuous=…; step_timeout=7200s grace=180s`）；冒烟题短未触发（如实记）。
- **判据 4（ACAF＋零密钥）达成**：reset 轮 ck1 44 issued/44 consumed、cont 轮全程 90 issued；
  两轮 trial 工件全树 grep 密钥前缀零命中。

## §3 设备侧修复（非 0cw 范围，如实登记）

- 冒烟首跑败于**镜像构建期 apt 断网**——根因＝Docker Desktop（system 模式）解析到死端口
  `127.0.0.1:3128`（netstat 无监听；WinHTTP/注册表均无代理，Desktop 内部状态残留）；主机直连
  deb.debian.org 正常、本机 7890 代理活着（docker.io 401/debian 200 双通实测）。
- 处置＝沿 2026-09-19 死端口先例：备份 `settings-store.json.bak-20261008` → `ProxyHTTPMode:
  system→manual` 指向活端口 `http://127.0.0.1:7890` → Docker Desktop 彻底重启；验证＝ghcr 基础
  镜像 9s 拉取＋容器内 `apt-get install` 通过。**回滚路径**＝恢复备份文件（代理软件常驻且 3128
  复活时）。

## §4 边界与登记

- **冒烟载体＝本地转换集任务目录**（`harbor-tasks/silverwhite/dag_execution`，设备侧装置验证）；
  S3 跑批用官方 registry 数据集 `gabeorlanski/slopcodebench`（起跑时 pin 版本＋digest，228 §1.2）。
- harbor 0.23 `options_model` 警告为噪声（0.20 形态差异）：`--ak` 具名 kwargs 在构造器具名捕获、
  已到位于证（`continuous=True`/超时读数进 install log）。
- usage 轴 null 按设计（228 §2）；轨迹 ATIF 面仅 AgentMessageChunk＋journal 映射（如实薄）。
- 冒烟两轮成本量级：6 checkpoint × flash off-peak，控制台实价随 215 批式报告在 S4 合并披露。
- S3 待放行项：官方数据集版本 pin＋起跑器（全目 36 题字母序、`--upload --public`、低谷价时段、
  实时花销监控＋暂停权〔227 裁决〕）。

## §5 台账

- 本档：`docs/audits/230_0CW_S2_ADAPTER_IMPL_SMOKE_2026-10-08.md`。
- BACKLOG：指针行（230）＋`0cw` 专节 S2 达成注记；计数不变（55）。
- TODO：头行指针（230）＋P1 路由行＋`P1-0cw` S2 勾选达成。
- BACKLOG 第二卷：§1.175。
- 索引：头行 v4.202 → **v4.203**；§6 `0cw` 条目 S2 注记；§8 pending 桶 0cw 更新。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑（§6）。

## §6 关联与关键词

[`228 批档`](228_0CW_S1_OFFICIAL_FORM_ADJUDICATION_2026-10-07.md)（S1 设计输入）／
[`226 批档`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md)（母批）／
[`229 批档`](229_GIST_TRANSPLANT_PROBE_ARCHIVE_2026-10-08.md)（撞号转 229 的并发批）／
[`215 批档`](215_SCB_COST_BREAKDOWN_2026-10-07.md)（花销报告先例）／
TB2.1 适配器先例 `tb_agents/orz.py`／[`09-19 死端口先例`](TB21_V41_TIMEOUT3_VERIFY_2026-09-18.md)。

关键词：0cw S2、scb_agents、ACP 驱动三模式、每 checkpoint 全新进程＋新会话、daemon 自启、
连续模式开关、逐档焕新证据、sid 恒一、RUN 后缀递增、工作区跨档持久、2h 帽接线读数、ACAF
44/90 issued、零密钥、Docker 代理死端口修复、harbor v0.23.0、计数 55 不变、索引 v4.203。

---

## §7 S3 路径预检补记（2026-10-08，用户令「再进行一轮确认，确保每checkpoint重置和上传没问题」）

- **官方数据集钉值（S3 起跑值）**：`gabeorlanski/slopcodebench@sha256:aec29354c19d3e762640ab6d7d3c63ba8fcf4895e98a5756aecb9157b5bb4ae0`（36 任务＋`metric.py`；hub API 经 harbor 0.23 自身 client 解析）。官方集与本地转换集**全树 diff＝仅任务名一行**（`gabeorlanski/*` vs `silverwhite/*`）＋行尾差（本地 CRLF/官方 LF，EOL 归一后逐字节全同）。
- **预检轮（官方数据集单题 dag_execution＋重置模式＋`--upload --public`；job `0cw-s3-precheck-reset-upload`，38m35s，completed 1/errored 0）**：
  - **重置复证 ✓**：三档三 sid 三 RUN 全焕新（`80e72735`/`80f6cff8`/`af494ddf`，RUN 各 `-0`），全 end_turn；core 0.833→0.600→**0.667**——ck3 core 2/3 首次未坍（dag_execution 系全部五轮中首个非零末档；k=1 方差在案，不作结论）。
  - **上传复证 ✓**：回执 `Uploaded to Harbor Hub: https://hub.harborframework.com/jobs/a77a77f0-a87d-44a0-9cda-c137b81a89c9 (visibility: public)`；匿名抓取复核＝页面公开可见（"Public" 标识、1/1 finished、Average core pass rate 0.70、owner silverwhite；"No per-trial token usage"＝usage 轴 null 契约在公开面的如实呈现）。榜单可见性（Model×Harness 聚合面）待 S3 全目 job 后核。
- 预检性质＝装置确认，成绩面零结论（k=1 单题）；S3 待放行不变（全目 36 题字母序、低谷价时段、实时花销监控＋暂停权〔227 裁决〕、`--upload --public`）。
