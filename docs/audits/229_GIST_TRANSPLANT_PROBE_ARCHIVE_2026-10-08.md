# 229 批：CONTEXT-GIST-TRANSPLANT-PROBE 立案存档（candidate）＋压缩凹陷 v0 初测（2026-10-07）

> **用户令**：「gist 移植本身就相当于是每个checkpoint必模型压缩一次吧？这个可以存档一下用当作大层面的
> 研究探针」「我考虑可以简单先做压缩凹陷测量」。
> **性质**：candidate 研究档存账＋零成本离线初测——零源码、零跑批；**计数不变 55**（candidate 不计）；
> 未提交、未推送。**主档**：[`CONTEXT_GIST_TRANSPLANT_PROBE_2026-10-07`](../CONTEXT_GIST_TRANSPLANT_PROBE_2026-10-07.md)。

## §1 存档内容（主档四件套）

1. **假设面**：命题 a（事件 vs 记录）／命题 b（清晰度 vs 窗口）＋竞争解释（正确性有上下文反馈回路、
   质量没有）三者同存，判别依赖 P3/P4。
2. **等价观察（用户）**：gist 移植 ≈ 每 checkpoint 强制压缩——**按模型面内容成立、按流连续性不成立**；
   探针因此＝内容匹配、连续性二分（reset+gist 落向连续侧 ⇒ 内容/gist 起作用；落向素跑侧 ⇒ 流连续性起作用）。
3. **探针清单**：P1 压缩凹陷（本批 v0）／P2 0cw 对照组（已立项）／P3 gist 移植（candidate 主探针，
   触发器＝0cw S4 收口后）／P4 质量反馈（自跑轮限定）。
4. **学界相关工作**：检索命中（ACON 2025 `arXiv:2510.00615`／2026-01 自主记忆管理／2026 压缩综述＝
   「充分状态近似问题」与命题 b 同构／Awesome-Agent-Context-Compression）＋经典线（induction heads、
   ICL 即隐式权重更新、gist tokens、AutoCompressors/ICAE、MemGPT/Generative Agents 反思、lost in the
   middle、fuzzy-trace）。**增量位置**：未见「重置型基准 checkpoint 边界的内容匹配/连续性二分移植对照」。

## §2 压缩凹陷 v0 初测读数（零成本离线；自校验过）

- 数据＝0cr 36 题 max-checkpoint snapshot 去重 journal；复现件 `0cr_official/compression_dip_v0.py`
  （仓外）→ `compression_dip_v0.json`。**自校验：压缩计数 186 与台账 186 恰等**；题 36／run 200
  （196＋4 拆双 run 如实记）／步 4,493（报告 4,593 的 −2.2% snapshot 覆盖差，如实记）。

| 窗口（K=5 步内工具调用） | n | exit≠0 | 重复调用 |
|---|---:|---:|---:|
| 压缩后窗 | 850 | 2.5% | 3.1% |
| 基线（其余全部） | 5,308 | 2.1% | 2.6% |
| 基线（run 后半段） | 2,021 | 1.6% | 6.8% |

- **判读**：压缩后窗相对基线**无凹陷尖峰**（差值在噪声量级）；run 后半段重复率 6.8% 高企＝重复调用是
  后段常态（测试重跑类）、与压缩无特异关联。**与命题 a 相容**（折叠丢记录后无即时失措）——但只是
  「无明显即时凹陷」的否定性证据：exit≠0／重复均为弱代理（合法失败/合法重跑混入）。v1 方向＝
  checkpoint 级相关＋0cw 对照。

## §3 台账

- BACKLOG：本批记录指针（228）＋优先级总览「条件触发」行收编 probe；计数行不变（55）。
- TODO：计数行指针（228）。
- BACKLOG 第二卷：§1.175。
- 索引：头行 v4.200 → **v4.201**；§6 新增 `CONTEXT-GIST-TRANSPLANT-PROBE` 条目（candidate）；§8
  candidate 桶收编。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑（§4）。

## §4 关联与关键词

[`226 批档`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md)（0cr 形态勘正与本探针的共同来源）／
[`207 全轮报告`](../SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md)／
[ACON (arXiv:2510.00615)](https://arxiv.org/abs/2510.00615)／
[Awesome-Agent-Context-Compression](https://github.com/YerbaPage/Awesome-Agent-Context-Compression)。

关键词：229 批、gist 移植 candidate、事件特征可读性、内容匹配连续性二分、压缩凹陷 v0、186 恰等、
无即时凹陷、弱代理边界、充分状态近似、计数 55 不变、索引 v4.202。
