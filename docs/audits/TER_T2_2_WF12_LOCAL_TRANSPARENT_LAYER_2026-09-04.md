# TER T2.2 W-F12 本地透明层实施审计（2026-09-04）

> 范围：TODO2 M2 步 T2.2——墙内 egress 先测基准，实现 DNS/TCP 本地拒答，
> 复测 ≤2s/目标；allowlist 内连通不变。验收 = mteb HF 探测 ≤2s、连通扫描
> 总成本 ≤10s（`run_wf12_egress_probe.ps1` 输出前后对照）+
> `allowlist_reachable` 不变。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.5/§10 S2-2；组件 README：
> [`_windows_high_nist/wf12/README.md`](../../_windows_high_nist/wf12/README.md)。

## 1. 根因与真实接线修正

**发现（实机）**：high-nist 墙若同时启用 AppContainer（空能力）与 egress
allowlist，per-IP allow 规则会被 AC 网络 compartment 吞掉——探针
`allowlist_reachable` 在 AC 模式下必 FAIL（多次实测 0–16ms 即拒绝）。生产
agent 墙自 2026-09-03 裁决起为 `--no-appcontainer`（orz.exe 在 AC 下 DLL
init 失败）+ allowlist（`run_agent_arm.ps1` 固定接线）。因此 T2.2 的
“allowlist 内连通不变”必须在 no-AC 生产墙上测量，而不是 AC 模式。

## 2. 改动

- `_windows_high_nist/policy/enforcement_probe.py`：
  - 新增 `-ExpectAppcontainer`（默认 1）；=0 时跳过 `appcontainer_token`
    断言并在 debug log 记 `SKIP appcontainer_token (mode=disabled …)`
    （与父侧 sandbox observation 在 no-AC 下 discard 该检查同语义）；结果
    JSON 增 `appcontainer_expected` 字段。
  - network/allowlist/metadata 三项判定附 stdout 时延行
    `WF12 <check>_ms=<ms>`（no-AC 子进程 stdout 可捕获，作为数值证据）。
- `_windows_high_nist/run/run_enforcement_probe.ps1`：
  - 新增 `-NoAppcontainer` switch（仅 high-nist；与 `-Native` 互斥），
    sandbox CLI 透传 `--no-appcontainer`，子探针传 `-ExpectAppcontainer 0`。
  - no-AC 时探针**原位执行**（`C:\s4\_windows_high_nist\policy\...`，不再
    复制进工作区）——复现修复：no-AC 子进程（受限 AgentUser）读不到
    SYSTEM 预拷入工作区的探针副本（`Errno 13`），而 C:\s4 布局与生产
    orz.exe 同源可读；AC 模式保持工作区复制（AC 子进程只能读包授权路径）。
- `assurance/windows_sandbox.py`：
  - 三处工作区 ACL grant（run-user / current-user / AppContainer SID）加
    `/T`（覆盖已存在的子文件——探针副本等 spawn 前由 SYSTEM 写入的文件）；
  - 捕获模式（非 AppContainer）下把子进程 stderr/stdout spill 到
    `<workspace>\.sandbox-child-{stderr,stdout}.log`（harness 诊断可读精确
    失败，如 no-AC 子进程早退原因）。
- `scripts/s4_vm_wf12_probe.ps1`（新增 host runner）+ `s4_admin_bridge.ps1` /
  `s4_bridge_request.ps1` 受控 op `vm-wf12-probe`：
  - Run A：AC-enabled + allowlist 基线（预期 allowlist_reachable FAIL，登记
    “AC 模式不可作 allowlist 墙”事实）；
  - Run B：no-AC 生产墙 enforcement 探针（网络三项 PASS + WF12 时延行）；
  - Run F/G：egress 4 客户端表（前测）+ 可逆 DNS 拒答部署后复测
    （保存并恢复网卡 DNS、临时拉起/停止 `dns_refusal.py`，`DNS_RESTORED`）。
- `scripts/s4_vm_arms_formal.ps1`：Sync-Harness 清单补
  `run_wf12_egress_probe.ps1` 与 `wf12/dns_refusal.py`+README（guest
  `C:\s4` 部署一致性）。
- `_windows_high_nist/wf12/README.md`：状态更新为 VM 实机部署验证完成。

## 3. 验收证据（win-s4 实机，2026-09-04）

### 3.1 双墙形态对照（enforcement-probe high-nist + allowlist
221.204.163.76）

- Run A（AC-enabled）：spawn compliant、19/19 除 `allowlist_reachable`
  **FAIL**（0–16ms 即拒）——AC 空能力 compartment 吞 allow 规则。
- Run B（no-AC 生产墙）：spawn compliant、network/metadata 项 PASS：
  `WF12 network_blocked_ms=0–16`、`WF12 metadata_blocked_ms=0–15`；
  **allowlist_reachable PASS（15–47ms，allowlist 内连通不变）**。

### 3.2 egress 前后对照（run_wf12_egress_probe.ps1，目标
huggingface.co/github.com/pypi.org，client 超时 20s）

| 阶段 | 总成本 total_wall_ms | 最差行 | 其它 11 行 |
| --- | --- | --- | --- |
| 前测（无 DNS 拒答） | 13091–13558 | Test-NetConnection huggingface.co
12906–13309ms（DNS 解析停滞） | 全部 0–68ms |
| 复测（DNS 拒答 127.0.0.1:53 + 网卡 DNS 指回环） | **920** | TNC
huggingface.co **735ms** | 全部 0–27ms |

- DNS 拒答器自检语义复验：`RESOLVE_NXDOMAIN=True`、`RESOLVE_ELAPSED_MS=7`；
- `DNS_RESTORED=True`（原 DNS 172.19.96.1 已恢复；探针 job 结束后无残留
  进程/规则——`WF12_FWRULES` 空、daemon 已停）；
- op 级干净状态：`NOAC_NETWORK_OK=True`、`WF12_PROBE_HOST_OK=True`、
  `EXIT=0`。

验收对照：≤2s/目标（最大 735ms ✓）、连通扫描总成本 ≤10s（920ms ✓）、
allowlist 内连通不变（PASS ✓）。mteb HF 探测 ≤2s 的 M3 实机回归由 T3.3
承接。

## 4. 边界与结转

- no-AC enforcement 探针的 `workspace_writable` / `temp_redirected` /
  `low_integrity`（行为代理）三项在 no-AC 子进程内 FAIL——子进程对
  Low 标签工作区根目录创建文件得 `Errno 13`（ACL 已含 AgentUser M，
  根因疑为 no-AC 令牌完整性/授权语义差异，仍需沙箱侧定案）。该项不影响
  W-F12 网络验收（生产 no-AC agent 实跑可写 workdir、journal 全回收），
  结转 T2.4/0l 沙箱校准处理，本步不伪造全绿。
- TCP WFP 快速拒答层未实施：实测防火墙 block + DNS 拒答已满足 ≤2s，WFP
  仅作可选未来项（README 注明），不阻塞本步验收。
- 子进程 stderr/out spill 只作用于一次性探针工作区；不改变 schema/观测
  契约（observation JSON 字段不变）。

## 5. 证据文件

- `_windows_high_nist/vm-wf12-probe-result.txt`（OK 全量输出；含 Run A/B
  observation 与 result JSON、Run F/G egress 前后表）
- `_windows_high_nist/job-wf12-probe-20260904_*.txt`（各轮 job 日志）
- guest `C:\workspace\wf12-ep\{ac-enabled,noac,egress,egress-post}\`
  observation/result/child stdout 原档（复制回宿主的可随时补录）
