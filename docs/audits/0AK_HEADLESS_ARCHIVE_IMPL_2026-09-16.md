# 0ak 实施：无头 `-p` 车道会话持久化与里程碑增量归档（2026-09-16）

> 来源：0ai 狗粮考核测试 run `RUN-CLI-6aa999d6` 考出的摩擦之二
> （跨 500K 里程碑而 `.gsa/archives/` 0 件）；取证与选项分析见
> [`0aj/0al 修复与 0.5.3 载体`](0AJ_0AL_FRICTION_FIX_AND_053_CARRIER_2026-09-16.md) §4。
> **用户 2026-09-16 裁决：采 B**（为一次性 run 引入对话持久化，理由「UI 部分
> 估计还要相当一段时间才能进行适配」⇒ 归档能力不押 ACP/UI 车道）；同日
> 用户放行实施（「请先进行 0ak 的剩余部分吧」）⇒ 本批落码。
> 本批不新增未闭合项（计数维持 36）；**0ak 状态维持开放**——判据行
> （无头长 run ≥500K 实证产出三键包）维持未勾，搭下一轮狗粮 run 收取。
> 设计转录：ADR-0010 **§14.68 / v1.69**；索引条目 `GAP-INCREMENTAL-ARCHIVE-HEADLESS`。

## 1. 实施五面（对齐裁决 §4.3 的登记）

| 裁决登记面 | 本批落地 |
|---|---|
| ① 会话身份 | 无头一次性 run 会话身份＝`{ts}-cli`（ts = run id 同秒后缀；session8 = ts，与 `RUN-CLI-{ts}` journal 目录一眼互认）。每次 `-p` 全新会话，**跨调用恢复不开启**（GAP-CONVERSATION-RESTORE 的「无恢复路径」边界保持） |
| ② 侧车同源 | 成功 run 收尾按既有 `StoredConversation` 形态装配（`StoredConversation::full`：对话＋LIF 会话轴＋黑板 live 快照＋v7 刻度水位），禁第二套对话格式 |
| ③ 复用归档原语 | 里程碑判定（`incremental_archive_due`：≥500K 且 ≥ 水位 +500K，单调幂等）、打包（`package_session_archive`：sidecar 原始字节零变换嵌信封＋三键互标）、ARC 审计 journal（`session_archive` v0.2 带 `incremental:true`＋水位落盘）**全部复用 ACP 车道既有实现** |
| ④ 设计边界登记 | 触及「one-shot CLI runs carry no session conversation」⇒ 已 ADR 级登记（ADR-0010 §14.68 / v1.69，导言区补记同批） |
| ⑤ 判据恢复 | 0ah S1「存档一致性（三键齐备率）」在无头车道恢复**可判**；0ak 判据行待下一轮狗粮 run 收取后可勾 |

## 2. 设计要点

### 2.1 对话携带（设计边界改写的主项）

`orz-bin run()` 以空 `Vec<Message>` 传入 `run_turn_with_guards(..., Some(&mut
conversation))`（原传 `None`）。空起点与 ACP 全新会话**同形**——seed 路径
byte-for-byte 等价（`unwrap_or_default()`），不改变首轮请求形态。随之与 ACP
同源生效的一项内在行为＝长 run 收尾的**会话末机械压缩**（`session_end`，
纯机械零模型调用）——这是「携带会话对话」设计变更的登记组成部分，非静默
漂移。成功后控制器回写过滤断言（注入块滤除、压缩 marker 保留）与 ACP 车道
同码，无新过滤逻辑。

### 2.2 归档接线（单一公开入口）

新增 `orz_host::acp_server::headless_session_archive()`（公开 API，orz-bin 在
journal `shutdown_async` 之后调用）：

1. `StoredConversation::full(...)` 装配 ＋ `context_scale_notified` 水位回写；
2. `incremental_archive_due` 判定——一次性 run 水位恒新 ⇒ due ⇔ ≥500K；
3. due ⇒ `persist_conversation_sidecar` ＋ `archive_session_package(...,
   incremental=true, explicit_runs=[本次 run_id])`；
4. 返回 `HeadlessRunArchive { session_id, sidecar_written, archived,
   archive_path, conversation_tokens }`；orz-bin 对归档成功打一行 stderr 指引；
   **best-effort**：内部失败只 warn，绝不影响 run 结局。

### 2.3 与 ACP 车道的三点口径差异（登记为设计边界）

1. **无会话关闭归档**——一次性 run 无 close 语义，只做里程碑增量归档；
2. **侧车仅在归档到期时落盘**——无头侧车的唯一消费者是归档打包（ACP 是每个
   成功 prompt 恒落）；未跨 500K 的常规短 run **零新产物**（无
   `.gsa/conversations/` 文件、无 archives、无 ARC journal）；
3. **三键 journal 键显式注入**——`journal_key_facts` 按 `RUN-{session8}-` 前缀
   扫描，无头 run id（`RUN-CLI-{ts}`）不携带会话段 ⇒ 打包原语链新增
   `explicit_runs: &[String]` 参数（去重合并、排序保键序确定；**ACP 调用点传
   空 `&[]`，行为零变化**）。

### 2.4 载荷边界（零契约变更）

`session_archive` 事件族无 schema 变更（`incremental` 字段既有）、Python 冻结
镜像零同步、工具面零变化。controller `session_id` 在无头车道维持 `None`——
session_end 压缩触发（默认 160K）低于归档里程碑（500K），若注入会话身份会让
压缩 marker 的侧车定位指针指向不存在的文件；**如实缺席优于不实指针**（登记为
有意边界）。

## 3. 实施面（文件与钉子）

- `orz/crates/orz-host/src/acp_server.rs`：`HeadlessRunArchive` 结构 ＋
  `headless_session_archive` 公开入口；`archive_session_package` /
  `package_session_archive` / `build_archive_keys` / `journal_key_facts` 增
  `explicit_runs` 参数（ACP 传空）＋ `archive_session_package` 改回传打包结果
  （`Option<PackagedSessionArchive>`，ACP spawn 路径忽略返回值）。
- `orz/crates/orz-bin/src/main.rs`：`run()` 会话身份派生（`{ts}-cli`）＋
  `session_started_at` 捕获 ＋ 对话携带 ＋ 收尾归档调用与 stderr 指引。
- **钉子 3 例**（orz-host `acp_server::tests`）：
  1. `headless_run_archive_produces_three_key_package_with_explicit_run`——端到端
     三键：侧车＋包＋`archive_keys.journal.runs` **显式纳入 `RUN-CLI-{ts}`**
     （前缀扫描不中，缺注入即三键不齐）＋ ARC journal `incremental:true` ＋ 水位
     落盘 ＋ `session_archive` 事件；
  2. `headless_run_archive_below_threshold_writes_nothing`——阈值下（<500K）
     **零产物**（无 conversations／archives 目录）；
  3. `headless_run_archive_is_idempotent_per_milestone`——同一里程碑第二次收尾
     不重复打包、不重落侧车（单调水位锚点）。

## 4. 读数（本机实跑，2026-09-16）

- orz-loop `--lib`：**821 passed / 0 failed / 3 ignored**（与本批前同值）；
- orz-host `--lib`（`--test-threads=1`）：**328 passed / 0 failed / 5 ignored**
  （本批前 325，+3 钉）；
- orz-assurance `--lib`：**229 passed**；
- `cargo fmt --all -- --check`：干净；clippy（`--workspace --all-targets`）与
  改动前基线**逐项持平**（仅 2 处既有告警，stash 对照实测，零新增）；
- Python assurance 全量：**1652 passed / 2 failed / 14 skipped**——两处失败均
  与本批无因果：`test_search_p3_action_authorization`（既有无关红灯，账本已
  登记，0AJ 审计 §5.4）、`test_doctor_full_repository_check`（批内中间态＝
  审计件未建＋orz 工作树未提交；提交后随门禁复验转绿）。

## 5. 未闭合与遗留

1. **0ak 判据行**（无头长 run ≥500K 实测产出 `.gsa/archives/<session8>.json.gz`
   且三键齐备）：维持未勾，搭下一轮狗粮 run 收取（用户 2026-09-16 指示暂不开始
   新狗粮线）；届时同步收取 0aj 判据（无头 run 内 `blackboard_write` allow→出账
   →读回）、0al 判据（冻结克隆内复验）与 0ah S1 三键齐备率读数。
2. **载体 0.5.4 重建**：判据收取须用含本批的载体；重建待另行排期（与 Linux
   载体、GitHub Release 一并留用户裁决），本批未做。
3. **既有红灯不变**：`runtime/tests test_v02_all_51_event_types_covered`、
   `test_search_p3_action_authorization`（均账本既有登记，与本批无因果）。
