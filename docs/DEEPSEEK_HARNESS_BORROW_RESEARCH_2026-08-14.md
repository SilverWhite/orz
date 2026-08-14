# DeepSeek Harness 借鉴调研：三项运行层机制（2026-08-14）

> 状态：调研定稿；采纳/撤回结论见 [`DSH_BORROW_DESIGN_DECOMPOSITION_2026-08-14.md`](DSH_BORROW_DESIGN_DECOMPOSITION_2026-08-14.md)（2026-08-14 复核：三项均不按原子系统形态实施）。
> 来源：`deepseek-ai/deepseek-harness`（v0.1.0-rc.5，MIT；本地浅克隆 `tmp_dsh_review/`，HEAD 47f943859b，2026-08-13）。
> 边界：按 ADR-0010 / BACKLOG 既有裁决，只借机制与设计思想，不引入 Node/Cordis 技术栈。

## 1. Windows ACL 受限令牌沙箱（dsh-sandbox-windows-acl）

### 1.1 机制

核心是把调用者令牌复制为 `WRITE_RESTRICTED` 受限令牌，限制 SID 列表携带两个写能力：

- workspace SID：由规范化工作区路径确定性派生（`S-1-4-...`），首次在工作区根 DACL 上落地一次 ACE，之后所有会话/进程/重启命中「exact-ACE skip」，避免重复全树传播；
- temp SID：每次 live session/workspace 配对随机私有临时目录 + 独立 SID，兄弟会话共享工作区但不能互写 temp；runner 退出/`dispose()` 撤销 temp ACE。

Windows 写检查 = 调用者常规访问 ∩ 限制 SID 交集。`read-only` 模式限制列表不带写 SID，此前工作区写 ACE 在 read-only 下自然失效；升级回 workspace-write 时复用 standing ACE，免再传播。

Runner 是 argv 前缀包装（同 bwrap/landlock-run 架构）：`node runner.js --workspace ... --temp ... --mode ... -- <argv>`；KILL_ON_JOB_CLOSE job 保证 runner 死亡即子进程树死亡；runner 侧任何失败 stderr 打印 `windows-acl-run: <detail>` 并退出 127，与沙箱 seam 的 RUNNER_FAILURE_RULES 匹配，拒绝永远不会被误判为业务 deny。

**fail-closed 是本实现的核心卖点**：每个 Win32 API 返回值都检查，失败携带 API 名 + Win32 code + FormatMessageW 文本；POC 在 `CreateRestrictedToken` 失败时会静默用完整令牌运行子进程，本实现直接抛错。

### 1.2 已实测边界（机制固有，非移植缺陷）

- 写受限、读/网络/进程可见性不受限（`WRITE_RESTRICTED` 只交集写访问）；
- Everyone 的环境写权限仍是 ambient 通道（keep-alive 组必须含 Everyone，否则 DLL 初始化失败 `0xC0000142` / CNG 崩溃）；Authenticated Users 从两个列表移除（关闭 C:\ 根树创建逃逸）；
- 硬链接是文件对象别名，外部别名可写；pnpm 内容寻址硬链接使「拒绝多链接文件」不可行；
- FAT 卷无 ACL → 受限令牌写检查通过，FAT 目标在两种模式下都可写；
- 控制台隔离不可用（`CREATE_NO_WINDOW`/`CREATE_NEW_CONSOLE` 子进程 DLL 初始化失败）；
- 受限进程内无法通过命名管道捕获孙进程输出（`stdio:'pipe'` 失败），继承 stdio 可；
- 授予是可传播的 standing ACE 变更，首次授予大工作区全树传播可达数十秒（每工作区每机器一次）；
- 必须目录属主可写 DACL（owner-implicit WRITE_DAC）；NUL 设备写为 ambient（设备 DACL 给 Everyone rwx）。

### 1.3 ORZ 现状与差距

- `orz-sandbox` 基于 nono（Linux Landlock/Seatbelt）+ deny 路径/网络策略 + `hook_write_deny`，Windows 侧只有 Job Object 进程树终止（`xai-tty-utils`），无令牌级写限制；
- ACAF Slice 4（D-11 Windows Sandbox backend）正是这个机制的落地载体，当前为待实施项；
- 现有 P2-SANDBOX 审计覆盖 Docker/AppContainer/Job Object，没有 WRITE_RESTRICTED 令牌方案。

### 1.4 Rust 移植可行性结论

可行，工作量为中。需要 windows-rs 面：

- `CreateRestrictedTokenW` / `OpenProcessToken` / `GetTokenInformation` / `SetTokenInformation`（default DACL 写 SID 全访问 ACE）；
- `ConvertStringSidToSidW` + `AllocateAndInitializeSid`/`CreateWellKnownSid`（logon SID、Everyone）；
- `GetNamedSecurityInfoW` / `SetNamedSecurityInfoW`（DACL 增删 ACE）+ `BuildExplicitAccessWithNameW`/`SetEntriesInAclW`；
- `CreateProcessW` + Job Object（已有 `xai-tty-utils` 基础）。

建议顺序：POC（ABI 探针 → token → DACL → spawn）→ runner 形态（argv 包装 + KILL_ON_JOB_CLOSE）→ orz-sandbox Windows backend 接线 → ACAF Slice 4 收口。README 中「限制 SID 列表、keep-alive 组、mode 选择」是移植时唯一需要注意的常量语义；边界表直接作为设计文档输入，避免重复踩坑。

## 2. 文件观察策略（dsh-fs-observation-policy）

### 2.1 机制

以事件门实现、不注册服务、不改工具接口：

- `fs/observed`：成功读/写记录 `{present, version}`，metadata miss 记录 `absent`；按 owner（agent session）弱引用分组，owner 释放即清理；
- `fs/write-intent`：未见/确认不存在 → `createIfAbsent`；确认存在 → `replaceIfVersion`（观察版本作为 CAS 基础）；
- `fs/edit-intent`：未读先改 → `FS_NOT_OBSERVED`（精确文案 `edit requires reading "<path>" first`）；已确认删除 → `FS_NOT_FOUND`；存在 → 以观察版本做 CAS。

版本新鲜度由 provider 原子 CAS 保证，策略只做「先观察后动手」的纪律；窗口读也记录整文件版本，因此外部改动会得到 `FS_STALE_VERSION` 并在错误文案里附恢复指引（重读后重试）。

设计要点：单 slot 首赢决策、策略通过事件叠加/移除（不加载则工具回到无条件读写）、观察状态不跨会话持久化（resume 后必须重读）。

### 2.2 ORZ 现状与差距

- `search_replace` 已有 `skip_read_before_edit` 配置项，但语义是「配置期要求 read 工具可用」，不是逐文件观察状态；
- 无 stale 检测：外部并发修改后，模型基于旧内容提交的替换要么匹配失败（普通错误），要么在巧合匹配时静默覆盖；
- 结构化操作协议 / CLASSICAL-EXEC-ASSISTANT 的 `workspace.search_replace` 小样 2 是天然合并点。

### 2.3 适配建议

1. orz-host `FileSystem` resource 增加 per-session observed-state registry：`canonical path → (mtime_ns, len, content_digest)`；`read_file`/`browser_read` 落盘记录，`search_replace`/`write` 校验；
2. 用内容 digest（或 mtime+len）充当 DSH 的 version CAS；Rust 侧无 NTFS version 语义，digest 更符合 ORZ 的可验证性取向；
3. 错误码并入结构化操作协议 reason codes（如 `fs_not_observed` / `fs_stale_version`），文案中文化并附恢复指引；
4. 恢复语义：journal 可回放观察记录（DSH 明确不持久化；ORZ 的 journal 侧车可承载），resume 后策略仍生效；
5. 默认开启还是配置开启需用户裁决——它改变模型可见错误面，必须先 Schema/文案/测试同步。

风险：双进程/并发编辑时观察状态的一致性；硬链接/重命名后的 canonical path 变化。收益：杜绝「基于旧视图覆盖新内容」这一类 benchmark 与日常任务的实际错误。

## 3. 工具结果模型无关裁剪（dsh-compaction-tool-result-pruner）

### 3.1 机制

- 阈值触发（默认 8192 code points）、头（4096）+ 固定标记 + 尾（1024），非文本块保序保留；按 Unicode code point 切分，不拆 surrogate pair；
- 原文完整保留在 append-only 会话日志；替换事件带 `surfaceOp: {op:'replace'}` + `sourceEventSeqs`，replay 可还原；
- 纯模型无关：不调用 LLM、不做语义判断；二次扫描幂等（结果严格小于原内容）；
- 与压缩引擎组合：先裁剪再决定是否摘要，可避免一次 LLM 摘要调用。

### 3.2 ORZ 现状与差距

- `orz-tools/src/util/truncate.rs` 已有 `truncate_with_preview`（头 + footer，无尾保留）与 `soft_wrap_line`；web_fetch 200K 截断、bash 30KB 上限；
- 但截断发生在工具侧（源），缺少请求组装边界的统一兜底：journal 保留全文、模型侧只看到裁剪面；
- 单轮工具结果注入预算（ORZ-CACHE-CONTEXT-COST，默认 50K token）尚为 pending，本机制是其「模型无关先行层」。

### 3.3 ORZ 采纳结论（2026-08-14 复核）

- **挂起**：A（Windows ACL 沙箱）不立项；本文机制与边界表仅作未来参考（与 BACKLOG「ACAF Slice 3/4 暂缓」一致）。
- **收编为契约规则**：B（先读后改 + 可选版本校验）作为 `workspace.search_replace` 动作契约规则，随 CLASSICAL-EXEC-ASSISTANT 小样 2 裁决；不建观察状态策略层。
- **收编为纯函数**：C（head/marker/tail 裁剪）在 COMPACTION-REDESIGN S2 或 50K 注入预算实施时顺带实现；不单独建模块、配置面与接线。
- 依据：orz 整体就是薄层（除成熟底座外的一切，单一二进制），底座可较简单切换；个人开发者无插件生态，不支付子系统化复杂度；新增机制以「有失败证据 + 最简形态落入现有组件」为准。
- 若未来恢复 C 实施，参考放置点为 `transport.rs::build_request` 请求组装单点（可重建性语义见 §3.1）。

## 附：不建议照搬的部分

- Cordis「一切皆插件」栈：与 ADR-0010 融合架构取向相反，只借鉴 seam 三段式（Service Definition / Provider / Consumer）组织组件采纳矩阵（FUS-COMPONENT-REGISTER）；
- Typert 类型协议生成 / Host-Client 分裂构建：TS 特定，Rust 侧无对应收益；
- Web/React 客户端：ORZ 为 TUI + Codex surface，不扩展。

## 附：全层扫描与候选记录（2026-08-14）

### 扫描结论

DSH 其余层与 orz 现状对照（只记结论，机制细节见 DSH 源码/README）：

- 已覆盖，不借鉴：hooks（orz 已有 `orz-hooks`）、transport 重试（ADR-0007 + stream retry）、token 计量、持久化/replay（journal + GAP-CONVERSATION-RESTORE sidecar）、停滞守卫（输出重复/ngram）、plan/goal、skills、terminal/lsp/web、ACP、凭据（ADR-0006）、子代理权限继承（角色契约已做）。
- 冲突或不适配，不借鉴：jobs/schedule/workflow（2026-08-09 已封禁异步调度生态）、e2b 远程沙箱、typert/api gateway/客户端 UI/bundles/presets/self-modification（插件生态形态）、session telemetry/OTEL、identity、feedback、attachment、storage domains、Python SDK（ORZ 已有 assurance reference）。

### 候选 1：崩溃恢复工具结果词汇（记录，有价值）

来源：dsh-session-persistence 崩溃恢复契约（`TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN`）。
登记：ADR-0010 §14.13 条件触发项；BACKLOG「条件触发（不占当前优先级）」。

- 机制：恢复中断轮次时补合成关闭事件——工具从未开始（`TOOL_NOT_STARTED`）与已开始但结果未知（`TOOL_OUTCOME_UNKNOWN`），并明确告诉模型「只重试只读/幂等操作，验证可能副作用或询问」。
- ORZ 现状：conversation sidecar（GAP-CONVERSATION-RESTORE）按完整消息列表原样恢复；尚未确认 sidecar 是否只 checkpoint 完整轮次。
- 价值：若恢复面可能出现「assistant 已声明 tool_calls 但无对应 Tool 消息」，协议会 400（polyglot P1 同款失败模式）；该词汇为模型提供副作用未知的诚实边界，避免恢复后盲目重试。
- 落地形态（仅当出现恢复面 400 或副作用未知的证据时）：恢复时补合成 Tool 消息 + 中性指引；单点修复，不建子系统。
- 登记纪律：不新增 TODO/BACKLOG 条目；出现证据时由恢复/会话工作项附带登记。

### 候选 2：停滞守卫补「同工具同参数」信号（评估：边际，暂不采纳）

来源：dsh-repeat-tool-reminder。
登记：BACKLOG「条件触发（不占当前优先级）」。

- 评估：实际作用有限。① 核心失败模式已被覆盖——输出重复由 stagnation guard 抓，连续拒绝由 denial breaker（3 轮同 key 策略切换）抓；② 同参重复若输出也重复则已被覆盖，若输出持续不同则多为合法轮询（DSH 自认会误报）；③ 已观察到的真实停滞案例（path-tracing 131+ 次 read、make-doom 侦察循环）参数不断变化，精确参数匹配抓不到；④ 提醒类机制不阻断，TB2 已有「机制触发但行为未矫正」前例，矫正力存疑。
- 结论：暂不采纳，保留为候选信号。若未来出现「同参循环且输出持续变化」的具体证据，在 stagnation guard 内加最小计数信号（同一工具连续 N 次调用），不复刻 reminder 链。

### 候选 3：中立问询升级为「强制模板轮」（2026-08-14 用户确认方向）

背景：TB2 缺口 2——中立问询触发多次但行为未矫正（path-tracing / make-doom）。现行为：注入三问文本、非硬门、循环继续（ADR-0010 §4.2；DC 同理 2→3→4→5 递进注入）。
设计文档：[`ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md`](ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md)；权威登记：ADR-0010 §14.13；BACKLOG 6c / TODO P1。

- 决策：触发点改为「下一安全动作间隙明确暂停，模型必须填写问询模板后才恢复动作」；Orientation 与 DC 两族共用同一机制，模板按触发类型微调。
- 目的定位（用户确认）：中立问询本身是拉回注意力、防跑偏与钻牛角尖；计划与目的在黑板上，模型回头看并填表即达成主要收益——「强制表达，不验证诚实」；填表式应付不是主要风险，但缓解手段必做。
- 机制要点：
  - 暂停 = 独立 checkpoint 轮，不派发任何工具；模型本轮只输出模板答案。
  - 模板字段：`task_position` / `progress_evidence` / `blockers` / `next_action`（枚举 continue|adjust|gather_evidence|ask_user|handoff）/ `changed_direction`；长度上限；复用 allowed/forbidden 词汇。
  - 机械校验：必填、枚举、长度；失败给一次错误反馈重填；仍失败 → 按已填部分机械降级 + journal 记录，不挂死。
  - 缓解必做：`progress_evidence` 与 journal 证据身份做存在性交叉校验；`next_action=gather_evidence` 必须给出缺失面。
  - 范围：主车道；暂停点在模型决策边界，与机械助理执行层不冲突。
- 实施前置：ADR-0010 §4.2 修订（「注入」→「暂停并填写模板」）；事件/Schema/verifier/fixtures 同步；测试。
- 登记纪律：不新增 TODO/BACKLOG；实施放行时由对应工作项登记。

### 候选 4：会话累计上下文监测（2026-08-14 用户确认方向）

- 定义：度量 = 当前会话累计模型可见输入 token（usage 实报优先，缺失时 journal 估算）；阈值 384k 机械提醒、500k 机械总结推荐（阈值可配）。
设计文档：[`SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md`](SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md)；权威登记：ADR-0010 §14.13；BACKLOG 6d / TODO P1。
- 用户确认：TUI 不急（尚未进入用户侧 beta）；最简实现 = 到达阈值的「最后一轮模型输出末尾」机械附上一句提醒；无用户侧（headless/自动化）仅写日志。
- 500k 推荐内容：复用压缩五段模板（目的/计划/变动文件/注意事项/后续衔接）+ 新窗口开场提示骨架；不自动开新窗口，只给产物。
- 与压缩关系：压缩继续自动运行（窗口钉在 384k 内），监测是会话生命周期信号，二者独立、不互斥。
- 后续可选（进入 beta 前）：journal 事件（Schema 先行）、TUI 横幅。
- 登记纪律：不新增 TODO/BACKLOG；实施放行时由对应工作项登记。
