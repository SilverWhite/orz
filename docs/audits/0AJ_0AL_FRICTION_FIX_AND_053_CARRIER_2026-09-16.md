# 0aj／0al 摩擦修复与 0.5.3 载体重建（2026-09-16）

> 来源：0ai 狗粮考核测试 run **`RUN-CLI-6aa999d6`** 考出的三条摩擦
> （[`0AI_DOGFOOD_ASSESSMENT_CLOSURE`](0AI_DOGFOOD_ASSESSMENT_CLOSURE_2026-09-16.md) §3；
> 2026-09-16 用户裁决直接立项 0aj／0ak／0al）。用户 2026-09-16 指示：
> 「先处理狗粮线最新的三条摩擦，先让 orz 可用，然后其他任务都给 orz 侧做，
> 用作测试」。
> 本批范围：**0aj（P1）与 0al（P1）落码 + 载体进件**；**0ak（P2）取证与
> 选项分析 + 用户裁决登记（采 B，零代码）**（见 §4；0ak 状态维持 `pending`）。
> **用户 2026-09-16 追加指示：暂不直接开始新的狗粮线** ⇒ 本批不启动新
> run、不建隔离工作区；§1.4／§2.3 的「判据闭环」与 §4.2 的 0ak 裁决
> 均**不随本批触发**，等用户日后指示再收。
> 源冻结基线：orz **`4049bdf0`**（＝ 0ai 合回 `b682a67f` ＋ 0aj 修复
> `12396e6a` ＋ 版本 bump `4049bdf0`）；载体 **0.5.2 → 0.5.3**（Windows
> 三件套已换装 `D:\tb-eval\orz-windows`）。

## 1. 0aj：`blackboard_write` 端到端接线（已落码，判据待无头 run 复验）

### 1.1 根因（两表脱同步的「同形第三例」）

0ae D0 加工具时改了一侧、漏了另一侧：

- **控制器侧**（`orz-loop/src/tool.rs::risk_class`）**已正确**把
  `blackboard_write` 定为 `ReadOnly`（与 `plan_write`／
  `blackboard_ action_write` 同族）；
- **权限桥侧**（`orz-host/src/permission.rs::access_kind`）**没有对应 arm**
  ⇒ 整工具落进末尾 `AccessKind::Edit` else 分支 ⇒ 无头 `-p`／死网关部署
  （gateway=None，fail-closed）在权限门**确定性 deny**——journal 实录
  `permission_requested{risk: ReadOnly}` → `permission_decision{deny}`，
  `plan_write` 事件 0 条、模型计划/笔记面恒空。

同形前两例：review P1-1（`blackboard_read`，2026-08-08）、0x/0v S4
（`browser_control`，2026-09-11）；本项为**第三例**，且**因跨表护栏样本表
未列该工具而漏网**（§1.3）。

### 1.2 修复面（orz `12396e6a`，5 文件 +163/−5）

1. `access_kind` 内存类 arm 补 `blackboard_write`（`AccessKind::Read(None)`：
   自动放行、无路径限制）——与 `blackboard_read`／`plan_write` 同判据。
2. **探针面补声明**（摩擦的另一半）：`WORK_TOOLS` **23 → 24**，三处同批——
   `orz-loop/src/tool_probe.rs`、`orz-assurance/src/journal/families.rs`、
   `assurance/run_event_journal_validation.py`；判据并入 `probe_storage`
   （会话存储可读，与 `blackboard_read` 同源）。连带更正：工作区不可读时
   `blackboard_write` 与其余工作工具一同从可见面移除（投影测试期望更新）。

### 1.3 钉子（三处，含"为何此前漏网"的补强）

| 钉 | 位置 | 作用 |
|---|---|---|
| `access_kind_mapping` 断言 | `orz-host/src/permission.rs` | **修复前实跑红**（本次先取证：`AccessKind::Edit` ⇒ 断言失败），修复后绿 |
| 跨表护栏样本补 `blackboard_write` | `read_only_tools_never_fall_into_the_edit_bucket` | 该护栏样本表漏列本工具，正是第三例漏网的**直接原因**；样本补齐 |
| 端到端链 | `host_exec/tool_run.rs::blackboard_write_lands_plan_write_event_and_reads_back` | 判据链「调用 → `ToolCompleted{exit_code 0}` → `plan_write` 出账 → `blackboard_read` 读回一致」 |

端到端测试宿主是 `AllowOnce` 测试 seam，**不能证伪权限门**——故权限门一半由
`access_kind` 表级断言承担，两半分工在上表与代码注释中写明。

### 1.4 读数与残余

- orz-loop `--lib` **819/0/3**（818 → +1 新钉）、orz-host `--lib` 串行
  **325/0/5**、orz-assurance **229** 全绿、`cargo fmt --all -- --check` 干净、
  clippy **无新增**（orz-loop lib 52→52；改动行零告警）。
- **判据未闭环**：0aj 判据要求**无头 run 内**实证（allow → `plan_write` →
  读回），本批仅到「代码 + 钉子 + 载体」；闭环读数搭下一轮狗粮 run 收取。
  0aj 状态维持开放（P1）。

## 2. 0al：门禁冻结克隆树漂移（已落码，判据待克隆内复验）

### 2.1 根因（实测取证）

本机存在**可编辑安装**（`__editable__.gsa_assurance-0.1.0.finder.__path_hook__`
⇒ `assurance` 恒解析到 `D:\CLI`）。按文档口径 `python scripts/check_repository.py`
执行时 `sys.path[0]` ＝ `scripts/`（**不含树根**），故 `assurance` 只能经
site-packages 解析 ⇒ 冻结克隆内导入的是**原始树**的
`run_event_journal_validation`（其 `ROOT=D:\CLI`）：路径不同形 → `relative_to`
抛错（门禁崩）；两侧同形 → **静默校验错误的树**。

实测对照（模拟克隆 `T`：内含 `assurance/` 与本脚本副本）：

| 版本 | `gate_ROOT` | `reference_module_root` |
|---|---|---|
| 修复前（`git show HEAD:scripts/check_repository.py`） | `T` | **`D:\CLI`（错树）** |
| 修复后（本批） | `T` | `T`（本树） |

### 2.2 修复面（父仓）

1. 模块级**显式锚定**：`sys.path` 首位插入 `ROOT`（脚本位置推导，禁依赖
   可编辑安装），紧随 `ROOT` 定义，早于任何 `assurance` 导入。
2. **fail-closed 复核** `_check_reference_root_anchor(errors)`：核对载入模块
   的真实来源树 = 本树，不一致即报
   `gate would validate the wrong tree: …`（进 `errors` ⇒ `valid: false`），
   杜绝"崩不了的错树"。
3. 钉子 `assurance/tests/test_gate_root_anchor_nails.py`（4 例）：本树锚定、
   reference 模块归属本树、**外来模块 fail-closed**（负例）、**克隆形态下
   克隆自证**（含 sys.path 投毒 + 还原）。

### 2.3 读数与残余

- 门禁实跑：`valid: true`／`error_count: 0`；新钉 4/4；
  `assurance/tests/test_probe_accuracy_audit.py` 等关联 51 例全绿。
- **判据未闭环**：判据要求「**冻结克隆内**跑 `python scripts/check_repository.py`
  校验克隆自身且与 `-m` 形态读数一致」；模拟克隆对照已取证，真克隆复验搭
  下一轮狗粮 run（其工作区即冻结克隆）收取。0al 状态维持开放（P1）。

## 3. 0.5.3 载体重建与换装（Windows 三件套）

- bump：orz `4049bdf0`（`crates/orz-bin/Cargo.toml` + `Cargo.lock`，
  0.5.2 → **0.5.3**）。
- 构建：`cargo build --release -p orz-bin`（`PROTOC=orz\bin\protoc.exe`）→
  `Compiling orz-bin v0.5.3` → `Finished release profile [optimized] target(s)
  in 1m 34s`，`CARGO_EXIT=0`；warning 仅既有 dead_code 两项。

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 53,624,320 | `79cbb1d67c2aa4e4b905660f7adec658f1ea100e8c2da21599f75c858a7fea2a` |
| orz-signer.exe | 6,738,432 | `969e7e66f959d3f2633541893c6d3bdb6a6afed4893d88db86b3194aad4d2f75` |
| orz-acaf-provision.exe | 6,642,176 | `68f26118f2de1e704be4e2959085ae4ddfc13f0f80421f64bba7729eab888355` |

- **进件核证（修复确实进载体）**：`blackboard_write` 字面量计数
  **0.5.2 = 94 → 0.5.3 = 95**（+1 ＝ `access_kind` 新 arm 的字符串常量；
  `WORK_TOOLS`／判据字符串与既有副本去重）。
- 换装（`D:\tb-eval\orz-windows`）：**先备份后覆盖**——原 0.5.2 三件备份为
  `*.0.5.2-bak` 后再以 0.5.3 覆盖（0.5.2 批 Linux 换装的教训：直写载体的
  批次，`.bak` 必须在覆盖前预留）；换装后逐件 SHA256 与构建产物 **MATCH**
  （三件全对）。
- 冒烟（换装后原位复跑，与 0.5.2 形态一致）：`orz-acaf-provision` 无参
  `usage: … <keystore-root> <manifest-output>` **exit 1**；
  `orz-signer` 无 `signer-manifest.json` **fatal exit 1**。
- manifest：`python scripts/generate_orz_source_manifest.py` → **1456 条**
  （条数不变——本批无新增/删除文件，0ai 批已至 1456），差异面恰 **7 行**
  ＝ 0aj 五文件（`families.rs`／`permission.rs`／`tool_run.rs`／
  `projection.rs`／`tool_probe.rs`）＋ bump 两文件（`Cargo.toml`／`Cargo.lock`）。
- 门禁：`valid: true`／`error_count: 0`。
- **未做（如实登记）**：Linux musl 三件套重建（Docker，含 0.5.2 批的
  代理摩擦，约 30 min）与 GitHub Release／推送——本批未获指示，留用户裁决；
  且用户已指示暂不开始新狗粮线，Linux 面不必为跑批赶工。
- **观察项（未立项）**：裸 `orz --version`（未知旗标）不报错而进入 TUI 并
  挂住（本批一次误触实测；已手工回收进程）。建议随启动器脚本化评估
  「未知旗标 fail-fast」。

## 4. 0ak：增量归档 `-p` 车道不可达（取证完成，**留用户裁决**）

### 4.1 取证（run 证据 + 代码路径）

- run `RUN-CLI-6aa999d6` 跨 500K 里程碑（actual 501,845）但 `.gsa/archives/`
  **0 件**；工作区 `.gsa/session/` 下只有 `terminal/`（无对话侧车）。
- 代码路径：增量归档判定 `incremental_archive_due` 与打包
  `package_session_archive` 均挂在 **ACP 车道**（`orz-host/acp_server.rs`
  prompt 尾 `+ close_session` 两点）；打包内容＝**对话侧车原文**
  （`conversation_sidecar_path`）。而 `-p` 一次性 run **不写对话侧车**
  （GAP-CONVERSATION-RESTORE：one-shot CLI runs carry no session conversation），
  故该车道**无归档源**，非"接线漏了一个调用"那么简单。

### 4.2 选项（需用户裁决；0ak 属 P2「生产化决策门」）

| 选项 | 内容 | 代价／影响 |
|---|---|---|
| **A：登记「归档面 ACP-only」** | 明示 `.gsa/archives/` 三键存档仅 ACP 车道产出；0ah S1「存档一致性（三键齐备率）」判据口径改挂 ACP 车道 | 零代码；**如实登记边界**；无头长 run 判据不再"不可判"而是"不适用" |
| **B：`-p` 车道接归档** | 为一次性 run 引入对话持久化（会话身份 + 侧车落盘 + 里程碑归档），使无头长 run 产出三键归档 | 触及「one-shot run 不携带会话对话」的设计边界（属设计变更，需 ADR 级登记）；实现面中等（侧车装配 + 会话身份 + 判据恢复） |

### 4.3 用户裁决（2026-09-16）：**采 B，本批不实施**

- **裁决内容**：**选 B**（`-p` 车道接归档——为一次性 run 引入对话持久化：
  会话身份 + 侧车落盘 + 里程碑增量归档，使无头长 run 产出
  `.gsa/archives/<session8>.json.gz` 三键归档）。**用户明示「请先将这一裁决
  记录下来，不直接进行改动」⇒ 本批零代码，0ak 状态维持 `pending`（开放＝
  B 的实施，待另行排期放行）。**
- **裁决理由（用户口径，原样登记）**：「UI 部分估计还要相当一段时间才能进行
  适配」——即归档能力不能押在 ACP/UI 车道上（该车道短期内不会成为长会话的
  常态承载），故取能力路径 B 而非边界登记 A。
- **后续实施面（B 的已知代价，登记备查）**：①会话身份——一次性 run 需有
  可归档的会话标识（现仅 `run_id`）；②侧车装配——按既有 `StoredConversation`
  形态落盘（与 ACP 车道同源，禁第二套对话格式）；③里程碑判定复用
  `incremental_archive_due`（≥500K 且 ≥上次水位 +500K，单调幂等）与
  `package_session_archive` 打包原语，避免复制实现；④**设计边界登记**：触及
  「one-shot CLI runs carry no session conversation」
  （GAP-CONVERSATION-RESTORE）语义，属设计变更，须 ADR 级登记与
  ADR-0010 §14 转录；⑤判据——0ah S1「存档一致性（三键齐备率）」在无头车道
  恢复可判（0ak 判据行届时可勾）。
- **不动项**：本裁决不改 ACP 车道既有归档语义、不改 `-p` 车道现有行为、
  不新增工具面。

## 5. 本批未闭合与遗留

1. **0aj／0al 判据闭环**：均搭下一轮狗粮 run 收取（无头 run 实证／冻结克隆
   内复验），本批状态维持开放。
2. **0ak 实施**：裁决已定（§4.3 **采 B**，零代码），**实施待另行排期放行**。
3. **Linux 载体与 Release**：见 §3 未做项。
4. **既有无关红灯（如实登记，非本批引入）**：`runtime/tests`
   `test_v02_all_51_event_types_covered`（账本既有登记）；
   `assurance/tests/test_retrieval_subagent_real.py::test_search_p3_action_authorization`
   ——**本批新观察**：该测试断言 `instruction_gate.py` 落在
   `ProjectDocIndex` 查询 top-10，实测排位 **#11**（语料自写入测试以来增长所致），
   与本批改动无因果（本批新增/修改的文件均不入该查询 top-40）。是否立项留用户裁决。
5. **orz-tui 三个既有红**（`allow_once_executes_tool_with_valid_journal` 等）：
   本批以 `git stash` 对照实证**修复前同形**（无我改动时同样 3 红），定性为
   环境/夹具面既有红，非本批引入。
