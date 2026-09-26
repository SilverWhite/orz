# 0BS 后半部分处理报告（0bt①②③＋⑪ 落码完成；⑧⑨⑩⑫⑬ 未竟留档）

- 日期：2026-09-26（b 轮）
- 基线：orz HEAD `a8430054`（0.6.16 bump 树面未提交）＋工作树改动；父仓 HEAD `a441ff1c`；Windows 载体 0.6.16 已重建（078 批）
- 纪律（用户令）：**不提交 / 不推送 / 不重建**；本报告为该轮产出
- 范围：0bs 未完成项（0bt①②③、⑧-⑬）＋上一轮摩擦项（f3/f4 等）

## 一、本轮完成项

### 1. 0bt① read_file 行窗 / 切片面（承接 0bb GAP-TOOL-LONGLINE-QUOTING；不新增工具）
- 新增三参数：`line` / `char_offset` / `char_limit`（`ReadFileInput` 加 `Default` derive；serde lenient i64；schemars GrokIntegerSchema）。
- 语义：行号 1-based、负值从尾、0→1；字符单位；`LINE_WINDOW_DEFAULT_CHARS=4000`、`LINE_WINDOW_MAX_CHARS=16000`；标记行 `[line N: ...]` 四态（空行 / past the line's end / complete / 续读指针 `showing a-b; continue with line=L, char_offset=C`）。
- 行窗模式先于 `outline` 与 coarse-gate 判定（两参数在该模式被忽略；超长单行不再触发 read-handle 信封）。
- 落码：`crates/codegen/orz-tools/src/implementations/grok_build/read_file/mod.rs`（`render_line_window` / `resolve_line_window_line` / `line_window_text_at`）；跨 5 文件 73 处 `ReadFileInput` 字面量补 `..Default::default()`。
- 验证：read_file 126/0、hashline 220/0、cursor_rules 11/0；新增 2 测试（30k 字符单行 64 字符窗分页+续读+尾窗 complete；越界行号报错含总行数、负行号、char_offset past-end 标记）。

### 2. 0bt② dogfood_launch 版本旁路（承接 0bj①；carrier=unknown→打包面）
- `orz-bin/src/main.rs`：新增 `build_info_line()`（`orz-build-info: version={CARGO_PKG_VERSION} os= arch= profile=`），`main_inner` 首句 `--build-info` 早退（println+return）——载体自带旁路，单一源、不依赖外部版本文件。
- `scripts/dogfood_launch.ps1`：版本段改读 `--build-info`（`$LASTEXITCODE=0` 且 `version=([^\s]+)` 匹配才采用；`try/catch` 回落 `unknown`）；文件头加 ⑯ 注记。
- 验证：`cargo check -p orz-bin` 过；launcher PSParser 语法过。

### 3. 0bt③ 宿主 shell 通道纪律单点（承接 0bj③+m9）
- `grok_build/bash/mod.rs` 三处描述模板的 `not has_unix_utilities` 条款改为「**rewrite the command instead of retrying it (0bt③, 2026-09-26)**」＋PowerShell 等价物表：`Select-String`（grep）/ `Get-Content -TotalCount N`・`Select-Object -First N`（head）/ `Get-Content -Tail N`・`Select-Object -Last N`（tail）/ `Get-ChildItem -Recurse`（find）/ `-replace` 或 Python（sed/awk）；工具层首选 `grep` 工具与 `read_file`。
- Windows notes 增 `cd /d` 是 CMD-only、PS 用 `cd <path>`／`Set-Location -LiteralPath`（m3 原文落点）。
- 验证：bash 161/0（`powershell_emits_unavailable_and_chaining_notes` 增改写表断言）。

### 4. f4 探针处置
- 删除 `crates/codegen/orz-workspace/tests/real_store_probe.rs`（707B 临诊探针，头注「跑完即删」；读真实 TrustStore+workspace_key(D:\CLI)）＋空 `tests/` 目录；orz 子仓 untracked 面清零。

### 5. ⑪ 本地浏览器车道：输入拟真 v1（落码完成）
- 新模块 `orz-host/src/local_browser/input_sim.rs`（单一源常数表，模型不可调参、机械层常驻施加）：
  逐字符 200ms±40%；句读符后 1.5s 句间；提交前 0.6s；**错字率 2%**（邻键错字＋120ms 后 Backspace 修正回路）；**WindMouse**（gravity 9 / wind 3 / maxStep 10 / 阈值 1.5）目标 **800px/s**；按下-抬起驻留 60–140ms；滚动 **110px/20ms**；动作后停留 0.8s；会话 **pacing ≥5s**（上限侧 0–50% jitter）；移动前反应窗 120ms。SimRng（SplitMix64）确定性种子；9 项单元测试。
- 动作面（`browser_control` 新增四动作；`BrowserControlAction::{Type,Key,Click,Scroll}`）：
  `type{text,submit}` / `key{key}` / `click{selector|(x,y)}` / `scroll{dx,dy}`；`click.selector` 走主机自持 JS 模板（querySelector+getBoundingClientRect 中心，JSON 字面量注入防逃逸）。
- 参数纪律（严格解析）：text≤800、key≤32、selector≤500、坐标 0..=20000、滚动 ±20000、click 二选一「selector 独占或 x+y 成对」；非输入动作携带输入参数一律拒绝。
- CDP 白名单放开：`Input.dispatchKeyEvent` / `Input.insertText` / `Input.dispatchMouseEvent` / `Input.synthesizeScrollGesture`（执行全走 input_sim 计划执行器；`Storage.*` 仍禁）。
- **类型留档**：信封新增 `browser_type`（headless/headed；来自会话 headless 标志，随每次使用回传，journal 落档）＋`input_events`（输入动作事件数）。单会话单实例 ⇒ **无头/真机天然不混用**（用户令 §4.6②）。
- 验证：`cargo test -p orz-host --lib -- local_browser` → **93 passed / 0 failed / 4 ignored**；白名单集测试更新（Input.* 四法放行、Storage.* 仍禁）；参数校验测试含 7 个新用例。

## 二、未竟（原因与下一步）

1. **⑪ 唯二门禁（下载/脚本）**：现行 permission 层无「强制询问」臂（`browser_control` 历史归 ReadOnly 类），「弹验证＋用户明确批准」需新机制（建议加 browser 动作族 ApprovalAlways 类或 host 侧 approval token）；`Runtime.evaluate` 任意脚本面保持关闭（仍仅主机固定表达式），待批准通道接线。
2. **⑧ 降级序与共享预算**：browser→本地 HTTP→provider 需跨层接缝（web_search 在 orz-tools、浏览器在 orz-host；registry 有 session-scoped backends 口子 `types.rs:1405-1415`）；共享 per-activation SERP 单账本需把 web_search 车道并入 loop 侧 `SerpSearchBudget`（现仅 `browser_control search` 计费）；首果≤10s 已具备（G1 `T_first=10s`）；ADR §14.65 脚注未落。
3. **⑨⑩ 三引擎与指纹**：本地 HTTP 车道默认集仍 `[bing_cn]`（代理链 `bing_cn,bing_global,duckduckgo,google`），解析器仅 `parse_bing_serp` 统一实现——duckduckgo/baidu/360search 需各自 SERP 解析器；Bing 出集与 TLS/HTTP2 指纹（Rust `wreq` 新依赖 vs sidecar `curl_cffi`（本机已装 0.16.3））未定论，未动码。
4. **⑫ 垂直源**（GitHub/StackExchange/arXiv/OpenAlex/Crossref/PyPI/npm）与 **⑬ 信任×ACAF S2**：未动（S1 口径见 0BS 检索线调研档）。
5. **f3 auto-mode 记账行**：未落。

## 三、摩擦项（本轮记录）

| # | 现象 | 处置/教训 |
|---|------|-----------|
| f9 | 批量补丁脚本把 `ReadFileInput` 切片首字符吃掉（`{eadFileInput`，机械脚本 `inner[1:-1]` 类越界） | 修复脚本二连（fix-literal / fix-comma）；教训：结构化批量改动必须走「编译验证闭环」，勿信单次正则 |
| f10 | PowerShell 将 cargo stderr 包装为 `NativeCommandError`（exit=1 假阳性） | 判据改用日志尾 `Finished` 行（与既有 f6 同族复证） |
| f11 | 大文件 `read_file` 一次读全文触发 read-handle 信封（180KB/4276 行） | 分页读必要；本轮标的（read_file 自身）即摩擦源，0bt① 行窗自证闭环 |
| f12 | 枚举/结构体增改引发全仓适配错误三连（E0004 非穷尽 / E0063 缺字段 / E0277 `f64:Eq`；另一次补丁把 `..Default::default()` 的尾逗号写进测试字面量） | 机械面必然成本；坐标改 i64 解决 Eq；尾逗号规则：`..base` 后不得有逗号 |
| 复证 | m3（`cd /d` 在 PS 5.1 不适用）、m9（宿主 shell 无 grep/head/sed） | 原文已并入 0bt③ 单点（bash 描述模板，:148/:154 原档） |

## 四、验证汇总

- `cargo test -p orz-host --lib -- local_browser`：**93 passed / 0 failed / 4 ignored**（97 项）
- `cargo test -p orz-tools --lib` 定向：read_file 126/0、bash 161/0、hashline 220/0、cursor_rules 11/0
- `cargo check -p orz-bin`：过；`dogfood_launch.ps1` PSParser：过

## 五、仓库状态（未提交，遵嘱）

- orz 工作树（未提交）：`grok_build/read_file/mod.rs`、`grok_build_hashline/read_file.rs`、`cursor_rules_on_read.rs`、`grok_build_concise/read_file.rs`、`types/tool_io.rs`、`orz-bin/src/main.rs`、`local_browser/{mod.rs,cdp.rs,input_sim.rs(新)}`、`grok_build/bash/mod.rs` 等。
- 父仓（未提交）：`scripts/dogfood_launch.ps1`、本报告（`docs/audits/0BS_PROGRESS_2026-09-26b.md`）。
- 无 untracked 探针残留（f4 已清）。
