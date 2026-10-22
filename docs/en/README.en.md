**[English](README.en.md) ｜ [中文](../../README.md)**

# orz — English README (expanded edition)

> LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.
> Source: [`/README.md`](../../README.md) (Chinese, authoritative), v0.9.8 face, 2026-10-22.
> **Expanded edition** (user adjudication 2026-10-02): the two editions mirror each other in
> content; this one is expanded in *role*, not in coverage — it doubles as the English-language
> umbrella stand-in for the project's design authorities
> ([`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) and the `docs/`
> design documents), which are Chinese and are not planned for translation, so English readers
> can follow the design without the Chinese originals.
> Terminology follows the [translation glossary](TRANSLATION_GLOSSARY.md).

## Quick Start

### Prerequisites

- Windows x86_64 or Linux x86_64;
- A DeepSeek API key.

(Note: Linux currently ships build artifacts only, with no dedicated UX adaptation or optimization; other vendors' models and APIs are not yet supported.)

The release package contains three programs: `orz` (main program), `orz-signer` (security signer), and `orz-acaf-provision` (one-time initialization tool); they must be placed in the same directory. The Web workbench (since 0.6.13) embeds its static frontend inside the `orz` binary; `orz web` listens on loopback only and requires the token printed at startup.

## Setup

Configure Windows and Linux as follows:

| Step | Windows (PowerShell) | Linux (sh) |
|---|---|---|
| 1. Unpack | Put `orz.exe`, `orz-signer.exe`, `orz-acaf-provision.exe` into one directory (e.g. `C:\orz`). | `mkdir -p ~/orz && cd ~/orz`<br>`tar -xzf orz-0.9.8-linux-x86_64.tar.gz`<br>`chmod +x orz orz-signer orz-acaf-provision` |
| 2. Configure the API key | Store it in Windows Credential Manager (Generic, target name `orz-deepseek/agent`; once only):<br>`cmdkey /generic:orz-deepseek/agent /user:agent /pass:YOUR_DEEPSEEK_API_KEY` | Use an environment variable (the explicit exception to the Windows Credential Manager channel):<br>`export ORZ_DEEPSEEK_API_KEY=YOUR_DEEPSEEK_API_KEY` |
| 3. Initialize security signing (one-time) | `.\orz-acaf-provision.exe "$env:USERPROFILE\.orz-acaf\keystore" "$env:USERPROFILE\.orz-acaf\signer-manifest.json"`<br><br>ACAF is fail-closed by default: unconfigured, it refuses to run (the process starts; runs are refused). | `./orz-acaf-provision "$HOME/.orz-acaf/keystore" "$HOME/.orz-acaf/signer-manifest.json"` |
| 4. Set startup environment | `$env:ORZ_ACAF_KEYSTORE = "$env:USERPROFILE\.orz-acaf\keystore"`<br>`$env:ORZ_ACAF_MANIFEST = "$env:USERPROFILE\.orz-acaf\signer-manifest.json"`<br>`$env:ORZ_ACAF_BINARY = "C:\orz\orz-signer.exe"` | `export ORZ_ACAF_KEYSTORE="$HOME/.orz-acaf/keystore"`<br>`export ORZ_ACAF_MANIFEST="$HOME/.orz-acaf/signer-manifest.json"`<br>`export ORZ_ACAF_BINARY="$HOME/orz/orz-signer"` |
| 5. Run | `.\orz.exe` (interactive TUI)<br>`.\orz.exe -p "your task" --real` (headless)<br>`.\orz.exe web` (Web workbench; prints the local token-bearing URL) | `./orz` (interactive TUI)<br>`./orz -p "your task" --real` (headless)<br>`./orz web` (Web workbench; prints the local token-bearing URL) |

Release notes and integrity verification live on GitHub Releases (latest [v0.9.8](https://github.com/SilverWhite/orz/releases/tag/v0.9.8), dual-platform packages + `SHA256SUMS`); the 0.1.0–0.5.1 trial packages are under [`releases/`](../../releases/).

### Common entry points

| Scenario | Command |
|---|---|
| Interactive TUI | `orz` |
| Web workbench (local loopback; prints token-bearing URL at startup) | `orz web` |
| Headless execution | `orz -p "<task>" --real` |
| Plan mode (record a plan before executing) | `orz --plan -p "<task>" --real` |
| ACP stdio server | `orz --stdio` |
| Read-only journal replay | `orz --replay <events.jsonl>` |
| Offline trial run (no credentials) | `orz --fake-provider -p "hello"` |
| Web retrieval switch | On by default; disable with `ORZ_RETRIEVAL_ENABLED=0` (`--retrieval-enabled` remains as a compatibility explicit-on; the old `--retrieval-mode` is deprecated and parsed only for compatibility) |

Web retrieval is **on by default** (default-on since batch 286, ADR-0010 §14.85); for deterministic offline work or control runs, disable it explicitly with `ORZ_RETRIEVAL_ENABLED=0` (legacy `--retrieval-mode off` is still mapped). When enabled, retrieval is executed by an external retrieval subagent: the local browser channel is preferred (engine SERP, Google primary, Bing fallback); if the browser is unavailable the model may switch on its own to the native web retrieval channel; launch results and lane switches are recorded in the event chain (`browser_launch_result`).

Permission switches for headless and batch scenarios:

- `--allow-write`: allow modifying local files; shell and network are still denied by default.
- `--allow-shell` / `--allow-network`: open the shell and network axes for evaluation/batch scenarios; must be combined with `--allow-write`, otherwise the process exits with an error.
- `--max-wallclock <seconds>`: whole-round wall-clock cap; on timeout the run ends with a `run_invalidated` terminal state — no process-level hard kill.

Common tuning environment variables: `ORZ_STALL_TIMEOUT` (stall watchdog, default 360 s) and `ORZ_TOOL_TIMEOUT_SECS` (per-tool timeout, default 300 s). `ORZ_ACAF_FAIL_CLOSED=0` can temporarily disable the security layer, but is not recommended for real work.

## What orz Is

orz is a **local-first**, **assurance-first** terminal AI coding agent/harness that works **directly on the real machine, with no sandbox**, and was built using AI coding throughout.

The control plane, the agent loop, and the assurance system are original work; beyond that, the framework directly reuses some mature tooling and workspace components from [`grok-build`](https://github.com/xai-org/grok-build), and borrows the code design language of [`codex`](https://github.com/openai/codex), while also using and adapting its command review. The execution-side service call shape borrows heavily from [`Home Assistant`](https://github.com/home-assistant) (`domain.service + target + data`), with smaller borrowings from [`deepseek-harness`](https://github.com/deepseek-ai/deepseek-harness) and other mature products. The Web workbench's UI style and form come from the repo's three UI design drafts, with looks lifted directly from the retro desktop themes [`98.css`](https://github.com/jdan/98.css) and [`XP.css`](https://github.com/botoxparty/XP.css); Markdown rendering uses [`marked`](https://github.com/markedjs/marked).

The architecture falls into two large blocks plus two small ones: the large blocks are the **agent layer** and the **mechanical layer**; the small ones are the **blackboard** (the core state panel) and the plug-in **temporal/activity-domain component**.

### Agent layer

- **Main agent**: the sole task driver. Its system prompt is near-zero, and it faces a production-assembled, fixed 12-tool surface — eight retained tools `read_file`/`grep`/`search_replace`/`run_terminal_cmd`/`web_search`/`web_fetch`/`get_task_output`/`kill_task`, plus `blackboard_read`, `blackboard_write` (writes to the blackboard plan/notes/findings areas, ≤8K per entry), `context_manage` (single tool with mode: compress/clear/clear_all), and `self_check`; `context_compress` is retired (0cz option A; the compression channel is inherited by `context_manage mode=compress`). With the retrieval gate on by default, `web_fetch` leaves the main surface under the single-entry projection (11 visible); with retrieval explicitly off, all 12 are visible. Counts are sourced from the production assembly in `orz-loop/src/controller.rs` (the retained list in `orz-host/src/tools.rs` is 8 plus controller additions). The default model is registered as DeepSeek v4 flash (thinking at max by default). (DeepSeek currently auto-routes to deepseek v4.1 flash, so the interface name is left unchanged for now.) A task ends with an explicit `[RUN_END]` terminal declaration (when this run has execution facts; at most two mechanical notices, then fail-open); before the final answer the model may actively call `self_check` for the three-question counterexample check (PULL; no automatic gate). `submit` has been retired from the model tool face; delivery status is shown on demand by the Web/TUI.
- **External retrieval subagent**: web retrieval runs through an external retrieval subagent; the retrieval enablement gate is on by default — explicitly disabling it closes the whole family (`ORZ_RETRIEVAL_ENABLED=0`). The main surface's `web_search` stays a single dispatch entry point while execution lives in the subagent (global concurrency 1). When enabled, the subagent's tool surface always registers both local-browser and native-web retrieval families (with static annotations carrying lane names and preference order; local browser first), and lane switching is the model's own choice: the local browser channel drives engine SERPs (Google primary, Bing fallback, DDG last resort; humanized typing = per-character keystrokes + pre-submit pauses + Enter, invisible to the model), while the HTTP segmented lane uses a self-built engine chain (default direct `360search,baidu` for networks without a VPN; the proxy chain adds `duckduckgo`; TLS/HTTP2 fingerprint cloaking; result-quality judgment is left to the retrieval subagent). Browser launch availability is recorded as a factual event (`browser_launch_result`) in the event chain. An internal retrieval lane remains in the design; its triggering tool is currently sealed off.
- **Sessions and plans**: the TUI does not resume across processes — the interactive session surface is read-only review/`--replay` (the code says never resumes); ACP/Web offer `session/load` to attach an existing sidecar conversation: an OS lease at `.gsa/sessions/<session_id>.lease` (a second process fails closed), an absolute canonical cwd, no 0cy sweep on attach, and ACP v1 history replay via `session/update` for user/assistant turns (tool/system messages are not replayed; text context, blackboard and LIF sidecar are restored from the sidecar; the supplement queue is not persisted). Still open under `0dk`: background tasks and the supplement queue do not survive refresh/process loss, and the `session/close` state machine is not complete. One-shot `-p` runs do not enable cross-call resume, but they still persist sessions and archive incrementally by milestone to `.gsa/archives/`. `--plan` provides a mechanical plan state-machine workflow; `plan_first` is dormant in the production path.

### Mechanical layer

- **Structure**: the mechanical layer carries all mechanisms, gates, and guards. Its execution side splits into a **semi-assistant layer** (command runs, write execution, retrieval dispatch — returning bounded structured results) and a **silent mechanical review layer** (during the run it only records review facts; before the final answer it produces a factual report, without recommendations).
- **Execution**: the model proposes tool calls directly; the mechanical layer processes them layer by layer: registry routing → target/contract validation → execution → verification. Commands, file writes, and network access (`web_fetch`/`browser_read`) pass the permission bridge and the ACAF ticket gate first; `web_search` has no URL target and skips the ticket gate. File reads/writes carry content-anchor verification. Tools have no hard timeout of their own; long foreground commands past a threshold (default 180 s) are automatically backgrounded and kept output/CPU-alive as a backstop against the idle-kill. Failures are recorded automatically by the semi-assistant layer (process/file/environment entity registration) and returned as structured error envelopes (step/code/message/trace_id).
- **Security**: instruction source gate (IPG), permission bridge, ACAF (`orz-signer` issues one-time tickets from an independent process; unconfigured means fail-closed), credential target registration and redaction, URL gating and source weighting, retrieval candidate counting. Permissions default to **yolo auto-approval** (writes/commands/network approved by default, without prompting per action; the three `ORZ_ALLOW_*` keys switch into the Benchmark axis, and `-p`/`--plan` startups print `[permission] mode=…`). The **interactive approval surface is implemented and wired**: the TUI approval dialog opens over the ACP gateway (allow-once / cancel; no answer within 300 s fails closed with a denial), the Codex app-server surface runs over the `approval/request`→`approval/response` channel, and browser download/script actions are forced-approval classes that require per-instance human approval even under yolo. Making ordinary writes prompt per action is a registered optional extension. The security boundary = mechanical-layer gates + write control + journal audit (wording corrected to implementation reality, 2026-10-02).
- **Write control**: the write-side backstop guards against **unrecoverable disk-wiping disasters**. The tool surface refuses only the **host-state narrow targets** — the `.gsa` session volume (including its ancestor directory itself) and the ACAF keystore root / signer manifest (including the keystore/manifest ancestor chain). The `run_terminal_cmd` command surface reviews against a closed enumeration of five rules — root-level recursive deletion (since 319: workspace root / ancestors / unconstrained wildcards / `find -delete` / `xargs rm` / `cd` into a protected directory then `rm -rf *`), block-device and volume destructive writes (`/dev/null` exempt), boot/firmware and security-mechanism flips, registry hive deletion, and host-state writes (`.gsa` / keystore / manifest and their ancestors). Maintenance subcommands (`orz archive` / `unarchive` / `delete-session` / `trust` / `rollback restore`) now have a bridge-injected `ORZ_MAINTENANCE_TICKET` capability gate (missing/too-short ⇒ exit 2) plus the command-surface block (319; the TTY double-check is not implemented — remaining work under `0dl`). Privilege elevation (`elevation`) and other destructive shapes are recorded as `[write control · note]` without blocking. Ordinary write actions (installing into `/usr`, editing `/etc`, `>/dev/null`, deleting stale patches) are allowed and handed back to the approval component. This part is positioned as a **host-machine catastrophic backstop** (worded across three grades: guarantee / resistance / audit), not an absolute guarantee; no writable-root allowlist. Known bypasses are listed under "Safety on the real machine" item 3 and "Known boundaries". Design document: [`docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md) (Chinese; English distillation: [`WRITE_CONTROL_CURRENT_EN.md`](WRITE_CONTROL_CURRENT_EN.md)); public-facing wording in [`orz/SECURITY.md`](../../orz/SECURITY.md).
- **Audit, state, context**: every run writes a hash-chained event journal (event schema v0.2) cross-checked by a verifier; the mechanical audit fact report and the single-package session-blackboard archive. Context is shared by the mechanical sliding window and the model, the model side being a self-controlled attention window (a main slider plus chunk pointers beyond it; chunk contents do not flow out of the model surface); the mechanical side narrows the model surface in steps (soft reminder → 320K hard interrupt → 500K mandatory compression; if two compression windows still yield nothing, a mechanical truncation backstop kicks in). Semantic compression produces a structured model-authored summary inside a compression window (which can be initiated knowledgeably via `context_manage mode=compress`); compression never overwrites the local copy — everything stays archived and replays chunk by chunk; ACP/Web can attach by sidecar (history replayed via `session/update`; background tasks/queues do not survive refresh — `0dk` open), the TUI is read-only; the journal supports read-only `--replay`.
- **Generation guards and round budget**: repetition detection (rolling hash + 3-gram fallback), an empty-response retry chain, a stall watchdog (`ORZ_STALL_TIMEOUT`, default 360 s of inactivity wraps up) and a whole-round wall-clock cap. The round budget is unlimited by default (`MAX_TOOL_ROUNDS=0`; the old default 120-round hard cap was removed). All inquiries are **soft gates** and never disable tools: a one-time opening triple-question (direction self-check) after the first action batch, then a short neutral triple-question about action targets and progress every 50 rounds.

### Blackboard

The blackboard is a single-session state panel shared by the agents and the mechanical layer: partitioned storage for plans, executed actions, entities (files/processes/environment), session and gate records. The main agent reads on demand via `blackboard_read` (PULL) — it never sits in the prompt; the model can write to the plan/notes/findings areas via `blackboard_write` (≤8K per entry), while stamping, granting, and archiving remain mechanical-layer work. The blackboard is conversation-scoped; writes are stamped with `(domain, round)`. `blackboard_read` folds rendering by domain and round on demand (render fold), and the response header carries the blackboard watermark (【x.xM/10M】) and a "N compressible chunks beyond the slider" reading. The session archive package (`.gsa/archives/<session8>.json.gz`) is initiated by an explicit user action (web archive button / `orz archive`), session close, or the hygiene sweep at a new-session start; headless runs archive incrementally by milestone; interactive runs do not auto-archive at run end (0dd).

### Temporal / activity-domain component

The mechanical layer maintains the session's temporal reference frame (LIF): it continuously computes time, progress, error rate, "is it stuck" and similar features (T̂, u_prog/u_err/u_stuck), makes coarse domain distinctions from action features, and records domain switches (spikes, archived with the session sidecar). All computation and triggering happen in the mechanical layer, invisible to the model; when the model needs it, it reads on demand via `blackboard_read`'s time query surface (now/recent/history/feature). RLI fires are **not** internal-only: at fire time the notice is appended as an `RLI提醒: …` line to that tool batch's returned content (the model surface) and journaled one-by-one as `mechanical_audit_update{kind:"rli_notice"}`. Continuous feature readings are still PULL-only, not resident in the prompt. For long tasks, the framework keeps the time books and the trigger-time notices carry the urgent part.

### Carrier and components

- Entry points: one program, `orz`, carries the TUI, `-p` headless, `--plan`, `--stdio` (ACP), and `--replay`; `orz-signer`/`orz-acaf-provision` exist only for security-layer initialization (see setup above).
- Rust production workspace (`orz/`): `orz-loop` (agent loop, blackboard, guards), `orz-host` (tool execution, permission bridge, credentials, local browser), `orz-assurance` (journal, events, ACAF, verifier), `orz-bin` (CLI entry), `orz-tui` (terminal workbench), `orz-web` (Web workbench: loopback bridge + embedded static frontend; the `orz web` entry).
- Supporting systems: `assurance/` is the Python reference/conformance suite; `runtime/` holds the event schema; `protocol/` holds the structured operation protocol draft.

A run flows roughly like this: entry → session & journal init → main agent rounds (near-zero prompt + fixed 12-tool surface) → direct tool-call execution → mechanical permission/ticket gates → execution and retrieval → results and events flow back → `[RUN_END]` terminal declaration → journal wrap-up. Afterwards you can `--replay` or attach the session to review.

The complete mechanism state, stable IDs, and deeper entry points are under "Developer entry points" below; the design is [`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) (Chinese), currently projected in [`architecture/current/README.md`](../../architecture/current/README.md).

### Safety on the real machine

orz running directly on the real machine is a design choice.

(Note: the dogfood rounds include long, messy task rounds run while many write surfaces were still unfilled, as risk testing — see the dogfood-round records in this repository. If that is unacceptable to you, you can simply place the whole of orz inside a sandbox yourself.)

There is no sandbox between the model and the host by default, and deliberately **no writable-root allowlist**: the writable surface is the whole real environment. Safety therefore comes not from containment but from **layered mechanical gates plus full audit**, described with three grades of wording — *guarantee / resistance / audit* — and never an absolute guarantee. Four layers stack up:

1. **Provenance — ACAF**: every action crossing a trust boundary carries a one-time HMAC ticket from an independent signer process whose keys the agent never sees; tickets are invisible to the model, bound to the resolved real target (TOCTOU-checked), consumed atomically, and audited. Fail-closed in the shipped carrier: unconfigured, runs are refused. → [ACAF_CURRENT_EN.md](ACAF_CURRENT_EN.md)
2. **Policy — the permission bridge and the approval component**: per-axis switches for headless and batch scenarios (`--allow-write`, `--allow-shell`/`--allow-network`); ordinary writes/commands/network default to **yolo auto-approval, without prompting per action**, while the **interactive approval surface is implemented and wired** — the TUI approval dialog (allow-once / cancel; no answer within 300 s is denied, over the ACP gateway; the Codex app-server surface uses the `approval/request` channel), and browser download/script actions are forced-approval classes that require per-instance human approval even under yolo; making ordinary writes prompt per action is a registered optional extension.
3. **Catastrophic write backstop — write control**: a closed enumeration of exactly five block rules catching only irreversible destruction shapes — root-level recursive deletion, raw-device/volume destructive writes, boot/security-mechanism flips, registry hive deletion, and host-state writes (the `.gsa` session volume and its ancestor directories; the ACAF keystore root / signer manifest and their ancestor chains). The 319 repair batch extends rule 1 to workspace root / ancestors / unconstrained wildcards / `find -delete` / `xargs rm` / `cd` into a protected directory then `rm -rf *`, and adds a command-surface block for maintenance subcommands (final readings are governed by the 319 report; remaining work is under `0dl`). Ordinary writes stay allowed and belong to the approval component. It is enforced at the tool surface, by lexical command review, and by a Linux Landlock kernel guard (child-process write surface only; skipped fail-open when the kernel lacks Landlock or enumeration fails). → [WRITE_CONTROL_CURRENT_EN.md](WRITE_CONTROL_CURRENT_EN.md)
4. **Audit and recovery**: a hash-chained journal with verifier cross-check, read-only `--replay`, an edit-face rollback window, the `orz rollback` undo CLI, and a carrier integrity self-check.

Known boundaries: no defense against a compromised signer, same-user malicious processes, kernel-level compromise, or a malicious model service; a ticket proves provenance, not that a command is wise; command review is best-effort lexical matching — variable expansion, aliases, in-script commands, `.NET`/interpreter calls, encoded payloads, and Windows 8.3 short names / junctions / UNC uncreated faces are still not guaranteed; the maintenance CLI now has the bridge-injected `ORZ_MAINTENANCE_TICKET` gate plus the command-surface block, while a TTY confirmation is not implemented (`0dl` remaining); the `.gsa` shell read surface follows the read-open policy. The real machine runs without a sandbox and permissions default to yolo auto-approval.

## 0.9.8 update (2026-10-22)

- **Session lifecycle**: the Web workbench supports close/delete/archive/restore; the session table is bounded at 32 (LRU evicts idle sessions only); `_orz/session_close|state|delete` private extensions; prompts after close are rejected with `-32010` and the four-part error; deletion writes a `.deleted` tombstone and never resurrects the session.
- **Refresh and disconnect**: the bridge keeps the agent child alive per workspace (default 30 minutes, `ORZ_WEB_KEEPALIVE_SECS`); refresh/short disconnects re-attach to the same session; a second tab gets 409 with an occupied-workspace message; keepalive expiry is stated explicitly.
- **Rewind and mid-run supplements**: `/rewind` truncates the user message and returns the original text to the input (audited via `recall_log`; journal history is not rolled back); input during a run enters a session-level supplement queue and is delivered at the next model-call boundary without interrupting the current turn.
- **Web detail faces**: delivery-status panel (`/api/delivery`), context readout (`/api/context`, chars/2 approximation), approval-displacement notice, background task "model-visible only" counter, persistent version-skew banner, `orz doctor` report, `orz --version`.
- **Reliability/security**: CLI deletion goes through the close/lease gate; bridge client/agent request ids are split, unanswered approvals are replayed across attach, and attached ingress is bounded; cross-root resolution and `closing` are observable; errors use the unified `code/class/hint/evidence` shape.
- **Real-machine regression**: the ten 0dn S2 scenarios (refresh reconnect, two-tab 409, rewind-and-resend, delete-no-resurrection, keepalive expiry, delivery panel, approval concurrency, context readout, background visibility, version skew) all pass 10/10 on 0.9.8.
- **Rebuild**: dual-platform 0.9.8 carriers; Windows install swap + ACAF re-provision; Linux static-pie trio.

## Developer entry points

- Project-wide routing, status, and stable IDs: [`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) (Chinese)
- Current design authority: [`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) (Chinese)
- Current architecture projection: [`architecture/current/README.md`](../../architecture/current/README.md)
- Unified backlog: [`docs/BACKLOG_AND_PRIORITIES.md`](../BACKLOG_AND_PRIORITIES.md) / [`TODO.md`](../../TODO.md) (Chinese)
- Implementation audits: [`docs/audits/`](../audits/) (Chinese)
- Python reference/conformance: [`assurance/README.md`](../../assurance/README.md)
- Historical README snapshots (Chinese archive directory): [`存档/readme/README.md`](../../存档/readme/README.md)

> [!IMPORTANT]
> Build prerequisite: the `orz-tools-api` build script needs `protoc`. The bundled
> `orz/bin/protoc.exe` is a **local, non-committed** dependency (excluded by `.gitignore`,
> absent from a clean clone) — set `PROTOC=<path>/orz/bin/protoc.exe` first (or install
> `protobuf-compiler`), otherwise dependency compilation fails at the build script only
> after ~20 minutes.

### Current status

- **Design**: ADR-0010 is the sole natural-language design authority, `accepted / evolving` (for evolution changes see ADR-0010 §14.79).
- **Implementation**: the Rust production workspace runs; overall status `partial`; open gaps are registered in [CLI_PROJECT_INDEX.md §3.1](../../CLI_PROJECT_INDEX.md) (Chinese) and not expanded here.
- **Releases**: 0.1.0–0.5.1 trial packages under [`releases/`](../../releases/); from 0.5.4 on, dual-platform installers are published on [GitHub Releases](https://github.com/SilverWhite/orz/releases); no native macOS packages at this time.
- All-green tests or a single benchmark run do not establish architectural conformance; conformance status is governed by the index and the audits.

## License

Apache License 2.0. See [`LICENSE`](../../LICENSE) and [`NOTICE`](../../NOTICE).

Third-party and vendored licensing/provenance: [`orz/THIRD-PARTY-NOTICES`](../../orz/THIRD-PARTY-NOTICES) (crate dependencies, vendored source ports, plus the Web workbench's embedded static assets 98.css / XP.css / marked and the Pixelated MS Sans Serif font), the [component register](../../upstream/fusion-component-register-v0.1.yaml), and the per-asset summary of embedded frontend assets [`vendor/MANIFEST.sha256.txt`](../../orz/crates/orz-web/assets/vendor/MANIFEST.sha256.txt).
