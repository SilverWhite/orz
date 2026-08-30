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
