# 重文件拆分勘察报告（2026-09-13）

> 日期：2026-09-13（只读勘察，非实施批；由全项目深审 S-15 讨论引申的用户提问触发
> ——「除了 S-15，是否还有应该拆的重文件」）。方法 = 全仓源文件 `wc -l` 扫描
> （orz crates 的 `.rs`、assurance 的 `.py`）+ 车道归属判定（生产车道 vs 休眠/
> 血统车道，车道权威 = `OBS-PERMISSION-DUAL-IMPL` 判定面单图与 AUTH-INDEX 各条目
> 状态）+ 文件职责核对（抽读）。**勘察不立项**：拆分候选留用户裁决，本文只承载
> 证据与建议。扫描时点基线 = orz `0b2a8f5b`（同日 P1-4 批之后）。

## 0. 结论速览

- **生产车道存在三个真拆分候选**（按优先序）：`orz-loop/src/host_exec.rs`（9,184 行，
  生产面第一重且仍在增长）、`orz-loop/src/gateway/transport.rs`（5,846 行，流式/非
  流式双实现同文件 = 深审 P2-7 重试链漂移的温床，拆分可与 P2-7 修复同批）、
  `orz-assurance/src/journal/families.rs`（6,491 行，按事件族可机械分模块，随每批
  新增事件族加重）。
- **超重但明确不建议拆的**：orz-workspace `handle.rs`（10,010）与
  `permission/manager.rs`（8,761）、orz-sampling-types `conversation.rs`（9,993）、
  xai-ratatui-textarea `textarea.rs`（9,762）、orz-mcp `servers.rs`（7,703）、
  xai-file-utils `queue.rs`（6,475）——全部位于休眠/血统车道，按既有治理
  （OBS-PERMISSION-DUAL-IMPL「薄层 owner、自研面不膨胀」）正确出路是**退役/冻结
  裁决**，不是投入拆分精力。
- **不属于拆分对象的重量件**：Python `run_event_journal_validation.py`（3,911）是
  Task D 翻转后的**冻结 reference**（合约面不动）；`windows_sandbox.py`（3,598）、
  `retrieval_subagent.py`（1,939）为 reference/冻结面。
- **已治理件核对**：`epoch.rs` 经 P0-GOV 任务 A 剥离后仅余 1,977 行（不再是问题件）；
  `controller.rs` 6,031 行，自 2026-08-29 拆分验收（4,142，验收线 ≤10,000）回长
  1,889 行，仍在验收线内，观察即可、暂不动。

## 1. 生产车道候选（留裁决，按优先序）

### 1.1 `orz-loop/src/host_exec.rs` —— 9,184 行（第一优先）

- 证据：当前生产车道最大单文件，超过 controller.rs 拆分时的验收线量级
  （controller 拆分验收 = 单文件 ≤10,000，本文件 9,184 已贴线）；且 0z S2/S2R
  又向其加入 serp-attempts 持久化（`persist_serp_attempts`）与资源门接缝等新职责，
  增长趋势未停。
- 职责混杂度（抽读）：host 工具装配/完成装配点（0q `stamp_failure` 单一漏斗）、
  SERP 预算结算与取证面落盘（0v-A/P2-4）、失败信封装配、预算结算等——至少四个
  可切职责域。
- 建议：沿 AUTH-CONTROLLER-SPLIT 先例做 pub(crate) 机械拆分（事件序列与 journal
  链不动、按职责归位、验收线单文件 ≤10,000 且各模块单一职责）。

### 1.2 `orz-loop/src/gateway/transport.rs` —— 5,846 行（与 P2-7 同批）

- 证据：深审 P2-7 实锤流式/非流式重试链行为漂移（非流式 D-6 空响应链无退避、
  错误直接 `?` 中止，transport.rs:2095-2101；流式有 backoff 500ms→10s + cancel
  感知，:2144/:2224-2226）——双实现共存同一文件正是漂移的结构性原因。
- 建议：拆分为 streaming / non-streaming（或 request/retry/frame）模块并在拆分时
  统一退避纪律，P2-7 随批闭合、一石二鸟。

### 1.3 `orz-assurance/src/journal/families.rs` —— 6,491 行（机械可切）

- 证据：journal-conformance 法官 35 族判定全在一个文件（0z S2 一批 +7 族后到达
  该量级），每批新增事件族都线性加重。
- 建议：按事件族分模块（如 terminal / resource / retrieval / delivery 分文件），
  纯机械搬移 + 注册表聚合；与 Python 冻结镜像的对拍面不受影响。

### 1.4 观察件（不动）

- `orz-loop/src/controller.rs`（6,031；拆分时 4,142）：验收线内回长，登记观察，
  再次逼近 10,000 时按同先例再拆。
- `orz-tools/.../bash/mod.rs`（6,021）：工具实现内聚度高，暂无漂移证据，不动。

## 2. 休眠/血统车道（不建议拆；治理出口 = 退役/冻结裁决）

| 文件 | 行数 | 车道归属与治理依据 |
|---|---|---|
| `orz-workspace/src/handle.rs` | 10,010 | orz-workspace 移植层栈（OBS-PERMISSION-DUAL-IMPL：15 文件 29,842 行，仅 manager actor 核心活跃，薄层 owner 形态特意保留） |
| `orz-sampling-types/src/conversation.rs` | 9,993 | 冻结期血统 crate（sampling 合约类型） |
| `xai-ratatui-textarea/src/textarea.rs` | 9,762 | 成熟组件 vendor 件（组件整进，不宜拆内部） |
| `orz-workspace/src/permission/manager.rs` | 8,761 | 同 handle.rs（移植层权限栈，休眠面冻结） |
| `orz-mcp/src/servers.rs` | 7,703 | 血统 MCP server 面（生产 MCP 默认未接线） |
| `xai-file-utils/src/queue.rs` | 6,475 | 血统工具件 |

拆分休眠面 = 给没有生产承诺的代码支付持续的搬移与回归成本，与「自研面不膨胀」
治理原则相反；其重量问题应在**退役/冻结裁决**（本体退出或明确冻结）时一并消解。

## 3. Python 保障侧

- `assurance/run_event_journal_validation.py`（3,911）：Task D 翻转后的冻结
  reference（仅对拍角色），**合约面不动**，不拆。
- `assurance/windows_sandbox.py`（3,598）/ `retrieval_subagent.py`（1,939）：
  reference/冻结面，不拆。
- 其余 `assurance/` 文件均 < 2,600 行（tests 与 TUI 血统为主），无拆分压力。

## 4. 与既有账面的关系

- 本勘察由深审 S-15（schema description 承载流水）讨论引申，但与 S-15、0ab
  （markdown 账本瘦身）**不同物**：S-15/0ab 处理文档与机器合约的流水承载，本文
  处理**源代码文件**的规模与职责结构。
- 深审 P2-7 在 §1.2 中作为拆分收益（同批闭合）引用，不因本文重复立项。
- 本批（orz `0b2a8f5b`）Docker Linux 验证连带发现的两处 Linux 构建断裂已随批
  修复并记入深审 P1-4 后记，与本文无涉，仅作时点注记。

## 5. 建议（留用户裁决，未立项、不动计数）

1. **host_exec.rs 拆分**立项（沿 CONTROLLER-SPLIT 先例，S1 设计切分图 → S2 机械
   搬移 → S3 回归核验；验收 = 行为不变 + 单文件 ≤10,000 + 职责域单一）。
2. **transport.rs 拆分 + P2-7 同批**立项（拆分即修复载体）。
3. **families.rs 按族分模块**立项（纯机械，可与下一个事件族批同车）。
4. 休眠面六件**不拆**，登记进 OBS-PERMISSION-DUAL-IMPL 终局治理视野（退役/冻结
   裁决时一并处置）。
