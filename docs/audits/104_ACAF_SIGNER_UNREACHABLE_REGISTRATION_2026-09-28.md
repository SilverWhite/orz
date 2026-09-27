# 104 ACAF 签名器不可达（桌面形态）正式立项（0by；2026-09-28）

> **日期**：2026-09-28；**用户裁决**：「这一部分值得正式立项，毕竟这个问题并没有进行修正，只是新版本中没有这一故障表现了而已」。
> **本批＝观察项升级为正式开放项**：`OBS-ACAF-SIGNER-UNREACHABLE-DESKTOP-080`（102 批仅记录、不占计数）⇒ **`0by` / `GAP-ACAF-SIGNER-UNREACHABLE`**（P1，`pending`）；**计数 54 → 55**。
> **形态**：立项登记批 ＋ 一处秒级排除探针；**零代码改动**（orz 子模块未动，pin `12dbaf90`）；**未提交／未推送**。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 现象 | **桌面形态**下，凡需控制票据的宿主工具（`browser_read`／`web_fetch`）被 `control_ticket_rejected: signer_unreachable` 拒绝，detail＝`io error: 管道正在被关闭。 (os error 232)` |
| 读数 | **0.8.0 桌面两轮＝27 条／23 条拒绝**（票据类工具面全灭）；**同条件 0.8.1 桌面轮＝0 条拒绝**（18 张票据全部 issued→consumed） |
| 排除实验（本批） | **签名器件与密钥库均排除**——三配置（长期库／新建库／新建库＋0.8.1 签名器）对 `initialize_session` **全部正常应答**（§2） |
| 形态边界 | **容器形态两载体（0.8.0／0.8.1）均 0 拒绝**；差异只出现在**桌面形态** |
| 定性（用户裁决） | **根因未明；「新版本中不再现」不等于「已修正」**——按未修问题立项 |
| 判定 | 立项 **`0by`（P1，`pending`）**；**计数 54 → 55** |

## §1 现象与原始读数

### 1.1 两会话读数（桌面形态，hand-off 条件预置，同 launcher／同题面）

| 轮 | 载体 | 事件 | `control_ticket_rejected` | 被拒工具 | 成功面 | 启动事实 / 浏览器调用 |
|---|---|---:|---:|---|---|---|
| `RUN-CLI-6ab93831` | 0.8.0 | 451 | **27** | `browser_read` 14＋`web_fetch` 3＋其余 | `browser_control` 32/32、`web_search` 9/9、`grep` 17/20、`read_file` 4/4 | 32 / 32 |
| `RUN-CLI-6ab93aaa` | 0.8.0（ACAF 件换 0.8.1 同源后复跑） | 429 | **23** | `browser_read` 11＋`web_fetch` 3＋其余 | `browser_control` 39/39、`web_search` 8/8、`blackboard_read` 4/4 | 39 / 39 |
| `RUN-CLI-6ab9363d` | **0.8.1** | 214 | **0** | —（`browser_read` 9/9 成功） | 全绿 | 1 / 10 |

### 1.2 拒绝的形状（journal 原文节选）

```json
{"tool":"browser_read","exit_code":1,"status":"error",
 "error":"control_ticket_rejected:signer_unreachable",
 "policy_denial":{"source":"acaf","code":"control_ticket_rejected:signer_unreachable",
                  "reason":"io error: 管道正在被关闭。 (os error 232)"}}
```

### 1.3 与 0bx（就绪判定）的关系＝**正交**

- 拒绝只落在**票据类工具**上，**从不产生 `browser_launch_result`**；而重复启动与 `browser_control` 调用 1:1（32/32、39/39），`browser_control` 本身全成功 ⇒ 102 批的 0bx 判别式不受本现象干扰（该结论已在 [`102 档 §4.3`](102_BROWSER_READY_HANDOFF_S4_CLOSE_2026-09-27.md) 记录）。
- 容器形态两载体均零拒绝（0.8.0 容器轮 ACAF 103/103 零拒，见 [`098 档`](098_0AC_S4_CLOSURE_2026-09-27.md)；0.8.1 容器轮 0 拒绝）⇒ 本现象**不是**通用环境面，而与**桌面形态**相关。

## §2 本批排除实验（秒级探针，直接驱动签名器）

执行器＝[`.tmp-b104-acaf-probe.py`](../../.tmp-b104-acaf-probe.py)（门禁豁免留档）：按**客户端同一用法**（`ORZ_SIGNER_MANIFEST` ＋ `ORZ_SIGNER_KEYSTORE_ROOT` ＋ stdio 管道）拉起签名器，投一条 `initialize_session` 请求并读回应。

| 配置 | 密钥库 | 签名器 | 结果 |
|---|---|---|---|
| `D:\tb-eval\orz-windows` | 长期库（在役） | 0.8.1 | **exit 0 ＋ 应答**（`session_key_hex`／`signer_measurement=62d91d47…`） |
| `D:\tb-eval\orz-windows-080` | **新建库**（0.8.0 provision） | 0.8.0 | **exit 0 ＋ 应答**（`signer_measurement=54bb982f…`） |
| `D:\tb-eval\orz-windows-080c` | **新建库**（0.8.1 provision） | 0.8.1 | **exit 0 ＋ 应答**（`signer_measurement=62d91d47…`） |

**推论**：

1. **不是签名器二进制、不是清单绑定、不是「新建密钥库不可用」**——三配置（含两处 fresh provision）全部正常应答。
2. **不是通用环境面**——容器形态两载体均零拒绝。
3. ⇒ 差异落在**桌面形态的 orz ↔ 签名器交互**上；且 **0.8.1 相对 0.8.0 只含 0bx 的 `cdp.rs` 改动＋版本 bump**（见 [`101 档 §1`](101_SUBMIT_PUSH_AND_RELEASE_2026-09-27.md)），**没有任何已知代码变更解释这一差异** ⇒ 属「同环境下的不可解释差异」，必须由 S1 定位而非按「新版没复现」结案（本批立项的直接理由）。

## §3 立项条目（登记面）

| 字段 | 值 |
|---|---|
| 稳定 ID | **`0by`**（序号；前项 `0bx` 已于 102 批闭合） |
| GAP ID | **`GAP-ACAF-SIGNER-UNREACHABLE`** |
| 优先级 / 状态 | **P1 / `pending`**（无代码改动、根因未定） |
| 计数 | **54 → 55** |
| 来源 | 102 批观察项 `OBS-ACAF-SIGNER-UNREACHABLE-DESKTOP-080` 经用户裁决升级（观察项记号随之撤） |
| 入口 | TODO `P1-0by` / 本档 / [`102 档 §4`](102_BROWSER_READY_HANDOFF_S4_CLOSE_2026-09-27.md) / BACKLOG `0by` |

## §4 勘定与闭合判据（S1–S4）

- **S1 勘定（根因定位，须先红）**：① 最小复现——桌面单工具最小题面（不叠加 hand-off 条件／不叠加检索题），确认「票据类工具必被拒」的可复现性；② 变量隔离矩阵——载体（0.8.0／0.8.1）× ACAF 件（长期库／新建库）× launcher 面（直起 orz／经 `.tmp` Tee 管道）× 负载（空载／并发）四轴逐格；③ **补观测缺口**——客户端 spawn 时签名器 stderr 被 `Stdio::null()` 丢弃（[`acaf.rs`](../../orz/crates/orz-loop/src/acaf.rs) `spawn_child`），失败原因当前**不可见**；S1 先补一条可开闭的签名器 stderr 旁路（或等价的 journal 面），把「管道被关闭」的真实原因拿到手（签名器自退／被收／握手超时／句柄泄漏四条候选），再谈修复。
- **S2 落码＋钉子**：按 S1 结论修，落**先红后绿**钉（红＝修复前复现拒绝；绿＝修复后票据 issued→consumed）。
- **S3 载体**：修复随下一载体重建进体（双平台）并进包。
- **S4 真机**：**桌面形态零拒绝**（票据类工具全通过）＋容器形态维持零拒绝（回归）。
- **闭合判据**：S1 根因成立（有钉）＋S2 修复（先红后绿）＋S3 载体 ＋S4 双形态真机读数 ⇒ `pending` → `implemented`，计数 55 → 54。

## §5 边界（如实记）

1. **0.8.1 未复现 ≠ 已修**：本现象在 0.8.1 桌面轮与两轮容器读数中均未出现，但**上游无对应修复**（见 §2 推论 3）；按未修问题立项。
2. **现象集中在桌面形态**：容器形态两载体均零拒绝；桌面形态的 0.8.0 两轮稳定复现、0.8.1 单轮未复现（**样本量＝1**，S1 需补轮次）。
3. **影响面**：被拒工具含 `browser_read`／`web_fetch`（即宿主侧读取与抓取面），fail-closed 语义下等价于**该形态下检索读取链被整体截断**；未观察到票据面以外的连带影响。
4. **本批未改码**：立项登记与排除探针之外无改动；不触载体、不发行。

## §6 台账同步

- BACKLOG：新增 `### 0by.` 节＋计数行 **54 → 55**＋P1 开放项清单（`/ 0by`）＋优先级总览表（`ACAF 签名器不可达（0by）`）；本批记录指针 → 第二卷 §1.56。
- TODO：计数行 55＋P1 路由行＋新增 `### P1-0by` 勾选块（S1／S2／S3／S4／闭合五枚）。
- 索引：§6 新条目 `GAP-ACAF-SIGNER-UNREACHABLE`（`pending`）＋§8 `pending` 桶；头行 v4.71 → **v4.72**（v4.71 滚入 [`存档卷`](../../存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md)，121 → **122 行**）。
- [`102 档`](102_BROWSER_READY_HANDOFF_S4_CLOSE_2026-09-27.md)：补 §7 勘误（观察项升级为 `0by`）。
- 第二卷：§1.56。台账改写脚本＝[`.tmp-b104-ledger.py`](../../.tmp-b104-ledger.py)（门禁豁免）。
- **门禁**：`python scripts/check_repository.py` ⇒ 目标 `valid: true`。

## §7 关联与关键词

[`102 闭合档`](102_BROWSER_READY_HANDOFF_S4_CLOSE_2026-09-27.md)（§4 观察项原文＋§7 勘误）／[`099 就绪判定档`](099_BROWSER_READY_HANDOFF_2026-09-27.md)／[`098 S4 收口档`](098_0AC_S4_CLOSURE_2026-09-27.md)（容器 ACAF 103/103 零拒）／[`101 发行档`](101_SUBMIT_PUSH_AND_RELEASE_2026-09-27.md)（0.8.1 变更面）／[`acaf.rs`](../../orz/crates/orz-loop/src/acaf.rs)／[`orz-signer.rs`](../../orz/crates/orz-bin/src/bin/orz-signer.rs)／BACKLOG `0by`／TODO `P1-0by`。

关键词：ACAF、签名器不可达、`signer_unreachable`、`control_ticket_rejected`、管道被关闭 os error 232、fail-closed 拒绝、票据类工具全灭、桌面形态、容器零拒绝、新建密钥库排除、秒级探针、根因未明、新版未复现不等于已修、0by 立项、计数 55。
