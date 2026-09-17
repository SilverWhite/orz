# 摩擦处理狗粮 run 报告（`RUN-CLI-6aabf5eb`；进程 panic 中止）

> 日期：2026-09-17；run＝**`RUN-CLI-6aabf5eb`**（本地 **22:15:08 → 22:26:35**，历时 **11m27s**，721 事件、74 模型轮、90 工具调用、`run_finished` 未发出——进程 panic 中止）。
> 载体＝**orz 0.6.0** Windows 三件套（`D:\tb-eval\orz-windows\orz.exe`，SHA256 `a642b8d1…`，与 0.6.0 重建记录一致）；ACAF 以 `orz-acaf-provision.exe` 在册签哈希（manifest `ade2db9a…` ↔ signer 实测 `ade2db9a…` 相合）。
> 题面＝用户 2026-09-17 指定原文逐字（`.tmp-friction-task.txt`）：先查 `CLI_PROJECT_INDEX.md` 路由 → 回查文档 → 处理 [`FRICTION_INVENTORY_2026-09-17`](FRICTION_INVENTORY_2026-09-17.md) 盘点的摩擦项 → 按项目惯例落报告文档 → 期间新摩擦一并登记。
> 启动口径：cwd＝`D:\CLI`（显式 `-WorkingDirectory`，吸取 M-1 误启动教训）、`ORZ_MAX_WALLCLOCK=0`（**无墙钟**）、`--real --allow-write --allow-shell --allow-network`。
> 性质：主会话报告（agent 自身报告随崩溃丢失）。**本批未提交、未推送**；orz 工作树只含运行前既有的两批未提交改动（0am 批／FR-C04 批），本 run 未产生新的代码改动。

---

## 0. 结论速览

| 项 | 结果 |
|---|---|
| run 是否收口 | **否**——`RUN-CLI-6aabf5eb` 在 t+11m27s 因**进程 panic** 中止，无 `run_finished`、无 agent 报告文档 |
| 崩因 | `orz-tools` 路径包含性判定对**含多字节字符的路径**做字节切片 ⇒ `end byte index 11 is not a char boundary`（Windows 分支） |
| 触发动作 | `read_file("存档/docs/README.md")`（模型在按 FR-A08「TER 断链 ×7」回查存档链接） |
| 摩擦处理进度 | 起跑健康、开局规划与台账写入正常（`plan_write` ×2）；**实际处置未完成**——agent 已扫描全仓断链并开始回查存档，第一项尚未落码即崩 |
| 新摩擦 | **1 条**（见 §3，P0 级进程崩溃，非登记在案项） |
| 上下文／token | 全 run 真实上传 **7,123,278**（hit 6,857,600 ／ miss 265,678；命中率 90.1%）；模型面估算峰值 **224,324**（2 分块）——**未跨 500K** |

---

## 1. 运行配置与启动健康

- **ACAF**：0.6.0 signer 哈希与 manifest 在册值相合，无需重 provision；`run_preflight` 落盘。
- **启动健康（前 2 分钟）**：`prompt_submitted` → `orientation_checkpoint` ×2 → `tool_availability_check` ×2 → `plan_write` ×2，权限决策全 `allow_once`（90 请求 90 决策），`host_resource_snapshot` tier=`watch`（`trigger=run_start`，卷余量 23.3 GiB／commit 余量 5.6 GiB）。**0aj 链路（`blackboard_write`→`plan_write`）再次实证可通。**
- **agent 开局规划**（`plan_write` 载荷）：goal＝「处理 `FRICTION_INVENTORY_2026-09-17` 摩擦项 + 落报告」（plan 段 1251 字符）＋「中间结论」台账（notes 段 1247 字符）。
- **能力面读数**：`main_agent_work_tools` 探针 `pass`（read_file／grep／search_replace／blackboard_read／blackboard_write／run_terminal_cmd 全在）；`retrieval_family` **整体 disabled**——本 run 未开检索车道（`retrieval_enabled=false`），与题面（处理登记摩擦项，含回查文档）匹配，非缺陷。
- **首条 WARN**：t+1m57s `grep timed out timeout_secs=20`（全仓断链扫描类命令），未阻断。

## 2. 崩溃现场（一手证据）

**stderr（`.tmp-friction-dogfood-run.err.log` 尾）**：

```text
2026-09-17T14:17:06.081780Z  WARN orz_tools::implementations::grok_build::grep: grep timed out timeout_secs=20

thread 'main' (4716) panicked at crates\codegen\orz-tools\src\types\resources.rs:509:40:
end byte index 11 is not a char boundary; it is inside '档' (bytes 10..13 of string)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

**journal 末段（`D:\CLI\.gsa\runs\RUN-CLI-6aabf5eb\events.jsonl`，721 行）**：

| seq | 事件 | 内容 |
|---|---|---|
| 715–716 | `mechanical_audit_update` | `budget`（已用 73/0 轮）＋`context_scale:224k`（估算 224,324、2 分块、tier=soft） |
| 717 | `model_output` | `read_file{target_file:"存档/docs/README.md"}`；`cache_hit 161,024`／`miss 4,938` |
| 718–719 | `permission_requested` → `permission_decision` | risk=ReadOnly → **`allow_once`**（放行无误） |
| 720 | `tool_started` | `read_file` 开始执行 |
| — | — | **进程 panic，事件流在此断**（无 `tool_completed`、无 `run_finished`） |

**崩点代码**（`orz/crates/codegen/orz-tools/src/types/resources.rs:500-522`，Windows 分支）：

```rust
fn candidate_is_under_raw(base: &std::path::Path, candidate: &std::path::Path) -> bool {
    if candidate.starts_with(base) { return true; }
    #[cfg(windows)]
    {
        let candidate_str = candidate.to_string_lossy();
        let base_str = base.to_string_lossy();
        if candidate_str.len() >= base_str.len() {
            let prefix = &candidate_str[..base_str.len()];   // ← :509 panic（按字节切 UTF-8）
            if prefix.eq_ignore_ascii_case(&base_str) { … }
```

**根因**：`base_str.len()` 是**字节长度**，与 `Path::starts_with` 的大小写不敏感兜底比较本身是字符级比较；当 base／candidate 含多字节字符（本例 `存档`，base 长度 11 字节）时，按字节边界切片落在字符内部 ⇒ panic。**同一函数族 2026-09-12 已出过同形故障**（`decode_html_entities` 的 12 字节切片，0ac 修复批记「去 12 字节切片→整体 find」）；本处为该族**新的存活实例**，且触发面更宽（任何含中文/多字节路径的包含性判定）。

**可达面**：`candidate_is_under`（:526）把 `candidate_is_under_raw` 同时用于 canonical 与词法归一化两路，调用方含 `is_within_registered_skill_roots`（:560）与工作区沙箱包含性判定（:571 起）。本 run 的触发路径为 **read_file 对 `存档/docs/…` 的工作区包含性判定**——即「读工作区内任何含中文名的目录」都能撞上，非仅技能面。

**影响面**：进程级硬崩（exit code 101），一次性 run 无收尾、无 `run_finished`、journal 断链、模型侧产物与报告全部丢失；在无墙钟长 run 下等同于「任意时刻可能静默丢全场」。

## 3. 新摩擦登记（本 run 发现，未在盘点档内）

**FR-N01（P0，进程崩溃｜沙箱/工具面）**：`candidate_is_under_raw` 在 Windows 分支按字节切多字节路径名 ⇒ 读取含中文目录（如 `存档/…`）时 `panic: end byte index N is not a char boundary`，进程 `abort` 级中止、run 无收尾。

- 证据：上表 seq 717–720 ＋ stderr panic 原文 ＋ 崩点 `resources.rs:509:40`。
- 复发形态：与 2026-09-12 `decode_html_entities` 12 字节切片同族（字节切片 vs UTF-8 字符边界）。
- 建议修法（最小面，待裁决）：`candidate_is_under_raw` 改用不 panic 的字符边界安全比较——`candidate_str.get(..base_str.len())`（`None` 即判否）或以 `as_bytes()` 做 ASCII 大小写折叠比较＋`/`、`\` 分隔符判定；补钉子：含多字节 base/candidate 的包含性判定不 panic 且语义正确（含大小写差异、短名、`..` 归一化负例）。
- 边界：属 `orz-tools` 生产码，修复需重建载体方能在狗粮线复验。

## 4. 上下文与 token 读数（按用户要求收取）

| 指标 | 读数 |
|---|---|
| 模型轮次 | 74 输出 / **73 轮**（`budget`：已用 73/0 轮，0＝unlimited） |
| 真实上传 token | **7,123,278**（cache_hit **6,857,600** ＋ cache_miss **265,678**；命中率 **90.1%**） |
| 模型生成 | completion **51,999** ／ reasoning **36,188** |
| 模型面估算峰值 | **224,324**（round 73；`blocks=2`、`tier=soft`、`truncate_tokens=500000`） |
| 上下文阶梯事件 | `first_block` 168,186（r53）→ `192k` 194,656（r62）→ `224k` 224,324（r73）；**未触发 H1(320K)/T1(500K)** |
| 压缩事件 | 0（未达窗口档）；`archives/` 未产生 |
| 资源面 | `host_resource_snapshot` tier=watch；卷余量 23.3 GiB、commit 余量 5.6 GiB |

注：本次 0.77 换算比（est/real）在 224K 档为 224,324 / 216,xxx 量级；因 run 中止于 224K，**深水区（224K+）读数本次未延伸**，与 0am run 的浅水区 1.32–1.34 结论不构成对照补充。

## 5. 相对原任务目标的完成度（如实）

- 已完成：路由回查起步（索引 → 盘点档 → FR-A08 断链回查）、全仓 Markdown 链接扫描、开局规划与台账写入。
- 未完成：**盘点档中任何一条摩擦项的实际处置**；无报告文档产出（agent 报告随崩溃丢失）。
- 因此本轮**不是「摩擦处理批」的完成记录**，而是「载体在真实任务下暴露进程级崩溃」的取证记录。

## 6. 未决与移交

1. **FR-N01 处置＝已闭合**（2026-09-17 用户令「直接进行修复并重建」）：已按 §3 建议修 `candidate_is_under_raw`（字节级比较＋钉子 2 条）、载体 **0.6.1** 重建换装并实证不再崩；登记面＝[`摩擦盘点 §7b`](FRICTION_INVENTORY_2026-09-17.md) ／ [`BACKLOG 0an`](../BACKLOG_AND_PRIORITIES.md) ／ [`TODO 0an`](../../TODO.md) ／ 索引 `GAP-BYTE-BOUNDARY-PANIC`。**Linux musl 三件套与代码提交留后续批**（用户 2026-09-17 指示「Linux 暂时用不上、重建不急」）。
2. **重跑口径**：若重跑，建议同一题面（处理盘点档摩擦项）＋本轮已确认的健康启动配置；无墙钟保持。
3. **工作树状态**：两仓未提交改动＝运行前既有（0am 批／FR-C04 批），本 run 未新增代码改动；`.tmp-friction-*` 三件留盘备查（`.tmp` 前缀在门禁排除面内）。
4. **卫生**：崩溃未留残进程（进程已退出，无孤儿）；journal `RUN-CLI-6aabf5eb` 保留为证据面。
