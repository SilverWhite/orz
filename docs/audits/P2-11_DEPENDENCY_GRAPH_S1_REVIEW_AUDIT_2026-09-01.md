# P2-11 依赖图主线 S1 全面审查处理记录

> 日期：2026-09-01；主题：P2-11 第 4 项依赖图 S1 全面审查（设计合理性 /
> 实现合理性 / 设计与实现符合性）发现项处理；状态：**S1 审查处理收口**
> （设计语义不变，实施仍 `partial`——S3 重建 + S4 实机复验待放行）；
> 审查对象：`docs/DEPENDENCY_GRAPH_MAINLINE_DESIGN_2026-09-01.md` +
> `orz-loop/src/dep_graph.rs` / `blackboard.rs` / `controller.rs` /
> `host_exec.rs` + `runtime/tool-completed-event-payload-v0.1.schema.json` +
> `assurance/run_event_journal_validation.py` + 测试与文档登记。

## 1. 发现与处理

| # | 严重度 | 发现 | 处理 |
|---|---|---|---|
| 1 | 中 | `render_text` 字节超限时无条件追加 `truncated=true` footer，bytes 接近上限时会把返回值推超 8 KiB；截断路径无测试 | **已修**：正文预算先扣除 footer 长度（`dep_graph.rs`），footer 完整落盘后总长仍 ≤8 KiB；补 `render_truncates_within_byte_cap_and_keeps_footer` 单测 |
| 2 | 轻 | `host_exec.rs` 非字符串 section 错误文案枚举缺 deps/temporal，与工具定义枚举不一致 | **已修**：文案补齐 `entities|deps|temporal` |
| 3 | 轻 | verifier docstring 自称校验 sha256 64-hex/字段类型，实际只查 object/null（schema 兜底）；`reads` 字典存未用 index | **已修**：docstring 改为「字段类型/sha256 hex 由 schema 兜底，本函数只查 object/null 与跨事件一致性」；移除未用 index |
| 4 | 轻 | 设计 §2 匹配降级措辞「read 无 sha256 时」与实现「任一方缺 sha256」不一致 | **已修**：设计措辞统一为「任一方缺 sha256 时」，并注明与 verifier/写前核证口径同构 |
| 5 | 轻 | 设计 §4「事实按 seq 升序」与实现「同一文件分组内先 read 后 write」不一致 | **已修**：设计措辞对齐实现 |
| 6 | 轻 | 设计 §4 truncated 语义「事实超容量（已淘汰）」与实现「len==cap 即标」不一致 | **已修**：设计措辞改为「队列已满（len==cap，继续登记将淘汰最旧；满而未淘汰亦标记，保守口径）」 |
| 7 | 轻 | 设计 §5.3「有字段即校验类型」与 verifier 实际职责（object/null + 跨事件）不一致 | **已修**：设计措辞改为分层表述（字段类型/sha256 hex 由 schema 兜底） |
| 8 | 建议 | 测试缺口：字节超限截断路径、read 锚点 None、deps 带 epoch/receipt_id 错误形状、verifier size+mtime 回退 | **已补**：Rust 单测 +2（渲染截断、read 锚点 None 不链接）、controller 单测 +1（deps 组合错误形状）、Python 用例 +2（size+mtime 匹配/不匹配） |
| 9 | 建议 | 锚点计算成本：每次成功 read/write 全量重读 ≤16MB 文件 + sha256，与实体登记/写前核证重复 | **登记**：设计 §8 边界登记，维持同口径不动，后续可复用 host 结果锚点 |
| 10 | 建议 | `dep_graph` 字段 `#[serde(default)]`：当前 EpochSnapshot/StoredConversation 均不序列化整黑板，live-only 成立；未来全黑板序列化路径会泄漏 | **登记**：设计 §8 序列化面登记，与 entities 先例一致暂不改，未来需改 `#[serde(skip)]` |

## 2. 验证结果（2026-09-01 实测）

| 项 | 结果 |
|---|---|
| orz-loop lib | **645 passed / 0 failed / 3 ignored**（基线 642 + 新增 3；含 dep_graph 7 单测 + host_exec 2 集成 + controller 1） |
| Python journal validation | **242 passed**（基线 240 + 新增 2，含 DepGraph 10 项） |
| Python conformance | **15 passed**（schema fixtures 未动，回归确认） |
| fmt | `cargo fmt -p orz-loop -- --check` 干净 |
| clippy | 无新增（全部告警为既有基线：acaf/action_ledger/agent_loop/retrieval/epoch/gateway/host_exec/planning/summary/lif 等既有代码） |
| git diff --check | 父仓与 orz 双仓干净（仅 CRLF 提示，无空白错误） |

## 3. 处理边界

- 只做审查发现项收口：不改变设计语义、事件面 schema、工具面、GetPut
  行为与渲染格式（除截断 footer 预算修复外输出内容不变）。
- 状态维持 `partial`：S3 重建 + S4 实机复验仍待用户放行；闭合时计数
  28 → 27（TODO/BACKLOG 同步）。
