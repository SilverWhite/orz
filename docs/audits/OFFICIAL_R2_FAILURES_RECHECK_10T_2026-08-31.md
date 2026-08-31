# 错题集 10 题小批复验记录（official-r2-failures-recheck-10t，2026-08-31）

> 日期：2026-08-31；批次：`official-r2-failures-recheck-10t`
> （harbor 实机，k=1，deepseek-v4-flash，`tb_agents.orz:Orz`，eval_browser=true）。
> 二进制：orz `f4f96eb8`（Linux musl 三件套，2026-08-31 13:39 构建，
> D:\tb-eval\orz-linux\orz，SHA256 `BECA36217CB5420D2DF426C5F7993B03942FD1D7417AA4EC9133C6E416AC7F5D`）。
> 配置：`scripts/configs/official-r2-failures-recheck-10t-config.json`；
> 结果：`D:\tb-eval\jobs-official\official-r2-failures-recheck-10t\result.json`。
> 状态：**小批复验完成，1/10 解出（rstan-to-pystan），框架健康度全绿，
> 三项待评估结论已登记（.gsa 写保护 / 环境探测前置 / 全量补跑）**。

## 1. 选样依据

自 2026-08-29 S4 失败归因（BACKLOG 0j / 设计 §9.5）的 31 题全量中取 10 题，
覆盖三类失败画像：

- **A 类框架错误终止（2）**：make-doom-for-mips、gcode-to-text——验证
  S5-1 fold 桥 reasoning_content 修复（不再 400）。
- **B1 web 检索时间黑洞（3，取最严重）**：mteb-leaderboard（单次 web_search
  1365s）、rstan-to-pystan（885s）、path-tracing（653s）——验证 web_search
  120s 超时修复。
- **B2 run_terminal 慢命令（2，取最慢）**：train-fasttext（1000s 训练）、
  adaptive-rejection-sampler（apt-get install R 458s）——验证 S5-2 终端分层
  超时 + 中间回报。
- **C 类自然结束交付失败（3）**：model-extraction-relu-logits、dna-insert、
  filter-js-from-html——模型真实能力边界基线。

## 2. 结果（10/10 完成，7 AgentTimeout，均值 0.100）

| 分类 | 题目 | Reward | 结束方式 | 对比 2026-08-29 基线 |
|---|---|---|---|---|
| B1 | rstan-to-pystan | **1.0** | 1800s 名义超时但交付完成 | B1 超时 → **解出** |
| A | make-doom-for-mips | 0.0 | 900s 墙钟超时，零真实 400 | A 类 400 崩 → B 类超时 |
| A | gcode-to-text | 0.0 | 900s 墙钟超时，零真实 400 | A 类 400 崩 → B 类超时 |
| B1 | mteb-leaderboard | 0.0 | 3600s 墙钟超时 | 仍超时 |
| B1 | path-tracing | 0.0 | 1800s 墙钟超时 | 仍超时 |
| B2 | train-fasttext | 0.0 | 3600s 墙钟超时 | 仍超时 |
| B2 | adaptive-rejection-sampler | 0.0 | 900s 墙钟超时 | 仍超时 |
| C | filter-js-from-html | 0.0 | 自然结束（4.7 min） | 模型能力边界（预期） |
| C | model-extraction-relu-logits | 0.0 | 自然结束（7.3 min） | 模型能力边界（预期） |
| C | dna-insert | 0.0 | 自然结束（6.1 min） | 模型能力边界（预期） |

## 3. 修复验证（全部实测生效）

1. **S5-1 fold 桥修复**：make-doom / gcode 事件链零真实 400（精确匹配
   `"400"` 与 http_status/status_code 均为 0 命中），从 A 类框架死亡转为
   普通墙钟超时——修复在实机消除 400 死锁，但两题未在官方限时内完成。
2. **S5-2 终端分层超时 + 中间回报**：train-fasttext 6 次「前台转后台」
   ↔ 事件链 6 条 `tool_running` 一一对应 + 1 次后台超期 kill
   （`tier_timeout=true`）；adaptive-rejection 300s 满触发中间回报 1 次。
   `foreground_block_budget_ms=300_000`、`auto_background_on_timeout=true`
   均为框架配置，模型面封闭（`is_background`/`&` 显式后台被拒绝并提示）。
3. **web_search 120s 超时**：全批 web_search 仅 5 次（mteb 4、rstan 1），
   无单次长时黑洞；模型主力转为 run_terminal_cmd + browser_read 直读
   （mteb 39 次 browser_read max 18.8s、rstan 10 次 max 2s）。
4. **事件链**：10/10 校验，6 条错误全部为「墙钟超时缺终止事件」（既有
   豁免模式），0 schema 违例、0 哈希链断裂、0 receipt 违例。

## 4. 框架整体与机械层流转（健康）

| 检查面 | 结果 |
|---|---|
| 事件链 | 10/10；仅墙钟超时缺终止事件（豁免），其余全绿 |
| ACAF 票据 | issued≈consumed（246/246、78/78、68/68…）；仅 gcode 1 次过期拒绝（正常路径） |
| 机械审计 | 10/10 零 anomaly |
| 策略拒绝 | 0 次 policy_denial，无拒绝风暴 |
| 门拒绝 | candidate_cap 2、retrieval_role_write_denied 1、is_background 拒绝 3、Background disabled 1——均为按设计拒绝，模型均继续推进 |
| 权限门 | 仅 mteb 2 次：read_file/grep 读 `.gsa/ledger` 被 access_in_scope 拒绝（正确 fail-closed） |
| 检索降级 | 每 run 恰 1 次 retrieval_mode_transition（正常） |
| orientation 软门 | mteb 4 次、其余 4 题各 1 次触发，无循环 |
| 400 / 框架崩溃 | 零真实 400、零 NonZeroAgentExitCode、零死锁 |
| 工具错误构成 | 大头为命令非零退出（exit_1/127，任务反馈）+ 模型幻觉工具名（fail-loud）+ 结构化浏览器错误（空页/坏 scheme）——均非框架缺陷 |

## 5. 观察项

### 5.1 模型间接动作：经终端读 `.gsa/ledger`（记录 + 风险排除）

mteb-leaderboard 模型于 11:07:09 尝试 read_file / 11:07:11 尝试 grep
`/app/.gsa/ledger/current.md`，均被 `access_in_scope` 拒绝（ReadOnly 读边界）；
11:07:14 改用 `run_terminal_cmd sed` 读到内容——`access_in_scope` 只拦
Read/Grep 类工具，终端通道不受该边界约束。

风险排查：
- ledger 内容仅「轮次 + 工具名 + 命令/URL 摘要 + 结果哈希 + 最终回复」，
  无任何密钥/凭据；全 `.gsa` 非 chrome 文件搜索
  `api_key/secret/bearer/permit/token` 零命中；
- keystore 为内存态（DPAPI unavailable → in-memory permit signer），
  `.gsa` 无密钥文件；
- 该行为属「模型用当前工具面工具做间接动作」，按 2026-08-31 用户裁决
  记录即可、不做过度处理。

### 5.2 模型幻觉工具名（fail-loud，不处理）

全批 10 个幻觉变体：`run_terminal_cpt / run_terminal_c句 / run_terminal_craft
/ run_terminal_cell / run_terminal_cplet`——框架 1–2ms 拒绝并提示，模型自行
回正，无一阻塞。按用户裁决不处理。

### 5.3 墙钟画像（支撑后续方向决策）

- 超时题轮数远未到 999 上限：mteb 228 轮/3600s（15.8s/轮）、make-doom
  93 轮/900s（9.7s/轮）、path-tracing 80 轮/1800s（22.5s/轮）——墙钟是
  瓶颈，每轮模型响应+执行本身消耗 10–22s；
- 命令阻塞型：adaptive-rejection 900s 仅 6 轮（apt 装 R 300s+ 转后台）、
  train-fasttext 52 轮中多次 300s 训练命令转后台——单命令墙钟是大头。

## 6. 待评估结论

### 6.1 `.gsa` 写/删保护：处理有副作用，建议不处理（登记为已知边界）

评估：
- 直接只读挂载不可行：运行时自身要向 `.gsa` 写 events.jsonl / ledger /
  session/terminal/*.log / chrome-profile，只读会破坏整个事件面；
- 命令级 hook 拦截（解析 shell 命令中的 `.gsa` 写）副作用明显：shell
  语义绕行空间大（cd + 相对路径、python/工具间接写、cp/tar 等），易产生
  「保护了但没保护住」的假安全感；实现复杂且回归风险高（终端行为变化
  影响现有测试面）；
- 现状已有兜底：事件链 verifier 会检出哈希断裂（事后检测），宿主侧
  harbor result.json 独立记录，评测数据损失不影响宿主机审计；
- 实际风险等级低：评测场景模型无动机破坏自身证据链（破坏即 reward 0）。

结论：**不引入写保护**。如未来要收紧，方向应是「运行时写区与模型面
隔离」的架构调整（读写分离卷），而非命令 hook。登记为已知边界。

### 6.2 机械层环境探测前置：收益中等偏弱，建议先单题 A/B 验证

用户设想：机械层唤醒前置自动完成环境探测（数据文件清单/依赖可用性/格式），
探测结果以极简形式随首轮请求注入。

严格评估（基于本批数据）：
- **收益证据（探索密集型）**：mteb-leaderboard ledger 实证前 20+ 轮全部为
  环境/数据探索（curl 探测 HF URL、下载 parquet、解析 language 字段格式），
  预置数据清单可省约 15–20 轮 ≈ 250–350s 墙钟（15.8s/轮）；
- **收益边界（命令阻塞型）**：adaptive-rejection / train-fasttext 的墙钟
  大头是 apt/训练命令本身，预探测仅能省 0–2 轮探索，不改变超时结论；
- **对通过率的预期**：6 个超时题根因是总工作量超出官方墙钟，省 10–20%
  轮次大概率不翻转结论（除恰好卡边界的题，无证据支持）；
- **成本**：通用探测协议需设计（探测什么、多任务覆盖、结果格式、注入面），
  探测本身须 <5–10s 否则得不偿失；探测结果注入首轮存在噪音/误导风险；
- **现有基础**：工具可用性探测（`tool_probe::probe_work_tools`）与检索
  能力探测（`retrieval_mode::probe_retrieval_capability`）已存在，但无
  「任务环境（数据/依赖）」前置探测。

结论：收益**中等偏弱**，不构成当前最高优先级。建议先用
mteb-leaderboard 单题 A/B（预置数据清单 vs 不预置）做最小验证，
用数据决定是否机械化，避免未验证的架构改动。

### 6.3 全量补跑与试次策略

- 单轮 k=1 波动大（rstan 1.0 含运气成分），但 2026-08-31 用户裁决：
  不考虑多次尝试（实际使用中用户不会多发同一请求博小波动）；
- 剩余 21 题全量补跑可闭环 W4-R4 S4 复验，是否执行待用户定。

## 7. 证据边界

- 事件链：`D:\tb-eval\gsa-volumes\official-r2-failures-recheck-10t\<uuid>\runs\RUN-CLI-*\events.jsonl`；
- verifier：`assurance/run_event_journal_validation.py`（validate_journal_file，
  10/10，6 条墙钟超时缺终止事件豁免）；
- 结果：`D:\tb-eval\jobs-official\official-r2-failures-recheck-10t\result.json`；
- 复验记录不修改设计、不改变计数；W4-R4 S4 复验条目由本次小批部分闭环。
