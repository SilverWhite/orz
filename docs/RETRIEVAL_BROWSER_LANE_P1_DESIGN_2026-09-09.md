# 0t S2-R P1 设计定稿：浏览器车道控制与事实口径（2026-09-09）

> 状态：`released`（2026-09-09 用户复核放行；P2 正确性批 / P3 语义声明批
> / P4 卫生批均已实施；P1–P3 实现复核与 X1–X4 处置见
> [`0T_S2R_P1P3_IMPL_REVIEW`](audits/0T_S2R_P1P3_IMPL_REVIEW_2026-09-09.md)）。
> 上游：
> [0t S2 Task 1 审查处理](audits/0T_S2_TASK1_REVIEW_HANDLING_2026-09-09.md)
> （P1-1/P1-2a/P1-2b/P2-2 处置与 §3.6 价值裁决）· 设计权威 v1.3 ·
> ADR-0010 §14.65。计数不动。

## 0. 背景与裁决

- 用户第五轮裁决：orz 定位 = 辅助模型更好地执行动作；导航级控制有开放
  价值，开放 Phase 1 最小动作集；click/type/任意 JS eval 与多内核接口
  扩展记录不行动（审查处理 §3.6）。
- 用户指示执行 P1（设计定稿轮）。本稿定稿三项：A) `browser_control`
  工具面与每动作日志特征回传；B) P1-2a 启动事实口径；C) P2-2 外部 lane
  `web_fetch` 声明恢复。退出条件 = 本稿经用户复核放行。

## 1. 目标与非目标

**目标**：让外部检索子代理在启用会话内获得「导航级浏览器控制 + 每步状态/
日志反馈」，模型能在真实页面（重定向/JS 等待/同意墙/失败阶段）中途分支；
启动事实与页面结果解耦，审计可区分「无启动尝试 / 启动成功但页面失败 /
启动失败」；原生 lane 声明面与设计措辞一致（含 web_fetch）。

**非目标**：click / type / 任意 JS eval（Phase 2，凭据/注入/ACAF 面，
记录不行动）；多内核（Firefox/WebKit）接口；无头评测容器特化；主面或
内部 lane 开放浏览器控制。

## 2. 设计 A：`browser_control` 工具面

### 2.1 形态与动作集（Phase 1）

单工具多动作：`browser_control`（参数：`action` + 动作专属参数）。动作集
只含导航/状态观测，**不含正文读取**——证据与候选纪律保持在 `browser_read`
单工具，避免在候选门内做 per-action 分类。

| action | 语义 | 返回 | 计候选 | 进 ledger |
|---|---|---|---|---|
| `navigate(url)` | 直导航到 URL（URL gate / ACAF 与 browser_read 同级） | 状态 + 有界日志特征 | 否 | 否 |
| `back` / `forward` / `refresh` | 历史后退/前进/刷新 | 同上 | 否 | 否 |
| `wait_load` | 等待加载/文本就绪，有界超时（默认 ≤30s，可配） | 同上 | 否 | 否 |
| `snapshot` | 当前页状态观测 | 同上 | 否 | 否 |

内容读取仍走 `browser_read`（正文全量 + W-F13 截断/检索对象纪律 + 候选
消费 + evidence ledger）。`browser_read` 增加「同 URL 免重复导航」优化：
目标 URL 与当前页一致时跳过导航直接读（对既有调用透明，url 参数保持必填
以兼容 schema/校验器与 ACAF 拒单路径）。

### 2.2 每动作返回信封（特征回传）

所有动作（成功或状态化失败）统一返回机械段：

- `action_status`：`ok` / `error`；
- `error_class`（error 时）：DNS / 连接重置 / 超时 / 拦截 / 证书 / 其它
  （真实类别，FP-2 口径，无教学句）；
- `nav_phase`：`idle` / `loading` / `completed` / `error`；
- `url` + `title`（当前落点，识别重定向）；
- `log`：有界日志特征——console error/warning + network error 类节选，
  默认 ≤12 行 / ≤2 KiB，超出截断标记；console 文本按页面注入面容器标记
  （`[browser_log]` 引用语域），不得以页面原声进对话正文；脱敏沿用既有
  纪律（URL/header/凭据 pattern）。

边界：日志特征**只进工具结果文本**（超限走既有输出检索对象/截断机制），
不进 journal 事件载荷、不进黑板、不进主面；事件面只落既有轻量事实
（`browser_launch_result`，§3）。

### 2.3 会话状态机与串行化

- 浏览器车道 = 单会话单 CDP 会话 + 单 tab 状态机；`browser_control` 动作
  经会话级控制 tab 全程持锁串行（实现在 CDP 会话层，P3 交付时用户接受的
  取舍）。`browser_read` 池化读与 `browser_control` 不互相阻塞：同 URL
  复用控制 tab 读取为 `try_lock` 尽力而为（忙则回退池读），无 tab 状态
  竞争与并发误读（2026-09-09 X4 注记同步，见
  `0T_S2R_P1P3_IMPL_REVIEW` F5）。
- 懒启动沿用 browser_read 机制（P1-1 生命周期 + P2-1 check+launch 竞态
  修复为前置，见审查处理 §5 依赖）。
- 导航动作失败按普通错误回传（FP-2）：启动层失败 = `BrowserLaunchFailed`
  （现码）；导航/等待层失败 = 真实类别错误文本，不做模式拒绝、不教学。

### 2.4 声明与门

- `browser_control` 为 host-owned 工具；启用会话恒声明（与 browser_read
  同声明路径——P2-3 将 registry 语义改为「enabled 恒声明、调用期懒启动」，
  本设计以此为最终口径）。
- external lane 投影：启用会话恢复 `browser_read` + `browser_control`；
  主面与内部 lane 不声明；未启用会话无任何浏览器/检索工具（fail-closed
  不变）。
- relay：`route(browser_control) = Host`；加入
  `is_retrieval_mode_gated_host_tool`（未启用即拒）；不加入
  `is_candidate_counted_tool`（正文读取仍走 browser_read，计数域不扩）。
- 静态标注：browser_control 属本地浏览器车道，description 附
  `[车道:本地浏览器检索|推荐首选]`（与 browser_read 同车道同文本，固定
  零动态）；不新增提示词推荐句（沿用既有 ≤1 句推荐序）。

### 2.5 与既有纪律的关系

URL gate / ACAF 网络规则与 browser_read 同级；SERP 主序/pacing 纪律保留
（搜索仍走 web_search/机械 SERP 层，控制工具不承担键入模拟）；来源加权/
证据分级/失败漏斗（0q）跨车道语义不变；导航动作无证据主张，不污染
structured result。

## 3. 设计 B：P1-2a 启动事实口径定稿

### 3.1 语义定义

`browser_launch_result` 记录**启动/探活尝试本身**的成败（fact 对象），与
本次调用后续动作结果**解耦**。时间顺序恒为 ToolStarted → fact →
ToolCompleted（若同轮发生懒启动）。

### 3.2 场景矩阵（浏览器工具通用）

| 场景 | 形态 | fact | ToolCompleted |
|---|---|---|---|
| S1 启动失败（首调） | `BrowserLaunchFailed(cause)` | failure(cause 真实类别) | error，稳定码 `browser_launch_failed`（现状） |
| S2 启动成功 + 动作成功 | Ok + `ToolResult.browser_launch_fact` | success（cause=null） | 正常完成（现状） |
| S3 启动成功 + 动作失败（首调） | Err 且携带 launch_fact=success（**新增**） | success（cause=null） | error，真实错误文本（页面/导航类别，不改写稳定码） |
| S4 已就绪后续调用 | — | 无 fact | 正常/错误（现状） |

### 3.3 接缝方案与触点

- 结果侧统一携带 `Option<BrowserLaunchFact>`：Ok 经
  `ToolResult.browser_launch_fact`（现状）；Err 侧新增
  `ToolError::BrowserStepFailed { reason: String, launch_fact:
  BrowserLaunchFact }`（仅「启动成功且动作失败」使用；启动失败仍走
  `BrowserLaunchFailed`，未启动的普通失败仍走 `ExecutionFailed`）。
- 映射：`host_error_code` → `CODE_EXECUTION_FAILED` 系（不新增稳定码）；
  `tool_error_kind` → ExecutionFailed；Display `"browser step failed:
  {reason}"`；0q `stamp_failure` HostError 分类归 ExecutionFailed 系。
- host_exec Err 分支：先 journal err 携带的 fact（如
  BrowserStepFailed.launch_fact / BrowserLaunchFailed 失败 fact），再写
  ToolCompleted；Ok 分支现状不变。
- orz-host `call_tool`：browser 动作在 launch_attempted 且启动成功后，
  若后续失败把原因包为 `BrowserStepFailed` 并携带 success fact。
- 触点清单：orz-loop host.rs / host_exec.rs、orz-host lib.rs /
  local_browser/mod.rs、console.rs（无新码，核对即可）；families.rs 规则
  不变（仍只强制 launch-failure 前置），但 P5（Task 2）补 S2/S3 正例与
  S1 反例场景及 Rust↔Python 对拍。

## 4. 设计 C：P2-2 外部 lane `web_fetch` 声明恢复

- 口径：启用会话 external lane 声明集 = 浏览器车道 `browser_read` +
  `browser_control` + 原生车道 `web_search` + `web_fetch`（与设计 §3.1
  双族措辞及激活提示的 web_fetch 验证句一致）。
- 方式：投影恢复列表由 `["browser_read"]` 扩为 `["browser_read",
  "browser_control", "web_fetch"]`，从 host registry 取 def（web_fetch
  属 finalized toolset，registry 恒含；registry 缺 def 时不发明，与既有
  EmptyRegistry 测试语义一致）。
- 标注：web_fetch description 追加 `[车道:原生检索]`（与 web_search 同
  车道固定文本）；提示词不改（声明补上后与既有 web_fetch 验证句一致）。
- 计数纪律：web_fetch 维持既有候选计数（`is_candidate_counted_tool`
  已含 web_fetch 族），无变化。
- 验证：投影测试补 enabled 场景（browser_read + browser_control +
  web_search + web_fetch 在 lane）与 disabled 场景（全无）。

## 5. 影响面与不变项

- schema/registry/事件面：无新事件类型、无新 payload 字段；
  `browser_launch_result` 复用；`browser_read` 载荷/校验不变。
- token/注入：新增 1 条工具声明 + 2 条静态标注文本（browser_control 属
  本地车道、web_fetch 属原生车道），只进一次性子代理工具面，零新增常驻
  token；不进主面/黑板/常驻提示。
- 记录不行动（重申）：click/type/任意 JS eval；多内核接口扩展。
- 回退口径：S4 宿主机日常实测若无实证收益，browser_control 降级回只读面
  （工具不下发即可，代码保留）。

## 6. 实施归属与验收

| 实施批 | 内容 | 验收 |
|---|---|---|
| P2（正确性批） | P1-1 跨 prompt 浏览器生命周期 + 跨 prompt ACP 测试；P1-2a（BrowserStepFailed 接缝 + S3 场景单测 + ToolStarted→fact→ToolCompleted 全序断言） | 新增/改写测试绿；orz-host/orz-loop lib 绿 |
| P3（语义/声明批） | P2-1 懒启动并发竞态；P2-2 web_fetch 声明恢复 + 标注/投影测试；P2-3 registry 声明语义对齐；P1-2b browser_control 实现（ToolDef/relay/投影恢复/会话动作队列/动作执行/特征回传/同 URL 免导航优化 + 单测） | 对应单测绿；投影与特征回传测试绿 |
| P5（conformance） | 两族正反例补全（含 S1/S2/S3 场景、gate 反例、声明面场景） | Rust↔Python 对拍绿 |

## 7. 待用户放行复核点

1. `browser_control` 动作集 = navigate/back/forward/refresh/wait_load/
   snapshot，正文读取仍走 browser_read（不含 read_content 动作）？
2. 特征回传字段与边界（≤12 行/≤2 KiB、`[browser_log]` 容器、只进工具
   结果、不进事件/黑板）？
3. P1-2a 语义 = 启动事实独立于页面结果；接缝用
   `ToolError::BrowserStepFailed{reason, launch_fact}`？
4. web_fetch 恢复 + `[车道:原生检索]` 标注？
5. browser_control 归本地车道标注 `[车道:本地浏览器检索|推荐首选]`？
6. 会话动作串行化放 host 侧、导航动作不计候选不进 ledger？
7. browser_read 同 URL 免重复导航优化（url 仍必填）？
