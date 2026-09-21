# ORZ-ENV-POLLUTION-001 — 环境污染假红：测试读数先核 env（案例候选）

- **状态**：`candidate`（2026-09-17 发现当日案例化候选；未宣称产品级闭环）
- **晋级理由**：环境类事故，复现命令明确、根因机械（启动 env 沿进程树下沉＋测试对预置 env 敏感）、同 run 双族（orz-loop 批量假红／orz-host 4 条）对照归因完整，且**误读代价高**（假红语义可信，足以打偏「既有红灯」基线判断）——具备可复用的读数纪律价值。
- **来源证据**：[`docs/incidents/ORZ-ENV-POLLUTION-001.md`](../../incidents/ORZ-ENV-POLLUTION-001.md)（登记号 FR-N02，处理批报告 [`FRICTION_INVENTORY_TREATMENT_2026-09-17` §2/§4](../../audits/FRICTION_INVENTORY_TREATMENT_2026-09-17.md)）
- **能力**：①**读数先核 env**——任何「测试红／绿」结论产出前，先核对进程 env 是否被启动器/外层会话预置（`ORZ_ACAF_*`／`ORZ_ALLOW_*`／`ORZ_REAL`／`ORZ_MAX_WALLCLOCK`／`GROK_HOME`／`GROK_AGENT`）；②**清 env 口令**——测试前 `unset` 上述键（或显式空置），读数口径注明「清 env＋串行」；③**假红识别特征**——panic 文案指向 fail-closed 执法路径（如 `ACAF fail-closed ... refusing`）而非断言失败时，优先怀疑环境态而非代码。
- **回归入口**：
  - 反例（污染态）：`ORZ_ACAF_FAIL_CLOSED=1 cargo test -p orz-loop --lib` ⇒ 批量 panic 假红；
  - 正例（清洁态）：清 `ORZ_*`/`GROK_HOME`/`GROK_AGENT` 后同一命令 ⇒ 791/0/3。
  - 修复形态（若后续批实施「测试初始化忽略 launcher env」，回归时按此核对）。
  - **0bc 落点（2026-09-21）**：新增测试入口脚本 `scripts/run_orz_tests.ps1`——自动清除 `ORZ_ACAF_*` 四键＋`ORZ_LIF_RLI_SHADOW`（＋ `RUST_MIN_STACK` 默认值），透传参数给 `cargo test`；经该脚本运行即得清洁态读数（0bc 批 orz-loop 全量测试使用该入口）。
  - **同族实例（2026-09-21，0bc 现场）**：`controller::tests::rli_reference_face_renders_shadow_signal` 假定 `ORZ_LIF_RLI_SHADOW` 未开（控制面构造处读 env）——dogfood 会话开影子时该测试假红（"RLI 影子 on" 读数混入「未启用」首断言）；清该键后单跑绿。已纳入脚本清理列表（代码级隔离留后续批）。
  - **同族实例（2026-09-21，0bc 现场）**：`orz-host` `grok_home::tests::*` 四条假定 `GROK_HOME` 未设——dogfood 会话设 `GROK_HOME=D:\tb-eval\orz-windows\grok-home`＋`GROK_AGENT=1` 时 `redirect_grok_home_with` 短路为 `EnvRespected`，四条断言全红；清两键后全绿。已纳入脚本清理列表（清理列表＝上述能力②「清 env 口令」的脚本化落点）。
- **同族先例**：`ORZ-TOOL-BINARY-COMPAT-001`（归因纪律：环境/机械因素先于代码归因）、`ORZ-VERDICT-EPOCH-001`（数据正确 ≠ 结论当前有效——本条补「读数干净 ≠ 结论可信，先核 env」）。
- **验证记录**：2026-09-17 run `RUN-CLI-6aac0af5` 内双族对照（orz-loop 清后 791/0/3；orz-host 未清 328/4/5 ⇒ 清后串行 332/0/5，主会话独立复现同读数）。
