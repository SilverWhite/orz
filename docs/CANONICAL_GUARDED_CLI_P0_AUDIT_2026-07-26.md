# Canonical guarded CLI P0 audit（2026-07-26）

## 裁决

首个 canonical guarded CLI offline 纵向路径已接通。该路径把已有保障部件串成一个可运行、可验证的最小主流程：

```text
task/run intent
  -> immutable run manifest
  -> source visibility ledger
  -> source visibility gate receipt
  -> fake DeepSeek-shaped adapter boundary
  -> structured answer packet
  -> runtime JSONL journal
  -> independent verifier
```

本轮没有重新调用 DeepSeek、Grok 或任何真实模型，也没有读取 API key、发网络请求、执行工具、抓取网页或解析 PDF。
目的只是证明魔改版 CLI 的主路径可以强制经过 source gate、adapter boundary、answer packet 和 journal/verifier，
而不是证明模型能力或科学正确性。

## 新增文件

- `assurance/canonical-cli-answer-packet-v0.1.schema.json`：冻结 CLI 最终回答包，要求包含 source visibility summary、claim boundaries 和 next actions。
- `assurance/canonical-cli-run-receipt-v0.1.schema.json`：冻结 run receipt，绑定 manifest、source ledger、source gate receipt、answer packet 和 journal digest。
- `assurance/canonical_cli.py`：offline/fake P0 run 与 verifier 实现。
- `assurance/canonical_cli_main.py`：`run` / `verify` 两个命令入口。
- `assurance/tests/test_canonical_cli.py`：正例与篡改反例。

## 机械门禁

该 P0 路径固定以下顺序与边界：

1. `run-manifest.json` 先写入并通过 `runtime/run-manifest-v0.1.schema.json`；
2. source visibility ledger 复制进 run root；
3. source visibility gate receipt 由 ledger 机械重算；
4. `answer-packet.json` 绑定落盘 gate receipt SHA-256；
5. journal 中 `gate_decision` 必须先于 `model_request`；
6. fake adapter 的 `model_request` 与 `model_output` 都只记录 metadata，固定 `real_network_used=false`；
7. terminal event 必须唯一且位于最后；
8. verifier 独立重读 run root，检查 schema、digest、hash-chain、事件顺序和 gate/answer 绑定。

## 正反例

当前回归覆盖：

- 正例：canonical CLI run 写出全部 artifact，verify 结果与 run receipt 完全一致；
- CLI 正例：`run` 与 `verify` 子命令都输出 receipt；
- 篡改 source gate receipt 后，verifier fail closed；
- journal 中把 `model_request` 移到 `gate_decision` 前会被 hash-chain/order 检测阻断；
- run root 非空时拒绝覆盖。

## 保留缺口

- 没有真实 DeepSeek/Grok 请求、真实 credential、真实 transport 或费用调用；
- fake adapter 只证明 message/order 边界，不证明 provider 行为；
- 没有 crawler、PDF parser、论坛 thread 展开或外部检索器；
- 没有 ToolBroker、交互许可、sandbox 中任意用户命令或真实 workspace 写入；
- 没有 EvaluationRunner、scoring、holdout、人类 baseline、failure taxonomy 或统计/科学正确性判断；
- source visibility gate 可以 defer/downgrade claim，但不会自动决定下一轮抓取策略。

## 机械结果

- `assurance/tests/test_canonical_cli.py`：5/5；
- `python scripts/check_repository.py`：133 schemas、1 个 canonical CLI fixture、0 errors；
- 本轮不新增 LIF 科学 claim，不修改 LIF INDEX/MAP/R。
