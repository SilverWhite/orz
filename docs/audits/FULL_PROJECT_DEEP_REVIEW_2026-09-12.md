# 全项目只读深审报告（2026-09-12）

> 定位：对本仓库（D:\CLI，含 orz 子模块）的一次性全面只读审查入档——设计合理性与
> 实现合理性双面。本文件只登记事实与证据，不改任何账本状态、不动 BACKLOG/TODO/索引、
> 不立项。0v-C 的修复工作已在 orz 侧交由 DeepSeek 处理中，本文对 0v-C 的发现仅作
> 审查时点快照，处理进度以 orz 侧实际提交为准。
> 审查方法：四路并行深查（orz-loop 控制面/Agent loop；保障/安全/journal 面；根目录
> 文档/协议/评测/脚本体系；0v-C 在途工作现场）+ 关键载荷性结论主会话逐点复核。
> 全程未修改任何文件。载荷性结论均经主会话亲自抽查验证（见 §5 复核记录）。

## 0. 审查范围与结论速览

覆盖面：orz Rust 工作区（orz-loop 约 58,500 行/45 文件、orz-host、orz-assurance、
orz-secrets、orz-sandbox、orz-bin、orz-tui）+ 根目录文档驱动体系（docs 137 份 +
docs/audits 141 份、protocol/runtime schema、evaluation 协议、assurance/scripts
Python 保障侧）。

**总体判断**：设计成熟度罕见地高、实现纪律严格；核心安全面 fail-closed 语义扎实、
钉子测试覆盖到位。最大两个实质问题：

1. 0v-C journal 断链缺陷的修复已在 orz 工作区完成待提交，且断裂面比最近一次提交
   信息（9526bce7）描述的更宽——不只是"脱敏命中"，URL 重序列化也会在无 secret
   命中时隐性改写 payload 断链（详见 §2 P0-2）。
2. 账本体系（索引/BACKLOG/TODO 三处）已出现同步断裂：索引 v2.86 已入账 0v 闭合
   （26 → 25），两份权威文档仍停留在 26（详见 §2 P1-5）。

## 1. 设计合理性

总体：设计层是本项目最强的一面，各处张力点都有显式裁决记录，未见结构性缺陷。

- **Agent 层/机械层分权**：模型面对冻结 8 工具面、机械层承载全部门禁的架构贯彻
  一致。主循环机制互相衔接自洽：上下文压缩只在 loop-top 安全间隙零模型调用触发
  （orz-loop `agent_loop.rs:1218-1244`），与请求/超时天然无并发竞态；reasoning-stall
  看门狗已物理删除（`agent_loop.rs:53`）；预算预留/结算/回滚三态齐备
  （`host_exec.rs:943/1072/1375/3801`）。
- **保障面设计成熟度高**：journal 双哈希（payload_sha256 + event_sha256 重算）使
  篡改检测独立于链验证；verifier 交叉校验完整（sequence 致密、run_id/manifest、
  prev 链、双哈希重算、单 terminal，`orz-assurance/src/journal/chain.rs:117-234`）；
  ACAF 用 K_install/K_session 派生 + TTL 5s（`orz-signer.rs:262`）+ nonce/sequence
  双查一次性（`orz/acaf/mod.rs:490-510`）+ signer 独立进程自测 hash，威胁模型清晰。
- **文档驱动体系**：权威顺序（ADR → 机器合约 → 实现 → 审计 → 存档）、schema 单一
  权威 registry（`runtime/run-event-payload-registry-v0.1.json`，Rust 法官直接读取 +
  Python 侧 `assurance/run_event_journal_validation.py` 导入派生，三方对拍）、
  manifest 门禁接入 CI（`.github/workflows/ci.yml:38` + `scripts/check_repository.py`
  `_check_orz_source_manifest`）——这套机制是项目能"活着记账"的原因。
  evaluation 协议对 oracle 隔离/未见性的定义（`evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`
  三存储域、五角色）超出多数正规评测团队水准。
- **主要设计张力**：读面 2026-08-24 有意放开后（`orz-host/permission.rs:312-317`），
  安全余量押在 ACAF 与 orz-tools 单点上——但 Windows 侧沙箱完全缺位（orz-sandbox
  全部 `#[cfg(target_os="linux")]`，Windows 仅 Job Object 进程收容 `job_object.rs:1-19`），
  而 Windows 是主力发布平台，该押注在此平台不成立。
- **设计落地短板**：单文件巨型化（`orz-loop/src/host_exec.rs` 9,116 行、
  `controller.rs` 5,856 行）与隐式状态机（散布布尔量 `pending_checkpoint`/
  `plan_gate`/`budget_exhausted` 等替代显式 FSM），主循环控制流依赖注释才能读懂。

## 2. 实现合理性与问题清单（按严重度）

### P0（在途，待收口）

**P0-1　0v-C 断链修复未提交**。orz 子模块工作区有未提交的完整修复批次
（5 文件 +198/-39，注释统一标注 "0v-C, 2026-09-12"）：`record()/record_async()`
改为返回落盘 `event_sha256`（`orz/crates/orz-assurance/src/journal/recorder.rs:79-105`，
seal_event 移到脱敏之后由漏斗唯一执行，writer ack 回传落盘 sha :312）；四个调用方
全部改为线程返回值推进 prev 链（`orz-loop/src/controller.rs:3905-3917`、
`orz-bin/src/main.rs:725-745`、`orz-host/src/acp_server.rs:170-194`、
`orz-host/src/session.rs:187-214`，session 侧顺带修掉 `next_sequence` 取自预封印
对象的问题）；新增两条钉子测试（recorder.rs:469 `record_returns_post_funnel_sealed_hash`、
recorder.rs:513 `chain_threads_returned_hash_and_replays_valid_after_funnel_rewrite`）。
方案正确（提交信息两方向中的方向①"重 seal hash 回传 emit 侧"），grep 确认生产
代码无漏斗外 seal 调用残留。**但 HEAD 构建仍会复现断链**；`event.rs:314` 的
"caller MUST call seal_event()" 文档注释未同步更新。未跟踪的
`crates/orz-assurance/examples/redact_forensics.rs` 为根因调查 scratch 工具，
不属生产面。

**P0-2　断裂面比账面描述更宽（主会话复核确认）**。提交信息 9526bce7 称"任何脱敏
命中事件使其后一行断链"，但脱敏漏斗的 URL 处理路径会把 URL 经
`url::Url::parse` 后重新 `to_string()` 序列化（`orz-secrets/src/sanitizer.rs`
`redact_urls_in`，由 `redact_secrets` 在 MATCH_ANY 命中后无条件执行）——默认端口
剥离、百分号规范化等情况下**无 secret 命中也会改写 payload**。新钉子测试正是用
URL 归一化触发改写的，即是佐证。这意味着自 0.3.2 起"任何含裸 URL 的卷都可能隐性
断链"，与提交信息"正常结束历史卷链完整只是恰无脱敏命中"的表述存在张力。建议对
历史卷做一次全量 verifier 复扫以核实实际断裂范围。

### P1

**P1-3　Windows `--allow-shell` 会话无写盘/网络内核限制**。orz-sandbox 全部
Linux-only；Windows 仅有 Job Object kill-on-close 进程树收容（`job_object.rs:9-19`）。
授权 `--allow-shell` 的 Windows 会话对写盘/出网无内核限制，仅靠 ACAF 兜底。至少应
在 README/安全文档明示，或补 Windows 侧限制。

**P1-4　Linux bwrap profile resolve 失败静默降级为无沙箱计划**
（`orz-sandbox/lib.rs:461-469`），对 write-deny 是 fail-open；read-deny 有
`requires_read_deny` fail-closed 启动路径防护（lib.rs:415-441），两形态不一致，
应统一 fail-closed。

**P1-5　账本三处同步断裂（主会话复核确认）**。`CLI_PROJECT_INDEX.md:3-5`（v2.86，
2026-09-12 14:15）已宣布 0v 闭合入账、计数 26 → 25、FUS-RETRIEVAL-ENGINE-SERP 转
implemented；但 `docs/BACKLOG_AND_PRIORITIES.md:11`（"未闭合计数（2026-09-11 口径）…
未闭合总数：26 项"）与 `TODO.md:19`（"未闭合总数：26 项"）仍为 26，0v 小节与
FUS 条目同样滞后。违反自定"完成一项同步 BACKLOG/TODO/索引三处"纪律，暴露该同步
靠人肉记性。反向对照：0x（27→26）与 0y 立项条目三处一致。

### P2

**P2-6　SERP 预算结算靠文本回读**。预算结算依赖对工具输出文本的 JSON 信封解析
（`orz-loop/src/host_exec.rs:3801-3803` `serp_navigations_from_output`），注释自认
"若日后出现非信封失败却真实导航的路径，需把导航数改为由宿主事实回传"
（host_exec.rs:3795-3800）。结构化事实应经宿主 API 回传而非文本回读。

**P2-7　非流式与流式重试链双实现漂移**。非流式 D-6 空响应链无退避 sleep、错误
直接 `?` 中止（`gateway/transport.rs:2095-2101`）；流式路径有 backoff 500ms→10s +
cancel 感知（transport.rs:2144, 2224-2226）。行为不一致，违反自述"官方节奏"。

**P2-8　begin_search pacing jitter（已知观察，仍未裁决）**。冷却过期后
（`saturating_sub` = 0）仍无条件叠加 0–2.5s jitter
（`orz-host/src/local_browser/serp.rs:281-284`，主会话复核确认）。应改为仅在补足
冷却时叠加，或由用户裁决语义。

**P2-9　常量跨 crate 硬复制**。`SERP_MAX_SEARCH_QUERY_CHARS=500` 在 orz-loop 硬
复制为字面量 `.take(500)`（`host_exec.rs:199`）与 orz-host 常量无编译期联动
（已登记 P3，双改一必漂移）。

**P2-10　ACAF 显式回退无审计痕迹**。`ORZ_ACAF_FAIL_CLOSED=0|false|no|off` 纯环境
变量静默切换 shadow 模式（`main.rs:398-412`，malformed 值 exit 2 正确），同用户
任意进程可关闭强制且不留痕。建议 opt-out 时写 journal/警告。

**P2-11　脱敏元数据不诚实**。漏斗改写 payload 后事件 `redaction` 字段仍为
`Redaction::None`（recorder.rs:102-105；`event.rs:223-227`），审计元数据谎报
"未脱敏"。

**P2-12　脱敏漏报面**。AWS secret access key（40 位无前缀）不匹配、非 http(s) URL
不覆盖、高熵裸值无检测、赋值形态 8 字符下限有漏报；运行期晚于启动登记的密钥不进
注册表（sanitizer.rs）。误伤控制较好（`\b` 锚定、segment 边界、8 字符下限均有测试）。

### 体系面

**S-13　evaluation"引擎造好未上路"**。runner（`assurance/evaluation_runner.py`
856 行，含 `_verify_oracle_isolation`、journal 哈希链）+ 测试真实存在；但两协议
自述"阈值未校准""尚未创建真实 evaluation/holdout"，考卷语料未冻结，execution
面停摆近两月。

**S-14　仓库卫生**。根目录未跟踪残渣 `scan15.py`/`scan16.py`/
`orz-0vc-run-console.log`（0v-C 排查过程副产品，scan15/16 为按文件名找
deep_audit.py/0V_S4_BATCH2 相关文件的临时脚本）无 ignore 规则命中；`sweep-s0/`
一次性产物 5 个证据文件被 git 跟踪，与自家"一次性产物归档存档/"纪律相抵；
`tmp0vc/`（28 个 scanN.py + chaincheck.py 取证脚本，tmp* 规则已覆盖）是 0v-C
触发源追查的在途现场。scripts/ 一次性探针（`s4_vm_repro_v3..v14.ps1`、
`__wtest*.ps1`）与核心门禁同目录同 CI，无生命周期标记（active/consumed）。

**S-15　schema 合约承载审计流水**。`runtime/run-event-v0.2.schema.json` 的
description 内嵌数千字变更史，机器合约被当 changelog 用，腐蚀 JSON 评审与 diff
可核对性。

**S-16　TODO2.md 非死文档但有流水过期风险**。TER 专用勾选树（302 行），被
TODO.md P0-0l「⑧」引用，M3（T3.1–T3.5 复验）仍开放；但其验收证据引用的版本
（0.3.x）已被 0.4.x 载体超越。

### 正面确认（复核无误的强项）

- orz-loop 约 250 处 unwrap/expect 几乎全为 Mutex lock 与测试、无业务 panic；
  lib 测试 451 `#[test]` + 321 `#[tokio::test]` = 772 个；TODO/FIXME 仅 8 处。
- fail-closed 面扎实：`request_permission` 默认 Deny（`host.rs:849-856`）、未知
  工具兜底 LocalMutation（`tool.rs:110-112`）、MCP `__` 名双层拒绝（tool.rs:201-203）、
  prompt 超时 → Deny、ACAF 未配置拒启动；browser_control 权限映射修复
  （340fe4a7，navigate/search → Read(None)，未知动作仍 Edit fail-closed）落位正确。
- journal 并发模型正确：多 producer → mpsc channel → 单 writer 顺序落盘 +
  oneshot ack，refused append 不推进链（P2-7 先例纪律保持）。
- 软备忘 `ordered_engines`（失败置尾不移除，`serp.rs:295-307`）与全量
  `engine_attempts` 信封（含 cap 拒绝路径 `cdp.rs:930-958`）落码完整；
  web_search 并发=1 经排除出 `PARALLEL_READ_TOOLS` + 单席位子代理双保险实现
  （`agent_loop.rs:693-704`）。
- 0v-C 修复响应质量高：一次修全四个调用点、无生产面残留、API 返回类型变化在
  编译期强制适配。
- Python 保障侧（assurance/，219 文件、71 个测试文件、P0–P5 合约 + verifier）
  CI 全量跑，`gsa.py` 为 `assurance.cli:main` 的 7 行入口 shim（R-3 裁决保留）。

## 3. 长期观察

该体系擅长登记与核证，缺乏"关闭与销毁"的机械机制——evaluation 语料长期缺位、
scripts 无限堆积、账本单条 bullet 承载十余日流水数千字（BACKLOG_AND_PRIORITIES.md
计数行已成数千字流水），三者同根。权威文档数量（docs 137 + audits 141）与单条
流水密度已接近人力可维护上限，"同步三处"由纪律而非机械保证，最近一轮（0v 闭合）
已实际断裂。建议把计数一致性与账本瘦身做成 `check_repository.py` 的机械检查项，
而非文字纪律。

## 4. 建议优先级

1. **0v-C 修复收口**（orz 侧 DeepSeek 处理中）：提交 + 全量测试验证 +
   `event.rs:314` 注释同步；建议附带对历史卷的全量 verifier 复扫（P0-2 断裂面
   核实），必要时补"旧断链 journal 只读回放兼容注记"。
2. **三处账本同步回补**：25 口径落到 BACKLOG/TODO（P1-5）。
3. **Windows 无沙箱现状明示**（README/安全文档）或补限制（P1-3）。
4. bwrap write-deny fail-open 统一 fail-closed（P1-4）。
5. 中期：计数一致性入机械门禁、evaluation 语料冻结、scripts 生命周期标记、
   schema description 瘦身。

## 5. 主会话复核记录（载荷性结论逐点验证）

| 结论 | 验证方式 | 结果 |
|---|---|---|
| 0v-C 修复存在于工作区且形态正确 | `git -C orz diff --stat` + recorder.rs grep（record 返回 on-disk sha、seal 在脱敏后、ack 回传 :312、测试 :427-589） | 确认 |
| URL 重序列化可无 secret 命中改写 payload | 读 sanitizer.rs `redact_secrets`/`redact_urls_in` 全文：MATCH_ANY 命中后无条件走 `url::Url::parse` + `to_string()` | 确认 |
| 账本三处不一致 | grep 三文件"未闭合"计数行（INDEX v2.86 = 25；TODO.md:19 与 BACKLOG:11 = 26） | 确认 |
| begin_search 冷却过期仍加 jitter | 读 serp.rs `begin_search`（`saturating_sub` 后无条件 `+ jitter_millis`） | 确认 |
| SERP 预算结算靠输出文本回读 | 读 host_exec.rs:3795-3806 及其自注 | 确认 |

—— 本报告为审查时点快照（2026-09-12），orz 子模块工作区含 0v-C 在途修复
（DeepSeek 处理中），后续以实际提交为准。