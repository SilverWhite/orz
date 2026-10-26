# 363 批：常驻策略收敛与 plan_first 改名

> 日期：2026-10-27
> 批号注：本档 2026-10-27 初稿误号「360 批」（与 360 Windows 换装批撞号），364 批收口时改称 **363 批**；正文语义零改。
> 用户令：「我同意你的判定，'resident only what changes the next action；其余全部 pull' 以及 plan 相关内容全改。请先落一份修改审计文档，然后具体怎么做就看你自己安排。」
> 基线：`29a103d6e25f` ＋ 未提交改动（`ORZ_PLAIN_INPUT` 实验开关、本批）。
> 性质：模型面注意力/暗示治理；不改本地面、侧车、压缩与截断的机械记账。

## §0 结论

尾部常驻块不是“催促”，但它们的**每轮显著性**会被模型读成催促。本批按
「resident only what changes the next action；其余全部 pull」收敛：

- 默认不再每轮常驻：D4（run 基线/自编辑清单/编辑指纹）、分块表、任务状态行、
  `SELF_CHECK` 常驻句；
- 保留：压缩/截断 marker、context-scale 硬门、工具结果、一次性初始问询/工作台；
- `plan_first` → `plan`：去掉“first”的时序暗示；旧“首轮必须 plan”的门改为
  opt-in 的 `plan_gate`；
- context-scale 文案去掉“质量已严重衰减/可靠下沿”一类降智暗示，只报事实。

## §1 证据（为什么改）

同一二进制、同一 prompt、同一低滑块环境（`slider=4000`、`block=2000`）的
两次独立 orz 实机运行（`--real`，DeepSeek）：

| 模式 | 模型自报的输入尾部附加块 |
|---|---|
| baseline（`ORZ_PLAIN_INPUT=0`） | `【上下文分块】`尾挂、`[INITIAL_ROUND_INQUIRY v0.1]`、`【工作台】`、`【本 run 基线】HEAD=29a103d6e25f`；追加 `[CONTEXT_SCALE 首个分块]`、`[上下文分块表 v0.1]` |
| plain（`ORZ_PLAIN_INPUT=1`） | 「无。本轮输入末尾除用户消息和工具结果之外，没有任何带标题的附加块。」 |

两次都正确读了文件并给出三条结论；plain 没有因为少看块而答得更差。该实验
只证明“注入在不在”，不证明“好不好”；本批不以质量 A/B 作为验收依据。

## §2 取舍标准

1. **改变下一步动作** ⇒ 可常驻（压缩 marker、硬门、工具结果、上下文读数）。
2. **记忆辅助** ⇒ 默认 pull（D4、分块表、任务状态、编辑指纹）。
3. **一次性引导** ⇒ 只发一次（初始问询、工作台）。
4. **任务结束提醒** ⇒ 按需/结束时（`self_check` 工具与提示，不作为每轮常驻句）。
5. **安全/事实** ⇒ 不删（压缩/截断、硬门、工具结果、终态语法行）。

## §3 落码清单

### 3.1 常驻策略（默认关闭，env 可回退）

| 项 | 默认 | 回退开关 |
|---|---|---|
| D4（`【本 run 基线】`/自编辑/指纹） | 不注入 | `ORZ_MODEL_FACE_D4=1` |
| 分块表 | 不注入 | `ORZ_MODEL_FACE_BLOCK_TABLE=1` |
| 任务状态行 | 不注入 | `ORZ_MODEL_FACE_STATUS_LINE=1` |
| `SELF_CHECK` 常驻句 | 不注入 | `ORZ_SELF_CHECK_RESIDENT=1` |
| 全量纯输入（实验） | 关闭 | `ORZ_PLAIN_INPUT=1` |

实现点：
- `prompt.rs`：`d4_resident_enabled()` / `block_table_resident_enabled()` /
  `status_line_resident_enabled()` / `self_check_resident_enabled()`。
- `model_face.rs`：`build_model_face`/`build_cleared_face` 的 D4 与块表追加按开关；
  `estimate_model_face_tokens` 同步，避免“估算含、实际不含”。
- `agent_loop.rs`：D4 渲染、`self_check` 常驻句按开关；`ORZ_PLAIN_INPUT` 仍为
  更强的总开关（同时跳过 context-scale/orientation）。
- `controller.rs`：`render_status_line()` 在开关关闭时返回空串。

### 3.2 plan 改名

- `plan_first_enabled` → `plan_enabled`；新增 `plan_gate_enabled`（默认 false）。
- `with_plan_first_enabled` → `with_plan_enabled`；旧名保留为 deprecated alias。
- 旧“首轮必须 plan”的门改由 `plan_gate_enabled` 控制；`plan_enabled` 只负责
  plan 工具可用与框架块注入。
- `PLAN_FIRST_FRAMEWORK_BLOCK` → `PLAN_FRAMEWORK_BLOCK`；旧名保留 alias。
- `<plan_first_framework>` → `<plan_framework>`；文案去掉“先计划、首轮提交”
  的硬时序，改为“复杂任务建议写，简单任务可直接行动；不强制首轮提交”。
- 旧测试里依赖 gate 的用例补 `with_plan_gate_enabled(true)`，保持原语义。

### 3.3 context-scale 文案

- `mandatory_compression_window_block` 第二次询问去掉
  「**上下文质量已严重衰减**——≈385K 真实 token 已过模型普遍可靠下沿」；
  改为「当前读数仍在必定压缩线之上；压缩是恢复工作现场的方式」。
- 320K/H1 块只报读数与动作，不评价模型质量。

## §4 兼容与回退

- 所有常驻项默认关闭，但均有 env 单键回退；无需改配置格式。
- `plan_first` 旧 builder/常量保留 alias；旧调用点编译不破。
- 未改：本地面全量、侧车、压缩/截断、`ORZ_PLAIN_INPUT` 语义。

## §5 验证（不做质量 A/B）

采用**故障消融**：默认关闭后跑真实任务，观察是否出现具体故障：

1. 编辑到陈旧文件（D4 编辑指纹缺失的代价）；
2. 压缩后找不到块（分块表缺失的代价）；
3. 计划丢失/重复（任务状态行缺失的代价）；
4. 上下文越线未压缩（硬门仍在，预期不出现）；
5. 收尾无证据（`self_check` 工具仍在，按需可调）。

无故障 ⇒ 保持关闭；有故障 ⇒ 只把对应项做成 pull/一次性，不整体回滚。
构建验证：`cargo check -p orz-loop -p orz-bin`；必要时 `cargo test -p orz-loop --lib`。

## §6 风险

- 分块表 pull 成本：压缩后模型需读 `.gsa/ledger/current.md` 或块档案；压缩
  marker 已带路径，成本可控。
- D4 编辑指纹缺失：模型可能重复编辑同一文件；`git status`/`read_file` 可补。
- plan gate 测试面较大：旧用例必须显式开 `plan_gate`，否则语义变化。
- 观察偏差：模型自报只能证明“看到/没看到”，不能证明“更好/更坏”。

## §7 文件

- `orz-loop/src/prompt.rs`、`model_face.rs`、`agent_loop.rs`、`controller.rs`、
  `context_scale.rs`、`planning.rs`
- `orz-assurance/src/plan/framework.rs`、`mod.rs`
- `orz-agent/src/prompt/agents_md.rs`
- 本文件

## §8 落码与验证（2026-10-27）

已落码（未提交）：

- `prompt.rs`：`d4_resident_enabled()` / `block_table_resident_enabled()` /
  `status_line_resident_enabled()` / `self_check_resident_enabled()`；
  `resident_flag()` 生产默认 off，单元测试默认 on（`cfg!(test)`，保留旧面回归）。
- `model_face.rs`：D4、分块表的装配与估算按开关；`ORZ_PLAIN_INPUT` 早退不变。
- `agent_loop.rs`：D4 渲染、`self_check` 常驻句、plan gate、框架块按开关。
- `controller.rs`：`plan_enabled` / `plan_gate_enabled`；旧
  `with_plan_first_enabled` 同时开 plan+gate（兼容）；`render_status_line()`
  默认 pull。
- `framework.rs`：`PLAN_FRAMEWORK_BLOCK` + 新文案；旧常量 alias。
- `planning.rs` / `agents_md.rs` / `plan/mod.rs`：新常量 re-export。
- `context_scale.rs`：第二次询问去掉“质量已严重衰减/可靠下沿”，改为
  “压缩是恢复工作现场的方式”。

验证：

- `cargo check -p orz-loop -p orz-bin`：通过（仅旧名 deprecated 警告）。
- `cargo test -p orz-loop --lib`：**922 passed / 0 failed / 3 ignored**。
- `cargo build -p orz-bin --release`：`Finished release profile in 3m 06s`；
  产物 `D:\CLI\orz\target\release\orz.exe`。

测试口径：单元测试默认保留旧常驻面（回归覆盖）；生产默认 pull。要验证新默认，
显式 `ORZ_MODEL_FACE_D4=0 ORZ_MODEL_FACE_BLOCK_TABLE=0
ORZ_MODEL_FACE_STATUS_LINE=0 ORZ_SELF_CHECK_RESIDENT=0` 再跑。

未落码（本批不做）：D4 的“按需注入”工具化（当前仅默认关，pull 用
`git status`/`read_file`）；分块表的 pull 工具化（当前读
`.gsa/ledger/current.md`）。
