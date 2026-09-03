# TER T1.10 W-F13a read_file 64KB 档实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.10——W-F13a read_file 单次有效返回放宽至 64KB
> 档：核对限制链（粗门 clamp / 行 limit / 50K 注入预算）后放宽；验收 =
> vm.js 量级文件 ≤2 次读完、单次注入不触发截断、大文件仍可结构化分段。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.6（W-F13）/ §10 S1-7；ORZ-LARGE-FILE-READ-CONTRACT
> (ADR-0010 §14.22) 演进。

## 1. 目标与验收

- 粗门（coarse gate）默认档 16 KiB → **64 KiB**（上限同步 64 KiB；
  下限 8 KiB 保留逃生阀）：≤64 KiB 文本单次返回全量内容。
- 限制链核对：64 KiB ASCII/代码文本 ≈ ≤16K token，低于单轮 50K token
  注入预算与既有 25K token / 1000 行读档 → 单次注入不触发截断。
- >64 KiB 文件仍返回有界 read-handle 信封（path/size/sha256/range/
  preview ≤4KiB/truncated/offset），offset/limit 结构化分段保持成立。
- orz-host 配置口子（`[toolset.read_file] coarse_gate_bytes`）与 env
  口子 clamp 同步到 8–64 KiB；描述文案同步（full/concise/handle 文档）。

## 2. 代码改动

- `grok_build/read_file/mod.rs`：
  - `READ_COARSE_GATE_DEFAULT` 16K→**64K**；`READ_COARSE_GATE_MAX`
    32K→**64K**（MIN 8K 不变）；参数/常量/函数文档与 DESCRIPTION_FULL
    更新为 64 KiB 档；
  - 测试尺寸随档位放大：`oversized_line…` 20K→70K、`single_line_payload…`
    49.5K→~70K、`skill_markdown_above_gate…` 19K→~74K；handle 断言
    `>16K`→`>64K`；param-override 文案 16K→64K；新增
    `default_64k_gate_reads_60k_file_in_one_call`（60K 全量 + 76K 信封）。
- `grok_build_concise/read_file.rs`：DESCRIPTION_CONCISE 档位文案更新。
- `grok_build_hashline/read_file.rs`：旧 16K 门槛用例文件放大至 8K 行
  （>64K），end_line 断言同步。
- `types/output.rs`：`ReadHandleEnvelope` 文档（64 KiB 档）。
- `orz-host/src/tools.rs`：`read_file_coarse_gate_from_config` clamp
  MAX 32K→64K + 注释/测试（65536/200000 → 64K 上限）。
- `orz-host/src/lib.rs`：默认门断言文案 16K→64K。

## 3. 验证证据

- `cargo test -p orz-tools --lib read_file`：**208 passed / 0 failed**
  （含新 64K 档验收用例与 hashline/concise/codex 全桶）。
- `cargo test -p orz-host read_file_coarse_gate_from_config_parses_and_clamps`：
  **1 passed**（8–64K clamp 断言）。
- `cargo fmt -p orz-tools -p orz-host -- --check`：净。
- orz 仓库 `git diff --check`：exit 0。

## 4. 边界声明

- 档位决策口径：默认=上限=64K，下限 8K 逃生阀不变；配置 >64K 一律 clamp
  到 64K（不提供“关掉粗门”的口子，fail-closed 纪律保持）。
- 行 limit（1000 行）语义保留：≤64K 但 >1000 行的文件仍走既有行档逻辑
  （结构化跳读成立）；W-F13b 输出检索对象（pattern/行区间/尾部）归
  T1.11。
- 中文/高密度文本的 64K 注入预算边界：64K UTF-8（≤3 字节/字）≈ ≤22K
  token，仍在 25K 读档与 50K 单轮预算内；超出由既有 token/行档截断。
