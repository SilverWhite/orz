# 254 批 0dc S1＋S2——/tmp 指令即焚闭环＋逐档 /tmp 面断言（2026-10-12）

> **批次**：254（前档＝[`253 批档`](253_R3_FULL_ROUND_STOP_TMP_LEAK_R4_PROJECT_2026-10-12.md)）。
> **日期**：2026-10-12。**用户令**：「请直接开始0dc S1+S2吧，把这个问题处理完」。
> **性质**：评测侧本地件落码批（tb-eval 三件：driver／adapter／runner）＋冒烟批；零 orz 源码、
> 零父仓源码、计数不变 61。
> **结论先行**：253 批发现的 /tmp 指令泄漏通道（适配器每档上传 `/tmp/orz-instruction-{N}.md`、
> driver 读取后从不清除、r3 14/14 题模型可读前档指令）已**双侧闭环**——S1＝driver 三模式读后
> 即焚（step 焚于读取后、client 焚于读取后、daemon 退出清 workdir）＋适配器上传前 `rm -f` 清旧
> （双保险，覆盖 driver 死于读取前的残档场景）＋/tmp 写出面自查（每档起点/终点 `/tmp/orz*`
> 双点快照入 `step_result.json`）；S2＝runner 重置核收扩展，0cy 收卷行断言之外增 /tmp 面断言
> （档起点恰为当档一个指令文件＋档终点焚毁空＋burned=true；`reset_conformance` 原口径①②③
> **不动**＝r3 台账可复现，/tmp 断言为并列字段）。冒烟 22/22 全过＋真实 r3 首题（cfgpipe，
> 6 档）回放 `reset_conformance=True` 不变。**r4 S3 起跑前置（S1）就此解除**；S3 全目重跑与
> 成本门、载体真值届时另裁。

---

## §1 泄漏链路与修复面（253 批 §3 的落地）

- 原链路：适配器（`orz_multistep.py` `run()`）每 checkpoint 上传题面指令至
  `/tmp/orz-instruction-{N}.md`（「长指令走文件不走 argv」）→ driver（`orz_acp_driver.py`）
  step 模式经 `--instruction-file` 读入作 ACP prompt → **从不删除**；容器/工作区跨档持久
  ⇒ 前档指令逐档堆积可读（r3 p04 实证 `cat /tmp/orz-instruction-1.md
  /tmp/orz-instruction-2.md`；全扫 14/14 题、150 处读取事件）。
- 修复面（双侧三处＋观测件）：

  | 位置 | 改动 | 覆盖 |
  |---|---|---|
  | driver `mode_step` | 读取后立即 `os.unlink`（焚于 orz 起跑前） | 官方重置形态主修复 |
  | driver `mode_client` | 读取后立即焚毁 | 连续形态（0cr 对照） |
  | adapter `run()` | 上传前 `rm -f /tmp/orz-instruction-*.md`（exec_as_agent） | 双保险：前档 driver 死于读取前的残档 |
  | driver `mode_daemon` | 退出时 `rmtree` workdir `/tmp/orz-acpd-<hash>`（socket 原已 unlink） | 连续形态收尾零残留 |

- 设计内不动：`/tmp/orz-home`（ACAF 机器面配置，253 批 §3 四角色表裁「跨档持久＝设计内」）
  ——/tmp 面快照如实记录它，但断言面只针对指令文件。

## §2 S1 写出面自查（/tmp 面双点快照）

- driver 新增三 helper：`tmp_surface(base_dir="/tmp")`（`orz*` 前缀 basename 有序表；
  OSError→[]）／`tmp_instruction_names`（严格 `orz-instruction-<N>.md` 数字形态——
  排除非数字名）／`burn_instruction_file`（unlink；失败 log 不阻断——指令已读入内存，
  后续流程不受影响）。
- step 模式 payload 新增：`instruction_file`（当档文件名）、`instruction_file_burned`、
  `tmp_orz_surface_start`／`tmp_orz_surface_end`（全 orz* 面）＋派生
  `tmp_instruction_start`／`tmp_instruction_end`。终点面在 `finally` 采集——killed／error
  路径同样落面，step_result.json 成为每档 /tmp 面的完整证物。
- 顺手修（冒烟暴露的既有粗糙边）：`count_rounds(None)` 无 None 守卫——orz 未产出 journal
  时 finally 内 TypeError 会掩盖原错误码；加 `if not journal_path: return 0` 一行
  （`copy_journal` 本有同形守卫，对齐）。

## §3 S2 逐档 /tmp 面断言（runner 重置核收扩展）

- `collect_reset_evidence` 每 checkpoint 读 `step_result.json` 新字段，行集新增四列：
  `tmp_instruction_start`／`tmp_instruction_end`／`instruction_file_burned`／
  `tmp_only_current`（＝起点名单恰 `[orz-instruction-<N>.md]`，N＝档序 1-based）。
- summary 新增两个并列字段：`tmp_only_current_all`／`tmp_burned_all`（burned=true 且
  终点名单空）。**`reset_conformance` 口径不动**（session 全新 ∪ journal 全新 ∪ 0cy 收卷行）
  ——r3 台账可复现；/tmp 断言不牵连原口径（负例实测：残留文件 ⇒ tmp 两字段 False 而
  conformance 仍 True）。
- r3 旧数据（无新字段）如实落 None／False，不抛；起跑横幅与题级收官日志同步补 /tmp 面
  读数（`/tmp面=tmp_only_current_all/tmp_burned_all`）。
- r4 S3 沿用建议（留 S3 落地时裁决）：起跑可要求 `reset_conformance ∧ tmp_only_current_all
  ∧ tmp_burned_all` 全真，checkpoint_1 即验、失败即硬停（此时全目未铺开、沉没成本最小）。

## §4 冒烟与真实数据回放（22/22 全过）

- 冒烟件＝`_smoke_r4s1_burn.py`（评测侧本地件，不入仓）三段：
  - **A 单元（6）**：面扫描收 `orz*` 有序／严格数字形态名单（排除
    `orz-instruction-readme.md`）／即焚成功与幂等失败（False 不抛）／缺目录 []。
  - **B stub e2e（9）**：假 ACP server 子进程（initialize／session-new／prompt→end_turn）
    走全 `mode_step`——exit 0＋指令文件焚毁＋payload 新字段在场＋`tmp_instruction_end=[]`＋
    journal 捕获 rounds=1（夹具 journal 使 find_journal／copy_journal／count_rounds 全链路
    被测）。
  - **C 假 job 目录 runner 断言（7）**：正例两档恰等当档名＋0cy 收卷行断言仍过；负例
    （起点残留前一档＋未焚毁）tmp 两字段 False；r3 旧形态字段 None 如实；台账 4 行落
    临时文件。
- **真实数据回放**：`collect_reset_evidence('r3-p01-cfgpipe')`（台账重定向临时文件、真实
  `jobs-r3/r3_reset_ledger.jsonl` 零触碰）＝n=6、`reset_conformance=True`（与 253 批核收
  一致）、/tmp 字段 None／False 如实——runner 改动对既有真实数据零回退。
- 适配器对 harbor venv 真实导入过（`import scb_agents.orz_multistep`，harbor API 兼容）；
  三文件 `py_compile` 过。

## §5 边界与登记

- 「每档起点」的操作化＝**driver 启动即扫描**（此时适配器已上传当档、driver 未焚）——该
  窗口由构造保证恰一档文件（适配器先清旧再传当档）；档中与档后为零（焚毁先于 orz 起跑）。
- 连续形态（0cr 对照形态）：client 焚毁后 daemon 收到的指令已在会话内（同一会话可回读自身
  prompt 面属形态内语义）；daemon workdir 会话内持久（wire/stderr 自用）、daemon 退出即清
  ——253 批 §3 四角色表定位不变。
- 本批零 orz 源码：修复全落评测侧本地件（tb-eval，不入父仓不入 orz 仓），在役载体 0.9.0
  无涉、无需重建；`step_result.json` 为评测侧自有件、无 schema 依赖方。
- r4 余下：**S3** 全目 36 题重跑（`run_r3_scb.py` 形态复用；identity gate 需更新为届时在役
  版三哈希；成本门／起跑窗届时另裁）＋**S4** V4 轮报告（r4/r3/0cw/0cr 四方聚合对照＋逐题
  四分类归因＋论文同向/反向终判；r3 的 /tmp 泄漏偏差〔ck2+ 可能轻度系统性偏高〕显式登记）。

## §6 关联与关键词

[`253 批档`](253_R3_FULL_ROUND_STOP_TMP_LEAK_R4_PROJECT_2026-10-12.md)（发现与 r4 立项）／
[`250 批档`](250_0CY_S4_PROBE_RESET_EQUIVALENCE_2026-10-11.md)（重置等价探针）／
[`251 批档`](251_R3_SCB_P01_CFGPIPE_OFFICIAL_RESET_2026-10-11.md)（r3 首题）／
[`235 批档`](235_0CW_CONTAMINATION_AUDIT_0CY_GSA_HYGIENE_2026-10-11.md)（0cw 污染审计）／
[`0cy 设计稿`](../GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md)。

关键词：254 批、0dc S1、0dc S2、/tmp 指令即焚、burn-after-read、写入面纪律落地、
适配器清旧、/tmp 面断言、tmp_only_current、双点快照、冒烟 22/22、r3 回放不回退、
计数不变 61、索引 v4.226。
