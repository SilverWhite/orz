# Grok compaction provenance probe（2026-07-23）

## 结论

锁定的 Grok `0.2.111`（build `94172f2aa4`）已在 Windows 管理员环境完成一次 fake-only
`/compact` 来源追踪实测。固定会话观测到：

- `PreCompact` → `PostCompact` 两个 hook，均登记 `source=manual` 和同一 session ID；
- `compaction_requests/*.json` 中的 5-item `chat_history` 来源 span、request ID 与 `trigger=manual`；
- `compaction_checkpoints/*.json` 中的 checkpoint ID、`prompt_index_at_compaction=1` 和派生摘要；
- 压缩前来源含 retained/omitted 两个 canary，摘要只含 retained canary，且摘要明确标为
  `derived_unverified`、不得替代原始 observation；
- 压缩后第三次主请求成功，证明固定 fixture 能继续会话；
- 独立 verifier 16/16 checks 通过，临时防火墙 6 条规则全部清理，真实模型调用为 false。

这证明的是 Grok `0.2.111` 手工压缩路径的机械 provenance 边界，不证明摘要语义正确，也不证明自动阈值触发。

## 实现

入口：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass `
  -File .\scripts\invoke_grok_compaction_provenance_probe.ps1 `
  -OutputDirectory .\.observed-runs\grok-compaction-provenance-v1 `
  -ReleaseMetadataPath .\upstream\grok-build.lock.json
```

实现由以下组件组成：

- `fake_deepseek_provider.py` 的 `compaction-provenance` 三主请求场景；
- `record_grok_compaction_hook.py` 的最小 hook 收据；
- `run_grok_compaction_provenance_probe.py` 的隔离 profile、loopback provider、Job Object、临时出站阻断、
  ACP 三轮驱动与 hash-only projection；
- `verify_grok_compaction_provenance_probe.py` 的独立 artifact replay；
- result/verification JSON Schema 和 synthetic/tamper regression。

原始 provider body、session event/update、compaction request/checkpoint 与来源快照只存在被忽略的
`.observed-runs/`。checked-in 文档只保留 digest 与边界结论。

## 成功观测

成功目录：`.observed-runs/grok-compaction-provenance-20260723-07`

| 项目 | 观测值 |
|---|---|
| probe | `COMPACT-9d244eea37e346dd9b467b41b312a83b` |
| session | `019f8e77-bff3-7363-90b3-6cc912ad6106` |
| source span | 5 items |
| source canonical SHA-256 | `a6c04e8f4c44a2aec7be7a8c0dc90c29e54c38ce1c6af35907add95126523f6f` |
| summary | 1052 chars, `derived_unverified` |
| summary SHA-256 | `030e9b6314b6186fc18d7a5b48a2b90ddb91fbe104c2a287a68f73c65127330d` |
| compaction request SHA-256 | `0c5e7e270553059ba81e2e2ac4b9ac5f658116ba93c98dc050d2cb7693f819d4` |
| checkpoint SHA-256 | `788163e2bbf33eea1c5f6a2549585b7fb43d4b16d34afd4a217d72d78443d76c` |
| hook receipt SHA-256 | `d6e60dfeb04fb32faf8b2fa9b29d70fc504d5523286f8d774f6feeb7d5f3e9a7` |
| provider capture SHA-256 | `cb8cf7f898734d25efcaf96421d5a010cea5d53497e33d681036076348ad1c9b` |
| result SHA-256 | `7588b602865aceb4fbcb613cb7f41ddf24edeea00573cc98f74f169d5d5efa69` |
| verifier | 16/16 checks passed |
| firewall cleanup | `remaining_rule_count=0` |

`summarized` 范围由 request artifact 直接给出。Grok `0.2.111` 没有逐 item 记录 retained/discarded
映射，因此两者保持空数组，映射登记为 `unknown`；探针不从摘要文本反推。

## 失败记录与修正

先前的 `-01` 至 `-06` 目录保留为诊断证据：

1. 防火墙规则前缀不符合既有白名单；
2. 标题生成 auxiliary request 被误计为主请求；
3. 固定摘要短于 Grok 的 500-char degenerate-summary 下限；
4. Windows hook 命令缺少 PowerShell `&` 调用运算符；
5. result 对 binary inspection/firewall add 收据字段名适配错误。

每次失败均通过 `finally` 清理临时规则；最后一次成功观测才用于 capability 结论。

## 明确未证明

- 未触发自动 compaction threshold；
- 未进行真实 DeepSeek 请求、费用调用或 credential 使用；
- 未证明 summary 的事实性、完整性或科学正确性；
- 未证明未记录的 retained/discarded 映射；
- 未把 summary 用作原始证据恢复来源。

因此下一阶段若要进行真实 DeepSeek development probe，仍需用户单独授权和费用确认。
