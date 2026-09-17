# ORZ-ENV-POLLUTION-001 — 环境污染假红：测试读数先核 env（事故登记）

- **状态**：已归因 / 处置配方已固化（2026-09-17，摩擦处理狗粮 run `RUN-CLI-6aac0af5` 期间发现；登记号 **FR-N02**，由 orz 内 agent 自主发现并登记于处理批报告 §2）
- **分类**：`harness_environment`（测试环境与读数归因）
- **现象**：狗粮启动 env（`ORZ_REAL=1`、`ORZ_ALLOW_WRITE/SHELL/NETWORK=1`、`ORZ_ACAF_FAIL_CLOSED=1`、`ORZ_ACAF_MANIFEST/KEYSTORE/BINARY`、`ORZ_MAX_WALLCLOCK=0`、`GROK_HOME`、`GROK_AGENT=1`）被测试进程继承 ⇒ `cargo test -p orz-loop` **批量假红**（panic：`ACAF fail-closed is enabled but no signer client is configured`，例 `blackboard.rs:3272` 区）；清 `ORZ_*` 后 **791/0/3 全绿**。同 run 内 `orz-host` 族同因：未清 env 串行 328/4/5，清 `GROK_HOME`/`GROK_AGENT`/`ORZ_*` 后 **332/0/5 全绿**（FR-A11 归因实验：`GROK_HOME` 单独预置⇒败、`GROK_AGENT` 单独预置⇒过）。
- **根因**：狗粮启动器为生产语义装配的 env（fail-closed 门、资源/权限键、模型家目录）沿进程树下沉到 cargo test；测试断言对「宿主 env 预置」敏感（ACAF fail-closed 无 signer 即拒、grok_home 测试对预置 `GROK_HOME` 不免疫），把**环境态**呈现为**代码红灯**。
- **放大因素**：假红批量且语义可信（panic 文案是真实执法路径），与「既有红灯」基线记忆混在同一读数面 ⇒ 极易把环境污染误读成回归（本 run 内 agent 首次全量读数即被打偏，二次归因后才复位）。
- **处置**：测试运行口令固定**先清 env**（`ORZ_ACAF_*`、`ORZ_ALLOW_*`、`ORZ_REAL`、`ORZ_MAX_WALLCLOCK`、`GROK_HOME`、`GROK_AGENT`）再跑；读数口径入处理批报告 §4（「后续批引用本次读数时以 §4 口径为先：清 env＋串行」）。产品面修复候选（测试初始化忽略 launcher env / 对预置 env 免疫）留后续批评估。
- **案例化理由**：见 [`案例候选`](../cases/harness_environment/ORZ-ENV-POLLUTION-001-env-pollution-fake-red.md)。
