# 任务 D S2b 复审处理批（2026-09-06）

> **审计日期**：2026-09-06
> **性质**：S2b 批（orz `809cdb4e` + 父仓 `ccc4be4`）三路全面复审（逐族
> 语义符合性 / 接线与测试基建 / 审计与登记符合性）的发现处理。复审结论：
> 语义镜像 5/7 入口精确等价，发现 **2 项 P1（会改变裁决结果）**、测试基建
> 与登记数字问题若干，本批全部收口（代码修复 orz 本批提交 + 父仓登记）。
> **关联前序**：[S2b 实施审计](TASK_D_S2B_FAMILIES_IMPL_AUDIT_2026-09-06.md)
> / [S2a 盘点表](TASK_D_S2A_INVENTORY_2026-09-06.md)。

## 1. P1 处置（会改变裁决结果，已修复）

### 1.1 policy_denial permission 工具集过宽

- **发现**：Rust `permission_gated` 复用了 `_is_retrieval_mode_gated_tool`
  前缀谓词（额外放行 web_search/web_fetch/retrieve_project_* 家族）；
  Python `_PERMISSION_GATED_TOOLS = _WORK_TOOLS | _RETRIEVAL_MODE_GATED_TOOLS`
  是 26 个精确工具名。`source=permission` + `tool=web_fetch` 的 schema-valid
  期刊：Python 报错、Rust 放过。
- **修复**：`families.rs` 改为精确集合 `WORK_TOOLS ∪ RETRIEVAL_MODE_GATED_TOOLS`
  （不含前缀谓词），注释登记根因。
- **锁定**：新增场景 `policy_denial_permission_web_fetch`（期望违规）+
  `policy_denial_permission_ok` / `policy_denial_taint_ok` /
  `policy_denial_retrieval_mode_ok`（期望通过）。

### 1.2 conformance 第 5 阶段 V01 短路前提不成立

- **发现**：跳过注释声称「every family filters `_is_v02`」——但 Python 的
  `_verify_v02_policy_denial` / `_verify_v02_failure_target` **无 `_is_v02`
  过滤**（只匹配 event_type），且 v0.1 tool-completed payload schema 同样
  定义这两个字段：携带违规的 schema-valid V01 期刊 Python 报错、Rust 整段
  跳过。
- **修复**：`conformance.rs` 第 5 阶段去掉 `first_track == V02` 条件——
  payload 无错即运行全家族（5 个带过滤的族在 V01 刊上为 no-op，与 Python
  逐事件过滤语义等价；另两族在 V01 上照常生效）。
- **锁定**：对拍语料纳入 6 个 v0.1 fixture 期刊（双侧裁决一致）；端到端
  家族负测（§2.3）经完整 `validate_journal_file` 管线锁死门控接线。

## 2. P2 处置（测试基建加固，已实施）

### 2.1 对拍测试稳健性

- **stdin 编码**：语料含非 ASCII fixture 文本，Windows 非 UTF-8 代码页下
  `json.load(sys.stdin)` 会环境性假失败——子进程加 `-X utf8`。
- **恒真底线**：`checked >= 7*(fixture_names.len()+35)` 化简为 `39≥35`，
  fixture/场景丢失静默通过——改为精确核算
  `assert_eq!(checked, 7 × corpus.len())` + 独立下限守卫（场景 ≥58、
  真实 fixture ≥18）。
- **失败诊断**：python spawn 失败改为带命令名与 `ORZ_PYTHON` 逃生提示的
  panic；stdin 写入失败显式归因；fixture 目录缺失/为空带路径断言；补
  ORZ-BUILD-MOUNT-001 挂载契约守卫（assurance/ 存在性）。

### 2.2 覆盖缺口补齐（+21 场景，39 → 60）

复审标出的 schema 兜不住、此前「Rust 写错也无法发现」的高风险子规则全部
落场景（含正/负两向）：off/local_browser 窗口内仅凭 target 命中的 dispatch
子句（原场景被 browser_read 双命中掩盖）、failure_target 的
file_target/url_target 分支（零场景 → 各一正一负）、lifecycle close 绑定段
负路径（decision/outcome/contract_revision/result_digest 错绑四向）、
`rejected_stale`/`rejected_conflicting` pass-through、
`replayed_idempotent` 首现语义（复审澄清：两侧均判 unknown outcome，close
引用分支不可达——场景钉死该 parity 结论）、restore 声明负分支（不先行 /
activation 不匹配）、continue+1 负路径、`fold_start ≥ fold_cut` 不变量、
窗口内 rounds 收缩。

### 2.3 端到端家族负测（补门控接线证据）

新增 `family_stage_tamper_detected_end_to_end`（fixture_journal_conformance）：
篡改真实期刊 orientation-fire-run 的 disposition CAS 字段 + 重封 payload/
event digest 与 previous 链（`resealed_journal` 助手）→ 经完整
`validate_journal_file` 断言**仅家族阶段报错**（无 envelope/payload/chain
错误混入）。此前若第 5 阶段门控条件写反，全套件依然全绿——该缺口已闭合。

## 3. 登记勘误

| 项 | 初版登记 | 实际/更正 |
|---|---|---|
| 合成场景数 | 38 | 初版实际 39；复审处理后 **60** |
| 表驱动裁决格 | 266 | 初版实际 273；现 **420**（60×7） |
| 对拍语料 | 50（38+12 v0.2） | 初版实际 51；现 **78**（60+12 v0.2+6 v0.1） |
| 对拍裁决格 | 350 | 初版实际 357；现 **546**（78×7） |
| 集成测试数 | 17 | 16（+1 doctest 计入口径差）；现 **18**（含端到端负测） |
| manifest 条数 | 1437 | 实为 1438（families.rs 入册；以当批重算为准） |

错误数字曾扩散至 orz `809cdb4e` message、父仓 `ccc4be4` message、BACKLOG
00 S2b 行、TODO S2b 行与实施审计 §2——历史提交不改写，以本表为勘误权威；
BACKLOG/TODO/实施审计已同步更正。方向均为少报（实际覆盖多于声称），非虚功。

## 4. 接受维持 / 留档项

1. **超过 i64 的 JSON 整数**：Python 视为合法大整数，serde_json 解析为
   f64 → `py_int` 返回 None → Rust 误报。schema 仅锁 `integer` 无 64 位
   上限，但生产者只写机器整数、语料不可达；修复需任意精度算术，超出双法官
   复核器定位——接受为已登记偏差。
2. **浮点 revision 比较**（Python `1.0 == 1` 放过 / Rust 拒绝）：schema 锁
   `type: integer`，仅绕过门控直接调用函数的畸形语料可达——接受。
3. **消息文本 Rust 形态**（`<missing>` vs `None` 等）：规格即有意差异，
   对拍按裁决级。
4. **S2B_FAMILIES 顺序**：已修正为 Python 调用序（lifecycle 移至第 3 位），
   消除「自称 Python 调用序」的措辞偏差。
5. **orz message 入口误指 S2a 盘点表**：历史提交不改写，本审计为正确入口。
6. **`index <= issue_index` 双向死代码**（control_tickets）：单遍扫描下
   issued 索引恒小于当前索引，两侧逐字镜像的对称死分支——保留（Python
   对等），注释已在 Python 侧说明。
7. **「fixture 正/负对拍」措辞**：初版 fixture 级负测缺失（由合成负测含糊
   承接）——本批端到端 tamper 负测补齐后措辞成立；BACKLOG/TODO 同步改写。

## 5. 本批验证

- orz-assurance 全量：lib 204 + 集成 18（fixture 8 含端到端负测）全绿；
  `cargo check --workspace` 零警告零错误；fmt/clippy 净（families.rs 零
  告警）。
- 对拍 0 差在修复后重验：78 语料 × 7 族 = 546 裁决格全一致（含 P1 修复的
  双向锁定场景与 6 个 V01 期刊）。
- 涉及文件：`families.rs`（P1-1 修复、+21 场景、期望表、对拍加固）、
  `conformance.rs`（P1-2 修复）、`fixture_journal_conformance.rs`（端到端
  负测 + 重封助手）、本审计、实施审计 §2、BACKLOG 00、TODO、S2a 盘点表
  §6 指针。
