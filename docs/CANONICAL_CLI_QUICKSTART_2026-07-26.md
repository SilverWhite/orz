# Canonical CLI quickstart（2026-07-26）

## 目标

本 quickstart 覆盖当前魔改版 CLI 的 offline P0.5 入口。它不调用真实模型、不发网络请求、不读取 credential。

## Doctor

快速检查 CLI wiring：

```powershell
python gsa.py doctor --quick
```

完整仓库机械检查：

```powershell
python gsa.py doctor
```

JSON 输出：

```powershell
python gsa.py doctor --json
```

## Source Visibility Gate

对 source visibility ledger 执行全文可见性门禁：

```powershell
python gsa.py source gate --ledger assurance/fixtures/source_visibility/mixed-visibility-ledger.json
```

机器可读 receipt：

```powershell
python gsa.py source gate --ledger assurance/fixtures/source_visibility/mixed-visibility-ledger.json --json
```

## Canonical Offline Run

执行 canonical guarded CLI offline 路径：

```powershell
python gsa.py run `
  --ask "Check whether this source can support the requested claim." `
  --source-ledger assurance/fixtures/source_visibility/mixed-visibility-ledger.json `
  --run-root .tmp/canonical-run
```

该命令会写出：

- `run-manifest.json`
- `task-contract.json`
- `source-visibility-ledger.json`
- `source-visibility-gate-receipt.json`
- `answer-packet.json`
- `events.jsonl`
- `canonical-cli-run-receipt.json`

## Verify

独立复核 run root：

```powershell
python gsa.py verify --run-root .tmp/canonical-run
```

## 边界

- `run` 当前固定使用 fake/offline adapter；
- `--ask` 会冻结为 `task-contract.json`，并记录 source ledger 绝对路径；也可以用 `--task task.json` 复用既有合同；
- source gate 可以 `defer`，但不会自动执行下一轮检索；
- 没有 crawler、PDF parser、ToolBroker、真实 DeepSeek/Grok 请求或 EvaluationRunner；
- 这些命令证明 CLI 编排和机械门禁，不证明科学正确性。
