# 362 批：开发者体验与工具面摩擦治理（设计稿）

> 日期：2026-10-26
> 用户令：「这一轮你提出的这些摩擦可以立项成为一个新的批次。」⇒ 立项 **362**（计数 65 → 66）。
> 状态：`implemented`（已闭合 66 → 65；S1-S6 已落码；S2 tail/filter 为 loop 内部后处理、长任务流式沿用既有 auto-background；S3 `range` 已进 read_file 工具层、grep 复用 `-A/-B/-C`；S6 活路径为 terminal `cargo test`，`run_tests` 仍 R1 封存）；审查：[`362 审查档`](audits/362_DEV_ERGONOMICS_REVIEW_2026-10-26.md)；来源＝358 审查/收尾 run 的实测摩擦（`RUN-3e6194aa-8`）。
> 定位：只治理“模型使用 orz 的不顺手”，不改模型面工具名/协议、不改 journal 链、不改审批姿态。

## §0 结论

本轮实际使用中最卡的两件是**上下文压缩的边界维护**与**终端输出的噪音/分页**；其余为跨 run 接续、capabilities schema、测试反馈、self_check 硬门。按 S1–S7 立项：P1 先行（S1/S2/S4），P2 随后（S3/S5/S6/S7）。

## §1 实测摩擦（证据）

1. **压缩工作现场维护**：`RUN-3e6194aa-8` 压缩 12 个块；每块要求模型手写 `[SEMANTIC_SUMMARY]`（块号/目标/已完成/关键决策/未决/下一步/关键文件），分块表在窗口尾部，块号轮次映射需自算；回读历史靠 `blackboard_read journal anchor`（≤512B）或读 `.gsa/compaction/blocks/*.md`。
2. **终端噪音/exit code**：cargo stderr 被 PowerShell 当 `NativeCommandError`，输出混入“所在位置 行:1 字符…”；`$LASTEXITCODE` 不显式打印会被覆盖；4 分钟构建被 `Select-Object -Last N` 缓冲，中途无进度。
3. **大文件/长输出**：`read_file` >64KiB 返回 read handle（需二次 offset）；`grep` 超 200 行分页；`cargo test` 输出 40K 字符截断；审查 `dispatch.rs`（5466 行）多次分页。
4. **跨 run 接续**：`plan` 黑板不跨会话；每个 run 开头读 `CLI_PROJECT_INDEX`＋接续档＋BACKLOG；上一 run 没写全即丢。
5. **capabilities schema**：`spawn_scout` 的 `params_schema=null`（`relay` 非 pub，host/loop 声明面分离），模型从工具面看不到参数结构。
6. **测试反馈**：失败时只保留最后 N 行，首个 panic 常在上文；并行全量有 flaky（进程树/symlink），本 run 串行跑 orz-host 70s。
7. **self_check**：informational，不阻止结束；本轮两次调用但无硬门。

## §2 目标 / 非目标

**目标**：把上述摩擦收敛为可机械验收的工具面/上下文面改造；每项有独立落点与验收。

**非目标**：不改模型面工具名/参数协议；不改 journal 事件链/哈希；不改审批姿态（interactive-yolo 缺省）；不做 LIF 组件。

## §3 分项

| 项 | 优先级 | 内容 | 落点 | 验收 |
|---|---|---|---|---|
| S1 工作现场接力 | P1 | 分块表标“工作现场起始轮次”；`context_manage` 摘要草稿；run 结束 handoff | `context_manage`／分块表渲染／`.gsa/handoff/<run>.md` | 压缩后模型无需自算块号；handoff 含目标/已完成/未决/关键文件/测试读数 |
| S2 终端结构化 | P1 | `run_terminal_cmd` 返回 `{exit_code,stdout,stderr}`；`tail`/`filter`；长任务流式；默认 `cmd /c` 或分离 stderr | 工具层（`orz-host`／`console_exec`） | cargo 失败 exit_code 不再被 PowerShell 覆盖；输出无 `NativeCommandError` 噪音 |
| S3 大文件/长输出 | P2 | `read_file` `range`／“最相关段落”；`grep` 上下文片段；终端 filter | `read_file`／`grep` 工具层 | 5466 行文件一次取到目标段；grep 直接给 `-C` 片段 |
| S4 跨 run resume | P1 | `orz resume` 汇总最近 run journal＋文档指针；handoff 自动生成 | CLI（`orz-bin`）／`.gsa/handoff` | 新 run 一条命令拿到上一 run 的目标/未决/测试读数 |
| S5 capabilities schema 单源 | P2 | `params_schema` 从 loop 侧导入；`declared` 给 schema 摘要 | `relay` 可见性／`capabilities` | `spawn_scout` 的 schema 非 null 或带摘要 |
| S6 测试失败摘要 | P2 | `run_tests` 内置 FAILED 行＋首个 panic＋相关测试名 | 工具层（`run_tests`） | 失败时一屏给出可定位摘要 |
| ~~S7 self_check 硬门~~ | — | **不立项（用户裁决 2026-10-26）**：self_check 仅非日常任务；`[RUN_END]` 已不显式要求（353/359），硬门与现行口径相悖。 | — | — |

## §4 验收

- 每项独立单测/集成；S1/S2/S4 为 P1 先行，S3/S5/S6 为 P2；S7 不立项（用户裁决）。
- 计数：立项 65 → 66；实施完成按项闭合回 65。
- 不改变 358/359/361 已验收读数。

## §5 边界

- 工具返回结构变更需兼容旧调用方（`run_terminal_cmd` 的 stdout/stderr 分离是新增字段，不删旧字段）。
- `read_file` 的 read handle 语义保留（大文件仍可 offset 分页），只加 range/摘要。
- 不改 journal 链；handoff 落 `.gsa`，不进 journal 事件面。

## §6 关系

- 358 审查修复（已入 0.9.14-1）：S5 的 `params_schema` 现状由该批暴露。
- 354 系统控制台（capabilities/inspect/Envelope）：S5 与之同源，避免双实现。
- 压缩线（P2-14／0bs）：S1 是使用面，不改压缩算法。
