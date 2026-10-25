# 356 批：355 批 §6 登记项用户裁决落码（self_check 无条件在位＋browser 超时文案分流）

> 日期：2026-10-25
> 用户令：「我考虑Readonly接受现状／我考虑self_check无条件在位即可／问题4需要处理一下／其他的仅暂时记录即可」——对 [355 批档](355_353_354_REVIEW_REMEDIATION_2026-10-25.md) §6 六项登记的一揽子裁决。
> 落码：orz `cc3d74a1`（5 文件，+81/−40）。

## §0 裁决与处置对照

| 355 §6 登记项 | 用户裁决 | 处置 |
|---|---|---|
| §6.2 ReadOnly 车道读面开放边界 | **接受现状** | 不落码；边界转为风险接受记录（本批档 §2），随 journal 审计＋敏感面提示兜底 |
| §6.3 self_check"教而不给"（orz plan 车道） | **无条件在位** | 本批落码（§1） |
| §6.6 browser 超时运行时文案失实 | **处理** | 本批落码（§3） |
| §6.5 conformance capture 重录 | 暂时记录 | 维持登记（随未来 ACAF bootstrap 修复小批一并） |
| 353 §5.3②③（RLI guide 句／被动退化一根针） | 暂时记录 | 维持待裁决 |
| 终答轴最后窗口／写放大 | 暂时记录 | 维持 353 知情取舍 |

## §1 self_check 无条件在位

- **退役面**：`AgentLoopController::self_check_enabled()` 谓词（原 `console_default_enabled || terminal_declaration_required`）与 tool_run 执行面 `self_check_disabled` 拒绝臂一并删除。理由链：教学常驻行 `SELF_CHECK_RESIDENT_LINE`（"请对非日常聊天类的任务调用反例自查"）本就不门控地拼进主车道系统提示——可用性若随 `ORZ_TERMINAL_DECLARATION` 同源关闭即成"教而不给"（355 批 §6.3 登记、controller.rs 留痕处）；355 前生产位恒真时本工具恒在声明面，本批恢复并**固化为无条件**。
- **保留面**：拒绝语义仅存 **"main-lane only" 臂**（子代理上下文 activation_id 在位时拒绝，`self_check_lane_denied`）；检索子代理声明面投影的 `self_check` 过滤器（`subagent_tool_projection`）不动——子代理不装配自查。
- **连带改述**：`orz capabilities` 的 `CAPABILITY_COVERAGE` 第④段——self_check 自 356 起无条件声明（schema 仍在 loop 侧、host 无单源，维持 NOT covered 口径不变）。
- **测试改判**：0dm 拒绝语义钉 `refused_self_check_is_not_counted_as_called` → `self_check_available_on_default_controller_counts_as_called`（缺省控制器上 self_check 执行成功且 `self_check_called=true`）；四枚面清单钉随批补 `self_check`（benchmark 面／ReadOnly 面／R1 list 投影／不可读工作区投影——self_check 属规则式非工作族，与 bash 同腕保留声明，不参与工作工具探针）。

## §2 ReadOnly 车道读面开放＝风险接受（记录性）

用户裁决接受现状：读面无 workspace 沙箱的口径全车道一致（含 ReadOnly policy 车道）。既有兜底不变——journal 审计逐次记录、`[敏感面]` 提示（`.ssh/.aws/.env*/*.pem/*.key/keystore`＋SSH 裸名＋`.envrc`）、`.gsa` 会话卷内部窗口契约、写面门禁不动。此登记项自本批起从"待裁决边界"转为"已接受的风险记录"，不再占用裁决队列。

## §3 browser 超时文案如实分流

`bounded_host_owned` 超时回执（`ToolError::Timeout`，进模型面）按工具族分流：

- **browser_read／browser_control**：不再宣称"无子进程树"，改述"调用未完成；会话级浏览器后端可能已被本次调用拉起、归会话句柄持有（随会话清理），本超时不杀灭"；
- **其余早退路径**（project_doc_index／pdf_read／web_fetch-PDF）：维持原"host-owned path (no child process tree)"——确无派生子进程；
- tracing warn 同源分流；355 批"运行时文案未动（有钉断言）"的留痕随批清账。

测试：`bounded_host_owned_passes_through_and_times_out` 增 browser 臂（`browser backend`／`session-owned` 正断言＋`no child process tree` 反断言）；非 browser 名原钉不动。

## §4 读数（orz `cc3d74a1`，全绿）

- orz-loop lib **910/0/3**（改名不改数）＋run_turn_integration **4/4**；orz-host 串行 **452/0/6**（browser 文案新臂在内）；orz-web **79/0**（历史 fixture 含 `self_check_disabled` 事件属存量数据回放，不受退役影响）；orz-bin 六套件全绿；orz-assurance tool_names 扫描净（coverage 豁免 needle 保持）。
- fmt 净；clippy 改动行零新增；`cargo check --workspace` 净。

## §5 剩余登记（不变）

capture 重录（随 ACAF bootstrap 修复）、353 §5.3②③、终答轴最后窗口／写放大——维持 355 批档 §6 登记口径，无新动作。
