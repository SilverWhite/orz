# 机械编码门控 实施审计（2026-08-13）

- 审计对象：GAP-ENCODING-GATE — orz 机械编码门控（固定解码链 + `output_encoding` 事件字段）
- 日期：2026-08-13
- 范围：`orz/crates/codegen/orz-tools`（新 `util/encoding` 模块 + 终端/read_file 接线）、`orz/crates/codegen/orz-hooks`（命令 runner 输出）、`orz/crates/orz-host`（run_tests + `call_tool` 元数据映射）、`orz/crates/orz-loop`（`ToolResult`/`TestRunResult` 字段 + controller journal payload）、`runtime/tool-completed-event-payload-v0.1.schema.json`（Schema 先行扩展）
- 契约：`OPS-PROTOCOL §8`（2026-08-13 定稿）；参考实现 `assurance/ops_executor.py::decode_text`（标签逐字对齐）
- 验证摘要：orz-tools terminal **37/0/5**（ignored 为既有）、read_file **91/0/0**（+2 新）、bash **149/0/0**（+1 新）、hooks **20/0/0**；orz-host **+1 新全绿**；orz-loop **+2 新全绿**；pytest runtime conformance **14/0**；check_repository **valid**；`git diff --check` 干净。注：orz-tools 全量 lib 单测另有 **42 个环境性失败**（opencode grep/glob 需执行外部 `rg` 二进制，本环境拒绝运行 `rg.exe`），与本次改动无关（相关模块零修改），复查时经现场验证确认。

## 1. 背景与缺口

| 缺口 | 登记出处 | 本切片状态 |
|---|---|---|
| orz 解码只做 `String::from_utf8_lossy`（无 BOM 剥离、无 GB18030、不记录命中编码） | `CLI_PROJECT_INDEX.md` GAP-ENCODING-GATE（2026-08-13） | **闭合** |
| `run_terminal_cmd`/`read_file`/插件 hooks 未走统一解码链 | 同上 + OPS-PROTOCOL §8 第 2/3 项 | **闭合** |
| run-event 无 `output_encoding` 字段 | OPS-PROTOCOL §8「审计事件记录 `output_encoding`」 | **闭合**（先扩展 Schema 再接线 producer） |

Windows PowerShell 5.1 是主要编码风险源（GB2312 控制台、`Add-Content` BOM）；调用侧 UTF-8 强制（`shell.rs` 的 `PYTHONUTF8=1`/`PYTHONIOENCODING` 注入）已存在，本切片补齐**捕获侧兜底**与**文件侧读取**。

## 2. 设计决策

| 决策 | 内容 | 依据 |
|---|---|---|
| D-1 解码链与标签 | 固定链：剥离 BOM → UTF-8 严格 → GB18030 → lossy；标签 `utf-8-sig` / `utf-8` / `gb18030` / `utf-8-lossy` | OPS-PROTOCOL §8 第 2 项；与 `ops_executor.py::decode_text` 逐字对齐 |
| D-2 GB18030 严格 | `encoding_rs::GB18030.decode_without_bom_handling`，`had_errors=true` 时**落入 lossy**（不视为 gb18030） | Python `decode("gb18030")` 对非法/截断四字节序列抛异常；encoding_rs 解码器是全量替换，`had_errors` 是忠实映射 |
| D-3 多块标签合并 | 同一流多个块各记标签，按首次出现顺序去重后逗号连接（`utf-8,gb18030`）；应用于截断输出的 front/back 与 run_tests 的 stdout/stderr | `ops_executor.py` 的 `",".join(dict.fromkeys(encs))` 同规则 |
| D-4 写入侧 | 新增 `encode_text_no_bom`/`write_text_utf8_no_bom` 固定契约（Rust 字符串本就是 UTF-8，函数把「无 BOM、不依赖系统代码页」钉为可测试入口） | OPS-PROTOCOL §8 第 3 项 |
| D-5 元数据流 | 解码点 → 工具输出结构（`TerminalRunResult`/`BashOutput`/`FileContent`）→ `ToolOutput::output_encoding()` → `ToolRunResult.output_encoding` → `orz-host::call_tool` → `orz_loop::host::ToolResult.output_encoding` → controller `tool_completed.output_encoding` | 单一数据流，无旁路；字段全部可选、缺省 `None` |
| D-6 Schema 先行 | `tool-completed-event-payload-v0.1.schema.json` 先加可选 `output_encoding`（string/null），再接线 producer | 项目纪律「先 Schema/fixture 扩展、再改 producer」（ADR-0010 §5.3） |
| D-7 模型零感知 | 模型只看到规范化后的文本；编码只进 journal，不进 conversation 文本 | OPS-PROTOCOL §8「模型对编码零感知、零负担」 |
| D-8 二进制/文档路径 | PDF/图片/PPTX 等无解码链路径 `output_encoding=None`（字段缺省省略） | 解码链只对文本字节有意义 |

## 3. 实现清单

### 3.1 `orz-tools`：新编码模块

- `crates/codegen/orz-tools/src/util/encoding.rs`（新）：`decode_text`（D-1/D-2）、`encode_text_no_bom`、`write_text_utf8_no_bom`、`merge_encoding_labels`（D-3）；8 个单测
- `crates/codegen/orz-tools/Cargo.toml`：`encoding_rs = "0.8"`（Cargo.lock 已有传递依赖，零新增下载）

### 3.2 `orz-tools`：终端输出

- `computer/types.rs`：`TerminalRunResult` 增 `output_encoding: Option<String>`
- `computer/local/terminal.rs` `ProcessState::to_result`：截断路径 front/back 各自走解码链并合并标签；非截断路径整缓冲解码；`combined_output` 语义不变

### 3.3 `orz-tools`：bash 输出

- `types/output.rs`：`BashOutput.output_encoding`（serde 可选）、`FileContent.output_encoding`（serde 可选）、`ToolOutput::output_encoding()`（Bash / ReadFile 两变体）、`ToolRunResult.output_encoding`
- `registry/types.rs`：`ToolRunResult` 构造处从 `output.output_encoding()` 填充（构造前取值避免 move）
- `grok_build/bash/mod.rs`、`opencode/bash/mod.rs`：生产构造从 `TerminalRunResult.output_encoding` 透传

### 3.4 `orz-tools`：read_file

- `grok_build/read_file/mod.rs`：文本路径 `from_utf8_lossy` → 解码链，命中编码写入 `FileContent.output_encoding`（含空文件分支）
- `codex/read_file/tool.rs`、`opencode/read/mod.rs`：同链同字段
- `grok_build_hashline/read_file.rs`：重读路径走同链（原始读取已记编码，不重复携带）

### 3.5 `orz-hooks`：插件命令输出

- `runner/command.rs::truncate_output`：lossy → 解码链；标签经 `tracing::debug` 记录（hooks 无 tool_completed 事件，不进 run-event）

### 3.6 `orz-host`：run_tests + 工具结果映射

- `lib.rs` run_tests：stdout/stderr 各自解码链 + `merge_encoding_labels`，超时路径同样记录；`TestRunResult.output_encoding` 透传
- `lib.rs` `call_tool`：`ToolResult.output_encoding = result.output_encoding`（run_terminal_cmd/read_file 生效；host-owned 工具为 None）

### 3.7 `orz-loop`：journal payload

- `host.rs`：`ToolResult.output_encoding`、`TestRunResult.output_encoding`（均 `Option<String>`，`Default` 兼容）
- `controller.rs`：通用工具路径与 run_tests 路径在 `tool_completed` payload 中条件写入 `output_encoding`（缺省省略，保持旧 payload 字节不变）

### 3.8 Schema

- `runtime/tool-completed-event-payload-v0.1.schema.json`：新增可选 `output_encoding`（`["string", "null"]`，含说明）；run-event v0.2 信封不变

## 4. 测试（新增 19 个断言点）

| 测试 | 覆盖 |
|---|---|
| `util::encoding` 9 个单测 | utf-8 / utf-8-sig（BOM 剥离）/ gb18030 / **非法 GB18030 落 lossy（复查补，锁定 D-2 与 Python 参考一致）** / lossy / 空输入 / 无 BOM 写入 / 标签合并 |
| `read_file_gb18030_records_output_encoding` | GB18030 文件内容正确 + `FileContent.output_encoding == gb18030` |
| `read_file_utf8_bom_stripped_and_labeled` | BOM 剥离不泄漏 + `utf-8-sig` 标签 |
| `foreground_command_carries_output_encoding` | mock 终端编码透传 `BashOutput` + `ToolOutput::output_encoding()` |
| `read_file_gb18030_forwards_output_encoding`（orz-host e2e） | 真实 host 调用 read_file：内容正确 + `ToolResult.output_encoding` |
| `tool_completed_carries_output_encoding_when_host_observed_one`（orz-loop） | journal `tool_completed.output_encoding` 落地 |
| `run_tests_tool_completed_carries_output_encoding`（orz-loop） | run_tests payload 携带 `utf-8` |
| `slice_reads_gb18030_content`（复查 P2-1） | codex slice 模式内容路径 GB18030 正确 + 编码标注 |
| `update_file_gb18030_content_preserved`（复查 P2-2） | apply_patch 读目标经解码链，GB18030 文件正确打补丁 |
| `grep_search_prompt_decodes_gb18030`（复查 P3-5） | rg stdout 渲染经解码链 |
| `gb18030_replacement`（复查补充） | **生产路径 search_replace** 读目标经解码链，GB18030 文件正确替换 |

## 5. 验证（全绿）

- orz-tools：terminal **37/0/5 ignored**（既有 ignore）、read_file **91/0/0**、grok_build bash **149/0/0**、encoding 单测 **8/0/0**
- orz-hooks：runner::command **20/0/0**
- orz-host：新增 e2e **1/0/0**（全 crate 编译通过）
- orz-loop：新增 2 个 journal 测试全绿（全 crate 编译通过）
- pytest `runtime/tests/test_run_event_conformance.py`：**14 passed / 0 failed**（schema 扩展兼容）
- `scripts/check_repository.py`：**valid / error_count 0**
- `git diff --check`：干净（见登记命令）

## 6. 登记边界

1. **二进制/文档路径不记编码**：PDF、图片、PPTX 等走 `output_encoding=None`（字段省略）；`pdf_read`/`browser_read`/`web_fetch`/`project_doc_index` 等无机械解码链工具同为 None。
2. **截断输出为近似值**：截断时仅 front/back 两段可解码，中间丢失段可能影响编码判定——标签按已见块合并，读取输出文件可获完整解码（`output_file` 原样字节）。
3. **hooks 不进 run-event**：插件命令输出规范化后仅 tracing 记录标签；hooks 无 `tool_completed` 事件，不扩展事件面。
4. **web_search 不标注**：DeepSeek 原生 `output_text` 是服务端综合文本，无 runtime 机械解码环节，不属于本门控（来源加权另登记 GAP-SOURCE-WEIGHTING-IMPL）。
5. **兼容性**：所有新增字段可选、缺省 None、旧 journal/payload 字节不变；v0.2 信封与事件枚举零变更。
6. **调用侧 UTF-8 强制为既有**：`orz-config/src/shell.rs` 的 Python 子进程 UTF-8 环境注入此前已存在，本切片只补齐捕获侧/文件侧；PowerShell 5.1 回退仍需显式输出编码设置（协议 §8 第 4 项，运行侧配置事项）。
7. **文件写入侧零新增接线**：`search_replace` 等写路径本就以 Rust `String` → UTF-8 字节写盘（无 BOM）；`encode_text_no_bom`/`write_text_utf8_no_bom` 以单测锁定该契约，不重复接线。

## 7. 复查问题处理（2026-08-13 第二轮）

对首轮实施做全面复查后，下列问题全部处理（对应复查报告 P2-1/P2-2/P2-3/P3-4/P3-5）：

| 问题 | 处理 |
|---|---|
| codex read_file 内容路径（slice/indentation）仍 lossy | `text_utils::format_display` 与 `indentation.rs` 两处 `raw` 解码改走固定链；新增 `slice_reads_gb18030_content` |
| codex apply_patch 读目标仍 lossy | `read_file_as_string` 改走固定链；新增 `update_file_gb18030_content_preserved` |
| 后台任务快照输出仍 lossy / 严格 UTF-8 | `to_task_snapshot` 三分支改走固定链（完成态改为读字节再解码，不再静默丢 GB18030）；`maybe_truncate` 保留 lossy（内部截断记账，非模型文本） |
| `encoding_rs` 浮动版本未走 workspace | 上移 `orz/Cargo.toml` workspace.dependencies，`=0.8.35` 精确锁定；orz-tools 改 `workspace = true`（Cargo.lock 已含 0.8.35，零新增） |
| 其余进程文本面 lossy（协议级） | 模型可见面全部改走固定链：`ToolOutput::GrepSearch` 渲染、codex grep_files stderr、`static_shell` 快照捕获、终端 login env 捕获；`shell_state` dump 改为**整缓冲解码**（避免逐块拆分多字节）；新增 `grep_search_prompt_decodes_gb18030` |

复查扫描又发现同一类的残余 lossy 点并全部处理：**search_replace 读目标（生产路径）**、`exit_plan_mode` 计划文件（含 strict-UTF-8 回退）、`web_fetch` 文本体（GB18030 网页）、`grok_build_hashline` edit/grep、`opencode` write/glob/grep、`grok_build` grep 解析路径、`monitor` 事件文本。

第二轮验证：codex read_file **36/0/0**、apply_patch **41/0/0**、types::output **81/0/0**、terminal **37/0/5**、search_replace **94/0/0**、web_fetch **132/0/0**、hashline **210/0/0**、opencode write **11/0/0**、exit_plan_mode **17/0/0**（ignored 为既有）；orz-tools 全量 lib **2702 passed / 42 failed / 6 ignored**——42 个失败与首轮完全同集合（opencode grep/glob 依赖外部 `rg`，本环境拒绝执行），新增测试全部通过，无回归。

复查后剩余 lossy 点（均为内部记账、流式显示或测试断言，不产生权威模型文本）：`terminal.rs maybe_truncate` 字符计数（内部截断记账）、`notification/types.rs` 流式块显示（逐块解码会拆分多字节；最终输出已整缓冲解码）、`bash`/`grok_build_concise`/`chat_completion_output` 对已规范化文本的二次 lossy（无损）、`embedded_search_tools.rs`/`static_shell.rs`/`shell_state.rs` 测试辅助断言。
