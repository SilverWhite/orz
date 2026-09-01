# DeepSeek V4 Flash 能力核验与减法方向调研（2026-08-29）

> 性质：调研记录（只读核验，不产生设计裁决）。背景：2026-08-29 用户
> 提问「关键是 deepseek 到底能力够不够，我们的减法到底应不应该做，
> 目前全部减法都建立在当前模型能力足够的基础上」。本文回答两个问题：
> ①模型能力是否足够（独立证据）；②减法方向在证据下的正确边界。
> 权威关系：本文是 THIN-HARNESS-REDESIGN 系列 §1 前提的核验输入；
> 若后续修订设计文档，以本文 §4 的修正为准。

## 1. 结论摘要

1. **模型能力足够——成立**。DeepSeek V4 Flash 0731 的 82.7%（TB 2.1）
   与 89.0%/61.4%（ARC-AGI-1/-2）均有独立第三方背书：Ante 公开 Harbor
   run 复现 82.7%（368/445）；ARC Prize 独立验证 ARC 双榜；OpenCode
   1.18.7 在 67.42% 基线上经 autoprompt 到 82.02%；Kinsley 在完整
   harness（OMP）下复现 71.9%（同模型同 benchmark）。
2. **减法方向成立，但边界必须收窄**：证据支持「模型可见面减法」
   （prompt/注入块/工具面收敛），**不支持「框架整体简化」**。Kinsley
   对照实验证明同一模型在粗糙极简 harness 下仅 49.4%、完整 harness 下
   71.9%——执行侧成熟度是能力放大器，不是摩擦源。
3. **本地设计文档有一处关键事实错误**：THIN-HARNESS-REDESIGN §1 写
   「82.7% = 4 工具、短提示、no skills、bare profile」，实际官方
   minimal mode 只有 **2 个工具**（bash + str_replace_editor）；Ante
   复现用的是 **Ante 默认完整工具面**（仅关 skills/session-save）。
   "4 工具"是 Ante curated `pi` profile（另一个实验），不是 82.7% 那次。
   该记载持续诱导「框架也可以更薄」的错误推论，需修正。

## 2. 模型能力独立验证（证据链）

| 来源 | 配置 | 结果 | 性质 |
|---|---|---|---|
| ARC Prize | DeepSeek V4 Flash 0731、max effort | ARC-AGI-1 89.0%；ARC-AGI-2 61.4%（$0.02/$0.04 每 task） | 独立验证（非 harness 相关） |
| Ante（Antigma Labs）0.preview.71 | 0731、max effort、TB 2.1 89 题 × 5 trials | **82.7% ±1.79 SE（368/445）**，$68.41，38.9 min/trial | 独立复现，公开 Harbor run |
| OpenCode 1.18.7 | 0731、TB 2.1 89 题 | 基线 67.42%（60/89）→ +autoprompt **82.02%（73/89）** | 非官方 harness 复现 |
| Kinsley（sentdex） | 0731、TB 2.1 89 题 | 自写粗糙 harness（minion）**49.4%**（44/89）vs 完整 harness（OMP）**71.9%**（64/89） | 同模型同 benchmark 对照 |
| Ante 官方对照（10 题 × 1 次） | deepseek-v4-flash-0731、high effort | Ante 10/10；Ante-short 9/10；Pi 7/10；OpenCode 7/10；Hermes 7/10 | 同模型跨 harness 单次对照 |

关键读数：

- 82.7% 不再是「仅官方 harness 宣称」——Ante 用公开可审计 Harbor run
  复现出完全一致的 82.7%。
- 同一模型在 harness 上的分数摆动达 **22.5 个百分点**（49.4% ↔ 71.9%），
  且完整 harness 是加分方向；GLM-5.2 同实验只摆动 4 题（61→65），说明
  **DeepSeek V4 Flash 是 harness 敏感模型**。
- Ante 官方对照里，完整工具面 Ante 10/10，仅短提示的 Ante-short 9/10：
  短提示损失小、成本与延迟双降——支持「模型可见面减法」。

## 3. "82.7% 的配置"事实核对（修正本地记载）

| 声称 | 事实 | 来源 |
|---|---|---|
| 官方 82.7% 是「4 工具」 | 官方 DeepSeek Harness **minimal mode 只有 2 个持久工具：bash + str_replace_editor**，无 web search、无 skills、无子代理、无规划 | DeepSeek API 文档脚注；第三方复述（grapeot / digitaltoday / essamamdani） |
| Ante 82.7% 是「4 工具 bare profile」 | Ante Harbor 适配器默认参数 `--yolo --output-format json --no-session-save --no-skills`，**未限制工具集**——Ante 默认完整工具面；bare profile 仅关 skills/MCP/session/auto-memory，不裁工具 | [ante-harbor/ante_agent.py](https://github.com/AntigmaLabs/ante/blob/main/ante-harbor/ante_agent.py) `DEFAULT_ANTE_ARGS` |
| 「4 工具」是什么 | Ante curated `pi` profile（Read/Write/Edit/Bash）——**独立实验**，不是 82.7% run | [Ante README](https://github.com/AntigmaLabs/ante) |
| 「短提示」依据 | Ante `--short-prompt` A/B（TB 2.1 全 89 题 × 1 次）：长 ~5.0K 字符 51/89 vs 短 ~2.3K 字符 53/89，性能打平；同结果子集输入 token 降 32% | [Antigma blog 2026-07-25](https://antigma.ai/blog/2026/07/25/short-prompt-small-models) |

推论修正：

- 官方 minimal mode 的「极简」是**模型可见面极简 + DeepSeek 自家成熟
  harness**（协议、重试、事件流、工具契约），不是「框架可以粗糙」。
- Ante 的 82.7% 是**完整工具面 + 成熟执行核**（5 分钟模型流式超时、
  进程组 kill、结构化 TurnEnd 错误分类、provider 适配）。
- Kinsley 的 minion（49.4%）才是「真·粗糙极简」——它证明模型面极简
  若叠加执行侧粗糙，分数会断崖。

## 4. 对减法方向的判定

1. **模型可见面减法：该做，证据无损**。Ante 短提示 A/B 打平且省 32%
   输入 token；Claude Code 减 80% system prompt 同向。本项目「prompt
   全空 + 8 工具 + 软门」的模型面方向正确。
2. **执行侧减法：不该做，且当前失败主体恰在执行侧**。S4 失败画像
   （THIN-HARNESS-REDESIGN V2 §9.5）：23 失败 = 18 超时（12 个
   web_search 无超时黑洞 + 6 个慢命令）+ 2 个 fold 桥剥
   reasoning_content 的框架 400 + 3 个真实交付质量。失败主体不是模型面
   摩擦，是工具效率与执行成熟度。
3. **一个扎眼对照**：官方 minimal mode 没有 web search；我们 8 工具
   保留 web_search/web_fetch，12/18 超时挂在 web_search。已定案的
   web_search 120s 超时、终端分层超时、5 分钟中间回报正是补官方/Ante
   本来就有而我们缺失的执行件——不是「又叠机制」。
4. **对「减法建立在模型能力足够之上」的回应**：
   - 前提成立（ARC + Ante + OpenCode + Kinsley OMP 四路独立验证）。
   - 推论需修正：能力足够 → 模型不需要 prompt 脚手架（减模型面）✓；
     能力足够 → 框架可以整体简化 ✗。Kinsley 49.4% ↔ 71.9% 即反例。

## 5. 待修正文档位置

- [`THIN_HARNESS_REDESIGN_DESIGN_2026-08-27.md`](THIN_HARNESS_REDESIGN_DESIGN_2026-08-27.md)
  §1「差距定位」句（4 工具/bare profile = 82.7%）。
- [`THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md`](THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md)
  §1 背景动机首条（"官方极简 harness"措辞需与本文 §3 事实对齐）。
- CLI_PROJECT_INDEX 无直接该记载；若登记本调研，按索引纪律补条目。

## 6. 后续动作

- 2026-08-29 用户指示：先落本调研记录，然后对当前框架做**全量机制
  盘点**（「机制侧东西已不少，很多在外部机械层」），输出「当前生效的
  实际设计清单」——以源码为准，对照设计文档标记 implemented/partial/
  退役差异，再决定下一步清理方向。
