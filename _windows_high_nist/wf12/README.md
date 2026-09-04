# TER T2.2 (W-F12) 本地透明层 — 组件与部署

> 状态：**T2.2 已闭合（2026-09-04）**——DNS NXDOMAIN 拒答器在 win-s4 墙内
> 可逆部署验证完成（网卡 DNS 临时指回环 + daemon 常驻于 job 内，测后恢复
> 原 DNS）；egress 复测总成本 920ms（≤10s）、最差单行 735ms（≤2s/目标），
> 见 [审计](../../docs/audits/TER_T2_2_WF12_LOCAL_TRANSPARENT_LAYER_2026-09-04.md)。
> TCP 快速拒答（WFP）未实施：防火墙 block + DNS 拒答已满足验收线，WFP 仅
> 作可选未来项。

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

VM 实测采用可逆方式（不落常驻配置）：同一 SYSTEM job 内保存网卡 DNS →
`Set-DnsClientServerAddress` 指 `127.0.0.1` → 拉起
`python dns_refusal.py --port 53` → `ipconfig /flushdns` → 复测 → 停
daemon → 恢复原 DNS。Windows DNS Client 实测接受回环 DNS（NXDOMAIN 7ms）。

## 3. TCP 快速拒答（可选未来项；2026-09-04 实测不需）

Windows 防火墙 block=drop 无法表达 RST；候选 = WFP 自定义 provider /
本机透明策略层（设计稿 §3.5）。实测防火墙 block + 本地 DNS NXDOMAIN 后，
四类客户端失败全部 ≤735ms（≤2s）且总成本 920ms（≤10s），验收线已满足；
WFP 层仅在后续摩擦轮需要“更接近 RST”语义时再评估。
