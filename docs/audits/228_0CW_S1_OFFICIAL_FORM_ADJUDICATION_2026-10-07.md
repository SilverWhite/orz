# 228 批：0cw S1 勘定——官方论文与上榜口径全参数定案（2026-10-07）

> **用户令**：「请开始0cw S1吧，这一次请完全按照官方论文和上榜口径进行」。
> **性质**：勘定批（只读勘证＋适配器设计定稿＋0cr 报告勘正注记）；零源码（orz 与父仓零改动；
> 评测侧适配器落码归 S2）；计数不变 **55**；未提交、未推送。
> **结论先行**：官方上榜形态＝**Harbor 多步任务**——官方题目仓将 36 题同时发布为 Harbor 数据集
> `gabeorlanski/slopcodebench`（一题一任务、checkpoint 链＝`[[steps]]`、每档 2h 帽、verifier 产
> 论文指标族），harbor 默认每步全新 agent run（`resume_trajectory` 显式开启才续会话）＝官方重置
> 形态的原生承载；0cw 采纳官方数据集＋每 checkpoint 全新 orz 进程＋全新 ACP 会话＋任务内建
> 7200s/档（不覆盖），`continuous` 开关保留 0cr 形态复现。226 悬项「2h 帽是否施加」收口＝**施加**。

---

## §1 官方口径全参数勘定（一手来源，本地 pin 实证）

### 1.1 官方承载形态＝Harbor 多步任务

| 来源 | 实证 | 含义 |
|---|---|---|
| 题目仓 README L18（scb-problems，本地 pin `38d627e`） | 「Problem definitions … also available as a **Harbor dataset** (registry.harborframework.com/datasets/**gabeorlanski/slopcodebench**/latest)」 | 官方双发布：题目仓＋Harbor 数据集 |
| Harbor Hub 数据集页（2026-10-07 实取） | 「This dataset contains **36 multi-step Harbor tasks** … official run: `harbor run -d "gabeorlanski/slopcodebench@latest" -a "<agent>" -m "<model>"`」 | 榜单口径的官方跑法即 harbor run |
| 官方转换产物结构（转换器 `scripts/scb_to_harbor.py`＋`conversion.toml` 均为题目仓 tracked 内容；本地 `harbor-tasks/silverwhite/` 为 0cr 以 `--org silverwhite` 的生成物、gitignored） | task.toml：`[[steps]]` 逐档 `name`/`artifacts`(source=/app)/`min_reward`＝`{core_pass_rate=0.0, strict_pass_rate=0.0}`/`[steps.agent] timeout_sec=7200`/`[steps.verifier] timeout_sec=3600`；`[agent] timeout_sec=7200` | 一 Harbor 任务＝一题；checkpoint 链＝steps；每档 2h 帽内建于官方任务 |
| harbor v0.23.0 `src/harbor/trial/multi_step.py` | 逐 step 串行；每步 `_run_agent_phase(instruction=task.step_instruction(step.name), timeout_sec=步超时)`；容器跨步存活（`_stop_agent_environment` 仅收尾）；每步独立 verifier＋artifacts；`_min_reward_failure` 阈值 0.0 恒不过线＝**不提前中止**；步异常（无 verifier_result）⇒ 该题后续档中止 | 工作区跨档持久＋每档独立评审判据的官方执行器 |
| harbor `multi_step.py::_step_resumes` | `resume = config.agent.resume_trajectory and index > 1`——**resume 是显式开关，默认每步全新 agent run**；`_reset_agent_logs_for_step` 非 resume 步清空 agent 目录 | **重置形态在 harbor 是默认**，无需任何适配 |
| harbor `agents/installed/acp.py` | runner 每 `run()` 以 exec 重拉（每步全新 ACP 进程＋全新会话）；`capabilities` 无 resume | harbor 自带 ACP agent 同形佐证 |
| 独立 harness `stream_processor.py` L33 | `DEFAULT_WAIT_TIMEOUT = 7200.0  # 2 hours` | 论文 App C「2 hour wallclock」在作者侧 harness 的落点 |

### 1.2 官方口径参数逐项与 0cw 采纳

| 项 | 官方口径（来源） | 0cw 采纳 | 0cr 对照 |
|---|---|---|---|
| 会话形态 | 每 checkpoint 重置（226 §1.1 三源＋本批 §1.1 harbor 默认） | **每步全新 orz 进程＋全新 ACP 会话** | 连续会话（偏离，226 勘正） |
| 墙钟 | 每 checkpoint 2h（论文 App C＋docs/problems/checkpoints.md 示例＋独立 harness 常量＋官方 harbor 任务 `timeout_sec=7200`） | **采纳任务内建 7200s/档，零覆盖**（不用 `--agent-timeout-multiplier`/`override_timeout_sec`） | 无帽（test_translator 189m） |
| 试次 | k=1（论文单跑；206 批勘误「无 ≥5 试次门槛」；harbor `n_attempts` 默认 1〔job/config.py L403〕） | 默认，不传 | k=1 同 |
| seed | 独立 harness 参数（0cr 冻 42）；**harbor 形态无 seed 轴**（run CLI 无该旗标，全查） | **N/A（如实登记）** | 42 |
| pass_policy | any ⇔ harbor `min_reward` 全 0.0（失败不中止后续档） | 数据集内建，不动 | any 同 |
| one_shot | off（独立 harness 形态）；harbor 无对应轴 | N/A | off 同 |
| 数据集 | 官方 `gabeorlanski/slopcodebench@latest`；S3 起跑时解析并 **pin 版本＋digest**（入 S4 账） | **官方集** | 独立 harness＋题目仓 checkout（205 批发布的 silverwhite 转换集是数据集件、非 0cr 跑批载体） |
| harbor pin | v0.23.0（本地 sibling；196 批勘定） | 不变 | 同 |
| 上榜通道 | `harbor run --upload --public`（208 批核证：榜单页按 Model×Harness 聚合公开 run——「Showing best version by % Checkpoints」） | 公开上传 | 未走 harbor run |
| 模型名 | `deepseek-flash`（0cv/217 批对齐现行官方名） | `--model deepseek-flash` | `deepseek-v4-flash`（legacy 名，同底模） |
| 步指令 | `steps/checkpoint_N/instruction.md`（`task.step_instruction`）；无 per-step setup（cfgpipe 实样） | 官方指令原样 | 独立 harness checkpoint 任务串同源 |

### 1.3 2h 帽裁决（226 悬项收口）

- **施加**。四源一致（论文 App C／官方 docs checkpoint 示例「(2 hours)」／作者 harness 常量 7200／
  官方 harbor 任务字段 7200）；用户本批令「完全按照官方论文和上榜口径」⇒ 官方数据集内建值即官方
  形态，**不做任何覆盖**。
- 后果如实登记：0cr 无帽面下 ≥2h 的档（test_translator 189m 等）在 0cw 将被帽；harbor 步超时＝
  步异常 ⇒ **该题后续档中止**（`_should_stop_after_step`）。这是官方口径的组成部分，不构成偏离。

## §2 orz 适配器重置模式设计（S2 落码输入）

### 2.1 形态定案

- **自定义 harbor agent**（`BaseInstalledAgent` 子类），以 import path 接入
  `harbor run --agent <module:Class>`（`AgentFactory.get_agent_class_from_config` 支持
  `module.path:ClassName`；`AgentName` 内建表无 orz）。落点＝评测侧本地件（沿 TB2.1
  `tb_agents/orz.py` 839 行先例的存放形态；不入父仓、不入 orz）。
- **install()**＝TB2.1 形态照搬：upload `orz`/`orz-signer`/`orz-acaf-provision`（0.8.15 Linux 载体
  三件）→ 容器内 ACAF provision（fail-closed ENFORCED）→ workspace trust → 安装自检。
- **run()＝每 checkpoint 全新 orz 进程＋全新 ACP 会话**：launch `orz --stdio --real` →
  initialize → session/new → session/prompt(instruction) → 等停（收 stop reason）→ 步末终止进程。
  容器与工作区由 harbor 跨步存续。**重置证据面＝每步新 session id＋新 journal `RUN-CLI-*`**。
- **A/B 轴登记**：目标差异＝会话连续性；进程生命周期同时变为每档焕新（＝官方 claude_code 每
  checkpoint 重启 CLI 的同形）。进程级模型可见状态仅 project_doc_index 缓存（透明、零语义差）——
  登记为第二已登记差异（对 0cr 对照的可归因性无涉：0cr 的连续会话是偏离项，0cw 向官方对齐）。
- **连续模式开关**：agent kwarg `continuous=true`＝0cr 形态复现（`--stdio` 常驻单进程单会话跨全部
  档，226 登记的「连续模式留开关」落点）；默认 `false`＝官方重置形态。S2 冒烟两态各验。
- **超时面**：harbor `wait_for(7200)` 为主执行器；适配器设**同值软看门狗**（deadline−grace 发
  `session/cancel` 等 `Cancelled` stop reason，grace 后杀进程）——防挂死 orz 进程滞留容器污染后续
  档（harbor 跨步不重建容器；0cr 适配器 3600s 看门狗形态沿用量值换 7200＋grace）。
- **usage 读数面**：orz ACP 仅发 `AgentMessageChunk`（源码核证：`SessionUpdate::` 唯一发射值），
  无 `usage_update`/`tool_call`；journal v0.2 契约无 usage 字段（0cr journal 实查 `"usage"` 0 命中）
  ⇒ **公开 run 的 cost/token 轴＝null**；模型轮数沿 0cr journal 吸收法入 `AgentContext`；花销透明＝
  S3 实时控制台监控（227 裁决）＋S4 215 批式分项报告。orz 侧**零改动**（新增 usage 事件属契约面
  变更，不在本线）。
- **证据公开面**：步 artifacts source=/app 不排除 `.gsa` ⇒ 逐档 journals 随 artifacts 公开（零密钥
  面，脱敏纪律在案）；轨迹公开形态＝harbor ATIF 转换面（仅 AgentMessageChunk，如实薄）。

### 2.2 冒烟判据（S2 出口）

1. 重置模式（默认）：单题全档逐档**新 session id＋新 journal RUN**；工作区跨档持久（前档产物后档
   可见）。
2. 连续模式（开关开）：单 ACP 会话跨档＝0cr 形态复现（wire transcript 会话 id 恒一）。
3. 2h 帽：以配置读数证步超时接 harbor 面（冒烟题短不触发）。
4. ACAF fail-closed 容器内生效（0cr 先例复核）＋零密钥泄漏面。

## §3 0cr 报告勘正（注记随批落，dated 档不回改）

- **§5「证伪」表述**：11/36 全档全绿是在**连续会话变体**下取得；「后期档必然坍缩」是基准对
  **重置形态**的设计预期——连续变体的反例不构成对该预期的证伪（形态变量未对齐）。勘正为
  「连续会话变体观察、不构成证伪」。
- **§6 对照表与边界③**：134/196 与任务级 11/36 同论文/榜单的差值混杂**会话形态**因子；「优先归因
  harness（会话连续性＋压缩管理＋黑板）」收窄为「harness 差（含会话形态）整体」，单因断言待 0cw
  A/B 读数（0cw 即隔离「会话连续性」的对照轮）。
- **落法**：报告追加 §15「形态勘正注记」（本批）。

## §4 S1 附带裁决与边界

- **0ct/0cu S4 真机复核不搭本轮**（维持 226 判）：harbor 评测容器形态不充当载体。
- **官方数据集 vs 我方转换集**：0cw 用官方集 ⇒ 205 批 silverwhite 数据集件保持摩擦产物定位不动
  （双标签不变）；两集同源（同 scb-problems 转换器 lineage），登记为已知载体差注记。
- **题序**：全目 36 题官方字母序（227 裁决不变；harbor 按数据集任务序串行）。
- **错误中止语义**：步异常中止该题后续档（官方执行器行为）＝与独立 harness 错误态停跑同形。
- 本批零源码；S2 适配器落码＋冒烟待放行。

## §5 台账

- 本档：`docs/audits/228_0CW_S1_OFFICIAL_FORM_ADJUDICATION_2026-10-07.md`。
- BACKLOG：指针行（228）＋`0cw` 专节 S1 达成注记（悬项收口＋官方集采纳）＋计数行不变（55）。
- TODO：头行指针（228）＋`P1-0cw` S1 勾选达成。
- BACKLOG 第二卷：§1.174。
- 索引：头行 v4.200 → **v4.201**；§6 `0cw` 条目 S1 注记；§8 pending 桶 0cw 更新。
- 0cr 报告：追加 §15 形态勘正注记。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑（§6）。

## §6 批号撞号登记（并发会话）

- 本批落盘后发现并发会话产出的候选存档档 `docs/audits/228_GIST_TRANSPLANT_PROBE_ARCHIVE_2026-10-07.md`
  （CONTEXT-GIST-TRANSPLANT-PROBE 立案 candidate＋压缩凹陷 v0 初测；零源码、计数不变 55、
  **零台账足迹**——四本台账均无其批注；mtime 00:04:17）与本批撞号「228」。
- **先后与处置**：本批批档先落盘（00:01:35）且台账已按 228 落；gist 批转 229 与否随其落账会话/
  用户裁决。沿第二卷 `1.166` 邻窗重号先例（登记不回改），本批不动第三方在飞工件。
- 另有并发未跟踪件 `docs/en/REDDIT_POST_DRAFT.md`（23:42）与主档
  `docs/CONTEXT_GIST_TRANSPLANT_PROBE_2026-10-07.md`，均非本批产物、不触碰。

## §7 关联与关键词

[`226 批档`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md)（母批）／
[`227 批档`](227_0CW_S3_SCALE_ADJUDICATION_2026-10-07.md)（S3 规模裁决）／
[`207 全轮报告 §15`](../SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md)（勘正注记落点）／
[`208 批档`](208_0CV_DEEPSEEK_IFACE_ALIGN_2026-10-07.md)（入榜通道核证）／
[`215 批档`](215_SCB_COST_BREAKDOWN_2026-10-07.md)（花销报告先例）／
[`196 批档`](196_0CR_S1_HARBOR_CHAIN_AND_MANIFEST_FREEZE_2026-10-05.md)（harbor v0.23.0 pin）／
TB2.1 适配器先例 `tb_agents/orz.py`。

关键词：0cw S1、官方口径、Harbor 多步任务、gabeorlanski/slopcodebench、`[[steps]]`、
MultiStepTrial、resume_trajectory 默认关、每 checkpoint 全新进程＋新会话、2h 帽施加、
n_attempts 默认 1、seed N/A、连续模式开关、usage 轴 null、0cr 报告勘正、计数 55 不变、
索引 v4.201。
