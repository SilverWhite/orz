# 250 批 0cy S4 前置机械探针——容器重置 ≡ 会话重置 ≡ 台账重置（2026-10-11）

> **批次**：250（前档＝[`249 补记`](249_RELEASE_V090_READBACK_VERIFICATION_2026-10-11.md)）。
> **日期**：2026-10-11。**用户令**：「为了避免再错失slopcode bench官方要求一次，先走一轮机械探针吧，
> 拉起orz，确定容器重置等价会话重置等价台账重置」。
> **性质**：零源码零改动——真机探针轮（在役载体 0.9.0、scratch 工作区、不动父仓/orz 源与在役载体）。
> **结论先行**：**三环等价成立**——新进程（容器重置等价）在会话起点把非存活会话足迹整卷收进
> `archives/`（会话重置），活区台账随之空起点（台账重置）；包级逐字对账 **9/9 MATCH**；
> `ORZ_GSA_HYGIENE=0` 关断对照成立（残差持续＝机制归因实证）；连续多次重置每轮净室。

---

## §0 背景与目的

SCBench 官方口径＝每 checkpoint 重置上下文（0cw S1 勘定：`agent session data reset between
checkpoints`；226 批 §形态分判核证）。0cw 审计（235 批）实锤旧载体在三因上违反该口径——
`ledger/current.md` 跨会话滚动＋上一轮足迹留活区＋0ct 读全开。0cy（S1 设计 D1–D8＋S2 落码
N1–N8＝246 批；S3 载体进体 0.9.0＝247 批；248 批发行）以「启动侧扫描收卷」闭合。本批在 SCB
官方轮重启之前，用机械探针在真机上验证 0cy 语义实际成立，避免再错失官方要求一次。

## §1 探针协议

- **载体**：`D:\tb-eval\orz-windows\orz.exe`，SHA256 `0c07208d121af341…`（与 248 批在役载体真值
  逐位一致）；`--build-info`＝`version=0.9.0 os=windows arch=x86_64 profile=release`（rc 0）。
- **驱动**：`orz -p "<prompt>" --max-wallclock 120` headless＋`ORZ_FAKE_SCENARIO` 确定性脚本
  provider（真工具执行：`read_file probe.txt` 一轮＋终答；TER T2.3 面真机口）。
- **工作区**：`D:\tb-eval\0cy-probe-ws`（净室 scratch；仅探针件 `probe.txt`）。
- **混合足迹协议（为何播种）**：台账行只在 v8 分块压缩时外挂落盘、无头收尾归档有里程碑 token
  门槛（`incremental_archive_due`，0ak 边界）——短会话两者都不触发，纯真机薄足迹验证不了
  「台账重置」。故 Run 1 真机造薄足迹（run journal）后，按 0cw 污染形态**受控播种**假想上一会话
  `deadbeef` 残差（台账／定向／激活／grill／压缩摘要／回放块／共享终端日志＋`RUN-deadbeef-0`
  journal＋`ARC-deadbeef-1` 豁免对照），逐件留 SHA256 真值，再由 Run 2 真机进程收卷对账。

## §2 读数（五轮）

| 轮 | 会话 s8 | journal 窗口（UTC） | sweep 行（stderr） | 结果 |
|---|---|---|---|---|
| Run 1 | `6acae91d` | 01:40:45→01:40:46（25 事件） | 无活区可扫 | 薄足迹＝journal＋机器面；收尾归档零产物（里程碑门，0ak 设计内） |
| Run 2 | `6acaea1f` | 01:45:03→01:45:04 | `swept ["6acae91d","deadbeef"]; shared→Some("deadbeef")` | 双会话整卷收进 `archives/`；活区残差全清 |
| Run 3 | `6acaeabe` | 01:47:42→01:47:43 | `swept ["6acaea1f"]; shared→…` | 连续重置每轮净室（SCB 逐题形态） |
| Run 4 | `6acaeac7` | 01:47:51→01:47:52 | **无**（`ORZ_GSA_HYGIENE=0`） | 关断对照：Run 3 journal `6acaeabe` **在活区持续存在**＝0cw 污染形态复现 |
| Run 5 | `6acaead5` | 01:48:05→ | `swept ["6acaeabe","6acaeac7"]; shared→…` | 重开扫清掉对照轮残差 |

**Run 2 后活区终态**（Run 3/5 同构）：`archives/` 两包＋keystore×2＋`resources_state.json`
（机器面，豁免）＋`ARC-*` 回执（豁免，且 sweep 自身对每次收卷写 ARC 审计 run）＋当前会话
journal——**仅此五类**；播种的 9 件残差与 Run 1 journal 全部从活区消失；**`.gsa/ledger/` 空**。

## §3 包级逐字对账（9/9 MATCH）

两包 schema 均 `session-archive-package-v0.3`；`footprint` 成员与 Run 2 前快照逐字节比对：

| 包 | 成员 | 判定 |
|---|---|---|
| `deadbeef.json.gz` | ledger.text（共享·winner=deadbeef）／orientation／activation／grill／compaction／blocks／terminal（共享）／RUN-deadbeef-0 events | **8/8 MATCH**；counts runs=1/compaction=1/blocks=1/grill=1/terminal=1/bytes=585 |
| `6acae91d.json.gz` | RUN-CLI-6acae91d events.jsonl（真机 Run 1 journal） | **MATCH**（22,935 B）；conversation 成员经 **journal 重构**在场（`reconstructed_from_journal=true`，5 messages——侧车缺席路径真机首证）；ledger 成员 null（该会话无台账） |

共享件归属＝mtime winner（`shared→Some("deadbeef")`），与设计 D8 一致；`ARC-deadbeef-1`
（播种豁免对照）原样存活。

## §4 判定：三环等价逐条

1. **容器重置 ≡ 会话重置**：同卷起新进程（Run 2/3/5）在会话起点扫描非存活∧非当前足迹并整卷
   收卷——进程边界即会话边界，两代真机 journal＋播种残差同轮扫收，无遗漏无残留。
2. **会话重置 ≡ 台账重置**：播种台账逐字节进包后从活区删除（`.gsa/ledger/` 空）——新会话台账
   空起点成立；若新会话足够长产生压缩行，台账将由其自身行从头累积（旧行只在包内）。
3. **机制归因**：`ORZ_GSA_HYGIENE=0` 下残差持续（Run 4）、缺省开下净室（Run 2/3/5）——收卷
   确由 0cy 启动侧扫描产生，非环境巧合。

**0cy S4 判据对照**（设计稿 §9）：②活区无前档足迹 ✅；③archives 在案（含足迹成员）✅；
①新会话空板＋「提交/推送/重建后不回归」属 harbor 多步重置联动轮，仍待跑（本批为其前置）。

## §5 边界与如实记

1. **误起 TUI 一次**（勘察期 `orz --help` 进入 TUI，cwd＝D:\CLI，约 1 min 后终止）：TUI 空闲态
   不建会话、未触发 sweep，D:\CLI 活区零改动（台账 mtime 08:31 未动）；已 `taskkill` 确认无残留
   orz.exe。D:\CLI 活区现存 `conversations/6acad03d.json`＋2.8 MB 旧台账（08:31，0.9.0 载体
   安装前产物）＝0cy 语义下的待收卷对象，留联动轮处置，本批不动。
2. **播种残差为受控 fixture**：非真机长会话产物（真机台账需压缩触发）；台账「跨会话滚动」的
   真机形态复验留 0cy/0cz/0da S4 联动轮。
3. **探针工具误差一件**：包级对账脚本首跑报 9 项假 MISMATCH（快照哈希值尾部空格未 strip），
   修正后复跑 9/9 MATCH——脚本误差非载体误差，如实记。
4. **每轮 journal 恒 25 事件**（确定性 provider；含 counterexample_gate／orientation_checkpoint/
   mechanical_audit_update 各 1）——面完整，供联动轮对照。
5. **零提交零推送**（沿 246 先例「报告即留痕」）；scratch 工作区探针产物（5 包＋ARC 回执）留存
   `D:\tb-eval\0cy-probe-ws` 供联动轮复核；探针资产（场景脚本＋快照＋stdout/stderr）留存
   `D:\tb-eval\0cy-probe-assets`。
6. 未闭合计数不变 **59**（探针不闭合任何项；0cy S4 仍开放）。

## §6 SCB 前瞻

官方轮每 checkpoint 新进程 ⇒ 每题起点净室（本批 Run 2/3 连续重置形态已实证）；`ORZ_GSA_HYGIENE=0`
为排障/对照逃生舱（Run 4 实证有效）；净室重跑如需归档移出工作区用 `ORZ_GSA_ARCHIVE_ROOT`（设计
D6，本批未触发，缺省路径收卷形态已覆盖）。

## §7 关联与关键词

[`0cy 设计稿`](../GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md)／
[`246 批档`](246_0CY_S2_GSA_SESSION_HYGIENE_2026-10-11.md)（S1/S2＋D1–D8＋N1–N8）／
[`247 批档`](247_CARRIER_REBUILD_V090_0CY_S3_2026-10-11.md)（0.9.0 载体）／
[`248 批档`](248_SUBMIT_PUSH_AND_RELEASE_V090_2026-10-11.md)（发行）／
[`235 批档`](235_0CW_CONTAMINATION_AUDIT_0CY_GSA_HYGIENE_2026-10-11.md)（三因审计）／
[`226 批档`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md)（官方重置口径勘定）。

关键词：250 批、0cy S4 前置、机械探针、容器重置等价会话重置等价台账重置、启动侧扫描收卷、
收卷对账 9/9 MATCH、journal 重构真机首证、ORZ_GSA_HYGIENE=0 关断对照、SCBench 官方重置口径、
计数 59 不变、索引 v4.222。
