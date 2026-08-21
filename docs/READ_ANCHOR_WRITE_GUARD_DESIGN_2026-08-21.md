# read_file 内容锚点下传与写前机械核证设计（2026-08-21 设计定稿；S1 代码已实施，S2-S4 待续）

> 状态：`pending`（设计已定稿；**S1/S2 已闭合 2026-08-21**、S3-S4 待续）。
> 2026-08-21 用户裁决：锚=哈希值（内容摘要太重；read_file 须机械返回、助理层
> 零理解、简单机械核证）；仅针对 orz；先设计、不动作。
> **2026-08-21 用户指示开始 S1 实施**（见 §8）；**同日指示开始 S2 测试**（见 §9）。
> 性质：FUS-LARGE-FILE-READ-CONTRACT（ADR-0010 §14.22）读取信封扩展 +
> 写订单契约（CLASSICAL-EXEC-ASSISTANT 写面）。关联：
> [ADR-0010 §14.38](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)、
> [BACKLOG 0f](BACKLOG_AND_PRIORITIES.md)、[TODO P0-0f](../TODO.md)。
> 实施路由：S1 代码（已实施 2026-08-21）→ S2 测试（已闭合 2026-08-21）→
> S3 重建 → S4 复验。
> 实施入账 27 → 28（S1）；S2 测试闭合（计数不变 28）；验证闭环 28 → 27。

## 1. 背景与问题

主 agent 经 `read_file` 读取文件快照后，把写订单（search_replace / file_write
类）下发给助理层执行。在"读取快照 → 订单执行"窗口内，目标文件可能被其他
进程/工具修改；此时若仍按旧快照内容生成编辑，就出现**基于陈旧印象修改更新
文档**的问题。

关键结构事实：助理层（orz）是纯机械、无提示词的执行层——注册表路由 → 契约
校验 → 目标解析 → ACAF 票据 → 执行 → verifier 逐层执行，不理解动作语义
（CLASSICAL-EXEC-ASSISTANT）。因此它不需要判断"内容改了什么"，只需要一个
能机械回答的判定："**目标文件是否仍与读取快照是同一份内容**"。

## 2. 现状与先例

- read_file 大文件路径已返回读取句柄信封，其中已含 `content_sha256`
  （FUS-LARGE-FILE-READ-CONTRACT，ADR-0010 §14.22）：path / size / encoding /
  content_sha256 / 可用范围 / 有界预览 / truncated / offset。但小文件全文路径
  不携带任何内容锚点，且信封无统一 mtime 字段。
- 订单过期已有 `order_stale` 先例：round/plan_epoch/run_id 三重防重放与过期，
  v0.2 `console_order_rejected` 事件 phase=pre_issue / step=protocol 显式拒绝
  ——"机械拒绝 + 错误码信封"的形态可直接复用。
- 黑板/结果栏只放指针（path/size/digest/offset）先例：内容本体留在盘上，
  指针承载身份信息。

## 3. 设计定案

### 3.1 read_file 统一返回内容锚点（size + mtime + sha256）

- orz `read_file` 文本路径统一返回内容锚点 `{size, mtime, sha256}`：
  - 大文件信封：补 `size`/`mtime` 字段（`content_sha256` 已有）；
  - 小文件全文路径：同样返回锚点（随全文附结构化锚点，一次往返）；
  - PDF/PPTX/图片等无文本解码链路径不适用，锚点为 None（与既有编码门控
    边界一致）。
- 锚点与内容必须在**同一次读取快照**中生成（先读内容再取 stat/哈希，或先
  stat 后读并回验 size/mtime），避免读面自身 TOCTOU；size/sha256 取读取
  快照，mtime 允许在 size 与读取不一致时重 stat 一次（S1 实现口径，见 §8）。

### 3.2 写订单携带期望锚点

- 主 agent 下发写订单时，把 read_file 返回的锚点作为期望值透传：
  `expected {size, mtime, sha256}`。
- 委派消息/订单 schema 增加可选 `expected_anchor` 字段；缺失时保持既有行为
  （本设计不强制必填；"写订单必须引用读取锚点"的必填加严列为可选后续）。

### 3.3 orz 写门禁机械核证

写订单执行编辑前，orz 写门禁机械比对：

1. **快速预检**：stat 一次，比较 size/mtime——不等即拒；
2. **权威核证**：重算当前文件 sha256，与期望 sha256 逐字节比较——不等即拒；
3. **拒绝语义**：不匹配 → 拒单，返回机械错误码信封（复用 `order_stale`
   错误形态；错误码如 `content_anchor_mismatch`，入 `console_order_rejected`
   事件面），**不执行任何编辑**；
4. **匹配**：继续既有执行链（契约校验 → 目标解析 → ACAF 票据 → 执行 →
   verifier）。

核证期 I/O 错误语义（S1 审查收口）：目标文件不存在（新建路径）跳过核证，
与 search_replace 空 old_string 建文件的既有语义一致；其余 stat/read 错误
（权限/瞬时 FS/路径变目录等）**fail-closed 拒单**——无法取得当前内容锚点
即不放行编辑，错误信封同 code 并注明失败原因，主 agent 重读后重下。

### 3.4 补救动作

主 agent 收到拒单 → 重读目标文件 → 以新锚点重新下单。重读是唯一补救动作；
机械层只回答"是不是同一份"，语义适配留在主模型。

## 4. 为什么锚是哈希而不是时间戳（决策记录 2026-08-21）

- 时间戳是文件系统元数据，不是内容身份：可被保留（git checkout / cp -p /
  touch -r / 部分同步工具）或被取整（部分文件系统/API 粒度粗），"内容变了但
  mtime 没变"真实存在 → 只做时间戳会有漏判。
- 哈希绑定字节内容：内容变则哈希必变；等值比较是纯机械运算，助理层零理解。
- 成本：文档级文件计算 sha256 为微秒~毫秒级；且大文件信封已有 content_sha256
  先例，新增成本近乎为零。
- 用户裁决明确：不加语义级"内容摘要"（重且机械层不需要）；read_file 机械
  返回、简单机械核证的内容=哈希值。

## 5. 边界与残留

- **校验-写入 TOCTOU 窗口**：核证通过后、写入完成前仍有极小窗口；正常接受。
  若后续需要更严，升级为"临时文件 + 原子替换（校验后落盘）"，列为可选后续、
  不占计数。
- **只答"是否同一份"**：哈希不提供差异内容；补救=重读，由主 agent 执行。
- **"根本没读过"的陈旧印象**：模型凭训练/旧会话记忆改文件时手中无锚点——
  本设计通过"写订单携带期望锚点"使该场景在核证路径上可暴露（expected 缺失
  时的必填加严为可选后续，见 3.2）。
- 范围仅 orz（机械助理层）；模型面、Python conformance 不新增语义。

## 6. 实施路由（待用户放行）

- **S1 代码**：read_file 锚点字段（大文件信封补 size/mtime；小文件返回锚点）；
  写订单 `expected_anchor` 参数；写门禁核证 + 拒单错误码 + `console_order_
  rejected` 事件面接线；工具定义/schema 同步。
- **S2 测试**：锚点返回正确性；同 mtime 异内容由 sha256 兜底（构造 fixture）；
  锚点不匹配拒单；拒绝后零编辑副作用；错误信封与事件面断言；回归全绿。
- **S3 重建**：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约）。
- **S4 复验**：陈旧写入场景断言（修改后拒绝 → 重读重下成功）、命中率 ≥90%、
  零 400。

## 7. 关联登记

- 本设计定稿后：ADR-0010 §14.38（v1.38）、BACKLOG 0f、TODO P0-0f、
  CLI_PROJECT_INDEX canonical 条目登记。

## 8. S1 实施登记（2026-08-21）

2026-08-21 用户指示开始 S1；本窗口实施闭合（orz 工作树、未提交）：

- **read_file 锚点字段（orz-tools）**：新增 `ReadAnchor {size, mtime, sha256}`
  输出类型；`FileContent` 增可选 `read_anchor`（文本路径填充；PDF/PPTX/图片/
  二进制路径 None）；`ReadHandleEnvelope` 增可选 `mtime`（size/content_sha256
  已有）；小文件 prompt 附 `[read anchor] sha256=… size=… mtime=…` 尾行、
  信封头增 `mtime=…`；锚点与内容同一读取快照（读后 stat、size 不一致时重
  stat 一次）；grok_build full/concise 工具描述同步（expected_anchor 用法）。
- **写订单 expected_anchor（orz-loop）**：`workspace.search_replace` 契约
  schema 增可选 `expected_anchor`（size/sha256 必填、mtime 可空、sha256
  `^[0-9a-f]{64}$`）；动作描述同步。
- **写门禁机械核证（orz-loop controller）**：`issue_pending_console_order`
  发放前 pre_issue 门——search_replace 订单携带 expected_anchor 时，
  stat 快筛 size/mtime + 重算 sha256 权威比对；不匹配拒单（复用 order_stale
  信封形态：phase=pre_issue / step=protocol / code=content_anchor_mismatch）
  入 `console_order_rejected` 事件面、清槽并写失败 receipt，不执行任何编辑；
  目标文件不存在（新建路径）跳过；其余 stat/read I/O 错误 fail-closed 拒单
  （同 code，消息注明失败原因，如 "failed to stat/read target"）。错误消息
  指引重读后重下。审查收口补测试：锚点不匹配拒单零编辑、I/O 失败 fail-closed
  两条 `console_anchor_*` 用例（2026-08-21）。
- **验证**：`cargo check --tests`（orz-tools/orz-loop）通过；orz-tools
  read_file 202 / types::output 84 / orz-loop console 69 / orz-loop 全量
  544 通过（0 失败）+ 新增 console_anchor 2 条通过；fmt 干净；clippy 无新增
  告警（审查修复 build_read_anchor collapsible_if）；orz-tools 全量 44 个
  grep/glob 失败为本机 rg 环境性既有失败（stash 基线复现一致，与本改动无关）。
- **计数**：实施入账 27 → 28；S2 测试闭合（2026-08-21，见 §9，计数不变）；
  S3 重建 → S4 复验后 28 → 27。

## 9. S2 测试登记（2026-08-21）

2026-08-21 用户指示开始 S2；本窗口测试闭合（orz 工作树、未提交）：

- **新增 10 条用例**：
  - read_file 锚点返回正确性 3：小文件（sha256/size 与读取快照逐字节一致、
    mtime 与 metadata 一致、prompt 附 `[read anchor]` 尾行）、空文件
    （size=0、空串 sha256、`File is empty.` 后附尾行）、大文件信封
    （mtime + content_sha256、prompt 含 `mtime=`）；
  - PDF 路径无文本解码链：`raw_text_to_file_content` read_anchor 恒 None；
  - prompt 尾行渲染（有锚点时附尾行）与 `ReadAnchor` serde round-trip
    （mtime=None 省略字段，旧 reader 兼容）；
  - 写门禁四场景：锚点匹配放行（mtime=null 跳过快筛、sha256 权威）；
    同 size 同 mtime 异内容 sha256 兜底 fixture（`FileTimes::set_times`
    保留 mtime，快筛通过但哈希权威拒单）；陈旧拒绝→重读重下成功
    （S4 场景单元级预演）；expected_anchor 缺失保持既有行为；
  - 错误信封/事件面完整断言：receipt error step/code/message（含
    expected/actual sha256 与 re-read 指引）、upstream expected/actual、
    trace 末事件 protocol/content_anchor_mismatch、事件面机械盖章
    （phase=pre_issue / step=protocol / code / round / plan_epoch /
    run_id）、零编辑。
- **验证**：orz-tools read_file 207 / types::output 86 / orz-loop 全量
  550 通过（0 失败，3 ignored 为既有 live probe）；fmt 干净；clippy 无
  新增告警（orz-tools 0 告警；orz-loop 30 条全部位于既有代码位置，逐条
  核对无新增）；`cargo check --workspace` 通过（orz-host 1 条既有告警）。
- **计数**：仍 28（实施入账 27 → 28；S2 为测试闭合不动计数）；S3 重建 →
  S4 复验后 28 → 27。
