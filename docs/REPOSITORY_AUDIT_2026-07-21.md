# 独立 CLI 仓库初始审计（2026-07-21）

状态：迁移后工程审计；不修改 protocol v0.1 的决策语义。

## 2026-07-21 upstream-first 补充裁决

后续官方上游复查确认 Grok Build 已原生提供 custom models、headless、sessions、通用 transport、
tools、permissions 与 sandbox。此前的 Python model/HTTP/approval 路线因此不再是生产实现计划，
而是冻结为 DeepSeek/Windows/LIF conformance fixtures。正式路线改为官方 Windows 预编译 Grok +
薄 LIF sidecar，详见
[`../architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`](../architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md)。

当前上游基线由 [`../upstream/grok-build.lock.json`](../upstream/grok-build.lock.json) 记录。机器上
现已将官方 stable `grok 0.2.106 (bde89716f6)` 安装到被 Git 忽略的项目内目录；文件长度、SHA-256、
PE header、Authenticode signer 与 `--version` 均和 lock 一致。本轮没有登录、读取凭据值、启动模型
session 或发起真实 API 请求。

## 审计契约

- MUST：确认 Git/文件状态、设计边界、schema/corpus/fixture 完整性、probe 可执行性与 CI 启动面。
- MUST：为下一阶段提供一个窄、可丢弃、可测试的 implementation spike，并覆盖本项目实际使用的 Windows 工程面。
- MUST NOT：创建或宣称真实 evaluation/holdout，不调用模型，不提升任何 FEP/LIF claim，不修改 reason code、gate decision 或状态机枚举语义。
- SOURCE OF TRUTH：本仓库 ADR、protocol、runtime/evaluation/regression 契约、schema、fixture 和 Python probe；旧研究工作区 locator 当前不可用。
- ACCEPTANCE：仓库机械审计通过；Windows/Ubuntu 的 Python 3.11/3.12 CI 安装路径明确；现有 probe 与新 spike 单元测试通过。

## Source-grounded 状态

- Git 是 unborn `main`，初审时没有 commit，现有内容全部未跟踪；`origin` 指向 `SilverWhite/CLI.git`，本地显示远端 `main` 不存在。
- 仓库当前包含 21 个 JSON Schema、40 个 development/challenge seed cases、10 个 fixture manifests、1 个上游锁、1 个 Grok custom-model 配置夹具和 1 个 observed-plan 示例；真实 evaluation/holdout 数为 0。
- 迁移前路由依赖的 `LIF_CURRENT_INDEX.md`、`fep_env_research.md`、`self_check_protocol.md` 及 MAP6 没有进入本仓库或父目录。现有引用只能作为 provenance locator，不能作为本轮已读取证据。
- 当前 44 个 probe 测试从 `prototype/` 运行时通过；从仓库根目录直接运行但未安装 package 时会因 import path 失败。CI 因此先执行 editable install，再从根目录运行测试。
- development export 在缺少独立语义复核时保持 `warn` 但可用于 smoke；challenge 会 `defer`；现有 probe 不创建或放行 evaluation/holdout。
- 本机 Git 全局启用 `core.autocrlf=true`，而 fixture digest 按原始字节计算；仓库用 `.gitattributes` 将哈希相关文本固定为 LF，避免首次提交后的 checkout 改变 SHA-256。

## 设计与实现边界审计

已确认的稳定边界：

- 模型输出只产生提案；task/action/evidence/claim 状态互相分离。
- 当前来源优先于历史案例；case retrieval 必须晚于 precommitment。
- 工具执行成功不等于产物、证据或 claim 合格。
- evaluation/holdout 的 oracle 隔离、语义 leak review、人类 baseline 与阈值仍是未实现前置条件。

当前工程风险：

- 旧研究来源不在本仓库；这是已接受的独立项目边界。普通工程工作不依赖它们，具体 LIF claim-bearing 任务再跨目录读取并记录来源。
- `prototype` 仍通过仓库相对路径读取 protocol/runtime/regression 文件；它适合 editable development probe，不是可独立安装的生产 wheel。
- journal append 没有多进程文件锁；当前只支持单进程 smoke，不应被当作并发 runner 的持久化实现。
- leak scan 只有机械层，语义 reviewer 未实现；其 `warn` 不能升级为 evaluation clearance。
- model adapter、tool broker、OS sandbox、EvaluationRunner、密封维护域和真实 claim-strength gate 均未实现。

## 初始提交结构

建议保持四笔可独立审阅的初始提交，不按此前每次 probe 的开发顺序制造十余笔历史：

1. `docs(protocol): import v0.1 evidence-constrained CLI baseline`
   - ADR、architecture、protocol、evaluation、runtime、regression、fixtures 与迁移 provenance 注记。
2. `test(ci): add repository integrity checks and Python matrix`
   - `.gitattributes`、`scripts/check_repository.py`、`.github/workflows/ci.yml` 与现有 contract-probe tests。
3. `test(conformance): preserve LIF DeepSeek and Windows boundary probes`
   - 将现有 session、process、DeepSeek、redaction、journal 与 fake transport 代码整体标记为 disposable conformance fixtures，而非生产 runtime 历史。
4. `feat(integration): lock Grok upstream and add DeepSeek discovery config`
   - 上游 commit/`SOURCE_REV`/ownership lock、upstream-first 设计、无凭据 custom-model 模板及对应机械检查。

工作树按上述职责边界准备；提交前必须保持 binary 与 `.observed-runs/` 产物在 Git 忽略范围内，并重新运行
仓库检查、44 个 probe 测试和 secret-like pattern 扫描。push 不属于本轮默认范围。

## 已完成并冻结的历史 implementation spikes

最初选择的 session semantic validator 已完成。它实现 protocol v0.1 已明确要求、且无需科学裁决即可机械判断的最小集合：

- record ID 唯一性与跨记录引用完整性；
- source-of-truth/source-grounded 读取状态；
- event sequence 全局不重复，且在每个 action 内单调递增；
- terminal action 恰有一个、且为该 action 最后一个 terminal event；
- artifact producer、evidence、claim、gate subject 与 source 引用存在；
- case retrieval 存在时 precommitment 及时间顺序成立。

明确不实现：task acceptance 的自然语言判定、evidence sufficiency、最弱维度 claim-strength 计算、科学有效性、oracle 语义隔离签发、模型调用或生产 runner。

Windows 后续路径已调整为轻量增量路线：先保持 Python/Win32 spike 可执行、可回归，再只迁移已稳定的接口到最小 Rust 控制面；不把完整 Grok Build workspace 或重建 Linux 环境作为当前开发前置。

当前 action-kernel spike 已把上述机械边界接通，但仍不开放任意 CLI command。provider-neutral mock transport、loopback HTTP timeout/cancellation 与 Windows DPAPI private transcript 已进入同一 journal 路线。HTTPS/auth 安全骨架现已完成离线 fake-connection 验证：真实 endpoint 固定、API key 只能来自当前用户 Windows Credential Manager、每个请求需要短时一次性 digest permit，但没有联网 CLI，readiness 固定为 blocked。

model-loop 现已对每轮/每次 retry 传递 attempt context，并由 broker 为三个 fake HTTPS attempt 分别签发 request-digest permit；每份可审计摘要只保留披露类别、计数、限额与 digest。fake-only 终端 UI 现要求逐次输入 digest challenge，allow/deny 写入 append-only hash-chain ledger，并由 result verifier 与三个 attempt 交叉核对。

Grok Windows 预编译程序核验、zero-model dry-run、单轮 fake-provider，以及两轮 tool/reasoning
continuity 与 Job Object 接入已完成。下一优先级是 local trace/export、workspace delta、hash-chain bridge 和
显式 child-tree timeout fixture；不跳到真实 provider。只有请求形状、事件补全、进程树取消和 sealed-private
边界均通过，用户另行授权且
凭据安全置入后，才考虑一次最小真实 DeepSeek development probe；这不自动扩张为 evaluation、holdout 或
通用工具授权。
