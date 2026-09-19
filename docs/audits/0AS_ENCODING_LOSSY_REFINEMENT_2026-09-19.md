# 0as 编码 lossy 兜底细化——实施回执（2026-09-19）

> 立项：BACKLOG 0as（P1，2026-09-19 用户令「值得做，让模型自己看着舒服些」；由 0ar S1 狗粮轮
> F4 回查引出）。本批为 0as 实施批（用户同日放行「请开始处理0as项」）；**工作树未提交、未推送、
> 载体重建随用户放行**；未闭合计数 37 不变（0as 闭合留判据入账批）。

## §1｜口径定稿

四级梯**前三级（BOM 剥离 → UTF-8 严格 → GB18030 清洁解码）整段判定顺序与命中语义不变**；
写入侧（统一 UTF-8 无 BOM）不动；`sniff_text_bytes` 解码优先门与无解码链路径（`None`）不动。
仅末级 lossy 细化：

1. **分段粒度＝行**（`\n` split_inclusive）。行粒度安全性：UTF-8 与 GB18030 均不以 `0x0A`
   作为多字节序列的组成字节，跨行截断不可能。
2. **段级走梯**：段为合法 UTF-8 → 原样保留（可读文本零损伤）；段为 GB18030 清洁解码 → 采用；
   否则进入最小替换择优。
3. **最小替换择优**：同段并行计算 UTF-8 侧与 GB18030 侧的降级单元数，**取少者；平手按梯序取
   UTF-8**（沿 BACKLOG 0as 注册候选②原文）。
4. **显式占位**：UTF-8 侧以 `Utf8Error` 步进（合法前缀逐字保留），每个极大非法子序列渲染为
   一个显式占位——单字节 `⟨0x8F⟩`、多字节 `⟨0xF0 0x9E 0x81⟩`（大写十六进制、空格分隔）。
   与现状 `from_utf8_lossy` 同为 maximal-subpart 粒度，**单元数一一对应、不增噪声**，但把
   字节级事实还给模型。GB18030 侧保持 U+FFFD（encoding_rs 不暴露错误字节位置）。
5. **降级读数**：标签变为 `utf-8-lossy:<p>%`——`<p>` ＝ 降级单元数（占位数＋U+FFFD 数）÷
   lossy 级输入字节数（BOM 剥离后）× 100，两位小数（`{:.2}`）。lossy 级保证单元数 ≥ 1
   （若逐段全部命中前两级则整段不会进入 lossy）。

## §2｜契约面（沿 0ar S1 两步走：契约面 → 实现面）

- `tool_completed.output_encoding` 在两份 schema（v0.1/v0.2）中为**自由字符串**（非闭枚举），
  标签带后缀不破 schema 结构；两份 description 同步补注 0as 形态与比例定义（本批契约面编辑）。
- **Python 冻结参照同步**：`assurance/ops_executor.py::decode_text` 移植同算法（行粒度、
  段级梯、最小替换择优、平手梯序、占位形态、比例公式）；实测 CPython `UnicodeDecodeError`
  的 start/end 与 Rust `Utf8Error` 同为 maximal-subpart 粒度、`gb18030 errors="replace"`
  与 encoding_rs 同粒度——六案（mixed／gb+bad／tie／multibyte／bom／truncated-4-byte）Rust
  与 Python 输出**逐字节对齐**，`encoding.rs` 模块头「Rust/Python audits agree」声明保持为真。
- **诊断签名兼容**：`encoding_lossy`（`diagnostics.rs`）按 `contains("lossy")` 匹配——后缀形态
  兼容，零改动；其测试以固定入参自洽，不依赖生产标签形状。
- **fixtures 生成器重跑零差异**（346 件）；`runtime/tests` **366 tests OK**（313.0s）；
  `compileall` exit 0。（两处表述勘误见 §9。）

## §3｜落码面

- 核心：`orz/crates/codegen/orz-tools/src/util/encoding.rs`——`decode_text` 签名
  `(String, &'static str)` → `(String, String)`（仅 lossy 级标签为动态串）；新增
  `decode_lossy_segmented`／`decode_lossy_line`／`utf8_lossy_placeholders` 三个私有函数。
- 调用点适配（label 消费者六处，`.0` 使用者零波及）：
  `computer/local/terminal.rs`（merge 参数 `as_str()` 化）、
  `implementations/codex/read_file/tool.rs`、`implementations/grok_build/read_file/mod.rs`
  （`label.to_string()` → `label` 直移）、`orz-host/src/lib.rs`（run_tests 超时臂直移＋
  `merge_encoding_labels` 迭代改 `map(String::as_str)`）、`orz-hooks/src/runner/command.rs`
  （tracing 字段 `label.as_str()`）。
- 文档注释同步：`computer/types.rs`／`orz-loop/src/host.rs`（×2）／`types/output.rs`（×3）
  标签集提及处补 `<p>%` 形态。
- **Cargo.lock / Cargo.toml 零改动**（零新依赖）。

## §4｜钉子与测试读数

新增钉子六件（判据 ①②③＋形态细节）：

- `lossy_mixed_sample_keeps_readable_parts_intact`——判据①：合法 UTF-8 行＋孤立非法字节行＋
  GB18030 行，可读文本逐字保留、GB 段保持可读（旧行为＝整段成片 `�`）；比例可复算
  （1 单元/34 字节 → `utf-8-lossy:2.94%`）。
- `lossy_prefers_gb18030_when_it_has_fewer_replacements`——判据①头条：GB 文本＋孤立坏字节
  （`[d6 d0 ff]`）→ `中\uFFFD`（GB 侧严格更少，正文不再被打碎）。
- `lossy_tie_keeps_ladder_order_utf8`——平手梯序钉（`[d6 d0 8f]` → `⟨0xD6⟩`＋合法对
  `[d0 8f]`＝U+040F，梯序取 UTF-8）。
- `lossy_multibyte_subsequence_placeholder_form`——多字节非法子序列占位形态
  （`⟨0xF0 0x9E 0x81⟩`）。
- `lossy_label_ratio_excludes_stripped_bom`——判据③：比例分母剔除 BOM（`utf-8-lossy:100.00%`）。
- `invalid_bytes_fall_through_to_lossy` 重写为不变式钉（单元数 ≤ 旧整段基线＋比例可自标签复算）；
  `invalid_gb18030_fall_through_to_lossy` 改钉 GB 胜出形态（`[81 30 81]` 为截断四字节
  GB18030 形式，GB 侧 1 单元胜出）。

既有测试更新两处（旧 lossy 预期）：`codex/read_file/slice.rs::reads_non_utf8_lines`、
`codex/read_file/tool.rs::slice_reads_non_utf8`（U+FFFD 对 → `⟨0xFF⟩⟨0xFE⟩`）。
纯编码回归由既有钉保持（`utf8_without_bom`／`utf8_bom_is_stripped_and_labeled`／
`gb18030_is_decoded`／`empty_input_is_utf8` 逐字节断言全绿——判据②）。

读数（2026-09-19 实测）：

| 面 | 读数（勘误补记见 §9） |
| --- | --- |
| orz-tools lib | 2889 passed＋2 failed＝LSP e2e 两例（负载敏感，串行复跑 18/0 全绿——RS-08 同族既有形态，与本批零接触面）；总数 2897（=2886 基线＋6 新钉＋2 改写＋在场未提交批自带增量） |
| orz-host 串行 | **333 / 0 / 5**（账面基线持平） |
| orz-loop lib | **805 / 0 / 3**（本批 orz-loop 仅文档注释） |
| orz-hooks | **192 / 0** |
| orz-bin | **11 / 0 ＋ 1 / 0** |
| runtime/tests | **366 tests OK**（313.0s） |
| fixtures 生成器重跑 | 346 件零差异 |
| `cargo fmt --all -- --check` | 本批触碰 crate 全净；残余违规全属在场未提交批（`rli_shadow_replay.rs`＝0am 影批 RS-06 在册项、`state_machine.rs`／`agent_loop.rs`／`tool_run.rs`＝0am/0ar S2 未提交内容），按文件级分离不动 |
| clippy（触碰 crate --all-targets） | 本批新码零告警；`encoding.rs:251` collapsible-if 为既有 sniff 代码（非本批行）；余者指向在场批内容 |

环境事件随批登记：**D: 盘 100% 满**（clippy 因 `os error 112` 中断一次）——清理
`orz/target/debug/incremental`（20 GB）后复绿（余 19 GB／94%）。RS-11 的
`D:\tb-eval\orz-cache\cargo-target` 迁移在本 shell 未生效（`CARGO_TARGET_DIR` 未设），
后续批次注意磁盘水位。另：本 shell `protoc` 不在 PATH，工作区级检查需显式
`PROTOC=D:\CLI\orz\bin\protoc.exe`（bin/ 内 dotslash 包装器在 Git Bash 探测链未命中；
开发机既有形态，非本批引入）。

## §5｜F4 并线处置（控制台 GBK 回显观感面）

- 机理：模型面工具结果经编码门**本就零乱码**（0ar S1 实证）；F4 乱码纯在**宿主控制台回显**——
  Windows conhost GBK 代码页把 orz 的 UTF-8 输出按 GBK 解释（子进程随控制台 CP 输出 GBK 亦同）。
- 处置：`orz-bin/src/main.rs` 启动时（Windows）`SetConsoleOutputCP(65001)`——**零依赖 FFI**
  （`unsafe extern "system" #[link(name = "kernel32")]`，沿 `orz-workspace` foreign_sessions
  capability 与 `xai-acp-lib` stdin_reader 库内先例；避免给 orz-bin 增 windows-sys 边而动共享
  Cargo.lock）。效应＝宿主控制台按 UTF-8 解释 orz 输出＋子进程（git 等）随 CP 65001 输出 UTF-8、
  编码门读回干净；TUI/ACP 车道不受扰（crossterm 自管控制台模式）；调用失败静默（无控制台/
  重定向管道态），绝不阻断启动。**载体重建后生效**（随用户放行）。
- 绕行纪律（关键内容一律 `read_file` 核证、勿以控制台回显判正误）保留入档（0ar S1 报告 §7-F4）。

## §6｜边界与残留

- **已知边界（梯序平手例）**：同一行内 GB18030 正文＋坏字节且两侧单元数相等时（如
  `[d6 d0 8f]`），平手按梯序取 UTF-8 侧 → `⟨0xD6⟩`＋一个错配合法对（U+040F）。行分离的
  GB 正文（判据①样本与主流日志形态）不受此影响（段级 GB 清洁解码直接保留）。沿注册候选
  「平手按梯序」执行；若 S3/狗粮读数显示该边界实际可感，可提占优度加权平手裁决，另行立批。
- 不改容器内环境；不动四级梯前三级语义；`output_encoding` 取值形状变更已按「先立 schema
  描述」路径完成（字段保持自由字符串）。
- 本批零语义写点变化：journal 透传链（`tool_run.rs` → `completed_payload["output_encoding"]`）
  未动，比例标签随既有链自然落 journal（判据③）。

## §7｜与在场批的分离声明

orz 工作树在场未提交批：0ar S2（orz-loop 检索三件）＋0am 影子 RLI 批＋0aq RS-14（锁 unwrap
中毒级联，跨 40 文件）。本批触碰文件中仅 `orz-host/src/lib.rs` 与 RS-14 内容**同文件不同
hunk**（本批 2 hunk 在 run_tests 区，RS-14 hunk 在锁治理区，逐一不相交）；提交时该文件需
hunk 级挑选或与 RS-14 同批，其余 0as 文件（encoding.rs／terminal.rs／read_file×2／
grok_build/read_file／orz-hooks command.rs／orz-bin main.rs／types 文档×3／orz-loop host.rs）
均为本批独占。父仓本批触碰：`assurance/ops_executor.py`、两份 `runtime/tool-completed-*.schema.json`、
本档与账本三件——与在场 v3.83 未提交批（裁决档＋案例＋启动器＋账本）文件不相交，唯索引/TODO/
BACKLOG 头部行双方都会改写，提交切分时注意先后。

## §8｜随批改动清单

子仓（未提交）：`crates/codegen/orz-tools/src/util/encoding.rs`（核心＋钉子）、
`crates/codegen/orz-tools/src/computer/local/terminal.rs`、
`crates/codegen/orz-tools/src/implementations/codex/read_file/{tool,slice}.rs`、
`crates/codegen/orz-tools/src/implementations/grok_build/read_file/mod.rs`、
`crates/codegen/orz-tools/src/{computer/types.rs,types/output.rs}`（文档）、
`crates/codegen/orz-hooks/src/runner/command.rs`、`crates/orz-host/src/lib.rs`、
`crates/orz-loop/src/host.rs`（文档）、`crates/orz-bin/src/main.rs`（F4）。
父仓（未提交）：`assurance/ops_executor.py`（Python 参照同步）、
`runtime/tool-completed-event-payload-v0.{1,2}.schema.json`（description）、本档、
`TODO.md`／`docs/BACKLOG_AND_PRIORITIES.md`／`CLI_PROJECT_INDEX.md`（账本随批）。

## §9｜审查处置批勘误（2026-09-19，主会话全面审查后随处置批补记）

1. **§4 读数补全**：上表"orz-tools lib 2889 passed＋2 failed"漏记 **6 ignored**——
   2889+2+6=2897 与总数自洽（审查报告"疑不自洽"系漏数 ignored，一并更正）。主会话全量
   复核实读（含处置批新钉 1 条）：**2891 passed＋1 failed＋6 ignored＝2898**；failed＝
   LSP e2e 一例（负载敏感，串行复跑绿，RS-08 同族既有形态，与本批零接触面）。
2. **§2"gb18030 errors='replace' 与 encoding_rs 同粒度"限定六案范围成立**；一般性被
   GB18030 全空间差分（1,611,796 例）证伪——越界四字节替换粒度 encoding_rs 1 个 U+FFFD
   vs CPython 2 个，可翻转 0as 择优分支；清洁度唯一分叉＝裸 0x80（label 分叉）；二字节
   20 对＋四字节 1 槽位映射分叉（GB18030-2000 PUA vs 2005+/WHATWG）。四类分叉双侧钉死，
   证据与裁决见 [`0AS_REVIEW_HANDLING_2026-09-19`](0AS_REVIEW_HANDLING_2026-09-19.md) §3。
