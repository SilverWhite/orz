> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# Interactive Network Approval Ledger 契约 v0.1

状态：development-only、fake-provider-only；实现逐 attempt 终端确认与 append-only ledger，不授权真实 DeepSeek 网络。

## 1. 交互边界

`deepseek-interactive-fake-https-smoke` 对每个 HTTP attempt 分别显示：

- `FAKE PROVIDER DRY RUN — no network, no billing`；
- turn/attempt 及 retry previous status；
- 固定 endpoint 与 model；
- request byte count 和 SHA-256；
- message/tool/private-reasoning 数量；
- 若真实发送将发生的内容、工具定义和 provider-private reasoning 披露类别；
- confirmation summary SHA-256。

UI 不显示 prompt、tool arguments、raw reasoning 或 Authorization。JSON 结果继续写 stdout，交互摘要和提示只写 stderr，避免破坏机器输出。

每次必须输入：

```text
ALLOW-<confirmation-summary-sha256 的前 12 位大写>
```

空输入、错误 token、EOF 或 Ctrl+C 都按 deny 处理。没有 `--yes`、session-wide allow、retry-wide allow 或自动接受真实网络的参数。

## 2. Fake-only 强制

`InteractivePermitBroker.fake_provider_only=true`。`BrokeredDeepSeekHttpsTransport` 在构造时确认 connection factory 必须是内建 `InProcessFakeDeepSeekConnectionFactory`；缺少该 marker 或使用默认真实 factory 会在读取凭据或构造连接前失败。

因此当前交互命令只能练习确认流程，不能访问 Credential Manager、DNS 或 DeepSeek。

## 3. Approval ledger

固定文件：

```text
network-approvals.jsonl
```

每条 `network_approval_decision` 记录：

- 单调 sequence 与 UTC 时间；
- previous event SHA-256；
- provider/endpoint、turn/attempt；
- request body 与 confirmation summary digest；
- allow/deny；
- `local-interactive-user` authority label；
- `typed-summary-digest` method；
- raw content、reasoning、Authorization 和 confirmation token 均未记录；
- 当前 event canonical digest。

写入采用单 writer append、flush 和 `fsync`。ledger 不允许复用已有路径；崩溃导致的截断行会使 verifier fail closed。本阶段没有跨进程文件锁，不能作为并发 approval service。

authority label 只描述交互入口，不证明操作系统用户身份、知情同意或批准质量。hash chain 证明本地文件内部一致性，不是数字签名或不可否认审计。

## 4. Model-loop artifact 与 verifier

成功交互 smoke 把 ledger 注册为 `model-loop-result` artifact。verifier 同时检查：

1. 文件 digest；
2. event schema、sequence、previous hash 和 event digest；
3. event 数量等于 HTTP attempt 数量；
4. 成功 run 的所有 decision 都是 allow；
5. 每条 event 的 turn/attempt/request digest 与对应 transport confirmation summary 一致；
6. confirmation summary digest 与 transport metadata 一致；
7. 未注册 ledger 或 ledger 篡改都会使 result verification 失败。

deny 会先写 ledger，再拒绝 permit；model loop 进入既有 `_INCOMPLETE.json`/`run_failed` 路径，不会连接 fake provider，更不会连接真实网络。

## 5. 尚未实现

- 真实网络 permit issuer；
- Windows 用户身份签名或 WebAuthn/硬件确认；
- ledger 数字签名、跨进程锁、集中审计或安全归档；
- Credential Manager provisioning UI；
- provider `/models` capability discovery；
- 真实 DeepSeek 请求、计费、evaluation 或 holdout。
