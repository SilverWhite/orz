# CLI_PROJECT_INDEX

> 索引版本：v2.0；状态：`current`；最近整理：2026-08-18（承接 2026-08-13 批次：CLASSICAL-EXEC-ASSISTANT 升 P0、小样 1 跑通与槽位表动态生成、v0.3 集成形态与 DeepSeek Harness 借鉴、fail-closed 返回契约、动作粒度裁决、v0.4 执行日志可见性、v0.5 黑板动作栏登记；工具探针 v0.2 A+C→B 定档、P0-A 批次与 P0-A-2 单一探针面实施闭合、ADR-0010 v1.8 登记；OPS-PROTOCOL 审查判定登记；P0-B B-1 闭合——web_search citations 结构化透传进 loop；2026-08-14 审查复核——投影入口与 README 冻结版本表述对齐 v1.8 补写、CLASSICAL-EXEC-ASSISTANT 与 FUS-TOOL-PROBE 状态头对齐、ADR-0010 §13 来源补齐、新增 AUTH-TODO 待办勾选清单路由；P0-B 步骤 2 闭合——web_fetch 候选机械计数门禁与计数反馈（ORZ_WEB_FETCH_CANDIDATE_CAP 定档 8）；2026-08-14 缓存与上下文成本收敛登记（ADR-0010 v1.9、FUS-REQUEST-CACHE pending）——保持 v1.8 探针可见性、请求 header 变化留痕、探针准确性优先、单轮注入预算 + 策略化读取；2026-08-14 压缩机制重设计定稿登记（ADR-0010 v1.10、FUS-COMPACTION-REDESIGN pending）——384K 有效窗口、160K/200K 触发、工具记录机械坍缩、五段模板摘要、恢复缺口 D2-2/D3-1 实施前置；P0-B 步骤 4 闭合——browser_read 范围/模式参数（full/preview/keywords）与第二段计数域复用；P0-B 步骤 5/6 闭合——输出级引用校验器与提示词相应缩短（ADR-0010 v1.11/v1.12，FUS-RETRIEVAL-MECH 转 implemented）；2026-08-14 步骤 6 复核补记——GAP-SOURCE-WEIGHTING-IMPL 索引条目补取代注记（P3 已处理），审查观察登记（提示词 observed-scope 枚举，P3 可选，见步骤 6 审计 §7）；2026-08-14 DSH 借鉴复核与两项设计确认登记——FUS-DSH-BORROW-REVIEW（A 挂起/B 收编小样 2/C 收编 S2）、FUS-ORIENTATION-FORCED-TEMPLATE 与 FUS-SESSION-CONTEXT-MONITOR（ADR-0010 v1.13、P1 6c/6d）、条件触发 FUS-RECOVERY-TOOL-OUTCOME/ORZ-STAGNATION-TOOL-SIGNAL；2026-08-14 P0-D 压缩机制实施闭合（用户放行）——S1 D2-2/D3-1 恢复前置、S2 动作台账机械坍缩、S3 五段模板摘要接线（context_compressed v0.2 payload + 存档 + retention + TUI）、S4 审计与同步，FUS-COMPACTION-REDESIGN 转 implemented；2026-08-14 黑板 plan epoch 轮换登记（用户裁决）——ADR-0010 v1.15、FUS-BLACKBOARD-PLAN-EPOCH pending、压缩不再滚动黑板；2026-08-14 黑板 plan epoch 实施闭合（用户指示优先）——S1-S5 全部实施（plan_epoch 批准事件/Schema/fixtures、原子轮换与 `.gsa/blackboard/epoch-<n>.json` 归档、压缩解耦 + marker/路径槽、blackboard_read epoch 参数、恢复装载 + retention），FUS-BLACKBOARD-PLAN-EPOCH 转 implemented，ADR-0010 §3.6 正文随实施登记；2026-08-17 console 面缓存注记登记（用户指示补记）见下。
> 2026-08-18 压缩机械模式定案与实施闭合登记（用户裁决 B 定案、D1=(b)）——S4
> 复验账单对账定位「未处理的部分」：压缩摘要调用以独立系统提示词重付整段视图
> miss（22:17 运行 10 个账单请求无 journal 对应、额外 miss ≈ 676K、账单口径
> 命中率 89.71% <90%，事件口径 93.26% 虚高；摘要调用两轮全部失败零产出）。
> 定案=纯机械压缩（ADR-0010 §14.29 v1.29）：`run_template_compact` 移除模型
> 摘要调用（agent/cancel/heartbeat 退役、`CompactDecision::Executed` 简化），
> 五段槽位=黑板 + 固定机械占位（注意事项/后续衔接，阶段 (c) HA 结构化事实
> 聚合落地前）；存档恒写入、marker 恒带 digest、无 `summary_incomplete`
> 终止态；事件 `mode=mechanical`（schema enum 保留 template_summary 回放）；
> fallback 紧急机械截断保留。实现=orz 子模块提交（2026-08-18）；orz-loop 465
> 通过、fmt 干净、clippy 与基线一致（lib 21 / test 26）、Python verifier 214
> 通过；CONTEXT_COMPACTION_DESIGN §4 修订、BACKLOG 0c / TODO P0-0c 同步；
> 计数不变（0c S3/S4 复验闭环后 29 → 28；阶段 (c) 待重看 HA 项目后另行裁决）。
> 2026-08-19 D1=(c) HA 结构化事实聚合设计定稿登记（用户裁决：先设计、不直接
> 动作；纯文档登记、未实施）——重看 HA（Home Assistant）上游实现与源码后定稿：
> 注意事项槽=HA 结构化事实聚合（助理层唯一新增输出；数据源=controller 已机械
> 写入的 `plan.steps` Failed/Blocked + `exec.errors` 最近 5 + `actions.results`
> 失败 receipt 最近 3；排序=计划面→执行错误→动作失败；空时「（无注意事项）」；
> ≤3K 超限截断+指针；压缩内部失败继续走 marker 标注）；后续衔接槽不交助理层
> （固定中性占位 + 回查入口，由主模型自行判断——避免限制或机械性误导）；零模型
> 调用、五槽 17K 上限、存档恒写入、marker 恒带 digest、schema 不变。ADR-0010
> §14.30（v1.30）/ 压缩设计 §4.4 / BACKLOG 0c / TODO P0-0c；实施路由=S1 代码
> → S2 测试 → S3 重建 → S4 命中复验；未闭合计数不变（29，0c 验证闭环后
> 29 → 28）。
> 2026-08-19 压缩机械模式全面审查处理登记（B 定案收口）——fallback 双段
> 截断轮数口径修复（事件/存档/marker「被压轮次」= 常规 drain + 紧急截断
> 之和 + 事件估计 marker 插入后重算）、`ledger_fold_write_failed` 事件契约
> 补齐（payload schema / fixtures / verifier 交叉校验 / 生成器，conformance
> 52→53 闭合 2b755d6 遗留红）、`orz_source_manifest.sha256` 重生成、生成器
> `context_compressed` 模板对齐 mechanical（防重生成回退）、文档/注释/schema
> 描述清理；orz-loop 466 / Python verifier+conformance 230 / 仓库门禁
> valid；计数不变（0c S3/S4 复验闭环后 29 → 28）。
> 2026-08-19 D1=(c) HA 结构化事实聚合 S1/S2 实施闭合登记（用户放行实施）——
> S1 代码：summary.rs 新增 render_facts_notes（HA 结构化事实聚合：plan
> 失败/受阻步骤 + exec 错误最近 5 条（每条截断约 200 字符）+ 动作失败 receipt
> 最近 3 条；排序=计划面→执行错误→动作失败；空时「（无注意事项）」；≤3K 超限
> 截断 + 「其余 N 条见 blackboard_read 分区/摘要存档」指针）与
> render_notes_capped/truncate_chars/failure_envelope_fields 辅助；
> 后续衔接占位改中性措辞（「由主模型自行判断」+ 回查入口含外挂台账）；
> run_template_compact notes 槽接线 render_facts_notes；S2 测试：summary
> 事实聚合单测 6 项 + 压缩 e2e 1 项（marker+存档三源事实槽同序）+ 空黑板 e2e
> 断言「（无注意事项）」；orz-loop 473 / 0 失败、fmt 干净、clippy 与基线一致
> （lib 21 / test 26）；计数不变（0c S3/S4 复验闭环后 29 → 28）。
> 2026-08-19 D1=(c) S1 全面审查处理登记（审查结论：实现无代码缺陷、设计与
> 实现符合）——①登记口径更正：summary 事实聚合单测实为 6 项、orz-loop
> 473 通过（原 5 项 / 472 更正）；②设计补充登记：O1 同一失败事件可同时
> 以步骤行+动作失败行双视角呈现、属有意冗余；O2 单条超长整行退化为仅指针、
> N 计 1，由 blackboard_read 回查恢复；O3 exec.errors 黑板侧无界为已知
> 边界，控制器侧加保留上限属可选后续、不占计数（登记于压缩设计
> §4.4.1/§4.4.3 与 ADR §14.30）；③索引条目内杂散控制字符清理
> （render_facts_notes 等标识符恢复）；计数不变（0c S3/S4 复验闭环后
> 29 → 28）。
> 2026-08-19 命中率归因与黑板读取缓存成本设计定稿登记（用户裁决：大机制不再
> 更改、只补回应命中而未命中的部分）——S4 换题复验（path-tracing 正式 1800s
> 预算、40 请求）provider 口径 85.43% / journal 86.04%；归因=8/8 大 miss
> 尖峰（合计约 190K = 64%）紧跟 blackboard_read（actions/exec），单次分区
> 结果 17–32K token 作为全新工具结果注入、前缀缓存无法命中——非折叠频率、
> 非大机制。定案=黑板分区渲染瘦身（actions 结果板去 response JSON、exec 行
> 截断 200 字符 + 段总长 4K；registration/order 板不变；阶段 2 可选 since
> 扩展 actions + 读取频率引导）；零模型调用、工具契约/schema 不变、折叠
> 压缩阈值不动；预期命中率 86%→95%。登记于 ADR-0010 §14.31 /
> BLACKBOARD_READ_CACHE_COST_DESIGN / BACKLOG 0c / TODO P0-0c；实施路由
> S1 代码→S2 测试→S3 重建→S4 复验（0c S4 前置，计数不变 29）。
> 2026-08-19 黑板读取缓存成本 S1/S2 实施完成 + 全面审查 + F1 处理=方案 B
> 定稿登记（用户裁决：方案 B 按需点读、整段恒定优先）——S1 渲染瘦身已实施
> （actions 结果板固定形态行去 response JSON、exec 行截断 200 + 段 4K；
> orz-loop 479 通过、fmt/clippy 基线一致）；审查发现 F1=console 面动作详情
> 不可回查（assistant.trace 不在直接工具面、receipt 响应同样被瘦身隐藏，
> 设计「经 trace 回查」假设机械上不成立）；定稿=`blackboard_read` 新增可选
> receipt_id 点读（单条完整内容、8K 上限截断+指针、epoch 归档点读、非法/
> 未找到显式报错、无 receipt_id 整段逐字节恒定）；零模型、黑板数据面/事件面
> 不动、工具定义增量扩展；路由 S1 代码→S2 测试→S3 重建→S4 复验；计数不变
> （29）。登记于 ADR-0010 §14.31 第 2 项 / BLACKBOARD_READ_CACHE_COST_DESIGN
> §4.5 / BACKLOG 0c / TODO P0-0c。
> 2026-08-19 黑板读取缓存成本方案 B S1 代码 + S2 测试实施闭合登记（用户放行）
> ——epoch.rs `render_section` 增可选 `receipt_id`（非 actions 分区携带=显式
> 报错；actions 分支点读优先）+ `render_receipt_point_read`（固定形态行 +
> 成功 `response=<JSON 原文>` / 失败 `error=<JSON 原文>`，`RECEIPT_DETAIL_
> MAX_CHARS=8_000` 超限截断 + 「…」+ 存档/TraceStore 指针行；未找到显式
> not found + 旧 epoch 归档提示）；controller.rs 参数解析（非字符串/空串=
> 显式报错、exit_code 1）与透传（live 与 epoch 归档共用）、blackboard_read
> 工具定义参数/描述增量扩展（事件面不变）；S2 单测 5 项（完整 response/error、
> 8K 截断+指针、未找到、非 actions 报错、无 receipt_id 与 S1 逐字节相等）+
> 工具级 4 项（点读回达、非法参数报错、非 actions 报错、跨 epoch 点读）；
> orz-loop 488 通过 / fmt 干净 / clippy 与基线一致（lib 21 / test 26）；
> S3 重建 → S4 复验（≥90%、无 400）待续；计数不变（仍在 29）。登记于
> ADR-0010 §14.31 第 2 项 / BLACKBOARD_READ_CACHE_COST_DESIGN §4.5 /
> BACKLOG 0c / TODO P0-0c。
> 2026-08-19 方案B 全面审查处理登记（用户指示处理审查全部问题；orz 提交 +
> 父仓库指针、未推送）——N1 点读截断尾部记账注释修正（「…」已计入
> truncate_chars 输出）；N2 receipt_id trim 规范化口径登记（前后空白忽略、
> trim 后为空同空串报错）；N3 `blackboard_read` section 非字符串显式报错
> （同非法 epoch/receipt_id 纪律、绝不静默回退 "plan"，消除与 receipt_id
> 组合时的误导性报错；新增工具级测试，orz-loop 489 通过 / fmt 干净 / 无
> 新增 clippy 告警）；N4 提交状态措辞统一为「已提交、未推送」（S1 提交
> 8dbaa01 / 方案 B 提交 ad74714）；O1 点读次数无机械上限登记为已接受边界
> （频率引导属阶段 2 §4.4 可选后续、不占计数）；O2 当前 epoch 超 8K live
> receipt 存档指针轮转前不成立、TraceStore 不在 console 直接工具面，登记
> 为已接受边界（8K 用户定档）；O3 「JSON 原文」措辞收敛为「重序列化完整
> 内容」（键/值/嵌套完整、非字节级原文）。登记于 ADR-0010 §14.31 第 2 项 /
> BLACKBOARD_READ_CACHE_COST_DESIGN §4.5 / BACKLOG 0c / TODO P0-0c；
> S3 重建 → S4 复验（≥90%、无 400）待续；计数不变（仍在 29）。
> 2026-08-19 S3/S4 复验执行 + 折叠桥接截断设计定稿登记（用户裁决：形态甲 +
> 桥 8K 真实 token + 触发 128K + 思维链不进桥/不进审计；先设计、不动作）——
> 重建成功（orz-linux 新三件套，04:10）；S4 单题复验（path-tracing 1800s，
> 04:13 运行）provider 口径命中率 88.84%（对照上次窗口 84.07%），仍低于
> 90%——归因=3 次折叠重付 47.8K/50.4K/56.4K（66.5% miss）：折叠重付 ≈
> 保留尾大小 + 新内容，本次保留尾实测 44–56K 真实 token；折叠间请求为纯
> 追加（miss 137–3.4K）。定案=折叠后其余进外挂台账、视图只留最新桥（默认
> 8K 真实 token，`ORZ_FOLD_TAIL_TOKENS` 可配，字符预算近似 + S4 实测校准；
> 取值依据=本次读/计划轮 1–3K、终端轮 5–7K，8K 能整轮装下、截断成例外）；
> 形态甲=先定裁剪再对桥内容级截断（最新完整轮结构全保留、超预算只截内容+
> 指针、保留尾部优先）；`reasoning_content` 剔除（思维链具幻觉性、审计仅
> 保留计数）；`fold_tail_rounds` 退役；指针文案更新一次；外挂台账/压缩/
> 白名单/400 防线不变。推算命中率 → 约 94%（零折叠上限 95.9%）。登记于
> ADR-0010 §14.32 / LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN_2026-08-19 /
> BACKLOG 0c / TODO P0-0c；实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验
> （≥90%、无 400、每窗重付 ≤~15K、截断频率 ≤30% 校准）；0c S4 前置，计数
> 不变（仍在 29）。
> 2026-08-19 折叠桥接截断 S1/S2 实施闭合 + 全面审查处理登记（用户放行
> 实施；orz 提交 52d698c、未推送）——S1 代码：`bridge_cut` 桥预算裁剪
> （完整轮累加至预算、最新轮恒入桥、不完整轮回退；4 字符/token ÷ 2 换算，
> 默认 8K → 16K 估计口径）、`build_request_view` 桥视图（reasoning 剔除、
> 超预算内容级截断=工具回复保留尾部 + 「…（前略）」+ sha256 指针、最终
> 回复 199+…、声明 content/tool_calls 完整、消息不删轮不拆；`bridge_end`
> 冻结=推进时消息末尾、折叠之间纯追加）、`advance_fold` 入账范围不变、
> 指针文案更新（「约 8K 桥接内容，更早轮次已按行归档于 <abs-path>」）；
> `controller.rs` `fold_tail_rounds` 退役 → `fold_tail_tokens`
> （`ORZ_FOLD_TAIL_TOKENS` 可配）；`agent_loop.rs` 推进与请求视图两处
> 透传。S2 测试：orz-loop 503 通过（+13 桥测试 +1 越界防御）/ fmt 干净 /
> clippy 与基线一致（lib 21 / test 26）。审查处理（实现无功能缺陷）：
> N1 指针文案按设计 §3.4 定稿原文落地；N2 实现决策登记（新增 `bridge_cut`、
> `collapsed_cut` 保留服务压缩）；N3 `bridge_end` 冻结机制补记；O1 中间
> assistant 文本口径；O2 索引越界防御守卫；O3 非默认配置文案固定不变。
> 设计文档 §3.1–§3.4/§7 修订。S3 重建 → S4 复验（≥90%、无 400、每窗
> 折叠重付 ≤ ~15K、截断频率 ≤30%）待续；计数不变（仍在 29）。登记于
> ADR-0010 §14.32 第 2 项 / LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN_2026-08-19 /
> BACKLOG 0c / TODO P0-0c。
> 2026-08-19 折叠桥接截断 S3/S4 复验执行 + 换算系数校准登记（用户指示
> 重建 + 单题复验；orz 校准调整待提交）——S3 重建成功（orz-linux 07:07
> 新二进制；USTC/清华镜像源 502 不可达，改用阿里云镜像源）。S4 单题复验
> （path-tracing 1800s，07:08 运行，job 2026-08-19__07-08-57）：reward
> 0.0（wallclock 耗尽）、无 400、113 请求；**journal 口径命中率 95.54%**
> （hit 4,775,422 / miss 222,869，对照上次 88.57%）；折叠 2 次（37/82
> 轮），**折叠后首请求重付 3,742 / 6,493 真实 token**（DoD ≤ ~15K，
> 对照上次 47.8K/50.4K/56.4K）、**截断频率 0%**（≤30% 达标）。校准：
> 第二次折叠桥 12,948 字符 → 重付 6,493 → 实测 ≈ **2 字符/真实 token**，
> `FOLD_TAIL_CHARS_PER_TOKEN` 4 → 2（桥回到 8K 真实 token 目标）；
> orz-loop 503 通过 / fmt 干净 / clippy 基线一致。provider 口径待账单
> CSV（07:00–08:00 时段）对拍；计数不变（29，provider 对拍确认后
> 29 → 28）。登记于 ADR-0010 §14.32 第 3 项 /
> LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN_2026-08-19 / BACKLOG 0c / TODO P0-0c。
> 2026-08-19 折叠桥接截断 S4 provider 对拍确认 + 0c 验证闭环登记（用户
> 上传新账单）——provider 口径命中率 **95.33%**（hit 4,896,384 /
> miss 239,752 / 115 请求，≥90% DoD 达标）；与 journal 口径（95.54%，
> 113 请求）差 2 请求，费用 1.9082 元验算吻合。S4 四项判定全达标
> （命中率、无 400、折叠重付 3,742/6,493、截断频率 0%）——**0c 验证
> 闭环，未闭合计数 29 → 28**；换算系数校准（4→2）待提交。登记于
> ADR-0010 §14.32 第 4 项 / LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN
> 2026-08-19 / BACKLOG 0c / TODO P0-0c。
> 2026-08-19 输出退化防护与工具结果可再读闭环设计定稿登记（用户裁决：
> 8K 全统一限值 + 桥 8K 不动 + 补读闭环硬约束 + 前两层补强；先设计、
> 不动作）——make-doom-for-mips 两次失败归因：模型退化复读（201K 字符、
> 2,316 次省略标注重复；历史先例 2026-08-11 479K 字符同任务）+ 160K
> max_tokens 放大 + 工具结果截断后无可再读闭环；排除网络（探针 60/60）；
> 停滞守卫仅事后评估未拦截。定案=①限值统一 8K（终端 20K→8K、点读 8K
> 确认、桥 8K 不动）；②补读闭环硬约束（截断末尾"完整内容见 \<路径\>，
> 请使用 read_file"；落盘 `.gsa/session/terminal/*.log` 可读性已验证；
> 点读指针改向、桥截断保留工具结果自身指针）；③生成期实时复读检测
> （on_chunk 连续块/重复率，治本）；④`REQUEST_MAX_TOKENS` 160K→32K
> （止损）。准确度判定=8K 截断信息守恒、闭环吸收差异、桥 8K 不扩窗。
> 登记于 ADR-0010 §14.33 / OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19 /
> BACKLOG 0d / TODO P0-0d；实施路由 S1 代码 → S2 测试 → S3 重建 → S4
> 复验；设计轮不动计数（28）。
> 2026-08-19 输出退化防护 S1/S2 实施闭合 + 全面审查处理登记（用户放行
> 实施；orz 提交 5e968ec + 审查处理 19b839f、未推送）——S1 代码：on_chunk 生成期实时退化
> 检测（连续相同 delta N=5 / 累计 ≥1K token 且最近 1K token 3-gram
> 重复率 >60%）、`StreamInterrupted{degeneration_detected}` 主动中断且
> 不重试、会话级连续计数 `DEGENERATION_LIMIT=3` 达限转
> `run_invalidated{status: degeneration}`（schema 先行）、
> `REQUEST_MAX_TOKENS`/`ModelConfig::max_tokens` 160K→32K、终端工具输出
> 20K→8K + 统一 read_file 补读指针三面（default/concise/chat-completion）
> + 指针块计入 8K 截断预算、点读指针改向落盘文件、桥截断保留工具结果
> 自身指针（sha256 兜底）、失败轮次 stagnation 审计评估。S2 测试：
> orz-loop 510 通过 / orz-tools 2763 通过（沙箱外，grep/glob 44 项为沙箱
> 拦截 rg 所致非回归）/ fmt 干净 / clippy 基线一致（lib 21）/ 仓库门禁
> valid。全面审查处理：P1=点读终端判定改按发放时订单动作名
> （`ActionResult.action`）——响应信封 `{"output": string}` 被
> read_file/grep/run_tests 等 text-output 动作共用，原按信封判定会给
> 非终端 receipt 死指针（违反「指针路径必须真实可读」），修复 + 非终端
> text-output 回归测试；P3=160K 陈旧注释清理（live 探针改 32K）、.gsa
> 符号链接可读性专属测试、登记同步；解释登记=「同一 run 连续 3 次」字面
> 不可达，实现为会话级连续计数（成功请求重置）。计数纪律：实施放行入账
> （28→29），S3/S4 验证闭环后 29→28。S3 重建 → S4 复验（无退化中断、
> 无 400、命中率 ≥90%、补读路径可用）待续。登记于 ADR-0010 §14.33
> 第 2 项 / OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19 §9 / BACKLOG 0d /
> TODO P0-0d。
> 2026-08-20 输出退化防护 S3 重建 + S4 复验闭环 + S4 缺口修复登记（用户
> 放行；orz 提交 f2cb1e0，已推送）——S3 Linux musl 重建（ORZ-BUILD-MOUNT-001
> 契约，两轮：build-20260819.log 11m04s / build-20260819b.log 3m56s，
> 三件套时间戳更新）；S4 make-doom-for-mips 单题复验（三轮运行 + 端到端
> 探针）：无退化复读中断 / 无 hang（agent 全程活跃至 900s 任务预算耗尽）、
> 零 400、journal 口径命中率 94.25%（124 请求）与 91.91%（73 请求）均
> ≥90%、补读路径真实可用。**S4 发现并修复缺口**：终端截断收据指向
> `.gsa/session/terminal/<order>.log` 并指示 read_file，但权限层
> `access_in_scope` 按「.gsa 树 agent-invisible + canonical 限会话 cwd」
> 拒绝全部 .gsa 读取（第二轮回执尝试补读被 policy_denied；对台账
> current.md 的 grep 同被拒）——修复=白名单会话 .gsa 卷内
> `session/terminal/*.log` 的 read_file/grep（对齐 run_tests_output.txt
> 先例；lexical 限目录 + canonical 限会话卷防符号链接外逃；run_tests
> 白名单补 symlink-aware 比较），orz-host 单测 221 通过（并发下 1 条
> 既有时序偶发单跑复过）/ fmt 干净 / clippy 无新增；端到端探针确认模型
> 按截断指针 read_file 三次成功（首次 1–1000 行、offset/limit 分页续读
> 1001–1200 行、操作台订单复核 line-1190–1200，1200 行完整取回）。
> 计数：S3/S4 验证闭环 29 → 28。登记于 ADR-0010 §14.33 第 3 项 /
> OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19 §6/§9 / BACKLOG 0d /
> TODO P0-0d。
> 2026-08-20 换题复验（gpt2-codegolf）+ 流式重试节奏设计定稿登记（用户
> 裁决：重试间隔缩短——idle 无数据判定 5s 一轮、10 次上限（总 50s
> 窗口），zero-chunk 重试窗口同步 50s；先设计、不动作）——P0-0d 闭环
> 后按用户指示再挑一道不同历史错题验证稳定性：gpt2-codegolf（历史
> 7 次失败、build/run 型需读约 500MB GPT-2 权重并写 <5000 字节 C 程序）
> 三轮运行（00-22-19 / 00-24-29 / 00-41-54）——第一轮首请求 20s idle
> 警告 → 90s 内连接中断 → zero-chunk 指数退避重试 2 次（478ms/721ms）
> 后 **32s 窗口耗尽**（瞬时连接错误，`NonZeroAgentExitCodeError`）；
> 第二/三轮首轮 request→model_output 约 10.5/10.6 分钟（AgentTimeoutError
> 正常收尾）、有效模型工作时间约 3 分钟、8 请求、journal 口径命中率
> 85.19%（样本不足非机制退化；三轮均零 400、无退化中断、ACAF 票据
> 全过）。**环境排查结论=容器/网络/API 均正常**：容器 DNS/TLS/TTFB
> 0.38s、宿主机流式 TTFB 0.19s 且带 tools 大请求 60–80s 完整流完
> （reasoning 持续流动非死线）、API 探测稳定；对照昨天 make-doom S4
> 首轮 8.3s——差异在 DeepSeek 端首轮生成慢 + 一次瞬时连接错误，非
> 机制退化。**定案**：`stream_idle_warn` 20s→5s、`stream_idle_timeout`
> 90s→50s（=5s×10 轮）、`request_retry_window` 32s→50s（zero-chunk
> 重试窗口同步；非流式 create 退避窗口同步放宽）、`request_max_retries`
> 10 次不变；idle 只看完全无数据（慢速 reasoning 流不误杀）、重试仍
> 指数退避（不改为固定 5s 间隔）、退化中断不重试纪律不变、无新增配置
> 旋钮、retry 参数参与请求头指纹（部署后首次请求一次性指纹变化，既有
> 纪律）。实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（≥90%、无
> 400、无退化中断、首轮不再 10 分钟级长等）；设计轮不动计数（28）。
> 登记于 ADR-0010 §14.34（v1.34）/ [设计](docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 官方 harness 对照 + 输出预算恢复与空流止损设计定稿登记（用户
> 方向：评估 256K+max、空流处理官方化、退化检测器大升级；先设计、不动作）
> ——官方 deepseek-harness（llm-deepseek/llm-retry，2026-08 master）源码
> 对照：默认 `reasoning_effort=high`、`maxTokens=256_000`、idle 5 分钟、
> EMPTY_RESPONSE 空即错即退（normal 5 次、退避 500ms→10s+10% jitter、重试
> 在 durable step 边界、无生成期退化防护）。今晚 7 次运行对照 + pcap + 账单
> 对账定论：空流根因链=max 档思考 + 32K max_tokens 截断 reasoning → 完成型
> 空响应 → D-6 原样重试放大（10 分钟级）；退化检测器只喂 content delta、空流
> 场景全程沉默。**定案**：①`REQUEST_MAX_TOKENS` 32K→256K（回落档 128K、
> S4 实测校准）；②D-6 空流链官方化收窄——完成型空响应快速有界重试 ≤2 次
> （500ms→10s+10% jitter）→ thinking 禁用降级；reasoning 族异常不原样、
> 直接降级；③退化检测器升级为输出健康哨兵——观测面扩到 content+reasoning+
> tool arguments，新增 reasoning 复读（灵敏层，循环特征即触发）与
> reasoning-stall（首 chunk 起 600s 无 content/tool_calls、或 reasoning
> 估算 ≥64K tokens，OR 触发——空转预算与 max_tokens 解耦；系数 2 字符/
> token）；**二轮修订（实测校准）**：合法难题首轮 17,757 reasoning/184s
> 正常产出（RUN-CLI-6a85f668）——原定 120s/16K 会误杀合法轮；**三轮修订
> （用户裁决：兜底兼容 max 思考、灵敏层负责快速）**：成本账（实测
> ¥4.592/M output，32K≈¥0.147/64K≈¥0.294/256K≈¥1.176；现状空流链
> 2×32K≈¥0.30）——64K 兜底单次最坏 ≈ 现状整条链且消除链式等待，兜底定
> **600s/64K**（S4 校准 300–900s/32–128K，合法锚点 3.6 倍思考空间）；
> 兜底管单次上限、D-6 管重试次数，两本账解耦。同日修订
> ——idle 死线 50s→30s
> （取代 STREAM-RETRY-RHYTHM 未实施的 50s）、160K 复读归因=架构工具设计
> （无再读闭环）已由 P0-0d 修正、作为恢复 256K 安全依据、重试层结论=保留
> transport 内链 + 吸收官方空流节奏（不迁移 step 边界）；重试分类=有可见
> 输出不重试（content 族，ADR-0007）、无可见输出降级；`DEGENERATION_LIMIT=3`
> 三族共享。设计轮不动计数（28）。登记于
> ADR-0010 §14.35（v1.35）/
> [设计](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 输出预算恢复与空流止损 S1 实施完成登记（用户放行实施、暂不
> 重建/测试——S2-S4 待续）——`REQUEST_MAX_TOKENS`/`ModelConfig::max_tokens`
> 32K→**256K**（回落档 128K 注释保留）；`stream_idle_timeout` 50s→**30s**
> （warn 5s / retry window 50s 不变）；D-6 流式空流链官方化收窄
> （`generate_stream`：完成型空响应快速有界重试 ≤2 次、退避 500ms→10s+
> 10% jitter → thinking 禁用降级 → 仍空显式失败；reasoning 族哨兵中断
> 不原样、直接跳降级；content 族不重试透传）；退化检测器升级为输出健康
> 哨兵（观测面 content+reasoning+tool arguments；三族信号 content 复读 /
> reasoning 复读灵敏层 / reasoning-stall 600s/64K 预算兜底、与 max_tokens
> 解耦；`REASONING_CHARS_PER_TOKEN=2` 估算 + usage 复核留痕；detail 前缀
> `degeneration_detected:content_repetition|reasoning_repetition|
> reasoning_stall`、`is_degeneration_detail` 收窄 + `is_reasoning_guard_detail`
> 新增、`DEGENERATION_LIMIT=3` 三族共享；run 层失败轮次审计补触发族标签）；
> 既有断言同步（256K 请求头 / idle 30s / 检测器 feed 签名）、live 探针与
> 指纹/注释同步。计数：实施放行入账（**28 → 29**），S3/S4 验证闭环后
> 29 → 28。登记于 ADR-0010 §14.35 第 2 项 /
> [设计 §4.1](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 输出预算恢复与空流止损 S2 测试实施完成登记（用户指示进行 S2）
> ——新增 15 项测试（退化检测器单测 10 项：reasoning 复读灵敏层、可见输出
> 停用 reasoning 族、stall 双信号 600s/64K OR 语义、估算校准、空转预算与
> max_tokens 解耦、空流重试参数/退避形状；空流链 e2e 5 项：完成型空响应
> 快速重试 2 次→降级、链尾显式失败、reasoning 复读/stall→直接降级、重试
> 中途哨兵→跳过剩余原样重试）；orz-loop lib 527 通过 / 0 失败 / 3
> ignored；fmt 干净、clippy 无新增告警（transport.rs 零告警，lib 21 与
> 基线一致）、`cargo check --workspace` 通过。计数不变（仍 29），S3/S4
> 验证闭环后 29 → 28。登记于 ADR-0010 §14.35 第 3 项 /
> [设计 §4.2](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 输出预算恢复与空流止损 S3/S4 闭合登记（用户指示重建与复验＋
> 对账；S1-S4 全部闭合）——S3 Linux musl 重建成功（ORZ-BUILD-MOUNT-001
> 契约，三件套时间戳更新，orz 104.4MB）；S4 gpt2-codegolf 单题复验
> （job `2026-08-20__18-46-52`，RUN-CLI-6a86db35，wallclock 跑满、
> reward 0.0、无异常）：**完成型空流 0**（32K 截断空流链根因消除，首
> 输出约 66s，对照 32K 时代 10.5 分钟级）、零 400、**journal 口径命中率
> 95.28%**（85 请求，hit 3,110,656 / miss 153,990）、`reasoning_repetition`
> 灵敏层拦截 1 次并降级收尾、stall 兜底（600s/64K）零触发零误杀、单请求
> 最大 completion 20,776（无预算放大异常）；output 191,623 tokens、按
> ¥4.592/M 估算输出成本 ≈ ¥0.88；控制台 CSV 待刷新补精确对账；600s/64K/
> 30s 初值维持不调。**计数：S3/S4 验证闭环 29 → 28**。登记于
> ADR-0010 §14.35 第 4 项 /
> [设计 §4.3](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 默认档 high + 三级降级梯修订设计定稿登记（用户裁决：方案 B
> + 中间档；先设计、不动作）——S4 实测（256K+max）机制已稳（空流 0、
> 命中率 95.28%、复读灵敏层拦截 1/85 并降级收尾、stall 兜底零误杀），
> max 不再是必要工作点；思考禁用本身质量影响大（「快答模式」）。
> **定案**：①默认 `reasoning_effort` max→**high**（官方默认档，
> `EnabledMax` 保留显式可选档，仍受哨兵保护）；②降级梯插入 **low** 中间
> 档——**high → low → disabled → 失败**（空响应快速重试与 reasoning 族
> 哨兵跳转共用；「middle」= `reasoning_effort=low`）；③兜底/重试节奏
> 不变（stall 600s/64K、idle 30s、退避 500ms→10s+10% jitter）；④指纹含
> thinking → 部署后一次性变化。代价=失败路径多一轮完整思考（每级受
> 64K/600s 兜底保护），病态率低（S4 1/85）可接受。实施路由 S1 代码 →
> S2 测试 → S3 重建 → S4 复验（难题单题 + high vs max 成本/产出对照）。
> 设计轮不动计数（28）；实施放行 28→29，验证闭环 29→28。登记于
> ADR-0010 §14.35 第 5 项 /
> [设计 §3.6/§4.4](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 S1/S2 实施登记（orz b72a0a4 已推送）——`ThinkingMode` 增
> `EnabledLow`、默认档 max→high（max 保留显式可选档）；`apply_thinking`
> 双旋钮统一；`generate_stream` 降级梯 high→low→disabled→失败（空响应
> 每档 ≤2 次快速重试、换档重置计数与退避；哨兵逐级下降；max 保留 S4 直跳
> disabled 基线）；指纹含 high/low；S2=默认 high/max/low 请求头断言 +
> 三级梯 e2e 3 项 + 既有 max 基线核对；orz-loop lib 531 通过 / 0 失败 /
> 3 ignored、fmt 干净、clippy 基线一致、workspace check 通过。计数：
> S1 放行入账 28→29，S2 不变（仍 29），S3/S4 闭环后 29→28。登记于
> ADR-0010 §14.35 第 6 项 /
> [设计 §4.5](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 默认 high + 三级梯 S1 全面审查处理登记（用户指示处理审查
> 全部问题；orz 1651f59，已提交、未推送）——审查结论=未发现功能缺陷，
> 设计合理、实现合理、设计与实现符合；处理 5 项观察级建议：O1 非流式
> `generate` 链不引入 low 档登记为有意不对称（已知边界，三级梯作用域=
> generate_stream，generate 仅服务 preflight/gate 快轮）；O2
> `build_request` 与 `apply_thinking` 双份映射补同步注释（直调仅测试
> 场景）；O3 `empty_response_backoff` 60s max_elapsed_time 登记为参数表
> 外兜底（每档 ≤2 次重试不可达）；O4 新增
> `config_fingerprint_reflects_thinking_tier`（默认档指纹==显式
> EnabledHigh、与 low/max/disabled 互异；设计 §3.6「指纹含 thinking 档」
> 补断言）——orz-loop lib 532 通过 / 0 失败 / 3 ignored（+1 项）、fmt
> 干净、clippy 基线一致（lib 21 均既有位置、transport.rs 零告警）；O5
> e2e 请求体子串匹配登记为已接受边界（mock 可控、无实际风险）。计数：
> 审查处理不改变未闭合计数（仍 29），S3/S4 闭环后 29→28。登记于
> ADR-0010 §14.35 第 7 项 /
> [设计 §4.6](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 默认 high + 三级梯 S3 重建 + S4 复验登记（用户指示推送后
> 重建、换题复验；S3/S4 验证闭环 29→28）——S3 Linux musl 重建成功
> （三件套时间戳 12:08，orz 104.4MB）。S4 **换题 make-doom-for-mips**
> （P0-0d 退化防护起源题）单题复验（job `2026-08-20__20-08-50`，
> RUN-CLI-6a86ee6c，wallclock 1740s 跑满、reward 0.0、零异常）——
> 完成型空流 0、零 HTTP 400、零 idle 死线、**journal 命中率 92.18%**
> （194 请求，hit 8,122,240 / miss 689,249，≥90% 达成）、哨兵/stall
> 全零触发零误杀（600s/64K/30s 初值维持不调）、首输出 5.5s（对照 max
> 基线约 66s）、output 169,182 tokens（reasoning 77%）输出成本估算
> ¥0.78（对照 max ¥0.88）。high vs max 跨题参照（任务不同非严格同题）：
> high 档 30 分钟内 194 请求/333 工具轮（max 85 请求）、每轮更快、
> output 成本更低；命中率差异属跨题非档位回归；input 侧不可直接对照。
> **计数：S3/S4 验证闭环 29→28**。登记于 ADR-0010 §14.35 第 8 项 /
> [设计 §4.7](docs/DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> / BACKLOG 0d / TODO P0-0d。
> 2026-08-20 冻结版本准备登记（用户指示：深入审查流转无问题后冻结跑分）——
> 深入流转审查结论：本轮改动部件接线无断点（请求构造 build_request +
> apply_thinking 双旋钮 → 流式 stream_once 哨兵 → generate_stream 三级梯
> → agent_loop 分类分流 → run_invalidated/run_failed；工具轮 175/194 正常
> 流转、哨兵/空流/idle 全零触发、指纹 request_header_change 实证 config
> 稳定）。冻结：orz-linux 三件套对应 orz commit 1651f59（工作树干净、
> 构建 12:08 UTC）——orz SHA256 6F20CA00CC736FE9222A1E7C7C93F46849A60C715
> D17BD21BD0DA17E13677068（104,375,344 B）、orz-signer AEB7630DC7E9A224BAE
> 267ADCC1F7081DC8BFDEB65175058BAA7D9D037AB92A7、orz-acaf-provision
> 6BF66CBC5A59F2366D2ACEA82E42DB32416E92F8F45ABF9F2FA4817FC9FC90C0；
> 冻结标记 [FROZEN-2026-08-20.md](D:/tb-eval/orz-linux/FROZEN-2026-08-20.md)
> （跑分目录内）。跑分入口就绪：`bash D:/tb-eval/run_official_2.1.sh`
> （TB 2.1 89 题 5 批次）；单题冒烟模板参照 run_pro_gpt2.sh。计数不变
> （28）；正式跑分启动另行登记。
> 2026-08-20 冻结刷新登记（用户指示推送后重建含模型舒适度引导的产物）——
> 重建成功（13:11 本地，orz 104,375,344 B）；二进制验证含「会话数据边界」
> 提示词（grep -a 命中）；**冻结刷新：源码 1651f59 → d250f11**，orz SHA256
> 6F20CA00… → **471A9A2437D6A73AB59638BF50F5734674F809029C817D1BB83A05567A346A8E**
> （orz-signer / orz-acaf-provision 与提示词无关、哈希不变）；
> [FROZEN-2026-08-20.md](D:/tb-eval/orz-linux/FROZEN-2026-08-20.md) 已刷新。
> 镜像源维护：rsproxy.cn /dist/ TLS 失败、aliyun rustup manifest 过期，
> build_orz_aliyun.sh 改用官方 static.rust-lang.org（容器内实测 200）；
> crates index 仍 rsproxy-sparse 实测可用；脚本 LF/无 BOM 修正。计数不变
> （28）。
> 2026-08-20 模型舒适度原则 + 会话数据边界先导引导登记（用户指示：不过度
> 限制模型、适度控制幻觉、让模型舒服些；orz d250f11，已提交、未推送）——
> 「模型舒适度」确立为显性设计原则：机械层负责兜底与防错（哨兵/门禁/
> 契约），自由留给模型思考与规划，正常路径零打扰；落地=提示词层改善
> （无新机械限制）：`BASE_SYSTEM_PROMPT` 新增「会话数据边界」引导
> （.gsa 树为运行时内部数据、不进入直接工具面，正向引导走 blackboard_read
> 分区/操作台反馈，如实告知直接访问只会得到拒绝）——减少模型反复碰壁
> （make-doom 复验 14 次 .gsa 直读尝试的提示词层改善）。system_sha256
> 变化 → 部署后首次请求一次性指纹变化（既有纪律）；新增提示词测试，
> orz-loop lib 533 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 基线一致。
> 登记于 ADR-0010 §14.35 第 9 项；未闭合计数不变（28）。
> 2026-08-20 冒烟跑分启动 + 合规预检处理 + 第 1 组结果登记（用户裁决：5 轮×
> 89 题、每组 5 题、每题一遍 k=1、仅本地 jobs-sweep、不构成最终提交）——合规
> 预检：三件套 SHA256 与冻结清单一致；数据集 2.1 sha256 与 leaderboard
> DATASET_REF 一致；静态检查口径（89 题 × ≥5 trials、无超时/覆盖/挂载）满足。
> 预检发现并处理 3 项：① `tb_agents/orz.py` 取证遗留 `ORZ_DEBUG_VIEW=1` 移除
> （预登记清理项，冻结二进制未动）；② 2.1 数据集任务名需带 `terminal-bench/`
> 命名空间前缀（run_sweep_5.ps1/.sh 与 run_official_2.1.sh 修正；原 `-i 裸名`
> 无法匹配——印证官方脚本此前未实跑）；③ 本机无 git-bash、WSL 不解析 D:/ 路径
> 且不传 PYTHONPATH → 新增 PowerShell 执行入口
> [run_sweep_5.ps1](D:/tb-eval/run_sweep_5.ps1)（list + print-config 验证
> 通过）；harbor 未登录（本地跑不需，上传/合并前需 auth login）。第 1 组
> （r1-g1=sweep-r1-g1，1 并发）结果：5 题仅 2 题完成、均 reward 0，3 题未跑
> （job 进程崩溃）——① schemelike-metacircular-eval：content_repetition
> （3-gram 重复率 0.87）→ orz exit 1（content 族已见输出不重试，设计内）；
> ② dna-assembly：reasoning_repetition（连续 5 段相同 delta）→ high→low，
> 随后 reasoning_stall（64K 估算 token、356s）再降档，最终 stream idle 30s
> 无数据 → model error；哨兵均按设计工作、止损生效，但两题连续触发说明模型层
> 今夜仍不稳定（与当晚症状同源）；③ 第 3 题起 harbor 拉任务时注册表 RPC
> `resolve_task_version` ConnectError（瞬时网络抖动；litellm 亦报 SSL EOF，
> 事后三主机 443 均可达）→ TaskGroup 未捕获使整个 job 进程崩溃（harbor
> 健壮性缺口：应只失败单 trial 而非整个 job）。**待用户裁决：暂停等模型稳定后
> 补跑 g1b（build-pov-ray / llm-inference-batching-scheduler /
> feal-linear-cryptanalysis 3 题），或按现状继续收集 89 题哨兵触发统计**。
> 计数不变（28）。
> 2026-08-20 模型侧退化定位取证登记（用户指示定位 + 网络波动确认：用户 Clash
> 节点超时已切换）——取证复跑 schemelike-metacircular-eval（带 gsa 卷实时
> journal，`jobs-diag/diag-schemelike-r1`，同冻结二进制）结论：① 前段行为健康
> （52 次工具调用、plan_write、操作台订单→ACAF 票据→发放全通、步骤门
> step_not_done fail-closed 正确拒绝），同提示词同题无「误导点」证据；②
> **reasoning_stall 复现**（335s、~64K 估算 token 纯推理无输出，14:11:43）→
> 按设计 high→low 降档后模型恢复工具调用——长推理空转是模型在难题上的真实
> 行为（dna-assembly 同形态）；③ 最终死因=**transport error: error decoding
> response body**（流多次 idle 5s 停滞、解码失败，14:14:21 run_failed）——
> DeepSeek 流今晚仍不稳定，网络是主导因素（harbor ConnectError、LiteLLM SSL
> EOF/握手超时同源）；④ 首跑 schemelike 的 content_repetition（0.87）在稳定
> 网络下未复现，判为网络压力下的流异常或单次劣采样，非哨兵误杀模式。结论：
> 冻结版无回归，r1-g1 失败主因=网络层 + 难题长思考（哨兵均按设计兜住）；
> 复跑仍 reward 0。待用户裁决后续跑法（等网络稳定后补跑 g1b 3 题 / 调整哨兵
> 预算 / 继续）。计数不变（28）。
> 2026-08-20 g1b/g1c 环境受阻 + 网络取证收口登记（用户裁决：先停；用户已重开
> 代理）——g1b（3 题）全败因=关代理后 docker.io 不可达 + 容器内 apt 超时
> （镜像经 docker.1ms.run 预拉已补齐，非架构问题）；g1c 第 1 题
> llm-inference-batching-scheduler 挂死 17+ 分钟（用户指示停止，job 已中断、
> 残留容器已清理）。**容器网络取证（决定性）**：容器无代理环境变量、DNS 正常、
> 小请求 curl 0.46s 返回、2MB 大请求体 4.6s 完整送达（排除 MTU/路径丢包）；
> orz 的 ESTABLISHED 连接 tx_queue=0、重传=0——请求体已完整送达 DeepSeek，
> 服务器 17 分钟零响应。结论：**非容器、非设计、非我方网络路径**——DeepSeek
> 服务端对大请求（10 万+ token 上下文 / 256K 输出预算 / reasoning=high）无
> 响应，与今晚账单/延迟异常同源（其他窗口请求小故正常）。跑分暂停；恢复方案
> 已定：镜像预拉齐（关代理可用）+ 容器 apt 直连验证 + DeepSeek 直连或稳定
> 代理下再跑。计数不变（28）。
> 2026-08-20 输出预算失控根因定位登记（用户怀疑「一次性输出太多」）——实测
> api.deepseek.com（deepseek-v4-flash，stream+reasoning_effort=high）：
> ① 同请求 max_tokens=8K：3.7s 自然收尾；② max_tokens=262144：首字节 0.8s
> 后**150s 收到 12.5MB 流仍未停止**（超时中断），25s 抽样确认为 reasoning 流；
> ③ 再跑一次 256K 则 25s 内自然收尾——**失控是概率性的**。结论：**256K 输出
> 预算使 deepseek-v4-flash 概率性无限生成（几十万 token），其他项目 max_tokens
> 小故正常；我方哨兵（content_repetition/reasoning_stall 64K/idle）实际成为
> 唯一刹车**，与今晚巨额 output 账单、r1-g1 两题退化、g1c 挂死同源。设计层面
> 待用户裁决：主代理 `REQUEST_MAX_TOKENS` 由 256K 收紧（建议 32K–64K，S4
> make-doom 194 请求/单轮输出远低于此）或维持 256K 仅靠哨兵兜底。冻结版
> 未动。计数不变（28）。
> 2026-08-20 失控触发条件精确定位 + 官方 DSH 对照登记——追加探针：①
> 256K+reasoning_effort=high+**重复性上下文**（2600 句相同填充）→ 150s 12.5MB
> 失控；② 同请求 256K 不带 effort / low / medium → 1.9–5.3s 自然收尾；③
> 256K+high+**非重复真实源码上下文**（166KB）→ 4s 自然收尾。**失控触发条件
> = max_tokens=256K × reasoning=high × 上下文/输出出现重复循环**——模型一旦
> 在难题上进入复读循环，256K 预算允许其无限延续（哨兵成唯一刹车）；官方 DSH
> （deepseek-harness v0.1）源码核验：`DEFAULT_MAX_TOKENS=256_000`、默认
> reasoningEffort=high 均与 orz 相同，但其架构笔记明确警告 256K 输出预算在
> 预分配端点会占满 1M 上下文、「gateway/模型只支持较小预算时必须调低
> maxTokens」——官方把 256K 当可配置上限且部署可调低；官方上下文形态不触发
> 重复循环，故用户侧未见失控。**待用户裁决（冻结版未动）**：A. 主代理
> `REQUEST_MAX_TOKENS` 256K→64K（正常轮次不受影响，S4 单轮均值 <1K；循环轮
> 快速触顶）保留 high 与三级梯；B. 维持 256K 仅靠哨兵兜底（现状成本/失败率）。
> 计数不变（28）。
> 2026-08-21 上下文机械结构块 PUSH→PULL 重设计定稿登记（用户裁决方向：从根本
> 上解决，架构独有问题）——根因链：失控 = 256K × reasoning=high × 重复性
> 上下文（API 实测四组对照钉死）；重复性上下文源自我们每轮 PUSH 的框架自有
> 机械块（`[TOOL_ROUND_BUDGET] REMAINING` 每工具轮 1 条且旧条不删、333 轮
> ≈333 条；`[任务状态]` 变化追加旧条不删；`[本轮编辑]`/压缩标记），这是
> 2026-08-07 前缀缓存修复（17%→98%）的副作用——用上下文重复换缓存稳定，
> 并放大复读失控触发面。官方 deepseek-harness minimal 模式（无运行时上下文
> 注入、无压缩、仅双工具、contextWindow 1M、idle 48h）为结构性避免参照。
> 设计：
> [CONTEXT_SCAFFOLDING_PULL_REDESIGN_DESIGN_2026-08-21.md](docs/CONTEXT_SCAFFOLDING_PULL_REDESIGN_DESIGN_2026-08-21.md)
> ——方案 A（推荐）：预算块 PUSH→PULL（退役 REMAINING 尾随消息，
> blackboard_read 新增 session 面按需读，机械门禁 budget_insufficient/
> 上限耗尽兜底）；方案 B（历史替换去重）因破坏前缀缓存否决；方案 C（兜底）：
> REQUEST_MAX_TOKENS 256K→64K；方案 D（可选）：状态行视 A 效果再定。路由
> S1→S4；待用户裁决实施范围。计数不变（28）。
> 2026-08-21 设计定稿登记（用户裁决：方案 A 先行、C 暂缓、状态行保留；
> S4 重点观测缓存命中）——`blackboard_read` 新增 `section=session`（remaining
> = controller activation tool_rounds_used/max_tool_rounds + render_status_line，
> live 面不进 epoch 归档，工具定义增量）；退役每工具轮 `[TOOL_ROUND_BUDGET]
> REMAINING` 尾随注入（D-8），总预算声明块保留，机械门禁（budget_insufficient
> 拒绝文本含剩余/耗尽块/run_invalidated）兜底；S2 需更新既有
> TOOL_ROUND_BUDGET 断言（controller.rs 12078 附近）；S4 观测：逐请求缓存
> 命中率（对照 S4 make-doom 98%+ 基线不减）、零 400、哨兵触发率下降、输入
> token 增长放缓。设计文档已更新为定稿。计数不变（28）。
> 2026-08-21 PUSH→PULL 重设计 S1 实施 + 全面审查处理登记（用户放行实施，
> 审查后指示处理全部问题；orz 7529a71）——退役每工具轮 `[TOOL_ROUND_BUDGET]
> REMAINING` 尾随注入（agent_loop D-8 注入点 + prompt `remaining_block` 移除，
> 零残留）；`blackboard_read` 新增 `section=session` live 会话面（controller
> `render_session_section`：BUDGET/USED/REMAINING + `render_status_line`，
> 数据源=in-run tool_rounds 含 activation 累计/max_tool_rounds；不进 epoch
> 归档；session+epoch / session+receipt_id 越权组合参数级显式报错 exit_code
> 1 + error 字段）；工具定义 enum/描述增量；系统提示词总预算块保留改为指向
> 按需读取（静态一次、前缀缓存纪律不变）；机械硬门禁原样保留。S2 测试随
> S1 交付：协议形态 5→4、无 REMAINING 尾随断言、session 面渲染/越权单测 +
> 工具级回达、budget_insufficient 拒绝文本仍含剩余；orz-loop 536 通过 /
> fmt 干净 / clippy 基线一致（31）/ workspace check / 事件一致性 15 通过。
> 全面审查=无功能缺陷；O1/O2/O3 已接受边界登记（口径差异/PULL 读取消耗轮/
> 状态行双通道，设计 §8）、O4 越权组合口径收紧、O5 工具级测试补齐、O6
> 工具描述去内部标签。登记于 ADR-0010 §14.35 第 10 项 / BACKLOG 0e /
> TODO P0-0e；计数不变（28）至 S3/S4 闭环。
2026-08-15 黑板 plan epoch 复查补强登记（ADR-0010 v1.15⑧）——plan_epoch 时间戳单调编号、身份一一对应强制、retention 保留最高编号快照。
> 2026-08-15 黑板 plan epoch 复查遗留闭合登记（ADR-0010 v1.15⑨）——F2 原子写盘+回退加载、F4 跨进程 `.claim-<n>` 占号、F5 归档目录单一来源、F6 非法 epoch 显式报错、F7 归档失败入事件面（新 v0.2 `epoch_archive_write_failed`）、F9 `persisted_at` 更名、F10 设计 §5 措辞对齐。
> 2026-08-15 ACAF fail-closed 生产启用裁决登记（用户裁决放行）——P2 IMPL-CONTROL-FABRIC 决策门放行；翻转执行与核查清单 ⑦⑨⑩⑪ 收口/边界登记待实施。
> 2026-08-15 强制模板轮实施闭合登记（用户指示优先）——ADR-0010 v1.16（§4.2 正文修订「注入」→「触发点暂停并填写模板」+ §14.16 实施登记）、v0.2 `checkpoint_response` 响应事件（Schema/verifier/fixtures/TUI）、强制模板轮实现（无工具 checkpoint 轮、一次重填、降级兜底、pending 单槽、主车道；检索车道不变）与缓解必做（证据身份交叉校验、gather_evidence 必填缺失面）；FUS-ORIENTATION-FORCED-TEMPLATE 转 implemented；实施审计见 `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。
> 2026-08-15 二次全面审查修复登记：DC fire 增 `agent_role=main`（Schema/fixtures/生成器）、验证器兼容旧 fire 并新增 outcome↔validation / gather_evidence 条件交叉、响应角色收紧主车道、Rust trim 长度口径、conformance 计数更名（详见实施审计 §6）。
> 2026-08-15 缓存上下文成本实施闭合登记（ORZ-CACHE-CONTEXT-COST 三项）——v0.2 `request_header_change` 事件（system+tools+config 摘要、initial/change、agent_role、previous_header_sha256）、验证器翻转↔header 交叉核对 + `assurance/probe_accuracy_audit.py` 误判审计、50K 单轮注入预算（`ORZ_MAX_INJECT_TOKENS_PER_ROUND`、无 ToolStarted 拒批、offset/grep 提示）+ 提示词读取纪律；FUS-REQUEST-CACHE 转 implemented；实施审计见 `docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md`。
> 2026-08-15 缓存上下文成本二次全面审查修复登记：`request_header_change` payload 增 `change_kind`（机械「变化原因」，Schema/verifier 摘要差一致性校验）；verifier 允许每车道多链 initial（子代理多 activation/主车道多 run 合法）；新增 `_verify_v02_inject_budget`；压缩摘要/预检等 loop 外辅助模型请求留痕边界登记（详见审计 §7）。
> 2026-08-15 P0-C 小样 2 闭合登记——编辑执行器 `workspace.search_replace` 对照实验通过（用户裁决 + 独立判定一致），下一裁决点=小样 3；结果工件见 CLASSICAL-EXEC-ASSISTANT 条目。
> 2026-08-15 P0-C 小样 3 闭合登记——机械组合脚本模式 `workspace.run_script` 对照实验通过（用户裁决 + 独立判定一致），下一裁决点=orz 内嵌集成；结果工件见 CLASSICAL-EXEC-ASSISTANT 条目。
> 2026-08-15 P0-C orz 内嵌集成 S1 登记——操作台核心（orz-loop `console`：注册表/契约/信封/trace，执行经 ActionExecutor 委托复用既有门）与黑板动作栏数据面（注册板块/动作栏单槽/结果栏）落地；模型面投影与轮末发放待续（S2）。同日全面检查修复登记——执行器错误细分 execute/policy（`policy_denied`）、响应契约强制必填、TraceStore 提交语义、最小参数提示投影、S2 验收点显式化。
> 2026-08-15 P0-C orz 内嵌集成 S2 登记——模型面投影（`blackboard_read` section=actions：注册板块/动作栏单槽/结果栏有界渲染，随 plan epoch 归档可读；`blackboard.action_write` 写单按钮：pending 机械拒绝、round/plan_epoch/run_id 机械盖章、主车道专属三重守卫）与轮末机械发放（post-tool-batch 安全间隙、checkpoint 优先；round/plan_epoch/run_id 防重放与过期 `order_stale` → 注册表/契约 → ControllerConsoleExecutor 委托 run_host_tool（权限/ACAF/模式门/事件链）→ 响应 schema 验证 → 结果栏 receipt+trace_id → TraceStore.commit；策略拒绝归一化 step=policy + policy_denied）；注册板块每轮机械刷新（基础动作集 6 项）；orz-loop 364 通过 / 0 失败；实施审计见 [GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT](docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md)。
> 2026-08-15 P0-C S3 前置闭合登记——结构化策略拒绝（P1-2 定案）：`ToolResult.policy_denial = {source, code, reason}`（source ∈ permission/acaf/retrieval_mode/taint）接线权限/ACAF/检索模式门五条拒绝路径；console 适配层只按结构化信号映射 step=policy，`console_policy_refusal` 前缀判定退役；ToolCompleted 增可选 `policy_denial`（Schema/verifier/fixtures 先行 + exit_code 非 0 / 工具族交叉规则）；内容碰撞回归；orz-loop 366 / acaf_e2e 21 / Python runtime 209 通过，仓库门禁 valid；实施审计见 [GAP_CLASSICAL_EXEC_S3_PRELUDE_IMPL_AUDIT](docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md)。
> 2026-08-15 P0-C S3 前置全面审查修复登记（F1-F8）——拒绝事件补 `exit_code=1` + `status=error`（含 host 级拒绝）、verifier ACAF 家族补 `web_fetch`/`browser_read`、新增生产者事件→验证器对拍测试（`PolicyDenialProducerParityTests`）、`orz/` 登记为父仓库 git 子模块（SilverWhite/CLI feat/fusion-architecture，提交 a0c9ffc 已推送）+ 源码完整性清单（`orz_source_manifest.sha256`，1406 文件）接入仓库门禁；orz-loop 367 / acaf_e2e 21 / Python runtime 213 通过；详见 S3 前置审计 §7。
> 2026-08-15 P0-C S3 实施闭合登记——`assistant.trace` 只读服务生产接线（`ActionKind::TraceRead`，按 trace_id 有界取回 + 读操作入 trace 与 ToolStarted/ToolCompleted 事件面）、`workspace.run_script` PTC 线性脚本生产化（`ActionKind::RunScript`：`$ref` 静态/运行时校验、逐行契约校验 + trace、8 步/30s/4MiB 上限、禁嵌套、fail-closed 保留内层 step/code + `script_step`）、Profile/Bundle 按钮组加载（`ActionBundle` standard/read_only/benchmark + 注册板块 = Profile/Bundle ∩ 探针完整集，同轮探针快照同时驱动工具投影与注册板块）；orz-loop 377 / orz-host/tui/bin/assurance check 通过、仓库门禁 valid；实施审计见 [GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT](docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md)。
> 2026-08-16 P0-C S3 全面审查收口登记（用户逐项裁决）——单订单步数上限 20→8（`MAX_SCRIPT_STEPS_PER_ORDER`）、`assistant.trace` 查无 id 定案 `step=execute`+`not_found`、checkpoint 轮跳过注册板块刷新、注册不变式补齐（内部动作带 target 拒绝/内部动作类唯一/bundle 非空/嵌套按 kind 拒绝）、最终超限信封补 `script_step`；orz-loop 384 / 0 失败；S4 登记=单步超时（host 层进程树收口）+ 脚本 tool-round 预算消耗；详见 S3 审计 §6。
> 2026-08-16 P0-C S4 实施闭合登记——单步超时下沉 host 层（`LoopHost::call_tool_with_timeout` 显式覆盖 = min(覆盖, 配置预算)、到期进程树收口；`ToolResult.timed_out` 结构化信号 → 直接订单 `tool_timeout`、脚本归一化 `script_timeout`+`script_step`）；脚本 tool-round 预算（发放前预检 `budget_insufficient` 零执行拒绝、实际执行步数减计、下一轮预算块机械反映）；端到端测试（FakeProvider 完整任务会话、checkpoint 轮板块保留、超时/预算边界）；orz-loop 392 / 0 失败；决策门材料清单齐备；实施审计见 [GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT](docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md)。
> 2026-08-16 P0-C S4 全面审查收口登记（二次）——host-owned 同步工具（`project_doc_index`/`browser_read`/`pdf_read`/PDF 路由 `web_fetch`）不经 timeout 包装为既有边界（配置预算=经注册表执行调用的硬上限）；脚本层每步后核对 30s 总截止（`script_timeout` 事后 fail-closed）；预算预检先静态校验脚本（不掩盖内层错误）；小样 1 结果工件补齐（`sample1_result.json` 90/90 复跑）；orz-loop 396 / 0 失败；详见 S4 审计 §8。
> 2026-08-16 P0-C S4 超时语义复核登记（用户复核 + Codex/Grok 成熟设计对照）——撤销 30s 总墙钟含进程时间语义：脚本每步由 host 配置预算独立约束（不传收缩剩余）；`MAX_SCRIPT_WALLCLOCK_SECONDS`/事后核对删除；orz-loop 395 / 0 失败；详见 S4 审计 §8。
> 2026-08-16 P0-C 正式组件决策门闭合 + PLAN-FIRST 阶段 A 实施闭合登记——用户裁决「P0-C 可转正式组件」（不达标即撤条款未触发）；模板去人格（主/子代理/apply-patch/Orchestrator body + XOR 模板重生成 + 无人格关键词渲染测试）、AGENTS.md 计划型机械包裹（`<plan_first_framework>` 固定前缀，用户内容之前）、首轮计划轮硬门（`plan_write` v0.2 事件 + 结构化校验/一次重填/降级留痕 + 结构化计划落黑板 plan epoch + 会话级门 + ACP server/CLI run 生产接线）；FUS-PROMPT-DEPERSONALIZE / FUS-AGENTS-MD-PLAN-WRAPPER 转 implemented，FUS-PLAN-FIRST-MODEL-SURFACE 转 partial；实施审计见 [GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT](docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md)。
> 2026-08-16 PLAN-FIRST 阶段 A 全面审查收口登记（用户逐项裁决）——计划轮不消耗 tool-round 预算、compaction whitelist 窗口顺延至计划落板后首个执行轮、结构上限定稿（步骤 ≤32 / 动作 ≤8 / 证据 ≤16 / 总量 ≤32K，参照 AutoGPT/oh-my-loop/joyagent/编排工具/LangChain 成熟设计）、AGENTS.md 包裹改为系统提示词层无条件注入（canonical 常量入 orz-assurance `plan::framework`，不依赖 AGENTS.md 存在）、P1 工具事件契约形状收敛（plan_write ToolCompleted 成功仅 exit_code、主车道拒绝事件去 target）、plan_write 面随开关收敛（子代理投影/ToolFilter 剔除、关闭态/grill 拒绝）、P3 闭环（TUI 按 outcome+degrade_reason 投影、同轮单次 plan_write、ToolDef 参数嵌套 schema、证据/总量上限、旧 epoch 归档兼容、PlanWrite 序列规则、persona 旧测试更新）；ADR-0010 §14.17⑯、实施审计 §7 登记；orz-loop 416 / orz-tui 178 / orz-assurance 152 / Python runtime 299 通过。
> 2026-08-16 PLAN-FIRST 阶段 B 实施闭合登记——注册板块=探针投影收口（controller 单一探针源 `console_probe_source` → `sync_console_registrations` 派生唯一路径，无探针轮次不再 bundle-only 刷新、沿用上一轮内容，移除静态基础集中间态；探针源随 run 起始复位）+ 工具栏刷新绑定黑板模型栏（`blackboard_read section=actions` 读取时由最近探针源派生并持久化，归档 epoch 读保持快照；工具投影与注册板块共用同一探针源、同源一致性测试锁定）；orz-loop 421 / 0 失败（新增 3 项阶段 B 单测 + 2 项审查收口单测：归档读不派生、run 起始复位）；实施审计见 [GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT](docs/audits/GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md)。
> 2026-08-16 PLAN-FIRST 阶段 C 实施闭合登记（console 默认 + direct 受控降级双模式）——v0.2 事件 +2（`console_mode_transition`：switch/stay/return 决策记录；`console_order_written`：订单身份/step 绑定/机械盖章，action_write ToolCompleted 收敛通用形状，阶段 A 审计 §7.4 债务收口）、tool-started/completed 增 direct 盖章字段、双模式状态机（run 级模式/3 连败故障面/无工具询问轮/switch+gate_log/stay/return）、步骤状态机（`pending → in_progress → done|failed`、`ActionOrder.step_id`、步骤门 `step_not_done`、`console.step_done` 证据门）、模型面收敛（console 面=黑板读写+只读核查无执行工具，direct 恢复工作工具投影并全链路盖章；生产接线 CLI run + ACP server）；FUS-CONSOLE-DUAL-MODE / FUS-PLAN-STEP-GATE / FUS-PLAN-FIRST-MODEL-SURFACE 转 implemented；实施审计见 [GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT](docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md)。
> 2026-08-16 PLAN-FIRST 阶段 C 二次全面审查收口登记——F1 verifier↔producer 对拍（`console_order_written` 增 `write_call_id`，Schema/verifier/fixtures/测试同步）、F2 `cargo fmt` 收口、F3 direct 内建工具 ToolCompleted 盖章对称、F4 拒绝路径不改步骤状态、F5-F7 文档同步、F8 clippy 基线核对；详见阶段 C 审计 §7。
> 2026-08-16 ACAF 全面审查处理登记——P2-I1 grill 路径补 D-15 启动期拒绝、P2-I2 AcafClient 改 per-session 缓存（多会话交错不重置一次性账本）、respawn 活性修复（原 e2e 未真实崩溃；respawn 后序列重启+账本随 epoch 重置）、P3-I4 签发器请求行 1 MiB 上限；权威文本四项登记（policy_revision 占位 policy_digest、previous_receipt_sha256 简化、web_search 例外、决策 11 前置由用户裁决豁免）；BACKLOG 陈旧可选项收口；详见 fail-closed 审计 §8。
>
> 2026-08-16 会话监测度量重定登记（用户裁决）——FUS-SESSION-CONTEXT-MONITOR 度量由累计 token（384K/500K）改为会话内压缩次数（`context_compressed` reason∈rhythm/fallback 计数、`session_end` 不计、一次一计）、阈值改次数制（≥2 提醒 / ≥3 推荐，可配，默认待校准）、chars/2 校准项废止；ADR-0010 v1.18/§14.18 登记；设计已改、实施未动。
> 2026-08-16 DeepSeek 主/子代理同构复核闭合登记——transport/retry/thinking 三实例共享单一 DeepSeekTransport/RetryPolicy/ThinkingMode::EnabledMax、160K 单一常量，`-p` 预检轮为文档化请求级覆盖；仍 partial=DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7）；跑分决定使用 deepseek-v4-flash（生产默认，无需 env 覆盖）。
> 2026-08-17 console 面缓存注记登记（用户指示补记）——console 默认面注册板块/工具栏为黑板数据（`blackboard_read section=actions` 取回），不参与请求 header 指纹（tools 摘要），工具栏刷新不再构成 v1.9「工具集变化即前缀 miss」的接受代价；header 变化仅剩 console↔direct 切换与只读工具探针翻转等真实状态变化；ADR-0010 v1.19/§14.19 登记；纯注记、无代码变更。
> 2026-08-17 评测冒烟暴露问题登记（最优先）——GAP-ACAF-HARNESS-PASSTHROUGH（ACAF fail-closed 默认强制后 TB2 适配器不转发签发器配置，`orz --real` 拒启；临时影子模式解阻，正式决策待定）+ GAP-CONSOLE-TOOLNAME-PATTERN（console 面工具名点号违反 OpenAI 兼容工具名模式，真实 API 计划落板后 400；修复=改名下划线 + verifier/schema/文档同步 + 重建重跑）；顺带观察 2 项（plan_write 提示词、actions 校验）；处理窗口=新 Codex 窗口；详见 BACKLOG 0a / TODO P0-E。
> 2026-08-17 GAP-CONSOLE-TOOLNAME-PATTERN 闭合登记（本窗口处理）——三个 console 面工具名点号改下划线（`blackboard_action_write`/`console_step_done`/`console_return_to_console`），同步 11 Rust 文件 91 处 + Python verifier/schema/测试 + ADR-0010 §14.20 与设计文档（orz 子模块 0304b23）；orz-loop 434 / orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 + real_flag 2 通过、clippy 无新增告警；manifest 重生成 1401 条目、仓库门禁 valid；Linux musl 重建后冒烟重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57`（30m21s 跑满 1740s 预算、`run_invalidated{wallclock}` 正常收尾，对比旧运行 400 即死）。GAP-ACAF-HARNESS-PASSTHROUGH 保持开放（正式跑分决策待定）。详见 BACKLOG 0a / TODO P0-E。
> 2026-08-17 ACAF 跑分决策 + P0-E 下一步实施项登记（用户裁决 + 冒烟重跑定位）——GAP-ACAF-HARNESS-PASSTHROUGH 用户裁决「跑分保持 ACAF 强制开启」（容器内供应 manifest/keystore/signer，不接受影子模式；实施待做）；两项 P1 观察升为实施项（plan_write 提示词/示例强化、steps[].actions 形状校验收紧）；冒烟重跑（`D:\tb-eval\jobs\2026-08-17__03-48-57`，reward 0.0）定位新增步骤门模型面缺口——`blackboard_read section=plan` 不渲染步骤 id（epoch.rs 仅 `[status] goal (actions; evidence)`），步骤门要求订单 step_id 精确绑定致模型猜测空转（4 次 plan_write）；另记 grep 侦查低效（目标字符串不存在→空结果被泛化为无源码）。详见 BACKLOG 0a / TODO P0-E / ADR-0010 §14.21。
> 2026-08-17 grep 侦查纪律项登记（用户确认一并处理；**2026-08-17 复核更正归因**）——原记「模型用不存在的目标字符串 grep 全树、空结果被过度泛化为「无 C 源码」（工具行为正确，exit_code=1 无匹配）」；复核 journal/trajectory/orz.txt 证据后撤回：两次 `doomgeneric_mips|frame\.bmp` grep 实际被 plan/console 门机械拒绝未执行；执行的 7 次 grep 全部无匹配（wall_ms 1–36ms），含 vm.js 实测存在的 entryPoint/symbolName/sectionsToLoad/syscallNum/runElf/program counter → 系统性工具层空结果（疑搜索范围/路径解析异常），「工具行为正确」不成立。更正实施方向=先容器内 grep 冒烟（对已知字符串断言匹配）定位根因 + grep 结果补机械范围报告（解析路径/搜索文件数/忽略文件数）+ exit 2 语法错误保持硬失败；模型侧侦查纪律降为次要契约提示。P0-E 5 项、未闭合 32 项（计数不变）。详见 BACKLOG 0a / TODO P0-E / ADR-0010 §14.21。
> 2026-08-17 订单发放前拒绝入事件面项登记（用户指示处理）——冒烟重跑 ORD-000011（workspace.run_tests，arguments={}）写入后发放前被拒，失败只进结果栏 receipt + TraceStore（consume_console_order 未写 journal 事件），事后核对看不到拒绝码；目标=发放前拒绝（order_stale/step_not_done/budget_insufficient/registry/contract/target/ACAF/policy/mode 门）统一入 v0.2 事件面（新增 console_order_rejected，Schema/verifier/fixtures 先行）；P0-E 6 项、未闭合 33 项。详见 BACKLOG 0a / TODO P0-E / ADR-0010 §14.21。
> 2026-08-17 GAP-ACAF-HARNESS-PASSTHROUGH 实施登记——前置设计变更（用户裁决）：新增 `file-0600-installation` 文件型安装密钥库（Linux 下 DPAPI 不可用，原 fail-closed 边界解除；ADR-0010 §14.21 项 4 / ADR-0011 §5），orz 子模块 e8274e1（keystore 文件型后端 + storage 分派 + provision/signer 非 Windows 分支 + 0600 测试），orz-bin 全量通过（acaf_e2e 23 / signer 14 / provision 2）；schema/fixture 同步、仓库门禁 valid；适配器容器内落 manifest+keystore、run 设 ACAF env=1；`.env` 影子覆盖已移除；冒烟待 Linux 重建后执行。
> 2026-08-17 GAP-ACAF-HARNESS-PASSTHROUGH 闭合登记——Linux musl 重建（父仓库挂载 `/orz`、工作目录 `/orz/orz`，15m45s）后容器冒烟通过：provision 落 0600 keystore + manifest 真哈希、signer stdio initialize_session 应答（文件密钥库加载成功）、`orz --real` 带签发器启动 3 次成功（启动期 fail-closed 不变量通过、wallclock 正常收尾、退出码 0）；冒烟 journal 见 `D:\tb-eval\jobs\2026-08-17__05-45-ACAF-SMOKE`；P0-E 剩 5 项、未闭合 32 项。
> 2026-08-17 ORZ-BUILD-MOUNT-001 事故/案例登记——容器构建挂载契约（构建须挂父仓库为 `/orz`、工作目录 `/orz/orz`，否则 orz-assurance include_str 编译期失败；用户确认此前已出现过一次）；预防=构建脚本前置守卫（秒级失败）+ 两份评测文档命令修正 + orz 源码注释契约；案例库新增 `harness_environment` 分类。
> 2026-08-17 计划视图渲染步骤 ID 实施闭合登记（P0-E 第 4 项，用户指示处理）——
> `blackboard_read section=plan` 每步行首渲染 `step.id`
> （`- [状态] <step_id>: <目标> (actions: N; evidence: M)`，epoch.rs 渲染 live
> 视图与归档 epoch 读同源）+ 系统提示词状态行当前步补 `[step_id]` +
> `blackboard_read`/`blackboard_action_write` 工具描述补取 id 提示；测试三层
> （epoch 渲染单测、工具级 `section=plan` 回达、跨 epoch 归档读）；orz-loop
> 436 / orz-tui 178 / orz-assurance 152 / orz-bin 全量通过、clippy 无新增告警、
> manifest 1401、仓库门禁 valid。P0-E 剩 4 项、未闭合 31 项。
> 详见 BACKLOG 0a / TODO P0-E / ADR-0010 §14.21。
> 2026-08-17 订单发放前拒绝入事件面实施闭合登记（P0-E 第 4 项，用户指示处理）——
> 新增 v0.2 `console_order_rejected`（order_id/step/phase/code/reason/round/
> plan_epoch/run_id），发放前拒绝统一入事件面（pre_issue=order_stale/
> step_not_done/budget_insufficient；issue=registry/contract/target/ACAF/
> policy/mode 门，归一化 policy_denied；execute/verify 不入本事件）；
> Schema/verifier/fixtures 先行（verifier 交叉核对=先有同 run 同 order_id
> 的 console_order_written、盖章一致、每订单至多一次拒绝、phase/step/code
> 一致性），结果栏 receipt 保留；orz 子模块 c67a452（事件变体 + 三处
> pre_issue / 发放期 issue 路径发事件 + TUI 投影 + 测试断言）；
> orz-loop 436 / orz-tui 178 / orz-assurance 152 / orz-bin 全量通过、
> clippy 与基线一致、manifest 1401、仓库门禁 valid。P0-E 剩 3 项、
> 未闭合 30 项。详见 BACKLOG 0a / TODO P0-E / ADR-0010 §14.21。
> 2026-08-17 P0-E 第四/六项复核更正登记（用户复核 + 证据回查；纯文档/待办更正、
> 无代码变更）——第四项 plan_write 放弃特化示例方向（单点偶发、示例强化属过拟合），
> 收窄为校验错误消息形状明确（plan 必须是含 plan_id/goal/steps[] 的对象，got string
> 时错误消息写明形状）；第六项 grep 归因更正为系统性工具层空结果（前两次 grep 实际
> 未执行、其余 7 次全无匹配含实测存在的字符串），实施前置=容器内 grep 冒烟定位根因 +
> 结果补搜索范围报告。P0-E 剩 3 项、未闭合 30 项（计数不变）。详见
> BACKLOG 0a / TODO P0-E / ADR-0010 §14.21。
> 2026-08-17 大文件读取契约设计定案登记（用户裁决；纯文档、未实施）——读取工具契约
> 升级为有界返回：超过粗门（默认 16KB、可配 8–32KB）的文件返回读取句柄信封
> （path/size/encoding/content_sha256/可用范围/有界预览 ≤2–4KB/truncated/offset
> 续读指针）而非全文；精门=50K 单轮注入预算（`ORZ_MAX_INJECT_TOKENS_PER_ROUND`）
> 兜底；小文件保持全文一次返回。语义适配留模型、助理层只提供机械原语；模型以
> `read_file(offset)`/`grep` 结构化续读（既有提示词策略化读取落成工具契约，与
> pdf_read document_id+page_range 先例对齐）。黑板/结果栏只放指针不放内容本体
> （内容留盘上/证据区，维持不新增自由随记区）；注入层事后拒批前移为契约层事先
> 有界返回。ADR-0010 v1.22/§14.22；FUS-LARGE-FILE-READ-CONTRACT
> `current-design`；实施路由 BACKLOG 6f / TODO P1。未闭合计数不变。
> 2026-08-17 grep 搜索范围契约设计定案登记（用户复核；纯文档、未实施）——
> 撤回「补范围报告文本」方向（局限=只增可见性、不修机制、grep 单点）；根因定位
> =finalize_grep 将「rg 搜索 0 文件」与「真无匹配」合并（exit 1 + 空 stdout 统一转
> "No matches found"；ORZ 总传显式路径、rg 不打印 "No files were searched" 警告，
> 该分支死代码）；定案=grep 结构化搜索信封（resolved root/files_searched/
> files_skipped/match_count/truncated，机械来源 rg --stats/--json）+ 结局三型
> 分型（searched=0 显式报范围空与过滤类别，不叫 "No matches found"）+ 搜索范围
> 语义显式化（与只读工具可见集对齐或 --no-ignore/--hidden 开关）+ 契约泛化
> （读/搜/列三族统一，与读取信封同构）。ADR-0010 v1.23/§14.23；
> FUS-TOOL-SCOPE-CONTRACT `current-design`；实施路由 BACKLOG 0a / TODO P0-E。
> 未闭合计数不变。
> 2026-08-17 grep 搜索范围契约实施闭合登记（容器冒烟复现根因后实施）——根因=
> 构建侧打包 glibc 动态 rg（trixie /usr/bin/rg 要求 GLIBC_2.39）进 musl orz，
> 任务容器 bookworm（2.36）加载失败 exit 1 + 空 stdout，stderr 被 finalize_grep
> 丢弃成 "No matches found"（7/7 grep 全空）；实施=finalize_grep 结局三型（非零
> 退出 + stderr 非空显式报错 / searched=0 零范围 / 真无匹配带计数）、空结果路径
> `rg --files` 探针（同过滤集、10K 截断；弃用 --stats 因 rg 15 stdout 与旧版
> stderr 位置差异污染流式面）、hidden/no_ignore 开关入参数面与 console 注册表、
> build.rs 非 Windows 覆盖路径 ELF PT_INTERP 静态守卫、两份评测构建脚本改静态
> musl rg；测试 grep 模块 42 / types 561 / orz-loop console 68 通过；Linux
> 重建后容器冒烟回归**已执行通过**（打包 rg 静态、version+已知字符串断言命中、
> 端到端 `orz --real` grep vm.js 命中 20 行并 done、exit 0；证据
> `D:\tb-eval\jobs\2026-08-17__GREP-FIX-SMOKE\smoke-notes.md`）。
> P0-E 剩 2 项、未闭合 29 项。详见 BACKLOG 0a / TODO P0-E / ADR-0010 §14.23。
> 2026-08-17 ORZ-TOOL-BINARY-COMPAT-001 事故/案例登记（用户裁决加入案例库 +
> 归因纪律）——打包 rg（trixie glibc 动态二进制要求 GLIBC_2.39）与任务容器
> （bookworm glibc 2.36）不匹配、加载失败被 finalize_grep 吞成空结果的完整
> 反面教材；归因纪律=命令/操作错误先排查环境与机械因素（二进制/运行时兼容、
> 工具包装吞错误、路径/作用域解析、沙箱/权限/ignore 语义、构建打包来源），
> 再归因模型或命令纪律。案例库 README 同步登记。未闭合计数不变（29 项）。
> 2026-08-17 P0-E 收尾两项闭合登记——plan_write 校验消息形状明确（plan 缺失/
> 非对象错误写明 `expected an object with plan_id / goal / steps[]` 与 got
> 类型；回归测试字符串计划→机械拒绝→错误含形状）；`steps[].actions` 实证审计
> +探针测试锁定（14 种宽松形状全被拒，原「空字符串通过校验」观察不成立、无需
> 收紧；with 内容/do 注册表核对仍留订单发放契约校验）。orz 子模块 11540fa；
> planning 13 / plan_first 9 测试通过、clippy 无新增告警。**P0-E 全部闭合
> （0 项）、未闭合 27 项**。详见 BACKLOG 0a / TODO P0-E / ADR-0010 §14.21 项 2。
> 2026-08-17 FUS-TOOL-SCOPE-CONTRACT 后续两项补记计数登记（用户指示）——
> grep 面实施审计「边界与后续项」两条入账：list_dir ignored/truncated 计数
> （读/搜/列三族统一）、grep 命中路径 files_searched 留空（--json/stats
> 位置收敛后再定）；P0-E 主项仍 0 项，未闭合 27 → 29（后续 2 项）。
> 实施路由 BACKLOG 0a / TODO P0-E。
> 2026-08-17 大文件读取契约实施闭合登记——GrokBuild `read_file` 文本路径有界
> 返回：超过粗门（默认 16KB、可配 8–32KB，env/TOML 口子）返回读取句柄信封
> （path/size/encoding/content_sha256/available_range/有界预览 ≤4KB/truncated/
> offset 续读指针）而非全文；小文件全文一次返回；信封 terminal-only；单行超长
> 预算内截断；SKILL.md/`skills` 全量豁免；`FileTooLarge` 文本路径被取代保留为
> 防御兜底。提示词/工具描述/console 注册表同步。测试 orz-tools read_file 199 /
> orz-loop 440 / orz-host read_file e2e 2；clippy 无新增可归因告警；orz 172b14e；
> manifest 1401、仓库门禁 valid；ADR-0010 §14.22 项 3 / BACKLOG 6f / TODO P1 /
> 实施审计 `docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md`。
> 该组 4 项此前未计入 P1 分组计数（口径遗漏），本次全部闭合后 P1 回 9、未闭合
> 总数维持 29（补计与闭合相抵）。
> 2026-08-17 大文件读取契约全面检查修复登记——P2-1 信封空窗口/越界 offset
> 语义（past-EOF truncated=false/offset=None、范围内空窗口 offset=start_line、
> 最后一行行内截断 offset=None）、P3-1 `[toolset.read_file]` 配置节端到端接线
> （coarse_gate_bytes 经 orz-config 分层装载注入工具参数、优先于 env）、
> P3-2 concise 描述、P3-3 envelope 不追加 cursor rules 边界登记、P3-4 提示词
> 措辞；orz 7c4a99e + bd8d485、manifest 1401、门禁 valid；
> ADR-0010 §14.22 项 4 登记。
> 2026-08-18 Benchmark 完全体执行面设计定案登记（用户裁决；纯文档、未实施）——
> TB2 跑分采用 orz 完全体：Benchmark 策略两轴参数化 `Benchmark{allow_shell,
> allow_network}`（默认 false/false）+ 探针 `ToolPolicy::BenchmarkFull` +
> console 注册表新增 `workspace.run_terminal` 动作（shell 仍不开放为模型直接
> 工具，执行全经助理层订单，与 run_tests 同构）；CLI 增 `--allow-shell`/
> `--allow-network`；适配器按任务 network_mode 透传（89 题全 PUBLIC）；
> ACAF/预算/墙钟/事件审计门禁不变。ADR-0010 v1.24/§14.24；
> FUS-BENCHMARK-FULL-EXEC `current-design`；设计文档
> `docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md`；实施路由=实施前登记
> BACKLOG / TODO。未闭合计数不变。
> 2026-08-18 Benchmark 完全体执行面实施登记（用户指示：实施、暂不测试）——
> 三层同时使能：① 权限 `PermissionPolicy::Benchmark { allow_shell,
> allow_network }`（默认 false/false 保旧语义；shell/SandboxEscape 在
> allow_shell、NetworkCall 在 allow_network 下 AllowOnce；MCP 恒 deny）；
> ② 探针 `ToolPolicy::BenchmarkFull`（tool_policy() 由
> `Benchmark{allow_shell:true,..}` 映射；policy_allows_exec 增
> BenchmarkFull；ActionBundle::allows 复用 benchmark 档）；③ console 注册表
> `workspace.run_terminal`（target=run_terminal_cmd、READ_WRITE、input 镜像
> BashToolInput：command/description 必填、timeout/is_background 可选、
> 不暴露 env/cwd）。CLI 增 `--allow-shell`/`--allow-network`（须与
> `--allow-write` 同用，否则 exit 2）。适配器 `tb_agents/orz.py`：
> allow_shell=True、allow_network 按任务有效 agent-phase network_policy==
> PUBLIC 透传（实施注记：取 environment.network_policy 而非
> task_env_config 基线，89 题全 PUBLIC 结果一致、严格不更宽）；env + 运行
> 脚本 belt-and-braces 同传。FUS-BENCHMARK-FULL-EXEC `pending`（实施完成
> 待验证）；orz 子模块 3f43478；BACKLOG 0b / TODO P0-F；未闭合计数
> 27 → 28（验证闭环后回 27）。
> 验证（用户指示暂缓）：orz 测试 + clippy、Linux musl 重建、单题
> make-doom-for-mips 复验（reward>0 + journal workspace.run_terminal→ACAF
> command_exec issued/consumed）、2–3 题交叉、89 题 5 批。
> 2026-08-18 审查收口处理登记（全面审查后）——`is_shell_tool` 补 `sh` 名级
> 兜底（permission.rs/tool.rs + 断言）；`workspace.run_terminal` timeout 契约
> lenient（anyOf integer/纯数字字符串 + default 120000）、is_background 补
> default false + 契约测试；CLI `--allow-shell=<v>` / `--allow-network=<v>`
> 值形式显式报错 exit 2（parse_benchmark_flags + 4 组单测）；bundle 保持
> READ_WRITE 实施选择确认；`is_background` 后台完成提醒留验证④观察。详见
> 设计 §12 / ADR §14.24 / BACKLOG 0b / TODO P0-F。
> 2026-08-18 验证①闭合登记（用户放行执行）——orz 各 crate 全量测试全绿
> （orz-loop 453 / orz-host 221 / orz-tui 178 / orz-assurance 152 /
> orz-bin 11+14+23+2+1 / orz-tools 2761，0 失败）；clippy 无新增可归因
> 告警；manifest 1401 + 仓库门禁 valid。过程中修复 PLAN-FIRST/console
> 双模式落地后的既有测试漂移（codex_app 12 + acp_server 1，orz c4772fc；
> orz-host 需 `--test-threads=1`）。验证②（Linux musl 重建）进行中——
> Docker 引擎卡死，用户裁定重启电脑后续跑。
> 2026-08-18 验证③取证存档登记——make-doom-for-mips 单题复验三次均
> reward 0（400 `insufficient tool messages` 退出）；核心机制已验证
> （workspace.run_terminal 订单→发放→run_terminal_cmd exit=0 + ACAF
> 票据路径），400 根因链闭合=折叠 cut 破坏 plan_write 轮配对 +
> safe_fold_cut idx==0 兜底（取证存档
> `D:\tb-eval\jobs\2026-08-18__08-44-56\ROOTCAUSE_FORENSICS_20260818.md`；
> orz 3bd09fc/5bc3add 取证 WIP）。修复待下一窗口。
> 2026-08-18 折叠视图 400 修复处理文档登记（用户指示：先落实处理文档）——
> 验证③ 400 根因复核修正：真正破坏点=压缩触发（未执行）时
> `run_template_compact` 顶部 retain 删除 marker 而折叠索引未失效
> （GuardBlocked 无 reset），冻结 preamble 吞入首轮 plan_write 声明（其
> 回复在折叠区），而非「折叠 cut 硬截断」（cut 始终在完整轮起点；
> `safe_fold_cut` 只防 cut 不防 fold_start）。处理文档
> `docs/LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING_2026-08-18.md`：主修复=
> retain 后移（guard 判定后）+ 执行路径 kept_start 重算；防御补强=
> `safe_fold_cut` idx==0 放弃折叠 + `build_request_view` preamble 边界
> 校验；S1-S5 步骤（修复/测试/重建/复验/文档同步）与验收标准已明确。
> 实施待下一窗口；未闭合计数不变（28）。
> 2026-08-18 折叠视图 400 修复实施闭合（处理文档 S1-S5）——S1 代码修复
> （retain 后移 + 执行路径 kept_start 重算 + `safe_fold_cut`→`Option` +
> preamble 校验）；S2 新增 6 项单测、orz-loop 全量 460 通过、fmt/clippy
> 无新增告警；S3 Linux musl 重建三件套（19:39 新构建）；S4 复验
> （`jobs\2026-08-18__19-40-12`）：0 异常、无 400、压缩执行后会话继续
> 102 条事件零失败、6 笔 console 订单→5 组 ACAF 票据、零 permission 拒绝
> ——机制断言全过；reward 仍 0（墙钟内未产出可运行 ELF，任务完成度问题，
> 非机制回归），验证③ reward 项保持开放；`ORZ_DEBUG_VIEW=1` 暂保留登记
> 为常驻诊断（验证③闭合后移除）。ADR-0010 §14.27（v1.27）/ 折叠设计
> §3.5+§8 / BACKLOG 0b / TODO P0-F 已同步。未闭合计数不变（28）。
> 2026-08-18 折叠历史外挂文件设计登记（用户裁决：先设计、不实施）——缓存
> 命中率复验 81.9% 根因=推进触发线不复位（视图内台账累积）致每 1.5 轮重写
> 一次（39 次/57 轮、93.5% miss 由推进+压缩贡献）。定案=单纯外挂文件：
> 已折叠视图改为 `[U0][固定指针消息][最近 1 轮原文]`，推进=每窗口一次纯机械
> 追加摘要行到 `{session_cwd}/.gsa/ledger/current.md`（只增、全局序号、跨压缩
> 连续），指针路径/文本字节级固定；不激活 memory_get/search（用户裁定：不需要
> 记忆本身、接口=固定路径+既有 read_file/grep）、不做模型总结负担、视图内
> 工具结果截断列为第二阶段候选。设计文档
> `docs/LEDGER_FOLD_EXTERNAL_FILE_DESIGN_2026-08-18.md`（取代折叠设计视图内
> 台账块部分；S1-S5 实施草案、验收=复验命中率 ≥90%、风险回滚）。预期 82% →
> ~91-93%（第二阶段冲 95%+）。未闭合计数不变（28）。
> 2026-08-18 折叠历史外挂实施登记（**用户指示优先实施**——「得先处理命中率
> 问题，不然成本太高了」，优先于 P0-F 验证序列）——S1 代码闭合：
> action_ledger 外挂文件（`ledger_file_path`/`build_pointer_message`/
> `external_row_line`/`append_ledger_rows`：尾行续号 + O_APPEND 原子追加；
> `folded_ledger` 语义=固定指针消息，首次推进设置后字节稳定）、
> `advance_fold` 返回 `Option<Vec<ActionLedgerRow>>`（仅新增行、IO 由调用方
> 执行）；agent_loop 推进写文件（失败回滚 fold + 重试不阻塞）、折叠视图尾轮
> 改用 `fold_tail_rounds`（默认 1、`ORZ_FOLD_TAIL_ROUNDS` 可配；压缩 drain
> 尾 `recent_tail_rounds` 不变）；summary marker「历史摘要累积于 <abs-path>」
> + 归档段「折叠视图（冻结快照：外挂指针）」。S2 测试闭合：action_ledger 18 /
> summary 12 / controller 折叠 e2e 2（指针前缀全请求字节稳定 + 外挂文件断言
> + marker 路径提示 + 压缩后续号）；orz-loop 全量 462 通过（-j 1）、fmt 干净、
> clippy 与基线一致（lib 21 / lib test 26）。ADR-0010 §14.28（v1.28）/
> BACKLOG 0c / TODO P0-0c 已同步。S3（Linux musl 重建）/ S4（make-doom-for-
> mips 复验命中率 ≥90%）待验证；未闭合 28 → 29（实施轮入账，验证闭环后
> 29 → 28）。**2026-08-18 B 定案（机械压缩）实施闭合**——S4 复验账单对账
> 定位「未处理的部分」= 压缩摘要调用换前缀重付整段视图 miss（账单口径
> 89.71% <90%）；定案=纯机械压缩（ADR-0010 §14.29）：移除模型摘要调用、
> 五段槽位=黑板 + 固定机械占位（阶段 (c) HA 事实聚合落地前）、存档恒写入、
> 事件 `mode=mechanical`；orz-loop 465 通过、clippy 基线一致、Python verifier
> 214 通过；0c 计数不变（S3/S4 复验闭环后 29 → 28）。
> 2026-08-18 折叠历史外挂二次审查修复闭合登记（用户指示处理全部审查发现）
> ——写失败不再 `continue` 空转（连续 3 次失败禁用折叠 + 新增 v0.2 事件
> `ledger_fold_write_failed`，Schema/TUI 全链同步）、外挂文件仅主车道（检索
> 车道不折叠）、行格式 `[全局序号]`/`轮次` 解耦、`tail_seq` 长行稳健化 +
> 损坏报错、`advance_fold` 落行前 preamble/safe_fold_cut 校验、marker 条件
> 路径提示、`ledger_fold_advance` 增 `view_estimate_after`（触发复位断言）。
> orz-loop 468 / orz-tui 178 / orz-assurance 152 通过、clippy 与基线一致、
> `cargo check --workspace` 通过。ADR-0010 §14.28 / BACKLOG 0c / TODO
> P0-0c / 设计文档已同步。未闭合计数不变（29，S3/S4 验证闭环后 29 → 28）。
> 2026-08-18 状态行缓存纪律 + 订单拒绝步骤语义设计定案登记（用户裁决；纯文档、
> 未实施）——① `[任务状态]` 常驻状态行移出系统提示词，改为变化时追加的尾随
> 用户消息（与预算剩余块同纪律）：根因=console 步骤机每笔订单 receipt 推进步骤
> 致状态行每轮变化、system 摘要 18 次变化、前缀缓存整段失效（命中率 66.5% 对
> 旧批次 95–99%）；② 订单发放期拒绝（registry/contract/target/policy）不再把
> 绑定步骤标 failed——步骤只随执行 receipt（execute/verify）迁移，状态行当前
> 步取首个非 done 与门禁对齐。ADR-0010 v1.25/§14.25；CLASSICAL-EXEC §14；
> FUS-REQUEST-CACHE / CLASSICAL-EXEC-ASSISTANT 条目补注。实施登记见下条。
> 未闭合计数不变。
> 2026-08-18 状态行缓存纪律 + 订单拒绝步骤语义实施闭合登记——orz 子模块
> ba86910：系统组装删 render_status_line（system 完全静态）、controller 增
> sync_status_line_message（状态行变化时追加尾随 user 消息、去重）、
> record_console_receipt 增 mutate_step（仅 execute/verify 失败标 failed，
> 发放期拒绝不迁移步骤）、build_status_line 当前步取首个非 done；orz-loop
> 443 通过、clippy 与基线一致；ADR-0010 §14.25 项 1/2 / CLASSICAL-EXEC §14
> / 实施审计 `docs/audits/GAP_STATUS_LINE_CACHE_STEP_RECEIPT_IMPL_AUDIT_2026-08-18.md`。
> 未闭合计数不变。
> 2026-08-18 动作台账折叠状态化设计定案登记（用户裁决：先设计、不实施）——
> 命中率审计（DeepSeek 控制台 8,845,056/4,279,939 ≈ 67.4%）定位到状态行修复
> 之外的残余主因：`build_collapsed_request` 每请求无状态重算折叠边界（tail=2），
> 每轮请求多一个完整轮次即滑动一次、前缀每轮被重写（复刻模拟首次折叠重合率
> 1.5%），与 v1.9 前缀缓存纪律冲突。定案=折叠点状态化（controller 会话级
> fold_start/fold_cut/folded_ledger 三态，请求视图=preamble+冻结台账+
> messages[fold_cut..] 纯追加；推进=视图估算 ≥128K 机械触发（2026-08-18
> 定案；MRCR 平台期边界、本仓库多文档读取需求）；压缩时旧台账归档+fold
> 重置+摘要输入同源；恢复后重新累积）；**参数定案（用户裁决，统一参数、不做
> 跑分特化）**：压缩普通触发 160K→192K（Flash≈0.81、压缩周期 ≈71 轮）、
> 兜底 200K→256K（Flash≈0.76，超线即强制压缩）；384K 为 prompt 维度质量线，
> 192K/256K 直接比较 <384K 成立，旧「224K=384K−160K」「352K 缓冲」推导
> 作废。384K 有效窗口出处复核=DeepSeek V4 技术报告 arXiv:2606.19348
> Figure 9（MRCR-8-needle/Average MMR，SVG 逐点读取：Flash-Max 8K=0.910/16K
> =0.840/32K=0.870/64K=0.850/128K=0.870/256K=0.760/512K=0.600/1M=0.490；
> Pro-Max 0.900/0.850/0.940/0.900/0.920/0.820/0.660/0.590；128K→256K 为
> 下滑最快区段）；Max 档官方评估窗口 384K（论文 §5.3.1），V4 输入上限实为
> 1M。
> 命中率估算 ≈95.5%、成本约现状 1/4（128K 阈值）。ADR-0010 v1.26/§14.26；
> FUS-LEDGER-FOLD-STATE `current-design`；设计文档
> `docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md`；实施路由=实施前登记
> BACKLOG / TODO。未闭合计数不变。
> 2026-08-18 动作台账折叠状态化实施闭合登记（用户指示优先）——S1 fold 三态
> + 有状态视图 + 推进（action_ledger.rs `LedgerFoldState`/`build_request_view`/
> `advance_fold`）、S2 loop-top 推进触发（视图估算 ≥128K、checkpoint 轮优先）、
> S3 压缩联动（摘要输入与主请求同源、drain 保留起点基于 fold_cut、压缩后三态
> 重置）+ 恢复语义、S4 参数接线（`ORZ_FOLD_TRIGGER_TOKENS` 默认 128K；压缩
> 普通触发 160K→192K、兜底 200K→256K）+ 测试（action_ledger 5 项 + orz-loop
> 循环级 2 项；orz-loop 450 / orz-assurance / orz-tui 178 / orz-bin 全量通过、
> clippy 无新增告警、manifest 1401、仓库门禁 valid）；orz 子模块 a5bea77；
> 实施审计见 `docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md`。
> 未闭合计数不变（设计轮按「实施前登记」口径未计入未闭合总数）。
> 2026-08-18 动作台账折叠状态化二次全面审查收口登记（用户指示处理全部审查
> 发现）——① 设计 §3.5 第 1 步归档缺口补实现（压缩成功分支冻结台账进摘要
> 存档「折叠台账（冻结快照）」段；终止态不落盘=接受边界）；② 新增 v0.2 事件
> `ledger_fold_advance`（fold_start/fold_cut/rounds_folded/view_estimate_tokens/
> agent_role，真实推进才发、防空转不发；Schema/verifier/fixtures/TUI 全链，
> verifier 窗口不变量=fold_start 恒定/fold_cut 严格递增/rounds_folded 不递减/
> context_compressed 重置开新窗）；③ `collapsed_cut` 完整性回退覆盖全部被
> 折叠轮（中途不完整轮不再折叠成 no_result 行）；④ 死代码
> `collapsed_round_count` 删除；⑤ 压缩联动测试修正（脚本 prompt_tokens 与
> 真实视图量级一致，首个 rhythm 触发走成功路径——原测试实际走终止态未被
> 断言暴露）；⑥ 口径/文档修正（session_end 全量估算显式区分、设计 §3.1 补
> per-loop local 注记、审计计数 450→452）；⑦ fixture 生成器回填 console
> 三事件与身份覆盖（既有脱节隐患）。orz 子模块 5274b39；orz-loop 452 /
> orz-tui 178 / orz-assurance 152、Python conformance 15 + journal validation
> 214 通过、clippy 无新增可归因告警、manifest 1401、仓库门禁 valid。未闭合
> 计数不变。详见 ADR-0010 §14.26 / BACKLOG 6g / TODO P1 / 实施审计 §6。
> 2026-08-18 FUS-TOOL-SCOPE-CONTRACT 后续两项闭合登记（P0-E 收尾；本窗口
> 处理）——① list_dir 目录信封：`ListDirContent` 增 listed/ignored/truncated
> 机械计数（ignored=未过滤走−可见走、同过滤语义、200K 封顶；truncated=可见
> 总数−实际渲染）+ 卡片尾部 `(scope: ...)` 脚注；legacy/codex 面保持 None
> 不报；② grep 搜索信封：`files_searched` 机械来源收敛为 v1 `rg --files`
> 探针、扩展为每次完成搜索都运行（含命中，摘要行内嵌 `(searched N files)`；
> 错误路径 None）——`--stats` 跨 rg 版本位置差异污染流式面、`--json` 需
> 重写输出契约，均不采用。orz 子模块 614bb3b；orz-tools 全量 2761 / grep 99 /
> list_dir 60 通过、clippy 无新增告警；ADR-0010 §14.23 / BACKLOG 0a / TODO
> P0-E / 实施审计 / 操作台设计 §12 / 黑板设计 §4 同步。未闭合 29 → 27 项
> （P0-E 后续项全部闭合）。
> 2026-08-21 PUSH→PULL S4 复验阻断与缺口修复登记——7529a71 冻结版
> make-doom-for-mips 单题复验首轮工具轮后第二轮请求 400
> （`reasoning_content must be passed back`）；根因=PUSH→PULL 退役
> REMAINING 尾随消息后暴露 2026-08-04 遗留「工具输出汇总 assistant 文本
> 消息」（`assistant_parts` 冗余副本）为请求末条，DeepSeek thinking 模式
> 对「工具结果后紧跟的 assistant 文本轮」强制回传 reasoning_content
> （API 探针 V1–V7 实测钉死）。修复=退役 `assistant_parts` 汇总消息
> （工具结果已以 Role::Tool 完整落库，协议形态 4→3；顺带每轮输入 token
> 节省）；orz-loop 536 通过 / fmt / clippy 无新增。登记于 ADR-0010
> §14.35 第 11 项 / 设计 §9 / BACKLOG 0e / TODO P0-0e；S3 重建（修复版）
> → S4 make-doom 重跑闭环（job `2026-08-21__01-48-26`，RUN-CLI-6a873e04）：
> 147 请求、journal 命中率 94.45% ≥90%（高于上轮同题 92.18%）、零 400、
> 零 idle 死线、哨兵/stall 全零触发、output 165,644（reasoning 77.8%）、
> 工具轮 226（基线 333）输入增长放缓达成；终态 wallclock 正常耗尽。
> **0e S1-S4 全部闭合，计数 28 → 27**。
> 2026-08-21 方案 C 裁决登记（用户裁决：无必需性、先看当前情况）——0e S4
> 复验闭环后哨兵/stall 全零触发、命中率 94.45%，`REQUEST_MAX_TOKENS` 维持
> 256K 暂不收紧；后续正式跑分中观察哨兵触发率与输出预算，若复发再评估 64K。
> 登记于 ADR-0010 §14.35 第 12 项 / 设计 §6 / BACKLOG 0e / TODO P0-0e；
> 计数不变（27）。
> 2026-08-21 第一轮 5 题冒烟扫描 + zero-chunk 重试窗口 50s→180s 修订登记
> （用户裁决：简单拉长窗口）——冻结版 cf0be20 环境预检全绿后跑 sweep-r1-g1
> 五题 2 通过 3 未过（零 400、零 run_invalidated、12 次哨兵约 ¥2.5）；
> llm-batching 断连归因=约 1 分钟级 DeepSeek 节点抖动在 50s 窗口内 5 次
> 重试耗尽、run 非零退出——节点超时降级无实际作用，`request_retry_window`
> 50s→180s（次数上限 10 不变、双上限先到者止）；S1/S2 已闭合、S3/S4 待续
> （与终端解码重试兜底批次合并一次重建）。登记于 ADR-0010 §14.36 / 设计
> STREAM-RETRY-RHYTHM 修订 / BACKLOG 0d / TODO P0-0d 后续 3；计数不变（27）。
> 2026-08-21 哨兵 fail-fast 化 + 流式中段解码重试设计定稿登记（用户裁决：
> fail-fast 方向有道理、前置证据门确认非架构原因后实施并显式标明；解码
> 兜底按「无完整 tool_calls 即重试（有界）」；先设计、不动作）——跨请求
> 烧 stall 归因=恢复机制副产品（降级梯每请求回 high、成功清零计数器）；
> harness 对照=step 边界有界重试、无降级、空即 step 失败。主案=会话级
> 档位 + 计数单调达限显式 run_invalidated + disabled 档即终止（严格案留
> 对照）；S0 证据门=下批扫描采集哨兵上下文判定。解码兜底=重试判定从
> 「零 chunk」改「无完整 tool_calls」，已见 chunk 的 Transport/解码截断
> 有界重试 1 次。登记于 ADR-0010 §14.37 / ADR-0007 修订注记 /
> [fail-fast 设计](docs/STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md) /
> [解码重试设计](docs/MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md) /
> BACKLOG 0d / TODO P0-0d 后续 4/5；设计轮不动计数（27）。
>
> 当前唯一自然语言设计权威是 [`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。本文件只负责召回和路由，不替代 ADR、Schema、审计结论、测试证据或源代码。
>
> 2026-08-09 整理前的完整索引已保存为 [`CLI_PROJECT_INDEX_FULL_2026-08-09.md`](存档/index/CLI_PROJECT_INDEX_FULL_2026-08-09.md)。历史实施流水只能从该快照回查，不得回填污染当前索引。

## 0. 固定写入格式与维护纪律

### 0.1 权威顺序

发生冲突时按内容类型分别裁决，不得用一种材料冒充另一种材料：

1. 自然语言设计：ADR-0010；新 ADR 只有在显式声明取代关系后才能改变它。
2. 机器合约：已登记 Schema、协议与 verifier；若不能表达 ADR-0010，登记为实现差距，不得反向削弱设计。
3. 实现事实：当前源代码和可复现测试；实现偏离 ADR 时标为 `gap`，不得写成新设计。
4. 审计状态：带日期、范围和证据边界的审计文档。
5. 历史材料：`存档/`；只作 provenance/evidence，不独立产生当前需求。

### 0.2 允许使用的状态

- `current-design`：ADR-0010 或其无新增语义的当前投影。
- `implemented`：有当前源码和验证入口支持，且未登记已知设计偏差。
- `partial`：已存在实现，但与当前设计仍有明确缺口。
- `pending`：当前设计已要求，尚无完整实现或审计闭环。
- `reference`：conformance、fixture、兼容层或历史先例，不拥有生产设计。
- `historical`：已归档，只用于追溯。
- `withdrawn`：明确撤回或已被取代；不得作为当前方案复活。

禁止使用“基本完成”“大致可用”“暂时一致”等不可机械核对的状态词。

### 0.3 标准条目格式

所有主题条目必须使用以下单段格式；同一概念只能有一个 canonical entry：

```text
- **<稳定 ID>** (`<允许状态>`; YYYY-MM-DD)：<一句话定义>。关键词：<别名/旧称/检索词>。入口：<权威文档> / <实现或验证入口>。
```

必要时可追加一个“差距：”或“边界：”句，但不得写实施流水。登记表可以使用表格，但只能映射 ID、状态和入口，不得在表中另写设计裁决。

### 0.4 长度与内容边界

- 每条只承载一个概念，保持一个项目符号和一个段落；正文建议不超过 220 个汉字，入口不超过 5 个。
- 禁止在文件开头追加“最新更新”、下一步、提交号、测试数量、跑分批次、终端日志或长篇修复过程；这些内容写入带日期的审计/评测/实施记录。
- 禁止复制 ADR 的完整参数表、状态机或论证；索引只保留足够检索的定义和精准入口。
- 设计与实现必须分条：设计项使用 `current-design`，不符合设计的实现使用 `partial` 或独立 `GAP-*`。
- 历史条目只保留一个归档路由；详细时间线不得同时出现在主题路由和状态路由。
- 路径必须真实存在；重命名、归档或删除文件时，必须在同一变更中修正入口。

### 0.5 更新检查

每次修改本索引必须同时完成：prior-existence scan、稳定 ID 去重、状态合法性检查、入口存在性检查、旧路径残留检查和 `git diff --check`。若只是一次实现进展且没有改变召回路由，不更新本文件。

---

## 1. 当前权威与治理入口

- **AUTH-ADR-0010** (`current-design`; 2026-08-09)：ORZ 融合 runtime 与 Agent 架构的唯一自然语言设计基线。关键词：融合架构、主 Agent、检索子代理、裁决、冻结。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **AUTH-CURRENT-PROJECTION** (`current-design`; 2026-08-09)：只接收由 ADR-0010 派生且不新增语义的当前设计投影。关键词：current architecture、派生状态机、接口清单。入口：[`architecture/current/README.md`](architecture/current/README.md)。
- **AUTH-V1.1-REVIEW** (`reference`; 2026-08-09)：记录 ADR-0010 v1.1 补写检查、裁决来源和未闭合工程项。关键词：supplement review、遗漏检查、设计复核。入口：[`ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md`](docs/audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。
- **AUTH-FREEZE-AUDIT** (`reference`; 2026-08-09)：记录冻结范围、首批 17 份历史材料及冻结时实现差距。关键词：freeze audit、archive audit、FUS-IMPL。入口：[`ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md)。
- **AUTH-ARCHIVE** (`historical`; 2026-08-09)：集中保存退出当前基线的架构、设计输入、实施记录和索引快照。关键词：存档、旧设计、provenance、historical evidence。入口：[`存档/README.md`](存档/README.md)。
- **AUTH-INDEX-SNAPSHOT** (`historical`; 2026-08-09)：保留索引 v2.0 整理前的完整主题、状态和实施时间线。关键词：旧索引、full index、progress history。入口：[`存档/index/README.md`](存档/index/README.md)。
- **AUTH-BACKLOG** (`reference`; 2026-08-13)：全部未闭合项的统一待办与优先级路由（P0-P3）；设计/审计文档的待办小节只保留指针或历史，明细统一在本文件维护。关键词：backlog、统一待办、优先级、开放项、todo。入口：[`BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md)。
- **AUTH-TODO** (`reference`; 2026-08-14)：面向实施与改动的待办勾选清单，派生自 BACKLOG 未闭合项；优先级与决策门仍以 BACKLOG 为准，本文件只做勾选跟踪。关键词：todo、待办清单、实施跟踪、checklist。入口：[`TODO.md`](TODO.md)。
- **AUTH-ORIENTATION-FORCED-TEMPLATE** (`implemented`; 2026-08-14；2026-08-15 实施闭合)：中立问询强制模板轮的设计权威（ADR-0010 §4.2 正文修订 v1.16 已随实施登记；§14.16 实施登记）。关键词：强制模板轮、checkpoint 轮、问询模板。入口：[`ORIENTATION_FORCED_TEMPLATE_DESIGN`](docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md) / [`ADR-0010 §14.13/§14.16`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`实施审计`](docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md)。
- **AUTH-SESSION-CONTEXT-MONITOR** (`pending`; 2026-08-14；2026-08-16 度量重定)：会话压缩次数监测的设计权威（实施未开始）——2026-08-16 度量由累计 token 改为会话内压缩次数。关键词：压缩次数、会话寿命、总结推荐。入口：[`SESSION_CONTEXT_MONITOR_DESIGN`](docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md) / [`ADR-0010 §14.13/§14.18`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **AUTH-BLACKBOARD-PLAN-EPOCH** (`implemented`; 2026-08-14；2026-08-15 补强)：黑板按 plan epoch 轮换的设计权威（实施已闭合；ADR-0010 §3.6 正文修订已随实施登记；v1.15⑧/⑨ 补强——编号时间戳化、身份一一对应强制、retention 保留最高编号快照；v1.15⑨ 复查遗留闭合（F2/F4-F7/F9/F10））。关键词：黑板、plan epoch、轮换、epoch 快照、压缩解耦。入口：[`BLACKBOARD_PLAN_EPOCH_DESIGN`](docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md) / [`ADR-0010 §14.15`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`实施审计`](docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md)。

- **AUTH-PLAN-FIRST-BLACKBOARD** (`current-design`; 2026-08-15)：模型面重构设计草案——计划-执行分离、黑板化指挥、首轮计划轮硬门、分步计划状态机、模板去人格、AGENTS.md 计划型包裹；双模式定案见 FUS-CONSOLE-DUAL-MODE（console 默认 + direct 受控降级，用户裁决）；2026-08-15 用户定案，进入实施路由（BACKLOG/TODO P0-C）。关键词：plan-first、黑板化、计划轮、分步计划、去人格、AGENTS.md 包裹。入口：[`PLAN_FIRST_BLACKBOARD_DESIGN`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`ADR-0010 §14.17`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。

## 2. 融合架构主题路由

- **FUS-CORE** (`current-design`; 2026-08-09)：采用成熟组件优先的融合架构；可复用成熟能力，但最终控制面和职责边界由 ADR-0010 裁决。关键词：ORZ、自研来源、成熟优先、fusion control plane。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-AGENT-TOPOLOGY** (`current-design`; 2026-08-09)：一个主 Agent、一个内部检索子代理和一个外部检索子代理复用同一 runtime 架构，模型、thinking、transport、工具、上下文、压缩和单会话预算默认一致。关键词：双子代理、同构 Agent、internal retrieval、external retrieval。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-CONCURRENCY** (`current-design`; 2026-08-09)：内外检索角色可双并发，各角色同时最多一个 active instance，全局 `web_search` concurrency 为 1。关键词：dual concurrency、web_search=1、子代理并发。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-RETRIEVAL-MODE** (`current-design`; 2026-08-09)：检索模式显式为 `local_browser`、`framework_fallback` 或 `off`，禁止失败后隐式切换。关键词：LBR、fallback、显式检索模式、source visibility。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`SOURCE_FULLTEXT_VISIBILITY_RULE`](存档/docs/design-inputs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md)。
- **FUS-INFORMATION-SUFFICIENCY** (`current-design`; 2026-08-09)：信息充分性完全机械判定；主 Agent 必须提交结构化 `close` 或 `continue(requirement_delta)`，新需求保持同一子代理 activation active。关键词：insufficient、assessment、parent disposition、contract revision。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-ORIENTATION** (`current-design`; 2026-08-09)：Orientation 仅承担 session-level 中性方向检查，按 7 个逻辑模型轮和 pre-handoff 规则触发。关键词：中立问询、orientation checkpoint、7 轮。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`ORIENTATION_RUNTIME_GUARD_AUDIT`](docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md)。
- **FUS-DIAGNOSTIC-COVERAGE** (`current-design`; 2026-08-09)：单 bug episode 使用机械硬信号按 `2→3→4→5` 递进触发，解决后恢复 2。关键词：Diagnostic Coverage Check、debug coverage、证据身份。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`diagnostic_coverage.py`](assurance/diagnostic_coverage.py)。
- **FUS-COUNTEREXAMPLE** (`current-design`; 2026-08-09)：Counterexample 只在计划或正式结论写入前承担一次性反例检查，不介入普通执行方向。关键词：反例询问、counterexample gate、plan gate。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-STAGNATION** (`current-design`; 2026-08-09)：输出重复、无进展和运行时挂死只归 Runtime Stagnation Guard；不得交给 Orientation。关键词：重复输出、停滞守卫、wallclock、heartbeat、tool timeout。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`RUN_STALL_GUARDS_PLAN`](存档/docs/implementation-history/RUN_STALL_GUARDS_PLAN_2026-08-08.md)。
- **FUS-BUDGET** (`current-design`; 2026-08-09)：主 Agent 与两个检索子代理各自拥有 120 个工具调用轮预算；工具调用轮、问询轮和恢复后轮次均计数；子代理预算按 session 累计（continue 重入不重置，仅激活关闭后新起），主 Agent 每 run 独立起算（v1.2 补写）。关键词：tool round budget、120、session budget、session 累计。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`ADR-0008`](adr/ADR-0008-tool-round-budget.md)。
- **FUS-STATE-RECOVERY** (`current-design`; 2026-08-09)：journal、blackboard、snapshot、compaction、recovery 和子代理 disposition 共同组成可审计状态链。关键词：journal、snapshot、CAS close、compaction、recovery。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-BLACKBOARD-PLAN-EPOCH** (`implemented`; 2026-08-14；2026-08-15 补强)：黑板生命周期按 plan epoch 轮换——plan 区单写者复写（`plan_id`/`plan_epoch`）；仅新 plan epoch 批准触发原子轮换（归档旧 epoch 快照 → 清 edits/tool_actions/exec → 复写 plan）；gate_log/白名单/检索分区不清；压缩不再清黑板（废止 v1.14 窗口滚动）；路径槽=本 epoch 增量；marker 带 `plan_epoch`；`blackboard_read` 跨 epoch 走归档（`epoch` 参数）；中立问询/DC 锚点跨压缩稳定。**实施已闭合（2026-08-14）**——PlanApproved payload 增 `plan_epoch`（Schema/fixtures/journals 同步）、`with_plan` 带身份并原子轮换（同 plan_id 修订不清板）、`.gsa/blackboard/epoch-<n>.json` 快照（批准/修订持久化当前 epoch + 轮换归档旧 epoch，有界重试）、`blackboard_read` 增 `epoch` 参数、压缩不再清板 + marker/路径槽/恢复接线、archive dir 装载最新 epoch 快照、retention 7 天清扫。**2026-08-15 v1.15⑧ 补强**——plan_epoch 时间戳单调编号（unix 毫秒基底、`max(now_ms, 磁盘 max+1)`）、身份不变式强制（同 plan_id 同 epoch、新 plan_id 严格递增，`rotate_to_plan`/`try_with_plan` 返回错误、`with_plan` fail-fast）、retention 保留最高编号快照（恢复入口）。**2026-08-15 v1.15⑨ 复查遗留闭合**——F2 原子写盘（临时文件+rename、降序回退加载）、F4 跨进程 `.claim-<n>` 原子占号、F5 归档目录单一来源、F6 非法 epoch 显式报错、F7 归档失败入事件面（v0.2 `epoch_archive_write_failed`）、F9 `persisted_at` 更名、F10 设计 §5 措辞对齐。关键词：plan epoch、黑板轮换、epoch 快照、压缩解耦、原子轮换。入口：[`BLACKBOARD_PLAN_EPOCH_DESIGN`](docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md) / [`ADR-0010 §14.15`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`BACKLOG 6e`](docs/BACKLOG_AND_PRIORITIES.md) / [`实施审计`](docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md)。
- **FUS-WINDOWS-BOUNDARY** (`current-design`; 2026-08-09)：Windows process/runtime spike、产品事故记录和精选案例库分开治理；现阶段只保留证据边界，不提前宣称案例闭环。关键词：Job Object、Windows compatibility、incident、case library。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`docs/incidents/`](docs/incidents/) / [`docs/cases/`](docs/cases/)。
- **FUS-UI-BOUNDARY** (`current-design`; 2026-08-09)：保留 Toolbar 和只读 session/run-history 投影，产品边界不扩展为完整 IDE。关键词：TUI、Toolbar、readonly session projection、Codex app-server、VS Code lifecycle。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-COMPONENT-REGISTER** (`partial`; 2026-08-09)：register 文件框架已建（65 组件全 `audit_required`，不得从 crate 名/编译推断采用档位），逐 crate 审计未开始。关键词：component matrix、crate ownership、mature adoption、audit_required。入口：[`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`fusion-component-register-v0.1.yaml`](upstream/fusion-component-register-v0.1.yaml) / [`AUTH-V1.1-REVIEW`](docs/audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。
- **FUS-CONTROL-FABRIC** (`current-design`; 2026-08-09)：跨信任边界控制事件与外部效果动作必须持一次性 HMAC 票据（ACAF）；签发器独立进程窄 IPC；三运行模式（正常审批/无运行自动/临时沙盒运行），不做完全授权。关键词：ACAF、ControlTicket、SandboxLease、PromotionPermit、签发器、三模式。入口：[`ADR-0011`](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。
- **FUS-SOURCE-WEIGHTING** (`current-design`; 2026-08-12)：检索来源质量——web_search（framework_fallback）三层结构：机械来源梯队（白名单=政府/机关单位 1.1、命中直接采纳；白名单外默认 1.0；劣质源 0.7，初始含 CSDN/知乎/百家号/B 站个人专栏/微博/独立新闻媒体/自媒体新闻与财经号/小站）+ 选择性原文核验（仅 web_search，web_fetch 抓候选原文）+ 子代理模型加权标注（v0 标注排序不拦截）；local_browser 直接分级加权（第一层+第三层，无第二层）；二存一禁止混用；共享判定器进 evidence ledger/visibility。关键词：来源加权、白名单、劣质源、原文核验、二存一、framework_fallback、local_browser。入口：[`ADR-0010 §3.7 条 12`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`RETRIEVAL_SOURCE_WEIGHTING_DESIGN`](docs/RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md) / [`SOURCE_QUALITY_SEED_LISTS`](docs/SOURCE_QUALITY_SEED_LISTS_2026-08-12.md)。
- **FUS-TOOL-PROBE** (`implemented`; 2026-08-13)：主 Agent 工作工具机械可用性探针——v0.2 单一探针面（A/C 面撤销：本轮模型可见 = 机械链路完整 ∩ 会话声明集，仅名称、不设免检面）+ 两态中性判定 + `tool_availability_check` 翻转事件 + 每模型请求前重算与最小上一轮映射（翻转才发事件；调用即探针回写限主/grill 车道）+ 兜底消息中性化；P0-A 批次（步骤 1-7，ADR-0010 §3.5 修订 = v1.8）与 P0-A-2 均已闭合——全部 23 个工作工具统一探针（read/write/storage/goal/plan/待处置激活/terminal/lsp/memory/image/video/MCP 链），投影=探针完整集∩声明集+非工作工具，事件/verifier/fixtures 同步。边界：orz-host 可选后端能力（lsp/memory/图像/视频/MCP）默认未接线，探针按 fail-closed 移除，接线时须翻转 orz-host 能力访问器。关键词：tool probe、单一探针面、机械链路、列表投影、工具可用性、tool_availability_check、翻转事件、兜底消息中性化。入口：[`TOOL_AVAILABILITY_PROBE_DESIGN`](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [`BACKLOG_AND_PRIORITIES`](docs/BACKLOG_AND_PRIORITIES.md) / [`ADR-0010 §14.8`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`P0-A-2 实施审计`](docs/audits/GAP_TOOL_PROBE_V02_SINGLE_FACE_IMPL_AUDIT_2026-08-13.md)。
> - **FUS-REQUEST-CACHE** (`implemented`; 2026-08-14；2026-08-15 实施闭合)：缓存与上下文成本收敛——保持 v1.8 探针可见性机制（工具集变化=真实状态变化，接受前缀 miss，第二轮自动恢复）；否决 per-window 探测 / 预热轮 / 工具层后置渲染 / 工具层 1K 压缩；三项已实施：请求 header 变化留痕（v0.2 `request_header_change`：system+tools+config 三摘要、initial/change、`agent_role` 车道区分、`previous_header_sha256`）、探针准确性与稳定性（验证器翻转↔header 交叉核对 + `assurance/probe_accuracy_audit.py` 假完整/假不完整审计）、单轮工具结果注入预算（默认 50K、`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 可调、按轮累计、超限无 ToolStarted 拒批 + offset 续读）+ 提示词策略化读取；短期仅 DeepSeek OpenAI 兼容面。2026-08-17 注：console 默认面注册板块/工具栏为黑板数据、经 `blackboard_read` 取回，不参与 tools 摘要，工具栏刷新不构成前缀 miss 源（ADR-0010 v1.19）。关键词：request header、前缀缓存、注入预算、策略化读取、cache、探针准确性。入口：[`ADR-0010 §14.9`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`TOOL_AVAILABILITY_PROBE_DESIGN`](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [`BACKLOG 6b`](docs/BACKLOG_AND_PRIORITIES.md) / [`实施审计`](docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md)。
- **FUS-BLACKBOARD-READ-CACHE-COST** (`implemented`; 2026-08-19)：黑板读取缓存成本处理——S1/S2 渲染瘦身已实施（actions 结果板固定形态行去 response JSON（order_id/ok/step/code/trace_id，缺失回退 `?`）、exec 行截断 200 字符 + 段总长 4K 上限；registration/order 板不变；orz-loop 479 通过）；全面审查发现 F1=console 面动作详情不可回查（assistant.trace 不在直接工具面、其 receipt 响应同样被瘦身隐藏）；用户裁决=方案 B 按需点读（`blackboard_read` 新增可选 `receipt_id`：单条 receipt 完整内容、8K 字符上限截断+指针、支持 epoch 归档点读、非法/未找到显式报错、无 receipt_id 整段逐字节恒定；工具定义增量扩展、黑板数据面/事件面不动；路由 S1 代码→S2 测试→S3 重建→S4 复验；**2026-08-19 用户放行后方案 B S1 代码 + S2 测试已实施**（epoch.rs 点读分支 + render_receipt_point_read（`RECEIPT_DETAIL_MAX_CHARS=8_000` 截断+指针）、controller 参数解析与透传、工具定义增量扩展；单测 5 项 + 工具级 4 项；orz-loop 488 通过 / fmt 干净 / clippy 与基线一致（lib 21 / test 26）；S3 重建 → S4 复验待续））；（阶段 2 可选）`since` 扩展 actions + 读取频率提示词引导；零模型、折叠/压缩阈值不动；预期单次读取 17–32K→1–3K token、命中率 86%→95%。来源：S4 换题复验归因（8/8 大 miss 尖峰紧跟 blackboard_read）。关键词：blackboard_read、分区渲染瘦身、response JSON、按需点读、receipt_id、工具结果注入、缓存命中、前缀缓存。入口：[`BLACKBOARD_READ_CACHE_COST_DESIGN`](docs/BLACKBOARD_READ_CACHE_COST_DESIGN_2026-08-19.md) / [`ADR-0010 §14.31`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`BACKLOG 0c`](docs/BACKLOG_AND_PRIORITIES.md) / [`TODO P0-0c`](TODO.md)。
- **FUS-COMPACTION-REDESIGN** (`implemented`; 2026-08-14)：压缩机制重设计——有效窗口 384K、160K 普通触发 / 200K 兜底（384K − 160K completion − 24K 余量）；工具调用记录每轮机械坍缩（零模型调用、动作台账行、指针/digest、请求视图）；五段模板摘要（目的/计划/变动文件路径/注意事项/后续衔接，≤17K 字符，derived_unverified + digest，冷却 2 模型轮（v1.14），摘要链只进审计）；滚动单 marker 附回查清单；推翻零模型摘要与仅最终答案间隙裁决；**S1-S6 已全部闭合**——D2-2 恢复预检截断（`context_recovery_truncated` 事件 + 完整侧车审计副本）、D3-1 marker/白名单恢复保留、动作台账机械坍缩（配对纪律 + 有界尾部）、五段模板摘要接线（160K/200K/2 模型轮/5K/0.6、重做 ≤3、summary_incomplete 终止态 + fallback 机械截断、`.gsa/compaction/` 存档 + 7 天 retention、context_compressed v0.2 payload + verifier/fixtures、TUI 投影；检索子代理同构触发，摘要调用不计工具轮/orientation 轮）、S5 审查修复（守卫失败重试 3 次后强制压缩 `guard_failed`、会话结束压缩 `session_end` 固定进 sidecar、存档写失败 `archive_write_failed`、退化守卫 300 等效字符 + CJK 折算、黑板 edit 窗口滚动、120s 摘要超时、路径槽 Top-40 双上限）、S6 复查对齐（session_end 160K 阈值补写、复用边界内联化、终止态 marker 占位 digest 改显式未生成、fixtures 生成器回写、chars/2 中文低估登记 P1 校准）。**v1.15 注**：『黑板 edit 窗口随压缩滚动』已由用户裁决废止，黑板改按 plan epoch 轮换（FUS-BLACKBOARD-PLAN-EPOCH `implemented`，实施已闭合；当前代码为 v1.15 行为）。**v1.29 注**（2026-08-18，ADR-0010 §14.29）：压缩机械模式 B 定案——五段模板摘要 LLM 调用退役、零模型调用、注意事项/后续衔接为固定机械占位（阶段 (c) HA 结构化事实聚合落地前由主模型按 marker 回查入口自行承接）、事件 `mode=mechanical`。**v1.30 注**（2026-08-19，ADR-0010 §14.30）：D1=(c) HA 结构化事实聚合设计定稿（纯文档登记、未实施；2026-08-19 用户放行后 S1/S2 已实施闭合，S3/S4 待验证，见头部登记）——注意事项槽=HA 结构化事实聚合（助理层唯一新增输出；controller 已机械写入的 `plan.steps` Failed/Blocked + `exec.errors` 最近 5 + `actions.results` 失败 receipt 最近 3；≤3K 截断+指针；压缩内部失败继续走 marker 标注）；后续衔接槽不交助理层（固定中性占位 + 回查入口，由主模型自行判断）；零模型调用、五槽 17K 上限、schema 不变。关键词：工具记录坍缩、动作台账、模板摘要、summary chain、derived_unverified、回查清单、384K、160K、200K、HistoryThenSteps、D2-2、D3-1、summary_incomplete、guard_failed、session_end、archive_write_failed。入口：[`CONTEXT_COMPACTION_DESIGN`](docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md) / [`ADR-0010 §3.6`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`ADR-0010 §14.10`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`ADR-0010 §14.14`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`P0-D 实施审计`](docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md) / [`BACKLOG_AND_PRIORITIES`](docs/BACKLOG_AND_PRIORITIES.md)。
- **FUS-RETRIEVAL-MECH** (`implemented`; 2026-08-13)：检索侧机械控制设计已定稿、实施已放行——web_fetch 候选机械计数与计数反馈（`ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8，per-activation 累计 + 侧车持久化、去重后 URL 计数、超限无 ToolStarted 拒绝 + 熔断同面、`tool_completed` 计数字段）+ 机械预筛（保守净化候选池、档位作排序信号不拦截）+ 引用纪律机械化（输出级 `[来源]` 标记校验，交付前机械降级）；**B-1 与步骤 2-6 已全部闭合**——web_search citations 经 `ToolResult.structured` 接缝透传进 loop（证据账本 `candidate_urls` + `raw_source_refs` 镜像，Schema/verifier 先行），web_fetch 候选计数门禁落地，步骤 4 browser_read 范围/模式参数（mode=full/preview/keywords + keywords 数组；preview 4K、keywords 16/64/3/160/12K 常量）与第二段计数域复用落地（与 web_fetch 共用同一 activation 计数域与 cap、拒绝码 browser_read_candidate_*、证据按 mode 降级、ACAF e2e browser_read 场景迁至子代理车道），机械预筛落地（canonical/host 去重、bad_url/login_wall/redirect_chain 移除、tier/weight + 词法相关性排序；`candidate_urls` 升级为预筛后保留池，`candidate_pool`/`prefilter_log` 契约先行），步骤 5 输出级引用校验落地（`[来源: ...]` 结构化解析 + ledger/path:line/URL/文档绑定 + §3.7.5 上限，失败机械降级块 + `citation_validation` 事件，主车道证据收集 + 主证据判定，v0.2 事件枚举 42→43），步骤 6 提示词相应缩短与测试更新落地（主提示词引用纪律缩减为标记格式+verifier 机械校验、检索提示词移除候选 ≤5 软约束改指机械预算反馈、来源加权/引用规则去冗余、prompt 测试同步）。关键词：web_fetch 计数、计数反馈、browser_read 模式、全文/预览/关键词、browser_read_candidate、机械预筛、候选池净化、candidate_pool、prefilter_log、引用校验、citation_validation、B-1、步骤 2、步骤 3、步骤 4、步骤 5、步骤 6、提示词缩短、candidate_urls、ORZ_WEB_FETCH_CANDIDATE_CAP。入口：[`RETRIEVAL_MECHANICAL_CONTROLS_DESIGN`](docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md) / [`步骤 6 实施审计`](docs/audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md) / [`步骤 5 实施审计`](docs/audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md) / [`BACKLOG_AND_PRIORITIES`](docs/BACKLOG_AND_PRIORITIES.md) / [`ADR-0010 §3.7.9`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-DSH-BORROW-REVIEW** (`reference`; 2026-08-14)：DeepSeek Harness 借鉴复核结论——orz 整体即薄层（除成熟底座外的一切，单一二进制，底座可较简单切换）；个人开发者无插件生态，不支付子系统化复杂度；A（Windows ACL 沙箱）挂起、B（文件观察策略）收编为 `search_replace` 动作契约规则随小样 2、C（工具结果裁剪）收编为纯函数随 S2/50K；其余层已覆盖或不适配。关键词：DSH、薄层、底座切换、借鉴复核、A/B/C 裁决。入口：[`DEEPSEEK_HARNESS_BORROW_RESEARCH`](docs/DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md) / [`DSH_BORROW_DESIGN_DECOMPOSITION`](docs/DSH_BORROW_DESIGN_DECOMPOSITION_2026-08-14.md) / [`ADR-0010 §14.13`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-ORIENTATION-FORCED-TEMPLATE** (`implemented`; 2026-08-14；2026-08-15 实施闭合)：中立问询升级为强制模板轮——触发点下一安全动作间隙明确暂停、独立无工具 checkpoint 轮、模型填写问询模板后才恢复动作；Orientation 与 DC 两族共用（主车道）；目的=拉回注意力防跑偏（强制表达、不验证诚实）；缓解必做（`progress_evidence` 存在性交叉校验、`gather_evidence` 必填缺失面）。**实施已闭合（2026-08-15）**——ADR-0010 v1.16（§4.2 正文修订 + §14.16）、v0.2 `checkpoint_response` 事件（Schema/verifier/fixtures/TUI）、无工具 checkpoint 轮/一次重填/降级兜底/pending 单槽（Orientation 优先、主车道；检索车道不变）、证据身份交叉校验 + gather_evidence 必填缺失面。关键词：强制模板轮、checkpoint 暂停、问询模板、拉回注意力。入口：[`ORIENTATION_FORCED_TEMPLATE_DESIGN`](docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md) / [`ADR-0010 §14.13/§14.16`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`BACKLOG 6c`](docs/BACKLOG_AND_PRIORITIES.md) / [`实施审计`](docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md)。
- **FUS-SESSION-CONTEXT-MONITOR** (`pending`; 2026-08-14；2026-08-16 度量重定)：会话压缩次数监测——度量=会话内压缩次数（`context_compressed` reason∈rhythm/fallback 计数、`session_end` 不计、一次一计）；≥2 次机械提醒、≥3 次机械总结推荐（可配、默认待校准；2≈旧 384K、3≈旧 500K）；最简实现=阈值到达的最后一轮模型输出末尾机械附言（附次数）；headless 仅日志；TUI/journal 事件为 beta 前可选；与压缩独立；chars/2 token 估算校准项废止。关键词：压缩次数、会话寿命、总结推荐、新窗口、rhythm、fallback。入口：[`SESSION_CONTEXT_MONITOR_DESIGN`](docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md) / [`ADR-0010 §14.13/§14.18`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`BACKLOG 6d`](docs/BACKLOG_AND_PRIORITIES.md)。
- **FUS-RECOVERY-TOOL-OUTCOME** (`pending`; 2026-08-14，条件触发)：崩溃恢复工具结果词汇——恢复中断轮次补合成 Tool 消息（`TOOL_NOT_STARTED` 工具从未开始 / `TOOL_OUTCOME_UNKNOWN` 结果未知）+「只重试只读/幂等操作、验证副作用或询问」指引；出现恢复面 400 或副作用未知证据时实施（单点修复，不建子系统）。关键词：TOOL_OUTCOME_UNKNOWN、TOOL_NOT_STARTED、副作用未知、崩溃恢复。入口：[`DEEPSEEK_HARNESS_BORROW_RESEARCH` 附录候选 1](docs/DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md) / [`BACKLOG 条件触发`](docs/BACKLOG_AND_PRIORITIES.md)。

- **FUS-PLAN-FIRST-MODEL-SURFACE** (`implemented`; 2026-08-15；2026-08-16 阶段 A/B/C 全部闭合)：主模型面=黑板读写 + 只读核查（`blackboard_read`/`assistant.trace`/工作区只读），执行/变更/shell/子代理/检索全部经助理层订单；首轮计划轮只暴露黑板读取 + `plan_write`（阶段 A）；注册板块=探针投影唯一事实源（阶段 B）；console 默认面 + direct 受控降级（阶段 C：投影收敛 + 调用面门禁 `console_mode_tool_denied`）。关键词：模型面、黑板动作栏、只读核查、助理层全承接、首轮计划轮。入口：[`PLAN_FIRST_BLACKBOARD_DESIGN`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`CLASSICAL-EXEC-ASSISTANT`](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [`阶段 C 审计`](docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md) / [`ADR-0010 §14.17`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-CONSOLE-DUAL-MODE** (`implemented`; 2026-08-15；2026-08-16 阶段 C 实施闭合)：操作台双模式——console 默认（黑板读写 + 只读核查，执行/变更/shell/子代理/检索全经助理层订单），direct 受控降级（连续 3 次助理层故障面失败→无工具询问轮→模型选择后切换并写 `console_mode_transition`+gate_log；direct 动作带 transition_id+trace_id 盖章；权限/ACAF/模式门不变；计划门约束 console 订单，direct 为有记录的例外，`console_step_done` 需证据置 done；run 结束复位）；放弃「直接执行面永久移除」。关键词：双模式、console、direct、受控降级、3 连败、console_mode_transition。入口：[`PLAN_FIRST_BLACKBOARD_DESIGN`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`ADR-0010 §14.17`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`阶段 C 审计`](docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md)。
- **FUS-CONSOLE-POLICY-DENIAL** (`implemented`; 2026-08-15；S3 前置实施闭合)：结构化策略拒绝机制——拒绝路径（权限/ACAF/模式门；taint 预留）在 `run_host_tool` 边界返回 `ToolResult.policy_denial = {source, code, reason}`；console 适配层仅按结构化信号映射 `step=policy`，删除稳定输出前缀字符串判定；ToolCompleted 增可选 `policy_denial`（Schema/verifier/fixtures 先行）。边界：taint 路径预留。关键词：policy_denial、结构化拒绝、策略拒绝、前缀判定退役。入口：[`CLASSICAL-EXEC-ASSISTANT §7`](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [`S3 前置审计`](docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md) / [`TODO P0-C`](TODO.md)。
- **FUS-PLAN-STEP-GATE** (`implemented`; 2026-08-15；2026-08-16 阶段 C 实施闭合)：计划=有序分步数组；步骤状态机 `pending → in_progress → done(receipt_id) | failed(receipt_id)`（`StepStatus` 状态机化 + 旧归档 serde 兼容）；下一步订单需上一步 receipt 机械放行（`step_not_done`），direct 为有记录例外（`console_step_done` 证据门）；防惯性幻觉。关键词：分步计划、步骤状态机、receipt、硬门、惯性幻觉。入口：[`PLAN_FIRST_BLACKBOARD_DESIGN`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`ADR-0010 §14.17`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`阶段 C 审计`](docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md)。
- **FUS-AGENTS-MD-PLAN-WRAPPER** (`implemented`; 2026-08-15；2026-08-16 实施闭合)：AGENTS.md 注入时机械包裹固定计划型执行框架（用户内容之前）；唯一机制，不做规范模板；阶段 A 已实现（`<plan_first_framework>` 固定前缀，主/子代理同一入口）。关键词：AGENTS.md、计划型包裹、plan-first、机械包裹。入口：[`PLAN_FIRST_BLACKBOARD_DESIGN`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`阶段 A 审计`](docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md) / [`ADR-0010 §14.17`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-PROMPT-DEPERSONALIZE** (`implemented`; 2026-08-15；2026-08-16 实施闭合)：删除除机械契约外全部人格化内容（主/子代理模板、apply-patch 模板、Orchestrator body）；子代理 `<persona>` 模板段退役、persona 指令不再注入系统提示词（roles 保留；`SubagentPersona` 配置解析保留为外部 shell 兼容层，审计边界登记）；XOR 模板重生成 + 无人格关键词渲染测试锁定。关键词：去人格、模板、persona 退役、机械契约。入口：[`PLAN_FIRST_BLACKBOARD_DESIGN`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`阶段 A 审计`](docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md) / [`ADR-0010 §14.17`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
- **FUS-LARGE-FILE-READ-CONTRACT** (`implemented`; 2026-08-17)：大文件读取契约——读取工具契约有界返回（超过粗门默认 16KB、可配 8–32KB 的文件返回读取句柄信封 path/size/encoding/content_sha256/可用范围/有界预览 ≤4KB/truncated/offset 续读指针，不返回全文；精门=50K 注入预算兜底；小文件保持全文一次返回）；语义适配留模型、助理层只提供机械原语（引用 + 范围读）；模型以 read_file(offset)/grep 结构化续读（提示词策略化读取落成工具契约，对齐 pdf_read document_id+page_range 先例）；黑板/结果栏只放指针不放内容本体（内容留盘上/证据区，维持不新增自由随记区）；**实施已闭合（2026-08-17，orz 172b14e）**——GrokBuild read_file 文本路径信封 + 有界预览 + offset 续读，SKILL.md/`skills` 全量豁免，FileTooLarge 文本路径被取代保留为防御兜底。关键词：大文件读取、读取句柄、有界返回、结构化续读、offset、内容指针、黑板只存指针。入口：[`CLASSICAL-EXEC-ASSISTANT §11`](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [`PLAN_FIRST_BLACKBOARD §4`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`ADR-0010 §14.22`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`实施审计`](docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md) / [`BACKLOG 6f`](docs/BACKLOG_AND_PRIORITIES.md)。
   **2026-08-17 全面检查修复**：信封空窗口/越界 offset 语义收口（past-EOF 无续读指针、范围内空窗口从请求行续读、最后一行行内截断 offset=None）；`[toolset.read_file]` 配置节端到端接线（coarse_gate_bytes 经 orz-config 装载注入、优先于 env；orz 7c4a99e + bd8d485，ADR-0010 §14.22 项 4）。
- **FUS-TOOL-SCOPE-CONTRACT** (`implemented`; 2026-08-17；2026-08-18 读/搜/列三族全部闭合)：工具契约家族——「范围/截断必须机械报告」：grep 返回结构化搜索信封（resolved root/files_searched/match_count/truncated；结局三型分型：searched>0 有匹配 / searched>0 无匹配=真无匹配 / searched=0=范围空显式报过滤类别，绝不叫 "No matches found"；非零退出 + stderr 非空先显式报错）+ 搜索范围语义显式化（保留 rg 默认 + 提供 --no-ignore/--hidden 开关，容器冒烟定案）；机械来源=定稿 `rg --files` 探针全结局运行（2026-08-18 扩展：命中路径亦返回 files_searched，摘要行内嵌 searched N files；弃用 --stats——rg 15 stdout / 旧版 stderr 位置差异污染流式面，--json 需重写输出契约，均不采用）；读取信封见 FUS-LARGE-FILE-READ-CONTRACT；**目录信封 2026-08-18 闭合**——list_dir 增 listed/ignored/truncated 机械计数（ignored=未过滤走−可见走、同过滤语义、200K 封顶；truncated=可见总数−实际渲染）+ 卡片尾部 (scope: ...) 脚注；legacy/codex 面不报。关键词：搜索信封、目录信封、范围报告、空结果语义、结局分型、ignore 语义、工具契约、files 探针。入口：[`ADR-0010 §14.23`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`CLASSICAL-EXEC-ASSISTANT §12`](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [`实施审计`](docs/audits/FUS_TOOL_SCOPE_CONTRACT_GREP_IMPL_AUDIT_2026-08-17.md) / [`BACKLOG 0a`](docs/BACKLOG_AND_PRIORITIES.md) / [`TODO P0-E`](TODO.md)。
- **FUS-BENCHMARK-FULL-EXEC** (`pending`=实施完成待验证; 2026-08-18)：Benchmark 完全体执行面——TB2 跑分按任务合规放开 shell/网络（orz 完全体），但 shell 不开放为模型直接工具：console 默认面仍只读+下单，执行全经助理层订单（`workspace.run_terminal`，与 run_tests 同构）。三层使能=权限 `Benchmark{allow_shell,allow_network}` 两轴参数化（默认 fail-closed）+ 探针 `ToolPolicy::BenchmarkFull` + 注册表终端动作；CLI `--allow-shell`/`--allow-network`（须与 `--allow-write` 同用，否则 exit 2）；适配器按任务 network_mode 透传（实施取有效 agent-phase network_policy，89 题全 PUBLIC）。ACAF 票据/预算/墙钟/事件审计不变；「放开」=策略允许面。**2026-08-18 实施完成（用户指示：暂不测试）**——orz 子模块见 [TODO P0-F](TODO.md)；待验证：orz 测试/clippy、Linux musl 重建、make-doom-for-mips 复验、2–3 题交叉、89 题分批。关键词：Benchmark 完全体、allow_shell、allow_network、BenchmarkFull、workspace.run_terminal、终端动作、TB2。入口：[`BENCHMARK_FULL_EXEC_DESIGN`](docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md) / [`ADR-0010 §14.24`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`CLASSICAL-EXEC-ASSISTANT §13`](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [`PLAN_FIRST_BLACKBOARD §4`](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md) / [`BACKLOG 0b`](docs/BACKLOG_AND_PRIORITIES.md)。

## 3. 当前实现与符合性路由

- **IMPL-RUST-RUNTIME** (`partial`; 2026-08-09)：`orz/` 是融合架构的 Rust 生产实现工作区（2026-08-15 登记为父仓库 git 子模块，SilverWhite/CLI feat/fusion-architecture），冻结审计已确认其尚未完全符合 ADR-0010。关键词：orz-loop、orz-host、orz-assurance、orz-bin、orz-tui。入口：[`orz/`](orz/) / [`AUTH-FREEZE-AUDIT`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md)。
- **IMPL-PYTHON-REFERENCE** (`reference`; 2026-08-09)：`assurance/` 主要承担 conformance、Schema authority、fixture、审计和窄兼容角色，不自动拥有融合 runtime。关键词：Python reference spec、conformance suite、assurance。入口：[`PYTHON_REFERENCE_SPEC_CONTRACT`](architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md) / [`assurance/`](assurance/)。
- **IMPL-RUN-EVENT-SCHEMA** (`implemented`; 2026-08-10)：v0.2 事件体系已建（8 个机制事件 + envelope + 双轨 verifier + §4.4 链校验 + mode/result/restore 跨层规则）；Rust producer 整轨写 v0.2，disposition/close/DC/result/restore/mode producer 已产，12 个真实 journals 捕获。关键词：run-event、event schema、receipt、replay、v0.2。入口：[`run-event-v0.1.schema.json`](runtime/run-event-v0.1.schema.json) / [`run-event-v0.2.schema.json`](runtime/run-event-v0.2.schema.json) / [`run_event_journal_validation.py`](assurance/run_event_journal_validation.py)。
- **IMPL-DEEPSEEK-TRANSPORT** (`partial`; 2026-08-09)：默认模型族为 DeepSeek，当前配置为 V4；主/子代理 transport、重试和 thinking 同构复核已闭合（2026-08-16——三实例共享单一 `DeepSeekTransport`/`RetryPolicy`/`ThinkingMode::EnabledMax`、160K 单一常量，`-p` 预检轮为文档化请求级覆盖）；仍 partial=DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7）。关键词：DeepSeek V4、deepseek-v4-flash、transport retry、thinking、同构复核。入口：[`DEEPSEEK_ADAPTER_CONTRACT`](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md) / [`ADR-0007`](adr/ADR-0007-transport-retry-policy.md)。
- **IMPL-WRITE-PLACEMENT** (`implemented`; 2026-08-09)：工作区、本体状态和系统必要状态按 ADR-0009 分域，Grill 使用独立只读审计链。关键词：GROK_HOME、write placement、Grill、read-only。入口：[`ADR-0009`](adr/ADR-0009-write-placement-policy.md) / [`WRITE_PLACEMENT_AND_GRILL_DESIGN`](docs/WRITE_PLACEMENT_AND_GRILL_DESIGN_2026-08-08.md)。
- **IMPL-GLOBAL-REVIEW** (`implemented`; 2026-08-09)：显式 Global Review Mode 负责激活全局审查义务，其 receipt 不冒充最终审查结论。关键词：global review、activation receipt、L1-L7。入口：[`global_review_mode.py`](assurance/global_review_mode.py) / [`GSA_GLOBAL_REVIEW_RECORD`](docs/GSA_GLOBAL_REVIEW_RECORD_2026-08-01.md)。
- **IMPL-CONTROL-FABRIC** (`partial`; 2026-08-12)：ACAF 四切片实施独立于 Phase C；**Slice 1（签发器 v1 + 控制事件票据）已闭合**，**Slice 2 两阶段已闭合（四类动作票就位：file_write 全链 + credential_read 机制先建零接线 + command_exec/network 同型扩展，全程影子模式）**，goal/policy 接线（GAP-DENIAL-POLICY-REVISION 消费面）已闭合，**Slice 2 fail-closed 机制已实施（D-12~D-16）且 2026-08-16 生产翻转已闭合**——fail-closed 改默认强制（未设置即强制、显式 0/false/no/off 影子、非法值 exit 2），CLI run / ACP stdio / TUI 三入口全部接线（ACP/TUI 此前未挂签名器客户端，随翻转补齐），核查清单 ⑦⑨⑩⑪ 收口（web_search 显式排除、host 稳定面、URL 规范化等价、重定向逐跳 gate），新增 `orz-acaf-provision` 供应工具与 `scripts/orz_acaf_run.ps1` 启动链；**Slice 3/4 待实施**。关键词：ACAF 切片、签发器、fail-closed、shadow mode、ControlTicket、resolved_target_sha256。入口：[`ADR-0011`](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md) / [`GAP_ACAF_SLICE2A_IMPL_AUDIT`](docs/audits/GAP_ACAF_SLICE2A_IMPL_AUDIT_2026-08-12.md) / [`GAP_ACAF_SLICE2B_IMPL_AUDIT`](docs/audits/GAP_ACAF_SLICE2B_IMPL_AUDIT_2026-08-12.md) / [`GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT`](docs/audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md) / [`生产翻转审计`](docs/audits/GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md)。

### 3.1 已登记实现差距

- **GAP-ACAF-HARNESS-PASSTHROUGH** (`implemented`; 2026-08-17；**2026-08-17 用户裁决：跑分保持 ACAF 强制开启**）：ACAF fail-closed 生产默认强制后，TB2 harbor 适配器不向任务容器转发签发器配置（`ORZ_ACAF_MANIFEST`/`ORZ_ACAF_KEYSTORE`/`ORZ_ACAF_FAIL_CLOSED`），`orz --real` 拒启；临时解阻=影子模式透传；用户已裁决正式跑分不接受影子模式，实施=容器内供应 manifest/keystore/signer（适配器透传 + 容器挂载 + 移除 `ORZ_ACAF_FAIL_CLOSED=0` 覆盖 + 冒烟验证）。**2026-08-17 闭合**：前置设计变更——新增 `file-0600-installation` 文件型安装密钥库（ADR-0010 §14.21 项 4 / ADR-0011 §5）；orz 子模块 e8274e1 + schema/fixture + 仓库门禁 valid；适配器 install 容器内 `/etc/orz-acaf` 落 manifest+keystore、run 设 ORZ_ACAF_MANIFEST/KEYSTORE/BINARY/FAIL_CLOSED=1；`D:\tb-eval\.env` 已移除 FAIL_CLOSED=0；Linux musl 重建后容器冒烟通过（provision 0600 keystore + signer stdio initialize_session + `orz --real` 带签发器启动 3 次、wallclock 正常收尾）。关键词：ACAF、fail-closed、harbor、orz.py、评测链路、容器供应、file-0600-installation。入口：[`BACKLOG 0a`](docs/BACKLOG_AND_PRIORITIES.md) / [`TODO P0-E`](TODO.md) / [`scripts/orz_acaf_run.ps1`](scripts/orz_acaf_run.ps1)。
- **GAP-CONSOLE-TOOLNAME-PATTERN** (`implemented`; 2026-08-17；2026-08-17 闭合)：console 默认面三个工具名含点号（`blackboard.action_write`/`console.step_done`/`console.return_to_console`），违反 OpenAI 兼容工具名模式 `^[a-zA-Z0-9_-]+$`，真实 DeepSeek API 在计划落板后下一轮 400（FakeProvider 不校验故单测未暴露）；修复=改名下划线（`blackboard_action_write`/`console_step_done`/`console_return_to_console`）并同步 verifier/schema/文档。**2026-08-17 已闭合**——orz 子模块 0304b23（11 Rust 文件 91 处 + fmt 收口）、Python verifier/schema/测试、ADR-0010 §14.20 与设计文档同步；orz-loop 434 / orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 + real_flag 2 通过、clippy 无新增告警；manifest 重生成 1401 条目、仓库门禁 valid；Linux musl 重建后冒烟重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57`（30m21s 跑满 1740s 预算、`run_invalidated{wallclock}` 正常收尾，对比旧运行 400 即死）。关键词：console 工具名、invalid_request_error、工具名模式、plan-first、改名下划线。入口：[`BACKLOG 0a`](docs/BACKLOG_AND_PRIORITIES.md) / [`TODO P0-E`](TODO.md) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs)。
- **GAP-TOOL-BUDGET** (`implemented`; 2026-08-09)：MAX_TOOL_ROUNDS 已改为 120（orz `c1513a5`），测试与 env 覆盖同步；ADR-0008 其余语义保留。关键词：MAX_TOOL_ROUNDS、ADR-0008、budget migration。入口：[`AUTH-FREEZE-AUDIT`](docs/audits/ADR_0010_FREEZE_AND_ARCHIVE_AUDIT_2026-08-09.md) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs)。
- **GAP-INQUIRY-SPLIT** (`implemented`; 2026-08-10)：混合 inquiry 已拆为独立机制——Orientation（会话级 7 轮状态机+真实注入+v0.2 事件）、Information Sufficiency（机械 assessment，`indeterminate`）、Counterexample（保留）、Stagnation（去双重消费）；`inquiry.rs`/`neutral_inquiry`/`retrieval_completion_check` 已删（v0.1 replay-only）；Rust producer 整轨翻 v0.2。边界：internal/external 车道零投喂（接 GAP-SUBAGENT-RUNTIME）、Diagnostic Coverage 未接线。关键词：7 轮、orientation_checkpoint、information_sufficiency_assessment。入口：[`orz/crates/orz-loop/src/orientation.rs`](orz/crates/orz-loop/src/orientation.rs) / [`GAP-INQUIRY-SPLIT 实施审计`](docs/audits/GAP_INQUIRY_SPLIT_IMPL_AUDIT_2026-08-10.md)。
- **GAP-SUBAGENT-RUNTIME** (`implemented`; 2026-08-10)：子代理与主 agent 复用同一共享 AgentLoop（agent_loop.rs：LoopProfile/RoundAgent/SharedLoopServices），独立 120 轮预算、同一 journal 链、写域 deny-only 门禁（写/执行/Shell 三类结构化拒绝）、activation 注册表（D3-3 身份正规化）、assessment→disposition→close 链（retrieval_disposition 控制工具 + outcome 机械判定）、internal/external 车道 orientation 投喂、DC producer（§4.6 机制完整，硬信号 4/6 产出）。真实检索工具/结构化结果/跨 turn 持久化/mode authority 由 GAP-RETRIEVAL-TOOLS 闭合。关键词：shared runtime、disposition、close record、activation、DC。入口：[`agent_loop.rs`](orz/crates/orz-loop/src/agent_loop.rs) / [`GAP-SUBAGENT-RUNTIME 实施审计`](docs/audits/GAP_SUBAGENT_RUNTIME_IMPL_AUDIT_2026-08-10.md)。
- **GAP-RETRIEVAL-TOOLS** (`implemented`; 2026-08-10)：真实检索工具（project_doc_index 内部 + web_search/web_fetch 接 grok_build，local_browser 自动化已由 GAP-LOCAL-BROWSER 闭合）+ 结构化结果五字段 v0.2 schema（[RESULT_JSON] 模型块校验 + 机械 ledger + visibility 真实分级 + 显式降级）+ 跨 turn activation 侧车持久化（restore 事件合法化跨 run disposition）+ retrieval mode authority（三态 + transition 事件 + off 投影/门禁）+ DC 两信号 + pre_handoff trigger。关键词：retrieval_mode、result_committed、activation_restored、evidence、project_doc_index。入口：[`GAP-RETRIEVAL-TOOLS 实施审计`](docs/audits/GAP_RETRIEVAL_TOOLS_IMPL_AUDIT_2026-08-10.md) / [`project_doc_index.rs`](orz/crates/orz-host/src/project_doc_index.rs)。
- **GAP-LOCAL-BROWSER** (`implemented`; 2026-08-10)：local_browser 浏览器自动化（CDP）——host 自有工具 `browser_read`（单调用内 create→navigate→read→close tab，§3.7.6 tab ownership）+ 自动启动浏览器（**有头默认**（用户裁决 D-13：窗口可见可手动登录，cookie 会话隔离 profile 内持久）；`ORZ_BROWSER_HEADLESS=1` 回退 headless；隔离 profile、DevToolsActivePort 轮询、跨 run 存活、taskkill 树杀 + A5 清扫）+ URL 门禁（§3.7.3 初始+每次 redirect 重检；**DNS 层 SSRF 复用 check_ssrf** + scheme/凭据/黑名单/单标签）+ host 内置固定表达式（任意 JS 非 MVP）+ evidence web_page 分级 + 探针真实化（失败 Degraded 显式）。关键词：browser_read、CDP、local_browser 自动化、URL 门禁、有头/headless。入口：[`local_browser/`](orz/crates/orz-host/src/local_browser/) / [`GAP-LOCAL-BROWSER 实施审计`](docs/audits/LOCAL_BROWSER_IMPL_AUDIT_2026-08-10.md)。
- **GAP-SUFFICIENCY-SCHEMA** (`implemented`; 2026-08-10)：v0.2 Schema/fixture 已建，双轨 verifier 覆盖 §4.4 机械链校验（CAS/幂等/冲突 decision/迟到 close/close 后禁止）与 inquiry_kind 交叉校验；assessment/disposition/close producer 全部落地并捕获真实 journals（GAP-INQUIRY-SPLIT + GAP-SUBAGENT-RUNTIME）。关键词：awaiting_parent_disposition、close receipt、requirement_delta、v0.2。入口：[`run-event-v0.2.schema.json`](runtime/run-event-v0.2.schema.json) / [`run_event_journal_validation.py`](assurance/run_event_journal_validation.py) / [`AUTH-V1.1-REVIEW`](docs/audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。
- **GAP-WEB-SEARCH-SEMAPHORE** (`implemented`; 2026-08-10)：全局 `web_search` 并发=1 semaphore（§3.7.7 第 7 项 web_search=1 分量）——唯一汇点 `OrzHost::call_tool` 挂载 `Arc<Semaphore>(1)`（每 OrzHost 实例一个；§11.3 双并发形态，**C2-1 解禁后 semaphore 自动覆盖检索 lane 内直执行路径**——2026-08-11 切片闭合无执行路径缺口，见 web_search 执行器审计），acquire 在 P0-1 timeout 包裹内（等待计入 300s 预算、timeout drop 自动释放 permit、等待超时不杀树），`web_search_*` 变体同 gate（relay 同构防拼写绕过），web_fetch 不 gate；三面审查闭环（P3-1/P2-1/P3-4/F7 修复）。关键词：concurrency=1、web_search semaphore、资源所有权约束。入口：[`WEB_SEARCH_SEMAPHORE_IMPL_AUDIT`](docs/audits/WEB_SEARCH_SEMAPHORE_IMPL_AUDIT_2026-08-10.md) / [`web_search 执行器审计`](docs/audits/ADR_0006_WEB_SEARCH_CREDENTIAL_AND_C2_1_UNBLOCK_IMPL_AUDIT_2026-08-11.md) / [`lib.rs`](orz/crates/orz-host/src/lib.rs) / [`tools.rs`](orz/crates/orz-host/src/tools.rs)。
- **GAP-CONVERSATION-RESTORE** (`implemented`; 2026-08-10)：conversation 跨 prompt 恢复——会话级侧车 `{cwd}/.gsa/conversations/{session8}.json`（StoredConversation envelope；明文含 reasoning_content——DeepSeek 多轮回放硬约束，隐私边界=侧车非 journal 证据面 §5.4.6 只约束 journal），run_turn 链 threading `conversation: Option<&mut Vec<Message>>`（clone 种子+成功-only 回写+注入块过滤），主 Agent 连续 prompt 自动延续（同 session_id 跨进程重建自动恢复），子代理 `StoredActivation.conversation` 随 activation 侧车恢复（跨 run continue 不再从头开始；`submitted` 仍不入——D-6 部分撤销）；**零事件/schema 变更**；显式 resume（UI/restore event/permission）登记边界留后续；grill/-p 不接；7 天 retention 纳入。关键词：conversation sidecar、跨 prompt、reasoning 回放、D-6 更新。入口：[`GAP_CONVERSATION_RESTORE_IMPL_AUDIT`](docs/audits/GAP_CONVERSATION_RESTORE_IMPL_AUDIT_2026-08-10.md) / [`acp_server.rs`](orz/crates/orz-host/src/acp_server.rs) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs)。
- **GAP-PROJECT-DOC-INDEX-CACHE** (`implemented`; 2026-08-11)：project_doc_index 索引缓存/增量扫描——每 query 全树 stat 遍历（零内容读）+ `size+mtime(secs,nanos)` diff（未变化文件零重读、变化重提取、消失删除）、快照跨 run 持久化 `{cwd}/.gsa/project-doc-index/cache.json`（cwd 级非 session8；envelope schema_version+指纹）、惰性 load + diff 后 best-effort persist（稳态零写放大）、逃生阀 `ORZ_PROJECT_DOC_INDEX_FORCE_RESCAN=1`（+测试 seam `set_force_rescan`）、retention 纳入 7 天清扫、并发 `Mutex<Arc<CachedSnapshot>>`（锁内 diff 锁外 content 读）；正确性优先——结构增删 stat 保证、任何缓存失败全量重建零结果变化；已知盲区：同 size 同 mtime 修改不可察（逃生阀兜底）。关键词：增量扫描、mtime diff、索引缓存、逃生阀。入口：[`PROJECT_DOC_INDEX_CACHE_IMPL_AUDIT`](docs/audits/PROJECT_DOC_INDEX_CACHE_IMPL_AUDIT_2026-08-11.md) / [`project_doc_index.rs`](orz/crates/orz-host/src/project_doc_index.rs) / [`retention.rs`](orz/crates/orz-host/src/retention.rs)。
- **GAP-PDF-EVIDENCE** (`implemented`; 2026-08-11)：内容寻址 PDF 证据管线——**双通道路由**（`ORZ_PDF_BROWSER_DOMAINS` env 通配域名白名单：命中→浏览器 CDP 下载（登录态文献库），未命中→web_fetch 直连内联；未配置=全直连；白名单内失败显式 `[web_fetch_pdf_*]` 不回退）+ 证据核心（orz-tools `pdf_evidence.rs`：magic/解析校验、sha256 内容寻址 `{cwd}/.gsa/pdf-evidence/{p2}/{full64}/{original.pdf,pages.jsonl,metadata.json}`、pdf_oxide 逐页抽文本、NO_TEXT_LAYER 显式 metadata、50MB 上限、marker 契约 `PDF evidence: N pages, document_id=sha256:…, text_layer=…`）+ `pdf_read(document_id, page_range)`（显式 range ≤20 页/调用、省略读全部页截断输出，跨 run 复用，relay mode 门禁）+ evidence 记账（PDF 分支 full/partial/metadata + `content_sha256` 取文档 hex + **legacy "PDF downloaded" 短提示误判 full_text_observed 修复**）+ retention 7 天清扫（rebuildable→sweepable）。零新第三方依赖（pdf_oxide 已有）；零 schema 变更。关键词：PDF 证据、document_id、pdf_read、白名单路由、INVALID_PDF。入口：[`GAP_PDF_EVIDENCE_IMPL_AUDIT`](docs/audits/GAP_PDF_EVIDENCE_IMPL_AUDIT_2026-08-11.md) / [`pdf_evidence.rs`](orz/crates/codegen/orz-tools/src/implementations/pdf_evidence.rs) / [`pdf_evidence.rs`](orz/crates/orz-host/src/pdf_evidence.rs) / [`cdp.rs`](orz/crates/orz-host/src/local_browser/cdp.rs) / [`ADR-0010 §3.7 条 11`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（v1.4 补写，2026-08-11）。
- **GAP-RUN-TESTS** (`implemented`; 2026-08-11)：RT-001~003 全部闭合——run_tests 分支移入通用 permission gate（Interactive 弹窗确认 / Benchmark 自动放行；2026-08-12 ADR-0010 v1.5 裁决后声明条件=host 携带 runner，ReadOnly/Grill 同样声明、只读保证由 gate 承担）；env_clear + 最小平台 allowlist + `TestRunner::env` 显式注入（`ORZ_TEST_RUNNER_ENV` JSON）；上下文注入前 orz-secrets 脱敏（先脱敏后截断，artifact 原样）；workspace delta 前后元数据 diff（200 条上限 + truncated 标志）写入 ToolCompleted 事件（Schema 先行扩展）；Job Object/timeout/输出上限维持达标。关键词：run_tests、D-9、execution permission、env allowlist、workspace delta、hidden test。入口：[`GAP_RUN_TESTS_IMPL_AUDIT`](docs/audits/GAP_RUN_TESTS_IMPL_AUDIT_2026-08-11.md) / [`V11_IMPL_005_RUN_TESTS_SECURITY_REVIEW`](docs/audits/V11_IMPL_005_RUN_TESTS_SECURITY_REVIEW_2026-08-09.md) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs) / [`lib.rs`](orz/crates/orz-host/src/lib.rs)。
- **GAP-STREAM-RETRY** (`implemented`; 2026-08-12)：流式中断重试已实施——**零 chunk 产出**中断（连接握手/首字节前失败；任何成功解码的 SSE item 含 reasoning delta 均计 chunk）且错误类 ∈ {Transport, Timeout} → `stream_once_with_retry` 重发同一请求体（幂等），`request_max_retries` 次数与 `request_retry_window` 退避窗口双约束先到者止（fork `execute_raw` 同节奏）；已产出 chunk / Cancelled / Model / Parse 不重试；新变体 `GatewayError::StreamInterrupted { attempts, detail }`（journal 可见重试历史）。边界：fork EventSource 读错误另有内部重连（叠加双保险，不改 fork）；握手 429/5xx 不重试（fork ApiError 无 status 字段）。关键词：stream retry、transport error、decoding response body、零 chunk。入口：[`ADR-0007 §4`](adr/ADR-0007-transport-retry-policy.md) / [`transport.rs`](orz/crates/orz-loop/src/gateway/transport.rs) / [`GAP_STREAM_RETRY_IMPL_AUDIT`](docs/audits/GAP_STREAM_RETRY_IMPL_AUDIT_2026-08-12.md)。
- **GAP-ACAF-SLICE1** (`implemented`; 2026-08-12)：ACAF Slice 1 已闭合——独立签发器进程 `orz-signer`（manifest 自校验启动、DPAPI K_install 非 Windows fail-closed、枚举化 stdio JSON-lines 接口、orientation 模板签发器持有）+ host 客户端（K_session HKDF 派生下发、七项验票、one-shot ledger）+ 四类控制事件持票接线（Orientation fire / accepted disposition / close record / goal revision，影子模式：失败仅记录 `control_ticket_rejected` 不阻断）+ 新事件类型 issued/consumed/rejected 先 Schema 后 producer（v0.2 枚举 42、verifier 机械配对、fixture 重生成）。边界：IPC=stdio（命名管道 v2）、manifest 无独立发布密钥签名（v2）、goal_version/policy_revision 恒 0（Slice 2 接线）、E2E Windows-only。关键词：ACAF、ControlTicket、签发器、影子模式、control_ticket_issued。入口：[`GAP_ACAF_SLICE1_IMPL_AUDIT`](docs/audits/GAP_ACAF_SLICE1_IMPL_AUDIT_2026-08-12.md) / [`acaf/mod.rs`](orz/crates/orz-assurance/src/acaf/mod.rs) / [`acaf.rs`](orz/crates/orz-loop/src/acaf.rs) / [`orz-signer.rs`](orz/crates/orz-bin/src/bin/orz-signer.rs)。
- **GAP-ACAF-SLICE2A** (`implemented`; 2026-08-12)：ACAF Slice 2 第一阶段已闭合——动作票据 `file_write_v1`（search_replace）全链贯通 + `credential_read_v1` 机制先建（D8 零接线）；`ControlTicket` 补 `resolved_target_sha256`（解析后真实目标摘要，check 5b 三形态 TOCTOU）；新目标解析模块 `target.rs`（解析原语同源 `orz-paths`，票据侧保留编排差异——verbatim 拒绝 + `..` 折叠 + reparse 组件检查 + 统一斜杠摘要）；签发器 +2 method；`run_host_tool` permission gate 后插入动作票据生命周期（签发→issued→**重新解析重算**→验票→consumed/rejected）；**全程影子模式**（解析失败 → null ticket_id + target_mismatch；失败照常执行，台账=journal 事件）；事件类型复用（仅 ticket_kind/capability_scope 枚举 4→6 + resolved_target 字段，v0.2 枚举 42 不变）。边界：fail-closed 未切换（完整 Slice 2 里程碑）、command_exec/network 未接线（D1 各作一步复制）、TUI/capture 零改动、E2E Windows-only；解析单源化 + goal/policy 接线见 [`GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT`](docs/audits/GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT_2026-08-12.md)。关键词：file_write 票据、resolved_target_sha256、目标解析、影子模式、action_kind_for_tool。入口：[`GAP_ACAF_SLICE2A_IMPL_AUDIT`](docs/audits/GAP_ACAF_SLICE2A_IMPL_AUDIT_2026-08-12.md) / [`acaf/target.rs`](orz/crates/orz-assurance/src/acaf/target.rs) / [`acaf.rs`](orz/crates/orz-loop/src/acaf.rs) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs)。
- **GAP-ACAF-SLICE2B** (`implemented`; 2026-08-12)：ACAF Slice 2 第二阶段已闭合——`command_exec_v1`（`run_tests` 宿主固定命令 + `run_terminal_cmd` shell 命令；argv/cwd/env 三元素目标摘要）与 `network_v1`（`web_fetch`/`browser_read` canonical URL 目标：url crate 规范化、http/https 限定、默认端口/fragment 去除、userinfo 拒绝）全链接线；签发器 +2 method；controller 共享 `run_action_ticket` 生命周期 + run_tests ToolStarted 前插票；`acaf.is_none` 门禁前移（未配置 fabric 零事件回归锁）；`web_search` 无 URL 目标有意不映射（fail-closed 翻转前核查⑦）；**全程影子模式**；枚举 6→8、事件类型 42 不变。边界：fail-closed 未切换（完整 Slice 2 里程碑，用户裁决）、检索车道 web_fetch activation 绑定 null（核查⑧）、command env 绑定面（核查⑨）、E2E Windows-only。关键词：command_exec、network、URL 规范化、run_tests、run_terminal_cmd、web_search 差异面。入口：[`GAP_ACAF_SLICE2B_IMPL_AUDIT`](docs/audits/GAP_ACAF_SLICE2B_IMPL_AUDIT_2026-08-12.md) / [`acaf.rs`](orz/crates/orz-loop/src/acaf.rs) / [`target.rs`](orz/crates/orz-assurance/src/acaf/target.rs) / [`orz-signer.rs`](orz/crates/orz-bin/src/bin/orz-signer.rs)。
- **GAP-DENIAL-POLICY-REVISION** (`implemented`; 2026-08-12)：policy_revision 接线已实施——live 值（`AtomicU64`，per-run 归零）穿透 ACAF 票据绑定（K_session HKDF 输入 + check 4）与 DenialKey（u64 对齐，bump 即 key 变化即熔断重置）；`bump_policy_revision()` 机制就位（Slice 3 ModeChangeTicket 为首个生产递增来源，登记）；goal_version 同步接线——AcceptedContinue 消费 GoalRevisionV1 票后更新 goal 绑定（digest+version 0→1）→ 下一票重派生 K_session 旧票死（ADR-0011 决策 5，Slice 1 D5 闭合，e2e 实测 sequence epoch 重启）。关键词：denial breaker、policy_revision、goal_version、K_session 重派生、V11-IMPL-012。入口：[`ADR-0010 §3.5.4`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [`GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT`](docs/audits/GAP_ACAF_GOAL_POLICY_WIRING_IMPL_AUDIT_2026-08-12.md) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs) / [`agent_loop.rs`](orz/crates/orz-loop/src/agent_loop.rs)。
- **GAP-ACAF-SLICE2-FAILCLOSED** (`implemented`; 2026-08-13)：ACAF Slice 2 fail-closed 机制已闭合——D-13 检索 lane 动作票 activation 绑定（动作票 activation 可选、Orientation 唯一禁止；sign_network_v1 可选参数）、D-14 缺参硬拒绝（missing_target_argument）、D-15 缺依赖/未配置硬拒绝（missing_snapshot_store / missing_goal_context / 启动 fail-fast）、D-16 rejected GoalRevisionV1 不迁移；reject_code Schema +3 先扩展再 producer；TicketGate 统一 gate（控制事件 + 动作票，无 ToolStarted 拒绝）；ORZ_ACAF_FAIL_CLOSED=1 显式翻转，默认影子；2026-08-15 用户已裁决生产启用放行（翻转执行待实施）。关键词：fail-closed、D-12~D-16、missing_target_argument、activation 绑定、TicketGate。入口：[`GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT`](docs/audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md) / [`acaf.rs`](orz/crates/orz-loop/src/acaf.rs) / [`controller.rs`](orz/crates/orz-loop/src/controller.rs) / [`orz-signer.rs`](orz/crates/orz-bin/src/bin/orz-signer.rs)。
- **GAP-WINDOWS-EVIDENCE** (`partial`; 2026-08-09)：三个案例候选（ORZ-WIN-PROC-001/002/003，晋级自 child-tree 探针三场景）已登记并引用探针 digest，均标 `candidate` 未宣称闭环；事故路由保留 WIN-LIM-001（raw TCP）与 WIN-INC-001（observer leak）。关键词：Windows incident、case selection、compatibility evidence、WIN-LIM、ORZ-WIN-PROC。入口：[`docs/incidents/windows/`](docs/incidents/windows/) / [`docs/cases/windows/`](docs/cases/windows/)。
- **ORZ-BUILD-MOUNT-001** (`case-candidate`; 2026-08-17)：容器构建挂载契约事故/案例——orz-assurance `include_str!("../../../runtime/*.json")` 相对 CARGO_MANIFEST_DIR 解析到父仓库 `runtime/`；错误挂载（仅挂 orz 子模块）在依赖编译约 20 分钟后失败；正确挂载=父仓库 `/orz` + 工作目录 `/orz/orz`。预防=构建脚本前置守卫（`D:\tb-eval\build_orz_aliyun.sh` 秒级失败 + 正确命令提示）+ 两份评测文档命令修正 + orz 源码两处 include_str 注释契约。关键词：构建挂载、include_str、runtime/、harness_environment、挂载守卫。入口：[`事故登记`](docs/incidents/ORZ-BUILD-MOUNT-001.md) / [`案例候选`](docs/cases/harness_environment/ORZ-BUILD-MOUNT-001-container-mount.md)。
- **ORZ-TOOL-BINARY-COMPAT-001** (`case-candidate`; 2026-08-17)：打包工具二进制兼容事故/案例——构建侧把 glibc 动态 rg（trixie `/usr/bin/rg`，要求 GLIBC_2.39）打包进 musl orz，任务容器 bookworm（glibc 2.36）加载失败 exit 1 + 空 stdout，`finalize_grep` 把 stderr 丢弃统一报 "No matches found"（7 次 grep 全空、模型错误泛化「无源码」）。处置=finalize_grep 结局三型（非零退出 + stderr 显式报错 / 零范围 / 真无匹配带计数）+ `rg --files` 探针 + hidden/no_ignore 开关 + build.rs ELF PT_INTERP 静态守卫 + 构建脚本静态 musl rg。**归因纪律（用户裁决）**：命令/操作错误先排查环境与机械因素（二进制/运行时兼容、工具包装吞错误、路径/作用域解析、沙箱/权限/ignore 语义、构建打包来源），再归因模型或命令纪律；案例库 README 已登记。关键词：glibc、二进制兼容、空结果、错误吞没、归因纪律、静态链接、harness_environment。入口：[`事故登记`](docs/incidents/ORZ-TOOL-BINARY-COMPAT-001.md) / [`案例候选`](docs/cases/harness_environment/ORZ-TOOL-BINARY-COMPAT-001-bundled-rg-glibc.md) / [`案例库 README`](docs/cases/README.md) / [`ADR-0010 §14.23`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。

- **GAP-SOURCE-WEIGHTING-IMPL** (`implemented`; 2026-08-13)：来源加权机制实现已闭合——机械三档判定器（白名单 1.1 / 默认 1.0 / 劣质源 0.7）+ 机器可读种子名单（`ORZ_SOURCE_WEIGHTING_CONFIG` 覆盖）+ web_search 第二层提示词合同（web_fetch 候选核验、禁 browser_read；候选 ≤5 软约束已由 FUS-RETRIEVAL-MECH 步骤 2/6 取代为机械硬门 `ORZ_WEB_FETCH_CANDIDATE_CAP`，默认 8，2026-08-14）+ 第三层 `source_annotations` 校验合并（非法丢弃记 filtering_log）+ 结构化结果 weight/tier 字段（Schema 先行）；local_browser 共享判定器。边界：web_search 引用 URL 未透传 loop（摘要条目不加权）、browser_read 主车道无 ledger；候选上限软约束已被机械硬门取代（FUS-RETRIEVAL-MECH 步骤 2/6，见上）。关键词：来源加权、分级加权、三层结构、tier、mechanical_weight、source_annotations。入口：[`RETRIEVAL_SOURCE_WEIGHTING_DESIGN`](docs/RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md) / [`SOURCE_QUALITY_SEED_LISTS`](docs/SOURCE_QUALITY_SEED_LISTS_2026-08-12.md) / [`source_weighting.rs`](orz/crates/orz-assurance/src/source_weighting.rs) / [`GAP-SOURCE-WEIGHTING-IMPL 实施审计`](docs/audits/GAP_SOURCE_WEIGHTING_IMPL_AUDIT_2026-08-13.md)。
- **GAP-ENCODING-GATE** (`implemented`; 2026-08-13)：orz 机械编码门控已闭合——encoding 适配模块（解码链：BOM 剥离→UTF-8 严格→GB18030→lossy，记录命中编码；写入统一 UTF-8 无 BOM）+ `run_terminal_cmd`/`read_file`/插件 hooks/run_tests 接线 + `tool_completed.output_encoding` 可选字段（Schema 先行扩展再接线 producer；模型零感知，编码仅进 journal）。边界：PDF/图片/PPTX 等无解码链路径 None；hooks 仅 tracing 不扩展事件面；复查后其余进程文本面（grep/glob 渲染解析、后台快照、状态捕获、编辑/补丁读目标、web_fetch 文本体）已同链处理。关键词：编码契约、UTF-8、GB18030、output_encoding、解码门控。入口：[`OPS-PROTOCOL §8`](protocol/structured-operation-protocol-v0.1.md) / [`encoding.rs`](orz/crates/codegen/orz-tools/src/util/encoding.rs) / [`GAP-ENCODING-GATE 实施审计`](docs/audits/GAP_ENCODING_GATE_IMPL_AUDIT_2026-08-13.md)。

## 4. 保障、合约与安全路由

- **P0-DATA-CONTRACT** (`reference`; 2026-08-09)：Python assurance 的基础 Schema、信封和合约语义先例；是否进入融合 production path 需按组件登记表复核。关键词：P0、contracts、envelope、canonical guarded CLI。入口：[`CANONICAL_GUARDED_CLI_P0_AUDIT`](docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md) / [`contracts.py`](assurance/contracts.py)。
- **P1-SESSION-LIFECYCLE** (`reference`; 2026-08-09)：Python 会话身份、permit、archive journal 和恢复生命周期先例。关键词：P1、conversation namespace、archive recovery、session governor。入口：[`P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT`](docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md)。
- **P2-SANDBOX** (`reference`; 2026-08-09)：Docker 严格沙箱及 Windows Native Sandbox/Job Object 的既有审计与 conformance 输入。关键词：P2、Docker、AppContainer、Job Object。入口：[`P2_DOCKER_SANDBOX_AUDIT`](docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md) / [`GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT`](docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md)。
- **P2.5-GUARDED-EXECUTION** (`reference`; 2026-08-09)：无模型动作、进程追踪和输出拦截的 Python 守卫执行先例。关键词：P2.5、guarded execution、process tracking。入口：[`P2_5_GUARDED_EXECUTION_AUDIT`](docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md)。
- **OPS-PROTOCOL** (`pending`; 2026-08-13)：结构化操作协议 v0.1——模型只提交结构化操作，执行宿主机械自报环境；删除默认回收站，缓存分类放行、非缓存超限拒绝，跨环境仅固定桥接传 JSON，动作全程 JSONL 审计；**审查判定（2026-08-13，用户无异议）：平行执行层过重，不按原样生产接线——保留删除安全为 host-owned 工具、跨环境桥接内部化、双执行器收敛单一参考（生产走 Rust 工具面），裁剪设计待产出**。关键词：structured operation、回收站、缓存分类、ops、fail-closed、裁剪。入口：[协议](protocol/structured-operation-protocol-v0.1.md) / [Schema](protocol/structured-operation-protocol-v0.1.schema.json) / [Python 执行器](assurance/ops_executor.py) / [PowerShell 执行器](scripts/ops_executor.ps1)。
- **CLASSICAL-EXEC-ASSISTANT** (`implemented`; 2026-08-13；2026-08-16 用户裁决转正式组件)：给主模型套“操作台”的机械执行配合部件设计（v0.5 操作台模型，P0-C 已转正式组件）——模型面只注册工具名/动作名，助理层按主模型输出逐层定位组件并执行（注册表路由 → 契约校验 → 目标解析 → ACAF 票据 → 执行 → verifier），不理解动作语义；威胁模型=只防幻觉与注入——助理层为单一策略执行点（围栏合并为策略表 + taint 动作组合禁令；动作级策略沙盒成立，OS 级执行沙盒按动作另行挂载）；槽位表由工作区索引动态生成（`workspace.index`，生产复用 `project_doc_index` 缓存）；集成形态=HA 助理层与 orz 深度融合（HA 为 orz 一部分），薄接缝在 orz 本体 ↔ 底座模型后端（Grok/Codex 等），POC stdio 仅原型隔离；协作形态=黑板动作栏（注册板块=当前轮动作投影、常驻按需读、动作栏=模型写订单、结果栏=receipt+trace_id；发放=机械单一出口、消费一次、round 防重、单轮一单；模型面=读板块+写订单，无执行/发送工具）；fail-closed 返回契约=step+code+message+upstream+trace_id（HA 仅 code+message，扩展供模型排障；step=execute 附有界 trace 尾部）；执行日志可见性=只读 `assistant.trace` 服务（有界、读入审计），主模型可结合日志与操作台覆盖未预录内容；动作粒度=细粒度优先（粗按钮才是限制模型；配套 PTC/管道/意图组合层 + 版本化动作契约，负担=构建期线性、运行时近零）；机械组合=PTC 线性脚本模式（步骤=动作实例 + `$ref` 数据引用，小样 3）；借鉴 DeepSeek Harness PTC 程序化工具调用 + Profile/Bundle 动作组合（仅设计本身，不引其栈）；小样 1 已跑通（`prototype/classical_console/`，28/28，含 fail-closed 契约与执行日志）；小样 2 编辑执行器对照实验已闭合（2026-08-15 用户裁决通过，独立判定一致；结果工件 `prototype/classical_console/sample2_result.json`）；小样 3 机械组合脚本模式已闭合（2026-08-15 用户裁决通过 + 独立判定一致；结果工件 `prototype/classical_console/sample3_result.json`）；内嵌集成 S1/S2 已落地（2026-08-15：S1 操作台核心 + 黑板动作栏数据面；S2 模型面投影（`blackboard_read` section=actions + `blackboard_action_write`）+ 轮末机械发放（round/plan_epoch/run_id 防重放、step=policy 归一化、TraceStore.commit 收口），实施审计见 [S2 审计](docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md)）；**S3 前置（P1-2 结构化策略拒绝）已闭合（2026-08-15，全面审查修复 F1-F8 已登记）**——`ToolResult.policy_denial = {source, code, reason}` 接线权限/ACAF/检索模式门五条拒绝路径、console 适配层退役前缀判定、ToolCompleted 增可选 `policy_denial`（Schema/verifier/fixtures 先行 + 交叉规则）、内容碰撞回归，实施审计见 [S3 前置审计](docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md)；**S3（assistant.trace 接线 + run_script PTC 生产化 + Profile/Bundle 加载）已闭合（2026-08-15）**——assistant.trace 按 trace_id 有界取回（读操作入 trace 与事件面）、workspace.run_script 逐行契约校验 + trace（8 步/30s/4MiB、禁嵌套、fail-closed 保留内层 step/code + script_step）、注册板块=Profile/Bundle ∩ 探针完整集（ActionBundle standard/read_only/benchmark + registrations_for，同轮探针快照同时驱动工具投影与注册板块），实施审计见 [S3 审计](docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md)（2026-08-16 审查收口 §6：步数上限 8、not_found=step=execute、checkpoint 轮跳过刷新、注册不变式补齐）；**S4 已闭合（2026-08-16）**——单步超时下沉 host 层（`call_tool_with_timeout` 覆盖 + 进程树收口、结构化 `timed_out` → `tool_timeout`/`script_timeout`+`script_step`）、脚本 tool-round 预算（发放前预检 `budget_insufficient`、实际步数减计、下一轮预算块机械反映）、端到端测试（完整会话/checkpoint 板块保留/超时/预算边界）、决策门材料清单齐备，实施审计见 [S4 审计](docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md)；**正式组件决策门已闭合（2026-08-16 用户裁决「P0-C 可转正式组件」，不达标即撤条款未触发）**；模型面双模式定案（console 默认 + direct 受控降级，2026-08-15 用户裁决，暂定）见 [PLAN_FIRST_BLACKBOARD_DESIGN](docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md)；调研结论：Rhasspy 已归档、Rasa 维护模式、n8n 非真开源；**Home Assistant 服务模型 + hassil（Apache-2.0、活跃）为首选成熟参考**，StackStorm/Node-RED/OVOS 备选；承接 OPS-PROTOCOL 裁剪方向，不新增平行协议、执行层全部确定性；PLAN-FIRST 阶段 A/B/C 全部闭合（2026-08-16，P0-C 无开放项）；**2026-08-17 大文件读取契约补充定案**——读取工具契约有界返回（超过粗门默认 16KB 的文件返回读取句柄信封而非全文，精门=50K 注入预算；语义适配留模型、黑板/结果栏只放指针），见 FUS-LARGE-FILE-READ-CONTRACT / ADR-0010 §14.22。关键词：操作台、console、注册表路由、action contract、策略执行点、taint、槽位表、workspace index、hassil、Home Assistant、机械执行层、Cortana、GOFAI、PTC、Profile/Bundle、fail-closed、step、upstream、trace、assistant.trace、动作粒度、脚本组合、黑板、动作栏、订单、发放、policy_denial、结构化策略拒绝。入口：[设计](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [POC](prototype/classical_console/README.md) / [S3 前置审计](docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md) / [S3 审计](docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md) / [S4 审计](docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md) / [阶段 C 审计](docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md) / [待办](docs/BACKLOG_AND_PRIORITIES.md)。
- **P3-INSTRUCTION-AUTHORITY** (`reference`; 2026-08-09)：instruction provenance、capability delegation 和 child capability enforcement 的既有合约输入。关键词：P3、instruction gate、授权、子能力。入口：[`P3_INSTRUCTION_AUTHORITY_AUDIT`](docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md)。
- **P4-AUDIT-RECOVERY** (`reference`; 2026-08-09)：audit ledger、compaction、archive verifier 和恢复授权的 Python conformance 输入。关键词：P4、compaction、recovery、lineage。入口：[`P4_AUDIT_COMPACTION_RECOVERY_AUDIT`](docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md)。
- **P4.5-WORKSPACE-FIRST** (`reference`; 2026-08-09)：workspace-first 集成与可恢复变更的既有审计输入。关键词：P4.5、workspace、mutation、snapshot。入口：[`P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT`](docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md)。
- **P5-TASK-PREFLIGHT** (`reference`; 2026-08-09)：任务合同、用户意图和 synthetic-user preflight 的 Python 先例。关键词：P5、task contract、preflight、readonly projection。入口：[`P5_SYNTHETIC_USER_TASK_PREFLIGHT`](docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md)。
- **GATE-CHAIN** (`partial`; 2026-08-09)：instruction、tool availability、adapter、source visibility、permission 和 runtime guards 构成分层 gate 链；与融合 runtime 的最终接线需随实现审计复核。关键词：Gate、IPG、permission bridge、source gate。入口：[`assurance/`](assurance/) / [`orz/crates/orz-assurance/`](orz/crates/orz-assurance/)。
- **SEC-CREDENTIALS** (`partial`; 2026-08-11)：凭据 registry、读取和脱敏——web_search 执行器复用主 DeepSeek key（2026-08-11：无新注册目标，`redacted()` 唯一序列化出口（当前生产接线=构建时 tracing::info）+ lane 内豁免权限门授权链=mode 门；xAI 独立搜索后端方案被用户裁决否决）；DeepSeek live 通道与 Windows 实机晋级证据仍 partial（ADR-0010 §11.7）。关键词：credential registry、keystore、scrub、GAK-CRED-001。入口：[`ADR-0006`](adr/ADR-0006-credential-target-registry.md) / [`DEEPSEEK_CREDENTIAL_HARDENING`](docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md) / [`web_search 执行器审计`](docs/audits/ADR_0006_WEB_SEARCH_CREDENTIAL_AND_C2_1_UNBLOCK_IMPL_AUDIT_2026-08-11.md)。
- **EVIDENCE-LOCAL-BROWSER** (`partial`; 2026-08-11)：Local Browser/PDF evidence 保留状态机、失败处理、URL/JS/prompt-injection 和全文可见性边界；**规范性裁决已转录 ADR-0010 §3.7**，网页读取生产实现在 Rust（GAP-LOCAL-BROWSER）、PDF 证据管线生产实现在 Rust（**GAP-PDF-EVIDENCE** 已闭合）；Python 实现（retrieval_workflow/evidence_store/pdf_evidence 等）待按 ADR-0010 重新符合性审查或退役。关键词：LBR-001、CDP、PDF evidence、prompt injection。入口：[`LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE`](存档/architecture/pre-adr-0010/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md) / [`GAP_PDF_EVIDENCE_IMPL_AUDIT`](docs/audits/GAP_PDF_EVIDENCE_IMPL_AUDIT_2026-08-11.md) / [`retrieval_workflow.py`](assurance/retrieval_workflow.py)。

## 5. ADR 登记表

本表仅登记状态和入口；设计内容必须回到 ADR 原文。

| ADR | 当前状态 | 入口 |
|---|---|---|
| ADR-0001 | proposed | [`ADR-0001`](adr/ADR-0001-evidence-constrained-local-agent-kernel.md) |
| ADR-0002 | accepted / deferred | [`ADR-0002`](adr/ADR-0002-defer-cloud-runtime.md) |
| ADR-0003 | accepted / partially superseded by ADR-0010 | [`ADR-0003`](adr/ADR-0003-runtime-neutral-assurance-kernel.md) |
| ADR-0004 | accepted | [`ADR-0004`](adr/ADR-0004-general-science-profile-layering.md) |
| ADR-0005 | superseded by ADR-0010 | [`ADR-0005`](adr/ADR-0005-neutral-inquiry-thresholds-finalized.md) |
| ADR-0006 | accepted | [`ADR-0006`](adr/ADR-0006-credential-target-registry.md) |
| ADR-0007 | accepted | [`ADR-0007`](adr/ADR-0007-transport-retry-policy.md) |
| ADR-0008 | accepted / numeric value partially superseded | [`ADR-0008`](adr/ADR-0008-tool-round-budget.md) |
| ADR-0009 | accepted | [`ADR-0009`](adr/ADR-0009-write-placement-policy.md) |
| ADR-0010 | accepted / frozen / current authority | [`ADR-0010`](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) |
| ADR-0011 | accepted / Slice 1+2 已实施；fail-closed 生产启用已闭合（2026-08-16：默认强制、三入口接线、核查清单 ⑦⑨⑩⑪ 收口、供应工具与启动链；用户 2026-08-15 裁决放行）；Slice 3/4 待实施 | [`ADR-0011`](adr/ADR-0011-authenticated-control-and-action-fabric.md) |

## 6. 评测与回归入口

- **EVAL-POLYGLOT** (`reference`; 2026-08-09)：多语言 benchmark 用于暴露 loop、transport、工具反馈和预算问题，不独立定义产品设计。关键词：polyglot benchmark、feedback loop、D-9。入口：[`POLYGLOT_BENCHMARK_FINDINGS`](docs/POLYGLOT_BENCHMARK_FINDINGS_2026-08-06.md)。
- **EVAL-TERMINAL-BENCH** (`reference`; 2026-08-09)：Terminal-Bench 2 记录任务级表现、挂死守卫和 harness 证据；2026-08-12 job1 评测（v1.5 验证达成 + GAP-STREAM-RETRY 暴露）。关键词：TB2、wallclock、mounts、hard tasks、v1.5 验证。入口：[`TERMINAL_BENCH_2_EVAL`](docs/TERMINAL_BENCH_2_EVAL_2026-08-08.md) / [`TERMINAL_BENCH_2_EXPLORATORY_SCORE_AUDIT`](docs/TERMINAL_BENCH_2_EXPLORATORY_SCORE_AUDIT_2026-08-08.md) / [`TERMINAL_BENCH_2_JOB1_EVAL_2026-08-12`](docs/TERMINAL_BENCH_2_JOB1_EVAL_2026-08-12.md)。
- **EVAL-SWE-BENCH** (`reference`; 2026-08-09)：SWE-bench Verified 记录软件修复任务的评测范围和结果边界。关键词：SWE-bench、verified、software repair。入口：[`SWE_BENCH_VERIFIED_EVAL`](docs/SWE_BENCH_VERIFIED_EVAL_2026-08-07.md)。

## 7. 源码与机器合约速查

| 领域 | 当前入口 |
|---|---|
| Rust Agent loop | [`orz/crates/orz-loop/`](orz/crates/orz-loop/) |
| Rust host / ACP | [`orz/crates/orz-host/`](orz/crates/orz-host/) |
| Rust assurance | [`orz/crates/orz-assurance/`](orz/crates/orz-assurance/) |
| Rust CLI / TUI | [`orz/crates/orz-bin/`](orz/crates/orz-bin/) / [`orz/crates/orz-tui/`](orz/crates/orz-tui/) |
| Python conformance / fixture | [`assurance/`](assurance/) |
| Runtime event Schema | [`runtime/`](runtime/) |
| Protocol | [`protocol/`](protocol/) |
| Repository checks | [`scripts/`](scripts/) |
| Current architecture projection | [`architecture/current/`](architecture/current/) |
| Current audits | [`docs/audits/`](docs/audits/) |
| Historical materials | [`存档/`](存档/) |

## 8. 状态速查

本节只列 canonical ID，不重复定义：

- `current-design`：AUTH-ADR-0010、AUTH-CURRENT-PROJECTION、FUS-CORE、FUS-AGENT-TOPOLOGY、FUS-CONCURRENCY、FUS-RETRIEVAL-MODE、FUS-INFORMATION-SUFFICIENCY、FUS-ORIENTATION、FUS-DIAGNOSTIC-COVERAGE、FUS-COUNTEREXAMPLE、FUS-STAGNATION、FUS-BUDGET、FUS-STATE-RECOVERY、FUS-WINDOWS-BOUNDARY、FUS-UI-BOUNDARY、FUS-CONTROL-FABRIC、FUS-SOURCE-WEIGHTING、AUTH-PLAN-FIRST-BLACKBOARD。
- `implemented`：IMPL-WRITE-PLACEMENT、IMPL-GLOBAL-REVIEW、IMPL-RUN-EVENT-SCHEMA、GAP-TOOL-BUDGET、GAP-INQUIRY-SPLIT、GAP-SUBAGENT-RUNTIME、GAP-SUFFICIENCY-SCHEMA、GAP-RETRIEVAL-TOOLS、GAP-LOCAL-BROWSER、GAP-WEB-SEARCH-SEMAPHORE、GAP-CONVERSATION-RESTORE、GAP-PROJECT-DOC-INDEX-CACHE、GAP-PDF-EVIDENCE、GAP-RUN-TESTS、GAP-STREAM-RETRY、GAP-ACAF-SLICE1、GAP-ACAF-SLICE2A、GAP-ACAF-SLICE2B、GAP-DENIAL-POLICY-REVISION、GAP-ACAF-SLICE2-FAILCLOSED、GAP-ENCODING-GATE、GAP-SOURCE-WEIGHTING-IMPL、FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH、FUS-COMPACTION-REDESIGN、AUTH-BLACKBOARD-PLAN-EPOCH、FUS-BLACKBOARD-PLAN-EPOCH、AUTH-ORIENTATION-FORCED-TEMPLATE、FUS-ORIENTATION-FORCED-TEMPLATE、FUS-CONSOLE-POLICY-DENIAL、FUS-REQUEST-CACHE、CLASSICAL-EXEC-ASSISTANT、FUS-AGENTS-MD-PLAN-WRAPPER、FUS-PROMPT-DEPERSONALIZE、FUS-PLAN-FIRST-MODEL-SURFACE、FUS-CONSOLE-DUAL-MODE、FUS-PLAN-STEP-GATE、GAP-CONSOLE-TOOLNAME-PATTERN、GAP-ACAF-HARNESS-PASSTHROUGH、FUS-LARGE-FILE-READ-CONTRACT、FUS-LEDGER-FOLD-STATE、FUS-TOOL-SCOPE-CONTRACT。
- `partial`：IMPL-RUST-RUNTIME、IMPL-DEEPSEEK-TRANSPORT、GAP-WINDOWS-EVIDENCE、FUS-COMPONENT-REGISTER、GATE-CHAIN、SEC-CREDENTIALS、EVIDENCE-LOCAL-BROWSER、IMPL-CONTROL-FABRIC。
- `pending`：OPS-PROTOCOL、AUTH-SESSION-CONTEXT-MONITOR、FUS-SESSION-CONTEXT-MONITOR、FUS-RECOVERY-TOOL-OUTCOME、FUS-BENCHMARK-FULL-EXEC（实施完成待验证）。
- `reference`：AUTH-BACKLOG、AUTH-TODO、AUTH-V1.1-REVIEW、AUTH-FREEZE-AUDIT、IMPL-PYTHON-REFERENCE、P0-DATA-CONTRACT、P1-SESSION-LIFECYCLE、P2-SANDBOX、P2.5-GUARDED-EXECUTION、P3-INSTRUCTION-AUTHORITY、P4-AUDIT-RECOVERY、P4.5-WORKSPACE-FIRST、P5-TASK-PREFLIGHT、EVAL-POLYGLOT、EVAL-TERMINAL-BENCH、EVAL-SWE-BENCH、FUS-DSH-BORROW-REVIEW。
- `historical`：AUTH-ARCHIVE、AUTH-INDEX-SNAPSHOT。

## 9. 使用红线

- 本索引不是事实证据或设计权威；任何结论都必须沿入口回查。
- 不得从 `存档/` 直接生成当前实现要求；先确认该内容是否已转录进 ADR-0010。
- 不得因现有代码或测试锁定旧行为，就把实现现状反写成设计。
- 不得把 `reference`、fixture 或 Python conformance 路径描述为 production runtime owner。
- 不得把一次跑分、测试全绿、提交完成或阶段关闭写成架构符合性结论。
- 不得为同一概念创建第二条长说明；补充关键词和入口应修改原 canonical entry。
- 不得在状态速查、ADR 表或源码速查中复制主题定义。
