# TER M1/M2 全面审查（设计合理性 / 实现合理性 / 设计与实现符合性）

> 日期：2026-09-04。性质：TER（工具执行层改革）M0 设计门 + M1（T1.1–T1.13，
> 已闭）+ M2（T2.1/T2.2 已闭；T2.3 代码半程；T2.4 未开始）的**全面审查
> 记录**。方法：多子代理并行只读静态核对 + 根代理抽查复核；三轴=设计
> 合理性、实现合理性、设计与实现符合性。结论：**有条件 PASS**（无 P0；
> P1 × 2 需先行定案，其余 P2/P3 可随 review-handling 批次收口）。
>
> 分片报告（原始证据，随本文保留）：
> [`00 设计层审查`](../../.ter_review_2026-09-04/00_design_review.md) /
> [`01 执行语义 T1.1–T1.5`](../../.ter_review_2026-09-04/01_m1_exec_semantics.md) /
> [`02 循环面 T1.6–T1.9+T2.1`](../../.ter_review_2026-09-04/02_m1_loop_surface.md) /
> [`03 阅读环境面 T1.10–T1.13`](../../.ter_review_2026-09-04/03_m1_read_env.md) /
> [`04 M2 W-F12/T2.3`](../../.ter_review_2026-09-04/04_m2_wf12_t23.md) /
> [`05 综述与门清单`](../../.ter_review_2026-09-04/05_review_summary_and_gate.md)。

## 1. 审查基线

- orz 子模块 TER 提交：`77ec1c14`（T1.1–T1.3）、`1835aa45`（T1.4）、
  `2169e39f`（T1.5）、`f2ed4d8e`（T1.6）、`e6e8aee0`（T1.7）、
  `95bf432c`（T1.8）、`da2c23a1`（T1.9）、`43d7b9d3`（T1.10）、
  `e0eaa251`（T1.11）、`ea444869`（T1.12）、`b18fabf5`（T1.13 Linux 门
  修复）、`dd5b1dac`（T2.3 确定性 fake 场景驱动）。
- 主仓库 TER 提交：`71465d8`（M0+T1.1–T1.3 登记）至 `91a390e`（T2.2 闭），
  含各步登记与审计；T2.1=`ac122e4`、T2.2 组件=`73ee183`/`d705a06`/
  `7cb0fb4`/`91a390e`。
- 审查期间 0l 并行合入 P2-14（主仓 `8655810`→`a7a3744`、orz
  `901cdcc3`→`29d57155`），与 TER 文件无交集，不影响本审查基线。
- 工作区存在 0l 未提交改动；本审查全部以提交态 + 实机证据文件为准，
  未复跑 cargo/pytest（纪律与工作树污染）。测试计数复核列为待实机项。

## 2. 总评

设计到实现的主链（默认收敛单源化 → 去硬杀 → idle+CPU 兜底 → processes/
env live 分区 → 轮预算默认无限制 → F6 三档 → W-F13 阅读面 → W-F11 环境
快照 → 墙钟单一化 → W-F12 本地透明层）在代码、测试、审计、实机证据四个
层面均能相互印证；审计文档总体诚实（T2.2 的 no-AC 三项 FAIL 如实结转，
实机数值 920ms/735ms/15–47ms 等可回查）。逐 T 步符合性见 §6。

审查门结论：**有条件 PASS**。两项 P1 为有限改动，不阻塞 T2.3/T2.4 并行
推进，但建议按 §7 顺序优先定案（尤其 M1L-1 影响 T2.3/T3.1 验收断言设计）。

## 3. P1（门放行前置，×2）

### P1-1（= M1L-1）tool_running `status=idle_killed` 事件只有“契约”没有“生产者”

- 位置：T0.2 契约 §4；T1.5 审计 §5（结转 T1.13）；T1.13 审计 §1.3
  （“由单测锁定”）；orz-loop `host_exec.rs:2982`（ToolRunning 唯一生产者，
  payload 无 status）。
- 证据（根代理抽查）：orz 全仓 `idle_killed` 仅命中终端快照
  （`terminal.rs:88-89` IDLE_KILL_SIGNAL）、KillReason 渲染
  （`bash/mod.rs:408-443`）、完成提醒（`task_completion.rs`）；orz-loop/
  orz-host 无任何构造 `{"status":"idle_killed","reason":…}` tool_running
  事件或对应单测；Python verifier/fixtures 有该形态（273 passed 只证明
  fixture 合规，不证明生产者接线）。
- 影响：真实 journal 在 idle-kill 场景只有提醒文本与 processes 快照；
  T0.2 链规则（每 call_id 至多一次 tool_running/不引入第二个
  tool_completed）无法实机验证；T3.1/T2.3 若按 idle_killed 事件设计断言
  将落空。
- 建议：二选一定案——(a) 在 host_exec 完成提醒/进程回收处补生产者
  （引用原 call_id/task_id + reason）并加单测；或 (b) 显式降级契约为
  “快照 + 提醒文本”。同时修正 T1.13 审计 §1.3 措辞（勿把 Python
  fixture 测试写成 Rust 生产者单测）。

### P1-2（= D-1）ADR §14.53 仍为“候选”，正文旧语义未转正/未标 gap

- 位置：ADR-0010 §14.53（L4306+，明示“未即时取代既有设计语义…各批次
  放行时再逐项转正”）；正文旧语义 L569 §4.5“Tool execution timeout
  300 秒…kill_active”、L370“120 轮预算”等；CLI_PROJECT_INDEX §1
  AUTH-TOOL-EXECUTION-REFORM（`pending`）；索引 §0.1（实现偏离 ADR 须
  标 gap）。
- 证据（根代理抽查）：ADR L565-575 守卫表仍写 300s kill_active；
  L370 仍写“不削减子代理 session 的…120 轮预算”；M1/T2.1/T2.2 已闭
  合并按 TER 语义运行（默认 0 轮、auto-bg、无分层硬杀），仓库无 GAP-*
  载体登记该过渡性偏离。
- 建议：落“转正/取代清单”（逐条列被取代 ADR 条款行 + 转正批次 + 10h
  绝对兜底例外句），在设计稿/ADR/BACKLOG2/索引 4 处同步；至少随 M2
  放行门签核。

## 4. P2（×5，随后续批次处理）

| ID | 位置 | 问题 |
|---|---|---|
| a1 S1 | `terminal.rs:1495-1535`/`1604`（0b 清扫）、`bash/mod.rs` BACKGROUND_TIMEOUT、设计稿 §2 P2/P3 | 10h 绝对兜底会硬杀**活跃**后台任务，与“无自身硬超时”文字口径并存未登记例外（并入 P1-2 修法；TODO2 T1.4 验收句建议同步加“10h 绝对安全兜底除外”） |
| a1 S2a | `terminal.rs:2036-2054`（process_done 守卫） | 子进程关管道（daemonize/nohup）但存活后 idle 采样停止，只剩 10h 兜底；建议登记限制或退化为 CPU/文件 mtime 采样 |
| a1 S2b | xai-tty-utils Linux CPU 记账（pgrp 汇总） | 自成进程组的忙碌后代不计 CPU，“无输出计算不误杀”在跨 pgrp 场景可能失效；建议补跨 pgrp 用例或 CPU 面不确定时回退不判 idle |
| D-2 / M2W-1 | T2.2 审计根因定案节；设计稿 §4“AppContainer off 与 allowlist 语义不变” | no-AC + allowlist“生产墙”是安全姿态变更，只有审计层登记，无设计层权威落点；建议 WINDOWS_HIGH_NIST 设计稿 + TER 设计稿 §4 各登记一行（AC 基线仅作探针对照臂） |
| a2 F6 | `terminal.rs:44`（COMPLETED_TASK_TTL=300s）+ `1543`（elapsed=now−start_time） | processes 分区 completed 行在 5 分钟保留期内 elapsed 持续增长，状态与计时矛盾；建议 completed 行冻结 end−start 并标注 ended |

## 5. P3 / 提示（合并去重，~24 项）

- 01（a1）：S3 idle-kill 提醒固定渲染“300s”（非默认阈值失真，T2.3 短
  阈值会读到误导文案）；S4 `computer/types.rs:101` 残留“typically 15s”；
  S5 模型面“默认超时 600s”按会话级渲染、逐调用普通命令实际 300s
  （预算=0/逃生阀路径文案报错值）；S6 纯 BashParams 下 `&` 面仍开
  （封闭依赖 host 注入 allow_background_operator=false，T1.3 审计已声明）；
  S7 三个 GROK_* env 为显式 override，建议 T0.1 “其它源”列显式列全。
- 00（设计层）：D-3 push 次数“3–4/≤4/≤3”三处表述不一（定单一口径：
  实现 ≤3、verifier 上限 ≤4）；D-4 env override 登记（同 S7/M1L-5）；
  D-5 设计稿头部状态行仍写“未实施”；D-6 T2.3/T3.x 验收行缺“事件
  字段+阈值”断言表；D-7 10h 例外句（并入 P2）；治理 §1 AUTH 状态词
  `pending` 与 §0.2 字面错位。
- 02（循环面）：M1L-2 runner `agent_timeout_seconds<60 → 60` floor
  （`run_agent_arm.ps1:335-336`，根代理抽查属实；“唯一墙钟”字面不完全
  成立，建议注释或去 floor）；M1L-3 命令摘要“≤80B”实为 ≤80 字符
  （CJK 下会超字节）；M1L-4（=D-3）；M1L-5（=D-4）；M1L-6
  `ORZ_MAX_WALLCLOCK=0` 与“未配置”渲染面不可区分；a2 F3 `controller.rs:86`
  残留“主车道仍为 MAX_TOOL_ROUNDS=120”注释（常量已 0，根代理抽查属实）；
  a2 F4 orz-bin 层 `ORZ_MAX_WALLCLOCK` 非法值 exit 2（fail-closed）而
  controller `parse_main_wallclock_limit_secs:113-124` 非法值静默回落
  unlimited（TUI/嵌入路径拼错会静默关预算，建议统一或留 warning，根代理
  抽查属实）；a2 F7 不支持 live 读取的后端渲染“（无）”，与 fail-closed
  不伪造意图有张力（建议显式标注 unsupported）。
- 03（阅读环境面）：M1R-1 64KB≈≤16K token 估算仅 ASCII 成立（CJK/emoji
  多字节可越读档上限，建议最坏 3 byte/char 复核或补多字节压测）；M1R-2
  `search_output_object` 命中整行无长度钳制（minified/base64 单行可撑爆
  注入预算）；M1R-3 “vm.js ≤2 次读完”未注 1000 行前提。
- 04（M2）：M2W-2 dns_refusal.py 只服务 UDP DNS（TCP/QUIC 回退面未覆盖
  且 README 未明示）；M2W-3 `--allowlist-domains` 空段静默忽略、bind/
  权限未启动自检；M2W-4 `ORZ_FAKE_SCENARIO` loader 三种形状非 fail-closed
  （text+tool_calls 并存静默取 text、call_id 非字符串静默回退、空
  tool_calls 批放行）；M2W-5 T2.3 验收与 M1L-1/S3 联动；M2W-6（待实机）
  ORZ_MAX_WALLCLOCK 穿 sandbox 未实证（T2.4 DryRun 需加 wallclock/budget_cue
  断言）；a2 F8（待实机）LIF run-origin 与 sandbox 启动时钟存在偏差，
  120s 档任务可能出现“显示剩余>0 却被沙箱 kill”。

## 6. 逐 T 步符合性

| 步骤 | 判定 | 关键证据 / 备注 |
|---|---|---|
| T1.1–T1.3 | PASS | Default+serde 收敛 true、180s 单源、schema 无 is_background、显式 false 逃生阀测试存在；纯 struct `&` 面见 S6 |
| T1.4 | PASS（10h 例外见 P2） | 分层 timeout 不再杀活跃命令；auto-bg deadline=min(预算,解析超时)；kill 仅逃生阀/评测墙钟/10h 兜底 |
| T1.5 | PASS（P3 见 S2a/S2b/S3） | ActivitySampler 输出字节+CPU；300s 默认、0=禁用；idle_killed 快照/提醒在层内自描述；journal 事件缺生产者见 P1-1 |
| T1.6 | PASS（P3 见 M1L-3） | processes 现算快照/越权/8KiB/空态/kill 不暴露 & |
| T1.7 | PASS | MAX_TOOL_ROUNDS=0 默认 unlimited；>0 守卫；budget_insufficient 仅显式上限；检索取 min 正确 |
| T1.8 | PASS | session 面 elapsed/limit/remaining；未施加不虚构；旧 wrapper 输出不变 |
| T1.9 | PASS | 默认 off 零注入；on 时 ≤3/run、事件载荷与 verifier ≤4 兼容；injected-block 不持久化 |
| T1.10 | PASS（P3 见 M1R-1/3） | 64K 档默认=上限、8K 下限、>64K 信封；行档 1000 保留；host clamp 8–64K 同步 |
| T1.11 | PASS（P3 见 M1R-2） | output_object pattern/行区间/尾部 + 单测；对象 id=log 路径；三字段落盘与透传 |
| T1.12 | PASS（墙内计时待实机） | env_snapshot ≤5s（本机 ≈1.6s）；kind 白名单双层保险；越权守卫 |
| T1.13 | 条件 PASS（P1-1） | Linux 门修复 2 处正确；709/201/273 计数待干净工作树复跑；§1.3 过度声明需修正 |
| T2.1 | PASS（P3 见 M1L-2/F4/F8） | 无 --max-wallclock/840；sandbox --timeout=官方值；DryRun 计划行可审 |
| T2.2 | PASS | AC→no-AC 根因实机证据链完整；920ms/735ms/15–47ms 与证据文件一致；3 项 FAIL 如实结转 |
| T2.3 | PARTIAL（未闭，与 TODO2 一致） | fake 场景驱动已提交；实机跨调用存活/idle-kill/journal 断言未做；断言设计依赖 P1-1/S3 定案 |
| T2.4 | 未开始 | 无半成品；stage-sync 清单仅含 T2.2 组件 |

## 7. 建议处理顺序

1. **定案 P1-1**（idle-kill journal 生产者：补代码+单测，或显式降级契约；
   修正 T1.13 审计 §1.3）——1 个 orz 提交，T2.3/T3.1 断言设计的前置。
2. **落 P1-2 转正/取代清单**（ADR/BACKLOG2/设计稿/索引 4 处同步，顺带
   10h 例外句与 no-AC 登记）——1 个主仓库提交。
3. P3 一轮 review-handling（S3/S4/S5/S6、M1L-2/3/6、a2 F3/F4/F7、
   M1R-1/2/3、M2W-2/3/4 等，代码+文档+审计登记）。
4. T2.3 实机脚本 + T2.4 同步接线（依赖 1 的新 build；断言加入 M2W-6
   env 检查与 S3 阈值文案检查）。
5. 复查门：干净工作树 cargo/pytest 计数复核 + musl release + 实机证据
   复核后正式签“审查放行”。

## 8. NOT-VERIFIED / 待实机（登记，非缺陷）

- 各步测试计数（224/226/229/230/231、40/47/48、49、699/700/703/704/
  709、201、273 等）未复跑；静态证据无不符。
- ActivitySampler Windows Job 实机 CPU 读数、env_snapshot 墙内 ≤5s、
  ORZ_MAX_WALLCLOCK 穿 sandbox、LIF run-origin 与 sandbox 时钟偏差
  （F8）、10h 兜底实际触发。
- musl release + smoke（T1.13 审计 §3 已登记为后续项）。

## 9. 结论

TER M1 与 M2 已实现部分的设计、实现与文档链总体自洽、证据可查、无 P0；
审查门为**有条件 PASS**。放行条件=P1-1（idle-kill 事件生产者或契约降级）
与 P1-2（ADR 转正/取代登记）先行定案；P2/P3 按 §7 顺序随批次收口。
本审查未对任何代码或文档实施修改。
