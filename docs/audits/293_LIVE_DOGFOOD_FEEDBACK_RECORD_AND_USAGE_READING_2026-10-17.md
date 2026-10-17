# 293 批：真机使用反馈四项取证登记（web 桥 0.9.2）＋0df S4 首批真机计量读数（2026-10-17）

> 性质：**记录批**（只读取证＋登记；用户令「请将其记录下来」）。
> 零源码、零 orz 改动、零跑批（本批未发起任何 orz 会话）；零重建、零发行；
> 装机位保持 0.9.2（290 批换装）。计数不变 62。

## §1 由来（用户四项反馈）

用户 2026-10-17 个人真机使用后反馈（原话要点）：

1. shift+enter 在消息发送后，排版是没有换行的，具体情况见消息存档，不便
   于消息回查，现在全都混在一起；
2. ctrl+z 目前不好使了；
3. 现在是不是还没有实装撤回和中途补充功能？中途补充的话我考虑是自动接在
   当前思考/动作结束后，不中途打断模型；
4. 感觉消耗低了非常多，感觉没有幻觉但是消耗低了好多。

本轮测试落点＝`C:\Users\1\OneDrive\Desktop\新建文件夹`；载体＝web 桥装机位
0.9.2（`.gsa` 内 `chrome-profile-b41b096d/` 与 ACP 会话形态为证）；活跃会话
`b41b096d`（3 run：17:18／18:17〔两个检索目录〕／19:42 起，19:49 仍在更新）。

## §2 取证（只读）

### N1 shift+enter 多行输入：存档保留换行、web 渲染折叠（缺陷成立，web 层独有）

- **输入面正常**：`orz-web/assets/app/main.js:732-736`——Enter（无 shift）提交、
  Shift+Enter 走 textarea 原生换行。
- **存档面正常**：`conversations/b41b096d.json` 用户消息 `\n` 逐字保留
  （本轮 R222 前文 2,253／续问 808／补发 1,541 字符三条均含多换行；消息体
  键 `role/content/tool_call_id/...`）。
- **渲染面缺陷（根因）**：`orz-web/assets/app/widgets.js:388-389`——模型卡走
  `renderMarkdown`（`\n` 转段落），**用户/系统卡 `body.textContent` 直填**；而
  `orz-web/assets/app.css:242` `.msg-card .msg-body` 只有
  `padding/overflow-wrap`、**无 `white-space: pre-wrap`** ⇒ HTML 默认
  `white-space: normal` 把 `\n` 折叠为空格 ⇒ 多行输入在对话卡里渲染为连续
  一段（「全都混在一起」）。journal 回放（`projection.js:48-50`
  `prompt_submitted` → `vm.addUserMessage`）与 ACP 直投同经此渲染点。
- **波及面**：仅 web 层；orz-tui 为终端逐行绘制不受影响（285 批「双实现同步
  纪律」本件不适用——TUI 无此缺陷）。
- **修复候选（待裁决后实施）**：用户/系统卡 CSS 加 `white-space: pre-wrap`
  （一行）；是否改走 markdown 渲染留裁决。

### N2 Ctrl+Z：web 全局键位＝「取消运行」语义；闲置期静默、输入框内抢占原生撤销

- **键位事实**：`orz-web/assets/app/keymap.js:79-83`——Ctrl/Cmd+Z 全局绑定
  `stopRun`（注释即「Ctrl+Z 取消运行」）；该分支在 `inInput` 豁免之前 ⇒
  **输入框内按下同样被抢占**（原生文本撤销死）。
- **静默面**：`main.js:140-145`——`!acpSessionId || !state.running` 时直接
  return，**无任何用户反馈**（对照运行中按下有 `flash('已发送取消')`）。
- **服务端链路完整**：`acp.sessionCancel`（acp.js:135 notify `session/cancel`）
  → 泵转发（`orz-web/src/acp_pump.rs` 双向透传）→ `orz-host/src/acp_server.rs`
  `cancel_current_run`（Phase 3 slice #7：live token 取消＋bootstrap 窗
  `pending_cancels` 记忆，代码注释齐备）⇒ 机制在位。
- **本轮 journal 无样本**：今日 3 run `run_finished` 全为
  `completed/conclude`、零取消痕迹 ⇒ 「运行中按下无效果」无法从存档确证。
- **候选读数（待真机复现取证）**：① 闲置期按下＝静默无反馈（按设计 return，
  体感即「不好使」）；② 输入框内按下＝被全局键位抢占、若预期为文本撤销则
  落空；③ 运行中若确无可见效果，需下一轮真机抓复现（桥端 flash＋服务端
  journal 取消族事件双取证）。
- **处置候选（待裁决）**：输入框内豁免（还原生撤销）；闲置期给「当前无运行
  中」反馈；语义是否拆分（文本撤销 vs 取消运行）留用户定。

### N3 撤回／中途补充：未设计、未实装（用户判断属实）；提议语义原样登记

- **全仓检索零命中**：`撤回`（产品语义）／`中途补充` 在 docs/、ADR、BACKLOG、
  web/TUI 源码均无设计与实现痕迹 ⇒ 两功能**未设计、未实装**。
- **运行中补充输入的现行为**：`main.js:76-79`——`state.running` 时
  `submitPrompt` 直接拒绝（flash「当前有运行在进行（Ctrl+Z 可取消）」），
  无排队／附加机制。
- **用户提议语义（原样登记，留设计裁决）**：中途补充**自动接在当前
  思考/动作结束后**投递，**不中途打断模型**。
- **撤回语义待定义**：用户未细述；涉及会话存档与上下文面的裁剪/标记语义
  （已发送消息如何从模型上下文与存档两面处置），需设计裁决后再定形态。

### N4 消耗读数：0df 计量真机生效＋首轮量化对照（下降属实，−58%）

**0df `run_usage_metered` 首批真机读数**（287 S1+S2／289 S3 进体／290 装机位
换装后首次真机服役；3 run 各恰 1 条、`usage_incomplete=false`）：

| run | calls | prompt | cache_hit | 命中率 | cache_miss | completion | reasoning |
|---|---|---|---|---|---|---|---|
| RUN-b41b096d-0 | 68 | 6,600,877 | 6,401,024 | 97.0% | 199,853 | 92,149 | 42,782 |
| RUN-b41b096d-1 | 22 | 3,346,745 | 2,909,952 | 86.9% | 436,793 | 44,925 | 20,458 |
| RUN-b41b096d-2 | 18 | 3,552,112 | 3,303,168 | 93.0% | 248,944 | 48,236 | 20,645 |
| **合计** | **108** | **13,499,734** | **12,614,144** | **93.4%** | **885,590** | **185,310** | **83,885** |

**对照**（10-13 会话 2b1f85d2，0.9.x≤0.9.1 无计量事件，自 `model_output`
逐次 usage 聚合；`RUN-2b1f85d2-0/1` 两 run）：

| run | calls | prompt | 命中率 | cache_miss | completion | reasoning |
|---|---|---|---|---|---|---|
| RUN-2b1f85d2-0 | 63 | 8,131,386 | 88.5% | 935,610 | 210,425 | 156,348 |
| RUN-2b1f85d2-1 | 40 | 9,839,701 | 80.4% | 1,932,885 | 965,938 | 887,111 |
| **合计** | **103** | **17,971,087** | **84.0%** | **2,868,495** | **1,176,363** | **1,043,459** |

**读数结论**（两次会话同为 FEP 讨论评判类任务，模型 `deepseek-flash`、
`effort_tier=max` 同档，可比性有限但方向明确）：

- **等效成本代理**（`miss + 0.1×hit + completion`，miss/completion 全价、
  hit 约 1/10 计）：**5.56M → 2.33M（−58%）**；其中 miss −69%、
  completion −84%、reasoning −92%；prompt 面总量 −25%。
- **驱动因素（如实记，不作因果定论）**：
  ① **缓存命中率 84.0% → 93.4%**——miss 是计费大头；会话内前缀稳定
  （`face_fingerprint` stable prefix 连续）＋ run 间隔更近利于 provider 缓存
  存活；
  ② **压缩负载骤降**——旧 run1 有 7 次 `context_compressed`（模型摘要压缩
  本身集中消耗大量 completion/reasoning：该 run 966K/887K），新会话 3 run
  合计仅 1 次；
  ③ **单轮输出显著变短**——completion/轮 11.4K → 1.7K、reasoning/轮
  10.1K → 0.8K，属模型侧行为（deepseek-flash），非 orz 改动所致；
  ④ **0.9.2 自身改动面均不减模型面 token**（285 渲染三修＝显示层；286 检索
  门缺省开反而新增工具面；287-288 计量＝旁路事件）⇒ **消耗下降非 0.9.2
  有意优化的结果**，主因＝缓存命中＋压缩负载＋模型输出行为。「没有幻觉
  但消耗低」与读数自洽（输出短、推理少、评判任务一次收束）。

**附带核证（同批登记）**：**286 检索门缺省开真机生效首证**——
`activations/b41b096d.json` `retrieval_enabled: true`；RUN-1 一次外部检索
（`browser_launch_result` 1＋`retrieval_result_committed` 1＋
`information_sufficiency_assessment` 1＋`serp-attempts/`、
`retrieval-results/` 落盘；`external_ret.web_sources` 收 Europe PMC 题录面
若干条）。

## §3 处置

- **登记 `OBS-WEB-DOGFOOD-2026-10-17`**（`candidate`，三件 N1/N2/N3，仅记录
  不计数；升级、修复与优先级留用户裁决）：N1 修复候选＝CSS 一行（用户/系统
  卡 pre-wrap）；N2 处置候选＝输入框豁免＋闲置期反馈＋语义拆分裁决；N3＝
  撤回语义定义＋中途补充设计立项与否待裁决（用户提议语义已原样登记）。
- **0df S4 首批真机计量读数登记**（本档 §2 N4；不作闭合裁决——0df S4 判据
  与闭合留用户裁决，与前例 285 批 0dd/0de 读数登记同口径）；**286 缺省开
  真机生效首证**同批登记（286 本身无 S4 台账项，此为事实登记）。
- 零源码、零跑批、零重建、零发行；计数不变 62。
