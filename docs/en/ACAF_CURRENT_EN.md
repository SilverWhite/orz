# ACAF (Authenticated Control and Action Fabric) — Current Design (English distillation)

> LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.
> This is a **distillation of the current state**, not a full translation.
> Source: [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)
> (Chinese, the design authority; decision register ADR-0011 derives from it), updated with
> in-carrier changes through **v0.8.10** (0.8.2 signing-assembly fix; 0.8.7 keystore-root /
> signer-manifest protection faces). Terminology: [TRANSLATION_GLOSSARY.md](TRANSLATION_GLOSSARY.md).

## 0. One-paragraph summary

ACAF is orz's answer to prompt-injection-driven tool abuse: **every action that crosses a trust
boundary must carry a one-time HMAC-signed ticket issued by an independent signer process**
(`orz-signer`) whose keys the agent process never sees. Tickets are invisible to the model
(zero token burden), bound to the resolved real target (normalized argv / absolute path
digests, TOCTOU-checked), consumed atomically, and fully audited (`issued`/`consumed`/
`rejected` receipt events in the hash-chained journal). In the shipped carrier ACAF is
**fail-closed**: unconfigured, the process starts but every run is refused.

## 1. Threat model

**One-sentence definition** (from the source design §2.3):

> ACAF defends against: a prompt-injection-affected agent process (model, subagent, plugin)
> attempting to make any control event or external-effect action happen across a trust boundary
> without mechanical authorization from the independent signer.

**Defends against**:

| Attack | Mechanism |
|---|---|
| Malicious web page / PDF / search result injecting instructions that induce privileged actions | Actions require tickets; the executor verifies before executing |
| Forged neutral inquiry / control markers | Control events carry tickets; templates live inside the signer |
| Tampered inquiry counts, replayed old inquiries | Mechanical round counting + monotonic sequence + one-time nonce |
| Swapping action parameters, then replaying an old ticket | Verification target = the parsed real object (canonicalized argv / absolute-path digest) |
| Plugins / subagents / log files impersonating the control plane | Ticket signature + session/agent/chain binding |
| Crash recovery re-executing a one-time write | Atomic consumption + receipt chain; recovery never replays |
| Old tickets surviving a target revision | Tickets bind goal version + digest; a revision kills old tickets immediately |

**Explicitly out of scope** (boundary declaration): a compromised signer (the trust root
itself); same-user malicious processes / debug injection / memory reads (userland cannot
address these); administrator/kernel-level control; a malicious model service or man-in-the-
middle proxy (endpoint trust); and **whether an allowed command is semantically wise** — a
ticket proves provenance and authorization, not correctness.

## 2. Key invariants

1. **Invisible to the model, zero burden** — tickets and verification never enter model context.
2. **Every action carries a ticket** — all cross-boundary control events and external-effect
   actions; system bookkeeping writes (journal appends, blackboard, snapshots, `.gsa`
   structure) explicitly do **not** need tickets (avoids self-lock).
3. **Verification targets the parsed real object** — symlinks/junctions/reparse points/path
   traversal are refused (anti-TOCTOU).
4. **Atomic consumption + no replay after crash**.
5. **Independent signer** — the master key never enters the agent process; the interface
   accepts only enumerated actions, never arbitrary text.
6. **Fail-closed** — any verification failure refuses and writes a security event; no
   ticketless channel exists.
7. **Full audit** — every issue/consume/reject has a receipt in the event system.
8. **Shadow mode first** — differences logged before enforcement flips (historical discipline
   during rollout).
9. **Explicit modes** — modes are switched manually by the user only; the model cannot switch,
   exit, or widen them.

## 3. Mechanism

### 3.1 Keys

```text
K_install = installation-keystore master key (Windows: DPAPI-protected; Linux: plain file, 0600)
K_session = HKDF-SHA256(K_install, session_id || goal_digest || policy_digest || signer_revision)
```

`K_install` exists only in the signer process. `K_session` is derived by the signer and handed
via narrow IPC to the host's verification module (mechanical code). A changed `goal_digest` or
`policy_digest` re-derives `K_session`, instantly invalidating old tickets.

Platform note: on Windows `K_install` lives in the DPAPI-protected installation keystore; on
Linux the carrier uses the plain-file store (`file-0600-installation`, 0600 permissions) built
for eval containers. Any other store fails closed — the signer refuses to start.

### 3.2 Signer (independent process, narrow IPC)

Launched by the trusted launcher chain, which hands the signer a **manifest + binary
self-hash** (the manifest carries `signer_revision` and this binary's SHA-256; a mismatch or a
missing manifest exits non-zero, so the signer does not vouch for itself). Signed-manifest
verification with an independent release key is the **registered v2 upgrade path** — today the
trust anchor is the launch chain handing over the manifest, the same trust level as the host
binary itself. Narrow IPC is **JSON-lines over the signer's stdin/stdout**, one request / one
response (v1 transport; the process boundary is the ACL — only the launching host holds that
pipe). The interface is **enumerated actions only** (`SignOrientationV1`, `SignFileWriteV1`,
`SignCommandExecV1`, `SignNetworkV1`, `SignCredentialReadV1`, `SignGoalRevisionV1`,
`SignModeChangeV1`, …). Updates ride the manifest + monotonic `signer_revision` + version
whitelist + rollback protection. "Pin one hash forever" was explicitly vetoed — that would
make the signer unsafely upgradeable; the intended end state pins the signing public key and
the manifest chain once v2 signed-manifest verification lands.

### 3.3 Ticket shape and verification

One uniform mechanism (HMAC + binding fields); types differ only in `ticket_kind` and bound
fields: `ControlTicket` (control events), `ExecutionPermit` (controlled process execution),
`SandboxLease` (in-root writes under a mode), `PromotionPermit` (staged diff promoted back to
the host), `ModeChangeTicket`, `GoalRevisionTicket`.

```text
ControlTicket:
  schema_version
  ticket_kind              # orientation_v1 / disposition_v1 / close_v1 / goal_revision_v1 /
                           # file_write_v1 / command_exec_v1 / network_v1 / credential_read_v1 / ...
  ticket_id
  signer_revision + signer_measurement
  session_id + agent_id + activation_id
  goal_version + goal_digest
  policy_revision
  action_kind + canonical_arguments_sha256
  resolved_target_sha256
  capability_scope
  template_sha256          # control-event copy templates (control tickets only)
  sequence + nonce
  issued_at + expires_at
  user_confirmation_sha256
  hmac = HMAC-SHA256(K_session, canonical_payload)
```

The verifier checks: signature valid; template matches the built-in version (control tickets —
the signer accepts no arbitrary text); sequence monotonic and nonce unconsumed; all session/
agent/activation/goal/policy bindings match; `resolved_target_sha256` recomputes to the parsed
real object; not expired. Any failure ⇒ refuse, write `control_ticket_rejected`, pause
high-risk tools. (The v1 design's per-ticket `previous_receipt_sha256` chain was deliberately
omitted in implementation — chain semantics ride the journal's event hash chain + verifier
issued→consumed/rejected pairing; a ticket-level chain is a recorded v2 path.)

### 3.4 Two-layer gating

The permission bridge answers "**does policy allow this**" (today: yolo auto-approval by
default); the ticket answers "**is this specific instance authorized**". The executor checks
both; either failing refuses the action. **A verified ticket grants no tool permissions by
itself.**

### 3.5 Audit

Every ticket produces `control_ticket_issued` / `control_ticket_consumed` /
`control_ticket_rejected` events in the v0.2 event system (schema-first, then producer).
The journal hash chain preserves integrity; tickets add provenance authenticity.

## 4. The three modes (design; "full auto trust" was rejected)

| Mode | Default | Model behavior | Process creation | Network / credentials / host |
|---|---|---|---|---|
| **Normal** | on | Current behavior + action tickets wired progressively | via permission + permit | current semantics |
| **Auto-Staged No-Run** | off | Long-horizon autonomous iteration, writes only into a staging root | **impossible** (the sandbox has no process-creation capability — not just "no exec bit") | none |
| **Ephemeral Sandboxed Run** | off | Iterate + run + test inside a one-shot VM/container | only inside the disposable VM | none by default |

Modes are entered/exited by explicit user action only, via `ModeChangeTicket`; switching
invalidates all unconsumed tickets and leases and bumps `policy_revision`. Promotion of staged
work back to the host requires a `PromotionPermit` with a `base_workspace_digest` re-check
(background host changes ⇒ refuse, re-review or mechanical rebase). Sandbox backend decision:
Windows Sandbox (Hyper-V lightweight VM) preferred for Ephemeral Run; same-kernel paths
(AppContainer/Job Object) are **not** acceptable isolation for that mode while raw-TCP residual
and child-process inheritance gaps are open.

**Deployment reality (v0.8.10)**: Normal mode is what ships; **it currently has no human
approval step** — the permission bridge defaults to yolo auto-approval (writes / commands /
network approved by default; `--allow-write` etc. switch axes for benchmark runs). The human
approval leg was designed but never implemented; the approval surface is a recorded future
optional extension (the permission bridge's `PermitSource` enum keeps an observation slot for
it). Consequently the effective security narrative is "policy + ticket + audit": ACAF defends
against unauthorized tool calls, ticket replay, and post-hoc parameter tampering — **not**
against the model making well-formed but wrong decisions. Combined with write control
([WRITE_CONTROL_CURRENT_EN.md](WRITE_CONTROL_CURRENT_EN.md)), the security boundary =
mechanical-layer gates + catastrophic write backstop + journal audit. Slices 3–4 (No-Run /
Ephemeral modes) are designed, not implemented.

## 5. Carrier deployment (what shipping looks like)

- Three binaries: `orz`, `orz-signer` (independent signer), `orz-acaf-provision` (one-time
  initialization: creates the keystore and the signed signer manifest).
- One-time provisioning: `orz-acaf-provision <keystore-dir> <signer-manifest.json>`; startup
  wiring via `ORZ_ACAF_KEYSTORE` / `ORZ_ACAF_MANIFEST` / `ORZ_ACAF_BINARY`.
- **Fail-closed default**: unconfigured ⇒ the process runs but every run is refused.
  `ORZ_ACAF_FAIL_CLOSED=0` can temporarily disable the layer — not recommended for real work.
- Windows stores the DeepSeek API key in Credential Manager (target `orz-deepseek/agent`);
  an environment variable is the explicit exception channel (used on Linux).
- Interlock with write control: the keystore root and signer manifest are **protected
  host-state targets** (write-control rule 5, including the ancestor-sweep arm) — the trust
  anchors cannot be swept away by `rm -rf <install>`.
- Fix history in carriers: 0.8.2 fixed the signing assembly-point mismatch and added the
  signer-startup-failure bypass; 0.8.7 introduced the keystore-root / signer-manifest
  protection faces (with ancestor-arm faces on the L1/L2 copy layer); the "signer
  unreachable" handling was reworked (root cause: a keystore-root mismatch at the launcher,
  fixed; carrier/kernel-side verification rework remains registered as 0by S3/S4).

## 6. Design decisions (D-1…D-16, compressed)

Independent implementation outside the main phase plan (D-1); independent-process signer with
narrow IPC (D-2); no "fully trusted" mode — highest tier is Ephemeral Sandboxed Run (D-3);
three modes, all manual (D-4); process-creation capability decided per mode (D-5); threat
boundary per §1 (D-6); K_install/K_session split (D-7); system bookkeeping writes un-ticketed
(D-8); shadow-mode-first rollout (D-9); tickets invisible to the model (D-10); Windows Sandbox
as backend (D-11); `web_search` deliberately ticketless — no model-visible URL target (D-12;
the original rationale assumed provider-side search, since superseded by the local retrieval
lane — the ticketless conclusion still holds); subagent action tickets bound to real
`activation_id`
(D-13); fail-closed turns silent skips into hard refusals — missing target argument
(D-14), missing config/dependency fail-fast (D-15); a rejected goal revision performs no state
migration (D-16).

## 7. Revision history (one line per version)

- **2026-08-09**: design finalized (decision register D-1…D-11).
- **2026-08-12/13**: Slices 1–2 implemented (signer + control tickets; action tickets +
  executor verification), shadow mode first, then fail-closed mechanics (D-12…D-16).
- **2026-09-26**: Normal-mode copy corrected — the human-approval leg was never implemented;
  permission bridge defaults to yolo (source §5.2 correction note).
- **2026-09-28**: 0by opened — signer-unreachable root cause (launcher keystore-root mismatch)
  fixed in S1/S2.
- **0.8.2 / 0.8.7 / 0.8.10 carriers**: signing assembly fix; keystore-root / signer-manifest
  protection faces; in service today.

Evidence trail (Chinese): implementation audits under [`docs/audits/`](../audits/)
(`GAP_ACAF_SLICE1_IMPL_AUDIT_2026-08-12.md`, `GAP_ACAF_SLICE2A/2B`, `GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md`),
carrier records in batches 121/122/144/147/155.
