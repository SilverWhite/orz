# 354 批落码：P0-3 收口＋T4b 心跳＋354b 控制台骨架＋354c 收口（0dq/0dr）

> 日期：2026-10-25
> 用户令：「请继续 P0-3 和 0dr 的剩余内容吧，先把 354 批都做完；做完以后请暂时不做重建；可派出子代理协助工作」
> 设计权威：[`354 设计稿`](../SYSTEM_CONSOLE_DESIGN_2026-10-25.md)（§3 三层／§4 专项／§7.5 复杂度预算／§7.6 子代理裁决）
> 执行形态：只读 scout×3（P0-3 wrapper 覆盖勘定／grep 游标与信封勘定／CLI 面与沙箱勘定）＋主车道统一落码（§7.6 纪律：scout 只在取证面）。

## §0 结论

- **P0-3（0dq）收口**：353 取证结论（进程内看门狗救不了预算内死亡）之上的剩余核查完成——**所有 loop 车道本就收敛同一 wrapper**（loop select! `tool_run.rs:4215` ＋ host 统一窗 `lib.rs` `tokio::time::timeout`）；唯一缺口是 host 侧 **4 条早退特例**（project_doc_index／browser_read／browser_control／pdf_read／web_fetch-PDF 浏览器路由）在统一窗之前 return。已补 `bounded_host_owned` 统一墙钟（同预算同 `ToolError::Timeout` 形状；早退路径无本次派生子进程树，超时不杀树只解卡）。run_tests 保留独立 30min TestRunner 预算（by-design 分账）。**0dq 验收四条全达成，闭合待用户裁决。**
- **T4b（354a）**：`tool_heartbeat` 事件全链落地——EventType 变体＋`run_host_tool_with_timeout` 心跳臂（默认 20s，`ORZ_TOOL_HEARTBEAT_SECS` 可调 0=禁用，跳过 elapsed≈0 首 tick）＋父仓契约面（payload schema／registry／信封枚举／fixture 三件，`prompt_submitted`／`supplement_delivered`「无法官族」同姿态）。模型面零注入（§7.5 B 暴露失败）。
- **354b**：①grep 真游标——`offset` 参数（上限 100_000）＋读取预算抬到 offset+limit（+1 探测）＋窗口切片＋`[page] total/shown/next_cursor` trailer（精确值/≥下界如实分流）＋流式同构＋console schema；②inspect 四面（`orz-host/src/inspect.rs`：session／run／context／events，只读投影＋Envelope v1 信封＋分页游标）；③`orz capabilities`（`tools.rs::capability_list`：name/kind/params_schema/needs_approval/readonly/one_line_usage，risk_class 机械判定）；④`orz` 二进制两子命令接线（tokio runtime 内枚举）。
- **354c**：①读面沙箱撤销（`is_path_allowed_for_read` 单点：workspace 外放行；技能根豁免与会话卷窗口契约不动）＋敏感面提示 `[敏感面] …`（`.ssh/.aws/.env*/*.pem/*.key/keystore`，只提示不拦截）；②outline 四族语言（Rust/Python/TS-JS/Markdown，纯行扫描零语法树）；③emoji 回执可解释（`[emoji 剥离] N 处：U+XXXX@L…`，样本 8 条封顶）；④审批模式可见（`[permission] mode=… source=…` 每 run 一行拼系统提示，`permission.rs` 单一实现，build_host 装配）。
- **附带修复（350 批遗留）**：`families.rs` 两处 parity 内联脚本改走包导入（`import assurance.run_event_journal_validation`）——350 批 `5805355e` 把 assurance 收成包（`from .contracts import` 相对导入）后，独立模块导入崩溃（os error 109 断管），Rust↔Python parity 门自此红。
- **未做（登记边界）**：Envelope v1 对模型工具面（read_file／run_terminal_cmd／blackboard_read）的整面字段名统一——三面各自已有等价 meta（ReadHandle 信封／`[output]` 三读数＋`[page]`／折叠语义），信息等价、纯形状统一，§7.5 预算下不冒险热路径；**0dr 保持 open，闭合随该边界一并待用户裁决**。载体重建未做（用户令）。

## §1 P0-3：wrapper 覆盖核查与收口

Scout 勘定（file:line 证据见 scout 报告归档）：

| 车道 | wrapper | 判定 |
|---|---|---|
| 主循环串行/并行批、grill、检索子代理、console direct | loop select（`tool_run.rs:4215`）＋host 统一窗（`lib.rs:1512`） | ✅ 本就同 wrapper |
| run_tests | host 自有 `tokio::time::timeout`（`lib.rs:2114`，30min） | ✅ 独立预算 by-design |
| **project_doc_index / browser_read / browser_control / pdf_read / web_fetch-PDF 路由** | 统一窗**之前** return，无任何墙钟 | ❌→✅ 本批收口 |

修复：`bounded_host_owned(name, budget, fut)` 自由函数（`lib.rs`）——`effective_timeout` 计算上提，四条早退统一包窗；差异如实（无子进程树可杀，超时只解卡）。单测 `bounded_host_owned_passes_through_and_times_out`（快路径透传＋20ms 掐 5s 挂死）。

## §2 T4b：`tool_heartbeat` 全链

- **Rust**：`EventType::ToolHeartbeat`（journal/event.rs，注释载语义与预算）；`run_host_tool_with_timeout` 等待循环新增条件臂（`Option<Interval>`，`journal_tick_ms = tool_heartbeat_tick_ms()`：缺省 20 000ms、`ORZ_TOOL_HEARTBEAT_SECS` 覆写、0=禁用、非法值回退缺省）；payload `{tool, call_id, elapsed_ms}`；TUI bridge 降级 `Unknown`（BudgetCueInjected 先例）。
- **父仓契约面**：`runtime/tool-heartbeat-event-payload-v0.2.schema.json`（required 三字段＋additionalProperties:false，description 载「无法官族」姿态）；registry v02 `tool_heartbeat → slug tool-heartbeat`；信封 schema 枚举 +1；fixture 生成器注册（GOOD/BAD 正负契约）并生成三件 fixture；`run_event_journal_validation.py` v02 计数注记 42→43；conformance 叙事注记 69→70。Python 合规测试 17/17。
- **测试**：`tool_heartbeat_journaled_while_tool_in_flight`（env=1s 节拍、慢工具 2.3s→≥1 行心跳、elapsed_ms 单调、事件序 started→heartbeats→completed）＋`tool_heartbeat_tick_ms_parses_env`（缺省/0/数值/非法值四态）。

## §3 354c：四件收口

1. **读面开放**（§4.3，用户已裁决方向）：`is_path_allowed_for_read` 分支 4 → `true`；注释载明保留边界（技能根豁免、会话卷 D3/0ct 窗口契约＝内部状态保护非读面沙箱）。`sensitive_path_notice` 新增（命中 `.ssh/.aws/.env*/*.pem/*.key/keystore` 组件 → `[敏感面] …` 一行，不拦截）；read_file 渲染两臂前置、grep `<workspace_result>` 首行注入。**测试翻转八例**（read_file×3／grep×2／list_dir×2／resources×2 改判为开放断言＋敏感提示断言），`sensitive_path_notice_matches_key_material_hosts` 新增。
2. **outline**（§4.7）：`build_outline` 按扩展名分流四族（rs／py/pyi／ts|tsx|js|jsx|mjs|cjs／其余→Markdown）；`outline_take_name` 切到首个结构性字符（`Foo<T>`→`Foo`、`fmt(&self…)`→`fmt`）；`impl` 头部整行携带；条目 80 字符截断、300 条封顶不变。测试＋3（Rust/Python/TS-JS 各一轮）；描述行更新。
3. **emoji 可解释**（§4.8）：`StripNotice.samples`（每 run 首码点＋行号）；render → `[emoji 剥离] N 处：U+XXXX@L…（行号为写入内容内部 1 基行）`，样本 8 条封顶省略。消费面断言三处翻转（search_replace×2／hashline×1）＋emoji_strip 内部两处。
4. **审批可见**（§4.6）：`LoopHost::permission_mode_line`（默认空串）；`permission.rs::permission_mode_line(policy)` 单一实现（readonly/benchmark/interactive×ORZ_YOLO 四态——缺省/显式开/显式关/非法值分别如实）；`build_host` 装配（`with_permission_mode_line`）；agent_loop 系统提示拼接（空串零注入、常量跨轮不动前缀缓存）。单测 `permission_mode_line_covers_policy_and_yolo_states`。

## §4 354b：控制台骨架

1. **grep 真游标**（§4.2／T3 改判）：`GrepSearchInput.offset`（0 基，上限 `MAX_PAGE_OFFSET`=100_000）；`GrepFormatConfig.page_offset`＋`read_budget()`（offset+limit）；阻塞/流式两条读取预算同步抬升；`finalize_grep` 窗口切片＋`[page]` trailer（`total_exact = !stdout_truncated`：rg 自然结束→精确 total，预算截断→`≥{seen}` 下界；has_more→`next_cursor: offset+limit`）；`BodyStreamer::push_line` 跳行同构（窗口前行不进体不占 limit）；流式等值测试的 `[page]` 剥离口径（card_body 尾序剥离：先 page 后 footer）；console.rs ActionSpec＋`offset` 属性。测试 `grep_offset_pagination_covers_all_matches_without_overlap`（5 命中×2/页 三页，不重不漏）。**边界如实**：模型面 total 在预算截断时是下界——精确 total 属 workspace 服务端路径（`content.rs` 全量读）。
2. **inspect 四面**（§3.2）：`orz-host/src/inspect.rs`——`session_overview`（消息计数/水位/未回收调用（P0-2 同口径只读投影）/recall_log/租约与墓碑）、`run_timeline`（事件流聚合：模型轮/工具 per-call_id（started/completed/exit/heartbeats）/审批/终态）、`context_view`（窗口近似投影 `source: sidecar`＋最新 run 的 face_fingerprint）、`events_stream`（passthrough＋`--limit/--kind/--cursor` 分页，next_cursor=已读行数）。全部 Envelope v1 信封（`{ok,data,error,meta}`）。测试 4 例（聚合/悬挂/分页/缺席报错）。
3. **capabilities**（§3.1）：`tools.rs::capability_list(cwd)`——生产 `build_toolset` 装配（web_search 凭据缺席＝Disabled 形态、仍在面上）＋`ToolDispatcher::risk_class` 机械分档（read/write/exec、readonly、needs_approval=!readonly）＋`one_line_usage`（描述首行）。`orz capabilities`（人读表格）／`--json`（Envelope）双形态；tokio runtime 内枚举（TerminalBackend 装配需要）；版本字段以 bin 为准。
4. **Envelope v1**（§3.3）：inspect/capabilities 为首批公民（字段表 §5 全型）；模型工具面迁移登记边界（§0 未做）。grep 的 `next_cursor` 已是 Envelope 分页语义在工具面的第一个落地。

## §5 读数

- orz-tools：`cargo test -p orz-tools --lib` **3052 passed / 0 failed**（grep 106/0 含分页钉、read_file 234+3、resources 88 含敏感面钉、emoji 17）。
- orz-loop：全套绿（心跳 2 例新钉；`tool_heartbeat_journaled_while_tool_in_flight` 2.8s）。
- orz-host：串行全绿（P0-3 单测＋permission_mode_line＋inspect 4 例新钉）。
- orz-assurance：**309/0**（含修复后的 Rust↔Python parity 2 例）。
- Python：`runtime/tests/test_run_event_conformance.py` **17/17**（tool_heartbeat 注册后）。
- 实跑：`orz capabilities`（8 工具分类正确）／`--json`（Envelope 全型）／`inspect session <缺席>`（结构化缺席事实）。
- clippy 零新增；fmt 净；`check_repository` 在 orz 提交＋manifest 重生成后回绿。

## §6 边界与剩余

- **未重建载体**（用户令）：本批全部改动在源仓与 `orz` 二进制 dev 构建，未 bump 版本、未重出 Windows/Linux 载体、未换装。
- **Envelope 工具面统一**：0dr 剩余边界（§0）；0dq 全达成（闭合待用户裁决）、0dr 保持 open。
- **两仓提交未推送**（与前批在途状态一致）。
