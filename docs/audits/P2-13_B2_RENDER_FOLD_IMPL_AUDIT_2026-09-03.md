# P2-13 B2 渲染折叠实施审计（2026-09-03）

> 主题：BLACKBOARD-CONVERSATION-SCOPE-FOLD（P2-13）B2 渲染折叠
> S1 实施 + S2 测试完成；B3（契约与收尾）/ B4（S3 重建 + S4 复验）待续。
> 设计权威：`BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md`
> v0.8 §9/§11/§12 + ADR-0010 §14.52；B1 先例与既有渲染口径见
> `P2-13_B1_CONVERSATION_BASE_IMPL_AUDIT_2026-09-03.md`。实施基于
> 提交 b6458d4（B1）+ 85a699a（顶层登记）之上的工作树，未提交（仓库
> 纪律：由用户统一提交）。

## 1. 实施范围（B2）

- blackboard_read 折叠态渲染：可折叠分区（exec / edits / tool_actions）的
  live 读取在触发折叠态时默认只展开「当前域段 ∪ 最近 K 轮 ∪ 最近 20% 行」，
  更早内容按域段聚合为标注行；存储零改写、无新增事件、零持久化（R3：
  不新增 `blackboard_fold`）。
- 展开参数：`domain` + `round_from`/`round_to`（含边界，相等 = 单轮；
  R2 定名），显式展开 = 折叠态 + 目标段行；与 `receipt_id`/`since`/`epoch`
  互斥 fail loud。
- pre-stamp 段：旧无章行（round=0/domain=None）独立段，标注 = 计数 +
  时间范围，只能经 `since`/`receipt_id` 展开（R1）。
- 分区渲染 cap 补齐：edits / tool_actions 此前无硬上限，按 exec 同口径
  加行/字符上限（50 行 / 单行 200 字符 / 段 4K 字符，超限截断 + 指针）。
- 参数定档：T=64K 字符/分区、W=10 MiB 存储字节、K=10、展开下限 20%
  （§11.1 v0.7），编译期默认 + env 覆盖（`ORZ_BLACKBOARD_FOLD_PARTITION_
  CHARS` / `ORZ_BLACKBOARD_LIVE_BUDGET_BYTES` / `ORZ_BLACKBOARD_FOLD_TAIL_
  ROUNDS`）。

## 2. 口径落地（设计未显式定稿处的实现决策，登记为裁决后口径）

1. **可折叠分区集合 = exec / edits / tool_actions**（带 (round, domain)
   章的累积行分区）。actions 结果板有界（50 条）且 receipt 按 id 点读，
   不进入折叠；plan/entities/deps/retrieval/session/temporal 为 live/
   状态面，携带展开参数 = 显式错误。R2 的「pre-stamp 只能 since/receipt_id」
   相应落在 edits/tool_actions（since）与 actions（receipt_id）上。
2. **段 = 存储序连续同域行**；跨段同域不合并（R6）。pre-stamp 行
   （domain=None）独立成段。标注行落在「该段折叠行」位置：段内折叠行是
   前缀时标注先行、展开行随后，保持时间序。
3. **当前域段** = 轮区间含 `current_round`（LIF 会话相对轮，B1 轴）的最晚
   段；尚无行写于当前轮（域机器刚迁移、读取在写行前）时回退到最晚段——
   这是「当前轮所在域连续段」在无行轮上的最佳近似（语义锚点 `current_domain`
   保留为注释记录，不做二次过滤）。
4. **最近 K 轮窗口 = 行 round ∈ [current_round−K+1, current_round]**（含
   边界），round=0 的 pre-stamp 行永不计入轮窗口；**最近 20% 行** = 存储序
   尾部 `ceil(total×20%)` 行（向上取整，保底可见短尾）。三者并集。
5. **显式展开在未达折叠阈值时语义**：折叠态不存在时目标行已在全量视图内，
   输出与普通读取一致（不额外裁剪、不报错）；达到阈值时 = 折叠态 + 目标行。
   展开范围无匹配行时输出显式提示（域段不存在 / 轮数范围外 / pre-stamp
   需 since/receipt_id），绝不静默空回。
6. **标注预览口径**（设计「Top-N 路径/失败行」的机械落地，不解析语义）：
   edits = 折叠段最新两条不重复文件路径；tool_actions = 折叠段类别计数
   Top-2（read/edit/terminal/retrieval/other，计数降序、平序稳定）；
   exec = 折叠段最新两条不重复文本预览（各 ≤200 字符再整体截断）。
   pre-stamp 标注 = 计数 + 时间范围（无时间戳时明示「无时间戳」）。
7. **cap 与折叠视图总上限**：非折叠 edits/tool_actions 与 exec 同纪律
   （50 行 / 200 字符 / 4K 字符，超限只留头行 + 计数行 + 指针）；折叠视图
   同样受 4K 字符总上限约束，从最旧整行开始丢弃并加头行说明（标注/行都不
   会静默消失，完整内容可经展开参数回查）。
8. **W 的读时计量 = `Blackboard::live_compact_bytes()`**（紧凑 JSON 字节，
   序列化失败回退 0 = 不折叠；live 面含 actions 注册/订单槽，与 run 末
   会话快照口径差一次注册/订单清理，量级可忽略）。T 的字符计量 =
   `live_foldable_partition_chars()` 对未截断行渲染文本的估算（T=64K 量级
   下估算偏差不改变触发结论）。归档 epoch 读（`--plan`/历史）不折叠、
   逐字节不变。

## 3. 组合守卫（fail loud，全部 exit_code 1 + error 字段）

| 场景 | 结果 |
|---|---|
| domain / round_from / round_to 缺任一 | 显式报错（all-or-none） |
| domain 非法字符串/非字符串 | 显式报错（start\|normal\|pressure\|low_progress\|stuck） |
| round_from/round_to 非 ≥1 整数 | 显式报错 |
| round_to < round_from | 显式报错（含边界，倒置拒绝） |
| 展开参数 + receipt_id | 显式报错（互斥） |
| 展开参数 + since_timestamp | 显式报错（互斥） |
| 展开参数 + epoch | 显式报错（归档为历史视图，无 live LIF 上下文） |
| 展开参数 + 非可折叠分区 | 显式报错（仅 exec\|edits\|tool_actions） |

## 4. 已知边界（登记，不阻断）

- exec 分区无条目级 `since` 过滤（既有语义）：pre-stamp exec 旧行 ts 为空，
  折叠后只能经分区全文（未达阈值）或整板存档回查；edits/tool_actions 的
  pre-stamp 行经 `since` 过滤展开（保留既有能力）。
- 折叠态只对 live 读取生效；归档 epoch 读不携带 live LIF 上下文，维持
  B1 渲染逐字节不变。
- 折叠视图行序以「results 后 errors」的存储序为轮序近似（与既有 exec
  渲染同链序），不做时间戳重排。
- W 阈值在单分区低于 T 时仍会因整板字节 ≥10 MiB 进入折叠（§9.2.4 OR 语义
  落地）；读时整板紧凑序列化为 O(板大小)，PULL 频次下可接受，B4 遥测记录。
- 折叠视图是纯渲染投影：不推进任何版本/游标语义之外的附加状态（增量头仍
  按分区版本计）；存储、事件面、机械消费方零改动。

## 5. 代码与测试入口

- 纯函数核心：`orz/crates/orz-loop/src/render_fold.rs`（参数定档 + env、
  展开参数解析、域段切分、默认展开子集、触发判定；10 个单测）。
- 分区折叠视图组装与 cap：`orz/crates/orz-loop/src/epoch.rs`（edits /
  tool_actions / exec 折叠渲染 + `render_capped_rows` 补齐 cap；5 个单测）。
- 接线：`controller.rs` `render_blackboard_section_fold`（触发路由 +
  live LIF 上下文）、`host_exec.rs`（展开参数解析 + 守卫）、工具声明
  （domain/round_from/round_to schema）、`blackboard.rs`
  `live_compact_bytes`。
- S2 工具链测试：`blackboard.rs` 3 项——声明面增量；五路守卫 fail loud；
  折叠默认视图 + 显式展开 e2e（T 触发 + B1 会话恢复轴）。

## 6. 验证证据（2026-09-03）

- orz-loop lib 全量：**676 passed / 0 failed / 3 ignored**（B1 基线 658 +
  新增 18）。
- orz-host lib 全量：241 passed / 1 failed / 4 ignored——唯一失败为既有
  flake `call_tool_timeout_kills_process_tree`（B1 审计 §2 已登记；单独
  复跑通过，与本批改动无关）。
- `cargo fmt --all --check` 干净；clippy（lib）无新增告警（仅既有告警）。

## 7. 后续

- B3 契约与收尾：空槽「（无）」、用户侧疲劳提醒（复用 W/水位计量与
  `session` 面）、存档单包 gzip + `session_archive` 事件、plan-epoch 生产面
  退役清理（`--plan` 保留）。
- B4 验证：S3 重建 + S4 实机复验（web 通道 A/B、折叠态读取与展开、恢复、
  长会话遥测 §13.3）；建议并入既有 B4 用例集：进程重启后黑板/LIF 续接、
  失败 run 后检索分区延续（B1 复审登记），以及折叠视图/展开在真实长会话
  中的读取分布与响应大小。
