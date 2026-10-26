# 362 批（开发者体验与工具面摩擦治理）审查

- 审查基线：`HEAD=29a103d6e25f`，run 起始 worktree 含未提交改动
- 审查范围：362 S1–S6（S7 已按用户裁决不立项），并覆盖与其同工作树的 358 验收修复
- 审查结论：审查时 **partial**；处置后 **P0 全部 fixed，P1 除 P1-2 accepted boundary 外 fixed**（见 §六）。

## 一、P0

| ID | 问题 | 证据 | 处置 |
|---|---|---|---|
| P0-1 | S2 `structured.stderr` 恒为空串，属于伪造；且整体覆盖原 `structured`，可能丢失 background/timeout 元数据 | `agent_loop.rs` `enrich_tool_result` | 改为 merge；不写 `stderr`，写 `stderr_captured=false` |
| P0-2 | S3 `range` 未进入 `read_file` tool schema；且对 `[read handle]` 直接跳过——大文件场景恰好失效 | `agent_loop.rs` range 分支；`orz-tools` 无 `read_file` range schema | 把 `range` 落到 read_file 输入/工具层（含 read handle 分页），或删除该承诺 |

## 二、P1

| ID | 问题 | 证据 | 处置 |
|---|---|---|---|
| P1-1 | S1 handoff 只在 completed 路径写；failed/cancelled 无；写失败静默；内容缺关键文件/未决项 | `controller.rs` 仅 4697 一处调用 | 覆盖所有终态；失败 warn；补关键文件/测试读数 |
| P1-2 | S2 `tail`/`filter` 未在 tool schema 声明；PowerShell 噪音清理不完整；长任务流式沿用 auto-background | `orz-tools` 无 `"tail"/"filter"` 参数；实际输出仍有 `orz.exe : [permission]` 行 | 补 schema 或明确为内部后处理；补噪音模式；文档标注流式边界 |
| P1-3 | S6 主路径 `run_tests` 处于 R1 封存；cargo test 摘要未抓 `failures:` 下测试名 | `tools.rs` `run_tests` sealed；`test_failure_summary` 仅抓 FAILED/panic | 摘要补测试名；S6 活路径明确为 terminal cargo test，run_tests 分支保留为休眠 |
| P1-4 | S4 `orz resume` 未输出设计要求的“文档指针”；按 mtime 选最新 handoff，无 run/workspace 过滤 | `main.rs` `resume_summary` | 追加 `CLI_PROJECT_INDEX`/BACKLOG/接续档指针；输出 handoff 文件名 |
| P1-5 | 为让 projection 测试通过，把 `spawn_scout` 塞进“非工作族”豁免；`full registry` 期望列表加入 loop 合成工具，名实不符 | `retrieval/projection.rs`；`host_exec/tool_run.rs` | 改为“loop-synthesized family”专用分类；测试改称 declared face |
| P1-6 | 362 已标 `implemented`、计数 66→65，但上述问题未闭环，属过度收口 | `BACKLOG/TODO/INDEX` | 修复后重新评估；未修复前改回 partial/66 |

## 三、S5 备注

- `spawn_scout` 的 `params_schema` 已从 loop 单源导入，`schema_source=derived`；测试 2/2。
- 建议补一条与 `spawn_scout_tool_def().parameters` 完全相等的断言，防漂移；`derived` 的命名可在 coverage 中注明是 loop-derived。

## 四、测试与验证评估

- `orz-loop` 922/0/3、`orz-host` 453/0/6、`capability` 2/2、fmt 净：只能证明既有面未回归。
- `write_handoff`/`resume_summary`/`enrich_tool_result`/`strip_powershell_noise`/`apply_tail_filter`/`test_failure_summary`/`range` 均无单测；`orz resume` 仅手工验证。
- 5 个 projection 测试的期望列表被修改以容纳 `spawn_scout`；这是 358 probe 修复的测试适配，不是 362 的功能证明。

## 五、修复顺序

1. P0-1（结构化 merge/不伪造 stderr）→ P0-2（range 落 schema/工具层）。
2. P1-1（全终态 handoff）→ P1-4（resume 文档指针）→ P1-3（测试名摘要）。
3. P1-5（taxonomy 测试）→ P1-2（schema/噪音/边界文档）→ S5 断言。
4. 补单测与 resume/handoff 集成测试；最后更新账务（partial → implemented 或保持 66）。

## 六、处置记录

| 项 | 状态 | 处置 |
|---|---|---|
| P0-1 | ✅ fixed | `enrich_tool_result` 改为 merge 既有 `structured`；不再写伪造 `stderr`，改 `stderr_captured=false`；terminal cargo test 的 `failure_summary` 也 merge |
| P0-2 | ✅ fixed | `ReadFileInput` 新增 `range{start,end}`；`run_read_file` 在 read-handle 判定前映射到 offset/limit；删除 loop 侧重复切片 |
| P1-1 | ✅ fixed | completed + failed/cancelled/invalidated 终态均写 handoff；写失败 `tracing::warn`；handoff 保留编辑文件/失败测试 |
| P1-2 | ⚠ accepted boundary | 噪音清理补 ` : [permission]` 与 `gsa hygiene sweep:`；`tail`/`filter` 仍是 loop 内部后处理、未进 tool schema；长任务流式沿用既有 auto-background。设计稿已标注边界 |
| P1-3 | ✅ fixed | `test_failure_summary` 增加 `failures:` 下测试名与 panic 行；`run_tests` 仍 R1 封存，S6 活路径明确为 terminal `cargo test` |
| P1-4 | ✅ fixed | `orz resume` 追加 `CLI_PROJECT_INDEX.md`/BACKLOG/362 设计稿指针 |
| P1-5 | ✅ fixed | `spawn_scout` 改称 loop-synthesized family；测试改称 declared face；不再冒充 host registry |
| P1-6 | ✅ fixed | 审查文档落盘；362 保持 `implemented`/65，但边界在文档中如实标注 |
| S5 | ✅ fixed | 新增 `params_schema == spawn_scout_tool_def().parameters` 完全相等断言 |

**复审读数**：`orz-loop` 922/0/3、`capability` 2/2；`orz-host` 全量此前 453/0/6。P0 全部闭环；P1 除 P1-2 的“tail/filter 模型面广告/流式协议”作为明确边界外全部闭环。未提交。

