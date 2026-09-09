# 检索子代理双车道并行标注面设计（v1 草案）

> 版本：v1（2026-09-09，用户裁决立项当日落稿；**设计未定稿，待用户复核**）。
> 状态路由：BACKLOG **0t**（P0，2026-09-09 用户裁决立项）；证据基线 =
> [`0S 细节分析`](audits/0S_DETAIL_ANALYSIS_2026-09-09.md) /
> [`R3 复跑审计`](audits/OFFICIAL_R3_UNSOLVED20_RERUN_2026-09-09.md)。
> ADR 转录点：定稿后新增 ADR-0010 §14.65，并修订 §3.7 / §14.40 / §14.43–44
> 的「二存一」语义（转录前本稿不具设计权威）。

## 0. 用户裁决（2026-09-09，登记口径）

1. 环境背景（R3 复跑当夜）：Clash 频繁超时但未真正掉线、扰乱测试，用户已关闭
   Clash；跑批使用**本地镜像**——Google 检索实际不可用、Chrome 实质不可用，
   但现行机制**无法降级到原生检索**。
2. 方向裁决：**原生检索（web 族）与本地浏览器检索（browser_read）都要开放给
   检索子代理**，像主 agent 工具栏一样**打标注**，**推荐模型先用本地浏览器
   检索**。
3. 新摩擦值得立项（R3 FP-1～FP-9，见 0S 审计 §4），随本项统一处置。

## 1. 背景与证据（R3 实证）

- **binary 工具面的两种死法**：现行模式 A 下 external 子代理工具面按激活模式
  二选一——`local_browser` 车道仅 browser_read（web 族调用被
  `retrieval_mode_requires_framework_fallback` 拒绝，orz
  `orz-loop/src/retrieval/dispatch.rs` `local_browser_lane_refuses_web_tools`）；
  `framework_fallback` 车道仅 web 族。浏览器「存在但坏」时（mteb 镜像全导航
  级 chrome-error；本地镜像下 Google/SERP 不可用致 Chrome 实质不可用），模型
  手握死车道而无法换到原生检索——mteb 52 次拒绝重试烧 865s（budget 2812s 的
  31%），靠模型自用终端 curl 才破局。
- **降级触发面过窄**：`browser_launch_failed`（注入失败，R3 四题）与 SERP 页
  级失败才降级；「能启动但导航全挂」「搜索引擎不可达」不触发任何降级。
- **拒绝信封不透明**：chrome-error 拒绝只报 scheme，不带导航失败原因与
  「勿重试」指引（FP-2）。
- **评测侧注入链脆弱**（FP-1）：快照回退依赖容器 `unzip` + 单源下载，报废
  4 题 browser 车道。
- 对照面：count-dataset-tokens 同形态 2 次即换路径（浏览器健康时 FP-2 不成灾）；
  mteb 通过靠 3600s 预算冗余 + 模型韧性，不靠车道健康（0S 审计 §5）。

## 2. 目标与非目标

**目标**

1. external 检索子代理工具面同时注册 `browser_read` 与 `web_search`/`web_fetch`
   两族；激活内模型可自行换道，无需整激活降级。
2. 工具带机械标注（主 agent 工具栏同构）：车道名、推荐序、探活/最近结局摘要；
   标注全部机械可核验，不新增常驻注入（走既有工具描述/探针投影面）。
3. 推荐序 = 本地浏览器优先；prompt 框架使用提示 ≤1 句（沿
   FUS-RETRIEVAL-SUBAGENT-WIRING 既有风格）。
4. 换道落事件面，检索记账（候选 cap、evidence、来源加权）跨车道继续生效。
5. 并入 FP-2（chrome-error 信封教学）与 FP-1（评测侧注入健壮化）。

**非目标**

- 不改主 agent 工具面与 console/direct 语义；不改 internal 子代理
  （`retrieve_project_docs`）。
- 不做自动重试循环、不做内容级成败判断；页面级失败仍不触发机械降级
  （换道由模型依标注自主决策）。
- `web_search` 全局并发 1、候选 cap、轮预算、ACAF 票据等既有硬门不变。
- 不为跑分特化（通用双车道能力）。

## 3. 设计

### 3.1 双车道工具面

- external 子代理激活时同时注册 browser_read + web_search/web_fetch；
  `dispatch.rs` 的 `retrieval_mode_requires_framework_fallback` 拒绝族退役
  （由标注偏好替代硬拒绝）。
- `retrieval_mode` 字段保留，语义修订：`local_browser` = 浏览器能力存在
  （探活通过）时的声明模式，**不再裁剪 web 族**；`framework_fallback` 收窄为
  「浏览器能力不存在（注入失败/探活失败）」的降级声明，两族工具同样并存
  （此时 browser_read 标注为不可用）。`off` 不变。
- 引擎 SERP 主序语义（Google 主序、Bing 回退）保留为「浏览器车道可用时的
  首选路径」；车道不可用时标注面直接反映，模型走原生检索。

### 3.2 工具栏标注面（主 agent 工具栏同构）

- 标注来源全部机械：探活结局（launch/probe）、本激活内最近 N 次 browser_read
  结局计数、web_search 最近失败计数。仅名称/短标签，不携带页面内容。
- 形态（示例，定稿时固化措辞）：
  - `browser_read — [车道:本地浏览器|推荐首选|探活:ok|本激活失败 0/3]`
  - `browser_read — [车道:本地浏览器|探活:导航全败(chrome-error)|建议换原生检索]`
  - `web_search — [车道:原生检索|并发1|延迟较高|最近失败 2]`
- 标注落点：工具描述字段 + 激活提示词的既有框架使用提示行（≤1 句推荐序），
  满足零新增常驻 token 纪律（探针投影既有面）。

### 3.3 换道语义与事件面

- 模型在激活内换道不需模式翻转；首次实际使用非偏好车道时落
  `retrieval_mode_transition`（新增 `reason_code=model_lane_switch`、
  `authority=model`），同一激活至多记一次，防事件面膨胀。
- 机械降级（`browser_launch_failed` 等）语义不变，authority 仍为
  `mechanical_probe`；两者并存时以机械降级优先。
- 检索记账跨车道：候选 cap（`ORZ_WEB_FETCH_CANDIDATE_CAP`）对 browser_read 与
  web_fetch 继续累计；`retrieval_result_committed` / close record 不区分车道
  （payload 可选 `lane` 字段，schema 先行）。

### 3.4 chrome-error 信封教学（FP-2 并入）

- browser_read 拒绝信封扩展：附导航失败原因（Chrome 导航错误码/DNS/断网分类）
  + 固定教学句「该页加载失败，勿原样重试；可换 web_search 或终端」。
- 同一 URL 连续 3 次 chrome-error 后，机械在下一轮结果附诊断提示（建议终端
  `curl` 对比网络/浏览器）——单次提示，不循环。

### 3.5 评测侧注入健壮化（FP-1 并入，随批实施）

- `tb_agents/orz.py` 注入链：解压改 `python3 -m zipfile`（不依赖容器 unzip）；
  快照下载加 3 次重试 + 失败原因透传 console/apt 双路径报告不吞并。
- 边界：评测侧工具链改动，不进 orz 判据面；与 0r 同族管理（环境形态文档化）。

## 4. 不变量

- 零注入边界：无新常驻 token；标注只出现在工具描述与一次性提示。
- fail-closed：探活不可得时 browser_read 标注按不可用处理，模型自然走 web 族；
  不出现无检索工具的激活（web 族恒在）。
- 审计不变：journal 事件面、failure 盖章、evidence 分级、来源加权三档全部
  跨车道生效。

## 5. S4 复验判据（草案，定稿时固化）

1. 双车道存在性：`local_browser` 模式下 web 族调用不再被
   `retrieval_mode_requires_framework_fallback` 拒绝（负样本转正样本）。
2. 标注面存在性：工具描述含车道/推荐序/探活标签；浏览器坏死场景标注反映
   「导航全败」。
3. 换道事件：`model_lane_switch` 至多一次/激活，authority=model。
4. 行为对照：chrome-error 后 5 轮内换道率（对照 R3 mteb 基线：52 次拒绝/
   865s 错误墙）；重复同 URL 重试次数显著下降。
5. 通用统计：命中率 ≥90%、零真实 400、token/墙钟对照 R3 同题。

## 6. 开放问题（定稿前需裁决/澄清）

1. 标注健康度的更新时机：每激活一次探活 vs 每次导航后滚动更新（后者更准、
   成本更高）——建议激活首探 + 首次导航失败后升级标注。
2. 「导航全败」的机械判定窗口（连续 N 次不同 URL 均 chrome-error 判车道坏死）
   ——建议 N=3。
3. mteb 镜像「curl 可用而 Chrome 全挂」的镜像侧根因是否追打（镜像
   `:20260430` 与众不同），还是仅靠本设计兜底——建议仅兜底，镜像不追打。
4. `framework_fallback` 收窄后旧会话回放/verifier 兼容（schema 枚举不变，
   reason_code 增量，verifier 放行新增值）。

## 7. 排期建议

- S1 设计定稿（本稿复核 + 开放问题裁决）→ S2 实施（orz 双车道 + 信封教学 +
  评测侧注入健壮化随批）→ S3 重建 → S4 实机复验（复用 R3 未通过集中检索
  主导 4 题做对照，或搭 0o 批次 L）。
- FP-3（web_search 慢通道失败率）不单独立项：作为 0d 后续 3/5 的真实样本
  引用 R3 journal；FP-4/FP-5/FP-6/FP-9 保持观察登记；FP-7/FP-8 为口径注记
  （TB2.1 基建形态，与 0r 同族）。
