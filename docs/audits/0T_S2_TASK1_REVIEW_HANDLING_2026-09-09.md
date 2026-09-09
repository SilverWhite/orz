# 0t S2 Task 1 全面审查处理（2026-09-09）

> **范围**：0t 检索子代理双车道 S2 已落地实现（Task A 独立启用门/双车道
> 网关改写 + Task 1 `browser_launch_result` 生产闭环，含其下
> schema/verifier/fixtures 先行层）对照设计 v1.3 / ADR-0010 §14.65 的全面
> 审查——设计合理性 / 实现合理性 / 设计-实现符合性三面。orz 子模块工作树
> 未提交改动共 22 文件（+1056/−1820），证据基线 = 审查当日工作树。
> **方法**：三路子代理并行只读深审 + 父级高危路径亲核（gate/双族/标注/
> 退役面/事件顺序/句柄生命周期/声明面）；**本轮未改任何实现文件**。
> **结论**：无 P0；P1×2 + P2×4 + P3×N；Task 2/3 排期内待办确认。
> **计数**：不动计数（审查与排期登记；闭合按条目入账纪律不变）。
> 入口：BACKLOG 0t / TODO P0-0t / 设计
> [`RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09`](../RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md)
> / ADR-0010 §14.65 / 索引 FUS-RETRIEVAL-DUAL-LANE。

## 0. 用户本轮裁决登记（2026-09-09 第三轮）

1. **P1-2 需处理**，目标 = 让模型更便捷地使用浏览器检索；用户提出方向：
   「每个浏览器动作返回部分浏览器日志中的内容或特征，让每一步都有合适的
   回应」——本文档 §3 记录父级评估与建议，并以设计先行子项排期（§5）。
2. 其余发现（P1-1 / P2-1 / P2-2 / P2-3 / P3 卫生批）**均批准按建议方案
   修复**。
3. 先落本文档 + 排期修复任务（S2-R），**接下一步执行，本轮不连做**。
4. **多内核接口口径（同日，用户第四轮）**：后期若有其他浏览器需求再扩展
   其他内核（Firefox/WebKit 等）接口——记录不行动；无其他补充，指示按 §5
   正式行动阶段划分并排期（不连做）。
5. **导航级控制工具开放裁决（同日，用户第五轮）**：orz 定位 = 辅助模型
   更好地执行动作；经价值评估（§3.6）确认导航级控制有开放价值——开放
   Phase 1 最小动作集（browser_control：navigate / back / forward /
   refresh / wait_load / snapshot；read_content 保持 browser_read 语义）；
   click/type 等交互动作与多内核仍记录不行动。
6. **P1 执行指示（同日，用户第六轮）**：同意 §3.6 判断并指示执行 P1
   （设计定稿轮）。P1 产出 =
   [`RETRIEVAL_BROWSER_LANE_P1_DESIGN_2026-09-09`](../RETRIEVAL_BROWSER_LANE_P1_DESIGN_2026-09-09.md)，
   待用户复核放行后进入 P2。

## 1. 审查范围与方法

- 设计权威：设计 v1.3（§3.1–§3.4 / §4 / §5）、ADR-0010 §14.65、
  BACKLOG 0t / TODO P0-0t 触点清单与替代 conformance 义务。
- 实现证据：orz 子模块 diff 面 22 文件——orz-loop（host.rs / host_exec.rs /
  console.rs / controller.rs / controller_test_support.rs / agent_loop.rs /
  prompt.rs / retrieval/{dispatch,projection,mode}.rs）、orz-host（lib.rs /
  acp_server.rs / stdio.rs；retrieval_mode.rs 已删）、orz-bin（main.rs /
  tests/acaf_e2e.rs）、orz-assurance（journal/{event,families}.rs /
  lif/channels.rs）、orz-tui（bridge.rs / events.rs / projection.rs），
  加 runtime schema/registry/fixtures、assurance Python 法官、
  scripts/generate_run_event_fixtures.py。
- 三线分工：设计合理性（语义/口径/触点/排期边界）· 实现合理性（正确性/
  生命周期/并发/错误映射/卫生）· 符合性（逐条要求→证据矩阵）。
- 测试证据（审查当日子代理定点实测，均绿）：orz-assurance lib 210；
  orz-loop retrieval 99 + browser_launch 2；orz-host browser_read 2；
  Python run-event conformance 15。

## 2. 发现总表

| 编号 | 严重度 | 主题 | 证据（审查当日） | 归属 |
|---|---|---|---|---|
| P1-1 | P1 | ACP/stdio/TUI 多 prompt 会话跨 run 浏览器生命周期回归：run 前存默认 unavailable 句柄、run 尾不回写真实句柄、host drop 杀浏览器 | orz-host/src/acp_server.rs:1351/1439/1937；local_browser/cdp.rs Drop→kill | Task 1 收口前置（R1） |
| P1-2 | P1 | 「每次启动/探活尝试落事实事件」口径偏差：实现仅在整次调用成功时落 success fact；启动成功但页面失败无 fact（R3「可拉起但实质不可用」审计不可区分） | orz-host/src/lib.rs:599–607；设计 §3.3 / ADR §14.65 | 需处理（§3 评估 + S2-R 排期） |
| P2-1 | P2 | 首次并发 browser_read check-then-launch 竞态：双调用同见 not-ready，第二 Chrome 同 profile 拉起失败误报 BrowserLaunchFailed | orz-host/src/lib.rs:579；agent_loop.rs 并发档位 | R2（建议修复） |
| P2-2 | P2 | 外部 lane 声明面缺 web_fetch：主面投影剔除 web_fetch、子代理投影只恢复 browser_read；声明 = browser_read + web_search，与设计 §3.1/激活提示「web_fetch 验证」不符 | orz-loop/src/retrieval/projection.rs:141；prompt.rs:417 | R2（口径按建议：registry 恢复 web_fetch 进 lane） |
| P2-3 | P2 | 声明模型残留 ready-gated 旧语义（注释 + registry 测试），与静态双族矛盾；现由 declare_browser_declared() 救回 | orz-host/src/lib.rs:239/1442 | R2 |
| P2-4 | P2 | bare TUI 无检索启用开关（`--retrieval-enabled` 仅 -p/--stdio 生效）；非 0t 回归，但影响日常交互面 | orz-bin/src/main.rs:1163；orz-tui acp_client | R-D1（随 S4 载体裁决） |
| P3-* | P3 | 卫生批：§3.7.1 条文号引用残留、env 非法值静默、测试未锁全序、families 注释 31→33、过时注释（lib.rs/relay.rs）、未用 import/dead code | 多处 | R3 |
| T2/T3 | — | 排期内待办确认（非 Task 1 缺口）：两新 conformance 族无正反例场景；三个 `#[ignore]` capture 仍断言退役语义 | families.rs；orz-bin/src/main.rs:2452 起 | Task 2 / Task 3（原排期不变） |

### P1 细节

**P1-1 跨 prompt 浏览器生命周期回归**（三线一致确认）：懒启动换入的真实
manager 只活在单次 prompt 的 OrzHost 内。run 前仅把未启动的默认
unavailable 句柄存入会话（acp_server.rs:1351），run 收尾只有
activation/conversation 持久化、无真实句柄回写（约 1439 行之后），prompt
结束 host drop 后浏览器被杀；下个 prompt 只能整体冷启动，close_session
永远拿到 unavailable 句柄、profile 目录不清理，且存在同 profile 撞锁竞态
风险。与既有注释「launched on a previous prompt — the process stays up
across runs」及模块「launch once, reuse across runs, kill on shutdown」
注记直接矛盾，实质削弱设计 §3.5 宿主机日常可用目标。建议修复：run 收尾
在 `host.browser_session()` ready 时回写 `StoredSession.browser`，删去
run 前存默认句柄的代码，并补一个跨 prompt 复用的 ACP 测试。

**P1-2 启动事实口径偏差**：设计 §3.3 / ADR §14.65 字面为「浏览器每次
启动/探活尝试落事实事件」；实现只在启动 + 页面读取整次成功时附加 success
fact，启动成功但导航/读取失败走 `ExecutionFailed`、无任何 launch fact。
conformance 只强制 launch-failure 前置事实，故不产生漏报，但审计语义
不对称。§3 按用户方向给出评估与建议。

## 3. P1-2 设计评估（用户方向：浏览器动作日志内容/特征回传）

### 3.1 目标与问题形态

用户目标：让模型更便捷地使用浏览器检索——浏览器「可拉起但实质不可用」时
模型此前得到的是低信息量失败，难以决定重试/换道；希望每个浏览器动作返回
部分浏览器日志内容或特征，使每一步都有合适回应。问题形态：browser_read
（及未来浏览器动作）的**结果反馈面**信息增益，而非只解决 journal 事实
不对称。

### 3.2 评估

**方向值得做**：真实错误类别（DNS/连接重置/超时/拦截/证书）+ 页面加载
阶段 + 日志节选能给模型可行动信号，与 FP-2「真实类别普通回传」一致（日志
是原始事实而非教学句）；同时与既有 W-F13 阅读面截断/检索对象纪律同族。
需先设计再实施，边界约束：

1. **有界节选**：每动作回传日志须截断封顶（如 ≤N 行/≤K 字符、按
   严重度/相关性过滤、禁止整页 console dump），防注入预算（既有 50K 工具
   结果预算）与上下文膨胀。
2. **脱敏与注入**：日志可能含页面内容/PII/凭据片段，需既有脱敏纪律；
   console 文本属页面注入面，回传须机械标记（如 `[browser_log]` 容器 +
   引用语域标注），不得以页面原声进入对话正文。
3. **事件面保持轻量**：journal 事实事件只承载结构化状态/类别/原因（现
   `browser_launch_result` 形态），日志节选只进工具结果文本或受控检索
   对象；不在事件面膨胀（事件不带 args、轻量纪律不变）。
4. **动作覆盖**：启动（probe_launch）与页面级（导航/读取）分层——
   启动事实独立于页面结果落事件（P1-2a 口径修复），日志特征挂动作结果。
5. **与 §3.4/FP-2 对齐**：仅事实回传，不加机械建议句（「勿重试」类仍
   禁止）。

### 3.3 父级建议

分层处置：

- **P1-2a（审计口径，小改，可先行）**：启动尝试事实与页面结果解耦——启动
  成功即落 success fact（含「启动后页面失败」路径），或按设计注记钉死
  口径并固化进 Task 2 场景。
- **P1-2b（用户方向，设计先行）**：浏览器动作日志内容/特征回传——先行
  设计轮（§3.2 五条边界 + 形态定稿），再排实施与 Task 2 正反例固化；
  推荐起始形态：browser_read 结果附机械日志特征段（动作状态 + 错误类别 +
  导航阶段 + 有界日志节选 + 注入容器标记）。

### 3.4 待用户裁决点

1. 接受 P1-2a / P1-2b 分层（审计事实先修、日志特征设计轮后行）还是
   合并为单一设计轮。
2. 日志节选是否仅进工具结果（推荐），还是需受控检索对象/独立事件承载。
3. P2-4 TUI 检索开关是否随 S4 宿主机实测载体一并裁决（建议载体
   `-p`/`--stdio`，TUI 接线另行排期）。

### 3.5 用户第四轮方向与父级评估（2026-09-09，P1-2b 细化）

**用户方向**：给子代理工具面加「浏览器控制」，默认机械层直接拉接口
（CDP 直驱）；不走黑板（黑板在可见面给主 agent 加压，且黑板/`.gsa` 两段
门实际运行显示模型需要具体详细内容、黑板过简会促使模型回查具体内容）。

**事实核验**：

- 浏览器发现面 = Chrome / Edge / Chromium（Windows 标准路径 + PATH），
  均为 Chromium 内核、统一走 CDP（local_browser/discovery.rs、cdp.rs）；
  「浏览器内核基本是 Chrome 内核」对 orz 支持的浏览器家族成立。控制工具 =
  现有 CDP 能力（navigate/read/download）的显式化，非新协议；Firefox/
  WebKit 不在当前发现面，设计轮建议显式限定 Chromium 家族 + 保留
  `ORZ_BROWSER_PATH` 显式覆盖。
- 子代理工具面 = 主面剔除主车道专属工具（plan_write/blackboard_action_
  write/console_step_done/console_return_to_console/compaction_whitelist_
  add/retrieval_disposition/retrieve_project_*）后恢复 browser_read
  （缺口的 web_fetch 待 R2 补齐），整体 ≤ 主面。建议「单工具多动作」形态
  （`browser_control` + action 参数），新增声明压到 1 条，符合 §3.2
  一次性增量口径。
- 黑板顾虑成立：黑板折叠渲染/自历史为摘要面；0p/`.gsa` 两段门实证模型
  拿到结构预览后仍回查具体文件，说明「要具体内容」是强需求。检索动作反馈
  若只进黑板会迫使模型二次 PULL、增加主面可见压力——状态/日志/内容特征
  应**直接进子代理工具结果（有界）**，黑板只承担既有 ledger/结构化结果与
  归档职责，不新造浏览器结果黑板。

**父级建议（分阶段，先设计后实施）**：

- Phase 1（P1-2b 推荐落点）：`browser_control`（external lane 专用，启用
  会话才声明）动作集 = navigate / back / forward / refresh / wait_load /
  snapshot（状态 + 有界日志节选）/ read_content（复用 browser_read 全文
  读取/截断/检索对象纪律，W-F13）。每动作返回：动作状态 + 错误类别 +
  导航阶段 + URL/title + 有界日志特征（console/network error 节选，≤N
  行/≤K 字符），实现「每一步都有合适回应」。
- Phase 2（设计轮评估，不默认排入）：click/type 等页面交互——涉及 ACAF
  网络动作、凭据纪律（key 不落卷）、prompt-injection 面扩大；只对机械可
  识别模式（如 cookie 同意）开放或留用户交互出口，不作为默认自动化面。
- 边界：导航级动作沿用既有 pacing/频率上限/候选域纪律；事件面只落轻量
  事实（沿用 `browser_launch_result` 形态或增 `browser_action_result`
  轻量事件，设计轮定）；日志节选不进 journal；console 文本按页面注入面
  容器标记（如 `[browser_log]` 引用语域），不得以页面原声进对话正文；
  工具不进主面（与用户黑板顾虑同构），未启用会话 fail-closed 无此工具。
- 前置依赖：P1-1（浏览器生命周期跨 prompt 复用）与 P2-1（懒启动并发竞态）
  先行修复后，控制工具才具备可靠的多步连续动作基础。

**待用户裁决点（并入 R-P1-2b 设计轮起手）**：

1. 采纳「单工具多动作」形态与 Phase 1 动作集（导航/状态/只读为主）？
2. click/type 等交互动作推迟 Phase 2（ACAF/凭据/注入面单独设计轮）？
3. 确认反馈不进黑板、直接进子代理工具结果，事件面保持轻量？
4. 外部 lane 的 web_fetch 声明（P2-2）是否与 browser_control 一并恢复定稿？

**用户口径定稿（2026-09-09，第四轮暂定、第五轮修订）**：多内核接口扩展
记录不行动；click/type 等 Phase 2 交互记录不行动。导航级控制工具经价值
评估后开放（§3.6）——P1-2b 范围 = **browser_control（Phase 1 最小动作集）
+ 每动作日志特征回传**；特征回传边界（动作状态 / 错误类别 / 导航阶段 /
有界日志节选，含脱敏与注入容器标记；事件面轻量；不进黑板、不进主面）
不变；待裁决点 3–4 按建议确认（反馈只进子代理工具结果；P2-2 web_fetch
声明恢复进 lane，列入 R2 修复）。

### 3.6 导航级控制工具价值评估与开放裁决（2026-09-09 用户第五轮）

**问题形态**：仅 browser_read（单发 URL 读取）时，模型拿不到页面中途
状态——重定向落点、JS/懒加载等待、同意墙/「继续阅读」等交互前置、失败
阶段定位都不可见；每次读失败只有终态错误，缺少「这一步为何失败、下一步
如何走」的过程信息。

**价值判断（父级结论：有开放价值，建议窄而实地开放）**：

1. orz 定位 = 辅助模型更好地执行动作；双车道设计本就让模型自主选择与试错
   （§3.1 / §4 干预边界）。导航级控制是「模型自主」在浏览器车道的自然
   补全，相对只读单发不引入语义跳跃。
2. 真实页面检索（§3.5 宿主机日常可用口径）常见 JS/同意墙/分页前置；
   控制工具 + 每动作状态/日志回传让模型能在中途分支（等待 / 回退 / 换道 /
   换 URL）——即「每一步有合适回应」的落点；browser_read 终态错误提供不了
   该分支信息。
3. 成本可控：CDP 能力已存在（discovery.rs / cdp.rs），新增 1 条工具声明
   （external lane、启用会话），子代理工具数仍 ≤ 主面；与 P1-1/P2-1 修复
   天然衔接。
4. 风险有界：只开导航级 + snapshot（无 click/type/任意 JS eval）；内容
   读取仍走 read_content（browser_read 语义），evidence ledger/结构化
   结果只认读动作——§4 审计不变不破。

**开放边界（Phase 1 最小动作集）**：`browser_control` = navigate / back /
forward / refresh / wait_load / snapshot（每动作返回：动作状态 + 错误类别
+ 导航阶段 + URL/title + 有界日志节选）；read_content 沿用 browser_read
全文读取/截断/检索对象纪律（W-F13）。导航动作不消耗候选、不进 ledger
（无证据主张）；read_content 维持候选域与 evidence 纪律。URL gate / ACAF
网络规则沿用 browser_read 同级；动作按会话串行化（单一 CDP 会话状态机，
P2-1 修复后接线）；SERP 主序/pacing 纪律保留（搜索仍走 web_search/机械
SERP 层，控制工具不承担键入模拟）。

**记录不行动（保留）**：click/type/任意 JS eval（凭据/注入/ACAF 面，未来
有实证需求再评估）；多内核接口扩展。

**实施归属**：范围并入 P1 设计定稿轮（§5 P1）；实现并入 P3（R2 语义/声明
批）；S4 宿主机日常实测作为价值验收——若无实证收益则降级回只读面，保留
回退口径。

## 4. 已确认符合项（三线一致，避免误伤）

- 双车道工具面：启用会话外部 lane 恒注册 browser_read + web 族；标注与
  §3.2 逐字一致（`[车道:本地浏览器检索|推荐首选]` / `[车道:原生检索]`），
  只落工具描述与一次性激活提示，零动态字段、零新增常驻 token；内部 lane
  不变。
- γ 退役：生产路径无 `retrieval_mode_transition` / 机械降级 /
  `model_lane_switch` 写点；mode.rs 仅 wire 兼容解析 + 弃用映射；schema
  枚举、LIF 允许码、旧 fixture 保留只读回放；`retrieval_mode_requires_*`
  拒绝族从执行路径消失。
- 授权门 fail-closed：默认 false 一致覆盖 controller/CLI/stdio/ACP 与侧车
  legacy 归一化；未启用会话主面/子代理面均无检索工具；派发与 host 双闸
  均为无 ToolStarted 的 `retrieval_not_enabled` 结构化拒绝，拒绝码入 LIF。
- 生产闭环：已落刊路径事件顺序 ToolStarted → browser_launch_result →
  ToolCompleted 正确；failure cause 真实类别；ToolCompleted.error 稳定码
  `browser_launch_failed`；0q 失败漏斗/failure_agg 口径一致；无教学/
  「勿重试」文案。
- schema/registry/fixtures/生成器/Python 法官/Rust families/TUI 渲染同步；
  failure 必带非空 cause、success cause=null 约束正确；旧 journal 回放
  路径保留。

## 5. 正式行动阶段划分与排期（2026-09-09 用户指示）

> 用户裁决：本轮审查与方向记录收档完成、无其他补充；按下列阶段正式行动。
> 实施不连做——每阶段以既有纪律（设计定稿→用户放行 / 测试验收 / 门禁）
> 为出入口。

### 阶段总览

| 阶段 | 名称 | 内容 | 出口/验收 |
|---|---|---|---|
| P0 | 审查收档（本轮，已完成） | 本文档落盘 + TODO/BACKLOG 同步 + 不行动项登记 | — |
| P1 | S2-R0 设计定稿轮（2026-09-09 已产出，待放行） | P1-2b 设计：browser_control（Phase 1 最小动作集 navigate/back/forward/refresh/wait_load/snapshot；正文读取仍走 browser_read）+ 每动作日志特征回传；P1-2a 启动事实口径定稿；P2-2 web_fetch 声明口径定稿；click/type 与多内核不开放边界定稿（§3.6） | P1 设计定稿文档 + 用户放行（见 [P1 设计](../RETRIEVAL_BROWSER_LANE_P1_DESIGN_2026-09-09.md)） |
| P2 | S2-R1 正确性批 | P1-1 跨 prompt 浏览器生命周期修复 + 跨 prompt ACP 测试；P1-2a 启动事实口径修复（按 P1 定稿） | 新增/改写测试绿；orz-host/orz-loop lib 绿 |
| P3 | S2-R2 语义/声明批 | P2-1 懒启动并发竞态（profile 锁内二次检查或串行化 check+launch）；P2-2 外部 lane web_fetch 声明恢复 + 标注/提示测试；P2-3 registry 声明语义对齐 0t；P1-2b 实现（browser_control 接线 + 每动作特征回传，按 P1 定稿） | 对应单测绿；投影测试覆盖 web_fetch/browser_control 在 lane；browser_control 特征回传测试绿 |
| P4 | S2-R3 卫生批 | P3 全项：§3.7.1 条文号引用清理、env 非法值提示、fact 全序断言、families 注释 31→33、lib.rs/relay.rs 过时注释、未用 import/dead code | cargo check/clippy 零新增告警；fmt 净 |
| P5 | S2-T2 | conformance 正反例补全（retrieval_enable_gate / browser_launch_result；含 P1-2a/P1-2b 口径固化） | Rust↔Python 对拍绿 |
| P6 | S2-T3 | 三个 `#[ignore]` conformance capture 重写（retrieval_not_enabled / 双族 / browser_launch_result） | capture 重放绿 |
| P7 | S2-T4 收口 | BACKLOG/TODO/索引/ADR 注记同步 + manifest 重算 + 门禁 Exit 0；S2 闭合入账 | 门禁 valid |
| P8 | S3 重建 | 双平台三件套 + manifest | 构建冒烟绿 |
| P9 | S4 实机复验 + 宿主机日常可用性 | 判据 §5；R-D1（TUI 载体）起手裁决；对照 R3 检索主导 4 题或搭 0o 批次 L | 判据全过 / 用户裁决 |

### 记录不行动项（用户裁决，观察注记）

- 多内核接口扩展（Firefox / WebKit 等）：未来有需求再评估，不行动。
- click/type / 任意 JS eval（Phase 2 交互）：ACAF/凭据/注入面未决前记录
  不行动；未来有实证需求再评估。导航级 `browser_control` 已按 §3.6 开放，
  不在此列。
- 默认口径（假定，P1 设计轮确认）：P1-2b 反馈落 browser_control 每动作
  结果 + browser_read/启动失败事实（P1-2a）；不进黑板、不进主面、不进
  journal 事件载荷；未启用会话 fail-closed 不变。

### 依赖与顺序

P1-1 / P2-1（生命周期 + 懒启动竞态）是浏览器反馈可用性前提 → P2/P3 先于
P1-2b 生效评估；P5/P6 依赖 P1–P4 语义定稿；P7 依赖 P5/P6（代码冻结）；P8
依赖 P7；P9 依赖 P8 工件；R-D1（P2-4 TUI 检索开关）在 P9 起手裁决（建议
载体 `-p`/`--stdio`，TUI 接线另排）。

### 工作量评估

P1–P4 含约 8–10 个独立修改点 + 配套测试，预计 4–6 个任务轮（P1 设计定稿
1 轮；P2/P3 各 1–2 轮；P4 顺带 1 轮）；P5–P7 约 2–3 轮；P8/P9 按既有规模
各 1–2 轮。合计至 S4 约 8–12 个任务轮（含复查处理轮）。计数纪律不变：
闭合按条目入账。

## 6. 收口注记

- 审查当日未改任何实现/文档（除本处理文档与 BACKLOG/TODO 排期登记）。
- 本文档证据行号为审查当日工作树（未提交）状态；修复后以提交基线复核。
- 子代理三线报告结论一致；P2-2（web_fetch 声明面）为实现线交叉补充项，
  父级抽查（dispatch 传入主面投影后 tool_defs）确认成立。
