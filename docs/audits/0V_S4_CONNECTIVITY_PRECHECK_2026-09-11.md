# 0v S4 连通性与 SERP 形态预检（2026-09-11）

> 入口：BACKLOG **0v** / TODO P0-0v。目的：在花掉一次约 35 分钟的实机复跑之前，
> 先确定 0v 判据 1/5/7 所依赖的两个前提——①F2 的「预挂载 Chromium 复用」在
> 真实评测镜像里是否成立；②无代理环境下「Google 腿失败 → 引擎链兜底 Bing」是否
> 真的会发生（设计 §2 原文写的就是「无代理环境（Google 不可用）」，此前只有
> R4「模型自己猜 duckduckgo 失败」的间接旁证）。
> 口径：评测任务镜像 + 装置真实启动参数 + 无代理直连，**不消耗跑批额度**。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0v-s4-refresh-20260911\`
> （本地，gitignore 覆盖，不入库）。

## 0. 探针清单

| 探针 | 文件 | 结果 |
|---|---|---|
| 端点/DNS 预检 | `connectivity-precheck.sh` / `.txt` | 半程有效（见 §1 说明） |
| 装置全序 SERP 探针 | `serp-probe-full.sh` / `.txt` | 三引擎连通性结论 |
| 无依赖顺序对照 | `serp-probe.sh` / `.txt` | 反证（缺库即不可用） |
| Bing 结果域清单 | `bing-domain-probe.sh` / `.txt` | 三条查询的域分布 |
| Bing DOM 结构确认 | `bing-structure-probe.sh` / `.txt` | 选择器命中 + 链接形态 |

**说明（探针自身的坑，先记下来）**：第一版 `connectivity-precheck.sh` 假设镜像里有
`curl`——实测 `curl` / `wget` / `python3` **全部缺失**，HTTP 部分未执行，只有 DNS
结论有效；后续改用装置自带的 Chromium 完成全部 HTTP 面探测。

## 1. F2 复验：预挂载 Chromium 在评测镜像里是否真的可用 —— **成立**

- 镜像底子：`alexgshaw/dna-assembly:20251031`（**Ubuntu 24.04.3 LTS**）。
- **反证（不装依赖直接挂载）**：`ldd` 报 **25 个共享库缺失**
  （`libglib-2.0.so.0` / `libnss3.so` / `libatk-1.0.so.0` / `libcups.so.2` …），
  `chrome --version` 直接加载失败。即「只挂目录」是不够的。
- **正证（按装置真实顺序）**：`tb_agents/orz.py::_install_browser` 的 snapshot
  分支里，`apt-get install … _BROWSER_DEPS_APT` 这一步位于
  `if [ ! -x /opt/chrome-linux/chrome ]` 判断**之前**——即预挂载只跳过下载、
  **不跳过依赖安装**。按该顺序实测：依赖安装墙钟 **186s**，
  装完 `missing shared libs = 0`，`chrome --version` →
  **`Chromium 155.0.8053.0`**。
- 结论：F2 的「宿主侧供给 + 只读挂载」在真实评测镜像里**可用**；
  此前 F2 改造记录只验证了 Chromium 能启动，**没有验证过「在缺库的评测镜像里
  跳过依赖安装会发生什么」**，本次把这一面补齐（并确认装置顺序本身没有缺口）。
- 边界：依赖安装 186s 属装置准备阶段，由既有
  `--agent-setup-timeout-multiplier 4` 覆盖；本探针顺序与装置一致，但不等价于
  装置代码路径的端到端执行。

## 2. 引擎链前提：Google 失败 / Bing 可用 —— **前提成立（首次直接取证）**

按生产启动参数（`discovery.rs::browser_launch_args(headless=true)`）、
按 `serp.rs::search_url` 的真实 URL 逐条探测：

| 引擎 | 端点 | 结果 | 判定 |
|---|---|---|---|
| Google | `www.google.com/search?q=…&hl=en-US&gl=us` | rc=124（120s 超时），**DOM 0 字节** | 不可达 |
| Bing | `www.bing.com/search?q=…&setmkt=en-US&count=10&first=1` | DOM **164,195 字节**，命中 `li.b_algo`，`<title>rust borrow checker - Search</title>` | **可用，返回真实有机结果** |
| DuckDuckGo | `html.duckduckgo.com/html/?q=…&kl=us-en` | rc=124，**DOM 0 字节** | 不可达 |

- DNS 面旁证：`www.google.com` 解析到 **`2001::1`**（污染地址），
  `html.duckduckgo.com` 解析到真实 IPv6 但连接不通；`www.bing.com` 解析到
  真实 IP（`202.89.233.100`）。
- **含义**：引擎链 `Google → Bing → DDG` 在本环境下必然落到 **Bing**——
  判据 1/5/7 从「很可能没样本」变成「有真实样本」。这是设计 §2 前提的
  第一次直接实测。

## 3. Bing SERP 形态与解析器命中 —— **选择器成立，链接是直链**

- 结构：`<ol id="b_results" class=""><li class="b_algo" …>`——即 orz CDP 选择器
  **`#b_results > li.b_algo`（直接子元素）命中**；本页 `li.b_algo` 计 **8** 条。
- 污染面（本次三条查询，`tech` / `commercial` / `general`）：**未出现**
  `li.b_ad`、`bing.com/challenge` / `b_captcha`、consent 插页。
- 链接形态：`bing.com/ck/a` 计 **0**、`u=a1` 计 **0**、站外直链 `href` 计 **22**。
  → 本环境下 Bing 有机结果是**直链**；P1-1 的 `u=a1<base64url>` 解码路径
  **不会被自然走到**（`decode_result_url` 走 `unwrap_or_else` 透传，行为正确，
  但拿不到「解码成功」的样本）。

## 4. 判据可及性重估（据本节结论修订 S4 复跑预期）

| 判据 | 复跑可否取到样本 | 依据 |
|---|---|---|
| 1 Bing 有机结果 | **可**（结构化、无 `b_ad` 已预证） | §2/§3 |
| 1′「URL 已解码」 | 仅能取到**透传**样本 | §3 直链形态 |
| 2 车道 navigate 无拒绝 | 可（需看实际调用） | 首轮已部分成立 |
| 4 通用统计 | 可（零 400 / 命中率 / 延迟） | 首轮已部分成立 |
| 5 CAPTCHA/空结果识别 | **不定**（本次三条查询未出现污染页） | §3 |
| 6 `setmkt=en-US` 生效 | **可**（URL 参数面） | §2 |
| 7 低质量域名加权 | **可**——`general` 查询同时返回内容农场域（`bloommindfully.com`／`calmsage.com`／`hollyherman.com`／`refreshyourlife.in`／`scienceinsights.org`／`sciencenewstoday.org`／`yourhealthmagazine.net`）与权威域（`harvard.edu`） | `bing-domain-probe.txt` |
| 9/10 车道预算 / 会话底线 | **不可**（需单次激活 ≥9 次引擎导航；最重任务也只给 3 次） | 首轮 + 本节 |

## 5. 观察项（登记备查，非缺陷）

- **O-a `setmkt=en-US` 的效果偏弱**：参数确实在 URL 上（判据 6 成立），但
  `tech` 查询的有机结果仍以中文域为主（`baidu.com`／`biomart.cn`／
  `bohrium.com`／`neb.cn`）。参数生效 ≠ 结果本地化生效；如需更强区域化须另议，
  不在本设计范围。
- **O-b `u=a1` 解码路径自然不可达**：见 §3；P1-1 的修复正确性目前只由单测覆盖，
  实机样本需构造性手段（或换到会出跳转壳的出口网络）才能取得。
- **O-c 探针的 rc=124 不可外推为产品延迟**：`--dump-dom --virtual-time-budget`
  在页面持续有后台活动时不会自然退出，被外层 `timeout` 杀掉属探针形态；
  Bing 的 DOM 在超时前已完整产出。**产品侧走 CDP，不受此影响**，真实页面
  加载延迟须由 S4 跑批取证。

## 6. 结论与下一步

- **两项前提均成立**：F2 的预挂载复用可用；Google 腿确实失败、Bing 确实可用。
  → **dna-assembly 复跑值得跑**，判据 1/6/7 有真实样本，2/4 可续证，5 不定。
- **换题不改变结论**：判据 9/10 需要单次激活 ≥9 次引擎导航，属调用量问题而非
  题目属性（首轮 50 轮里 `browser_control search` 仅 3 次，检索主力是原生
  `web_search` 13 次），任何自然任务都不产生该量级；这两条应交给**定向探针**
  （同容器、固定查询集、显式要求多次引擎导航）。
- 下一步：按用户放行执行 dna-assembly 复跑（载体 orz 0.4.2，见
  [0v S4 复跑重建记录](0V_S4_REFRESH_REBUILD_2026-09-11.md)），
  journal 取证沿用 `scripts/analyze_0x_0v_s4_journal.py`。
