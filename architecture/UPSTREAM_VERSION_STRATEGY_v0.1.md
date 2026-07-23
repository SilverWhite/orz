# Upstream version strategy v0.1

状态：2026-07-23 生效；不修改 LIF protocol、reason code、gate 或 claim 语义。
Grok 单一底座与多源借鉴定位见
[`PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`](PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md)。

## 1. 决策

本项目不把某个 Grok Build 版本永久固定为产品上限。版本状态分为三条轨：

1. **historical baseline**：已经完成可复现实测的旧版本，用来重放历史 artifact、定位回归和比较行为；
2. **current candidate**：从官方 stable 指针发现、已核验本地身份、正在通过本项目 conformance gate 的升级候选；
3. **selected default**：全部必要门禁通过、由独立 promotion 提交写入 checked-in lock 的默认版本。

`upstream/grok-build.lock.json` 当前记录已提升的默认版本 `0.2.111`。
`upstream/grok-build.candidate.json` 保留 `0.2.111` 从 candidate 到 selected default 的完整 receipt，其中
`baseline` 继续指向对照版本 `0.2.106`，不随默认 lock 重写。promotion 裁决见
[`GROK_UPSTREAM_PROMOTION_2026-07-23.md`](../docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md)。

## 2. 为什么不能简单追 latest

官方 stable 指针、公开 changelog、开源同步仓库和 Windows binary 是四种不同证据：

- stable 指针回答“当前分发什么版本”；
- changelog 回答“公开说明了什么”，可能滞后；
- GitHub snapshot 回答“同步出的源码有什么”，不证明对应某个 binary build；
- 本地 binary 的长度、哈希、PE、Authenticode 和 `--version` 回答“实际下载了什么”，不证明行为兼容。

因此 latest 只是候选来源，不自动成为默认。相反，旧 lock 也只是重放锚点，不得阻止更优候选进入验证。

## 3. Promotion gate

候选至少需要：

| Gate | 目的 |
|---|---|
| binary identity | 固定 URL、长度、MD5、SHA-256、签名者、版本/build ID |
| ACP initialize | 比较 protocol、capability、扩展与无模型隔离 |
| fake tool allow | 核验 permission、tool terminal、两轮 provider continuity 与三路证据 |
| fake tool cancel | 核验 cancel、无 completed tool、无第二次 primary request |
| Windows child-tree timeout | 以 tool timeout、background task cancel、parent exit 三场景核验 descendants 的 bounded teardown 与输出回收边界 |
| DeepSeek reasoning continuity | 核验 tool 后续轮次保留 provider 所需 reasoning |
| repository regression | Windows/Ubuntu CI 的 schema、fixture 与文档完整性 |

门禁失败时有三种合法结论：修复 LIF 专项科学保障层中的窄 adapter、等待/跳过该上游版本、或缩小
capability claim。不得为了升级而修改 LIF 协议语义，也不得把未验证的上游行为写成已采用能力。

## 4. 选择更优内容

版本选择不是只看版本号。优先级为：

1. 修复准确性、取消/进程泄漏、session persistence、compaction、permission 和 ACP 一致性的上游能力；
2. 减少本项目平行实现，能够由 Grok runtime 原生承担的通用能力；
3. Windows 实测可用且能被现有留痕/独立 verifier 重建；
4. 不削弱 DeepSeek thinking/tool continuity、凭据隔离和 LIF 科学保障 hard gates；
5. UI、主题或云端功能不优先于上述科学保障边界。

当前源码 compare 暴露了 ACP tracker、background task、compaction relocation、managed config、
session persistence、permission 和 worktree 等相关变化。这些是升级审查的候选收益，不等于已证明与
`0.2.111 (94172f2aa4)` binary 一一对应。

## 5. 工程约束

- launcher 接受显式 release metadata；省略时使用 checked-in default lock，避免跟随网络 latest 静默漂移。
- 候选下载到 `.tools/grok/<version>/`，不覆盖其他版本、不修改 PATH、不自动登录。
- `.observed-runs` 按版本分目录，原始 transcript 与失败链不进入 Git。
- checked-in candidate receipt 记录发现时刻和局限；stable 指针变化时新增审计，不原地伪装成同一观察。
- “默认版本切换”必须是单独提交，不能夹带在 probe 或文档修复中。
