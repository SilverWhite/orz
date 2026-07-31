# D3.22 Finding ↔ Permission Separation — Design Decision

Date: 2026-07-31
Status: ruled — thin architectural invariant layer, not a permission engine

## 1. Source

From `architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2` §4.3, informed by the
[Goose security inspector](https://github.com/block/goose/blob/65e1e3d508b9f85c53998564db04708c90031881/crates/goose/src/security/security_inspector.rs)
pattern:

> scanner outputs finding ID, evidence, and confidence; permission independently
> records allow/deny/ask and rationale. Heuristic findings must not masquerade as
> security proofs, and permissions must not erase findings.

## 2. Problem

Two data types with fundamentally different lifecycles are at risk of conflation:

| Type | Lifecycle | Owner |
|---|---|---|
| **Finding** | Immutable evidence — once a scanner detects something, the finding persists as an audit artifact | Assurance (this repo) |
| **Permission** | Mutable decision — allow/deny/ask, may change per-session or per-user | Grok (runtime-owned) |

Without code-level enforcement:

- A `LeakFinding(severity=CRITICAL)` could be produced by a scanner, but the
  permission gate could `allow` the same tool call without any record of having
  considered the finding — erasing the finding from the decision trail.
- A permission `allow` could be misread as "no findings existed" when findings
  were simply never checked.
- A finding's severity could be misinterpreted as a permission recommendation
  (e.g. "CRITICAL finding = must block"), conflating evidence with decision.

## 3. Design Constraints

From the Runtime-First Graft Decision (2026-07-30):

- **Grok owns** the permission surface: tool registry, permission request/response
  lifecycle, sandbox gating.
- **Assurance owns** findings: scanners, validators, evidence collection.
- The adapter is a **thin observer** — it must not inject findings into the ACP
  stream or override Grok's permission decisions.

## 4. Decision

### 4.1 What to build

A **thin architectural invariant layer** that enforces separation without
rebuilding the permission system:

1. **Unified `Finding` schema** (`finding-v0.1.schema.json`)
   - Immutable, content-addressed by SHA-256.
   - All scanners (`LeakScanner`, `credential_scrub`, `validator_bridge`) emit
     this unified type.
   - Fields: `finding_id`, `category`, `severity`, `source_scanner`, `location`,
     `evidence` (never raw content), `snippet_hash`, `created_at`.
   - Does NOT contain: permission recommendation, block/allow signal.

2. **Independent `PermissionRecord` schema** (`permission-record-v0.1.schema.json`)
   - Records a single permission decision.
   - Fields: `record_id`, `decision` (allow_once/cancelled/denied), `authority`
     (user/adapter/policy), `tool_name`, `referenced_finding_ids` (SHA-256
     references to findings considered — never embeds finding content),
     `basis` (free-text rationale), `created_at`.
   - Does NOT contain: finding details, severity, evidence.

3. **`FindingRegistry`** — in-memory registry scoped to a single adapter session.
   - `register(finding)` → stores finding.
   - `list_active()` → returns all findings for the current session.
   - Thread-safe (read-heavy, write-rare).

4. **ACP integration point** — in `run_grok_acp_once()`, when a permission
   decision is made (user or auto), construct a `PermissionRecord` that
   references any findings registered during the session. Write the record
   alongside the existing receipt artifacts.

### 4.2 What NOT to build

- **Finding→permission workflow engine** — Grok owns the permission surface.
  The assurance layer observes and records; it does not gate.
- **ACP stream injection** — the adapter must not insert findings into the
  ACP JSON-RPC message stream. Findings are an assurance-layer artifact.
- **Automatic permission gating based on findings** — the adapter does not
  override Grok's or the user's permission decisions based on scanner output.
  The invariant is about audit trail integrity, not automated enforcement.
- **Cross-session finding correlation** — findings are session-scoped.
  Cross-session pattern analysis is a future concern.

### 4.3 Invariants enforced by construction

1. `Finding` has no `recommended_action` or `should_block` field.
2. `PermissionRecord` has no `severity` or `evidence` field.
3. `PermissionRecord.referenced_finding_ids` is a list of SHA-256 hex strings —
   it can reference findings but cannot embed or override them.
4. A finding, once registered, is never mutated or deleted for the session
   lifetime.

### 4.4 Rationale for "thin layer" rather than "full engine"

- Goose's security inspector is part of Goose's **own** tool execution pipeline —
  Goose owns both scanning and gating. In our architecture, Grok owns gating.
- Building a full finding→permission engine would duplicate Grok's permission
  surface and violate the Runtime-First graft boundary.
- The thin layer achieves the essential invariant (findings ≠ permissions,
  permissions can't erase findings) without overreaching into Grok's domain.
- Future: if Grok exposes a pre-tool-call hook point, the finding registry
  could feed into it without architectural changes.

## 5. Implementation Scope

| Component | Lines (est.) | Description |
|---|---|---|
| `finding-v0.1.schema.json` | ~50 | Unified Finding schema |
| `permission-record-v0.1.schema.json` | ~45 | Dissociated PermissionRecord schema |
| `assurance/finding_registry.py` | ~200 | `Finding`, `FindingRegistry`, `PermissionRecord`, `record_permission_decision()` |
| `assurance/grok_runtime_adapter.py` | +15 | Integration: ACP permission decision → record |
| `assurance/tests/test_finding_registry.py` | ~250 | ~12 tests |
| `CLI_PROJECT_INDEX.md` | update | Mark D3.22 closed |

Total: ~560 lines, ~12 tests.

## 6. Verification

- `test_finding_has_no_permission_fields` — schema enforces absence of decision fields
- `test_permission_record_has_no_finding_content` — schema enforces absence of evidence fields
- `test_registry_isolation` — two registries don't share state
- `test_finding_immutable_after_registration` — registered findings can't be mutated
- `test_permission_record_references_finding_by_id` — reference via SHA-256, not embedding
- `test_acp_integration_produces_permission_record` — end-to-end: decision → record written

## 7. Relationship to Other Components

- **P3 (Instruction Authority)**: `PermissionRecord.authority` documents who made
  the decision — complements P3's provenance chain.
- **P4 (Audit & Recovery)**: `PermissionRecord` is an audit artifact; its
  `referenced_finding_ids` can be cross-referenced in audit seals.
- **D1.13 (Interactive Permission Bridge)**: TUI user decisions produce
  `PermissionRecord(authority="user")`; auto-decisions produce
  `PermissionRecord(authority="adapter")`.
- **D2.15 (EvidenceKernel)**: Finding is an evidence artifact; PermissionRecord
  is an action artifact. The finding→evidence→claim chain remains separate from
  the permission decision chain.
