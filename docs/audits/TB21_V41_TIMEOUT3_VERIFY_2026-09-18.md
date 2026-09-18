# TB 2.1 三题超时验证轮记录（official-verify-timeout3，2026-09-18）

用户令：「先进一轮新的跑分测试吧，这一轮用来验证我们最近的修补和检索侧的优化对于超时问题究竟有没有一定程度的缓解；请选 tb 2.1 中超时情况最为严重三道题作为验证测试，看看具体情况。」

## §0 结论摘要

**缓解部分成立，但不足以翻转三题结局。** 检索侧优化（0ac S3 G1–G4 等）与近期修补在官方 harness 真机上逐项核证生效——web_search 单次最坏时延从 120 s 静默超时收敛到 ~34 s（引擎链 T_overall 兜底）、失败全部结构化快速返回、浏览器死车道零预算烧蚀、检索子代理零墙钟中途死亡、0z 资源快照事件首次在真机落账、优雅墙钟收尾生效。但 gpt2-codegolf / torch-pipeline-parallelism 两题仍用满 900 s 官方预算（检索墙钟占比 50–53%，gpt2 与 R0 基线 54% 基本持平——**单次检索变快了，模型的重试次数没有减少**）；extract-elf 则死于装置出网不稳下的 DeepSeek 传输 10 次重发耗尽（5.9 min 即终止，根本没到超时）。三题 reward 全 0，其中 torch-pipeline 的 verifier 自身因 PyPI 网络下载不了 torch（R0 同形），reward 轴被装置污染。

**主瓶颈定性**：本机容器出网质量是当前第一干扰源（DeepSeek API 读体错误、PyPI TLS EOF、Docker Hub 层 EOF 三类同窗口复现），其次是模型在降质网络下的检索重试习惯。检索侧机制面（截止、结构化、探针、优雅收尾）在本轮无一失效。

## §1 轮次身份与执行面

- 作业：`official-verify-timeout3`（单组 `-t900`，三题同 900 s 预算），起跑 20:19:07 / 结束 21:06:01（exit=0，含预拉 6.2 min）。
- 载体：**0.6.2**（`D:\tb-eval\orz-linux\orz` sha256 `d14d6d9c…`，与 0.6.2 发布件逐位一致）；适配器 `tb_agents/orz.py` sha256 `2737cfad…`＝R0 锁定值（零改动）。
- 口径：官方数据集 pin `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…`、`deepseek-v4-flash`、`-k 1 -n 1` 串行、`--upload --public`、无时间倍率；每题官方 agent 超时 `--ak max_wallclock=900` 透传（P0-2＋2026-09-02 口径收口）。
- 预拉：gpt2-codegolf / torch-pipeline 命中本地缓存；**extract-elf 3 次拉取失败（manifest EOF / 层下载停滞）后第 4 试成功（349 s）**——R0 硬发现 1「出网路径在大文件传输上不稳」的同窗口复现。
- 执行器：`scripts/run_r0_heavy_official.py`（`--tasks` 子集形态，零改动复用）；代际身份核验按设计放行新代际（`--allow-identity-drift`，新载体哈希已入轮次日志 plan 行）。

## §2 选题依据（全量聚合）

扫 `jobs-official` 全部 TB 2.1 作业（79 作业 / 271 试次）按 `exception_info.exception_type = AgentTimeoutError` 聚合，超时最严重且检索归因成立的三题：

| 题 | 试次 | AgentTimeout | 比率 | 检索归因证据 |
|---|---|---|---|---|
| gpt2-codegolf | 10 | 8 | 80% | R0：56% 工具调用为检索、web_search 占 900 s 预算 54%（488 s） |
| adaptive-rejection-sampler | 10 | 7 | 70% | R4 已 891 s 自然完成（未超时；超时主因非检索）→ 不选 |
| torch-pipeline-parallelism | 7 | 6 | 86% | R0：68% 调用为检索、占预算 **91%**（817/900）——检索主导第一名 |
| extract-elf | 7 | 6 | 86% | R4：15 次失败网络目标（github×7/huggingface×5）、163 s 错误墙 |

`make-doom-for-mips`（6/11）与 `dna-assembly`（6/7，1800 s 档）检索归因弱于上表三题或预算翻倍，未入选。

## §3 逐题读数（journal 一手证据）

| 读数 | torch-pipeline-parallelism | gpt2-codegolf | extract-elf |
|---|---|---|---|
| 账面结局 | AgentTimeout（900 s 用满）＋ `run_invalidated{wallclock}` 优雅收尾 | AgentTimeout（900 s 用满） | **NonZeroAgentExitCodeError**（5.9 min 即终止） |
| reward | 0（**verifier 自身 PyPI 下载 torch 失败**：TLS handshake EOF，测试未跑成） | 0（`/app/gpt2.c` 未产出） | 0 |
| 模型轮 / 工具调用 | 39 轮 / 69 完成 | 27 轮 / 49 完成 | 11 轮 / 20 完成 |
| web_search | 22 次 437.5 s（均值 19.9 s、最坏 33.8 s；15/22 成功） | 23 次 451.4 s（均值 19.6 s、最坏 36.0 s；19/23 成功） | 4 次 82.3 s（3/4 成功） |
| web_fetch | 21 次 29.3 s（均值 1.4 s；19/21 成功） | 18 次 25.2 s（15/18 成功） | 4 次 29.4 s（2/4 成功） |
| 检索墙钟占 900 s 预算 | 51.9%（web_search 单算 48.6%） | 53.0%（web_search 单算 50.2%） | 12.4%（提前死亡） |
| 终端工作量 | 14 次 1.0 s（未进入实际解题） | 4 次 0.0 s（末段 2 次被资源门 deny） | 11 次 1.1 s |
| 提前死亡根因 | — | — | `transport_retry{outcome:exhausted, kind:zero_chunk, retries:10}` → `run_failed`：DeepSeek `/chat/completions` 连续 10 次重发无 chunk |

web_search 单次时延分布与 0ac G1 计时语义逐簇吻合：≤5 s 快速成功簇、~10 s 首结果截止簇、28–34 s 整链 T_overall 兜底簇——**没有一次 120 s 量级的调用**。

## §4 与基线对比

| 维度 | 基线（R0＝0.5.0／R4＝0.4.0） | 本轮（0.6.2） | 判定 |
|---|---|---|---|
| web_search 单次最坏 | 120 s 静默超时（R4 逐字 `timed out after 120.0s`）；慢成功 46–79 s | 最坏 36.0 s，时延三簇清晰 | **缓解 ≈3.5×（尾部）** |
| 检索失败形态 | 静默挂起 / 无 cause | 全部 `status:error`＋结构化 cause（`execution_failed` 等） | **成立** |
| 浏览器死车道 | R0：14 次 launch failure、惰性按 dispatch 重复 | 每题 1–3 次 `browser_not_found` 后 `browser_control` 快速失败 `capability_unreachable`，墙钟 0.0 s | **成立** |
| 检索子代理烧预算 | R0：5 次 `subagent_wallclock_timeout_mid_tool` | **0 次**，close ticket／close record 干净收口 | **成立** |
| 0z 资源快照事件 | R0：13 次 WARN、journal 0 事件（audit-face loss） | 三 run 共 8 条 `host_resource_snapshot` 正常落账 | **成立（GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP 真机验证）** |
| 超时后孤儿 | R0：agent 超时后 orz 不停、与 verifier 抢容器（一次已通过未记账） | torch-pipeline `run_invalidated{status:wallclock}` 预算内自收尾 | **成立（P0-2 透传口径真机首验）** |
| 检索预算占比 | torch-pipeline 91%／gpt2 54%（web_search 单算） | 48.6%／50.2% | **torch-pipeline 大幅下降；gpt2 持平** |
| 三题结局 | R0：两题 AgentTimeout＋verifier 网络污染 | 全部 reward 0（两超时＋一传输死亡） | **未翻转** |

## §5 残余与新观察

1. **模型重试习惯未变**：降质网络下单次检索已快且结构化，但模型把「失败→再搜」循环执行到预算耗尽（gpt2 23 次 web_search 仅 4 次差成功……成功率 19/23 并不低，问题在 27 轮里几乎全部轮次花在检索而非解题——两题终端墙钟合计仅 ~1 s）。
2. **装置出网是第一干扰源**：同窗口三类中断（DeepSeek API 读体/发送错误 ×7＋传输 10 次重发耗尽 ×1、PyPI TLS EOF、Docker Hub manifest/层 EOF ×3）。extract-elf 的「非超时死亡」与 torch-pipeline 的 verifier 污染皆源于此。
3. **资源门 deny 信封真机首见**（gpt2 末段 2 次）：`commit_free 858 MB / limit 6.27 GB` 读数齐全 fail-closed——机制正确；但 8 GB Docker VM 形态下 6.27 GB commit 上限对「末段才动手装依赖」的模型路径是否阈值过紧，留观察不作结论。
4. extract-elf 轮预算仅用 7/999 轮、11 模型轮即死于传输，检索行为本身健康（4+4 次调用，占比 31.8%）。

## §6 后续建议（留裁决，不立项）

- 官方批次前给容器出网做**预检＋代理通道**（DOCKER_PROXY_RECIPE 第 0 步「预检优先」先例）；DeepSeek API 走代理稳定通道可同时救传输死亡与检索质量。
- 「检索重试降速」类提示词/机械纪律（如同题连续 N 次检索失败后强制转入本地路径）可作为下一缓解杠杆——本轮证据表明瓶颈已从「单次慢」转移到「次数多」。
- torch-pipeline 的 verifier 依赖现下 825 MB torch，装置网络不修复则该题 reward 轴持续不可用（R0 已登记同形）。

## §7 证据物

- 轮次日志：`D:\tb-eval\jobs-official\official-verify-timeout3-round.log`（plan 行含载体/适配器双哈希）
- 作业产物：`D:\tb-eval\jobs-official\official-verify-timeout3\<trial>\`（result.json／exception.txt／verifier/）
- 三 run journal：`D:\tb-eval\gsa-volumes\official-verify-timeout3\<session8>\runs\RUN-CLI-*\events.jsonl`（RUN-CLI-6aad2cb7 ／ RUN-CLI-6aad31a5 ／ RUN-CLI-6aad35bc）
- 预拉日志：`D:\tb-eval\jobs-official\preroll-images.log`

## §8 增补：第二轮直连对照（r2，同日 2026-09-18）

用户补充事实：r1 起跑时**宿主代理未关**（且该代理不稳定）；指示「用 http 直达也行，暂不挂浏览器，已关闭代理，请重试」。

### §8.1 环境修复记录（起跑前置）

- 关代理后容器内仍全断（TLS unexpected eof）→ 分流定位：**宿主直连全通**（api.deepseek.com 401@0.1s／pypi 200@0.8s），唯容器断 ⇒ 不是网络本身。
- 根因：**Docker Desktop 代理设置为 `manual` 模式硬编码 `http://127.0.0.1:7890`**（settings-store.json `ProxyHTTPMode`），代理软件关闭后容器流量仍被推向死端口（plain HTTP 报 `connectex: actively refused` 坐实）。
- 修复：`ProxyHTTPMode: manual → system`（清空 Override；备份 `settings-store.json.bak-20260918`）＋**彻底重启**（首次仅杀 UI 未生效——wget 黑洞式挂起；二次连同 `com.docker.backend.exe` 与 `docker-desktop` WSL VM 全停后重启才生效）。
- 修复后金丝雀：容器内 api.deepseek.com **401 ✓**／pypi **200 ✓**；registry-1.docker.io 容器与宿主**均直连超时**＝本网络对 Docker Hub 的固有限制（与代理无关）。三题镜像 r1 已本地化 ⇒ r2 以 `--no-pre-pull` 起跑。

### §8.2 r2 逐题读数（job `official-verify-timeout3-r2`，21:46–23:19）

**勘误（同日自查）**：本表初稿把 torch-pipeline 与 extract-elf 两行读数标反（journal 归属误用 glob 排序序号而非 run id；串行跑批时后两卷尚未落盘）。下表为按 run id 铁证（`6aad40db`/`6aad4806`/`6aad4f4d`，与各 trajectory session_id 互证）重写的正确归属。

| 读数 | torch-pipeline（`6aad40db`） | gpt2-codegolf（`6aad4806`） | extract-elf（`6aad4f4d`） |
|---|---|---|---|
| 结局 | AgentTimeout（`run_invalidated{wallclock}`，2 命令在飞） | AgentTimeout（`run_invalidated{wallclock}`，1 在飞） | AgentTimeout（`run_invalidated{wallclock}`，2 在飞） |
| 模型轮 / 完成调用 | 38 / 49 | 22 / 49 | 46 / 92 |
| web_search | **4 次 87.6s（9.7%）** | 20 次 410.6s（45.6%）**20/20 全部成功返回** | 21 次 362.1s（40.2%） |
| web_fetch | 3 次 27.8s | 16 次 95.0s（5 failed） | 15 次 154.7s（5 failed） |
| 检索合计占 900s | 12.8% | 56.2% | **57.4%** |
| 终端真实工作量 | **40 次 566.2s（Python 环境自举）** | 9 次 13.3s | 17 次 2.0s |
| 传输重试耗尽 | 0 | 0 | **0（r1 死因消失）** |
| reward | 无（verifier 卡 torch 825MB 下载） | 无（verifier 卡 cpython 下载） | 无（verifier 卡 cpython 下载） |

### §8.3 r1→r2 结论修订（按正确归属）

1. **r1 的网络失败面主因＝不稳代理，实锤**：直连后三题 `transport_retry` 全零（r1 extract-elf 10 次重发耗尽致死消失）、DeepSeek API 全程稳定、检索失败仅剩零星结构化错误。
2. **r1「检索占比 50%＋」在 torch-pipeline 上确系网络诱发**：r1 22 次检索（52%）→ r2 4 次（9.7%），agent 转向实干——但该题镜像**无 Python**，566s 全花在环境自举（apt→uv→micromamba→**手工 `apt-get download`＋`dpkg -x` 拼出 python3.12**→get-pip），超时死在装环境，非解题。
3. **gpt2 检索习惯实锤（与网络无关）**：r2 直连下 20 次 web_search **全部成功返回**仍继续搜（检索+fetch 合计 56.2%），无一次框架面失败——见 §9.2 定性。
4. **extract-elf 形态翻转但未转为解题**：r1 5.9 分钟传输早死 → r2 全程 900s，前段真做了 a.out ELF 分析（readelf/objdump/python 原型），随后陷入「找参照实现/翻测试件」检索螺旋（21+15 次检索、57.4% 墙钟；甚至读自己的 journal 与会话日志找线索），wallclock 收尾时仍未写 extract.js。
5. **reward 轴在本网络环境不可读**：三 verifier 均卡死在依赖下载（torch 825MB／cpython 32MB），reward.txt 全缺——官方通过率轴需镜像内预置依赖或快网络，装置侧待解。
6. 机制面两轮零失效：优雅收尾 3/3、结构化失败、`capability_unreachable` 快速失败、0z 快照落账、零子代理墙钟死亡、零传输死亡。

### §8.4 r2 证据物

- 轮次日志：`D:\tb-eval\jobs-official\official-verify-timeout3-r2-round.log`
- 作业产物：`D:\tb-eval\jobs-official\official-verify-timeout3-r2\<trial>\`
- 三 run journal：`D:\tb-eval\gsa-volumes\official-verify-timeout3-r2\<session8>\runs\`（torch-pipeline=`81ef3950…`／gpt2=`26662849…`／extract-elf=`06699fe6…`）

## §9 增补：外置任务评价＋两项用户裁决（2026-09-18）

官方 verifier 因网络未产出 reward，主会话按各题 `tests/test_outputs.py` 判分口径＋trajectory／journal／终端日志做外置评价（用户令「需要知道完成水平和进度如何」）。

### §9.1 完成水平（r2 为准；r1 无一产出生成，同为 0%）

| 题 | 判分物 | r2 状态 | 判定 | 进度叙述 |
|---|---|---|---|---|
| extract-elf | `/app/extract.js` | **未创建** | **0%**（`test_extract_js_exists` 即不过） | 前段真做 ELF 分析（readelf/objdump/python3 原型解析 a.out），后段陷入找「参照实现」的检索螺旋（21 搜索＋15 fetch、57.4% 墙钟；甚至翻自己的 journal／会话日志找线索），wallclock 收尾时文件仍未落 |
| gpt2-codegolf | `/app/gpt2.c`（<5000 B，gcc 可编） | **未创建** | **0%**（首测试即不过；gcc 在镜像内、写作可行） | 探了 ckpt 结构（xxd 头尾）与工具链，随后全程搜参照实现（20 搜索全成功仍继续搜）；一个值得注意的对照＝gpt2 里程碑 TarGM 之类开源参考确实是模型想找的锚 |
| torch-pipeline-parallelism | `/app/pipeline_parallel.py` | **未创建** | **0%**（首测试即不过） | **镜像无 Python**——38 轮全部耗在环境自举（apt→uv→micromamba→手工 `apt-get download`＋`dpkg -x` 拼出 python3.12→get-pip），wallclock 死在装 pip；非检索问题、非模型懈怠，环境＋慢直连是直接死因 |

历史对照（全量 271 试次聚合）：三题本就是 TB 2.1 未解池成员（gpt2 历史 0/10 记分、extract-elf 0/7 记分），本轮 0% 与历史一致——**这两轮验证的是「超时机制」不是「解题能力」**，选题目的即在于此。

### §9.2 gpt2 定性：模型习惯，非框架缺口（用户裁决：不干预）

用户裁决（2026-09-18）：「gpt2 这个问题，只要不是框架缺口，就不进行任何干预。」主会话按 r2 证据链定性：

- **框架面零故障证据**：20/20 web_search 全部成功返回结构化结果（`retrieval_result_committed` 在账）、零静默失败、零传输重试、错误全部结构化带 cause、优雅收尾生效——模型每次都「拿到了」才决定再搜。
- **行为面证据**：查询串全部是找参照实现/现成解（`"extract memory values..."`、github/CSAPP 式关键词），拿到结果后换词再搜——是「先找现成答案再动手」的任务approach，不是对失败反馈的应激重试。
- **先前裁决覆盖**：FP-2（检索失败不抑制、模型自主）为既有用户裁决，r2 行为在其范围内。
- **结论：非框架缺口 → 按裁决不做任何干预**。若未来载体检索面出现故障形态（结果未达、静默挂起），届时按框架缺口另立案。prompt 等说明类内容嫌疑另经 §9.5 全量排查排除（用户追加令）。

### §9.3 网络事项登记（用户提供）

r1 起跑时宿主代理未关且代理商确认「暂时未修复完成」；用户正在更换机场（新代理商），当前以直连口径运行。**影响**：直连下 DeepSeek API／PyPI 可用但外域大文件下载慢（verifier 依赖下载卡死、torch-pipeline 环境自举拖满预算），Docker Hub 直连不可达（镜像须预拉本地化）。**后续官方跑批建议待新机场就绪后起跑**，并把「代理在位且稳定」加入起跑前检查（金丝雀：容器内 DeepSeek＋PyPI＋registry 三点测）。

### §9.4 本节证据物

- trajectory：`D:\tb-eval\jobs-official\official-verify-timeout3-r2\<trial>\agent\trajectory.json`（extract-elf 47 步／gpt2 23 步／torch-pipeline 39 步）
- 终端日志：`D:\tb-eval\gsa-volumes\official-verify-timeout3-r2\<session8>\session\terminal\`
- 判分口径：`D:\tb-eval\terminal-bench-2-1\tasks\<task>\tests\test_outputs.py`（extract-elf 参照实现内嵌可复刻）

### §9.5 增补：模型面说明性内容全量排查（用户令「prompt 等说明类内容的嫌疑也要排除」；按 0.6.2 载体源码 `08ab194c` 逐面核对）

gpt2 定性要立得住，须排除「框架说明文字在怂恿检索」。模型可见面全量清单与核对结论：

| # | 模型面 | 0.6.2 实文（要点） | 检索怂恿？ |
|---|---|---|---|
| 1 | 主系统提示词 `BASE_SYSTEM_PROMPT` | **全空字符串**（THIN-HARNESS-REDESIGN-V2 §9.1，2026-08-29 用户裁决：模型可见面＝任务指令＋工具列表＋注入块，契约全在工具描述/信封/机械门） | 无（面不存在） |
| 2 | 计划型框架块 `PLAN_FIRST_FRAMEWORK_BLOCK` | **勘误（2026-09-19 复核）**：初稿记「无条件注入系统提示词」有误——注入点被 `plan_first_enabled` 开关管住，该开关生产默认 false（controller.rs:922），orz-bin:1195／acp_server.rs:1856 明示「plan_first 仅作休眠开关保留（测试/回退）」⇒ **r2 生产车道模型从未见过此块**；文本本身亦零检索字眼（4 条执行风格，第 1 条指向本地阅读） | 无（面不存在——生产休眠未注入） |
| 3 | 工具描述·`web_search` | "Search the web for up-to-date information, tailored for coding and software development tasks."——能力描述句（「为编码任务定制」是描述适配面，非使用指令；此类工具的标准描述形态） | **唯一轻微相关句**，定性＝描述性非指令性，不构成驱动 |
| 4 | 工具描述·`web_fetch` | "Fetch the content of a specific URL... WILL FAIL for authenticated or private URLs..."——中性＋失败警示 | 无 |
| 5 | 工具描述·其余主面（read_file/grep/blackboard_read/plan_write/submit/context_compress 等） | 纯机械契约（分区枚举、锚点、两阶段、步门）；`blackboard_write` 描述仍含 **RS-03 的 920K 残留句**（上下文陈旧文案，与检索无关） | 无（RS-03 照旧在案） |
| 6 | 机械注入块全集（反例门／周期 orientation／初始轮三问／SESSION 面／F6 墙钟档／`[TOOL_POLICY_BREAKER]`／`[工作台]` 首轮提醒） | 三问全自检式（「实际交付什么／什么阶段下一步／做法优劣」）；**`[TOOL_POLICY_BREAKER]` 是反重试向**（「请勿继续调用该工具；请切换策略」）；`[工作台]` 提醒只推黑板写入（含 RS-03 第二处 920K 残留，检索中性） | 无（且有反向抑制块） |
| 7 | 检索子代理系统提示（主模型不可见） | 双车道契约「[车道:本地浏览器检索\|推荐首选]…web_search＝原生道（高价值结论才 web_fetch 核验）」——浏览器优先 | 无（且只作用于子代理） |
| 8 | 工具结果信封/输出头 | "Web search results for: …"、结构化错误信封；R4 审计已扫「零勿重试类教学文案」——本轮补扫确认也**零鼓励重试文案** | 无 |

**结论**：框架说明性内容嫌疑**排除**——主提示词为空、框架块生产休眠未注入（见上勘误）、工具描述为能力描述非使用指令、机械块中性或反重试；生产在场的约束性文字实际仅 `[工作台]` 首轮提醒一条（与检索无关）。gpt2 的「找参照实现」检索习惯（20/20 搜索成功仍继续搜）没有任何框架文字驱动，§9.2「模型习惯、不干预」的定性**维持成立**。附带产物：RS-03 两处 920K 残留（`blackboard_write` 描述＋`[工作台]` 首轮提醒）在 0.6.2 仍在，与既有立项一致。

### §9.6 增补：约束性内容清零裁决收束（2026-09-19，零代码）

用户先令「除中立问询外，一切提示和约束性内容全部清零，仅剩模型必需的使用框架所需的注解」，经主会话提交残量清单（生产在场仅 `[工作台]` 首轮提醒＋DP-2 条件提醒；休眠面框架块；边界项 BREAKER／预算耗尽通知）与回归风险（0ae D0 教训：黑板写入可能正由提醒驱动）后，用户裁决收束：

1. **plan-first 机制维持休眠即可**——不正式退役、不清休眠文本（框架块随开关留在树上）。
2. **`[工作台]` plan 写入提醒保留**（0ae D1 设计维持；DP-2 补救提醒同族保留）。
3. `[TOOL_POLICY_BREAKER]` 防跑飞护栏保留（随「无需额外处理」默示通过主会话建议）。
4. **920K 残留维持 RS-03 在案、暂不动**（随 0aq 批文案修正）。

**净效果：无清零批次、零代码、计数不变**；gpt2 定性（模型习惯、不干预）与 §9.5 排查结论均维持。

## §10 增补：第三轮直连复跑（r3）＋检索失败面三轮横比（2026-09-19）

用户令：「请检查当前最新运行的三道检索类题目的运行情况和结果」「现在容器内是 http 直连形态，
检索失败的次数多吗？请对三轮测试的内容进行分析，寻找摩擦和问题」。本节落 r3 读数与三轮横比，
摩擦面只登记观察、不立项、不动计数（是否转 RS/0aq 待裁决）。

### §10.1 轮次身份与执行面

- 作业：`official-verify-timeout3-r3`，起跑 2026-09-19 00:43:39 / 结束 01:30:48（46m42s，exit=0）；
  Harbor Hub 公开上传（job `801dd13e-105b-408b-835e-e4afe0dc9df4`）。
- 口径与 r1/r2 逐项一致：载体 **0.6.2**（`d14d6d9c…`）、适配器 `2737cfad…`（R0 锁定值）、官方 pin
  `7d7bdc1c…`、`deepseek-v4-flash`、k=1 串行、`--ak max_wallclock=900` 透传、`n_retries=0`。
  三题镜像已本地化（preroll 0.1 min 全 OK，`--no-pre-pull`）。
- **直连形态坐实**：三 run 的 `tool_availability_check.retrieval_family` 均为
  `web_search local_segmented=off chain_detail="bing_cn; proxy=off"`。
- 代际身份检查照旧报 `carrier DRIFT / adapter OK` 并显式放行（三轮同形，非本轮新事）。

### §10.2 总结果（三试次）

| 题 | 结局 | reward | agent 墙钟 | verifier 墙钟 | 判分物 |
|---|---|---|---|---|---|
| extract-elf | 正常完成（`run_finished{completed}`） | **1.0（2/2 通过）** | 408 s | 16 s | `/app/extract.js` ✓ |
| gpt2-codegolf | AgentTimeout（`run_invalidated{wallclock}`） | 0 | 900 s | 32 s | `/app/gpt2.c` ✗ |
| torch-pipeline-parallelism | AgentTimeout（`run_invalidated{wallclock}`） | 0 | 900 s | 363 s | `/app/pipeline_parallel.py` ✗ |

批次：mean **0.333**（1.0×1、0.0×2）、`AgentTimeoutError`×2、`n_errored_trials=2`。
三轮累计 9 试次中**首次产出判分物**（且是同批唯一一次）。

### §10.3 逐题读数（journal／trajectory 一手）

**extract-elf**（`RUN-CLI-6aad7388` / session `c1b272d4…`）：30 工具轮、43 段模型输出、
6m48s 收工；run_terminal_cmd 25（5.3 s）、web_search 5（92.7 s，max 35.3 s，**0 失败**）、
web_fetch 7（3.6 s，**0 失败**）、search_replace 3、submit 2。动作链：`readelf/objdump` 解析
`a.out` → 少量检索 → 命中公网 explorer 页（该页**连带给出本题 `test_outputs.py` 与内嵌 REF 实现**，
落 `/app/.gsa/session/web_fetch/2.md`）→ 据此复刻 `/app/extract.js`（ELF32/64 头＋节表解析、
`.text/.data/.rodata` 按 4 字节小端取值、原样 `sh_addr` 无基址）→ `/tmp` 自检（少样本 108/108 一致）
→ `chmod` → submit。两点定性注记：①**通过路径含评测泄漏成分**（判分物可从公网检索获得），
官方 1.0 成立但自主解题成分须打折；②中途读同批**前序 run 的 journal**（见 §10.5 F1）。

**gpt2-codegolf**（`RUN-CLI-6aad6fad` / session `445f1ff2…`）：28 模型轮、50 工具调用、
`run_invalidated{wallclock}`；**本轮零检索**（三轮九试次中唯一一次 web_search=0），
29 次 run_terminal_cmd（229.6 s，max 177.8 s）＋20 次 search_replace（全部落在
`/app/probe.c`、`/app/probe2.c`、`/app/ref.c`）。终端日志显示其真实完成了 ckpt 解析、前向推理、
NLL／生成取样、embedding 与 WPE 余弦校验等实证工作，但**到点仍未把交付文件写到题目要求的
`/app/gpt2.c`**（工具墙钟仅 230 s，余约 670 s 为模型侧）。伴随 7 次 `host_resource_denied`
（17:09:07–17:11:17Z，commit headroom 13.5–13.7 % < 25 %，tier soft，`Nothing was started`）。
该形态把 §9.2 的定性边界从「找参照实现的检索习惯」扩展到「**不检索也会不交付**」——两次失败
路径不同、终点相同，仍属模型侧习惯面。

**torch-pipeline-parallelism**（`RUN-CLI-6aad6aa2` / session `4f65c2ea…`）：`/app` 为空、
镜像内 `python: command not found`（3 次 run_terminal_cmd 全败：1×`command_exit_127`、
2×`retrieval_role_write_denied`）；21 次 web_search 发起／19 完成（427.6 s，max 29.8 s，**0 失败**）
＋16 次 web_fetch（9.1 s，**0 失败**），查询串全部指向 `train_step_pipeline_afab` /
「all-forward-all-backward」的**现成参照实现**（DeepSpeed、PyTorch PiPPy 源码与文档），
未动笔写文件。verifier 本轮**成功装齐依赖**（cpython 3.13 32 MB、torch 825 MB 及 CUDA 轮子等，
363 s 内完成）→ 4 项测试全红（文件不存在）→ **reward 轴恢复可读的 0**（r2 的「无 reward」是
verifier 自身 900 s 卡死下载，非本题真实读数）。

### §10.4 检索失败面三轮横比（回答「直连形态下失败多不多」）

口径：计数取 `tool_completed`（发起但未完成者＝收尾时在飞，单列）；失败＝`exit_code≠0`。

| 轮次（网络形态） | web_search 完成/失败 | web_fetch 完成/失败 | browser 尝试/失败 | 合计失败率 | 非 browser 失败率 |
|---|---|---|---|---|---|
| r1（代理在位、不稳） | 49 / **12** | 43 / **7** | 6 / **6** | 25/98＝25.5 % | 19/92＝20.7 % |
| r2（去代理直连） | 45 / **0** | 34 / **13** | 5 / **5** | 18/84＝21.4 % | 13/79＝16.5 % |
| r3（直连＋镜像本地化） | 24 / **0** | 23 / **0** | 3 / **3** | 3/50＝**6.0 %** | 0/47＝**0 %** |

- **web_search 连续两轮零失败**（r2 45 次、r3 24 次全部成功返回）。r1 的 12 次失败与
  `transport_retry`（r1 三 run 分别 2／2／3 事件，含 extract-elf 一次 10 连重发耗尽）
  同源，主因已实锤为不稳代理，直连后归零。
- **web_fetch 在 r3 也归零**（r2 尚有 12 次 `execution_failed`＋1 次
  `web_fetch_candidate_cap_exceeded`，多集中在外域大件）；单次最坏从 r2 的 60.0 s 降到 1.8 s。
- **唯一恒败面＝浏览器车道**：三轮 14／14 失败，全部 `browser_not_found`→
  `capability_unreachable`，墙钟 0.0 s（快速失败、零预算烧蚀，与 R0 的 14 次 launch failure
  同族但已不重复惰性派发）。
- 结论：**直连形态下检索失败次数已经很少**——r3 非浏览器检索 47 次调用零失败，总失败率 6 % 且
  全部来自设计内恒死的浏览器车道。当前瓶颈不在「检索失败」，而在**检索投量与转化**
  （r3 torch：21 次搜索全成功、占预算 47.5 %，仍无任何交付物）。

### §10.5 摩擦与问题（观察级；不立项、不计数，处置留裁决）

| # | 摩擦 | 证据（三轮） | 建议去向 |
|---|---|---|---|
| F1 | **跨 run 隔离缺口**：同批串行三题共用 `/orz-gsa` 卷，后跑的 run 可读前序 run 的完整 journal | 三轮 100 % 复现，均由末位 extract-elf 触发（r1→`2abd7dc7`/`8bb5f16d`；r2→`81ef3950`/`26662849`；r3→`445f1ff2`/`4f65c2ea`）；r3 实测 `grep` 前序 run 的 `events.jsonl` 找线索 | **已处置（§10.7）**：容器启动前把兄弟试次移出挂载根，冒烟取证零引用 |
| F2 | **评测泄漏**：本题判分物可经公网检索获得 | r3 extract-elf 命中单页即拿到 `test_outputs.py`＋内嵌 REF，据此复刻得 1.0 | 建议判分物不入公网可检索面，或对该来源记注／阻断 |
| F3 | **浏览器车道恒死且探针脱同步** | 三轮 14 次全败；同期探针恒报 `retrieval_family.browser.present=true` | **维持现状（§10.8 用户裁决）**：按官方要求，不在 orz 做环境提前补强（不启用 `eval_browser`）；探针语义按「工具在位≠可执行件在位」记注 |
| F4 | **资源门 deny 常态化** | 三轮共 29 次：r1 3／r2 12（torch 10）／r3 14（gpt2 7＋extract 7）；均为 commit headroom 8.7–13.7 % < 25 %，tier soft | 随资源批评估 6.27 GB commit 上限与 8 GB VM 形态的匹配 |
| F5 | **任务镜像与 verifier 环境不对称** | torch-pipeline agent 容器无 Python（`command not found`、`/app` 空），verifier 却用 uv 现装 cpython3.13＋torch | **维持现状（§10.8 用户裁决）**：官方镜像与 `allow_internet=true` 即题目环境，预置＝漂移 |
| F6 | **检索车道写盘被拒** | r3 torch 2 次 `curl -o /tmp/…` 被 `retrieval_role_write_denied` 拒（设计内 deny），模型改走 web_fetch | 记录即可；若需大文件落地核验再议 |
| F7 | **web_fetch 候选上限** | r1 gpt2 ×2、r2 extract ×1 `web_fetch_candidate_cap_exceeded` | 既有机制，留观察 |
| F8 | **交付物落盘纪律（模型侧）** | 三轮 9 试次仅 1 次产出判分物；r3 gpt2 20 次编辑全落 `probe/ref` 文件、到点未写 `/app/gpt2.c`；r2 gpt2/extract、r3 torch 均为「找参照实现」检索螺旋 | 沿 §9.2 用户裁决维持不干预；证据面已扩展到「不检索亦不交付」 |
| F9 | **verifier 依赖下载**（已缓解） | r2 三 verifier 各 900 s 卡死、reward 缺；r3 全部预算内完成（16／32／363 s） | 装置面观测；随网络环境再评 |

边界：本节取证部分为只读取证＋外部评价，零子仓改动、计数不变；F1 经用户令「F1必须要处理」
已当日处置（§10.7，改装置侧适配器、不动任务镜像）；F2–F9 仍为观察项，是否转 0aq／RS 项待裁决。

### §10.6 证据物

- 轮次日志：`D:\tb-eval\jobs-official\official-verify-timeout3-r3-round.log`（plan 行含载体／适配器双哈希）
- 作业产物：`D:\tb-eval\jobs-official\official-verify-timeout3-r3\<trial>\`（result.json／verifier\reward.txt／verifier\ctrf.json）
- 三 run journal：`D:\tb-eval\gsa-volumes\official-verify-timeout3-r3\<session8>\runs\`（torch=`6aad6aa2`／gpt2=`6aad6fad`／extract-elf=`6aad7388`）
- trajectory：`D:\tb-eval\jobs-official\official-verify-timeout3-r3\<trial>\agent\trajectory.json`
- 泄漏面取样：`D:\tb-eval\gsa-volumes\official-verify-timeout3-r3\c1b272d4-…\session\web_fetch\2.md`
- Harbor Hub：`https://hub.harborframework.com/jobs/801dd13e-105b-408b-835e-e4afe0dc9df4`

### §10.7 F1 处置：跨 run 隔离缺口（2026-09-19，装置侧修复＋冒烟取证）

**根因**：harbor 的 `--mounts` 是**作业级**参数（`harbor/jobs.py`），一个作业的全部试次
共享同一宿主编译卷并整卷 bind 到 `/orz-gsa`；适配器只在容器内把 `$PWD/.gsa` 指向
`/orz-gsa/<本试次 uuid>` 子目录，**兄弟试次目录仍留在挂载根内**，而容器内 agent 以 root
运行（权限位拦不住）⇒ 后跑的试次可读前序试次的 `runs/`、`session/`、`terminal/`、`web_fetch/`。

**修法（`tb_agents/orz.py`，仅装置侧）**：新增 `gsa_isolate_trials`（默认 true，可用
`--ak gsa_isolate_trials=false` 关闭）；容器启动前把挂载根内**非本试次**的条目
**移出**挂载根到 `<vol_root>/.quarantine/<job>/<entry>`（移动而非删除，宿主侧证据保留；
14 天龄期清理同步覆盖 quarantine；目标重名时加毫秒后缀）。默认假设串行试次（与本 runner
`-n 1` 一致）；并发跑批须显式关闭，否则会误伤在飞试次。文件头 docstring 的「subdir 即隔离」
表述已就地更正。

**代际身份影响**：适配器哈希 `2737cfad…`（R0 锁定值）→ **`fdd161d4…`**，后续跑批的代际身份行
将出现 `adapter DRIFT`（`--allow-identity-drift` 显式放行）——这是本处置的**预期**结果；
r1–r3 已落读数仍属旧适配器口径，不回溯改写。

**冒烟取证**（作业 `f1-isolation-smoke`，gpt2-codegolf＋extract-elf，`max_wallclock=60`、
`-k 1 -n 1`、**未上传**、4m20s、两试次零 exception 优雅收尾）：

| 判据 | 结果 |
|---|---|
| 卷根（＝容器内 `/orz-gsa`）条目 | 仅末位试次 1 条（修复前 r3 为 3 条） |
| quarantine 收纳 | 前序试次 1 条，journal 完整（含 `run_invalidated{wallclock}`） |
| 末位试次容器可见面＋其作业产物对兄弟 session id 的引用 | **0 次**（全库扫描） |
| 对照：r3 末位试次同口径扫描 | **6 次** |
| 单元级检查 | 兄弟移出、本试次保留、二次调用幂等；`_parse_bool` 四例符合预期 |

**证据路径口径变化**（新跑批起）：先跑的试次落在
`gsa-volumes/.quarantine/<job>/<session8>/`，只有末位试次留在 `gsa-volumes/<job>/`；
回查时两处都要看。**边界**：只动适配器与卷布局，不动任务镜像、不动 verifier、
不改官方数据集 pin。

**冒烟证据物**：`D:\tb-eval\jobs-smoke\f1-isolation-smoke\`（作业产物；已移出
`jobs-official` 以免污染官方全量聚合口径）、`D:\tb-eval\gsa-volumes\f1-isolation-smoke\`
（末位试次＝extract-elf `ec64ebda…`）、`D:\tb-eval\gsa-volumes\.quarantine\f1-isolation-smoke\`
（前序试次＝gpt2 `95df6f2d…`，journal 完整）。

### §10.8 官方环境口径与 F3／F4／F5 处置（2026-09-19，用户令「环境类内容按官方跑分要求」）

**官方约束（一手证据）**：任务环境由数据集内 `task.toml`（`[environment] docker_image`／
`cpus`／`memory_mb`／`allow_internet`）与 `environment/Dockerfile` 定义，且数据集按 digest
pin（本轮 `terminal-bench-2-1@sha256:7d7bdc1c…`）；官方 README 要求提交用官方数据集跑
（`harbor run -d terminal-bench/terminal-bench-2-1 … -k 5 --upload --public`）。⇒
**改任务镜像或把任务依赖预装进容器＝环境漂移、成绩不可比**；本仓库 2026-08-29 就同族问题
已有裁决——方案 A（agent install 层装浏览器，「与官方任务镜像零漂移」）首选，方案 B
（改 89 个 task Dockerfile）**否决**。允许面＝镜像预拉/本地缓存（不改内容）与 agent 侧自带工具。

**本案证据**：torch-pipeline 官方环境即 `FROM ubuntu:24.04`＋`WORKDIR /app`（无 Python）＋
`allow_internet = true`，verifier 侧用 uv 现装 cpython/torch 属官方 verifier 行为。故：

- **F5（镜像无 Python）＝维持现状**：自举环境是该题设计内的难度（`allow_internet=true`），
  预装 Python 会实质降低题目难度并与官方镜像漂移。若需剥离自举成本，只能在「换题/另立参考系」
  层面解决，不在容器内部动刀。
- **F3（浏览器车道恒死）＝维持现状，若启用须走 agent install 层**：官方 minimal 无浏览器；
  补浏览器只能开项目既有的 `--ak eval_browser=true`（apt chromium 30–60 s/容器，失败零退出），
  这会改变与最小口径的可比性 ⇒ 是否在正式跑批启用留裁决，**不改任务镜像**。附带小项：
  探针 `retrieval_family.browser.present=true` 与实际 `browser_not_found` 的不一致按「探针
  语义＝工具在位、≠ 可执行件在位」记注，不单独改码。
- **F4（资源门 deny 常态化）＝维持现状＋框架侧留观察**：容器资源由 task.toml 固定
  （本轮 `memory_mb=8192`），deny 阈值与 Docker Desktop VM commit 上限属框架/装置侧参数，
  不涉任务环境。

**用户裁决（2026-09-19，收束）**：「按照官方要求做，而且我们不在 orz 做内部环境提前补强」
⇒ F3／F5 定案为**维持现状**：不启用 `eval_browser`、不在容器内预装 Python／依赖、不改任务
镜像；`eval_browser` 开关保留在树上但不进入正式跑批口径。环境类摩擦（F3／F4／F5）**不立项**，
仅作观察留档。

### §10.9 检索延时判定（三轮时延分布；回答「延时问题是否已解决」）

口径：`tool_completed.payload.wall_ms`（**整调用**墙钟，非「首个结果」时延——见下方测量边界）。

| 轮次 | web_search n | p50 | p90 | max | >10 s | >30 s | >60 s |
|---|---|---|---|---|---|---|---|
| r1 | 49 | 21.4 s | 32.1 s | 36.0 s | 36 | 13 | **0** |
| r2 | 45 | 20.6 s | 29.7 s | 31.5 s | 36 | 4 | **0** |
| r3 | 24 | 26.9 s | 29.8 s | 35.3 s | 21 | 2 | **0** |

web_fetch：r1 p50 1.2 s／max 9.4 s；r2 p50 0.9 s／max 60.0 s（外域大件）；r3 p50 0.5 s／max 1.8 s。

**判定（分两面）**：

1. **尾部灾难已解决（可确定）**：基线 R0／R4 的 120 s 静默超时与 46–79 s 慢成功在三轮中
   **一次未复现**——单次最坏 31.5–36.0 s，零调用 >60 s，且失败全部结构化带 cause。
   该封顶来自框架侧截止（0ac G1 计时语义：5 s 快成功簇／~10 s 首结果截止簇／28–34 s
   T_overall 兜底簇），**与网络好坏无关**（r1 代理在挂时同样未见 120 s 量级）。
2. **典型值仍偏慢、且原判据在本轮口径下不可测**：p50 仍在 20–27 s、>10 s 占比 36/49、
   36/45、21/24。0ac 判据是「检索类**首个结果** p99 ≤10 s」，但
   `retrieval_progress`／`result_delivered` 两个投递事实在**九个 run 中零出现**
   （官方口径下 `web_search local_segmented=off`，分段/流式投递未启用）⇒ 首个结果时延
   无事件面可读，只能用整调用墙钟近似。
3. **与超时结局的关系**：延时不再是主要消耗，**投量才是**——r3 torch 21 次 × ~20 s
   ＝427.6 s＝预算 47.5%，且全成功仍无交付物。故「单次延时已封顶」≠「超时问题已解决」。

**新增观察 F10（不立项）**：官方口径下投递族事件（`retrieval_progress`／`result_delivered`）
不落账 ⇒ 「首个结果 ≤10 s」判据在官方跑批口径下不可测；若要保留该判据，需明确它在
`local_segmented=off` 口径下是否仍适用，或另立可测代理（如首个结果时间戳入 `tool_started`
载荷）。留裁决。
