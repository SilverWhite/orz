# Prototype Boundary

Status: retired from the curated runtime-first chain

This directory is intentionally kept as a boundary marker. The earlier
`fep_agent_proto` implementation was useful as a disposable development probe,
but it duplicated too much generic agent runtime work: model loop, transport,
action kernel, approval ledger, loopback HTTP, fake providers, session handling,
and standalone CLI execution.

Those responsibilities now belong to a mature upstream agent runtime, with Grok
ACP / Grok Build as the first reference path. The curated chain keeps the
design lesson and the contract evidence, but does not keep the self-built
prototype package as active implementation.

Retained value from the retired prototype:

- It identified runtime responsibilities that should be upstream-owned.
- It informed the assurance schemas, adapter probes, and Windows process
  contracts now kept in `assurance/`, `runtime/`, `integration/grok/`, and
  `architecture/`.
- It remains available through previous git history if a narrow conformance
  fixture needs to be recovered deliberately.

Do not add new production runtime code here. New work should go through:

- `integration/grok/` for Grok/ACP adapter probes and conformance fixtures.
- `assurance/` for runtime-neutral gates, receipts, evidence, audit, and UI.
- `runtime/` for schemas and scenario/probe contracts.
- `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md` for ownership rules.
