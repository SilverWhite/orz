# evaluation/

评测协议、语料冻结与结果合约目录——**本目录不含 pytest 测试面**
（命名≠测试；RS-16 去误导注记，2026-09-19）。

- Python 测试面权威位置：[`assurance/tests/`](../assurance/tests/)（conformance
  ／评审器）与 [`runtime/tests/`](../runtime/tests/)（事件 schema／契约）。
- 本目录承载：评测协议（`SCORING_PROTOCOL`／`PARTITION_AND_ORACLE_ISOLATION`／
  `HUMAN_BASELINE_PROTOCOL`）、语料冻结清单（`corpus-freeze/`）、结果 schema
  （`evaluation-result-v0.1.schema.json`）与 V4.1 轮数据（`round-v41-k1/`）。
