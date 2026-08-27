# HA 服务模型调研笔记（2026-08-28：Home Assistant 架构 → 半助理层目标形态）

> 状态：`调研笔记（v0.2，2026-08-28）`；初步映射已获用户确认并收口
> （§5 开放点已全部关闭），设计裁决见 THIN_HARNESS_REDESIGN_V2_DESIGN §4.5。
> 目的：回答 THIN-HARNESS-REDESIGN V2 §4.4 的五个问题，为半助理层加厚
> （失败自动诊断 / Windows 环境预适配 / 工具自解释）提供成熟参考。
> 结论去向：经用户确认后补入 THIN_HARNESS_REDESIGN_V2_DESIGN_2026-08-28.md
> §4.4（半助理层目标架构小节）。

## 1. 调研来源

- [Integration service actions（服务模型）](https://developers.home-assistant.io/docs/dev_101_services/)
- [Entities: integrating devices & services（实体架构）](https://developers.home-assistant.io/docs/architecture/devices-and-services/)
- [Entity（实体基类）](https://developers.home-assistant.io/docs/core/entity/)
- [HassIL（意图文法）](https://github.com/OHF-Voice/hassil)
- [service response 语义（SupportsResponse）](https://github.com/home-assistant/core/pull/115046)

许可证确认（2026-08-28）：**Home Assistant core = Apache-2.0**；
**hassil = Apache-2.0**（LICENSE.md 已核验）。本项目只借形态（服务注册表 /
target / response 语义 / 实体状态视图），不搬代码（HA 为 Python 生态，我们的
半助理层是 Rust）；若日后有少量代码级借鉴，Apache-2.0 允许修改/再分发，保留
许可与变更声明即可，与 ORZ 现有 LICENSE/NOTICE 结构可兼容。

## 2. HA 核心形态摘要

### 2.1 服务模型（domain.service）

- 服务按集成域注册：`hass.services.async_register(DOMAIN, SERVICE, handler, schema=..., supports_response=...)`；
  调用形态 = `service: domain.service` + `target`（可选）+ `data`（参数）。
- **schema**：voluptuous 声明参数（required / default / example / selector）；UI 由 selector 驱动，
  数据仍是扁平 JSON（分组只影响展示，不影响数据结构）。
- **target 语义**：按动作真正作用的对象定级——实体级（entity_id）/ 设备级（device_id）/
  配置项级（config_entry_id）；**target 不应可选、不应设默认**（否则脚本/自动化行为不可预测）。
- **response 三态**（`SupportsResponse`）：`NONE`（不可响应）/ `OPTIONAL`（可响应）/ `ONLY`（必须响应）；
  ONLY 服务不带 return_response 调用即报错，NONE 服务带则报错。
- **响应数据纪律**：响应必须是 JSON 可序列化 dict；**错误用异常抛出，响应数据内不得携带错误码**——
  用户不需要在脚本里做复杂错误处理。
- **services.yaml**：服务描述声明式存放（名称走翻译、字段走 selector），集成在 async_setup 注册，
  不在平台/配置项里注册。

### 2.2 实体模型（entity）

- 实体 = 标准化的数据点/可操作对象；设备集成负责建连，实体集成（light/switch/sensor…）负责定义
  抽象类与服务；**实体注册表 + 状态机是单一事实源**。
- 实体基类属性：`state` / `available` / `device_class` / `extra_state_attributes` /
  `supported_features` / `entity_category`（CONFIG / DIAGNOSTIC）等；属性只读内存，不做 I/O。
- 重要纪律：**适合表达为实体状态的数据（如温度）不要放进服务响应数据**；响应数据只用于
  不适合状态机的对象流/查询结果。
- 平台/注册表：Entity Component 分发配置、Entity Platform 管理轮询/更新、注册表登记 device/entity。

### 2.3 HassIL（意图文法）

- 语言：句子模板 + 备选词 + 可选词 + 槽位列表 + 展开规则 + 范围槽；YAML 声明式；用于 NLU
  （语音/文本 → intent + slots），如 `HassClimateGetTemperature`。
- 定位：**面向自然语言识别**，把口语/文本解析成结构化意图；本身不负责执行。

## 3. 五个问题的初步映射

### Q1 服务注册表粒度：工具/动作如何建模为 domain.service

直接沿用 `domain.service` 形态，与现有操作台注册表路由收敛：

- 域划分（初拟）：`file`（read/grep/search_replace/write）、`terminal`（run）、
  `retrieval`（web_search/web_fetch/browser_read）、`diagnostics`（diagnose/probe）、
  `delivery`（submit）、`blackboard`（read/write_section）。
- 每个服务 = 名称 + JSON Schema 参数契约 + 处理器 + `supports_response` 三态 + target 声明。
- 关键借用：**target 语义**（动作作用对象定级、不设默认）——例如写文件作用对象=文件实体
  （content_anchor 锚点），杀进程作用对象=进程实体；避免"目标可选"造成的不可预测。

### Q2 失败诊断在 HA 模型中的位置

- HA 的答案：诊断数据若持续存在 → 实体（DIAGNOSTIC 类别传感器）；一次性查询 → 服务响应数据。
- 我们的映射：`diagnostics.diagnose` 服务，由执行器在命令失败时自动调用，返回响应数据
  （极简诊断记录 §4.2）。持续环境状态（进程在跑/工具可用/路径存在）→ 可查询的实体状态，
  模型按需点读，不用反复探测。
- 差异处理：HA 主张"错误用异常、响应不携带错误码"；我们的 fail-closed 信封（step/code/message/
  trace_id）保留作为**失败通道**（对应 HA 的异常面），成功通道才是响应数据——两通道分开，互不污染。

### Q3 Windows 实体适配

- HA 借形态不借平台（HA 本身非 Windows 原生）。实体抽象恰好解决"模型要自己摸索 Windows"：
  - `process` 实体：state（running/exited）+ pid/exit_code/start_time + services（wait/kill/status）；
  - `file` 实体：exists/size/hash/encoding + 锚点（content_anchor 已有基础）；
  - `environment` 实体：shell 类型（PowerShell/cmd）、工具可用性、路径语义（盘符/UNC/分隔符）、
    编码栈（GAP-ENCODING-GATE 已有基础）；
  - 实体实现层消化 Windows 细节（Win32 进程语义、路径 canonical、编码、静态链接守卫），
    模型面只见稳定状态与稳定服务。

### Q4 hassil 文法是否引入

- 初步结论：**不引入**。模型面是结构化 JSON 订单，不是自然语言；文法层解决的是 NLU，
  我们无此需求。服务调用形态直接借 `domain.service + target + data` 结构化契约即可。
- 唯一可借鉴的点：hassil 的声明式模板思想 → 我们已有 JSON Schema，等价且更严。

### Q5 与 ACAF / 半助理层边界

- 服务注册表 = **接口层**（路由 + 契约 + 命令模式）；ACAF = **控制面**（票据/K_session/
  ledger/fail-closed），包裹在服务执行之外，不动。
- 收敛纪律（既有不变量 4）：服务处理器复用现有 host-owned 工具与执行路径，不建第二套审计链；
  注册表只做路由/校验/响应组装。

## 4. 对我们现有设计的对照

| 维度 | HA | 我们（现状/目标） |
|---|---|---|
| 调用形态 | `domain.service` + target + data | 操作台订单（动作名 + 参数）→ 可收敛为同构 |
| 参数契约 | voluptuous schema + selector | JSON Schema + 契约校验（已有） |
| 响应 | supports_response 三态 + JSON dict | 失败信封（fail-closed）+ 成功响应（待定型） |
| 状态 | 实体注册表 + 状态机 | 黑板分区 + journal（已有，可对齐"实体=稳定状态视图"） |
| 错误 | 异常抛出 | 结构化信封（保留，对应异常面） |
| 意图 | HassIL 文法 | 不需要（结构化订单） |

## 5. 开放点收口（2026-08-28 用户确认）

1. 域划分与实体清单：按 §3 初拟推进（file / terminal / retrieval / diagnostics /
   delivery / blackboard 六域；process / file / environment 三实体域起步）。
2. target 语义：**实体级**（用户裁决）——动作作用对象显式、必填、不设默认；
   无作用对象的动作省略 target（不引入设备/配置项两级，也不硬造全局实体）。
3. 诊断服务自动接线：按 §3 Q2 推进——执行失败 → `diagnostics.diagnose`
   （target=失败对象）自动派发，返回 V2 §4.2 极简记录。
4. 实体状态查询：**并入 blackboard**（用户裁决）——不新增只读工具；
   实体状态经 `blackboard_read` 按需点读，分区清单由工具结果自解释。
5. 黑板定义：**写进 blackboard_read 工具描述**（用户裁决）——简定义几个词，
   不进系统提示词；Windows 实体首波优先级（file / process / environment）
   随 R2 施工顺序定，不阻塞设计。
