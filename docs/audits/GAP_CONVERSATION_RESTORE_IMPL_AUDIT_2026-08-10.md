# GAP-CONVERSATION-RESTORE 实施审计（2026-08-10）

> **2026-08-14 状态注记**：D2-2（恢复超窗卡死）与 D3-1（marker/白名单被恢复过滤）已由压缩机制
> 重设计裁决为实施前置（ADR-0010 v1.10 / `docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md`），
> 实施切片 S1 登记于 TODO/BACKLOG；本边界条目在对应切片闭合前保持开放。

> **三面审查闭环（2026-08-11 更新）**：设计合理性/实现合理性/符合性三独立代理审查——**无 D1/P0/P1/C1/C2**；修复批 8 项（见 §3 更新 + §7），登记项见 §4/§8。

- 范围：conversation 跨 prompt 恢复——轨道 A 剩余切片之一（GAP-RETRIEVAL-TOOLS 审计 §4 登记缺口"conversation 跨 prompt 恢复（侧车只存状态机字段；跨 run continue 子代理上下文从头开始）"）
- 用户裁决（2026-08-10）：① 范围=主 Agent + 子代理统一机制（会话级 conversation 侧车；子代理 activation 恢复时同时恢复其 conversation）② **reasoning_content 明文持久化**（DeepSeek 多轮回放硬约束，缺失 400；隐私边界=侧车非 journal 证据面，§5.4.6 只约束 journal）③ 跨进程=同 session_id 自动延续 + 落盘；显式 resume（UI/restore event/permission）登记边界留后续切片
- 事件纪律：**零事件/schema 变更**——恢复是输入侧透明变化；journal 与 conversation 无内容重叠（orz producer 从不写 model_request 事件；model_output 只含本 run 输出）；`retrieval_activation_restored` 不加 conversation digest（digest 无对账对象——conversation 内容本就不入 journal）
- 验证：workspace 全量 **0 failed**（orz-loop 167 / orz-host 153 / capture 13 字节稳定不重捕）；clippy 变更文件零新增（4 条 pre-existing 不动）；pytest 103 passed；check_repository **valid**；git diff --check 干净

## 1. 已变更（核心）

### 1.1 Message serde（orz-loop，先行，零行为变化）

- `orz-loop/src/gateway/model.rs`：`Role`（+`Serialize/Deserialize` + `#[serde(rename_all="snake_case")]`→"system"/"user"/"assistant"/"tool"，与 provider 协议串一致）、`Message`（+`PartialEq` 供测试断言）、`ToolCall`——均补 serde。无新依赖（orz-loop 已有 serde；orz-host 已依赖 orz-loop 并已导入 `{Message, Role}`）

### 1.2 run_turn 链 threading（orz-loop controller.rs）

- `run_turn` / `run_turn_with_cancel` / `run_turn_with_guards` / `run_turn_inner` 四函数各加尾参 `conversation: Option<&mut Vec<Message>>`（紧随 `orientation`；`#[allow(clippy::too_many_arguments)]` 已有）——orientation 参数先例的逐层 threading
- **种子点**（run_turn_inner normal 分支）：`conversation.as_deref_mut().map(|c| c.clone()).unwrap_or_default()` + push 当前 prompt——**clone 非 take**（grill :2240 先例；错误路径保留调用方历史，无需全错误路径回写）；`None` 时产物与历史单条消息种子逐字节相同
- **回写点**（成功-only，grill 同语义）：`*conv = messages.filter(|m| !is_injected_block_text(&m.content))`——**机械注入块（counterexample gate/budget/breaker/编辑推送/orientation）是运行脚手架非对话，过滤后持久化**（否则恢复的 prompt 会重放"一次性"gate）
- `run_grill_turn` 调用追加 `None`（grill 走自身 `GrillTurn.history` 路径，不动）；全部既有调用点（controller 测试 ~58 + orz-bin 产品/测试 ~14）追加 `None`（编译期保证）

### 1.3 host 侧 conversation 侧车（orz-host acp_server.rs）

- 新类型 `StoredConversation { schema_version, session_id, messages: Vec<Message> }`（envelope，与 orientation/activation 侧车一致）；注释写明隐私边界（reasoning 明文；非 journal 证据面；7 天 retention）
- 三函数（镜像 orientation 侧车模式）：`conversation_sidecar_path`（`{cwd}/.gsa/conversations/{session8}.json`）/ `load_conversation_sidecar`（NotFound→None、corrupt→warn+None）/ `persist_conversation_sidecar`（**空 conversation 短路不落盘**——新会话首 prompt 前无文件；best-effort，失败 warn 不 fail run）
- `StoredSession.conversation: Option<Vec<Message>>` 字段（take-out/回写纪律同 orientation/activation）
- `handle_session_new_with_options`：`load_conversation_sidecar` 恢复（**跨进程**——进程重启重建同 session_id 自动续接）
- `handle_session_prompt`：take-out 块（build_host 成功后，与 orientation/activation 同纪律——失败不 take 防空覆盖）；run 调用 `Some(&mut conversation)`；**成功-only 落盘**（`run_result.is_ok()` 才 persist——失败 run 的中间消息不进 conversation，journal 是失败证据）+ 写回 session

### 1.4 子代理 conversation 恢复（orz-loop controller.rs）

- `StoredActivation` 加 `#[serde(default)] pub conversation: Vec<Message>`（**D-6 更新**：原"conversation 不入侧车"裁决撤销 conversation 部分；`submitted` 账本仍不入——跨 run 幂等重放限进程内，call_id 派生 disposition_id 天然防冲突；`serde(default)` 兼容旧侧车无此键）
- `ActivationRegistry::snapshot_json` 构造带 `a.conversation.clone()`；`seed_from_json` 恢复填入 `ActivationState.conversation`（替换原置空注释）
- `run_retrieval_subagent` 与 host 侧**零改动**（continue 已把新 goal push 进 `a.conversation` 并直接作为 `run_agent_loop` 的 messages in/out）——跨 run continue 子代理上下文**不再从头开始**（登记缺口闭合，ADR-0010 §3.1 三 Agent 同构）

### 1.5 retention 纳入 conversations 目录（orz-host retention.rs）

- `PruneReport` 加 `removed_conversation_sidecars`；`prune_old_records` 加 `prune_old_files(..., &gsa_root.join("conversations"), cutoff)`（镜像 one_shot_permit；conversation 是最大侧车——数百 KB；活跃会话侧车每成功 prompt 重写，mtime 恒新天然免扫）

## 2. 已删除

- （无删除——本切片为增量）

## 3. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | messages 携带=逐层 `Option<&mut Vec<Message>>` 尾参（非 TurnContext 结构体/新方法） | orientation 参数先例；单一规范入口；churn 是编译期保证的机械 `, None`；结构体方案需重写全部既有调用点、新方法方案变体爆炸 |
| D-2 | 种子语义=clone（非 take） | grill :2240 先例；错误路径保留调用方历史，无需全错误路径回写 |
| D-3 | 回写时机=成功-only | 与 grill :2364 一致；失败 run 的中间消息不入 conversation；journal 是失败证据 |
| D-4 | 空 conversation 不落盘 | 新会话首 prompt 前无文件；load NotFound→None 自然兜底 |
| D-5 | 侧车结构=envelope（StoredConversation） | 与 orientation/activation 一致，版本化锚点 |
| D-6 | **注入块过滤**（回写时剔除机械注入块） | 机械注入块（gate/budget/breaker/编辑推送/orientation）是运行脚手架非对话；恢复的 prompt 不得重放"一次性"gate；纯对话侧车。**审查修复（P2-1/D2-1/D2-4，2026-08-11）**：① 过滤只删 **User 角色**注入块（所有注入点恒 User）——Tool 消息内容前缀匹配也保留（误删会悬空 assistant 的 tool_calls 声明，重放必 400 永久卡死）；② 首条（种子起点）永不删；③ **子代理 conversation 同过滤**（snapshot_json 序列化前）；④ 回写**门控 `StagnationDecision::Continue`**——run_invalidated run 的消息不进 conversation（触发内容进侧车 → 恢复后再触发 → 重启/重试逃逸通道失效，永久 run_invalidated 循环） |
| D-7 | 事件/schema=**零变更** | model_request 事件 producer 从不写、payload schema 无消息内容；restored 事件加 digest 无对账对象；避免 schema→fixture→verifier→重捕全链路 |
| D-8 | 子代理载体=StoredActivation 加 `#[serde(default)]` 字段 | host 侧 activations 是 Value 数组自动透传（host 零改动）；serde default 兼容旧侧车；submitted 仍不入 |
| D-9 | 失败 run 的 prompt 不保留 | clone 语义自然结果；失败=未回答，下一 prompt 取代之（与 grill D2-5 保留用户回答不同——主 run 是任务语义） |
| D-10 | 恢复后 stagnation 跨 prompt 累计 | evaluate_stagnation 本就按整条 messages 计算；与长单 run 语义一致；注入块已被 is_injected_block_text 过滤 |
| D-11 | -p/stdio 一次性不接（None） | 无会话语义；orientation=None 同先例 |
| D-12 | retention 纳入 conversations 目录 | conversation 是最大侧车（数百 KB）；一行 prune_old_files；活跃会话按 mtime 免扫 |

## 4. 边界（明确未做，登记给后续切片）

- **显式 resume 留后续**：本切片只做同 session_id 自动延续 + 侧车落盘备将来；会话列表 UI / restore 事件类型 / permission 驱动的显式恢复（§5.4.5 独立 restore event/receipt）未接——机制已就位（侧车跨进程可恢复），后续只需接线
- **恢复 conversation 超 provider 窗口（D2-2 登记，2026-08-11）**：A6 compaction 的 safety 触发依赖上一轮实测 tokens（首轮 None 不触发）——恢复的长 conversation 首请求若超窗直接 400，成功-only 回写保留超窗历史 → 每次 prompt 同路径失败，会话卡死（retention/删侧车兜底）。修复（首请求前 token 估算截断或侧车大小上限）留后续切片
- **子代理对话失败持久化不对称（D2-3 登记，2026-08-11）**：主 conversation 侧车成功-only；子代理 conversation 经 activation 侧车**无条件**落盘（既有 persist 模式）——失败 run 的子代理中间消息会跨进程持久化，恢复后子代理从残缺对话继续。journal 是失败证据面；对称化（按 run 成功与否过滤）留后续
- **跨会话不共享**：侧车按 session8 文件隔离，不同 session_id 互不可见
- **grill 不接**：`GrillTurn.history` 既有路径不变（run_turn_inner 的 grill 分支零改动）
- **-p headless / orz-bin 一次性 run 不接**：`run_turn` 调用方全部传 None
- **submitted 账本仍不入侧车**：D-6 语境澄清——原"conversation/submitted 不入侧车"裁决只撤销 conversation 部分；跨 run 幂等重放仍限进程内（call_id 派生 disposition_id 天然防冲突）
- **reasoning_content 明文入侧车（隐私边界）**：侧车非 journal 证据面——ADR-0010 §5.4.6「reasoning 不写入 journal」只约束 journal；`.gsa/conversations/` 为本地明文会话产物，已纳入 7 天 retention
- **恢复的 journal 不可观测性**：零事件变更——conversation 恢复只影响模型输入与后续输出内容（非事件序列）；`retrieval_activation_restored` 不带 conversation digest
- **侧车大小**：长会话数百 KB，每成功 prompt 全量重写（best-effort 不阻塞 run；retention 兜底；增量/截断策略留后续）
- **同 session 并发 prompt 竞态（P3-2 登记）**：`runs` 注册表对 prompt-vs-prompt 是既有覆盖语义（不互斥）——并发 prompt 的 conversation take-out 竞争，last-writer-wins 丢一方对话（journal 留证）。TUI 有 running 门，直接 API 客户端可触达；与 orientation/activation 既有模型一致，非新缺陷
- **压缩 marker/白名单被过滤（D3-1 登记）**：压缩 marker 与白名单块注册于 is_injected_block_text，回写一并剔除——恢复后模型看不到压缩告知且丢白名单任务事实（journal ContextCompressed 事件是证据面）；恢复面保留 marker 留后续

## 5. 验证

- `cargo test --workspace -j4`：全量 **0 failed**（167 测试目标全 ok；orz-loop 167 passed / 3 ignored、orz-host **154** / 2（+1 重启恢复测试）、orz-bin 6+2+1、capture 13）；新增 17 测试（controller 7 + acp_server 8 + retention 1 + 既有 snapshot 测试断言更新 1）
- 新增测试清单：`run_turn_conversation_seeds_and_writes_back` / `run_turn_conversation_error_preserves_history` / `run_turn_none_conversation_unchanged` / `seed_from_json_restores_conversation` / `stored_activation_roundtrip_with_conversation` / `stored_activation_old_sidecar_no_conversation_field` / `restored_activation_continue_seeds_history`（controller）；`conversation_sidecar_roundtrip` / `conversation_sidecar_corrupt_warns_and_none` / `conversation_sidecar_missing_returns_none` / `empty_conversation_not_persisted` / `cross_prompt_conversation_continues` / `failed_prompt_keeps_pre_run_conversation` / `restored_activation_conversation_across_runs`（acp_server）；`sweep_removes_old_conversation_sidecars_keeps_fresh`（retention）
- clippy：变更文件零新增（4 条 pre-existing：orz-config 1 / xai-fast-worktree 2 / agents/main.rs too_many_arguments 1，与 WEB_SEARCH 审计 §5 登记一致）
- `cargo test -p orz-bin -- --ignored conformance_capture`：**13 passed**——既有 13 个 capture journals 字节稳定（不重捕；fixtures 不需动——恢复是输入侧变化，事件序列不变）
- pytest：`test_run_event_journal_validation` + `test_run_event_conformance` **103 passed**
- `python scripts/check_repository.py`：**valid**
- `git diff --check`：干净
- 测试坑记录：① 单轮 run 实际 2 次模型调用（round + final answer）——脚本项数按此配 ② `run_events` helper 只支持单 run 目录（多 prompt 测试自行枚举 runs/） ③ `Message` 需 `PartialEq` 才能 `assert_eq!` 整个 conversation ④ registry 访问路径=`controller.activations.lock()`（`snapshot_json`/`seed_from_json` 在 `impl ActivationRegistry`，非 controller 直接方法）

## 6. 与相邻文档的衔接

- `GAP_RETRIEVAL_TOOLS_IMPL_AUDIT_2026-08-10.md` §4 边界行（"conversation 跨 prompt 恢复（侧车只存状态机字段；跨 run continue 子代理上下文从头开始）"）——**本切片闭合**；D-6 行（"conversation/submitted 不入侧车"）——conversation 部分撤销，submitted 仍不入
- `CLI_PROJECT_INDEX.md`：新增 `GAP-CONVERSATION-RESTORE`（implemented）canonical 条目
- ADR-0010：**不补写**（侧车是实现形态，§5.4 一般条款已覆盖；§3.1 三 Agent 同构已含恢复机制条款）

## 7. 三面审查修复批（2026-08-11，审查后）

| ID | 严重级 | 修复 | 位置 |
|---|---|---|---|
| P2-1 | P2 | 过滤收窄为 **User 角色**（注入点恒 User，已验证）——Tool 消息内容前缀匹配也保留（误删悬空 tool_calls → 重放 400 永久卡死）；首条（种子起点）永不删 | controller.rs 回写点 |
| D2-4 | D2 | 回写**门控 `StagnationDecision::Continue`**——run_invalidated run 的消息不进 conversation（触发内容进侧车 → 恢复后再触发 → 重启/重试逃逸失效，永久 run_invalidated 循环） | controller.rs 回写点 |
| D2-1/P3-4 | D2 | 子代理 conversation **同过滤**（snapshot_json 序列化前）——共享 loop 也向子代理注入 budget/orientation 块，恢复后不得重放 | controller.rs snapshot_json |
| P3-1 | P3 | `session.conversation` 写回收进 `is_ok()` 分支——flush 失败（run_turn_with_guards 的 journal.flush_async）时侧车/session 不分叉 | acp_server.rs |
| P3-3 | P3 | `load_conversation_sidecar` 校验 envelope `session_id` 与请求一致（8 字符前缀碰撞/手移文件不恢复错会话） | acp_server.rs |
| P3-5 | P3 | `debug_assert!(grill.is_none() \|\| conversation.is_none())`——类型层互斥保护 | controller.rs |
| P3-6 | P3 | 测试 `let mut c2` unused_mut | controller.rs |
| P3-7 | P3 | main.rs 5 处插入 `None,` 缩进修正（后向引用正则，不误伤合法参数行） | orz-bin/main.rs |
| P3-8 | P3 | 补端到端**进程重启恢复**测试 `new_server_resumes_conversation_from_sidecar`（新 server 重建同 session_id → 首请求含前进程历史） | acp_server.rs |

## 8. 审查记录项（登记不修）

- **D2-2 超窗卡死**（见 §4）：恢复 conversation 超 provider 窗口 → 首请求 400 → 成功-only 回写保留超窗历史 → 会话卡死；截断策略留后续
- **D2-3 子代理失败持久化不对称**（见 §4）：activation 侧车无条件落盘（既有模式）vs 主对话成功-only
- **C3-1**：审计正文 "grill :2240/:2364" 为 base 版本行号（当前工作树 :2276/~2414）
- **C3-2**：orz-bin 调用点实际 15 处（审计 §1.2 "~14" 近似）
- **C3-3**：D-8 措辞——activation 侧车是类型化 `StoredActivationSnapshot` serde 往返（非"Value 数组透传"）；"host 零改动"结论正确
- **C3-4**：相邻文档数字——RETRIEVAL-TOOLS §5 记 `test_run_event_journal_validation.py 81 passed`，本切片零事件变更实测 **89**（81 为前序陈旧数字，不改前序文档历史记录）
- **D3-2**：失败 run 的 prompt 不保留 → 重试话语无指涉（与 grill D2-5 分裂）；失败后重试须完整重述请求
- **D3-3/P3-2**：同 session 并发 prompt 竞态（既有覆盖语义；TUI running 门）
- **D3-4**：侧车明文含工具结果（工作区文件内容）——隐私边界登记补注
- **D3-5**：session8 前缀碰撞 + envelope session_id 校验已修（P3-3）；collision 恢复错会话风险仍低概率
- **D3-6**：过滤后可能连续同角色消息（DeepSeek 严格性未验证）——补真实多轮冒烟或后续验证
- **P3-9**：每成功 prompt 全量重写成本（O(n) 序列化 + 写盘；best-effort；长期会话 O(n²) 累积，compaction 可截断）
- **P3-10**：全部调用点 15+1 处追加 None 位于签名末位 conversation 槽，逐一核对正确

- `GAP_RETRIEVAL_TOOLS_IMPL_AUDIT_2026-08-10.md` §4 边界行（"conversation 跨 prompt 恢复（侧车只存状态机字段；跨 run continue 子代理上下文从头开始）"）——**本切片闭合**；D-6 行（"conversation/submitted 不入侧车"）——conversation 部分撤销，submitted 仍不入
- `CLI_PROJECT_INDEX.md`：新增 `GAP-CONVERSATION-RESTORE`（implemented）canonical 条目
- ADR-0010：**不补写**（侧车是实现形态，§5.4 一般条款已覆盖；§3.1 三 Agent 同构已含恢复机制条款）
