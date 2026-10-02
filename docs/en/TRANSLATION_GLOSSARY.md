# orz English Translation Glossary (D1)

> **Status**: `v1.0` (2026-10-02, 0cm closure) — seed table landed from the charter appendix
> ([charter §Appendix A](../EN_TRANSLATION_PROJECT_CHARTER_2026-10-02.md)), refined through the
> first translation round and the later independent review passes, then frozen on the user's
> **English-README sign-off (2026-10-02)** with no term changes requested. This file is the
> terminology authority for all `docs/en/` deliverables; later amendments follow §4 (change the
> table first, then regenerate the affected documents).
>
> LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.

## 1. Purpose

1. Fix one English name per Chinese project term, so every English document uses the same word
   (injected into every translation call; no cross-document drift).
2. Record the **document mapping**: which English document translates/distills which Chinese
   authority document, pinned to which version — so a later batch that revises a subsystem
   knows exactly which English documents to regenerate.

## 2. Term table (Chinese → English)

| Chinese | English (fixed name) | Notes |
|---|---|---|
| 载体 | carrier | The shipped/install form: install directory + the three binaries + rollback point |
| 三件套 | the three binaries | `orz` / `orz-signer` / `orz-acaf-provision` |
| 写控 / 机械写控 | write control / mechanical write control | The write-protection subsystem |
| 灾难保底 / 硬边界 | catastrophic hard backstop / hard boundary | Purpose word: guards against unrecoverable disk-level disaster, not a general review |
| 宿主机灾难保底 | host-machine catastrophic backstop | v3+ positioning: protect host state only |
| 审批组件 | approval component | The Codex-lineage orz-workspace/permission chain that owns ordinary write approvals |
| 宿主态 | host state | The two narrow protected targets: session volume + ACAF keystore root |
| 会话卷 | session volume | `.gsa` |
| keystore 根 / 签名器清单 | keystore root / signer manifest | Already English; article usage fixed |
| 规则面 / 封闭枚举 | rule surface / closed enumeration | Exactly 5 block rules; no heuristics |
| 拦截 / 放行 | intercept / allow | Write-control contexts |
| 表外恒放行 | anything off-table is always allowed | Default-allow property |
| 装配 | wiring | e.g. 装配同源 env = wiring-sourced env |
| 机械层 | mechanical layer | The framework-side machinery: gates, guards, registry routing |
| Agent 层 | agent layer | Main agent + retrieval subagent |
| 黑板 | blackboard | Shared session state panel |
| 时间与动作域判断组件 | temporal/activity-domain component | LIF; framework-side time accounting |
| 权限桥 | permission bridge | permission gate in front of tool execution |
| 事件账本 / journal | journal | Hash-chained event log |
| 票据 | ticket | ACAF one-time signed authorization |
| 租约 | lease | `SandboxLease` |
| 签发器 | signer | `orz-signer`, the independent signing process |
| fail-closed | fail-closed | Kept as-is |
| 影子模式 | shadow mode | Log-only mode before enforcement flips on |
| 判官 | judge | Evaluation judge (scoring/replay adjudication) |
| 试次 | attempt | Smallest scoring unit |
| 题 / 任务 | task | TB context: always "task", never "problem" |
| 整轮 / 收官 | full round / final round | |
| 定向重跑 | targeted rerun | |
| 翻盘 | turnaround | Failed task flipped to 1.0 on rerun |
| k=1 筛查轮 | k=1 screening round | Not a leaderboard score |
| 榜单成绩 | leaderboard score | Requires ≥5 attempts/task |
| 载体重建 / 换装 / 进体 | carrier rebuild / swap-in / landed in-carrier | Rebuild + binary replacement + verification that new literals are in the shipped binary |
| 源冻结 | source freeze | Pinned source revision for a rebuild |
| 批 | batch | Ledger batch unit (e.g. "batch 156") |
| 立项 | project initiation (charter) | |
| 沿革 | revision history | Compressed to one line per version in English docs |
| 台账 | ledger | BACKLOG / TODO / INDEX |
| 狗粮轮 | dogfood run | |
| 摩擦 | friction | Workflow obstacle |
| 落码 / 复验 | code landed / re-verification | |
| 权威链 | authority chain | Which document is authoritative for what |
| 用户裁决 | user adjudication | |
| 擦墙通过 | at-the-wall pass | Killed by the official timeout, but delivery was already complete and scored 1.0 |
| 撞时限 | timeout kill | Killed by the official timeout without passing |
| 自完判负 | self-completed, judged fail | Model declared completion; verifier scored 0 |
| 墙钟 | wall clock | `--max-wallclock` agent-side round time limit |
| 死代码清退 | dead-code retirement | |
| 载体集 | carrier set | Write-control L1 target set: C1 session volume + C2′ keystore root / signer manifest |
| 根本性树根 | fundamental tree roots | Rule 1's recursive-delete targets (Windows four roots / Linux twelve trees) |
| 宿主态祖先链臂 | host-state ancestor-chain arm | Rule 5's sweep-verb arm (`ANCESTOR_SWEEP_VERBS`, closed subset of 15) |
| 信任锚 | trust anchor | ACAF keystore + signer manifest as a pair |

## 3. Document mapping (EN ↔ CN authority, version-pinned)

| English document | Chinese authority | Pin |
|---|---|---|
| `README.en.md` | `/README.md` (repo root) + retired D6 umbrella | 2026-10-02 **expanded edition** (the two editions mirror each other in content; the English edition is expanded in role, not in coverage — it doubles as the umbrella stand-in for the Chinese design authorities, which are not planned for translation); **user sign-off 2026-10-02**, frozen as the published English face |
| `TB21_V41_89_FULL_ROUND_REPORT_EN.md` | [`docs/TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md`](../TB21_V41_89_FULL_ROUND_REPORT_2026-10-01.md) | final-round report (batch 142 account) |
| `WRITE_CONTROL_CURRENT_EN.md` | [`docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md) (**v4.0**, current rule surface) + [`docs/WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md) (**v1.0**, surviving architecture) | 0ch S3 landed in carrier 0.8.8; S4 three-task re-verification landed via 145/150 batches, 0ch closed 2026-10-02 |
| `ACAF_CURRENT_EN.md` | [`docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md) (design authority) + in-carrier changes through **v0.8.10** (0.8.2 signing assembly fix, 0.8.7 keystore-root/signer-manifest faces) | v0.8.10 in service |
| ~~`CURRENT_DESIGN_EN.md`~~ | **retired same day** — content merged into the expanded `README.en.md` (user adjudication) | superseded 2026-10-02 |

## 4. Maintenance

- Trigger: any batch that revises a translated subsystem updates this file first (terms +
  doc pins), then regenerates the affected English documents in one LLM call with the full
  table injected.
- The disclaimer line on every English document is fixed:
  `LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.`
