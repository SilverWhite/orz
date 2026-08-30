# Google 门禁观察实验报告（0k 第一批第 5 项，2026-08-30）

> 对应 TODO P0-0k 第 5 项 / DUAL §3.2 / 调研 §8.4。job：
> `gate-google-20260830-1`（D:\tb-eval\jobs-gate）。

## 1. 实验配置

- 任务集（DUAL §3.2 检索密集题）：mteb-leaderboard（3600s）/
  path-tracing-reverse（1800s）/ rstan-to-pystan（1800s）/
  configure-git-webserver（900s）/ mteb-retrieve（1800s），k=1。
- 模型：deepseek-v4-flash；二进制：orz b604773（2026-08-30 S3 重建）；
  agent：tb_agents.orz:Orz（orz.py SHA256 047830C8，含 eval_browser）。
- 检索面：orz run 固定 `--retrieval-mode local_browser`；容器 chromium
  注入开启（eval_browser=true）；无 max_wallclock 覆盖，按官方 agent
  超时执行。
- 时间：13:05–14:08 HKT（总 job 墙钟 ~63min）。

## 2. 容器浏览器注入验证（通道前置，全部通过）

| 镜像 | 任务 | 注入方式 | 版本 |
|---|---|---|---|
| python:3.10-slim-bookworm | mteb-leaderboard / mteb-retrieve | apt chromium → /usr/bin/chromium | Chromium 151.0.7922.173 |
| ubuntu:24.04 | path-tracing-reverse / rstan-to-pystan / configure-git-webserver | 官方快照 → /opt/chrome-linux/chrome | Chromium 154.0.8034.0 |

- 5/5 容器注入成功；install 阶段 `agent_setup` 1.7–4.6min（apt ~1–2min、
  快照下载+解压 ~4min，均含 ACAF 供应等既有步骤）。
- 全部 run 的 `retrieval_mode_transition` 定档
  `off → local_browser`（authority=session_bootstrap、capability_status=
  available）——local_browser 通道可用，probe 无失败。

## 3. 结果总表

| 任务 | reward | agent 执行墙钟 | tool_rounds | 主要工具 | 检索调用 | 结局 |
|---|---|---|---|---|---|---|
| mteb-leaderboard | **1.0** | 60min（3600s 超时 kill） | — | 终端 207+ | 0 | 超时前已提交正确答案，verifier 通过 |
| rstan-to-pystan | **1.0** | 15.5min | 36 | 终端 25 / 读 7 / grep 3 | 0 | 正常完成 |
| configure-git-webserver | 0.0 | 4.5min | 24 | 终端 25 | 0 | 正常完成（未解出） |
| mteb-retrieve | 0.0 | 4.2min | 28 | 终端 31 / submit 2 | 0 | 正常完成（未解出） |
| path-tracing-reverse | 0.0 | 30min（1800s 超时 kill） | — | 终端 55+ | 0 | 超时 |

通过率 **2/5（40%）**；2 道超时（mteb-leaderboard 超时但解出、
path-tracing-reverse 超时 0 分）；无 errored trial。

## 4. 与历史基线对比（k=1 单样本，仅列变化）

| 任务 | official-r1（08-25） | official-r2（08-29） | 本轮（08-30） | 变化 |
|---|---|---|---|---|
| mteb-leaderboard | 超时 0 | 超时 0 | **1.0** | ↑ 首解 |
| rstan-to-pystan | 超时 0 | 超时 0 | **1.0** | ↑ 首解 |
| configure-git-webserver | **1.0** | — | 0.0 | ↓ 回归 |
| mteb-retrieve | 0.0 | **1.0** | 0.0 | ↓ 回归 |
| path-tracing-reverse | 超时 0 | 超时 0 | 超时 0 | — |

5 题合集通过数：1（r1 基线）→ 2（本轮）；2 升 2 降，样本小不做强结论。

## 5. 门禁观察结论（核心）

1. **Google 门禁无样本可观察**：5 题 agent 执行全程**检索调用为 0**
   （web_search / web_fetch / browser_read / 外部检索子代理均为 0 次）。
   local_browser 通道定档 available 但从未被模型使用——SERP 请求未发生，
   CAPTCHA / 429 / 页面结构 / pacing 校准**无数据产出**。主序裁决（Google
   vs Bing）本轮无法判断，需"会触发检索"的样本。
2. **零检索使用是与官方 minimal 同向的关键发现**：近零提示词下模型自主
   决定不检索；历史"检索超时"风险本轮未出现（无检索路径耗时），但解出
   完全依赖终端 + 已有知识/代码内网络调用（如 mteb-leaderboard 用 curl
   调结果仓库 API，属终端命令内网络，不经过检索工具）。
3. **通道工程验证成功**：5/5 容器注入 + probe available + 零
   browser_launch_failed，容器内浏览器组件就绪度闭环（0k-5 前置完成）。
4. **对后续的含义**：门禁观察与 pacing 校准需要"强制/引导检索"样本
   （如子代理检索提示或任务天然触发检索）；是否机械引导模型检索属
   模型面改动，与"机械层加厚、模型层零改动"的当前方向冲突，需用户裁决。

## 6. 遗留与建议

- 观察实验本身闭环；Google 门禁/pacing 数据待补样本（建议：待定）。
- configure-git-webserver / mteb-retrieve 的回归为 k=1 噪声候选，可并入
  后续小批复跑确认，不单独处置。
- 无检索样本下 DUAL §3.1 的"引用准确性"维度不可度量；双检索模式的正/
  负面判定（local_browser vs framework_fallback）需检索触发样本。

---

## 7. 第二轮（2026-08-30 追加）：count-dataset-tokens + train-fasttext

> job `gate-google-20260830-2`（D:\tb-eval\jobs-gate）。样本选择：错题集
> 中检索依赖最明确的两道（huggingface README 依赖 / fasttext 文档依赖）。

### 7.1 结果

| 任务 | 镜像 | 超时 | reward | agent 执行 | 结局 |
|---|---|---|---|---|---|
| count-dataset-tokens | python:3.13-slim-bookworm | 900s | **1.0** | 3.95min | 完成；browser_read 被拒后转终端解出 |
| train-fasttext | python:3.13-slim-bookworm | 3600s | 0.0 | 60min（超时 kill） | 超时；/app/model.bin 未产出，verifier 失败 |

- count-dataset-tokens 4 分钟解出：终端内访问 HF（datasets 库/API）读
  README 元数据 + 下载 Qwen2.5-1.5B-Instruct tokenizer，token 数 79586
  写入 answer.txt，verifier 通过。
- train-fasttext：模型 3600s 内未完成训练（工具面全部 run_terminal_cmd，
  无检索调用），verifier 报 model.bin 缺失。

### 7.2 关键发现（P1）：主 Agent 面 browser_read 声明/执行不一致

count-dataset-tokens 的**首次模型响应就调用了 browser_read**
（URL=https://huggingface.co/datasets/ryanmarten/OpenThoughts-1k-sample，
mode=full），但立即被机械层以 `browser_read_candidate_count_unbound`
（候选计数域未绑定）拒绝，无 ToolStarted。模型随后转终端完成并解出。

**定位**：

- 声明面：`controller.rs` 工具投影链的
  `apply_retrieval_surface_projection` 在 local_browser 模式只剔除 web
  族、**保留 browser_read**（RETRIEVAL-SUBAGENT-WIRING 2026-08-25）；
  journal `request_header_change.tools` 实测主 Agent 工具列表含
  browser_read（第一轮 5 run 亦全部如此，7 工具：read_file/grep/
  search_replace/run_terminal_cmd/browser_read/blackboard_read/submit）。
- 执行面：`relay::route("browser_read")` → **Host 直执行**（区别于
  web_search/web_fetch → ExternalRetrieval 子代理派发）；主车道
  `LoopProfile::main` 的 `fetch_candidates = None`，candidate_gate 对
  None 域 fail-closed（`{family}_candidate_count_unbound`，
  host_exec.rs 注释明示 "main/grill lane — retrieval tools never execute
  there; belt-and-braces"，P0-B 2026-08-14 用户裁决"主 Agent 不执行
  检索任务、主车道投影移除 browser_read"）。

**性质**：声明层与执行层不一致——模型看到 browser_read、尝试调用、被
机械拒绝（ADR v1.5 明令避免的"声明面与执行层不一致对模型不可预测"）。
P0-B 步骤 4 的"主车道投影移除 browser_read"未在 local_browser 主面上
生效；`R1_SEALED_MAIN_TOOLS` 亦未含 browser_read。子代理外部 lane 的
browser_read 恢复（subagent_tool_projection，local_browser 模式从
registry 恢复）不受影响。

**修复（方向 A，2026-08-30 已实施）**：`R1_SEALED_MAIN_TOOLS` 加入
`"browser_read"`——主面投影一律剔除，模型不再看到无法执行的工具；
子代理外部 lane 恢复逻辑独立（registry.get），不受影响；web 族维持
local_browser 下剔除、framework_fallback 下可用（外部子代理派发路径
正常）。与"模型层零改动、机械层正确"方向一致。实现：orz-loop
projection.rs（R1_SEALED + 注释 + 测试改写/迁移），orz-loop 623/0/3
全绿、projection 12/12、fmt 干净、clippy 无新增；S3 重建待续。

### 7.3 第二轮对门禁观察的意义

- 出现首次真实检索尝试（browser_read），但被声明/执行缝隙拦截——Google
  SERP 仍无样本（browser_read 未真正执行，无页面访问）。
- 若方向 A 落地，模型将不再尝试 browser_read；外部检索依赖（如 HF
  页面）需经 web 族/外部子代理或终端内访问完成——本轮 count-dataset-tokens
  已证明终端内访问可解出，检索通道缺位不阻塞解出。

---

## 8. 第三轮（2026-08-30 追加）：方向 A 修复验证

> job `gate-google-20260830-3`（D:\tb-eval\jobs-gate）。样本：方向 A
> 修复后（`R1_SEALED_MAIN_TOOLS` 含 browser_read，orz 1973511 + s3b
> 重建）重跑 count-dataset-tokens 单题验证。

### 8.1 结果

| 任务 | 镜像 | 超时 | reward | agent 执行 | 结局 |
|---|---|---|---|---|---|
| count-dataset-tokens | python:3.13-slim-bookworm | 900s | **1.0** | 2.95min | 完成；主面 6 工具无 browser_read，零拒绝事件 |

- 主面请求头工具列表 6 工具（read_file/grep/search_replace/
  run_terminal_cmd/blackboard_read/submit），browser_read 已消失；
  journal 零 browser_read 痕迹、零 `*_candidate_count_unbound` 拒绝事件。
- 解出路径同 R2（终端内访问 HF README + Qwen2.5-1.5B-Instruct
  tokenizer，answer=79586），agent_exec 2.95min 较 R2 3.95min 略快
  （无 browser_read 尝试烧轮）。
- 方向 A 闭环：声明/执行缝隙消除，模型不再看到无法执行的检索工具。

### 8.2 第三轮对门禁观察的意义

- 修复后主面检索入口彻底归零（web 族在 local_browser 下隐藏 +
  browser_read 封存）——模型全程终端路径解出，Google SERP 仍无样本。
- 结构结论强化：12 个 gate run（R1 5 + R2 2 + R3 1 + R4 2）检索调用
  全部为 0，是主面工具面缺失检索入口的必然结果，从未真正测到「模型
  看到 web_search 时会不会用」。

## 9. 第四轮（2026-08-30 追加）：build-pmars + sam-cell-seg

> job `gate-google-20260830-4`（D:\tb-eval\jobs-gate）。样本：错题集
> 其余两道检索依赖候选，k=1 local_browser（2h 档超时）。

### 9.1 结果

| 任务 | 镜像 | 超时 | reward | agent 执行 | 结局 |
|---|---|---|---|---|---|
| build-pmars | python:3.13-slim-bookworm | 3600s | **1.0** | 2.95min | 完成；终端构建解出 |
| sam-cell-seg | python:3.13-slim-bookworm | 7200s | **1.0** | 15.1min | 完成；终端+读写解出 |

- build-pmars：agent_exec 2.95min，全程零检索调用，终端构建
  （pmars 编译）解出，verifier 通过。
- sam-cell-seg：agent_exec 15.1min，工具面 run_terminal_cmd 32 /
  read_file 11 / search_replace 6，**零检索调用**；模型自写
  convert_masks.py（MobileSAM CPU 推理 + 重叠消解 + 轮廓提取），
  终端内安装 mobile_sam（git clone + pip install -e），verifier
  通过。

### 9.2 第四轮对门禁观察的意义

- 两题均 1.0 解出，再次印证：主面无检索入口时模型全部走终端/读写
  路径，检索通道缺位不阻塞解出（同 §7.3 结论）。
- 四轮累计 10 个 gate run 检索调用均为 0——Google 门禁/pacing 无
  SERP 样本是**结构性缺检索入口**的结果，不是「模型倾向不用检索」。
- 用户裁决（2026-08-30）：先恢复外部——主面恢复 `web_search` 单一
  派发入口（调用即派发外部检索子代理，执行面 local_browser 下仍仅
  browser_read 引擎 SERP），用于验证「模型在有入口时用不用检索」与
  「Google SERP 门禁」两个问题；内部检索（retrieve_project_docs）
  维持 R1 封存。实施见 TODO P0-0k 与本报告 §10。

## 10. 第五轮（2026-08-30 追加）：主面恢复 web_search 后的首个观察轮

> job `gate-google-20260830-5`（D:\tb-eval\jobs-gate）。样本：
> count-dataset-tokens（R2 首次响应即想访问 HF 页面的最强检索触发候选），
> k=1 local_browser；二进制 = s3c 重建（orz 4baf766，106,412,016 B，
> 主面恢复 web_search 单一派发入口）。

### 10.1 结果

| 任务 | 镜像 | 超时 | reward | agent 执行 | 结局 |
|---|---|---|---|---|---|
| count-dataset-tokens | python:3.13-slim-bookworm | 900s | **1.0** | 5.1min | 完成；step 1 即调 web_search，转终端解出 |

- 主面请求头工具列表 7 工具（read_file/grep/search_replace/
  run_terminal_cmd/**web_search**/blackboard_read/submit）——web_search
  已恢复为唯一检索入口；web_fetch 隐藏、browser_read 保持 R1 封存。
- **完整检索链路实证（11 轮以来首次）**：step 1 主面调用 web_search
  （query=「ryanmarten/OpenThoughts-1k-sample dataset huggingface
  README」）→ `tool_started` target=external_retrieval（外部子代理派发）
  → 子代理 local_browser 面 browser_read
  （https://huggingface.co/datasets/ryanmarten/OpenThoughts-1k-sample，
  exit 0、wall 2.4s、candidate 1/8）→ `retrieval_result_committed`
  （evidence ledger：SRC-001 full_text_observed + content_sha256 +
  tier=default / mechanical_weight=1.0；SRC-002 metadata_only）→
  `retrieval_close_record` 自动闭环 → web_search 完成回传主面
  （exit 0）。全程零拒绝、零 CAPTCHA/429。
- 模型后续 17 次 run_terminal_cmd 解出（datasets 读 README 元数据 +
  Qwen tokenizer，answer=79586），verifier 通过。

### 10.2 第五轮对门禁观察的意义

1. **「模型在有入口时用不用检索」已实证：会用**——web_search 恢复后
   首次响应即被调用（step 1 双工具并行：run_terminal_cmd + web_search），
   查询构造良好；此前四轮 10 run 零检索确系结构性缺入口，非模型倾向。
2. **派发-执行链路完整**：主面 web_search → 外部子代理 → 子代理
   browser_read（local_browser 面二存一仅此一检索通道）→ evidence
   ledger 回传 → 自动闭环，机械层无缝隙。
3. **Google SERP 仍无样本**：子代理直接导航到已知 HF URL（未走
   Google 搜索页）——SERP 门禁数据需「子代理不知道目标 URL」的查询或
   第二批引擎 SERP 工具（`search_engine_search` + 键入模拟 + pacing）
   落地后才能采集；本轮的结论是双模式链路可用，引擎 SERP 门禁观察
   继续挂起至第二批。
4. **观察项**：retrieval_result_committed 的 SRC-002 呈
   metadata_only（missing_scope=content）但标题含「full page text,
   observed」——子代理结果形成层的小瑕疵候选（同一 browser_read 的
   二次声明），不阻塞解出。

### 10.3 P1 登记（2026-08-30 深挖 retrieval_result_committed 形成链）

对 SRC-002 的深挖升级为正式 P1（原「小瑕疵候选」表述作废）：

1. **直接成因（SRC-002）**：子代理终答的 `[SOURCE]` 声明行
   （URL + 标题写在同一行）被 `parse_retrieval_text` 整行收为 ref →
   `source_url_or_ref` 是一整句话而非干净 URL；机械层按 §3.7.5 正确降为
   metadata_only（声明≠观测，模型自称 full page text 不被采信），与
   SRC-001（真实 browser_read 证据）重复。小毛病：声明行无 URL 形状
   校验/规范化，ref 被标题污染。
2. **结构性 P1（比 SRC-002 大）**：`[RESULT_JSON]` 组织块的 source_ids
   必须命中机械 ledger 的 `SRC-001` 等 id，但这些 id 是子代理跑完后
   才在 `build_structured_result` 分配、运行中不可见；提示词却要求
   「source_ids must reference an actual tool result you received」——
   契约不可实现。模型只能自造 id（R5=SRC-HF-OPENTHOUGHTS、
   mteb-retrieve=SRC-1/2/3，格式均不匹配）。
3. **实证**：0h S4（2 次派发）/ mteb-retrieve（5 次）/ R5（1 次）全部
   organized_response 空、visibility_degraded=true、assessment
   reason_codes 含 structured_result_validation_failed；唯一通过路径是
   知情单测（structured_result_accepts_valid_model_block，测试作者预先
   知道 SRC-001 会被分配）。生产环境该契约从未生效。
4. **影响**：任务解出不受影响（主代理拿到 bounded prose 含块文本 +
   [DOC]/[SOURCE] 行 + blackboard 全文）；机械层照常以 tool evidence
   ledger 工作；但「结构化结果」功能实际失效，visibility_degraded 对
   每次真实块误触发、审计信号失真，organized_response 消费链（dispatch
   摘要结论计数 / schema / Python verifier / fixtures / 测试）同步空转。
5. **修复方向（C，待用户裁决）**：删除 `[RESULT_JSON]` 契约——子代理
   提示词移除模板段、build_structured_result 删 block 解析/校验/
   annotation 合并、visibility_degraded 语义重定义、dispatch 摘要改口径、
   schema/verifier/fixtures/测试同步、ADR §3.7 条 12 第三层（模型加权
   标注，生产中也从未生效）处置；附带 `[SOURCE]` 声明行 URL 规范化。
   登记：BACKLOG 0k / TODO P0-0k / CLI_PROJECT_INDEX
   （GAP-RETRIEVAL-STRUCTURED-RESULT）。
