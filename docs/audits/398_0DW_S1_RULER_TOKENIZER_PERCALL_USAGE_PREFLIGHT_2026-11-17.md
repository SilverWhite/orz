# 398 批：0dw S1 三件落码——量尺换 provider tokenizer 单源、逐调用 usage 落账、preflight 载体身份章（2026-11-17）

> 用户令：「请先进行0dw S1吧」。范围＝0dw 立项档 §3 S1 全部三件（①量尺换 tokenizer 单源过 perf 关、②逐调用 usage 事件全链、③ preflight 盖载体版本＋构建哈希）。零版本 bump、零重建、零换装（S2 载体进体随下次重建；在役 0.9.15 容忍）。未闭合计数 73 不变（0dw 保持开放至 S3 真机读数）。子仓 orz `e5103563`，未推送。

## ③ run_preflight 载体身份章（最薄先落）

- 新模块 `orz-host/src/carrier.rs`：`carrier_version()`（进程级 `OnceLock` 单写——orz-bin `main_inner` 顶部注入 `env!("CARGO_PKG_VERSION")`，与 `--build-info`／banner／capability `orz_version` 同一 bump 纪录串；未注入如实 `"unknown"`）＋ `binary_sha256()`（运行中 exe 文件 SHA-256，与载体清单同口径；进程内缓存；读失败如实 `"unavailable"` 不虚构）。
- `session.rs bootstrap_session`：`run_preflight`（每 run 首事件）manifest 载荷自三键扩五键（＋`carrier_version`＋`binary_sha256`）。envelope schema 对 preflight 载荷无形状约束（registry/judge 两侧均无此族），扩键安全；立项档 §1.3 的「跨版本行为归因不可为」自此闭合——journal 单源即可判定任一 run 的载体版本与二进制。
- 钉测试：`preflight_pins_carrier_identity`（session.rs）＋ carrier 模块两钉（哨兵值诚实形态）。

## ② `model_call_usage` 逐调用 usage 事件全链

- 事件面：`EventType::ModelCallUsage`（v0.2 非终态机制事件）。载荷＝`agent_role`／`call_seq`（run 内跨车道 1 基严格递增，源＝`RunUsageMeter::model_calls()` 新读数）／`face_estimate_tokens`（请求模型面估算，run_round 前捕获）／五项 token（整数或显式 null——流中断等缺席如实 null 不虚构 0）／`usage_complete`（五项全在 ⇔ true）。
- 登记链全件：envelope schema enum＋1；payload schema `runtime/model-call-usage-event-payload-v0.2.schema.json`（新建）；registry v02 条目；fixtures 三件（payload 正型〔全五项〕／负型〔call_seq=0 违 minimum〕／envelope valid）。
- 法官族双镜像：Rust `families.rs::verify_model_call_usage`（载荷形状＋call_seq 严格递增〔降级刊只丢尾不破单调〕＋usage_complete 与 null 集合机械一致）注册进 `ALL_FAMILIES` 与派发器；Python `assurance/run_event_journal_validation.py::_verify_v02_model_call_usage` 同形注册进派发（花名册 lockstep 注记随批）。对拍 corpus 264 → 266（＋`model_call_usage_ok` 完整五项＋null 缺席两案连排／＋`model_call_usage_violations` 六违例案），Rust↔Python 布尔对拍全绿。
- 生产者＝`agent_loop.rs` usage 折算点（0df fold 同址）：`request_face_estimate` 在 `build_model_face` 后、`run_round` 消耗前捕获；事件落**取消检查点之前**——消耗已发生，取消轮的 usage 行如实留存（该轮 `model_output` 可缺席、链条仍有效）。事件序＝`face_fingerprint → model_call_usage → model_output`（同轮紧邻，估算↔实测逐调用可配对——验收 §4.1「journal 单源复算每次调用估算 vs 实测偏差」）。run 级聚合 `run_usage_metered`（0df）不动，分布面与之分立。
- loop 集成钉：`model_call_usage_lands_per_response_with_pairing_estimate`（controller.rs）。

## ① 模型面量尺换 provider tokenizer 单源（perf 关已过）

- 新模块 `orz-loop/src/ruler.rs`：全局有界缓存（内容 64 位哈希 → token 数，65536 条、满表清空重暖）＋ `text_tokens`／`message_tokens`（命中路径零分配；字段集与旧 chars/2 完全一致：content＋reasoning＋tool_calls 的 name/序列化 arguments，role 与 tool_call_id 不入尺）。tokenizer `Err` ⇒ chars/2 保守回退（内嵌资产实践中不可达；硬预算面 state/recenter/context_map 各自的 fail-closed `Result` 语义不受影响）。
- 三处接入：`compact::estimate_message_tokens`、`model_face::injected_estimate`（指针/D4/分块表/结束行）、`agent_loop::estimate_static_overhead`（系统提示词＋工具定义）。同尺随批换装面＝阶梯（192/224/256/288/320/500K）／主滑块切点 `bridge_cut`／分块累计／era 折叠分组（391 ext 单源同参）／台账指针化循环／注入预算（50K 语义不变、估算换尺）／context_map 行标签（自动继承块估算）。chars/2 局部口径在 orz-loop 量尺面退役；orz-host context_view 全量会话估算仍 chars/2（333 批登记的 UI 近似，不在模型面范围）。
- **perf 关读数**（探针 `perf_probe_full_encode_vs_cached_increment`，`#[ignore]` 入库）：482,400 token 工作点（1.45M 字符、300 轮）＝冷全量编码 **2.32s** → 暖缓存全量走读 **2.9ms**（≈800× 坍缩）→ 追加一轮后全量走读 **8.8ms**（增量形态：steady-state loop-top 只编码新尾轮）。逐 loop-top 开销自秒级坍缩回毫秒级（与旧 chars/2 走读同量级）。
- 测试漂移对齐（21＋2 处，量尺换装的合法连带）：CJK 载荷夹具（`conversation`/`s1_actual_context_prompt` 降 450K 字符〔≈450K token，避 700K 守卫〕/map 与 clear 测试载荷）＋按轮推导预算（`payload_round_est`：滑块 k×轮估算／块 N×轮估算，量尺无关拓扑）＋map 标签断言 5Ktk→10Ktk。

## 读数与门禁

| 套件 | 读数 |
|---|---|
| orz-loop lib | 986/0/3 |
| orz-assurance lib | 311/0（含对拍 266 场景＋fixture 刊、corpus 计数钉 264→266） |
| orz-host lib（串行，RUST_MIN_STACK=32M） | 462/0/6（＋3 新测试） |
| orz-bin 六套件 | 全绿（stdio_e2e 1/1） |
| orz-tui | 186/9（既有 ACAF 环境基线，零新增失败） |
| Python runtime conformance | 17/17（registry/schema/fixtures/envelope 全链） |
| Python assurance journal 族 | 52/52 |
| clippy 五 crate | 120=120 持平 |
| fmt／`git diff --check` | 净 |

## 随批对齐与漂移注记

- **事件计数钉 11 → 12**（acp_server ×2＋stdio_e2e ×1）：本批 `model_call_usage` 的直接序列演化（非漂移），随批对齐并注记。
- **`explicit_background_surface_visible_in_production_toolset`（host）＝397 前既有漂移**：0dz 后台墙钟策略改 `is_background` 描述为四臂如实形态（"until they exit, the session ends, you stop them, or an explicit timeout fires"）后旧子串断言失效；干净基线 `f6fb9412` stash 对照同形失败核实（397 未跑全量 host 套件故未暴露）。沿 146/298「随批对齐实现事实」先例对齐为三段断言。
- orz-host 测试进程在默认栈下 `STATUS_STACK_OVERFLOW`＝既有环境性（干净基线同形；RUST_MIN_STACK=32M 串行可跑），非本批引入，登记不修。

## 边界与不做

- 注入预算（50K）与检索 session-end 模板压缩同尺随批换装——单一量尺纪律（保留第二把 chars/2 尺即保留下项谴责的双尺漂移）；预算语义（估算 token ≤ 50K）不变。
- 零发行、零推送（发行与推送随用户令；S2 进体随下次载体重建）。
- S3 真机核证（CJK 主会话逐调用估算 vs 实测偏差读数、阶梯触发与实测对齐、阈值裁定）待真机窗口——可与 0dy/0dz/0dx/0dt 合并首启窗口同车或其后单独执行。

## 入口

- BACKLOG `0dw`／TODO `0dw` 条目（S1 达成改注）／索引头 v4.358。
- 关联：386 立项档（§1 三件机械事实／§4 验收判据）／0df `run_usage_metered`（run 级聚合面，本批分立不覆写）／0bz `face_fingerprint`（同轮紧邻的配对锚）／376/M1-S2（tokenizer 硬预算先例）。
