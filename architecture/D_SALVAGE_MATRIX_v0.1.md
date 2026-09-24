> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# Project D 源码 salvage matrix v0.1

状态：2026-07-21 定向源码审计；只迁移能力级经验，不迁入 D 产品、人格、云服务或旧记忆。

## 1. 来源与边界

Drive 来源：[`D(存档)`](https://drive.google.com/drive/folders/1yxnaY_ErMyrQdmZxN6Cf2zw5ZWWHMaPg)。
本轮通过 Google Drive for desktop 流式挂载，只读访问 `G:\我的云端硬盘\D(存档)`。

D 自带 README 明确称这些版本简陋、漏洞多、早期主要靠 prompt 约束，并建议只重点看“设定及相关补充”
与存档一设计。因此本文件把 D 视为候选模式来源，不把它当作已验证架构。

明确排除：`.env`、`.dev.vars`、`token.txt`、`node_modules`、`__pycache__`、人格/社交/语音/Live2D
产品面。`D（存档0）/二号存档/config/settings.yaml` 中发现明文 provider credential；本文件不复制其值，
用户确认这些 credential 已在归档时全部更换，因此当前不把它们列为 active incident；但“密钥明文进入
配置/存档”仍是本项目明确拒绝的工程模式。

## 2. Source ledger

以下是本轮实际读取并计算 SHA-256 的文件。hash 只证明本轮所读版本，不证明代码正确。

| 相对路径 | SHA-256 |
|---|---|
| `README.txt` | `407c1aa9cd25541e8580e5a9e364b7ab33db95f83ebba7b03bd50f73b9094611` |
| `D（存档1）/cloud-brain/src/types.ts` | `8377a1fd8ae4c6147620445d304e16fa71ae74663972a12fe6d20046c0242ba1` |
| `D（存档1）/cloud-brain/src/core/state.ts` | `b7ec91617e8b7be9113a728bf514d1c4da1deced98f91737cb28cf21d8a5efbf` |
| `D（存档1）/cloud-brain/src/core/brain.ts` | `250844afab4acdc9e7e6f4a76fe30096c4e6f8a9d42c412bcc806a2630ae2cd0` |
| `D（存档1）/cloud-brain/src/services/memory.ts` | `94d1f0d81fbda1da22b19a865da1a3f0f468a9537af5fc299be510ac34f972ce` |
| `D（存档1）/cloud-brain/src/core/tools.ts` | `07f8d0c2380d6ca82d6950bb79d4d2b26da7274a8941a3833781b3e02689757d` |
| `D（存档1）/cloud-brain/src/services/llm.ts` | `08a36400c74b0e9b9eb45abafa4bfff68b2830ee2daeba7481698d1bea2f8287` |
| `D（存档1）/cloud-brain/src/core/mcp_remote.ts` | `553fcd397cf686ae2403bee23765418fd9cfc330c08252fe3cf913f25e8e7cc3` |
| `D（存档1）/cloud-brain/src/core/mcp_types.ts` | `7d7a9d631844a0d18ba3b20eb821921793c9eeb073007ad605fcbc2e7c512f44` |
| `D（存档1）/cloud-brain/src/handlers/scheduled.ts` | `451ea079f3d836b088bb0cfa85238504b48a688f0e611757e78bfc30c842d77f` |
| `D（存档1）/cloud-brain/src/index.ts` | `cb0c80330479d284f2096b102b3d8360ce759ebc7a2a10eaca6b1649f545ecab` |
| `D（存档1）/cloud-brain/schema.sql` | `55a6fa0a3b2601f73df6abd46af0c6b2ec60facda3d6f367d5273c5cc222b08f` |
| `D（存档1）/cloud-brain/package.json` | `fc1a5585abba0ca18379dee2893c3da4a9f8047558014695f4462ca402c0cf7b` |
| `D（存档0）/一号存档/设定及相关补充内容.txt` | `a0732f83c963c918bbe9d2f2bec5558a2d70a06bfd30eb153e0b9cfa29a8ffe1` |
| `D（存档0）/一号存档/prompts_deepseek.py` | `bbf46077e49c7ac2e38cd418e7aaa5e3ac26866c9a16687e49b3612e3320cb54` |
| `D（存档0）/二号存档/src/safety.py` | `ac74b3065fa60ae222da93d8ccc703fa0fa2868f83b1796d10d81c627652d056` |
| `D（存档0）/二号存档/config/settings.yaml` | `e43347f5d08bd44310d9bc230748cb43919ba546b1dc7b885190e63fdb23816d` |

`local-body/main.py` 和 `vts_client.py` 的 metadata/文件大小已确认，但正文按需下载持续返回
`network name is no longer available`，所以不把其行为列为 source-grounded；对应本地执行端仍是 unchecked。

## 3. 裁决矩阵

| D 中的机制 | 裁决 | 当前项目映射与理由 |
|---|---|---|
| cloud-brain / local-body 分离 | adapt locally | 只映射为 Grok runtime + Windows supervisor；云端 runtime 已由 ADR-0002 延期，不用 Cloudflare/公网 body API |
| analytics 表是事实源、state 是缓存 | adopt | event journal 为 authoritative；summary/state 只能由 journal 重建 |
| 输入先读历史、再保存当前消息 | adopt concept | 防止同一输入重复注入，但必须以事件 sequence 而非时间猜顺序 |
| tool result 的 `content + isError` envelope | adapt | 使用 Grok typed events，并补 exit/terminal/artifact/provenance |
| 有限工具轮次 | adopt | manifest 冻结 max-turns/token/tool/time budget，超限产生明确 terminal event |
| 归档全部成功才删除原日志 | adapt | 保留 no-data-loss 原则；改成幂等事务和 content-addressed chunk，避免部分成功后重试重复 |
| 高风险操作二次确认 | adapt | 不用裸 `y/n`；改为 action-digest-bound、one-shot、TTL permit + Grok permission/sandbox |
| pending command queue | adapt | 必须有 UUID、schema、expiry、idempotency key、ack、receipt 和唯一 terminal state |
| “真实准确优先、Ask Don’t Guess、工具数据优先” | adopt as policy | 转成 source/evidence gate；不依赖角色 prompt 自觉执行 |
| provider `reasoning_content` 连续性 | adopt as conformance | 只用于 DeepSeek协议连续性；原文进入 sealed private，不进入 evidence |
| memory/vector recall | defer/default-off | 普通会话可用；LIF claim/evaluation 要 blind-first、来源标记、countercase 和污染控制 |
| 自动定时唤醒与主动外发 | reject V0 | 与本地研究 CLI 无关，增加不可控 side effect |
| 任意 `run_python` 远程执行 | reject | 源码注释已承认 body server 未检查 auth；无 timeout/sandbox/receipt |
| 400 后静默删除历史重试 | reject | 改变请求语义且污染 provenance；应失败并要求新 manifest/显式决策 |
| 读取失败返回默认 state / 空 memory | reject | 把 unavailable 伪装成真实空值；必须区分 `not_found`、`unavailable`、`invalid` |
| model summary 直接成为长期记忆 | reject for evidence | 摘要是派生产物，必须保留 source refs/置信边界，不能升级成事实 |
| 泛化 preload query 自动注入历史 | reject for LIF | 会制造先验污染并覆盖 blind-first |
| 正则删除输出中的“系统/决策”文本 | reject | 静默改写模型输出；应保留 raw、另生成 redacted derivative 和记录 |
| raw thought、tool args、query 写普通日志 | reject | 可能泄漏 reasoning、prompt、路径或 secret；按 audit/sealed/never-record 分层 |
| `Date.now()` command ID / queue success before receipt | reject | 可冲突且把 queued 误报为 executed |
| 明文 API key 与固定旧模型/价格 | reject | credential bridge + current capability snapshot；费用由实际 usage/profile 计算 |

## 4. 对 observed wrapper 的直接影响

下一 implementation spike 保留 D 的“事实日志优先、脑/身分离、危险动作确认”三个方向，但用当前仓库已有
immutable manifest、hash-chain journal、Windows Job Object、Grok permission/sandbox 和 DeepSeek conformance
重写。尤其新增以下硬要求：

1. 任何 fallback/default 必须记录来源，读取失败不得变成默认事实；
2. queued、started、completed、failed、cancelled 分开，queued 不等于 executed；
3. 每次 retry 保留原请求 digest 和语义变化，禁止静默丢 history；
4. raw output 永不原地清洗，redaction 产生可追溯派生 artifact；
5. memory 对 claim-bearing run 默认关闭，模型摘要永远是 derived/unverified；
6. actual execution 前要求 digest-bound approval；dry-run 只生成计划，不调用模型。

## 5. 未核风险

- `local-body/main.py` 未成功读取，D 本地 body 的实际 auth、subprocess、pyautogui 和 system-control 实现仍未核。
- 未运行 D，也未验证 Cloudflare/D1/KV/Vectorize 的部署状态。
- 未读取人格、社交、语音、视觉、GitHub MCP 的完整实现，因为它们不属于当前 observed wrapper 的最小范围。
- DriveFS 流式读取间歇失败；source ledger 不代表 D 全目录完整性。
