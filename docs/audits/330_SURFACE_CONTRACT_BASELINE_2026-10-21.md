# 330 批：产品面契约基线——12 面 × 六家业界对照＋0dn 立项（2026-10-21）

> 性质：**调研＋立项批**（零源码、零跑批、零载体重建、零装机位改动）。
> 用户令：用户问「怎么这么多不成熟和缺失的细节……orz 明明借鉴了很多成熟项目和机制……是否考虑再查一遍业界实现和 orz 的当前情况？」；随后裁决「330 做 12 面全量契约」「0dn 三件全立」。
> 产出：本档＋[`architecture/current/surfaces/`](../../architecture/current/surfaces/) 12 页短契约＋0dn 立项（计数 **63 → 64**）。

---

## §1 结论先行

1. **不是错觉，也不是能力问题**：这是一个 66 万行、自建面只有 4–10 周、验证仍靠人工走查的系统的正常形态。要改的是形态，不是再修五个 bug。
2. **业界没有可直接抄的「会话生命周期」**：六家里五家是单进程本地写，没有跨进程租约；orz 319 自建的 OS lease 反而超前。真正可抄的是**语义边界**（撤回不回文件、取消不回副作用、第二客户端归属），不是实现。
3. **最扎心的一条**：ACP 已到 **v1**，orz 还在 **0.10.4**。v1 明文有 `session/resume`（MUST NOT replay）、`session/close`（MUST 先 cancel 再释放）、`session/list`；0dk 里最难的三件——close 状态机、resume、replay 幂等——协议已经写好了。329 的私有 `_orz/bridge_attached` 实质是重造 `resume`。
4. **交付语义是 orz 唯一超前的地方**：四家成熟产品都没有机械「完成」判定，orz 的 `[RUN_END]` 是自造，因此更需要自己的契约与真机钉。
5. **验证闭环是根问题**：12,251 个测试挡不住涌现型 bug；前端 smoke 用 DOM stub，全仓零浏览器自动化；每轮真机走查稳定产出 3–5 个新问题。

---

## §2 诊断：为什么「到处缺细节」（六条，带读数）

| # | 原因 | 读数与证据 |
|---|---|---|
| 1 | **底座被删掉的正是出事的那半** | 2026-08-02 从 Grok Build 分叉；08-03 删约 92 万行 shell/pager（commit `610dc9ab`）。仓内 66 万行 Rust：自研 7 crate **17.7 万**（loop 8.6 万／host 3.6 万／assurance 3 万／tui 1.25 万／web 0.46 万），其余约 48 万为继承组件。近五天 322/327/328/329 四批真机问题**全部**落在自研面；写控/journal/工具面几乎无新 bug |
| 2 | **测试挡不住涌现型 bug** | 12,251 个 `#[test]`：loop 911／host 410／assurance 306／tui 186／**web 63**。前端 1,605 行 smoke 用 DOM stub，文件头自述「浏览器内观感/渲染验证仍在 S3 真机首读」；orz-web 零浏览器自动化依赖 |
| 3 | **借鉴停在机制，没落成不变量** | 329 的修法（本地截断＋按锚重建 marker＋`recall_log` 过滤重放）本质是事后补上 CC 的不变量：checkpoint 绑定消息、UI 从单一日志派生 |
| 4 | **根项没有设计稿** | 0dk（会话生命周期）只有 318 的验收清单，无状态机/owner；0dd、0di 都有设计稿。`session/new` 在 319、329 被改两轮，同一行为四处打补丁 |
| 5 | **验证闭环靠人** | 至少 8 项卡 S4（0ct/0cu/0cy/0cz/0da/0dh/0di/0dm）；10-11 到 10-21 **七个 Release**，最近 48 小时四次构建/换装（0.9.4/0.9.5/0.9.6/0.9.7）；318 审查 P0 组、328 五问题、322 摩擦批、327 恢复 P0 皆真机走查直接产出 |
| 6 | **台账没有「已发行」维度** | 329 五个修复只在源码/远端，装机位仍是 0.9.7，台账却写「落码/达成」 |

---

## §3 业界契约基线（2026-10-21 实查）

版本：Claude Code 2.1.306、Codex CLI 0.163.0、Gemini CLI 0.65.0、opencode 1.19.3、aider 0.86.0、OpenHands SDK 1.56.0、ACP **v1**。

### 3.1 会话 / 连接 / 撤回 / 取消 / 审批

| 面 | 机械事实 | 对 orz 的判定 |
|---|---|---|
| 会话持久化 | 六家全是单进程本地写；**无一家有跨进程租约**（OpenHands 仅进程内 FIFOLock） | 319 的 OS lease 领先，不必自卑 |
| 第二客户端 | CC 后台会话**独占**（第二 opener 报 already open）；Codex app-server 多订阅；opencode 多端 | 409＋busy 重同步＝CC 同侧，属正当选型，须写契约 |
| 撤回 | CC 只回滚 Edit/Write，Bash/subagent 不回滚、快照 30 天过期；Gemini `/rewind` 不回滚手改与 shell；OpenHands fork 共享工作区不回滚；opencode `revert` 返回 true 却不还原（[#20955](https://github.com/anomalyco/opencode/issues/20955)） | 296 裁决「不连带工作区、git 兜底」＝业界主流；缺的是 UI 明示 |
| 取消 | 语义上限＝「不再发起新的模型调用」，无人承诺回滚副作用；aider 唯一明文保证 partial response 留存 | 不必为「取消后副作用残留」道歉 |
| 审批 | CC deny>ask>allow、超时无文档；Codex auto_review fail-closed、断线归属未查证 | 300s fail-closed 是本地正确选择；并发审批需可见 |
| 已知坑 | CC #40487 重复 uuid 丢 1232 条；Codex #49225 断网静默降权；Gemini resume 重复注入 tool response（PR #29366） | 都要进本地失败模式清单 |

### 3.2 上下文 / 交付 / 诊断 / 版本 / 输入

| 面 | 机械事实 | 对 orz 的判定 |
|---|---|---|
| 上下文 | CC `/context`＋`/compact`＋可配 `autoCompactWindow`；Codex 默认 **94.7%** 才自动压（[#40095](https://github.com/openai/codex/issues/40095)）；Gemini 阈值 0.5＋`/compress`；opencode 只有 `/compact` | **Web 端零读数**是真实差距（S9） |
| 回合结束 | 四家**都没有**机械完成判定，靠模型自述＋提示符空闲；opencode `attention.completed` 是唯一机械信号（默认关） | orz `[RUN_END]` 超前；需自建契约与真机钉 |
| 诊断 | CC `/doctor` 可自动修；Codex `codex doctor`；Gemini/opencode 无 doctor 但有 `debug` | **orz 无 doctor**（S11） |
| 版本 | CC 自动更新＋组织最低版本硬报错；Codex `codex update`＋`/status` 显示 server 版本；opencode `upgrade v0.1.48` 可回滚 | **orz 无 `--version`、无偏斜检测、无更新入口**（S12） |
| 输入 | CC 最完整（10 条草稿含粘贴/图片、`[Pasted text #N]`）；Codex/Gemini 多行键位文档缺失；三家中文字 IME 都有未关 issue | orz Web 端 IME/大段粘贴无契约（S4 附） |

### 3.3 ACP v1 与本地代差（本批最重要发现）

| 方法 | ACP v1 明文 | orz 现状 |
|---|---|---|
| `session/load` | 可选；**MUST** 全量 replay，流完才响应 | 有；只回放 user/assistant，工具/系统不回放 |
| `session/resume` | 可选；**MUST NOT** replay | **无**；329 用私有 `_orz/bridge_attached` 代替 |
| `session/close` | 可选；**MUST** 先视同 cancel 再释放 | **无**（代码注释「0.10.4 无此方法」） |
| `session/list` | 可选；无结果 **MUST** 空数组 | **无**（用 `/api/conversations` 本地替代） |
| `session/cancel` | notification；必须落 `cancelled` | 有 |
| 规范留白 | 并发 prompt、客户端断开、心跳、load 期间收新 prompt、持久化格式 | 本地全靠自己定，且未写成契约 |

---

## §4 12 面矩阵

| 面 | 业界最硬一条 | orz 现状（一句话） | 等级 | 归属 |
|---|---|---|---|---|
| S1 启动与工作区 | 环回＋令牌是本地工具标配 | 环回/令牌/Origin 齐；无端口回退、无 doctor | 可延 | 0br/0dk |
| S2 会话生命周期 | 六家都无跨进程租约 | lease 领先；**无 close 状态机、sessions 无界、删除后复活** | **阻塞** | 0dk |
| S3 连接与多端 | 第二客户端：CC 独占 / Codex 多订阅 | 409＋保活 1800s；**无心跳、无占用者提示** | 阻塞 | 0dk/329 |
| S4 发送与回合 | 取消只保证不再发起调用 | 单飞＋排队＋stop 齐；**取消无超时兜底、错误面无下一步** | 应修 | 0di/0dk |
| S5 队列 | CC 在工具边界注入、Esc 后立即发 | FIFO 16/256 KiB、边界投递、丢弃计数齐；跨进程不恢复 | 可延 | 0di |
| S6 撤回 | 无人回滚全部；CC 绑定 prompt | 截断＋锚重建＋过滤齐；**降级无提示、ordinal 可错位** | 阻塞 | 0di/329 |
| S7 长任务 | Codex #10767 跨轮收割是通病 | 跨 run 存活＋30h＋增量读出；**Web 无可见面** | 应修 | 0dh/0dk |
| S8 审批 | CC/Codex 断线归属都无文档 | yolo 默认＋300s fail-closed；并发请求挤掉无提示 | 应修 | 0dk/0dl |
| S9 上下文 | CC/Codex/Gemini 都有占用读数与手动压缩 | 事件有、**Web 零读数、无手动压缩** | 应修 | 0cz/0da |
| S10 交付 | 四家都无机械完成判定 | `[RUN_END]`＋self_check PULL＋面板；**空态无文案、前端 0 测试** | 应修 | 0dm |
| S11 诊断 | CC/Codex 有 doctor | journal/runs/replay 齐；**无 doctor** | **阻塞** | 0dk/新建 |
| S12 版本 | 四家都有版本面与偏斜处理 | `--build-info`＋清单告警；**无 `--version`、无偏斜检测** | 阻塞 | 0dl/新建 |

完整契约见 [`architecture/current/surfaces/`](../../architecture/current/surfaces/)（12 页，每页含状态机/不变量/失败模式/测试映射）。

---

## §5 0.9.8 前必闭清单（10 条）

1. **0dk S1**：会话单一状态机＋owner；入口收敛（web/TUI/ACP/CLI 六动词一张表）。
2. **ACP v1 对齐**：`resume`（不 replay）/`close`（先 cancel）/`list`（空数组）；私有 `_orz/bridge_attached` 降为兼容层。
3. **close 与删除**：close 先行 cancel；删除后绑定清除、侧车不复活；`sessions` 有界。
4. **回放口径**：`load` 是否回放工具面必须裁定并写进 UI；replay 按 `tool_call_id` 幂等去重。
5. **保活边界诚实化**：窗口外/child 退出后队列与后台不恢复，必须显式事件或文案；第二客户端显示占用者。
6. **撤回**：>200k 事件降级提示；UI 明示「只回会话、不回文件」；ordinal off-by-one 复核结论落档。
7. **交付面板**：空态文案；前端至少一条交付/自查钉；`terminal_source` 可见。
8. **诊断与版本**：`orz doctor`（配置/载体哈希/`.gsa`/端口/令牌/检索/CDP/最近 run）＋`--version`＋桥与 child 版本一致性检查。
9. **上下文读数**：Web 至少显示占用或最近一次压缩的可读读数（不要求滑块）。
10. **后台可见面**：Web 任务列表/状态，或明确「后台仅模型可见」。

---

## §6 明确不做（分歧登记，非缺陷）

| 项 | 理由 |
|---|---|
| 文件级撤回/checkpoint | 用户 296 裁决：文件面 git 兜底；业界同样不保证 |
| 多写者/多端并发写 | 单写者＋OS lease 是本地裁决；CC 亦独占 |
| 用户手动压缩滑块 | 压缩归模型/机械层（ADR §14.69）；改动属设计变更 |
| 自动更新/二进制回滚 | 发行由人工换装＋ACAF 重 provision |
| 机械完成判定 | 四家均无；机械层不判定「是否完成」 |
| 跨机共享/云同步 | Codex #52050 仍为 FR |

---

## §7 0dn 立项：产品面契约基线与真机回归（SURFACE-CONTRACT-BASELINE-AND-REALMACHINE-SUITE）

**P1；330 批；63 → 64。** 三件同批立项：

1. **契约基线**（本批已交付主体）：12 面 × 5 列矩阵＋每面一页契约（`architecture/current/surfaces/`）＋本档必闭清单。后续维护＝每面变更必须同步契约页与测试映射。
2. **真机场景套件**（S2）：10 条脚本化场景，用仓内已有 CDP 客户端驱动真 Chrome＋`--fake-provider`，不引 Playwright；每次载体重建后必跑，读数进审计。首批覆盖：刷新重连、双标签 409、撤回后重发、删除后不复活、保活过期提示、交付面板、审批并发、上下文读数、后台可见、版本偏斜。
3. **台账三级状态**（S3）：`landed`（源码）/`shipped`（装机位）/`verified`（真机）；`implemented` 只在 `verified` 后使用。

验收：S1＝12 页契约齐全且与代码证据对齐；S2＝10 条场景在 0.9.8 载体上全绿、失败可复现；S3＝INDEX/BACKLOG/TODO 状态词表更新且旧条目回填。
边界：不新增功能项；差距挂现有项或显式不做；不换底座、不重写 loop；不改 ADR-0010 设计语义。

---

## §8 读数与本批边界

- 本批：零源码、零跑批、零载体重建、零装机位改动；新增文档 13 件（本档＋12 面契约）。
- 实查日期 2026-10-21；来源：CC sessions/checkpointing/permissions/settings/errors、Codex app-server/developer-commands/approvals/config、Gemini session-management/checkpointing/hooks、opencode server/troubleshooting、aider commands/git、OpenHands persistence/fork、ACP v1 session-setup/prompt-turn/cancellation/session-list。
- 未查证项已在 12 面契约中逐条标注（如 Claude Code 审批超时、Gemini Ctrl-C 传播、opencode SSE 自动重拉）。
- 下一步：0dn S1（状态机设计稿）与 0.9.8 重建排序，待训练进程结束后按 S3→S4 顺序执行。
