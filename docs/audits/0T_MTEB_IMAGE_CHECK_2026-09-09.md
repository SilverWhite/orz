# 0t 子项：mteb 镜像核查——不同与实际情况（2026-09-09）

> 任务：用户裁决核查 mteb 镜像是否存在问题（线索：既往 VM high-nist 跑通过、
> 那次模型自行下载组件构建成功）。本注记为文档轮核查（未做活体实验），
> 活体探针计划见 §4，随 0t S2 执行。
> 关联：[`0S 细节分析`](0S_DETAIL_ANALYSIS_2026-09-09.md) FP-2 /
> [`设计 v1.1`](../RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md) §3.6。

## 1. 结论（先答问）

1. **镜像本身降为主嫌疑之外**：同一 mteb 镜像，2026-08-31（R2-recheck 批）
   browser_read 39/41 成功（健康），2026-09-08/09（R3）0/52 全导航
   chrome-error。镜像不变，变的是运行时环境。
2. **Chrome 不在镜像里**：浏览器由评测侧适配器（`tb_agents/orz.py`）每次
   运行时注入——apt 最新版优先、官方快照回退。08-31 注入 **Chromium
   151.0.7922.173**，R3 注入 **Chromium 152.0.7977.82**（apt 浮动版本在两次
   运行间自动升级）。151→152 升级与「可拉起但导航全灭」时间吻合；同容器内
   curl 可用而 Chrome 全灭，指向新版 Chrome 在该容器内的自身网络栈/DNS 回归。
   **相关性成立、因果未证**——需 §4 活体探针定案。
3. **用户记忆中的「下载组件并构建跑通」有两次，都在本机容器、都不是 VM
   那次**：①2026-08-30 gate-google-1（reward 1.0，377 次终端调用，浏览器仅
   试 1 次即弃）；②2026-08-31 sweep-0h-s4-lb（reward 1.0，curl HF API +
   GitHub API + `git clone` mteb 仓库本地构建，browser_read 0 次调用）。
4. **VM 那次（2026-09-07 W2-chunk1-031）不构成镜像健康证据**：它是 VM 上
   `orz.exe -p` 直跑提示词——无 task 容器、无镜像、无 verifier；模型
   Invoke-WebRequest 探 HF 被沙箱拦截后，收尾文本**明言答案出自模型自身
   知识**（「live access … was blocked … the value reflects the
   best-supported answer from available knowledge」）。
5. **mteb 从未靠 browser 车道通过过任何一次**；唯一 browser 健康的一次
   （R2-recheck）死于 3600s 墙钟（199 次终端调用），不是死于浏览器。

## 2. 逐次证据时间线（同数据集 pin terminal-bench-2-1@7d7bdc1c）

| 时期 / 批次 | 载体 | 浏览器注入路径与版本 | browser_read 画像 | mteb 结果 |
|---|---|---|---|---|
| 08-29 R2-failures-c3 | 本机容器 | （未使用，0 次调用；web_search 49 + web_fetch 46） | — | 未过 |
| 08-30 gate-google-1 | 本机容器 | 快照 `/opt/chrome-linux/chrome`（旧注入逻辑） | 1 次尝试即 `browser_read_candidate_count_unbound` 机械错弃用 | **PASS reward 1**（377 次终端调用，下载+本地构建） |
| 08-31 R2-recheck-10t | 本机容器 | apt **Chromium 151.0.7922.173** | **39/41 OK（健康）** | 3600s 超时 reward 0（199 次终端调用，死于时间） |
| 08-31 sweep-0h-s4-lb | 本机容器 | apt Chromium 151（版本行在案） | **0 次调用**（模型直取终端） | **PASS reward 1**（curl HF/GitHub API + git clone 构建） |
| 09-07 W2-chunk1-031 | **VM 直跑提示词**（无容器/verifier） | 不适用 | 不适用 | run_finished；HF 被拦，**知识作答**（模型原文承认） |
| 09-08/09 R3 | 本机容器 | apt **Chromium 152.0.7977.82** | **0/52，全导航 chrome-error**（同容器 curl 可用） | PASS（2812s），15 分钟死循环后 curl 转向 |

证据指针：R3 journal `gsa-volumes/official-r3-unsolved20-mteb-leaderboard`
（RUN-CLI-6aa04ffb）；recheck journal
`gsa-volumes/official-r2-failures-recheck-10t/*/runs/RUN-CLI-6a9559dd`（mteb
按 prompt 鉴定）+ 版本行 `official-r2-failures-recheck-10t/job.log`；gate
journal `gsa-volumes/gate-google-20260830-1`（RUN-CLI-6a93baa8）+ 注入路径
`jobs-gate/gate-google-20260830-1/mteb-leaderboard__JVUUm7z/agent/orz.txt`；
sweep journal `jobs-sweep/sweep-0h-s4-lb/mteb-leaderboard__zeBUscM/agent/gsa`；
W2 journal `_windows_high_nist/formal-2026-09-02/evidence-agent-tb2.1-w2-chunk1-031-high-nist/journal-agent-mteb-leaderboard.jsonl`
（终局文本为知识作答自认）。R3 版本行见 R3 console log。

## 3. 与 0t 设计的关系（嫌疑转移）

- FP-2 的「浏览器组件可拉起但实质不可用」在 mteb 个案上进一步具体化为：
  **运行时注入的 apt 浮动版本 Chromium 升级（151→152）后，新版在该容器内
  无法完成任何导航**（curl 对照可用）。这是注入策略问题，不是镜像损坏。
- 候选处置（活体探针定案后择一，随 S2 裁决）：注入时钉验证过的版本；
  快照路径优先（版本受控）；或注入后加一次导航自检、失败即按 FP-2 正常
  回传错误（不阻拦、不教学，模型自然换道——与双车道设计互补）。

## 4. 活体探针计划（S2 执行，约 5–10 分钟）

1. `docker run` mteb 镜像（`terminal-bench/mteb-leaderboard:20260430`）。
2. apt 注入 Chromium 152（当前版）→ 无头导航 `https://example.com` 与
   `https://huggingface.co` 各一次，记录结局；同容器 curl 对照。
3. 钉回 Chromium 151（snapshot/apt pin）重复步骤 2。
4. 判定：152 败 151 成 → 版本回归定案，处置按 §3；两版皆败 → 转环境向
   （容器 DNS/网络栈）追查；两版皆成 → 转 R3 当夜环境快照差异（低概率）。

## 5. 边界

- 本注记为文档轮核查，未做活体实验；151→152 为相关性结论。
- R2-c3（08-29）browser 0 次调用无法区分「车道未启用」与「模型未用」，
  不影响结论链。
- W2-031 的 VM run 无 reward 语义，其「完成」不能与官方 reward 互推。
