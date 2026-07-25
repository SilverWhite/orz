# GSA-CORE-001 通用科学只读纵向切片审计（2026-07-25）

## 裁决

`GSA-CORE-001` 已从“只有扩展名称和合同”推进为一个可执行、确定性、非 LIF 的初始纵向切片：

`来源文件 → 任务契约 → 研究设计 → 已完成动作 → 结果制品 → 直接比较证据 → claim 强度门禁 → 结构化报告`

本轮只关闭上述只读机械路径，不关闭完整科学保障核心。它不执行研究代码、不调用模型、不联网、不启动
子进程、不写入被审查来源，也不代替统计审查、复现实验、同行评审或正式 EvaluationRunner。

## 输入与输出合同

- `assurance/general-science-review-bundle-v0.1.schema.json` 冻结任务契约、source of truth、设计/动作
  文件引用、source/artifact digest、证据类型、JSON Pointer 比较、claim 类型及只读安全边界。
- `assurance/general-science-review-result-v0.1.schema.json` 冻结文件摘要、门禁结果、claim
  `allow/defer/block`、原因码、已验证比较、执行安全事实及限制。
- `assurance/general_science_review.py` 只读取 bundle 根目录内的常规文件；拒绝绝对路径、`..`、反斜杠、
  symlink/reparse point、目录逃逸、空文件、摘要不符、非 UTF-8 JSON、非有限数、悬空引用、
  重复 ID/路径、非 completed 动作和非 exactly-once terminal。

报告 ID 由 bundle 内容摘要确定，不含时钟或随机数；相同输入产生相同输出。
`python -m assurance.general_science_cli --bundle-root <path>` 提供直接入口；成功审查把 result 输出到
stdout，机械无效则把结构化错误输出到 stderr 并返回 exit code 2。claim 的 `defer/block` 是有效审查
结论，不与输入损坏混为一类。

## 非 LIF 独立样例

`assurance/fixtures/general_science/computational_decay/` 是一个独立的合成数值分析包：

- 问题：`dy/dt=-y, y(0)=1`；
- 方法：显式 Euler；
- 固定比较：`dt=0.05` 与 `dt=0.1` 在 `t=1` 的预计算绝对误差；
- 允许的唯一主张：在这两次 checked-in run 中，小步长的最终绝对误差更小；
- 明确不允许：收敛阶、因果、机制、普遍性、生产就绪或任何 LIF 结论。

设计、动作清单、结果和来源说明分别存放，bundle 只保存引用和 SHA-256，不把多文件回查伪装为单文件
自述。该 fixture 是 development/conformance 正例，不是 evaluation、holdout 或阈值校准题。

## Claim 门禁

- 无 supporting evidence：`block / MISSING_SUPPORT`；
- 存在 observed refuting evidence：`defer`；
- bridge hypothesis、用户未验证输入、agent inference 或 unchecked risk：`defer`；
- supporting evidence 不是 observed：`defer`；
- observation/association 只有在机械来源和直接比较条件满足时才可 `allow`；
- causal/mechanism/generality 即使声明 controlled intervention，也因本切片没有专用强主张 validator
  而固定 `defer / STRONG_CLAIM_VALIDATOR_UNAVAILABLE`；
- generality 还要求 coverage complete，否则额外 `COVERAGE_INCOMPLETE`。

因此本切片不能仅靠输入文件自称“controlled”就获得强主张。

## 正反回归

新增 7 个组合测试覆盖：

1. 非 LIF 正例、确定性输出和源文件前后摘要不变；
2. CLI 输出结构化报告且不改变被审查文件；
3. artifact 篡改导致 digest fail closed；
4. 路径逃逸与 JSON 非有限数 fail closed；
5. 未知 bridge support 只能 defer；
6. mechanism claim 因设计与 validator 不足 defer；
7. 无支持 claim block，重复 terminal 语义失败。

仓库检查器另行验证 bundle schema、ID/path 唯一性、所有引用文件存在及摘要一致，避免只依赖执行器自证。

提交前完整机械结果：81 schemas、prototype 50 tests、Grok integration 44 tests、runtime 6 tests、
assurance 44 tests，共 144 tests，repository error count 为 0。该结果只证明本仓库合同与 fixture
自洽，不是科学有效性或正式评测成绩。

## 保留缺口

- 研究设计和动作文件目前由窄语义解析器验证，尚未形成可扩展 validator registry；
- 没有统计假设、不确定性、效应量、功效、稳健性、多重比较、缺失数据或独立样本量审查；
- 没有执行研究动作或复现产物，generation provenance 只在摘要和交叉引用层得到验证；
- 没有跨 artifact 变换、表格/图像/PDF 语义抽取或单位/量纲检查；
- 没有 source mutation 的系统调用级证明；当前保证来自封闭的只读实现和前后摘要回归；
- 没有真实 runtime/tool adapter、模型、正式任务分区、oracle 隔离、人类 baseline 或评分阈值。

因此 `GSA-CORE-001` 状态应记为“初始只读纵向切片完成，完整核心仍开放”，不能记为全部关闭。
