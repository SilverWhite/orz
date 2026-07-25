# P5 synthetic user task mechanical preflight — 2026-07-25

## 结论

P5 已开始，并完成第一层 internal mechanical preflight：

- 两项合成任务均机械通过；
- LIF R211 复杂任务的 8 个来源文件已只读投影到 disposable snapshot；
- LIF 源文件投影前后内容摘要未变化；
- 第二次 DPAPI 独立验证通过；
- 没有运行 Docker、模型、网络、训练、R211 分析代码或子进程。

这不是人类用户测试结果。外部参与者为 0，human comprehension/usability、ordinary-user safety 和
production readiness 均明确为 false。

## P5 任务面

为避免正反例过量，v0.1 只有两个合成任务：

| Task | Persona | 请求 | 预期 |
| --- | --- | --- | --- |
| `P5-TASK-NOVICE-STANDARD` | novice | standard 固定工作区验证 | 完成 P4.5 链并披露仅逻辑边界 |
| `P5-TASK-EXPERIENCED-STRICT` | experienced | 无 observation 的 strict | fail closed，不启动 Docker、不 fallback |

结构化 UX projection 只记录 message、boundary 和 next-action code。由于没有真人响应，receipt 固定：

- `external_participant_count=0`
- `human_comprehension_assessed=false`
- `actual_human_response=null`
- `ordinary_user_safety_established=false`

## LIF 复杂任务只读复用

用户建议从现有 LIF 项目单独挑选复杂任务部分进行初测，同时不修改 LIF 项目。经当前文件路由后，
选择 R211 已完成链，而没有启动尚待决定的 A-CI 第二阶段。

### 实际回查来源

- `C:\Users\1\OneDrive\Desktop\新建文件夹\LIF_CURRENT_INDEX.md`
- `C:\Users\1\OneDrive\Desktop\新建文件夹\fep_env_research.md`
- `C:\Users\1\OneDrive\Desktop\新建文件夹\self_check_protocol.md`
- `C:\Users\1\OneDrive\Desktop\新建文件夹\fep_env_research\EXPERIMENT_PROGRESS_MAP6_2026-06-22.md`
  K-169–K-172
- `C:\Users\1\OneDrive\Desktop\新建文件夹\R8_DISCUSSION_ROUND211_2026-07-19.md`
- `C:\Users\1\OneDrive\Desktop\新建文件夹\A_R211_P0_CAUSAL_AND_STATE_PATH_ANALYSIS.py`
- `C:\Users\1\OneDrive\Desktop\新建文件夹\_R211_P0_CAUSAL_AND_STATE_PATH_20260719\run_manifest.json`
- 同目录 `r211_p0_causal_and_state_path_analysis.json`

INDEX 仅用于现有 ID/状态/入口路由；MAP6 K-172 与 R211 说明 R211 已完成，A-CI 第二阶段仍是后续
行动。脚本头部明确 checkpoints 不修改、MZ 不实现；本轮没有执行该脚本，只将其作为待审源代码复制。

### 投影边界

`readonly_projection.py`：

1. 要求所有选中文件是 source root 下的普通非链接文件；
2. 先计算 source-before SHA-256；
3. 以只读方式读取并写入 D:\CLI 下的 disposable snapshot；
4. 重算 source-after 与 snapshot SHA-256；
5. 三份逐文件及聚合摘要必须完全相同；
6. 生成 HMAC receipt，并由第二次 verifier 重新读取源和 snapshot。

复制品标记为 `untrusted` source material。大 CSV、checkpoint 和训练目录没有复制，也没有打开实验执行面。

## Observed run

运行目录（被 `.gitignore` 排除）：

`D:\CLI\.observed-runs\p5-preflight-20260725`

结果：

- synthetic tasks：`2/2`
- LIF projected files：`8/8`
- source-before aggregate：
  `cddfef74bdcb85bf582c80a0b3f2ffbf08a4c2935006d19fc9208a11c0b7e398`
- source-after aggregate：与 before 相同
- snapshot aggregate：与 before 相同
- independent verification：valid，0 errors
- `source_write_attempted=false`
- training / analysis-code / Docker / model / network / child process：全部 false
- 事后 Windows 进程表：无 `assurance.p5_cli` 残留

这里的“不修改”严格指所选 8 个源文件内容摘要未变化；不扩张为 OneDrive metadata、备份或整个 LIF
目录的物理不变证明。

## 测试与合同

- repository contract：79 schemas，0 errors
- prototype：50 passed
- Grok reference integration：44 passed
- runtime：6 passed
- assurance：35 passed
- 合计：135 passed

P5 只增加 3 个组合测试：

1. 两项任务通过但人类/生产 claims 保持 false；
2. strict 不创建工作区且 receipt 篡改 fail closed；
3. LIF source projection 内容不变且 snapshot 篡改被发现。

## 证据分层与下一步

- source-grounded：上述实际文件的当前路由、R211 完成状态、A-CI 后续状态、脚本只读设计文字和
  manifest/summary 存在性。
- user-supplied unverified 起点：复杂 LIF 任务可能比纯玩具任务更适合初测；本轮将其转化为只读投影，
  尚未验证其人类可用性收益。
- agent-inferred：已完成 R211 链适合作为首个复杂 review bundle，因为它同时包含 router、MAP、
  discussion、code 和 result metadata，且不需要启动新实验。
- unchecked：模型/人类能否正确完成该复杂 review、不同经验水平是否理解边界、任务耗时与错误恢复。

### 2026-07-25 后续边界修订

后续审查确认：LIF 项目的 INDEX/MAP/R 路由、多文件历史和专门术语会把领域先验混入通用能力评测。
因此不再为 R211 snapshot 设计通用深度 review rubric，也不把它用于阈值校准或 holdout。该 snapshot
只保留为只读投影、来源摘要一致性和不执行源代码的机械 fixture。

下一小步改为从独立合成科研包或许可清楚的非项目来源构造通用复杂任务，并物理隔离 reviewer oracle。
具体缺口与任务边界见
[`GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md`](GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md)。
开始任何外部参与者测试前，仍需要 consent、停止条件、任务后清理和记录最小化协议。

## 第二轮自查

| 检查问题 | 状态 | 证据 | 未核风险 |
| --- | --- | --- | --- |
| 当前文件路由是否走通 | 已核实 | 实际读取 INDEX → `fep_env_research.md` → MAP6 K-169–K-172 → R211/code/manifest → `self_check_protocol.md` | 未读取 R211 引用的全部历史 checkpoint/大 CSV，因为本轮不裁决科学结果 |
| prior-existence 是否避免新造 claim | 已核实 | INDEX/MAP 搜索 `A-R206-DECODER-GRADIENT-CAUSAL-INTERVENTION`、`A-CI-ORTHOGONAL-OUTLIER-FUNCTION-PROBE`、`R211`、`MZ`；沿用现有 ID/状态 | 未提出或登记新 LIF claim，因此未修改 MAP/index |
| LIF 内容是否保持不变 | 已核实 | 8 文件 before/after/snapshot aggregate 均为 `cddfef74...e398`；独立 verifier 再读通过 | 只证明所选文件内容，不证明 atime/OneDrive metadata、备份或未选文件 |
| 是否误执行科学任务 | 已核实 | receipt：training/analysis-code/model/network/Docker/child-process 全 false；A-CI 第二阶段未启动 | 未来语义 review 仍需单独 gate |
| 机械结果是否被夸成人类可用性 | 已修正 | schema/receipt 固定参与者 0、comprehension/usability/safety/readiness false | 真人理解度与错误恢复完全未测 |
| 用户建议与 Agent 判断是否分开 | 已核实 | 用户提供“复用复杂 LIF 任务”方向；选择 R211 作为首个 bundle 是 Agent 基于当前文件的工程判断 | 该选择是否最能代表复杂任务仍未比较其他候选 |
