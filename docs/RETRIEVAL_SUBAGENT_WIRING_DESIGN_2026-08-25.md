# 检索子代理接线重设计（2026-08-25：外部 A 档自动定档 + 内部子代理工具注册）

> 状态：`current-design`（2026-08-25 设计定稿；S1 待实施）。
> 性质：MECHANICAL-AUDIT-LAYER（2026-08-24）检索恢复的接线补强——修复
> 「S4 实机检索零调用」根因（harness 未传检索模式 + 内部子代理触发工具
> 未注册），并落实 2026-08-25 用户裁决。
> 关联：ADR-0010 §3.7.1/§3.7.2/§14.40 / BACKLOG 0h / TODO P0-0h /
> CLI_PROJECT_INDEX v2.21。

## 1. 来源与问题

### 1.1 S4 复验暴露：检索零调用，根因在模式门+触发面

- MECHANICAL-AUDIT-LAYER S4 复验（2026-08-24，sweep-mal-s4，5 题全过
  reward=1.00）中，主模型工具面 13 个工具，检索族（web_search /
  web_fetch / browser_read / retrieve_project_*）**一个都不可见**。
- 根因一：harness（`tb_agents/orz.py`）启动 orz 时不传 `--retrieval-mode`，
  会话停在默认 `off`——mode=off 投影剔除全部检索工具
  （controller.rs `is_retrieval_dispatch_name` + `is_retrieval_mode_gated_
  host_tool`），调用面拒绝 `retrieval_mode_off`。S1 的「主面恢复检索」
  只恢复了投影函数本身（非 off 模式下检索族可见），实机入口未接通。
- 根因二：内部子代理触发工具 `retrieve_project_docs` /
  `retrieve_project_source_ledger` **从未在 host/controller 声明面注册**
  （全仓仅 relay 路由与测试引用）；投影规则「registry 未声明不发明」，
  主模型永远看不到它们——内部子代理从启动器到触发面都是断的。

### 1.2 用户裁决（2026-08-25 多轮讨论定案）

1. **外部子代理 = 模式 A（自动定档）**：不命中（浏览器不可用）成本很低，
   采用机械 probe 定档——本地浏览器可用 → 工具面只有 browser_read；
   不可用 → 工具面只有 web 族（web_search/web_fetch），模型自然走原生检索；
   浏览器启动失败时机械降级 framework_fallback 并记 transition。
   不做「同面双工具 + 顺序门」（B 档被否：多余顺序门状态机）。
2. **内部子代理 = 重新定位 + 直接注册**：主代理已有 read_file/grep/
   search_tool 点读，内部子代理不再做「读文件」，改做**结构化检索外包**——
   跨多文件/目录调研、需要聚合结论的任务打包成检索任务派发；子代理多轮
   检索后只回 `[DOC]` 结构化结果 + source ledger，主对话不膨胀（ADR-0010
   §3.7 条 8：隔离上下文、保留原始来源、降低主对话污染）。
3. **触发方式 = 注册工具 + prompt tips**：直接注册 `retrieve_project_docs`
   到主模型声明面（relay 路由已存在），prompt 以框架使用提示（tips）告知
   使用方式——什么时候该用（多文件/跨目录调研）、与 read_file/grep 的区别
   （点读 vs 检索外包）；模型自主决定，不做硬门、不增加仪式。

## 2. 定案设计

### 2.1 外部子代理模式 A：probe 自动定档 + 显式降级

- 模式语义修改（ADR-0010 §3.7.1 登记）：`local_browser` probe 失败
  （`browser_launch_failed`）→ **机械降级到 `framework_fallback`** 并记
  `retrieval_mode_transition`（old=new 实值、authority=mechanical_probe、
  reason=`browser_launch_failed`）——取代「禁止因浏览器不可用自动切换」；
  页面级失败（LOGIN_REQUIRED/CAPTCHA/PAGE_BLOCKED 等 §3.7.2 显式状态）
  **不降级**，仍按显式失败返回。
- 工具面定档：降级后工具面只有 web 族（web_search/web_fetch）；
  browser_ready=true 时工具面只有 browser_read（web 族隐藏）。
- TB harness：`tb_agents/orz.py` 在 `allow_network`（PUBLIC）时传
  `--retrieval-mode local_browser`——TB 容器无浏览器后端 → probe 失败 →
  降级 framework_fallback → 主面/子代理自动只见 web 族。不命中成本低。
- 保持：候选计数、web_search 并发=1、ACAF 前置、显式拒绝形态不变。

### 2.2 内部子代理触发面注册

- 在 controller 声明面构造 `retrieve_project_docs` ToolDef（类比
  `retrieval_disposition` 的声明方式；参数 `query` 必填 + 可选 `scope` /
  `max_results`），随主模型工具投影声明（mode≠off 时可见）。
- relay 路由已存在（`retrieve_project_*` → InternalRetrieval），声明即触发。
- 子代理工具面：内部 lane 只见读族（read_file/list_dir/grep/search_tool/
  project_doc_index）+ 检索合同；web 族与 browser_read 不进入内部 lane
  （内部检索对象=工作区/项目文档，不是外部网络）。
- 结果契约沿用 `[DOC]`/`[SOURCE]` + 结构化 ledger；机械 assessment +
  parent disposition 生命周期不变。

### 2.3 prompt 使用提示（tips）

- BASE_SYSTEM_PROMPT 执行面段补一句内部检索提示（轻量、无仪式）：
  多文件/跨目录调研用 `retrieve_project_docs`（检索外包、隔离上下文、
  返回结构化结果）；单文件点读用 read_file/grep/search_tool。
- 不做硬门、不做「必须先用检索」引导；模型自主决定（用户裁决：
  prompt 已精简，tips 即可）。

## 3. 边界与残留

- A 档自动降级仅覆盖「浏览器启动失败」这一机械条件；页面级失败不降级
  （§3.7.2 显式状态不变）。
- 内部子代理在 TB 场景可能仍低频（单容器任务工作区小），但触发面接通后
  通用场景（大项目、多文件调研）可发挥作用；TB 构造题验证检索调用出现。
- `retrieve_project_source_ledger` 暂不单独注册（内部 lane 结果 ledger
  由结构化结果机械构建）；如需独立查询面后续补。
- 既有子代理状态机、并发=2、预算、写域 deny-only 约束不变。

## 4. 实施路由与计数

- S1 代码（合并：A 档降级 + 内部工具注册 + prompt tips + harness 传参）
  → S2 测试（降级 transition、内部派发、投影可见性、tips 零仪式断言）
  → S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）→ S4 复验（构造题：
  检索调用出现、零 400、命中率 ≥90%、reward 不降）。
- 计数：设计轮不动计数；实施放行入账 +1；S4 闭环 -1。
- S1 代码（2026-08-25 完成）+ S2 测试（2026-08-25 完成）：①A 档降级——
  `apply_mode_a_auto_degrade`（acp_server：local_browser probe 失败 →
  framework_fallback + bootstrap_transition_pending）+ controller
  `with_retrieval_mode` 增可选 transition 元数据（降级时 authority=
  mechanical_probe、reason_code=browser_launch_failed 落盘）；②主面声明
  `retrieve_project_docs` ToolDef（controller 声明面，relay 路由已存在，
  mode=off 时被检索族投影剔除）；③`subagent_tool_projection` 加 role
  参数——内部 lane 剔除 web 族/browser_read/retrieve_project_*（仅读族），
  外部 lane 维持 web 族 + browser_read（可用时）；④prompt 执行面段补
  ≤1 句内部检索 tips；⑤TB harness（orz.py）PUBLIC 时传
  `--retrieval-mode local_browser`。验证：orz-loop 556 / 0 / 3、orz-host
  222 / 0 / 4（串行）、orz-tui 178 / 0、orz-assurance 144 / 0、
  orz-bin 11 / 0（+ acaf_e2e 23 / signer 14 / provision 2 /
  stdio_e2e 1 / real_flag 2）、fmt 干净、clippy 无新增告警。新增专项测试
  5 项：模式 A 降级规则（browser_launch_failed 降级 / 可用不降级 /
  非启动原因不降级 / framework_fallback 不降级）、降级 transition 元数据
  透传（authority/reason_code/old/new）、主面 retrieve_project_docs
  声明（mode≠off 可见 / off 隐藏）、主车道 retrieve_project_docs 派发
  内部子代理（ToolStarted target=internal_retrieval、内部 lane 零
  web/browser 调用）、内部 lane 投影仅读族（web/browser/retrieve 全剔除）。
  登记于 ADR-0010 §14.40 / BACKLOG 0h / TODO P0-0h。

## 5. 验收标准（DoD）

1. mode=local_browser 且浏览器不可用 → probe 机械降级 framework_fallback，
   `retrieval_mode_transition` 带 old/new/authority/reason 落盘。
2. 降级后主面/外部子代理工具面只有 web 族；browser_ready=true 时只有
   browser_read（web 族隐藏）。
3. `retrieve_project_docs` 在主模型声明面可见（mode≠off），调用 relay
   派发内部子代理，`[DOC]` 结构化结果返回。
4. 内部 lane 工具面无 web 族/browser_read；既有候选计数/并发=1/ACAF 前置
   不变。
5. prompt tips 轻量（≤1 句）、无硬门/无仪式；既有 plan 门/审计/引用零残留
   测试保持通过。
6. TB harness 传 `--retrieval-mode local_browser`（PUBLIC 时）；
   构造题实机出现检索调用。
7. orz-loop 全量测试通过、fmt/clippy 干净、Linux musl 三件套重建。

## 6. 关联登记

- ADR-0010 §14.40（v1.40）/ BACKLOG 0h / TODO P0-0h /
  CLI_PROJECT_INDEX v2.21。
- 修改：ADR-0010 §3.7.1（local_browser probe 失败显式降级取代禁止切换）。
