# GAP-ACAF-FAILCLOSED-PRODUCTION-ENABLEMENT 实施审计（2026-08-16）

> 状态：**fail-closed 生产启用翻转已实施并验证**（用户 2026-08-15 裁决放行，
> 2026-08-16 指示处理）——默认语义由影子翻转为强制，CLI run / ACP stdio /
> TUI 三个生产入口全部接线，核查清单 ⑦⑨⑩⑪ 收口并登记边界，供应工具与
> 启动链补齐。范围：`GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md`
> §5 下一步第一项（生产翻转）与 Slice 2B 审计 §6 核查清单 ⑦⑨⑩⑪。
> 设计权威回查：[`ADR-0011`](../../adr/ADR-0011-authenticated-control-and-action-fabric.md)
> 与 [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。

## 1. 摘要

ACAF Slice 2 fail-closed 机制（D-12~D-16）此前已实施但默认影子模式，生产
启用需要用户裁决。2026-08-15 用户裁决放行；本批完成翻转执行与核查清单
收口：

- **默认值翻转**：`ORZ_ACAF_FAIL_CLOSED` 由「显式 1/true 才开启」翻转为
  「未设置即强制；`0|false|no|off` 显式影子；`1|true|yes|on` 确认；其他值
  exit 2 fail-closed」（保持取值语义——拼写错误绝不静默关安全门）。
- **生产入口全部接线**：CLI run（既有）外，ACP stdio server 与 TUI 此前
  **未挂签名器客户端**（`AcpServer` 构造 controller 时无 `.with_acaf()`），
  随本批补齐（`AcpServer` 增 `acaf`/`acaf_fail_closed` 字段与 builder，
  prompt 与 grill 两个 controller 构造点接线；TUI 经 `TuiConfig` 透传）。
- **核查清单 ⑦⑨⑩⑪ 收口**：⑦ web_search 显式排除并加 e2e 回归锁；⑨ host
  稳定面复核为不补绑定；⑩ URL gate 与票据摘要规范化等价（既有单测锁定）；
  ⑪ 重定向逐跳 URL gate 覆盖已有测试，票据一次 consumed 语义登记。
- **供应与启动链**：新增 `orz-acaf-provision` 工具（DPAPI keystore +
  signer manifest 生成，幂等）+ `scripts/orz_acaf_run.ps1`（供应并带环境
  启动 orz）。

## 2. 决策依据与范围

- 用户 2026-08-15 裁决：ACAF fail-closed 生产启用放行（BACKLOG P2
  IMPL-CONTROL-FABRIC 决策门）；翻转执行与核查清单 ⑦⑨⑩⑪ 收口待实施。
- 用户 2026-08-16 指示：处理 ACAF 安全门；web_search 按既有设计（走
  provider 原生搜索，ADR-0010 §3.7 条 10 冻结：同一 provider / 同一 key /
  同一计费面，禁止独立检索供应商）显式排除，不新增票据类型。
- 本批不触碰 Slice 3（ModeChangeTicket）与 Slice 4（Windows Sandbox），
  仍为开放项。

## 3. 已变更

### 3.1 默认值翻转（orz-bin `src/main.rs`）

- `acaf_fail_closed_enabled()`：未设置 → `true`（强制）；显式
  `0|false|no|off`（大小写不敏感）→ `false`（影子）；`1|true|yes|on` →
  `true`；**其他值 eprintln + exit 2**（fail-closed，与 wallclock/stall
  解析器同一纪律：畸形值绝不静默关安全门）。
- 新增单测 `acaf_fail_closed_defaults_enforced_with_explicit_opt_out`
  （未设置=强制、显式关闭=影子、显式确认=强制）。

### 3.2 ACP server 接线（orz-host `src/acp_server.rs`）

- `AcpServer` 新增 `acaf: Option<Arc<Mutex<AcafClient>>>` 与
  `acaf_fail_closed: bool` 字段；`with_gateway` 默认 None/false（库默认
  零行为变化）；新增 `with_acaf(...)` / `with_acaf_fail_closed(...)`。
- prompt 会话 controller 与 grill controller 两个构造点补
  `.with_acaf(self.acaf.clone()).with_acaf_fail_closed(self.acaf_fail_closed)`
  ——ACP 会话与 CLI run 共享同一签名器客户端与 fail-closed 姿态（D-15
  未配置 fabric 时在 `run_turn_with_guards` 启动期拒绝，无静默降级）。
- 新增测试 `acaf_fail_closed_without_fabric_refuses_prompt`（ACP 路径
  D-15 回归锁：无 fabric + fail-closed → prompt 报错且错误含 "fail-closed"）。

### 3.3 TUI 接线（orz-tui `src/runner.rs` + orz-bin `src/main.rs`）

- `TuiConfig` 新增 `acaf` / `acaf_fail_closed` 字段（Default 保持
  None/false，库测试零行为变化；Debug 手工实现省略签名器句柄）。
- runner 构造 `AcpServer` 时应用两个 builder。
- `run_tui` 与 `run_stdio` 均调用 `build_acaf_client()` 并按
  `acaf_fail_closed_enabled()` 设置 fail-closed。
- `tests/stdio_e2e.rs` 的 `spawn_stdio` 显式设 `ORZ_ACAF_FAIL_CLOSED=0`
  （wire-shape E2E 不测 fabric，跑影子模式；注释登记）。

### 3.4 供应工具与启动链

- 新 bin `orz-bin/src/bin/orz-acaf-provision.rs`：参数
  `<keystore-root> <manifest-output>`；`WindowsDpapiInstallationKeyStore::create_or_load`
  （幂等，已存在不覆盖密钥）+ 定位同目录 `orz-signer` 并计算真实
  binary_sha256 + 写 manifest（manifest_version=1 / signer_revision=1 /
  binary_name="orz-signer"），输出启动环境变量。非 Windows fail-closed
  （DPAPI 不可用，与签名器同姿态）。单测锁定 manifest 形状。
- 新脚本 `scripts/orz_acaf_run.ps1`（父仓库）：默认 bin 目录
  `orz/target/debug`、keystore 根 `$env:LOCALAPPDATA\orz\acaf`；供应（幂等）
  → 设 `ORZ_ACAF_KEYSTORE` / `ORZ_ACAF_MANIFEST` / `ORZ_ACAF_BINARY` /
  `ORZ_ACAF_FAIL_CLOSED`（`-Shadow` 开关可显式影子）→ 转发参数启动 orz。

### 3.5 ⑦ web_search 显式排除回归锁（orz-bin `tests/acaf_e2e.rs`）

- 新增 `fail_closed_web_search_executes_unticketed_with_zero_ticket_events`：
  真实签名器 + fail-closed，检索子代理 web_search 正常执行，断言
  ToolStarted/ToolCompleted 存在且 `control_ticket_*` 事件为零。

## 4. 核查清单 ⑦⑨⑩⑪ 收口

| 项 | 结论 | 证据/登记 |
|---|---|---|
| ⑦ web_search 票化形态 | **显式排除**（不新增票据类型） | 走 provider 原生搜索（DeepSeek `/v1/responses`、同一 key/计费面，ADR-0010 §3.7 条 10 冻结，禁止第二供应商）；无第三方 URL 目标，绑定面未定型且按既有裁决排除；e2e 回归锁（§3.5） |
| ⑨ host 侧执行参数绑定面 | **不补绑定，登记边界** | `run_terminal_cmd` 模型可控面=`{command,...}`；cmd_prefix / shell 后端 / SessionEnv 为 host 配置面，模型经工具参数不可控（`grok_build` bash `BashParams.cmd_prefix` 非工具参数）；env 绑定语义不变（run_terminal_cmd 空 env、run_tests 只绑 `TestRunner::env`） |
| ⑩ 执行面与票据绑定面错位 | **等价成立，登记残余** | 票据摘要 `resolve_network_url` 规范化（默认端口去、fragment 去、host 小写、凭据拒）由 `network_url_canonicalisation` 单测锁定（orz-assurance target.rs）；URL gate 对 scheme/host 做安全检查，默认端口/fragment 不影响网络效果（fragment 不发送）；残余：路径百分号编码规范化差异由 HTTP 客户端/CDP 同向规范化，登记不追 |
| ⑪ network 重定向不重新票据 | **登记语义，覆盖已有** | browser_read 每跳重查 URL gate（`Page.frameNavigated` 逐跳 + final location.href），探针测试 `frame_navigated_to_blocked_url_aborts_load` 已锁定重定向到被禁主机中止加载；web_fetch 手工管理重定向（reqwest policy none + SSRF 逐跳重检 + 跨主机显式 `CrossHostRedirect` 不静默跟随）；票据 ledger 一次 consumed 语义=票据授权初始 URL，逐跳强制属 URL gate 职责（独立于 ACAF 的 host 边界） |

## 5. 验证证据

- orz-bin 单测 7/0（+1 翻转语义；14 ignored 为既有）；
- orz-bin acaf_e2e **22/0**（+1 ⑦ 回归锁；真实签名器，Windows）；
- orz-bin stdio_e2e **1/0**（显式影子 env 后经真实 `orz --stdio` 帧）；
- orz-host acp_server 子集 **37/0**（36 既有 + 1 新增 D-15 ACP 回归锁）；
- orz-tui **178/0**；
- orz-acaf-provision 单测 1/0；
- 备注：orz-host 全量在本机出现既有长时/挂起测试（CPU 近乎零，疑似
  CDP/浏览器或长超时用例，非本批引入）；已中断并改跑受影响子集（acp_server
  37/0）。仓库门禁 `check_repository.py` 与 `git diff --check` 于 §6 前复核。

## 6. 边界登记

- **`--plan` 路径不接线**：仅模型轮（counterexample gate）无 host 工具，
  无外部效果动作，无需票据；翻转不影响该路径。
- **供应为逐用户/逐机器**：DPAPI 密钥作用域=当前 Windows 用户+机器；
  keystore 根默认 `$env:LOCALAPPDATA\orz\acaf`。签名器重建后须重跑供应
  （manifest 刷新为当前二进制哈希；工具幂等）。
- **web_search 显式排除**：若未来要求 web_search 也持票，属新切片
  （query+端点绑定 → Schema/signer/verifier/fixtures 全链），不在本批。
- **⑩ 残余**：路径百分号编码的规范化差异由客户端同向规范化，票据摘要与
  实际请求的网络效果一致；不追为缺口。
- **⑪ 语义**：票据 ledger 一次 consumed；重定向逐跳强制由 URL gate 承担
  （host 边界、独立于 ACAF）。若未来要求"每次重定向重新票据"，属设计变更。
- ACP/TUI 库默认保持 shadow（`AcpServer::new` / `TuiConfig::default` 零行为
  变化）；翻转只发生在 orz 二进制入口（env 驱动），测试与库消费者不受影响。

## 7. 下一步

> 优先级：本项（P2 IMPL-CONTROL-FABRIC fail-closed 生产启用）已闭合；
> 统一待办路由见 [`BACKLOG_AND_PRIORITIES.md`](../BACKLOG_AND_PRIORITIES.md)。

- Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 +
  policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）；
- Slice 4：Windows Sandbox backend（D-11）；
- ACAF 可选工程项：检索车道 web_fetch activation 绑定接线、conformance
  capture 票据场景、normalize_lexical 单源化、ACP 会话路径接 ACAF（已随
  翻转完成）；
- 实际操作项：通过 `scripts/orz_acaf_run.ps1` 建立生产启动链（供应 + 启动），
  观察真实运行影子台账/拒绝事件。
