# 0ap／0ao 过夜实施批回执：压缩交互第九工具（`context_compress`）＋工具名单源全库收敛（2026-09-18）

> 批次性质：用户 2026-09-18 放行「0ap 全部（包括工具注册等内容，**不重建**）、0ao 全部」的
> 过夜长任务批。**工作树内完成，不重建、不提交、不推送**（提交/闭合入账随下一提交批，
> 按 0ai「产出在工作区未提交则项保持开放」先例）。
> 设计权威：[`COMPRESSION_INTERACTION_NINTH_TOOL_DESIGN_2026-09-18`](../COMPRESSION_INTERACTION_NINTH_TOOL_DESIGN_2026-09-18.md)
> （current-design，2026-09-18 用户审稿通过；工具名终版 `context_compress` 用户定名）。
> ADR 转录：ADR-0010 **§14.72 / v1.73**（8 工具面冻结例外 +2＋工具名单源）随本批落档。
> 基线：orz `07405e61`＋工作树在场 0am 影子 RLI 批（未提交，随 O2 裁决）——本批与 0am 批
> **hunk 级分离**（重叠文件仅 `blackboard.rs`／`controller.rs` 两件；`prompt.rs` 本批零改动）。

---

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 0ap 机制落码 | **全部完成**——九点位注册（§4 清单逐项）＋ D3 开窗接线（请求位/在程位原子通信＋loop-top 消费）＋滑块读数表（读时现算）＋`blackboard_read` 头水位标增段＋防抖三态＋归因分流＋子代理面剔除 |
| 0ao 收敛落码 | **全部完成**——字面单一源上移 `orz_assurance::tool_names`，全库生产判等面/文案面收敛，豁免清单（当前为空）＋机械扫描钉子 |
| S0 回查五项 | 完成（§2），四项 S0 发现如实登记（含 Python 镜像表格数据 +1 的必要性与设计 §4-6 口径解释） |
| S1 四钉＋样本钉 | 落码 5 枚（读数对账／响应三态／端到端链／权限桥放行链〔遍历＋桥测〕／声明面分类样本）＋**复核批补钉 2 枚**（并行批读数真值／归因映射，见 §8） |
| 测试读数 | orz-loop **796/0/3**（＝基线 791＋S1 钉 3＋复核钉 2）、orz-assurance **246/0**（＝244＋2 钉）、orz-host 串行 **332/0/5**（持平）——全绿 |
| clippy | 三 crate 全目标与基线**逐位持平**（orz-loop lib 50／lib test 68；orz-host 12/19；orz-assurance 4/6） |
| fmt | 本批文件全净（`rustfmt --edition 2024` 逐文件）；唯一残留 diff＝0am 影子批未提交件 `examples/rli_shadow_replay.rs`（先于本批存在，**不动**） |
| 环境口径 | FR-N02：执行 shell 实测零 `ORZ_*`/`GROK_*` 污染（`env` 直查）；FR-N03：`RUST_MIN_STACK=134217728` 常设 |
| 未做（按指示/判据边界） | S3 狗粮实证、单工作树复核（未提交态无可复核基线，随提交批）、提交与推送；**S2 已于 2026-09-18 放行完成**（0.6.2 双平台，见 [`062 载体重建`](062_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-18.md)） |

---

## 1. 0ao：重名面全库收敛（`GAP-TOOLNAME-LITERAL-CONVERGENCE`）

### 1.1 单源落位

- 新增 [`orz-assurance/src/tool_names.rs`](../../orz/crates/orz-assurance/src/tool_names.rs)：
  `pub const BLACKBOARD_WRITE_TOOL_NAME`／`pub const CONTEXT_COMPRESS_TOOL_NAME`（后者为
  0ap 新工具，自诞生即单源）。**落位理由**：依赖方向 orz-loop → orz-assurance ←
  orz-host，orz-assurance 是双方均可引用而不倒挂的唯一公共层（0ao 立项批明令「禁引依赖
  方向倒挂」）；工具名字面的语义权威本就是契约侧判官（`journal::toolsets::WORK_TOOLS`
  即在本 crate），同源落位。
- `orz-loop/src/blackboard.rs`：原 `pub(crate) const` 定义改为**零字面再导出**
  （`pub(crate) use orz_assurance::tool_names::BLACKBOARD_WRITE_TOOL_NAME;`）——既有
  引用形态（controller／agent_loop 经 `crate::blackboard::` 别名）零改动。

### 1.2 收敛面清单（字面普查 13 文件 → 分类处置）

| 文件 | 面 | 处置 |
|---|---|---|
| `tool.rs:82` | `READ_ONLY_EXEMPT_TOOLS` 注册表 | 常量化（+0ap `context_compress` 条目） |
| `tool.rs:180` | `action_category` 判等 | 常量化（+0ap 同族 "other" 折叠） |
| `tool_probe.rs:59` | `WORK_TOOLS` 探针表 | 常量化（+0ap 条目，24→25） |
| `tool_probe.rs:394` | 探针判据 match 臂 | 常量化（const pattern，+0ap 或臂） |
| `tool_run.rs:1119` | 派发判等 | 常量化 |
| `tool_run.rs:1127/1167/1204/1241` | 运行时错误文案 | `format!` 隐式捕获常量化 |
| `tool_run.rs:1305` | `plan_write` 事件 `plan_id` 载荷值 | 常量化（值不变，验证面零影响） |
| `tool_run.rs:2178` | plan 读尾段渲染标签 | `format!` 隐式捕获常量化 |
| `agent_loop.rs:4219/4255` | `[工作台]` 引导文案 | `format!` 隐式捕获常量化 |
| `context_scale.rs:403/415/432` | 提示/窗口/丢弃文案 | 常量化（432 同批改写双工具并存口径） |
| `permission.rs:593`（orz-host） | `access_kind` ReadOnly 臂 | 常量化（+0ap 臂） |
| `families.rs:56`（orz-assurance） | 判官工作工具表 | 常量化（+0ap 条目＋计数注释 25+4=29） |
| `blackboard.rs:20` | 原定义处 | 改零字面再导出 |

**豁免面（机械扫描钉子的分类放行）**：注释（行首 `//`）、`#[cfg(test)]` 测试模块
（花括号深度追踪）、定义文件非定义行。`LITERAL_EXEMPTS` 具名豁免表**当前为空**——
生产判等面全部收敛，生产文案面（format! 可拼接处）亦经隐式捕获收敛；新增豁免必须
在钉子里具名登记理由。

### 1.3 机械扫描钉子（`tool_names::tests`，2 枚）

- `tool_name_literals_live_only_at_definition_comments_tests_and_exemptions`：遍历
  全 crates 源树 `.rs`，标识符级匹配（防 `context_compress` 误命中 `context_compressed`
  事件名）——字面出现在定义处／注释／测试／具名豁免之外即报红；并断言每名**恰一个
  定义处**。
- `orz_loop_alias_is_a_reexport_not_a_second_literal`：blackboard.rs 必须是 `use` 再导出
  形态，不得回退为第二字面定义。
- 边界（如实登记）：测试区识别只解析标准 `#[cfg(test)] mod` 形态；行尾注释的不配对
  花括号可致深度失步——失步只会**漏报**（误判为测试行），定义处断言与空豁免表独立兜底。

### 1.4 0ao 判据核验

- 机械扫描钉子绿（字面仅存常量定义处；豁免清单空）✓
- 全部测试绿（清 env 口径，见 §5）✓
- clippy 与基线逐位持平 ✓　- fmt 干净（本批文件）✓

---

## 2. 0ap S0 现行面回查（取证落档，只读）

- **(a) model_selected 开窗现行触发路径**：现行「模型自选压缩」＝模型回复文本携带
  `[SEMANTIC_SUMMARY]` 语义摘要块（`context_scale::extract_model_summary` 识别，
  `agent_loop.rs` 响应处理段统一入列 `pending_semantic`），在下一 loop-top 安全边界由
  `compress_blocks_now` 按块压缩；`reason` 由 `window_opened_since_compaction` 单旗标
  决定（本 epoch 有 H1 窗口 ⇒ `context_scale_window`，否则 `model_selected`）。
  **无显式动作面**——本工具收编该路径为显式入口，自发路径保留不变；多路径二合一
  不适用（自发路径不是可收编的代码路径而是文本协议）。
- **(b) blocks=N 账源**：`model_face.rs` `blocks_outside_slider`（块号锚会话起点、
  `closed` 标志、`estimate_tokens` chars/2 估算）＋`face_markers`（v8 marker 反解
  Live/Compressed/Truncated）。**「已压缩」标记现成**（marker 幂等以块号为键）——
  零补账；可压缩判定与 `compress_blocks_now` 同尺（`closed ∧ Live`）。
- **(c) `blackboard_read` 头水位标渲染点**：`controller.rs::attach_pull_delta`
  （`[黑板 增量 水位【x.xM/10M】]` 头装配；增量徽章与域迁移段同方法）。滑块读数段
  以 `Option<&str>` 实参传入（执行点读时现算），头形态 `None` 时与 0ae D0 基线逐字一致。
- **(d) 注册面 census 与探针现值**：注册＝controller `run_turn_inner` 内联
  （`blackboard_write` 无条件 push 先例）；探针 `WORK_TOOLS` 现值 **24**（0aj 批
  23→24）⇒ 本批 24→**25**；判官表／Python 镜像同值同批。权限桥 `access_kind` 臂、
  `READ_ONLY_EXEMPT_TOOLS` 单源表、遍历式护栏（`read_only_tools_never_fall_into_the_edit_bucket`
  ＋代表参数表覆盖断言）均现成，按表登记即可。
- **(e) 锁序纪律位**：`attach_pull_delta` 既有锁序 blackboard → lif → cursors **不变**
  （滑块读数段在调用方执行点现算——纯函数零取锁）；`context_compress` 执行路径零新锁
  （原子位＋config 字段＋`messages` 切片）；loop-top 消费点的 `svc.blackboard.read()`
  与 H1 开窗点同形（临时读守卫）。
- **S0 发现（四项，均已按发现处置并如实登记）**：
  1. **Python 冻结镜像表格数据须 +1**：设计 §4-6 写「Python 镜像零改动」，但 §4-4
     探针 +1 的三处同步先例（0aj 批：探针表／判官表／Python `_WORK_TOOLS`）中镜像
     表格在列；且镜像的 probe-partition 子集校验（`extra = complete ∪ incomplete −
     _WORK_TOOLS` 即报错）在 Rust 表 +1 后**必然**对新工具报 `extra`。处置＝镜像
     `_WORK_TOOLS` 数据 +1（`assurance/run_event_journal_validation.py`，父仓），
     **判别规则零改动**——「零 Python 镜像改动」按「零规则改动」口径执行；此为
     设计稿内部张力点的机械裁决，随本回执登记。
  2. **归因单旗标不足**：现行 `window_opened_since_compaction` 单旗标会把工具发起
     窗口的压缩归因为 `context_scale_window`；设计 §1 明文该场景为
     `reason=model_selected`。处置＝新增独立旗标 `tool_window_since_compaction`
     （优先级高于 H1 旗标；压缩落地与 T1 截断两处随 epoch 复位），H1 通路行为不变。
  3. **子代理面透传**：`subagent_tool_projection` 为 denylist（`blackboard_write`
     先例透传子代理面），而 controller 请求位跨车道共享——子代理声明会在主车道误
     触发开窗。处置＝denylist 显式剔除 `context_compress`（**main-lane only**；D3
     窗口为主车道结构，DP-7）。
  4. **教学面落位**：主车道系统提示为空串（THIN-HARNESS V2，`build_system_prompt`
     仅回探针块），无「提示词一句教学」的常驻落点。处置＝描述自包含教学（~140 字符，
     设计「≤120 字符目标」的贴近值，多出部分为 `[SEMANTIC_SUMMARY]` 协议必要教学；
     常驻成本读数 S3 照收）＋H1 窗口块内一句（读数/再发起指引，零常驻成本）。设计
     §7「窗口轮并存形态微调（S1）」开放点就此定案（窗口轮双工具并存＋文案双工具口径）。

---

## 3. 0ap 机制落码（九点位逐项）

| # | 点位 | 落码 |
|---|---|---|
| 1 | controller `run_turn_inner` 注册 | 无条件 push（any-guard 沿先例）；描述自包含教学；参数 `{"type":"object","properties":{}}` |
| 2 | 权限桥 `access_kind` 内存类 arm | `permission.rs` +`context_compress` 臂（ReadOnly 全策略自动放行；0aj 同形第三例的预防登记） |
| 3 | `READ_ONLY_EXEMPT_TOOLS` 单源表 | 收录（`risk_class` ReadOnly；宿主遍历护栏＋代表参数表 `{}` 自动覆盖） |
| 4 | 探针面 `WORK_TOOLS` +1 | 24→25；probe 判据共用 `probe_storage`（纯内存会话状态同族）；三处同批（探针/判官/Python 表） |
| 5 | 声明面分类护栏 | 遍历式自动纳入；`declared_face_tools_are_work_tools_or_rule_based_non_work_families` 补 0ap 正例样本断言 |
| 6 | journal/conformance | 零新事件族（复用 `context_compressed`＋`tool_started/completed`）；Python 判别规则零改动 |
| 7 | schema | 零改动（`ToolCompleted` 载荷仅 `tool`/`call_id`/`exit_code`；计数走响应文本面） |
| 8 | 压缩窗口轮工具面 | `compression_window_tool_defs` 双工具并存；窗口轮调用筛分同步放行（窗口内调用＝in-progress 读数） |
| 9 | 工具名 regex | `context_compress` 合规（`^[a-zA-Z0-9_-]+$` 与 MCP 面 `^[a-zA-Z_][a-zA-Z0-9_-]{0,63}$` 均过；后者为 orz-mcp MCP 工具校验器，不涉常驻面） |

**执行链**（`tool_run.rs`，同 `compaction_whitelist_add` 形态：权限门后、`ToolStarted`
后内联拦截、无宿主派发）：读数现算（`model_face::slider_readout(messages,
slider_window_tokens, model_face_block_tokens)`——串行派发传会话 `messages`，取证 §2(c)）→
三态判定（在程位原子读取／N==0 中性／否则 `request_compression_window()`）→ 响应文本
（`context_scale::context_compress_response`，三态共用读数渲染）→ `ToolCompleted`
＋Tool 消息＋动作区盖章，exit 0。

**开窗链**（`agent_loop.rs`）：请求位原子（controller 字段；`&self` 跨派发段通信）→
loop-top 消费点（GAP-INQUIRY-SPLIT 注入点之前；`pending_checkpoint.is_none()` 才取走
＝锁存顺延；gate `AgentRole::Main`）→ `PendingCheckpoint::ModelCompression`（≤3 轮、
`window_start_writes` 同 H1）→ `window_kind = Some(ModelRequested)`（**复核批 §8②**：
归因随摘要固定，不再用「epoch 内开过哪个窗口」旗标）→ 在程位同步
（current_tool_defs 装配前，值＝本迭代 pending 快照）→ 窗口面双工具 → 收口统一出口
照旧。归因分流见 §2 发现 2。

**主同步面**：`blackboard_read` live 读（非 epoch、非渲染错误形状）头增段
「滑块外可压缩 N 块 ≈ est K」（`estimate_tokens_label`：≥1M 一位小数 M／≥1K 取整 K）；
`None` 路径（epoch 读/错误形状/既有测试）头形态逐字不变。**读数会话视图（复核批 §8①）**：
段文本由 `readout_conversation(messages, conversation)` 现算——串行路径＝消息槽本体，
同轮读类并行批＝**批首会话**（并行批的 `messages` 只是本调用的注入槽，批首为空 `Vec`）。

## 4. S1 判据钉（5 枚＋复核批 2 枚，全绿）

1. **读数一致性对账**（`model_face::tests::slider_readout_matches_block_and_marker_state`）：
   读数与 `blocks_outside_slider` ∩ `face_markers`（closed ∧ Live）逐块对账；压缩
   marker 落地后恰降对应块的数量与估算；整段在滑块内 ⇒ 全零读数。
2. **响应信封三态**（`context_scale::tests::context_compress_response_covers_the_three_debounce_states`）：
   Requested／InProgress（不得再次宣请开窗）／NothingToCompress（未开窗）；三态同表。
3. **端到端链**（`agent_loop::tests::context_compress_end_to_end_request_window_summary_and_selected_reason`）：
   调用→请求→开窗（下一 loop-top）→窗口面双工具并存→窗口内再调用 in-progress 防抖
   （有窗口调用 ⇒ 延迟收口，`model_participated=false` 如实）→出窗语义摘要→
   `context_compressed{mode=model_summary, reason=model_selected}`。0aj 同形端到端钉。
4. **权限桥放行链**：遍历护栏 `read_only_tools_never_fall_into_the_edit_bucket` 经
   代表参数表自动覆盖新工具（缺 `access_kind` 臂或漏样本即红）＋只读策略桥测列表 +1。
5. **声明面分类护栏样本**（`projection.rs` 护栏测试 ③）：`context_compress` 正例断言
   （探针面漏网即红，防 0aj 同形第三例）。

另附 ⑥：`attach_pull_delta` 头增段渲染断言（Some 段落拼装／None 基线不变）。

复核批补钉（2026-09-18，§8）：

6. **并行批读数真值**（`agent_loop::tests::blackboard_read_slider_readout_reads_the_parallel_batch_conversation`）：
   同轮读类并行批（0k）内 `blackboard_read` 读数＝**批首会话**而非本调用空注入槽；
   钉含并发峰值断言（证明批确实并行）＋读数 ≥1 断言。**修复前实跑红**：头渲染为
   `[黑板 增量 水位【0.3M/10M】 滑块外可压缩 0 块 ≈ est 0]`。
7. **归因映射**（`agent_loop::tests::semantic_compression_reason_maps_window_kinds`）：
   H1 ⇒ `context_scale_window`；工具窗口与无窗口（模型自选）⇒ `model_selected`。

---

## 5. 验证读数（基线 → 本批后）

| 项 | 基线（07405e61＋0am 工作树态） | 本批后 |
|---|---|---|
| orz-loop `--lib` | 791 passed / 0 failed / 3 ignored | **796 / 0 / 3**（+3 S1 钉＋2 复核钉；796 为主会话复核批独立复跑值，过夜批落码时为 794） |
| orz-assurance `--lib` | 244 / 0 | **246 / 0**（+2 钉） |
| orz-host `--lib`（`--test-threads=1`） | 332 / 0 / 5 | **332 / 0 / 5** |
| clippy（`-p orz-loop -p orz-host -p orz-assurance --all-targets`） | lib 50/68、12/19、4/6（loop/assurance/host × lib/lib-test） | **逐位持平**（50/68、12/19、4/6） |
| fmt（`cargo fmt -p 三crate -- --check`） | 0am 件 1 文件既有 diff | 本批文件全净；残留仅 0am 件（先于本批） |

- 环境口径：FR-N02（清 `ORZ_*`/`GROK_HOME`/`GROK_AGENT`）——执行 shell 经 `env` 直查
  **零污染**（污染源＝狗粮 launcher 预置，非本 shell）；FR-N03 `RUST_MIN_STACK=134217728`
  常设；orz-host 串行。
- 基线原始读数：`.tmp-0ap0ao-baseline.log`／`.tmp-0ap0ao-clippy-baseline-scope.txt`
  （orz 根目录，`.tmp` 前缀门禁豁免）。

## 6. 既有环境观察（先于本批、不在本批处置面）

1. `cargo clippy --workspace --all-targets` 在 `orz-tools-api v0.1.220-alpha.4` 构建脚本
   失败（`failed to run custom build command`，基线即失败）——本批 clippy 以三 crate
   scope 执行（其闭包构建全绿）；workspace 级失败留环境面复核。
2. 0am 影子批未提交件 `examples/rli_shadow_replay.rs` 存在 fmt diff（0am 批遗留态）——
   本批不触碰（hunk/文件级分离纪律）。

## 7. 边界与后续

- **未提交态的复核边界**：S1 判据中的「单独工作树复核（ORZ-BUILD-MOUNT-001 挂载）」
  以已提交基线为对象——本批工作树内完成，该复核**随提交批**执行（先例：07405e61
  单独工作树复核 786/0/3）。
- **hunk 级分离登记**：本批与在场 0am 影子批重叠文件仅 `blackboard.rs`（常量→再导出
  hunk）与 `controller.rs`（字段/构造/方法/注册/attach_pull_delta hunk）；提交批按
  07405e61 先例做 hunk 级分离，0am 批维持未提交随 O2 裁决。
- **S2 已放行并完成（2026-09-18，用户令「请进行重建吧，双平台」）**：0.6.2 双平台重建换装＋
  字面量核证（Windows `orz.exe` 53,989,888 B `d33d904b…`／Linux `orz` 111,394,672 B
  `d14d6d9c…`；ACAF 按 052/061 先例重 provision）——同日后续批按用户令
  「请提交并推送吧，双平台安装包也发布上去」完成：本批随 **orz `ad8c0da3`**（实现）／
  **`08ab194c`**（bump 0.6.2 源冻结）／父仓 **`ffdde3b3`** 入库并推送（远端
  `feat/fusion-architecture`／`main` 均已同步），**GitHub Release v0.6.2** 双平台包
  （zip 27,511,203 B `8d158150…`／tar.gz 35,770,317 B `d7b8e0c4…`）已发布且
  本地↔服务端↔下载重哈希三方一致。详见
  [`062 载体重建 §9`](062_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-18.md)。
- **待放行**：S3 狗粮实证（单轮如实标注——读数表**括号内总数**与 journal
  `blocks=N` 一致〔主数＝可压缩子集 `closed ∧ Live`，与 `blocks=N` 不同量，只作
  advisory〕／模型经新工具完成至少一次 model_selected 压缩／常驻成本读数评估）。
- **账本**：计数维持 37（0ao 判据虽已达成，按 0ai 先例**闭合入账随提交批**；0ap 本就
  开放至 S2/S3）。索引头行与 §8 状态桶随提交批更新（§0.5：实现进展不改召回路由不更新
  索引）。
- **提交批清单（在 §7 上列之外须逐项完成）**：①索引头行新增本批复述＋§6 两处**句中
  措辞同步**（`DESIGN-COMPRESSION-INTERACTION` 状态句「实施未开始，S0/S1 待放行」→
  实际进展；同句「零 Python 镜像」→「零判别规则改动〔表格数据 +1〕」）＋§8 桶
  （`GAP-TOOLNAME-LITERAL-CONVERGENCE` 出 `pending`）；②提交**必须用显式 pathspec**、
  不得 `git add -A`（工作树含 `.tmp-*` 过程件与 0am 未提交批；`.tmp-*` 已入两侧
  `.gitignore`，但显式 pathspec 仍是纪律）；③提交后按 0ai 先例闭合 0ao 判据（35 → 36）
  并重算 `orz_source_manifest.sha256`。

---

## 8. 主会话只读复核批处置（2026-09-18；用户令「不考虑最小修复，仅最优修复，处理全部问题」）

复核以只读方式独立复现读数（orz-loop 794/0/3、orz-assurance 246/0、orz-host 332/0/5、
clippy 逐位、fmt 仅 0am 件、门禁 `error_count=1` 唯一＝orz 子模块脏）、独立扫描 0ao
字面面（生产面零残留，残留枚举项全在注释/`#[cfg(test)]` 区内）与契约面（schema
`complete` 为裸字符串 ⇒ 加工具零 schema 改动；Python `_WORK_TOOLS` +1 由
probe-partition 子集校验坐实为机械必需）。查出并**全部处置**的问题如下。

| # | 级别 | 发现 | 处置 |
|---|---|---|---|
| 1 | **P1（功能）** | 同轮读类并行批次（0k）内 `blackboard_read` 的滑块读数段按该调用的**空注入槽**现算 ⇒ 恒定渲染「滑块外可压缩 0 块 ≈ est 0」（与设计 §3「读时现算」及 S3 判据冲突） | 工具执行面新增**显式只读会话视图**：`host_exec/tool_run.rs` 增 `readout_conversation(messages, conversation)` + `run_host_tool_with_plan_gate`／`run_host_tool_with_timeout` 增 `conversation: Option<&[Message]>` 形参；并行批传**批首会话**（`agent_loop.rs`：`batch_conversation`），串行路径传 `None`（消息槽即会话），其余 11 处调用面显式 `None`（dispatch×4／serp×6／console_exec×1）。读数同尺保证不再依赖「该工具恰不在并行集内」的隐式前提；补真值钉（修复前实跑红，见 §4-6） |
| 2 | **P2（审计面）** | 归因旗标残留：工具窗口**未产出摘要**即收口时不复位 ⇒ 其后 H1 窗口产出的压缩被记 `reason=model_selected`；另「落地前恰有 H1 开窗」会抢走已产出摘要的归因 | 归因改为**随摘要产出固定**：新增 `CompressionWindowKind{Ladder,ModelRequested}`＋`semantic_compression_reason`；`pending_semantic` 记 `(摘要, 促成窗口)`；`window_kind` 随开窗置位、窗口不在程清空；两个消费点（loop-top／run 尾）改用记录值；补归因映射钉（§4-7） |
| 3 | **P2（设计-实现）** | 设计稿 §0「读数不可得 ⇒ 中性返回」与 §1「窗口开不起（守卫越线等）⇒ 结构化原因」未落码，且 v8 勘误已把「越线不开窗」退役为 T1 硬截断 ⇒ 该分支不可达而未登记 | 设计稿 §0 防抖行／§1 第 3 条就地改为**实装三态**（Requested／InProgress／NothingToCompress）并写明 v8 退役依据；ADR-0010 §14.72 第 4 项同步 |
| 4 | **P2（设计稿措辞）** | 设计稿 §0 契约面行与 §4-6 仍写「零 Python 镜像改动」，与实现（镜像 `_WORK_TOOLS` 表格 +1）及 §7/ADR 记录自相矛盾 | 就地收窄为「零**判别规则**改动（表格数据 +1＝probe-partition 子集校验的机械必需）」；§4-1 补 S0 教学面处置注；索引 §6 同句列入提交批清单（§7） |
| 5 | **P2（样本失真）** | 0aj-review 先例要求工作工具表 +1 时同步 fixture 的 `complete` 面（否则「全分区」样本失真）；本批未同步 | `generate_run_event_fixtures.py` 两处 `complete` 面补 `context_compress` 并**重跑生成器**：340 件中**仅 3 件**按预期变化（v0.2 两 payload＋一 envelope，逐文件 SHA256 前后对照，零新增/零删除） |
| 6 | **P3（口径）** | S3 判据「读数表与 journal `blocks=N` 一致」口径含混：读数主数＝可压缩子集（`closed ∧ Live`），journal `blocks=N`＝主滑块外**总**块数 | 设计稿 §6-S3／TODO／本回执统一为「**括号内总数** ↔ `blocks=N`；主数只作 advisory」 |
| 7 | **P3（打磨）** | `context_scale.rs` 新增 `use` 插在 doc comment 与常量之间（常量文档注释被错挂到 `use` 上）且 import 落文件中部 | import 归位至文件头部（与既有 `use` 同区），常量恢复自有文档注释 |
| 8 | **P3（卫生）** | 24 个 `.tmp-*` 过程件未被 `.gitignore` 覆盖（`tmp*/`、`*.tmp/`、`tmp_*` 均不匹配点号前缀），提交批若 `git add -A` 会被扫入 | 父仓与 `orz/.gitignore` 各加 `.tmp-*`（保留文件作证据，不做删除）；提交纪律写入 §7 清单 |
| 9 | 已核无问题 | 0ao 单源收敛、契约面零事件族/零 schema、批次 hunk 级分离面（真重叠＝`blackboard.rs`＋`controller.rs`）、读数尺同源（配置同一实例、判定同尺） | 无需改动；本回执留证 |

**复核批读数（主会话独立复跑）**：orz-loop `--lib` **796/0/3**（＝794＋复核钉 2）、
orz-assurance `--lib` 246/0、orz-host `--lib --test-threads=1` 332/0/5；
clippy 三 crate 全目标 50/68、12/19、4/6（**逐位持平**）；`cargo fmt -p 三crate -- --check`
仅 0am 件既有 diff；`git diff --check` 两仓干净；门禁 `error_count=1`
（唯一＝`orz submodule working tree is dirty`，未提交态预期）；fixture 重跑前后
340 件、变更恰 3 件。

**边界**：本次处置仍在工作树内（不提交、不推送；**S2 载体重建已于 2026-09-18 单独放行完成**
＝0.6.2 双平台，见 [`062 载体重建`](062_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-18.md)）；
0am 影子 RLI 批维持未提交随 O2 裁决；S3 狗粮实证待放行。
