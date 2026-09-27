# 111 0bz S1 指纹件落码（模型面前缀指纹事件族；2026-09-28）

> **日期**：2026-09-28；**用户令**：「立项完成后就请你按照你提出的『下一步建议』开始吧」（110 立项批后的续作）。
> **本批＝`0bz` S1 落码＋钉子**：模型面前缀指纹事件族 `face_fingerprint`——**纯观测面**（不改请求内容、不进工具面、模型零感知）；**计数 56 不变**（S1 达成、`0bz` 维持 `pending`）。
> **形态**：orz 子树单笔 `7a6fe91e`（7 文件 `+644/−2`）＋父仓合约面（schema／注册表／载荷 schema／fixture／Python 镜像）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 新事件族 | `face_fingerprint`（v0.2 非终态机制事件）：逐请求对投影视图做逐消息 sha256 指纹＋与上一请求的首分歧判定 |
| 指纹粒度 | **逐消息**（实现勘定：v8 视图＝消息列表投影，消息即装配自然单元——比设计稿的「分段」更细、定位更直接） |
| 哈希输入 | **wire 同字段投影**（`map_message` 五字段：role/content/tool_call_id/tool_calls/reasoning_content）；`round` 不落 wire 故不入哈希——防内部轮章变化造成**假分歧** |
| 生产者 | 主车道每请求 `build_model_face` 后 emit；`model_round`（run 内 1-based 严格 +1）；事件紧邻 `model_output` 之前 ⇒ 「分歧消息 ↔ miss」逐轮对账 |
| 法官族 | `verify_face_fingerprint`（Rust）↔ `_verify_v02_face_fingerprint`（Python 镜像，逐格同形）：payload 形状＋**跨事件 LCP 重算对账**＋`first_divergent` 两侧哈希核证＋轮序连续性 |
| 读数 | orz-assurance **278/0**（含 Rust↔Python parity 绿）／orz-loop **858/0**（3 ignored）／orz-tui **178/0**／conformance **16/16**；fmt 噪声与 HEAD 同面；clippy 零新增 |
| 随批勘误 | 合约钉值 65 → 67（`test_v02_all_event_types_covered` 在 0bw③ 加 `write_control_review` 时漏校准——HEAD 上已红，本批随批校准） |
| 状态 | `0bz` S1 达成；余项＝S2 真机采集〔须先载体重建进体〕／S3 两处稳定化修码／S4 真机对账 |

## §1 设计与实现

### 1.1 指纹本体（`orz-loop/src/model_face.rs`）

- `face_fingerprint(&[Message]) -> FaceFingerprint`：逐消息条目 `{role, chars, hash(12 hex), head}`。
  - 哈希＝对 wire 投影 JSON（BTreeMap 键序规范化）取 sha256 前 12 hex；投影字段与 transport `map_message` 一一对应（`round` 除外——**模型不可见字段不入哈希**）。
  - `head`＝诊断面（content 前 120 chars；tool_calls/纯结果消息给 `[tool_calls: …]`／`[tool_result]` 机械摘要）——仅驻内存，分歧时才落 journal。
- `FaceFingerprint::first_divergence(prev)`：最长公共哈希前缀（LCP）判定——`None`（逐条全同）／`mutated`（同位异哈希）／`inserted`（尾部追加，正常轮尾巴形态）／`removed`（尾部收缩，压缩落地轮形态）。
- `digest()`＝逐条 hash 串（`;` 连接）再 sha256 取 16 hex（全脸对账键）；`compact()`＝`role:chars:hash12` 分号串（事件载荷面）。

### 1.2 生产者（`orz-loop/src/agent_loop.rs`）

- 位置：`build_model_face` 成功之后、`run_round` 之前——journal 上事件紧邻其后的 `model_output`，同轮 hit/miss 读数可直接对账。
- 载荷：`agent_role`（恒 `main`）／`model_round`／`message_count`／`total_chars`／`face_sha256`／`messages`／`stable_prefix_messages`／`first_divergent`。
- 跨轮状态：`prev_face_fingerprint`（首分歧基准）＋`face_model_round`（1-based 严格 +1）——仅主车道。

### 1.3 合约面（父仓 `runtime/` ＋ `assurance/`）

- 信封枚举 `run-event-v0.2.schema.json` 66 → 67；注册表 `tracks.v02` 注册（slug `face-fingerprint`）。
- 载荷 schema `face-fingerprint-event-payload-v0.2.schema.json`（新建）：`messages` 紧凑表正则、`face_sha256` 16-hex、`first_divergent` 九字段对象（`hash`/`prev_hash` 允许空串＝removed 无当前侧——语义核证交判官）。
- fixture 三件由 `generate_run_event_fixtures.py` 同批扩展后重生成（good＝三轮 progression 之轮 2 形态；bad＝未知 role 破坏紧凑表模式）。
- Python 镜像 `_verify_v02_face_fingerprint`（166 行）：与 Rust `verify_face_fingerprint` 逐格同形（形状＋LCP 重算＋kind/hash/head 交叉核证＋轮序）。

### 1.4 判官核证面（Rust ↔ Python 同形）

1. `agent_role == "main"`；`model_round` 从 1 起逐事件严格 +1。
2. `messages` 逐条 `role:chars:hash12` 可解析、`message_count` 与条数一致、`face_sha256` 16-hex。
3. **跨事件对账（实质面）**：`stable_prefix_messages` 必须＝上一事件 `messages` 与本事件的重算 LCP；`first_divergent.index == LCP`；`kind` 由 LCP 与两侧长度唯一决定；`mutated` 两侧 hash 各对上当前/上一事件同位条目；`inserted` 无 prev 侧；`removed` 无当前侧 hash 且 `prev_hash` 对上被删首条；mutated/inserted 必带 head。

## §2 钉子（先红后绿机械面）

1. `face_fingerprint_same_wire_same_hash_and_round_field_excluded`：同 wire 同哈希／**round 变化不入哈希**（假分歧防护钉）／content 与 tool_calls.arguments 变化必变哈希。
2. `face_fingerprint_digest_and_compact_shape`：digest 16-hex、compact 格式、head 机械摘要形态。
3. `first_divergence_insert_mutate_remove_and_none`：inserted（尾部追加）／mutated（早期同位异哈希——空跑 compress 与自发塌陷的候选形）／removed（尾部收缩）／全同 None 四形。
4. families 场景 `face_fingerprint_progression_and_violations`：三轮 progression（冷启动→inserted→mutated 违例）＋`expected_violations` 注册＋corpus 钉值 258 → 259＋Rust↔Python parity 绿。

## §3 读数与回归

| 套件 | 结果 |
|---|---|
| `cargo test -p orz-assurance --lib` | **278 passed / 0 failed**（含 `s2b_family_verdicts_match_python` parity 绿） |
| `cargo test -p orz-loop --lib` | **858 passed / 0 failed**（3 ignored） |
| `cargo test -p orz-tui --lib` | **178 passed / 0 failed** |
| `python -m pytest runtime/tests/test_run_event_conformance.py` | **16 passed** |
| `cargo fmt --check`（两 crate） | 噪声与 HEAD 同面（既有 rustfmt 版本噪声不变，零新增） |
| `cargo clippy`（两 crate） | 零新增 |
| 事件序列钉值校准 | `controller::run_turn_full_gate_sequence`／`tool_run::text_deltas_forwarded_in_order…` 期望序列补 `FaceFingerprint`（每 `ModelOutput` 前一条——新事件的既有测试面兑现） |

## §4 边界与如实记

1. **既有 flaky（与本批无关）**：`user_cancel_closes_pending_activations_before_run_cancelled` 偶发红（计数断言 0 vs 1）——干净树（stash 后）单跑可复现、整轮跑绿；时序敏感面，如实登记不处置。
2. **既有漏校准随批勘误**：`test_v02_all_event_types_covered` 的枚举钉值 65 在 0bw③（094 批）加 `write_control_review` 时未校准（HEAD 枚举已 66、测试已红）——本批随批校准 65 → 67 并注释两步来历。
3. **磁盘空间事件**：`cargo test -p orz-tui` 首跑遇 D 盘满（os error 112）——清理 `orz/target/debug/incremental`（5.3 GB，可再生构建产物）后复跑绿；D 盘余量自此 5 GB 级，后续重建批需留意。
4. **orz-host unused import 警告**（`permission.rs` Path/PathBuf）＝109 批已注记的既有面，不动。
5. **未做**＝S2 真机单轮采集（须先随载体重建进体——重建待用户令）；S3 两处稳定化修码（S2 定位的前置）；未推送。

## §5 台账同步

- TODO：`P1-0bz` S1 勾选＋S2 钉子半注记＋计数行改指本批。
- BACKLOG：`### 0bz.` S1 bullet＋状态行＋计数行/指针改指本批（第二卷 §1.63）。
- 索引：§6 `0bz` 条目补 S1 达成＋头行 v4.78 → **v4.79**（v4.78 滚入存档卷 154 → 155 行）。
- 第二卷：§1.63。orz pin `93625868` → **`7a6fe91e`**；源清单重生成 1506 条（`--check` valid）。
- **门禁**：`python scripts/check_repository.py` ⇒ `valid: true`（error_count 0）。

## §6 关联与关键词

[`110 立项档`](110_CONTEXT_FACE_TRANSIENT_FORK_REGISTRATION_2026-09-28.md)／[`0bm 报告 §8`](0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md)／[`0bi §10-④`](0BI_FRICTION_CARRYOVER_2026-09-23.md)／[`model_face.rs`](../../orz/crates/orz-loop/src/model_face.rs)／[`agent_loop.rs`](../../orz/crates/orz-loop/src/agent_loop.rs)／[`families.rs`](../../orz/crates/orz-assurance/src/journal/families.rs)／[`run_event_journal_validation.py`](../../assurance/run_event_journal_validation.py)／BACKLOG `0bz`／TODO `P1-0bz`。

关键词：模型面前缀指纹、`face_fingerprint`、首分歧、LCP、wire 投影哈希、mutated/inserted/removed、假分歧防护、round 不落 wire、判官族、Rust↔Python parity、0bz S1、计数 56 不变。
