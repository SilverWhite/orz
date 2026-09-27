# 0bv S4 真机复验（切换矩阵＋SERP 链首＋0bw 读数＋0bq 判据；2026-09-27）

> **日期**：2026-09-27；**用户令**：「请进行0bv S4吧」＋中途指引「实在不行找跑分题目做，不用探针」
> （执行即按此收敛：Web 矩阵为产品自身功能走查；真机轮题面改用**真实跑分题目**，不做合成探针题）。
> **载体**：`D:\tb-eval\orz-windows\orz.exe` **0.7.3**（sha256 `c47a5a155085a1ce…`，与 [091 重建档](091_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-27.md) 逐位一致）。
> **形态**：两段——A) `orz web` 切换矩阵真机走查（`--real`＋ACAF fail-closed，UI 实驱）；B) 狗粮轮＝TB2.1 跑分题目 `fix-code-vulnerability` 本地化为题面（launcher 起跑，`ORZ_RETRIEVAL_ENABLED=1`）。
> **边界**：未提交／未推送／未重建；台账（索引/BACKLOG/TODO）本档不动、随落账批同步（沿 D-1 惯例）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 切换矩阵 ② 运行中禁切 | **✅ 真机达成**（前端 `state.running` 主判：运行中点击 → flash 拒绝、零请求发出） |
| 切换矩阵 ① 未信任拒绝 | 403＋fail-closed 文案（但受 F-1 影响＝「恒拒绝」的一半证明，见 §2） |
| 切换矩阵 ③ 旧会话尾不断／④ 新区新会话 | **未达（被 F-1＋F-2 阻断）**；④ 的 spawn 半面（连接时点 cwd spawn）已实证 |
| SERP 链首（0bv①） | **✅ 让渡/兜底语义真机实证**（`browser_serp` ×2 `browser_unavailable` → `local_http` 注记；就绪后承接变体本轮未采样） |
| 0bw 锁死面／L2／CFA | 零违规 ⇒ 零拦截零误拦；L2 v1 零 block/warn 痕迹（与实现一致）；CFA 1123/1124 轮前轮后均零行 ⇒ **⑤ 维持不翻** |
| 0bq 判据 | **✅ 零复现**（出身登记在册；收尾扫净 20 目标＝19 `not_running`＋1 `fingerprint_unknown` fail-closed 拒杀；机面核查零残留） |
| 新发现缺陷 | **F-1（P1 服务端切换信任门 Windows 恒 403）／F-2（P1 前端 `sendJsonBody` 未定义）**——相互独立、串联同链；**已并件 REV-083 余项＝REV-083-21／22**（2026-09-27 用户令；执行随 0bv、闭合随 0bv、不新增计数） |
| 狗粮轮任务 | `completed`；CWE-93 识别正确＋修复与上游 HEAD 逐字节一致＋本地测试 6/6 |

## §1 狗粮轮（RUN-CLI-6ab812c6，跑分题目题面）

- **题面**：TB2.1 `fix-code-vulnerability` 本地化——bottle pinned `0207a34`（clone 后按 Dockerfile 同序删 oracle 行 1562-1563／1567-1568）；官方测试路径本地化随附；边界＝只在 `D:\CLI\.tmp-0bvs4-work\` 内读写、不 commit/push、不动 D:\CLI 跟踪文件。
- **轮读数**：2026-09-27 02:45:26 起跑 ≈5.5 min；journal **337 events**；`run_finished status=completed`、`turn_count=1`、`tool_rounds=23`。工具 45 次＝run_terminal_cmd 21／web_search 3／browser_read 5／grep 4／search_replace 4／read_file 2／blackboard_write 2／browser_control 1／blackboard_read 1／submit 2；permission `allow_once` ×41；ACAF 票据签发/消费 31/31；`orientation_checkpoint` ×1。
- **任务结果**：**CWE-93**（CRLF/HTTP 头注入）识别正确（与 oracle 一致）；`_hkey`/`_hval` 校验修复回**与仓库 HEAD 逐字节一致**（oracle 即删除的上游四行，修复精确复原故 `git diff HEAD` 为空）；`report.jsonl` 落盘；本地化测试 **6/6 passed**（三种 cwd 情形复验）；仓库自带套件 359 passed／8 failed（Windows 固有，与改动无关）。检索旁证＝CVE-2016-9964（bottle redirect CRLF）；无 URL 合成文本被机械面排除、未计入可用额度。
- **边界遵守**：全部读写限于 `.tmp-0bvs4-work`；零 commit/push；D:\CLI 跟踪文件零改动。

### 1.1 SERP 链首读数（0bv① 真机面）

- **时序**：web_search ×3（seq 204/214/216）全部发生于 `browser_launch_result success`（seq 224）**之前**。
- **链首让渡**：检索子代理结果明注「`browser_serp` 两次回退 `local_http`（**cause=browser_unavailable**）」——0bv① 接缝在树且工作：链首先行、未就绪即让渡、失败注记随链序并入（`0bs⑧` 降级序＋0BV 报告 D-f use-if-ready/不自动拉起语义真机实证）。
- **浏览器就绪后**：`browser_control` 搜索链 **google 首个成功**；`browser_read` ×5 读源；检索面候选 5/8、可用证据 5/5。
- **未采样面（如实）**：「浏览器就绪后 web_search 由链首承接交付」的变体本轮未出现（模型先搜后开浏览器的时序使然）；留给后续真机轮顺带采样。

### 1.2 0bw 读数（锁死面／L2／CFA）

- 本轮**零越界尝试**（自然跑分任务不自发攻击系统面）⇒ 锁死面**零拦截、零误拦**（正常工作不受扰＝负向读数成立）；`[写入管控·提示]` 零出现；`write_control`／`exec_policy` 专串零事件。
- **L2 留痕口径核实**：v1 语义下命令面留痕落点为 **block/warn**；零违规 ⇒ 零痕迹，与实现一致。「allow 也留痕」依赖 **0bw③ 专用 journal 事件族（结转未做）**——本面在 0bw③ 落码前结构性不可观测，如实登记。
- **CFA（runbook `.tmp-0bw-cfa-audit.ps1` 免提权面）**：轮前/轮后两次读数相同——`EnableControlledFolderAccess=0 (Disabled)`、事件 **1123/1124 均零行**。⇒ **0bw⑤ CFA enforce 翻转的判据输入仍为空，维持不翻**（[0BV 结转报告 D-3](0BV_CARRYOVER_REV083_0BW_2026-09-27.md) 经真机轮读数再确认；enable 需提权＋用户裁决，runbook 已备）。

### 1.3 0bq 读数（出身登记＋收尾扫净）

- **出身登记在册**：`.gsa/process_trees/call_01_jv2li…-15700.json`＝pid 15700、`parent_chain:[416]`（run 根）、run_id 绑定、started_at、job 归属——登记面真机工作。
- **收尾扫净**：`process_tree_sweep reason=run_shutdown` ×2（run 结束时刻）——20 登记目标＝**19 × `refusal:not_running`**（正常退出）＋**1 × `refusal:fingerprint_unknown`**（pid 15700，指纹/归因三条件不满足 ⇒ **fail-closed 拒杀、如实留痕**，不误杀）。
- **判据达成**：「主进程结束而派生进程仍存活」**零复现**——15700 经机面核查已自行退出（扫净后数秒内），**零孤儿、零残留 orz 进程**。sweep-log 历史另覆盖同机 web 会话等多次 run_shutdown（登记面跨 run 复用正常）。

## §2 切换矩阵真机走查（A 段）与两缺陷

**服务端 API 读数**（curl，token 门全过）：

| 探针 | 结果 |
|---|---|
| switch → 未信任 `D:\tb-eval\.tmp-0bv-untrusted` | **403**「未信任工作区：请先在该工作区启动 orz 并点「信任」（fail-closed）」 |
| switch → 不存在路径 | **400**「工作区路径不存在或不可读取（canonicalize 失败）」 |
| switch → 已信任 `D:\tb-eval\.tmp-0bv-ws-b` | **403（异常，见 F-1）** |
| switch → 当前已信任 `D:\CLI` 自身（判定性） | **403（F-1 实锤）** |
| `?root=D:\CLI`（当前根回看面） | **403「工作区根不可用」（F-1 同病于 resolve_root）** |
| boot／conversations 无参读面 | 正常（200） |

**UI 实驱读数**（`orz web --real`＋ACAF，回环 127.0.0.1:21617）：

- **spawn 半面 ✅**：UI 发消息 → ACP 会话即点 spawn——新会话 `9aae0680` 落 `D:\CLI\.gsa\conversations`；子进程 `orz.exe --stdio`（pid 9936）挂在 web 服务（pid 10684）之下（Win32 进程链核证）；模型回合正常（「连通正常」）。
- **② 运行中禁切 ✅**：45 s 命令运行中（toolbar 停止钮激活＝`state.running`）点击已信任 B 行 → flash「**运行中禁切：等本轮结束（或 Ctrl+Z 取消）后再切换工作区**」，**零请求发出**；本轮结束后同一点击 → 请求路径激活。
- **③／④ 未达**：切换被 F-1＋F-2 双重阻断，「切后旧会话标『旧工作区会话（回看）』」「新 cwd spawn 新会话」无法真机到达（代码面语义维持 S2 测试读数）。

**F-1（P1）服务端：切换信任门／按根解析在 Windows 恒拒绝**——`orz-web/src/server.rs` 的 `api_workspace_switch` 信任门与 `ServerState::resolve_root` 用 `std::fs::canonicalize`（Windows 恒返回 `\\?\` verbatim 前缀形态），而 `path_eq`（server.rs:108-116）只做分隔符/大小写归一、**不剥 verbatim 前缀** ⇒ 与信任库键（非 verbatim 形态）比较恒 false ⇒ 任何目标（含当前已信任工作区自身）一律 403。`resolve_root` 同病 ⇒ `?root=` 按根解析面全线不可用。单测 orz-web 42✓ 未覆盖 canonicalize 真实返回形态——**「符号在位≠接线」族复发**（QUAD-BATCH-DEEP-REVIEW 同族第三例）。
**F-2（P1）前端：切换函数引用未定义符号**——`orz-web/assets/app/api.js:74` `switchWorkspace` 调用 `sendJsonBody(...)`，该符号**全文件未定义亦未导入**（实际只有 `sendJson`，api.js:90）⇒ UI 点击切换抛 `ReferenceError`，被 `switchWorkspaceTo` try/catch 捕获回显「工作区切换失败：sendJsonBody is not defined」⇒ **UI 面切换从未能发出请求**。前端冒烟 13✓（含 switchWorkspace 断言）未覆盖真实 fetch 链路。
**两缺陷相互独立、串联在同一切换链上**（UI 先死于 F-2、API 层再死于 F-1）；修复须同批（path_eq 剥 `\\?\`〔含 `\\?\UNC\`〕前缀＋api.js 补 `sendJsonBody` 或扩展 `sendJson` 收 body），修复批重建载体后**重走矩阵补 ③④ 面**。

## §3 f13/f14/f15 采样情况（如实）

本轮 4 次 search_replace 无歧义场景、无折行剧变、`turn_count=1` 无压缩窗口 ⇒ **f13 候选行号／f14 版式提示／f15 摘要落点骨架均未自然触发**（journal 专串 0 命中）。三件维持 S2 测试读数，S4 采样留后续自然轮或专项轮。

## §4 结论与余项

- **0bv 不闭合**：S4 已跑——①链首让渡/兜底语义实证、②运行中禁切达成、0bq 判据达成；但**切换矩阵 ③④ 被 F-1/F-2 阻断**（修复须进树并重建载体后补验）、f13/f14/f15 未采样、并件 24 项中 REV-083-05/11/12/14/19 与 0bw①–④ 未开工。~~缺陷修复批的归属（并 0bv 后续轮）留用户裁决~~ **〔批注 2026-09-27 用户令「请将本轮遇到的摩擦项并进REV-083 余项中吧」〕已并件 REV-083 余项＝REV-083-21（F-1）／REV-083-22（F-2）**：执行随 0bv、闭合随 0bv、不新增计数 58；BACKLOG `### 0bv.` S4 块与 TODO `P1-0bv` 并件四同批登记；修复同批＋重建载体后重走矩阵补 ③④ 面与链首就绪承接采样。
- **0bq**：S4 判据达成（零复现＋fail-closed 拒杀留痕）⇒ 具备 `partial →` 闭合讨论条件；**翻转与计数（58 → 57）待用户裁决**（与 0bw 同批处理为宜）。
- **0bw⑤**：CFA 判据输入经真机轮读数仍为空 ⇒ **维持不翻**；runbook 已备，enable＋读数批留用户排期。
- **清理留痕**：web 服务与 ACP 子进程零残留；测试信任条目已从 `grok-home/trusted_folders.toml` 移除（删前 223 B → 删后 146 B，余两条目均为既有）；测试目录 `D:\tb-eval\.tmp-0bv-*` 已删；任务环境 `.tmp-0bvs4-work` 与题面 `.tmp-0bvs4-task.txt` 保留（门禁豁免前缀，任务产物可复核）。

## §5 关联与关键词

[`0BV 轮报告`](0BV_SERP_LANE_AND_WORKSPACE_SWITCH_2026-09-26.md) ／
[`0BV 结转报告`](0BV_CARRYOVER_REV083_0BW_2026-09-27.md) ／
[`092 落账档`](092_SUBMIT_PUSH_2026-09-27.md) ／
[`091 重建档`](091_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-27.md) ／
[`0BW 报告`](0BW_WRITE_CONTROL_IMPLEMENTATION_AND_FRICTION_2026-09-26.md) ／
[`写入管控设计`](../WRITE_CONTROL_MECHANICAL_DESIGN_2026-09-26.md) ／
BACKLOG `0bv`／`0bq`／`0bw`

关键词：0bv S4、真机复验、切换矩阵、运行中禁切、browser_serp 链首让渡、local_http 兜底注记、
browser_unavailable、出身登记、process_tree_sweep、fingerprint_unknown、零孤儿、CWE-93、
fix-code-vulnerability 本地化、F-1 verbatim 前缀、F-2 sendJsonBody、符号在位≠接线、CFA 1124 零行、
维持不翻、0bq 待裁闭合。
