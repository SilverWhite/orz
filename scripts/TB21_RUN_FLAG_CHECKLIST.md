# TB21 `run_official_2.1.sh` 起跑前旗标对账清单

> FR-D01 处置（2026-09-17 摩擦处理批）；来源＝[TB21_V41_ROUND0_MEMORY_HEAVY_START §6.9](../docs/audits/TB21_V41_ROUND0_MEMORY_HEAVY_START_2026-09-13.md)（索引 `GAP-ORZ-ADAPTER-FLAG-DRIFT`）。
> 背景＝适配器（`D:\tb-eval\tb_agents\orz.py`）对三类旗标的处理与装置预期不完全同步——最易把「装置缺件」误读成「镜像缺陷」。本清单固化「起跑前对账」；**不改装置侧代码**，读数与归因以两个示例场景各自登记。

## 一、三类旗标现状对账（每轮 run 前逐项核对；以现行适配器实读为准）

1. **`--max-tool-rounds`**（工具轮上限）
   - 立案口径＝**静默忽略**（不生效、不告警）。
   - 2026-09-17 复读 `tb_agents/orz.py`：`_DEFAULT_MAX_TOOL_ROUNDS = 999`（评测门；产品默认 40，ADR-0008），经 `--allow-write --allow-shell --max-tool-rounds {rounds}` 传递；env 后备 `ORZ_MAX_TOOL_ROUNDS`。
   - **对账点**：起跑后从 run 读数确认真实生效的工具轮上限并记录（勿以 999 假定）。
2. **`--retrieval-mode`**（检索模式）
   - 立案口径＝**已弃用**（WARN 后忽略）；2026-09-17 复读未见于现行传参路径。
   - **对账点**：确认本次 run 的检索面实际配置（如 `retrieval_enabled` 读数）；「检索面为空」不得直接判镜像缺陷。
3. **`eval_browser`**（浏览器评测开关）
   - 立案口径＝**未传递**；2026-09-17 复读：`--ak eval_browser=true` 或 env `ORZ_EVAL_BROWSER=1` 可开（`install()` 配浏览器面）。
   - **对账点**：起跑前确认开关状态；容器内无浏览器为**设计内边界**（0ac 判据），缺件读数须回宿主面核对后再归因。

## 二、起跑前打勾清单（逐项勾选留痕）

- [ ] 已读本清单，并对照**现行**适配器/启动脚本版本（三者现状可能已漂移）。
- [ ] 已记录「本次实际生效的旗标组合」：工具轮上限实际值＝____；检索模式实际值＝____；浏览器评测实际值＝____。
- [ ] 出现「工具轮早停 / 检索面空 / 浏览器缺件」类读数时，**先按本清单核对**，不得直接判为镜像缺陷。
- [ ] 归因结论二选一登记（附证据：命令行/日志/读数）：装置缺件（装置侧修）／镜像缺陷（orz 侧修）。

## 三、关联

- FR-D02（同属装置侧）：修复已落（`run_r0_heavy_official.py` 传 `--ak max_wallclock`，commit `419b1c8f`），**待实机复验**——随 0ac S4 收取；本条仅覆盖「起跑前对账」。
- 处理批报告：[`FRICTION_INVENTORY_TREATMENT_2026-09-17`](../docs/audits/FRICTION_INVENTORY_TREATMENT_2026-09-17.md)。
