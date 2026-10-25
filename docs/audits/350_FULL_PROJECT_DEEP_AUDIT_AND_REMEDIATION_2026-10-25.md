# 350 批：全项目深度审查与问题处置

> 日期：2026-10-25
> 范围：`D:\CLI` 父仓、`orz` 子模块、Python assurance 分发面、当前架构投影、用户面契约、治理台账与仓库门禁。
> 目标：同时审查设计合理性、实现合理性、设计—实现符合性，并处理本轮发现的全部可执行问题。
> 结论：发现的实现与文档缺陷均已处置；默认 YOLO 经用户确认是有意设计，不作回退，仅补异常配置的响亮失败。未执行提交、推送、载体重建、发行或真实付费 API 调用。

## 1. 总体结论

项目的主体架构仍然合理：自然语言设计、机器合约、运行实现、审计证据和历史材料已经有明确分层；Rust 主运行时、Python assurance 参考/验证层与 Web 工作台的职责边界基本成立。主要风险不在总体方向，而在若干“边界身份、安装态资源、当前文档和自动化证据”没有随实现演进同步。

本轮共归纳 11 个问题族：1 个发布门禁阻断、3 个高优先级实现缺陷、4 个符合性/证据缺口、3 个治理与权威边界问题。全部完成修复、重分类或明确风险接受。初始仓库检查为 33 errors；处置后只剩 `orz submodule working tree is dirty` 一项事务性错误，它准确反映本轮子模块修复尚未提交，而不是未处理的产品缺陷。该门不得通过放宽检查规避。

## 2. 审查方法与证据

- 设计面：README、ADR/ACAF 设计、`architecture/current/`、12 个用户面契约、协议目录与项目索引交叉核对。
- 实现面：Rust 会话生命周期/审批/CLI/Web 取消路径，Python 资源定位、构建分发、事件与评估消费者，仓库完整性检查器。
- 符合性面：从设计声明追到生产入口和直接测试；重点检查“文档声称已覆盖但测试实际未覆盖”的情形。
- 机械验证：仓库门禁、Python wheel 安装态 smoke、Python 合同测试、Rust 格式与 crate 测试、Web 前端 smoke、差分空白检查。

## 3. 问题与处置

| ID | 级别 | 问题 | 处置 | 状态 |
|---|---:|---|---|---|
| F-01 | P0 | 仓库门禁初始 33 errors：25 个 `orz` 清单摘要漂移、1 个 v0.2 正例 fixture 未映射、7 个台账瘦身问题 | 补 fixture 映射；收缩历史流水与 P1 路由且保持 token 一致；重新生成 1,494 项子模块源清单 | 已修复；提交前门禁仅保留 dirty-submodule 事务错误 |
| F-02 | P1 | 会话持久资源只用 8 位前缀，同前缀不同 full ID 可互相读取/覆盖 | 新增 `<s8>.owner.json`、跨进程 owner lock、旧证据迁移和 fail-closed 碰撞判定；new/load 在任何共享访问前 claim；load 复用预取得 full-ID lease；删除同步清 owner | 已修复 |
| F-03 | P1 | wheel 只含 Python/局部 package data，安装后 runtime/protocol/evaluation/regression 资源缺失 | 构建钩子把合同资源复制进 `assurance/*`；统一 source/wheel 双布局；所有生产消费者改走布局层；新增安装态独立 cwd smoke | 已修复 |
| F-04 | P1 | 默认 pytest 可收集真实 API 脚本和 `.tmp-*` 外部工作树，且 API 脚本吞异常导致假绿/意外费用 | `scripts/` 与 `.tmp-*` 从默认收集排除；脚本 import 无副作用；必须显式 `--live`；异常和无效 receipt 返回非零 | 已修复 |
| F-05 | P1 | `architecture/current` 的工具数、终态、Web/TUI 定位等叙述落后于实现 | 更新当前投影：12 个名义工具、默认检索下 11 个可见工具、`[RUN_END]`/`self_check`、退役工具和 Web 主界面边界 | 已修复 |
| F-06 | P1 | 默认 YOLO 与传统 confirm-first 安全叙述冲突 | 用户确认默认 YOLO 是有意设计；README、SECURITY、ACAF、索引和台账统一记录为“明确接受的高风险姿态”，保留 `--no-yolo` 退出路径 | 已接受设计，不改默认值 |
| F-07 | P1 | Web 30 秒取消兜底只有间接/错误映射证据 | 增加 fake-timer 直接回归：发送 cancel、精确 30 秒、超时清 running/busy、journal 边界提示、终态先到清 timer | 已修复 |
| F-08 | P2 | 索引中的审批状态与当前实现矛盾，曾把已接线 prompter 写成 stub | 将审批 prompter 状态改为 implemented；0dp 仅保留 S3 载体暂缓；默认 YOLO 不再列缺陷 | 已修复 |
| F-09 | P2 | S8 审批契约引用的测试不能证明 read-only 在 manager 前拒绝 | 改指直接测试 `read_only_policy_denies_non_read_before_manager`，保留组合顺序边界说明 | 已修复 |
| F-10 | P2 | 非 UTF-8 `ORZ_YOLO` 在 Unix 可被当成未设置，从而回到默认自动批准 | 区分 NotPresent 与 NotUnicode；未设置仍默认 `true`，非 UTF-8/非法值启动失败且装配点返回错误 | 已修复，不改变默认 YOLO |
| F-11 | P2 | 索引/审计/协议的权威措辞含混，协议文件缺少当前性与消费者说明 | 明确“ADR 管设计、代码/测试管事实、索引/审计只路由证据”；新增 `protocol/README.md`，逐类说明 Draft、非规范性、消费者和取代关系 | 已修复 |

## 4. 关键设计判断

### 4.1 默认 YOLO

默认 `true` 不是本轮需要纠正的缺陷，而是用户明确确认的产品姿态。它意味着在不可信输入场景下必须主动使用 `--no-yolo` 或外部沙箱；ACAF 提供真实性和动作约束，但不能替代用户意图授权。本轮只修复“配置损坏时静默落回自动批准”的鲁棒性问题。

### 4.2 会话身份

8 位短 ID 可以继续作为路径/UI 句柄，但不能单独充当持久资源所有权。full ID ownership sidecar + 独占锁把短句柄与真实身份分离，既保持兼容接口，也封闭碰撞读写和竞态窗口。旧数据仅在可由现有伴随证据唯一证明时迁移，否则拒绝。

### 4.3 Python 分发

源码树相对路径不等于安装态资源契约。资源现随 wheel 进入包内，生产代码只通过布局层解析；安装态 smoke 从临时 cwd 和解包 wheel 执行，避免测试意外借用源码树。

## 5. 验证结果

- `python scripts/check_repository.py`：初始 33 errors；修复后 1 error，且仅为 `orz submodule working tree is dirty`。fixture、台账、链接分类和 1,494 项清单摘要检查均无内容错误。
- `python scripts/check_python_distribution.py`：通过；wheel 解包安装态资源、protocol digest 和 `gsa --help` 均成功。
- `python -m pytest -q assurance/tests/test_distribution_safety.py assurance/tests/test_p0_contracts.py`：13 passed。
- `python -m pytest --collect-only -q`：2,111 tests collected；`scripts/` 与 `.tmp-*` 不再进入默认收集面。
- Python 受影响模块扩展回归（子任务执行）：129 passed。
- `cargo fmt --all -- --check`：通过。
- `cargo test -p orz-host --lib -- --test-threads=1`：436 passed，6 ignored，0 failed。
- `cargo test -p orz-bin`：各套件通过；主二进制 24 passed/12 ignored，signer 15 passed，ACAF E2E 24 passed，real_flag 2 passed，stdio E2E 1 passed。
- `node tests/frontend_smoke.mjs`：全部通过，含新增 16.11 取消兜底回归。
- 父仓与子模块 `git diff --check`：无内容错误；仅 `pytest.ini` 的工作区换行提示。

## 6. 未执行项与交付边界

- 未提交或推送。因此父仓完整性门正确拒绝 dirty submodule；提交 `orz` 后必须再次生成 `orz_source_manifest.sha256`，再提交父仓，门禁才能形成可复现的全绿提交态。
- 未重建 Windows/Linux 载体、未换装、未发布版本；0dp S3 继续按既有用户裁决暂缓。
- 未运行 live browser、真实 DeepSeek/API、付费或联网验证；相关测试保持 ignored/显式 `--live`。
- 本轮没有更改默认 YOLO，也没有把风险接受伪装成安全等价。

## 7. 最终判定

- 设计合理性：**通过，带明确风险接受**。总体分层与主路径合理；默认 YOLO 的风险现已准确写入权威边界。
- 实现合理性：**通过**。本轮发现的身份碰撞、安装态资源、测试副作用与配置异常路径均已修复并有直接回归。
- 设计—实现符合性：**通过（工作区态）**。当前投影、契约映射、索引状态和实现已对齐；提交态完整性仍须在正式提交子模块并重生成清单后核收。
