# Global Progress Sentinel no-model spike（2026-07-21）

状态：fixture-only、no-model、protocol-compatible。未启动 Grok，未调用 DeepSeek，未读取真实用户 workspace，未
新增 protocol task/action/evidence 状态或 reason code。

## 1. 实现范围

本轮只实现 [`GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`](../architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md)
首个 spike 的三类 warning：

- `direction_concentration`：最近三个成功 action 均属于 `runtime_acp`，同时还有其他 active direction；
- `acceptance_uncovered`：`ACC-HANDOFF` 没有映射 plan step；
- `verification_debt`：已完成写入步骤 `STEP-ACP-01` 的最新 verification 为 `stale`。

设计中的 starvation、scope drift、repeated failure 等规则仍未实现。三类 warning 是 heuristic control signal，
不会改变 evidence/claim 状态，也不会自动 block。

## 2. Fixture 与实现

固定输入包含六个 direction、七个 step、七个 acceptance、五条 journal event 和一个未解决的 no-cloud 用户约束。
`STEP-ACP-02` 保持 `in_progress`，专门验证模型可以对方向集中 warning 给出有验收引用的
`reasoned_continue`，而不是被强迫转向。

实现文件：

- [`build_global_progress_review.py`](../scripts/build_global_progress_review.py)：schema + semantic preflight、确定性
  direction/acceptance coverage、warning/dedupe key 和 create-new JSON；
- [`verify_global_progress_review.py`](../scripts/verify_global_progress_review.py)：不导入 builder，独立重建 review，
  核验文件 digest、warning 全覆盖和 disposition 语义；
- 四个 runtime schema 分别约束 input、review、disposition 和 verification report；
- CI 新增 `runtime/tests`，Windows/Linux Python 3.11/3.12 均会运行相同 fixture tests。

## 3. 实际观察

本地生成 review ID `GPR-0eb74b6dfc582037`，包含六个 direction 与恰好三条预期 warning。两种 checked-in
disposition 均通过 verifier 的 7/7 checks：

- `reasoned_continue`：继续 `STEP-ACP-02`，明确服务 `ACC-ACP`，保留其他未决方向与下一 review 条件；
- `replan`：不选择下一 step，先映射 `ACC-HANDOFF` 并刷新 stale verification。

专用 unittest 结果为 6/6：

1. 三类 warning、六方向与 source refs；
2. `reasoned_continue` 和 `replan` 均合法；
3. 相同 warning dedupe 后不重复注入且保留 suppressed key；
4. plan revision 与 journal head 篡改导致独立重建失败；
5. warning disposition 缺失与 review digest 错误 fail closed；
6. output overwrite、unknown acceptance 与 embedded task-contract digest 篡改被拒绝。

原有 Grok integration fixture tests 同轮为 18/18；repository check 为 0 errors。

## 4. Artifact ledger

本地 ignored 运行目录：`.observed-runs/global-progress-sentinel-v2/`。早期 dev1 未重算 embedded task-contract
digest，只保留为 pre-fix 调试记录，不作为下表证据。

| artifact | SHA-256 |
|---|---|
| input fixture | `c060ea02360d5637490fbf2ecd00f40d9432b3d5a26c84997bca47775deeab8f` |
| generated review | `1b0d5182375a26f6920f48ec1a5e83c07112c6232d6e23192ff9a96c0949f636` |
| `reasoned_continue` disposition | `9ab681e8c4057811fa3d922e1b498866f91c70e31dae45d893492e1f4ccc9ec1` |
| continue verification | `86cf21e6ba0c1c3a83a88ac578cf8f26df25ce8851f91189217f8b7e59ae6647` |
| `replan` disposition | `0f793218d0a7b49a8db7a3cc8c73d0ff36a8d0535e7a719145c2f820aacf61f6` |
| replan verification | `6dfdac0851dd9a69e074fe2a32bc9210aa0350e47db4ebc316012be544c31c87` |
| builder script | `868a5b30e290660ed1cfca5998438326700521bd9843614450ea928225de2cdb` |
| independent verifier | `4a06d840f659a625f2db932e261589829abee1cfbc2f9a12e0394d7a4809eb69` |

## 5. Observation-first task board

`fep-script-validation` legacy-review board 对 builder 为 11 PASS / 12 WARN / 0 FAIL，对 verifier 为
13 PASS / 10 WARN / 0 FAIL；均无 catastrophic red line。seed、backend、dataset、spike/rate 等 WARN 是研究脚本
validator 与通用确定性控制工具的 domain mismatch，不通过增加伪参数清除。atomic-write WARN 记录一项真实限制：
当前 create-new + fsync 能拒绝覆盖且异常时尝试清理，但进程被强杀或掉电仍可能留下 partial final file；JSON/schema
和独立 verifier 会拒绝该文件，未来正式 runtime 集成前再评估同目录临时文件 + no-overwrite atomic publish。

artifact board 能解析 review 且 finite scan 通过，但无法把 `warnings` 识别为其标准 records/conditions table；因此
target count 的通用检查保持 WARN。专用 schema/unittest 已直接断言 warning count=3，这一结果不能反向提升为
科学或行为正确性证明。

## 6. 结论与下一门槛

本轮证明的是：结构化步骤留痕可以确定性生成方向/验收覆盖和 source-linked WARN；合法的集中推进与主动 replan
都能被显式记录；关键 linkage 篡改不会静默通过。

尚未证明：Grok ACP plan update 能稳定映射 `step_id/direction_id`，真实模型会认真处理 warning，提示注入不会造成
新的偏置或 token 退化，以及 GPS 对长期任务结果有净收益。下一步仅做 no-model ACP `initialize`/capability probe；
在能力映射明确前，不把 GPS 注入真实 Grok/DeepSeek 回合。
