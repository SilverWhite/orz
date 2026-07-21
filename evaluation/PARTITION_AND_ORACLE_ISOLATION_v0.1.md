# Evaluation / Holdout 分区与 Oracle 隔离协议 v0.1

状态：设计草案；尚未创建真实 evaluation 或 holdout。

## 1. 目的

本协议定义如何证明案例在测试前未被被测 Agent、提示词作者或实现调试过程污染。它不把现有 development/challenge 案例重新命名为 evaluation，也不允许从当前案例库“晋升”出未见案例。

核心原则：**未见性是可审计的暴露历史，不是目录名。**

## 2. 分区语义

| 分区 | 用途 | 可用于设计/调试 | Oracle 可见范围 | 重复使用 |
|---|---|---:|---|---|
| teaching | 解释纪律与示例 | 是 | 可由教师按需展示 | 是 |
| development | 开发、错误分析、回归 | 是 | curator/reviewer | 是 |
| challenge | 反惯性、permission reversal | 是 | curator/reviewer | 是 |
| evaluation | 冻结后的比较评测 | 否 | sealed maintainer/adjudicator | 有限、须记录暴露 |
| holdout | 一次性接受门禁 | 否 | sealed maintainer/adjudicator | 原则上一次 |

凡被提示词作者、CLI 实现者或被测模型看过的案例，都不能再声称为真正未见的 evaluation/holdout。泄漏案例必须转入 development，并以新来源创建替代案例，而不是更换 ID 后继续使用。

## 3. 物理隔离

建议把资料分成三个存储域，而不只是在同一 YAML 中设置字段：

1. **公开设计域**：schema、评分器、development/challenge、公开 digest 和协议。
2. **密封维护域**：evaluation/holdout 的原始候选、oracle、source key 与身份映射；不挂载到被测 Agent 工作区。
3. **临时运行域**：只包含一次运行所需的 scenario export、允许工具和写入目录；运行结束后归档输出并销毁工作副本。

密封维护域应位于独立访问控制边界。单纯使用隐藏文件、Git 分支、压缩包或“请勿阅读”提示，不构成隔离。

## 4. 角色分离

- **corpus maintainer**：接触候选来源和 oracle，创建密封包；不得参与该版本系统的提示词调试。
- **runner**：只获得 scenario export 与运行配置；不得获得 oracle、源文件名或簇标签。
- **tested Agent**：只获得 `scenario_only` 内容及显式允许的工具。
- **reviewer A/B**：独立评审匿名输出；评审时不知道系统身份和另一人的判断。
- **adjudicator**：仅在 A/B 分歧后接触 rubric、匿名输出和必要 oracle；不得反向改写运行输出。

小团队无法完全角色分离时，必须在报告中降级为“受限评测”，不能用 holdout 语言包装。

## 5. 包结构与 digest

每个冻结分区至少有四个逻辑对象：

- `scenario_bundle`：被测 Agent 唯一可见的案例材料；不得含 oracle、历史纠正结论或可反推出答案的路径名。
- `oracle_bundle`：期望 gate/state、允许与禁止 claim、评分锚点。
- `identity_map`：匿名 case token 与内部 case ID/来源的映射。
- `partition_manifest`：分区状态、计数、上述对象 digest、污染事件与暴露策略。

冻结前公开承诺 `scenario_bundle`、`oracle_bundle` 和评分协议的 SHA-256 digest。公开的是 digest，不是密封内容。scenario 与 oracle 必须分别哈希，避免用重新打包掩盖单侧修改。

## 6. 生命周期

```text
candidate -> sealed -> active -> retired
                 |         |
                 +-------> contaminated -> development
```

- `candidate`：可编辑，不具有未见性声明。
- `sealed`：内容、评分协议和 digest 均冻结，尚未运行。
- `active`：允许按预注册计划运行。
- `retired`：完成预定用途，保留审计记录。
- `contaminated`：发生未授权暴露、提示词针对性调参或 oracle 泄漏；立即停止未见性声明。

任何状态变化都追加事件，不覆盖历史。

## 7. 暴露与污染规则

以下任一事件构成污染：

- scenario 或 oracle 进入系统提示词、检索索引、模型上下文或调试日志；
- 实现者依据具体 evaluation/holdout 失败逐案修改提示词、reason code 或 gate；
- reviewer-only 历史纠正摘录进入 tested-Agent context；
- case ID、簇名、源文件名或配对关系足以泄露预期结论；
- 运行后修改 oracle、rubric 或分母而未废弃原评测。

污染后必须记录时间、对象、暴露对象、影响范围和处置。受影响案例转入 development；若无法确定影响范围，整包降级。

## 8. 场景导出门禁

运行前，导出器必须机械确认：

- 仅复制 `scenario_only` fixture；拒绝 `reviewer_only`；
- 删除 corpus 中的 `oracle`、`classification`、`countercase_ids`、源路径和 curation fixture 引用；
- 使用不含簇含义的随机 case token；
- 生成导出文件清单与 digest；
- 在空白临时目录中完成一次 leak scan；
- 固定 system prompt、工具清单、预算、超时、随机性和模型版本。

`oracle_fields_present: false` 只是结构检查，不足以证明无答案泄漏；还必须检查自然语言语义。

## 9. 评测构成最低要求

未来创建 evaluation/holdout 时，应由未参与当前案例编写的人从新来源构造，并至少覆盖：

- 每个高频错误簇至少一个 permission-reversal；
- root-cause contrast 与 mode-boundary 案例；
- 相似表面但不同 gate 结论的配对案例；
- 负例与“可以继续”的正例，避免只学习阻止动作；
- 不同文档时期、运行配置和错误表述，降低历史措辞记忆。

具体通过阈值必须在双人盲审 baseline 后确定，不能由当前 development 成绩倒推。

## 10. 当前状态

当前仓库中的 40 个案例均为 development/challenge seed corpus。5 份历史 excerpt fixture 是 `reviewer_only` 的 curation provenance，不是可见场景，也不提高未见评测覆盖。真实 evaluation/holdout、密封存储和 baseline 结果仍为零。
