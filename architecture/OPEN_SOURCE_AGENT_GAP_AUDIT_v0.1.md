> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# 开源 Agent 架构缺口审计 v0.1

状态：2026-07-21 source-grounded 工程裁决；不修改 protocol v0.1、reason code、gate、claim 或
provider 协议语义。

## 1. 结论

本审计以 Grok Build `0.2.106` Windows 预编译程序为当时的可复现实测 baseline，不重建通用 Agent CLI。
baseline 不是版本上限；后续版本按
[`UPSTREAM_VERSION_STRATEGY_v0.1.md`](UPSTREAM_VERSION_STRATEGY_v0.1.md) 进入 candidate/promotion gate。
Codex CLI、Gemini CLI、OpenHands、Aider、Goose 与 OpenCode 的成熟设计表明，本项目当前真正缺少的是
Grok 外侧的四类可验证边界：

1. 在读取项目配置、规则、hook、plugin 或 MCP 前，明确建立 workspace trust；
2. 在有副作用的 tool 前冻结可恢复 checkpoint，并把恢复动作写入审计链；
3. 用有序、append-only、可重放的事件桥补足 Grok `streaming-json` 的工具事件缺口；
4. 把 compaction 作为有来源清单的派生变换记录，不能让被丢弃内容在审计层悄然消失。

Grok 已有权限合并、`PreToolUse` hook、OS sandbox、session、compaction hook 和 structured headless
output。上述四项应采用薄 launcher/sidecar 适配，不能演变成第二套 permission、session、transport 或
tool runtime。

## 2. 方法与证据边界

- 只读取项目官方仓库的固定 commit 源码/文档；未引用二手博客或 benchmark 营销结论。
- 没有 clone、安装或执行其他 Agent，也没有把其代码复制进本仓库。
- 外部设计只提供工程候选，不证明其安全性或适合 LIF；最终裁决仍受本仓库 Windows-first、
  upstream-first、evidence-constrained 边界约束。
- Grok 行为以已核验的本地 `0.2.106 (bde89716f6)` binary 自带文档和实际 fake run 为准；
  [`grok-build.lock.json`](../upstream/grok-build.lock.json) 明确标记 binary/source correspondence
  尚未证明。
- 下表中的 SHA-256 是审计时读取到的 UTF-8 文件内容摘要，用于防止以后把更新后的上游内容误当成本轮证据。

## 3. Source ledger

| 项目 / 固定 revision | 选取文件 | SHA-256 | 本轮使用的事实 |
|---|---|---|---|
| Codex `b9800de4867e500a92add3cde795cf4790306d0f` | [`codex-rs/rollout/src/recorder.rs`](https://github.com/openai/codex/blob/b9800de4867e500a92add3cde795cf4790306d0f/codex-rs/rollout/src/recorder.rs) | `9505be36de6acd5a01076fe0dedb9c35adec538de55215e9431b967da7d2eb95` | canonical rollout 使用 JSONL，可显式 persist/flush |
| Codex 同 revision | [`rollout-trace/src/writer.rs`](https://github.com/openai/codex/blob/b9800de4867e500a92add3cde795cf4790306d0f/codex-rs/rollout-trace/src/writer.rs) | `5efa68c044b9f880e3ee1db7d208fecdea806a6e1fbdfa2f9037b74e076c6d4a` | raw event append、单调 sequence、payload 先落盘再引用 |
| Codex 同 revision | [`rollout-trace/src/protocol_event.rs`](https://github.com/openai/codex/blob/b9800de4867e500a92add3cde795cf4790306d0f/codex-rs/rollout-trace/src/protocol_event.rs) | `d832ceb1888e9594e97f1a6dbba876f63878cd55baad5e0ccd06641717936ac2` | turn、terminal、patch、MCP 和协作工具映射为 typed trace event |
| Codex 同 revision | [`session/rollout_reconstruction.rs`](https://github.com/openai/codex/blob/b9800de4867e500a92add3cde795cf4790306d0f/codex-rs/core/src/session/rollout_reconstruction.rs) | `47f1e2923255480036e065ba41f4068248201586639b43e274843e94f7faa410` | 从 rollout 重建 history/world state；完整 replacement checkpoint 成为新基底 |
| Gemini CLI `acae7124bdd849e554eaa5e090199a0cf08cd782` | [`docs/cli/checkpointing.md`](https://github.com/google-gemini/gemini-cli/blob/acae7124bdd849e554eaa5e090199a0cf08cd782/docs/cli/checkpointing.md) | `cb4261f5390134e586459de23a1fef62d2741d9e0c0b3ca1c1372c629e8a89af` | 文件修改前 shadow-Git snapshot，同时保存 conversation/tool call |
| Gemini CLI 同 revision | [`docs/cli/trusted-folders.md`](https://github.com/google-gemini/gemini-cli/blob/acae7124bdd849e554eaa5e090199a0cf08cd782/docs/cli/trusted-folders.md) | `a9d986314ad4806d356eeca678a9ea0f838851372c84979471d1ba6e0250e44d` | project configuration 读取前的 folder trust 概念 |
| Gemini CLI 同 revision | [`docs/reference/policy-engine.md`](https://github.com/google-gemini/gemini-cli/blob/acae7124bdd849e554eaa5e090199a0cf08cd782/docs/reference/policy-engine.md) | `4895bc05bccef1a16ab1442c7a9b12ebe61945f0f079e9fca3261cf8b9e2d0f5` | allow/deny/ask_user、条件、优先级与匹配规则可解释 |
| Gemini CLI 同 revision | [`invariantChecker.ts`](https://github.com/google-gemini/gemini-cli/blob/acae7124bdd849e554eaa5e090199a0cf08cd782/packages/core/src/context/utils/invariantChecker.ts) | `6d23cdc993a338c7f1b022a50cbc83a15942cc7a4c8673071a6ab56379861fad` | context graph 检查重复 ID、孤立 turn 与非线性时间 |
| OpenHands `2f8ea2e5e2343c6d01a0dbe8fa78651bd46a983a` | [`event/README.md`](https://github.com/All-Hands-AI/OpenHands/blob/2f8ea2e5e2343c6d01a0dbe8fa78651bd46a983a/openhands/app_server/event/README.md) | `4a0222a86861bf22fea46fd3f0d32109ce36481f42ac121a84b233439695be39` | conversation event 的持久化、查询、流式传递与分页 |
| OpenHands 同 revision | [`sandbox/README.md`](https://github.com/All-Hands-AI/OpenHands/blob/2f8ea2e5e2343c6d01a0dbe8fa78651bd46a983a/openhands/app_server/sandbox/README.md) | `fe18acef00784ac01bc796c073e2f18ba2ae646e199e6bcf8aaeab61a0416041` | sandbox create/start/stop/destroy 与多 backend 生命周期 |
| Aider `5dc9490bb35f9729ef2c95d00a19ccd30c26339c` | [`repomap.md`](https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/docs/repomap.md) | `ac41c8843d66c04bb5951155015118ef702f107417be62f280cb8955baf1eec8` | token-budgeted 代码结构摘要与按需取文件 |
| Aider 同 revision | [`git.md`](https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/docs/git.md) | `1469e351b879780852b550b3b1d702d26cba36e2a7738b3613e2577441bbcf8c` | AI edit 的 diff/review/undo 与 dirty file 处理 |
| Aider 同 revision | [`architect.md`](https://github.com/Aider-AI/aider/blob/5dc9490bb35f9729ef2c95d00a19ccd30c26339c/aider/website/_posts/2024-09-26-architect.md) | `39d04275c25031742ef2d458a8214479f7c96a3b56921b474c75e14bc637011f` | architect 提案与 editor 修改分离；benchmark claim 未被采用 |
| Goose `65e1e3d508b9f85c53998564db04708c90031881` | [`permission_store.rs`](https://github.com/block/goose/blob/65e1e3d508b9f85c53998564db04708c90031881/crates/goose/src/permission/permission_store.rs) | `bf07de5cd1245efe7777ccc40f3cd6cca603fe2d19b1c75ea86b2e35998f3f84` | 可持久化、可过期的工具 permission record |
| Goose 同 revision | [`security_inspector.rs`](https://github.com/block/goose/blob/65e1e3d508b9f85c53998564db04708c90031881/crates/goose/src/security/security_inspector.rs) | `e3277b3a7e4c0002773896904e27ca3117d5e219d0288d62bdabeb7507fa7b43` | security finding 与最终 permission decision 分离 |
| Goose 同 revision | [`context_mgmt/mod.rs`](https://github.com/block/goose/blob/65e1e3d508b9f85c53998564db04708c90031881/crates/goose/src/context_mgmt/mod.rs) | `f8ad0646de5e2f8144afe2db096b67ef131d54cdb90d26726d770db2b06197a9` | threshold compaction 与 summary continuation |
| OpenCode `cb562b2c6289c2eee707078f9ab644cbe1d3d8a9` | [`permission/index.ts`](https://github.com/anomalyco/opencode/blob/cb562b2c6289c2eee707078f9ab644cbe1d3d8a9/packages/opencode/src/permission/index.ts) | `5b9e4aa65290a39363722b9fae4c68080188d8ed76896afa0d96cd9dbfd2821d` | rule evaluation、default ask、pending request 与事件通知 |
| OpenCode 同 revision | [`snapshot/index.ts`](https://github.com/anomalyco/opencode/blob/cb562b2c6289c2eee707078f9ab644cbe1d3d8a9/packages/opencode/src/snapshot/index.ts) | `d301ef3dce67c870247bf65db962c2f0c7881f9ffd0e564167605111c9a2065f` | 独立 Git dir、track/patch/restore/revert 与并发锁 |
| OpenCode 同 revision | [`session/revert.ts`](https://github.com/anomalyco/opencode/blob/cb562b2c6289c2eee707078f9ab644cbe1d3d8a9/packages/opencode/src/session/revert.ts) | `5dd351fad8b01c5a4684092ddd427bb1c6fe8cfe6dd0000d58ffeba485d099a9` | snapshot patch 与 session/message 边界关联 |
| OpenCode 同 revision | [`session/compaction.ts`](https://github.com/anomalyco/opencode/blob/cb562b2c6289c2eee707078f9ab644cbe1d3d8a9/packages/opencode/src/session/compaction.ts) | `5ce0f453dd2b2446cce626e7a840010d848c8e99c302fbdcaf76488c3c587fa0` | compaction event、recent-token protection 与 summary 跟踪 |

本地 Grok `0.2.106` 文档补充证据：

| 文件 | SHA-256 | 观测 |
|---|---|---|
| `14-headless-mode.md` | `d5178da68f4bd9c7936de476bc6be07e01ef05cb5e82c8eb82b8bf815d9f7dcb` | documented streaming event 只有 `text`、`thought`、`end`、`error`，另提示存在非穷举 compaction event；usage 明确可能 incomplete |
| `22-permissions-and-safety.md` | `462e52c075997a92735736fe9130ba10a2600d30d8d15b779863176ef12e6c81` | hook → deny/ask/allow → remembered grant → built-in → mode；项目 permission/config 没有独立 trust prompt |
| `10-hooks.md` | `135a71daae77d09527bdc866794038d294ba900eb7b80f4bc9fa1c59e4f954f4` | 有 `PreCompact`/`PostCompact` 和 tool lifecycle hook；hook failure 默认 fail open |
| `04-slash-commands.md` | `60d82b420c8218f292763211417c8eb43f2311da6cf7c7b85e56c0fb0611d2dd` | `/flush` 生成 LLM summary 保存当前知识，不构成逐项无损 provenance |

这些本地文件来自被 Git 忽略的隔离 profile，不提交 raw 副本，防止把完整自动生成 profile 纳入仓库。

## 4. Adopt / adapt / defer / reject

| 候选设计 | 裁决 | 本项目落点 | 理由 / 限制 |
|---|---|---|---|
| workspace trust bootstrap | **adapt** | launcher 在 Grok discovery 前冻结项目 `.grok/config.toml`、`.claude/settings*`、hook、plugin、skill、MCP 和 project rules 的 digest | Grok 已用 folder-trust 保护 project hook/MCP/LSP/plugin code；sidecar 补未信任时仍可见的 instruction/skill/permission config 与统一审计 receipt |
| ordered append-only rollout + replay | **adapt** | sidecar `run-event` sequence/hash chain；引用 Grok trace/export 和 content-addressed payload | 不复制 Grok session；记录 completeness 与缺失来源 |
| typed tool begin/end/permission/patch event | **adapt** | 用 hook/provider capture/workspace delta 组合成观测桥，每条标 `source` 与 `confidence=observed` | Grok stdout 当前不能单独重建 tool loop；不得从最终文本猜事件 |
| pre-mutation snapshot + restore | **adopt** | 独立 shadow store，tool 前冻结 before digest；restore 必须 no-overwrite、显式授权并产生 receipt | 不自动 commit 用户分支，不把 user dirty work 混入 AI commit |
| policy priority/explanation | **reuse + adapt** | 复用 Grok `deny > ask > allow`；sidecar 只记录匹配来源、最终原因与 policy digest | 不另造通用 permission engine；LIF hard gate 仍不得放进 fail-open hook |
| security inspection 独立于 permission | **adopt** | scanner 产生 finding ID/解释；permission 决定另记，finding 不自动等于 deny/allow | 避免把启发式模式匹配伪装成权限证明 |
| context compaction manifest/invariant | **adapt** | 每次 compaction 记录输入 event 范围/digest、派生 summary digest、保留/丢弃类别和 missing fields | model-facing 可无缝继续；audit 层不得隐藏 compaction 或把 summary 当原始证据 |
| context graph invariant checker | **adopt** | replay 检查 event ID/sequence、turn/action 归属、tool pair、terminal 唯一性、compaction lineage | 只做机械完整性，不判科学正确性 |
| scoped expiring permission store | **defer/adapt** | 只考虑低风险重复操作；危险动作继续使用一次性 request digest permit | 长期 grant 会削弱当前更强的逐 attempt 授权 |
| repo map | **defer** | 将来只作为代码导航 cache；不迁入旧研究 INDEX/MAP/self-check | Aider repo map 与 FEP/LIF claim index 不是同一对象；当前直接跨文件夹读取即可 |
| architect/editor 双模型 | **defer** | 只在未来 evaluation 中比较，不进入主 runtime | 现有 task/action/evidence 分层已覆盖职责分离；多一次模型调用增加成本和相关幻觉 |
| event query/pagination | **adapt later** | 先固定 monotonic cursor 与 chunked replay 接口，不建设 server/UI | 本机日志变大后需要，但不是当前 trace bridge 前置 |
| 多 backend/cloud sandbox lifecycle | **defer** | 保持 Windows 本地 supervisor + Grok sandbox；云端继续由 ADR-0002 延期 | 用户明确不承担持续云运维 |
| 自动 Git commit 每次 AI edit | **reject as default** | checkpoint store 与用户 Git history 分离 | 自动改写/整理用户 dirty work 超出 sidecar 权限边界 |
| 在审计中隐藏 compaction | **reject** | 明确记录 derived summary 和 discarded span | 与“未知保持未知、尽量避免幻觉补全”的产品目标冲突 |

## 5. 新发现的真实缺口

### 5.1 Workspace trust 在权限系统之前

Grok 的权限系统本身已有清晰顺序，project hook、MCP/LSP 与 plugin code 也已有 unified folder-trust；但
实际 `grok inspect` 证明 `projectTrusted=false` 时 instruction、skill、`.grok/config.toml` 与
`.claude/settings.json` 仍进入 discovery。sidecar 因此必须在任何 Grok 进程前先静态冻结所有候选来源。
trust receipt 至少固定：

- canonical workspace path、project root 与 discovery scope；Git remote/commit/dirty 摘要继续由 run manifest 冻结；
- 将被加载的项目配置、规则、hook manifest、plugin/skill/MCP 定义的路径与 digest；
- `trusted`、`restricted` 或 `denied`；作出决定的主体与时间；
- trust 后文件变化时 receipt 失效，不能沿用旧批准。

当前 `restricted` 模式不声称可以用参数禁掉全部 project discovery：只要静态扫描发现任一候选控制文件，
就生成 receipt 并令 `launch_permitted=false`。只有零候选 workspace 可继续；`trusted` 必须显式确认当前
aggregate digest。上游 folder-trust 与 sidecar receipt 两层都不能互相替代。实测见
[`GROK_WORKSPACE_TRUST_AUDIT_2026-07-21.md`](../docs/GROK_WORKSPACE_TRUST_AUDIT_2026-07-21.md)。

### 5.2 Event completeness 不能由 stdout 假装

两轮 fake run 已证明 provider 后续请求包含 `role=tool`，但 Grok documented streaming event 没有 tool
begin/end。当前 trace/event bridge 对每种事件记录：

- `event_type`、`sequence`、`timestamp`、`run_id`、`turn_id`/`tool_call_id`；
- `observed_from`：`grok_stdout`、`grok_trace`、`grok_export`、`hook`、`provider_capture`、
  `workspace_scan` 或 `supervisor`；
- raw payload digest、redaction state、completeness state；
- 若无法建立一一对应，使用 `unknown`/`ambiguous`，不得按相邻时间自动拼接。

sidecar 只对已观测来源做 reducer；不生成“推测事件”。Codex 的 payload-before-reference 顺序可直接采用：
先安全落盘/加密 raw payload，再 append 引用它的 event。

### 5.3 Checkpoint 与 workspace delta 是同一事务的两侧

在每个可能写文件的 tool 前生成 checkpoint ID 和 before manifest；tool 后记录 after manifest/delta。若 tool
没有发生或被拒绝，仍写 terminal event，不能复用 checkpoint ID。restore 是新的危险 action，必须：

1. 指向固定 checkpoint 和期望 current-state digest；
2. 检测自 checkpoint 后的人类/外部写入并 fail closed；
3. 单独确认，将实际 restored/skipped/conflicted 文件写入 receipt；
4. 不删除 checkpoint 之后的原始审计事件。

首个 spike 只覆盖 fixture workspace 的 create/modify/delete 与冲突检测；不对真实用户工作树自动 restore。

### 5.4 Compaction 必须可追溯但不保存伪造的“完整思维”

Grok 已发出 compaction 生命周期 hook，但 summary 是派生模型输出。sidecar 应登记 compaction boundary，
不尝试记录或复原模型脑内过程。manifest 至少包含：

- 原始 event 起止 sequence 和每个 content-addressed blob digest；
- compact request/response 的 sealed-private digest；
- retained、summarized、discarded、unknown 四类范围；
- summary 标记 `derived_unverified`；replay 不用 summary 替代原始 observation。

若原始内容因策略未记录，状态必须是 `not_recorded_by_policy`，而不是从 summary 反推。

## 6. 修订后的 implementation spike 顺序

### Spike A：trusted observed launch + event completeness（首轮完成，partial）

1. 新增 workspace discovery/trust manifest 的 schema 与 fixture；默认对未登记 workspace 使用 restricted。
2. 已用 isolated fake workspace 验证：untrusted instruction/skill/permission config 可见，hook 被跳过；
   `inspect --trust` 不签发 trust。真实 session trust grant 仍保持 unknown。
3. 执行本地 `grok trace --local` 与 session `export`，先做 leak scan，再登记 content digest。
4. 合并 stdout、trace/export、provider capture、supervisor 和 workspace scan 到 append-only event bridge。
5. verifier 检查 sequence、payload-before-reference、唯一 terminal、usage/event completeness，不提升 claim。

当前进度：1–5 已完成 fake-only 首轮实测。`events.jsonl` 直接补足 tool lifecycle，`updates.jsonl` 直接提供
tool-call ID；34 条 metadata-only bridge event 的 schema 与 hash-chain 通过。纠正后的 Windows v4 post-run 在
隔离 `GROK_HOME` 与全网络阻断下成功取得 5702-byte local trace 和 297-byte export。整体仍为 `partial`，因为
cross-source runtime order、sealed encryption 与 stdout tool lifecycle 尚未观测；空 search index 是派生缓存状态，
不影响 list/export/trace 的直接 session 路径。详见
[`GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md`](../docs/GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md)。

### Spike B：pre-tool checkpoint + delta + restore receipt（hash-only audit receipt 已完成）

disposable fixture workspace 已完成双 pass、hash-only checkpoint 与 create/modify/delete delta，专用 verifier 与
合并 regression 均通过；它能证明差异但不能恢复。恢复 spike 改用独立 shadow Git fixture，audit receipt 只引用
commit/tree ID、expected-current digest 和 conflict-only plan；不得修改用户 Git history 或自动覆盖真实文件。

### Spike C：Windows containment race

显式 child-tree timeout/background cancel/parent-exit fixture、独立 verifier 与 baseline/candidate 管理员
observed 矩阵已完成。candidate `0.2.111` 全部通过；baseline `0.2.106` 的 tool-timeout 在返回终态后未退出。
suspended launch 仍未实现；若只能靠自编译 Grok 或大幅接管 runtime，则保留已知 race，不启动完整源码构建。

### Spike D：compaction provenance

manual `/compact` fake-only observed run 已完成：`PreCompact`/`PostCompact`、request `chat_history` source
span/digest、checkpoint、派生摘要与压缩后 continuation 均经独立 verifier 通过。摘要标为
`derived_unverified`；上游未逐 item 记录的 retained/discarded 映射保持 `unknown`，不从摘要反推。automatic
threshold 仍未观测。真实 DeepSeek development probe 仍需单独授权。

## 7. 明确不扩张

- 不迁入旧 `index/map/self-check`；具体 LIF claim 任务需要时再跨目录读取并记录来源。
- 不增加云端 runtime、remote queue、多人 server、移动端之外的运维面。
- 不增加通用多 Agent 编排、双模型 architect/editor 或新的模型 transport。
- 不 clone 六个参考项目、不复制其实现代码；需要实现时只采用可独立表达的架构不变量。
- 不把 trace 完整性、checkpoint 可恢复或 leak scan PASS 提升为模型正确、evidence sufficiency 或科学 claim。

更深的职责收缩、ACP-first 路线和 shadow-Git 裁决见
[`MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md)。
