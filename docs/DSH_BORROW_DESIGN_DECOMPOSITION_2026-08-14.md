# DeepSeek Harness 三项机制：采纳/撤回结论（2026-08-14）

> 状态：已复核（2026-08-14 用户裁决）：三项均不按原子系统形态实施；本文件降级为调研参考，不构成实施计划。
> 依据：`docs/DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md`（机制调研）；ADR-0010 / ADR-0011、BACKLOG 复杂度治理（2026-08-13）、CLASSICAL-EXEC-ASSISTANT 设计。
> 总边界：只借机制与设计，不引入 Node/Cordis 技术栈；跑分侧（Harbor/adapter/数据集）零改动；本结论不新增 TODO/BACKLOG/索引条目。

## 1. 裁决框架与结论

### 1.1 框架

- orz 定位：orz 自身（除借鉴成熟底座外的一切）就是薄层，整体化单一二进制；自制 loop/transport 使底座可较简单切换。
- 薄层内部不再叠加功能层；新增机制以「有失败证据 + 最简形态落入现有组件」为准。
- 个人开发者无插件适配/生态需求，DSH 的 seam/插件化/事件门形态不引入。

### 1.2 结论

| 机制 | 裁决 | 去向 |
|---|---|---|
| A Windows ACL 沙箱 | 挂起（不立项） | 调研参考；与 BACKLOG「ACAF Slice 3/4 暂缓」一致；未来若出现需要 OS 级 Windows 写限制的真实自动化场景再重启 |
| B 文件观察策略 | 收编为一条动作契约规则 | `workspace.search_replace` 契约加「先读」校验（可选版本校验），随 CLASSICAL-EXEC-ASSISTANT 小样 2 裁决；不建观察状态策略层 |
| C 工具结果裁剪 | 收编为一个纯函数 | head/marker/tail 函数在 COMPACTION-REDESIGN S2 或 50K 注入预算实施时顺带实现；不单独建模块、配置面与接线 |

### 1.3 登记纪律

- 本结论不新增 TODO/BACKLOG/索引条目；实施动作发生时由对应工作项（小样 2 / S2 / 注入预算）附带登记。
- 本文以下内容为原设计拆解，仅作参考，不构成实施计划或实施授权。

---

## 2. 原设计拆解（仅参考）

> 以下工作包、依赖与决策门保留自 2026-08-14 初版拆解；按 §1.2 裁决，A 挂起、B/C 收编，任何恢复实施都需重新裁决。

### 2.0 跨项纪律（仅参考）

1. **错峰**：另一窗口正在全量验证并修改 `orz`（controller.rs / citation_validation.rs / local_browser / acaf_e2e）与父仓库文档；本拆解批准后，实施必须等其提交/验证收敛后再落盘，避免同文件冲突。
2. **ORZ 实施纪律**：Schema/事件契约先行 → producer 接线 → verifier/fixtures 同步 → 测试 → 实施审计 → BACKLOG/TODO/索引登记；不反向削弱 ADR。
3. **默认 fail-closed**：任何机制在未配置/未接线/能力缺失时保持现状或显式拒绝，不得静默降级。
4. **可重建性**：模型可见内容必须可从 journal + 确定性函数重建（ADR-0010「model-visible ⟺ logged」的 reconstructable 形式）。
5. 每项完成标志 = 测试全绿 + 对应审计文档 + 索引状态同步；未登记不算闭合。

---

## A. Windows ACL 受限令牌沙箱（Rust 移植）——已撤回，仅参考

### A.0 目标与边界

- 目标：为 orz 提供 Windows 原生写限制沙箱后端（`workspace-write` / `read-only` 两模式），任意 Win32 失败 fail-closed（绝不无限制运行子进程）。
- 非目标：读侧限制、网络限制、控制台隔离、FAT 卷防护、容器化；不改变 TB2.1 评测。
- 落地载体：ACAF Slice 4（ADR-0011 D-11 Windows Sandbox backend）；`orz-sandbox` 是代码宿主。
- 与现状关系：不替换 Job Object（进程树终止）与 deny 路径策略，三者叠加；令牌沙箱管「写面」，Job Object 管「生命周期」，deny 管「策略拒绝面」。

### A.1 机制要点（来自 DSH，固定语义）

- `CreateRestrictedTokenW` + `WRITE_RESTRICTED`；限制 SID 列表按模式选择：workspace-write = logon SID + Everyone + workspace SID + temp SID；read-only = logon SID + Everyone（无写 SID）。
- workspace SID 由规范化工作区路径确定性派生，ACE standing（每工作区每机器一次，exact-ACE skip）；temp SID 每 live session/workspace 随机、ACE revocable。
- 写检查 = 调用者常规访问 ∩ 限制 SID 交集；grant/revoke 通过 `SetNamedSecurityInfoW`；owner-implicit WRITE_DAC 前置校验。
- runner：argv 前缀包装 + KILL_ON_JOB_CLOSE；失败 stderr `windows-acl-run: <detail>` + exit 127，拒绝不得被误判为业务 deny。
- 已实测边界（移植文档必须保留）：Everyone ambient 写、硬链接、FAT、NUL 设备、受限进程管道捕获限制、首次授予全树传播耗时。

### A.2 工作包

| WP | 内容 | 输入 | 输出 | 验收 |
|---|---|---|---|---|
| A1 | 契约与边界定稿 | DSH README/源码/设计笔记、ADR-0011、orz-sandbox 现状 | `architecture/WINDOWS_SANDBOX_CONTRACT_v0.1.md`（模式/SID 派生/错误枚举/边界表/fail-closed 语义） | 边界表与 DSH 实测一致；无未登记假设 |
| A2 | ABI 探针 | windows-rs 依赖面、DSH `verify/abi-probe.cpp` | Rust/C++ 探针（常量、结构布局、函数签名静态断言） | 探针在 Windows 实机通过；布局漂移即编译失败 |
| A3 | 令牌与 DACL 原语 | A1/A2 | `orz-sandbox` 新模块：token 创建、SID 解析/派生、grant/revoke（standing/revocable）、owner 校验、Win32Error 等价错误 | 单元测试覆盖成功/失败/幂等/重叠拒绝 |
| A4 | spawn runner | A3、`xai-tty-utils` Job Object | runner 入口：argv 包装、stdio inherit/pipe 语义、退出码镜像、stderr 契约、KILL_ON_JOB_CLOSE | runner 失败 exit 127；子进程树随 runner 死亡 |
| A5 | orz-sandbox Windows backend 接线 | A4、`SandboxProfile`/`ProfileName` | 模式映射（read-only/workspace-write）、`SANDBOX_UNAVAILABLE` 等价错误、denial 分类签名 | 请求受限模式但后端不可用 → 显式拒绝，不裸跑 |
| A6 | ACAF Slice 4 收口 | A5、ADR-0011 D-11 | Windows sandbox backend + 票据生命周期接线；影子/强制开关 | Slice 4 审计文档闭合；E2E Windows-only |
| A7 | 测试与审计 | A1-A6 | 单元 + Windows 实机 E2E + 边界回归 + 实施审计 + 索引/ADR 登记 | 门禁全绿；边界案例（Everyone/硬链接/FAT）有 pin 测试 |

### A.3 依赖与顺序

A1 → A2 → A3 → A4 → A5 → A6；A7 贯穿。A1/A2 可与 B/C 并行（纯文档/独立探针），A3 起依赖 orz-sandbox crate 稳定（与另一窗口无重叠）。

### A.4 决策门（需用户裁决）

- A-D1：默认影子模式还是强制模式（参照 ACAF Slice 2 fail-closed 的 `ORZ_ACAF_FAIL_CLOSED` 先例）。
- A-D2：workspace-write 下临时目录策略（每会话随机私有 temp + 重写 TMP/TEMP，采纳 DSH 语义）。
- A-D3：read-only 是否与现有 deny 写面叠加为「双重拒绝」还是「任一即可」。

### A.5 风险

Windows API 版本差异（探针兜底）；首次授予大工作区全树传播耗时（standing ACE 缓存缓解）；Docker/沙箱环境无法验证（需 Windows 实机，与 GAP-WINDOWS-EVIDENCE 合并证据）；实施量大，建议在 TB2.1 跑分之后开始（不影响跑分）。

---

## B. fs-observation-policy 并入结构化操作协议——已收编为契约规则，仅参考

### B.0 目标与边界

- 目标：`read_file`/`browser_read` 记录观察状态；`search_replace`/`write` 强制「先读后改」+ 版本 CAS（stale 拒绝）；错误码进入 OPS-PROTOCOL reason codes。
- 非目标：跨会话观察状态持久化（首版不做，resume 后重读）；多进程并发一致性（首版单进程内 WeakMap/侧车）；不新增平行执行层（沿用 OPS-PROTOCOL 裁剪方向）。
- 落地载体：orz-host `FileSystem` resource（观察登记 + 校验）+ `search_replace` 工具 + OPS-PROTOCOL reason codes + CLASSICAL-EXEC-ASSISTANT 小样 2（`workspace.search_replace`）。

### B.1 机制要点

- observed-state：`canonical path → {present, version}` 或 `absent`；version 载体二选一：content digest（推荐，符合 ORZ 可验证取向）或 mtime+len（廉价）。
- write-intent：未见/absent → `createIfAbsent`；present → `replaceIfVersion`（CAS）。
- edit-intent：未读 → `fs_not_observed`（文案附恢复指引「先读取再重试」）；absent → `fs_not_found`；version 不匹配 → `fs_stale_version`。

### B.2 工作包

| WP | 内容 | 输入 | 输出 | 验收 |
|---|---|---|---|---|
| B1 | 契约与 Schema | OPS-PROTOCOL v0.1、run_event schema、DSH policy 语义 | reason codes（`fs_not_observed`/`fs_stale_version`/`fs_not_found`）+ 可选字段（如需）+ Python verifier 交叉校验 | Schema 先行；verifier 负例覆盖三码 |
| B2 | 观察登记与校验 | B1、orz-host FileSystem/read_file/search_replace | observed-state registry；read 落观察；search_replace/write 校验（createIfAbsent/replaceIfVersion） | 单测：未读拒绝、stale 拒绝、absent 区分、新文件创建放行 |
| B3 | 错误面与提示词 | B2、FUS-TOOL-PROBE 中性化契约 | 中文文案 + 恢复指引；工具描述同步；与探针事件不冲突 | 文案契约测试；无旧措辞残留 |
| B4 | 结构化操作协议并入 | OPS-PROTOCOL 裁剪设计定稿（P2 前置）、B2 | `workspace.search_replace` 动作契约 + fail-closed 返回（step/code/message/upstream/trace_id） | 小样 2 收益裁决点：编辑应用成功率 + 主模型工具轮数 |
| B5 | 测试与审计 | B1-B4 | 集成（并发外部修改、重命名/硬链接边界）、fixtures、实施审计、BACKLOG/TODO/索引登记 | 门禁全绿；既有 fixtures 无漂移 |

### B.3 依赖与顺序

B1 → B2 → B3；B4 依赖 OPS-PROTOCOL 裁剪设计定稿（P2 决策门）；B5 贯穿。B1/B2 与另一窗口的 controller.rs/citation_validation.rs 无文件重叠，但仍需等其收敛。

### B.4 决策门（需用户裁决）

- B-D1：默认开启还是配置开启（`ORZ_FS_OBSERVATION_POLICY=1` 先例）。建议默认开启（纪律收益大于行为变化风险）。
- B-D2：版本载体：content digest（强一致、成本高）vs mtime+len（廉价、弱一致）。建议首版 mtime+len，digest 作可选强化。
- B-D3：观察状态是否随 journal 侧车持久化（resume 后策略延续）。首版建议不持久化（与 DSH 一致），持久化列为后续。

### B.5 风险

模型可见错误面变化 → 既有 fixtures/测试需同步；硬链接/重命名后 canonical path 漂移；双进程编辑一致性（首版声明边界）；与 CLASSICAL-EXEC-ASSISTANT 小样 2 的收益量化耦合（若小样 2 被裁撤，B4 单独收口）。

---

## C. 工具结果模型无关裁剪层（并入 orz-compaction）——已收编为纯函数，仅参考

### C.0 目标与边界

- 目标：请求组装边界对超大工具结果做 head/marker/tail 确定性裁剪，journal 与内部消息保留全文；作为 COMPACTION-REDESIGN 的模型无关先行层，与 50K 注入预算（ORZ-CACHE-CONTEXT-COST）叠加。
- 非目标：不做 LLM 摘要；不追加 surface 替换事件（首版）；不改 journal Schema（首版）；不做跑分侧任何改动。
- 落地载体：`orz-loop` gateway `transport.rs::build_request` 单点（对 `Role::Tool` 消息裁剪）。

### C.1 机制要点

- 阈值触发（草案默认 60000 code points）、头（40000）+ 固定标记 + 尾（8000）；按 code point 切分，不拆 surrogate。
- 幂等：结果严格小于输入且 ≤ 阈值；二次扫描无输出。
- 可重建：journal 全文 + 确定性函数 ⇒ 模型可见内容可重建。
- 与 COMPACTION-REDESIGN S2 关系：S2 负责历史工具记录机械坍缩（台账行），本层负责当前请求面的单条超大结果；先裁剪再决定是否摘要（借鉴 DSH pruner→compact 顺序）。

### C.2 工作包

| WP | 内容 | 输入 | 输出 | 验收 |
|---|---|---|---|---|
| C1 | 设计定稿 | 调研文档 §3、COMPACTION-REDESIGN、ORZ-CACHE-CONTEXT-COST | 放置点、env 命名/默认值、标记文案、与注入预算关系、可重建性说明 | 设计文档批准（本拆解裁决后并入 CONTEXT_COMPACTION_DESIGN） |
| C2 | 纯函数模块 | C1 | `tool_result_pruner` 模块：measure/prune/clamp + 单测（多字节、幂等、病态预算、非 Tool 消息直通） | 单测全绿；无 I/O、无环境依赖 |
| C3 | 接线 | C2 | `transport.rs::build_request` 单点裁剪；journal/messages 保留全文；env 覆盖 | 请求面断言测试；既有 journal fixtures 零漂移（裁剪只影响请求面） |
| C4 | 并入 orz-compaction | C3、COMPACTION-REDESIGN S2（P0-D 实施放行） | 压缩触发前先裁剪的组合语义；与 S2 台账坍缩不重复 | 组合测试；S2 审计交叉引用 |
| C5 | 测试与审计 | C1-C4 | 集成测试、实施审计、BACKLOG/TODO/索引登记 | 门禁全绿；登记闭合 |

### C.3 依赖与顺序

C1 → C2 → C3；C4 依赖 P0-D（COMPACTION-REDESIGN 实施放行）与 S2；C5 贯穿。C2/C3 文件（`orz-loop/gateway/*`）与另一窗口改动同 crate，实施必须错峰。

### C.4 决策门（需用户裁决）

- C-D1：默认开启还是默认关闭。建议默认开启（仅裁剪 >60K 字符的异常大结果，正常输出零影响），跑分前即生效（属于 orz 自身优化，不违反「跑分零改动」）。
- C-D2：阈值/头/尾默认值（60K/40K/8K 草案）。
- C-D3：是否在 ToolCompleted 增加可选字段记录裁剪（涉及 Schema/verifier；首版建议不加，由标记文本 + journal 全文承担可审计性）。

### C.5 风险

与另一窗口同 crate 文件冲突（错峰）；模型对截断尾部依赖的场景（标记明示原文保留于 journal，风险低）；后续 50K 注入预算实现时需定义两层先后（先单条裁剪，再整轮累计）。

---

## 恢复实施的前置条件（未来若需要）

- **A**：出现需要 OS 级 Windows 写限制的真实自动化/外部代码执行场景；BACKLOG 解除「ACAF Slice 3/4 暂缓」；具备 Windows 实机验证条件。
- **B**：小样 2 数据显示 stale 覆盖是真实失败模式（编辑应用失败率或数据丢失证据）。
- **C**：跑分或实际使用出现「单条超大工具结果撑爆上下文」的证据，且 50K 注入预算 / S2 不足以覆盖。
- 任何恢复都需重新走设计裁决；本文件不构成实施授权。
