# TER T2.2 (W-F12) 本地透明层 — 组件与部署

> 状态：组件就绪（DNS NXDOMAIN 拒答器 + 自测）；TCP 快速拒答与 VM 墙内
> 复测待实机环境（T2.2 未闭合）。

## 1. 目标

墙外目标（allowlist 外域名/地址）在 ≤1–2s 内返回可判定失败（连接拒绝 /
DNS 拒答），不再黑洞式挂到超时；allowlist 内连通不变。验收：mteb HF
探测 ≤2s；连通扫描总成本 ≤10s（[`run_wf12_egress_probe.ps1`](../run/run_wf12_egress_probe.ps1)
输出前后对照）。

## 2. DNS 拒答器

[`dns_refusal.py`](dns_refusal.py)：loopback UDP DNS（127.0.0.1:53），非
allowlist 域名立即 NXDOMAIN；默认无域名 allowlist（评测允许端经
`--allowlist-ip` 直连 IP，DNS 面全局拒绝、不泄露 allowlist 内容）。

自测：`python dns_refusal.py --selftest` → `DNS_REFUSAL_SELFTEST_OK`。

部署（VM guest，需管理员）：

1. 以计划任务/服务常驻运行拒答器；
2. 把网卡 DNS 指向 `127.0.0.1`（`Set-DnsClientServerAddress`）；
3. 复跑 `run_wf12_egress_probe.ps1` 与 `run_enforcement_probe.ps1`。

## 3. TCP 快速拒答（待实机层）

Windows 防火墙 block=drop 无法表达 RST；候选 = WFP 自定义 provider /
本机透明策略层，需在 VM 实机以实测定实现（设计稿 §3.5）；落地后在同一
探针与验收线上复测（≤2s/目标、总成本 ≤10s）。
