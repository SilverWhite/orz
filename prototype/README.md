# FEP Agent disposable prototype

This package is a development-only conformance-fixture suite. It records protocol
and provider/platform boundaries discovered before the upstream-first decision; it
is not a production runtime backlog. It implements:

- scenario export from corpus revision 4;
- safe copying of `scenario_only` fixture files;
- mechanical leak scanning with an explicit unreviewed semantic warning;
- immutable no-model smoke manifests;
- RFC 8785 / SHA-256 chained JSONL journals and replay verification;
- a narrow session semantic-validator spike for cross-record references,
  event sequencing, and terminal-event invariants already stated in protocol v0.1;
- a no-network DeepSeek adapter spike for V4 model/profile preflight,
  thinking-mode tool transcript continuity, SSE keep-alive parsing, and HTTP
  retry classification;
- a Windows-first, no-shell process-control spike with Job Object containment,
  bounded stdout/stderr capture, timeout cancellation, and a schema-validated
  terminal result;
- a no-model action-kernel integration spike that freezes a run manifest,
  journals one fixed local process action, materializes a protocol session,
  and verifies cross-file digests;
- a provider-neutral scripted streaming transport plus a two-turn DeepSeek V4
  thinking/tool-call loop whose ordinary artifacts retain only content and
  reasoning digests.

Except for the separately authorized two-phase command described below, the
fixture suite does **not** run a model. It does not score an Agent, create
evaluation/holdout data, provide a sandbox, or modify MAP/INDEX. A development
export may be `ready` with a leak warning because the semantic review is
intentionally not performed. Challenge warnings remain blocked/deferred.

Grok Build owns the production model transport, session, tool, permission and
sandbox layers. The transport, loopback, fake-provider, broker and approval-ledger
modules below are frozen disposable fixtures. They may prove an upstream gap, but
must not be expanded into a parallel general-purpose Agent stack. See
`../architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`.

From the repository `prototype` directory:

```powershell
python -m fep_agent_proto.cli export `
  --case-id FEP-SYN-006 `
  --policy-mode development `
  --token-seed local-smoke-only `
  --output-dir ..\_prototype_smoke\export

python -m fep_agent_proto.cli journal-smoke `
  --export-root ..\_prototype_smoke\export `
  --output-dir ..\_prototype_smoke\run

python -m fep_agent_proto.cli replay `
  --run-manifest ..\_prototype_smoke\run\run-manifest.json `
  --journal ..\_prototype_smoke\run\events.jsonl

python -m fep_agent_proto.cli validate-session `
  --input .\path\to\session-record.json

python -m fep_agent_proto.cli validate-deepseek-profile `
  --input ..\runtime\examples\example-deepseek-adapter-profile.json `
  --messages .\path\to\normalized-messages.json

python -m fep_agent_proto.cli windows-process-smoke `
  --output-dir ..\_prototype_smoke\windows-process

python -m fep_agent_proto.cli action-kernel-smoke `
  --export-root ..\_prototype_smoke\export `
  --output-dir ..\_prototype_smoke\action-kernel

python -m fep_agent_proto.cli verify-action-kernel `
  --output-dir ..\_prototype_smoke\action-kernel

python -m fep_agent_proto.cli deepseek-model-loop-smoke `
  --export-root ..\_prototype_smoke\export `
  --profile ..\runtime\examples\example-deepseek-adapter-profile.json `
  --output-dir ..\_prototype_smoke\model-loop

python -m fep_agent_proto.cli deepseek-loopback-http-smoke `
  --export-root ..\_prototype_smoke\export `
  --profile ..\runtime\examples\example-deepseek-adapter-profile.json `
  --output-dir ..\_prototype_smoke\loopback-model

python -m fep_agent_proto.cli deepseek-external-readiness `
  --profile ..\runtime\examples\example-deepseek-adapter-profile.json

python -m fep_agent_proto.cli deepseek-brokered-fake-https-smoke `
  --export-root ..\_prototype_smoke\export `
  --profile ..\runtime\examples\example-deepseek-adapter-profile.json `
  --output-dir ..\_prototype_smoke\brokered-fake-model

python -m fep_agent_proto.cli deepseek-interactive-fake-https-smoke `
  --export-root ..\_prototype_smoke\export `
  --profile ..\runtime\examples\example-deepseek-adapter-profile.json `
  --output-dir ..\_prototype_smoke\interactive-fake-model

python -m fep_agent_proto.cli deepseek-real-development-plan `
  --output-dir ..\_prototype_smoke\real-development

python -m fep_agent_proto.cli deepseek-real-development-execute `
  --plan ..\_prototype_smoke\real-development\plan.json

python -m fep_agent_proto.cli verify-model-loop `
  --output-dir ..\_prototype_smoke\model-loop
```

Outputs are no-overwrite by default. Delete or choose a different smoke root
manually when intentionally starting a new probe.

The session validator is an implementation spike, not a scientific gate. It
checks only mechanically decidable protocol invariants and does not establish
task acceptance, evidence sufficiency, claim eligibility, or evaluation
readiness.

The DeepSeek spike does not read an API key or perform network requests. Its
provider-specific rules are documented in
`../architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`.

The Windows process smoke must run on Windows. It executes only the current
Python interpreter, never invokes a shell, and records digests and byte counts
instead of raw process output. It is a process-control probe, not an OS sandbox;
see `../architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md`.

The action-kernel CLI intentionally exposes no arbitrary command argument. Its
fixed smoke joins the existing manifest, journal, Windows runner, artifact, and
session contracts without claiming model, scientific, filesystem-sandbox, or
network-isolation readiness. See
`../architecture/ACTION_KERNEL_CONTRACT_v0.1.md`.

The DeepSeek model-loop smoke is fully scripted: it opens no socket, reads no
API key, executes only the fixed `mock_echo` fixture, and never persists raw
`reasoning_content`. See
`../architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md`.

The loopback HTTP smoke additionally exercises real HTTP/SSE framing but accepts
only a literal `127.0.0.1` endpoint created inside the smoke. On Windows it
persists the provider-private transcript with current-user DPAPI and verifies a
decrypting round trip; it still performs no external DeepSeek request. See
`../architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md`.

`deepseek-external-readiness` 始终输出 `ready_for_network=false` 并返回阻断码 2。它只检查固定
endpoint、TLS context、一次性 permit 和 Windows Credential Manager provider
的构造策略；真实 HTTPS transport 没有接入 CLI/model loop，测试使用 fake
connection，不产生 DNS、socket、凭据访问或 API 计费。参见
`../architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md`。

`deepseek-brokered-fake-https-smoke` 通过无 socket 的内建 provider 执行一次
503 retry 和两轮工具调用。每个实际 attempt 都生成新的 request-digest permit，
并把不含 prompt、raw reasoning 或 Authorization 的确认摘要写入 transport
metadata。它不读取 Windows Credential Manager，也不调用真实 DeepSeek。参见
`../architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`。

`deepseek-interactive-fake-https-smoke` 会为 503 retry 和两个模型 turn 分别要求
输入 `ALLOW-<摘要前12位>`，并把 allow/deny 写入 hash-chained
`network-approvals.jsonl`。摘要与提示走 stderr，JSON result 走 stdout。该 broker
在构造期被限制为内建 fake provider，不能访问真实 endpoint。参见
`../architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md`。

`deepseek-real-development-plan` 是纯离线第一阶段：生成固定请求的脱敏确认摘要，
不读取凭据、不解析 DNS、不建立 socket。`deepseek-real-development-execute` 只接受
同一目录中未变化的计划，要求现场输入 `ALLOW-<摘要前12位>`，然后最多发出一次
固定 `deepseek-v4-pro` 请求；没有 retry、工具、任意 prompt、任意 endpoint 或环境变量
credential 入口。API key 只能来自当前用户 Windows Credential Manager 的 Generic
Credential `FEP-Agent/DeepSeek`。结果仅保存内容/reasoning digest、usage、transport
metadata 与 approval ledger，不保存原始回复或 Authorization。它仍只是 development
transport conformance，不产生科学证据。

execute 在读取 credential 前还要求当前 Windows 短生命周期 CLI 进程成功启用并验证
WER `NOHEAP`，关闭 Python `faulthandler`，且 transport 固定不使用 proxy environment、
redirect 或 HTTP debug。成功和失败 terminal artifact 都会在写入前执行 bounded
common-secret-pattern scan；扫描不会为此读取真实 API key。该层缩小但不能消除
Python/HTTP/内核内存、pagefile、hibernation、外部 dump、管理员/debugger/malware 或
provider 认证层的暴露面。
