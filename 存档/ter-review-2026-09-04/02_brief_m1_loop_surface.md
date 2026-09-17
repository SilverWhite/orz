# 工单：TER M1 循环面切片审查（T1.6–T1.9 + T2.1）

你是 TER 实现审查代理（补跑 a2 切片；先前的 a2 仅口头回报、明细未落盘）。
环境：Windows PowerShell，cwd=D:\CLI。

## 纪律
- 只审查【已提交】内容；禁止修改/删除/创建仓库文件（唯一例外：报告写入
  `D:\CLI\.ter_review_2026-09-04\02_m1_loop_surface.md`）；禁止 git 写操作；
  禁止触碰工作树未提交内容（0l 并行改动：git status 里的 M/?? 一批）。
- 提交态内容用 `git -C D:\CLI show <commit>:<path>` 提取，可输出到 $env:TEMP
  再读；也可用 `git -C D:\CLI\orz show <commit>:<path>`。
- 不运行 cargo/构建；python 仅允许只读语法检查；PowerShell 只静态阅读不执行；
  禁止联网。
- 证据给出 commit、文件:行号、审计文档原文；无法验证写 NOT-VERIFIED。
- 审查三轴都要覆盖：设计合理性、实现合理性、设计与实现的符合性。

## 审查范围（对照 TODO2.md 验收行）

orz 子仓库相关提交（按历史顺序）：
- f2ed4d8e = T1.6 黑板 processes live 分区
- e6e8aee0 = T1.7 轮预算默认无限制
- 95bf432c = T1.8 F6 pull 面 wallclock
- da2c23a1 = T1.9 F6 push 档（budget_cue_injected）

主仓库相关提交：
- ac122e4 = T2.1 墙钟单一化（_windows_high_nist/run/run_agent_arm.ps1 等）

对应审计文档（声称值/测试名/边界声明都要回源码核对）：
- docs/audits/TER_T1_6_PROCESSES_LIVE_PARTITION_2026-09-04.md
- docs/audits/TER_T1_7_UNLIMITED_ROUND_BUDGET_DEFAULT_2026-09-04.md
- docs/audits/TER_T1_8_F6_PULL_WALLCLOCK_2026-09-04.md
- docs/audits/TER_T1_9_F6_PUSH_BUDGET_CUE_2026-09-04.md
- docs/audits/TER_T2_1_WALLCLOCK_SINGLE_SOURCE_2026-09-04.md

必读设计/登记材料：
- docs/TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md（§3.1/§7/§9 参数与边界）
- TODO2.md（T1.6–T1.9、T2.1 验收行）
- docs/BACKLOG2.md（TER-0 状态）
- adr/ADR-0010-fusion-runtime-and-agent-architecture.md §14.53 候选项
  （Select-String 定位后按行区间读）
- 参考切片 a1 已完成的报告：
  .ter_review_2026-09-04/01_m1_exec_semantics.md（了解其已报项，避免重复；
  它把 journal 侧事件接线与全量测试复跑留给本切片复核）

## 重点核对项

1) T1.6 processes live：Terminal TaskLiveSnapshot/list_live_tasks 读取时现算
   （pid/elapsed/status/字节/CPU/killable、fail-closed 默认空）；LoopHost
   terminal_live_processes 映射；blackboard_read section=processes 渲染
   （命令摘要 ≤80B、≤8KiB 预算、空态「（无）」）；live-only 越权守卫
   （epoch/receipt_id 显式报错）；kill 形态不暴露 &/is_background。
2) T1.7 轮预算：MAX_TOOL_ROUNDS 120→0；轮数闸 >0 守卫；默认无
   tool_rounds_limit/exhaustion；session 面 budget=0 渲染 unlimited；
   budget_insufficient 仅显式非零上限时生效；检索子代理与主车道取 min 组合。
3) T1.8 F6 pull：session 面 WALLCLOCK_ELAPSED（LIF run-relative 只读换算）/
   LIMIT+REMAINING（ORZ_MAX_WALLCLOCK>0 生效；未施加渲染 limit none）；
   旧 wrapper 输出逐字节不变声明是否可信。
4) T1.9 F6 push：EventType BudgetCueInjected（snake_case 与 run-event schema
   一致）；ORZ_F6_PUSH 默认 off；on 且 ORZ_MAX_WALLCLOCK 上限配置时主车道
   每轮按 <600/300/120 注入 ≤3 次/run（T0.2 verifier ≤4 兼容）；注入文本
   中性（只报剩余/上限/已用轮）；注册 injected-block 不持久化；off 零注入
   零事件零文本。
5) T2.1 墙钟单一化：run_agent_arm.ps1 提交态无 --max-wallclock/无 perTask-60/
   无 840；sandbox --timeout=官方 agent_timeout_seconds 唯一墙钟；
   ORZ_MAX_WALLCLOCK 透传；DryRun 计划行；AGENT_ERRORS=0 声明；
   与 orz 侧 T1.8/T1.9 读源一致。
6) 跨层一致性：processes/env 分区的越权边界与其它 blackboard 分区一致；
   tool_running idle-kill 事件生产者在 orz-loop/orz-assurance 是否已接线
   （a1 把此项留给本切片/根代理复核——优先本切片查 journal 生产者与 verifier
   分支，若在 T1.13 门文件里也登记则指出）；审计声称测试名/数量是否在提交态
   源码可定位；TODO2/BACKLOG2/audits 三处登记互相一致。

## 输出要求
最终回答中文：总评 → 发现清单（ID M1L-xx、P0/P1/P2/P3、类别、位置、问题、
证据、建议修法）→ T1.6–T1.9/T2.1 逐项符合性表（PASS/PARTIAL/FAIL/
NOT-VERIFIED+证据）→ 审计文档真实性核对小节 → 正面发现。
同时把完整报告写入 `D:\CLI\.ter_review_2026-09-04\02_m1_loop_surface.md`
（只限该目录），并在最终回答发给根代理。
