# orz — English README (expanded edition)

> LLM-assisted translation; the Chinese originals are authoritative. Corrections welcome.
> Source: [`/README.md`](../../README.md) (Chinese, authoritative), v0.8.10 face, 2026-10-02.
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
| 1. Unpack | Put `orz.exe`, `orz-signer.exe`, `orz-acaf-provision.exe` into one directory (e.g. `C:\orz`). | `mkdir -p ~/orz && cd ~/orz`<br>`tar -xzf orz-0.8.10-linux-x86_64.tar.gz`<br>`chmod +x orz orz-signer orz-acaf-provision` |
| 2. Configure the API key | Store it in Windows Credential Manager (Generic, target name `orz-deepseek/agent`; once only):<br>`cmdkey /generic:orz-deepseek/agent /user:agent /pass:YOUR_DEEPSEEK_API_KEY` | Use an environment variable (the explicit exception to the Windows Credential Manager channel):<br>`export ORZ_DEEPSEEK_API_KEY=YOUR_DEEPSEEK_API_KEY` |
| 3. Initialize security signing (one-time) | `.\orz-acaf-provision.exe "$env:USERPROFILE\.orz-acaf\keystore" "$env:USERPROFILE\.orz-acaf\signer-manifest.json"`<br><br>ACAF is fail-closed by default: unconfigured, it refuses to run (the process starts; runs are refused). | `./orz-acaf-provision "$HOME/.orz-acaf/keystore" "$HOME/.orz-acaf/signer-manifest.json"` |
| 4. Set startup environment | `$env:ORZ_ACAF_KEYSTORE = "$env:USERPROFILE\.orz-acaf\keystore"`<br>`$env:ORZ_ACAF_MANIFEST = "$env:USERPROFILE\.orz-acaf\signer-manifest.json"`<br>`$env:ORZ_ACAF_BINARY = "C:\orz\orz-signer.exe"` | `export ORZ_ACAF_KEYSTORE="$HOME/.orz-acaf/keystore"`<br>`export ORZ_ACAF_MANIFEST="$HOME/.orz-acaf/signer-manifest.json"`<br>`export ORZ_ACAF_BINARY="$HOME/orz/orz-signer"` |
| 5. Run | `.\orz.exe` (interactive TUI)<br>`.\orz.exe -p "your task" --real` (headless)<br>`.\orz.exe web` (Web workbench; prints the local token-bearing URL) | `./orz` (interactive TUI)<br>`./orz -p "your task" --real` (headless)<br>`./orz web` (Web workbench; prints the local token-bearing URL) |

Release notes and integrity verification live on GitHub Releases (latest [v0.8.10](https://github.com/SilverWhite/CLI/releases/tag/v0.8.10), dual-platform packages + `SHA256SUMS`); the 0.1.0–0.5.1 trial packages are under [`releases/`](../../releases/).

### Building from source

The Rust toolchain is pinned by [`orz/rust-toolchain.toml`](../../orz/rust-toolchain.toml); build inside the `orz/` workspace:

```powershell
cargo build -p orz-bin
cargo run -p orz-bin -- --fake-provider -p "hello"   # offline trial run, no credentials needed
cargo run -p orz-bin -- --real -p "your task"        # real DeepSeek transport
cargo run -p orz-bin -- --fake-provider               # TUI
```

> [!IMPORTANT]
> Build prerequisite: the `orz-tools-api` build script needs `protoc`. The bundled
> `orz/bin/protoc.exe` is a **local, non-committed** dependency (excluded by `.gitignore`,
> absent from a clean clone) — set `PROTOC=<path>/orz/bin/protoc.exe` first (or install
> `protobuf-compiler`), otherwise dependency compilation fails at the build script only
> after ~20 minutes.

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
| Web retrieval switch | `orz --retrieval-enabled` (off by default; the old `--retrieval-mode` is deprecated, parsed only for compatibility) |

Web retrieval is off by default; tasks that need network access must pass `--retrieval-enabled` explicitly (an independent retrieval enablement gate, fail-closed). When enabled, retrieval is executed by an external retrieval subagent: the local browser channel is preferred (engine SERP, Google primary, Bing fallback); if the browser is unavailable the model may switch on its own to the native web retrieval channel; launch results and lane switches are recorded in the event chain (`browser_launch_result`).

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

- **Main agent**: the sole task driver. Its system prompt is near-zero, and it faces a frozen, fixed 10-tool surface: `read_file`/`grep`/`search_replace`/`run_terminal_cmd`/`web_search`/`web_fetch`, plus `blackboard_read`, `submit`, `blackboard_write` (writes to the blackboard plan/notes areas, ≤8K per entry), and `context_compress` (informed initiation of model-participated compression). The default model is registered as DeepSeek v4 flash (thinking at max by default). Tasks are delivered via two-phase `submit` (request → confirm); before the final answer there is one round of mechanical audit and counterexample self-check.
- **External retrieval subagent**: web retrieval runs through an external retrieval subagent; the retrieval enablement gate is fail-closed — disabled means the whole family is closed (`--retrieval-enabled`). The main surface's `web_search` stays a single dispatch entry point while execution lives in the subagent (global concurrency 1). When enabled, the subagent's tool surface always registers both local-browser and native-web retrieval families (with static annotations carrying lane names and preference order; local browser first), and lane switching is the model's own choice: the local browser channel drives engine SERPs (Google primary, Bing fallback, DDG last resort; humanized typing = per-character keystrokes + pre-submit pauses + Enter, invisible to the model), while the HTTP segmented lane uses a self-built engine chain (default direct `360search,baidu` for networks without a VPN; the proxy chain adds `duckduckgo`; TLS/HTTP2 fingerprint cloaking; result-quality judgment is left to the retrieval subagent). Browser launch availability is recorded as a factual event (`browser_launch_result`) in the event chain. An internal retrieval lane remains in the design; its triggering tool is currently sealed off.
- **Sessions and plans**: interactive sessions (TUI/ACP) can be resumed across processes; one-shot `-p` runs do not enable cross-call resume, but they still persist sessions and archive incrementally by milestone to `.gsa/archives/`. `--plan` provides a mechanical plan state-machine workflow; `plan_first` is dormant in the production path.

### Mechanical layer

- **Structure**: the mechanical layer carries all mechanisms, gates, and guards. Its execution side splits into a **semi-assistant layer** (command runs, write execution, retrieval dispatch — returning bounded structured results) and a **silent mechanical review layer** (during the run it only records review facts; before the final answer it produces a factual report, without recommendations).
- **Execution**: the model proposes tool calls directly; the mechanical layer processes them layer by layer: registry routing → target/contract validation → execution → verification. Commands, file writes, and network access (`web_fetch`/`browser_read`) pass the permission bridge and the ACAF ticket gate first; `web_search` has no URL target and skips the ticket gate. File reads/writes carry content-anchor verification. Tools have no hard timeout of their own; long foreground commands past a threshold (default 180 s) are automatically backgrounded and kept output/CPU-alive as a backstop against the idle-kill. Failures are recorded automatically by the semi-assistant layer (process/file/environment entity registration) and returned as structured error envelopes (step/code/message/trace_id).
- **Security**: instruction source gate (IPG), permission bridge, ACAF (`orz-signer` issues one-time tickets from an independent process; unconfigured means fail-closed), credential target registration and redaction, URL gating and source weighting, retrieval candidate counting. Permissions default to **yolo auto-approval** (writes/commands/network approved by default; no human approval at present; the three `ORZ_ALLOW_*` keys switch into the Benchmark axis, and `-p`/`--plan` startups print `[permission] mode=…`). An approval surface is a future optional extension and is not implemented; the current security boundary = mechanical-layer gates + write control + journal audit (review 083 adjudication ②, 2026-09-26).
- **Write control** (0bw v1 → 0cb/0cc v3, narrowed 2026-09-29): the write-side backstop is a **host-machine catastrophic hard boundary** (against unrecoverable disk-wiping disasters), not a general write review. The tool surface refuses only the **two narrow host-state targets** — the `.gsa` session volume, and the ACAF keystore root / signer manifest. The `run_terminal_cmd` command surface reviews against a closed enumeration of five rules — root-level recursive deletion, block-device and volume destructive writes (`/dev/null` exempt), boot/firmware and security-mechanism flips, registry hive deletion, and host-state writes (including a host-state ancestor-chain arm: sweep-style deletions/moves of protected targets are caught too). Privilege elevation (`elevation`) and other destructive shapes are recorded as `[write control · note]` without blocking. Ordinary write actions (installing into `/usr`, editing `/etc`, `>/dev/null`, deleting stale patches) are allowed and handed back to the approval component. Positioning = a **host-machine catastrophic backstop** (worded across three grades: guarantee / resistance / audit), not an absolute guarantee; no writable-root allowlist. Design authority: [`docs/WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md) (Chinese; English distillation: [`WRITE_CONTROL_CURRENT_EN.md`](WRITE_CONTROL_CURRENT_EN.md)); public-facing wording in [`orz/SECURITY.md`](../../orz/SECURITY.md).
- **Audit, state, context**: every run writes a hash-chained event journal (event schema v0.2) cross-checked by a verifier; the mechanical audit fact report and the single-package session-blackboard archive. Context is handled jointly by a mechanical sliding window and the model — the model side is a self-controlled attention window (a main slider plus chunk pointers beyond it; chunk contents do not flow out of the model surface), and the mechanical side narrows the model surface in steps (soft reminder → 320K hard interrupt → 500K mandatory compression; if two compression windows still yield nothing, a mechanical truncation backstop kicks in). Semantic compression produces a structured model-authored summary inside a compression window (which can be initiated knowledgeably via `context_compress`); compression never overwrites the local copy — everything stays archived and replays chunk by chunk; sessions are resumable and the journal supports read-only `--replay`.
- **Generation guards and round budget**: repetition detection (rolling hash + 3-gram fallback), an empty-response retry chain, a stall watchdog (`ORZ_STALL_TIMEOUT`, default 360 s of inactivity wraps up) and a whole-round wall-clock cap. The round budget is unlimited by default (`MAX_TOOL_ROUNDS=0`; the old default 120-round hard cap was removed). All inquiries are **soft gates** and never disable tools: a one-time opening triple-question (direction self-check) after the first action batch, then a short neutral triple-question about action targets and progress every 50 rounds.

### Blackboard

The blackboard is a single-session state panel shared by the agents and the mechanical layer: partitioned storage for plans, executed actions, entities (files/processes/environment), session and gate records. The main agent reads on demand via `blackboard_read` (PULL) — it never sits in the prompt; the model can write to the plan/notes areas via `blackboard_write` (≤8K per entry), while stamping, granting, and archiving remain mechanical-layer work. The blackboard is conversation-scoped (the old plan-epoch production semantics are retired); writes are stamped with `(domain, round)`. `blackboard_read` folds rendering by domain and round on demand (render fold), and the response header carries the blackboard watermark (【x.xM/10M】) and a "N compressible chunks beyond the slider" reading. Interactive sessions are packaged by `session_archive` into a single gzip archive at session end; headless runs archive incrementally by milestone.

### Temporal / activity-domain component

The mechanical layer maintains the session's temporal reference frame (LIF): it continuously computes time, progress, error rate, "is it stuck" and similar features (T̂, u_prog/u_err/u_stuck), makes coarse domain distinctions from action features, and records domain switches (spikes, archived with the session sidecar). All computation and triggering happen in the mechanical layer, invisible to the model; when the model needs it, it reads on demand via `blackboard_read`'s time query surface (now/recent/history/feature). Fires are recorded internally only, never injected into the model surface — for long tasks, the framework keeps the model's time books.

### Carrier and components

- Entry points: one program, `orz`, carries the TUI, `-p` headless, `--plan`, `--stdio` (ACP), and `--replay`; `orz-signer`/`orz-acaf-provision` exist only for security-layer initialization (see setup above).
- Rust production workspace (`orz/`): `orz-loop` (agent loop, blackboard, guards), `orz-host` (tool execution, permission bridge, credentials, local browser), `orz-assurance` (journal, events, ACAF, verifier), `orz-bin` (CLI entry), `orz-tui` (terminal workbench), `orz-web` (Web workbench: loopback bridge + embedded static frontend; the `orz web` entry).
- Supporting systems: `assurance/` is the Python reference/conformance suite; `runtime/` holds the event schema; `protocol/` holds the structured operation protocol draft.

A run flows roughly like this: entry → session & journal init → main agent rounds (near-zero prompt + frozen 10-tool surface) → direct tool-call execution → mechanical permission/ticket gates → execution and retrieval → results and events flow back → two-phase submit delivery → journal wrap-up. Afterwards you can `--replay` or resume the session to review.

The complete mechanism state, stable IDs, and deeper entry points are under "Developer entry points" below; the design authority is [`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) (Chinese), currently projected in [`architecture/current/README.md`](../../architecture/current/README.md).

### Safety on the real machine

orz runs directly on your machine — by design. There is no sandbox between the model and the host by default, and deliberately **no writable-root allowlist**: the writable surface is the whole real environment. Safety therefore comes not from containment but from **layered mechanical gates plus full audit**, described with three grades of wording — *guarantee / resistance / audit* — and never sold as an absolute guarantee. Four layers stack up:

1. **Provenance — ACAF**: every action crossing a trust boundary carries a one-time HMAC ticket from an independent signer process whose keys the agent never sees; tickets are invisible to the model, bound to the resolved real target (TOCTOU-checked), consumed atomically, and audited. Fail-closed in the shipped carrier: unconfigured, runs are refused. → [ACAF_CURRENT_EN.md](ACAF_CURRENT_EN.md)
2. **Policy — the permission bridge and the approval component**: per-axis switches for headless and batch scenarios (`--allow-write`, `--allow-shell`/`--allow-network`); the default today is yolo auto-approval, and the human-approval surface (the Codex-lineage approval component) is a recorded future extension, not implemented — stated plainly rather than papered over.
3. **Catastrophic write backstop — write control**: a closed enumeration of exactly five block rules catching only irreversible destruction shapes — root-level recursive deletion, raw-device/volume destructive writes, boot/security-mechanism flips, registry hive deletion, and host-state writes (the `.gsa` session volume and the ACAF keystore root / signer manifest, with an ancestor-sweep arm); ordinary writes stay allowed and belong to the approval component. It is enforced at the tool surface, by lexical command review, and by a Linux Landlock kernel guard. → [WRITE_CONTROL_CURRENT_EN.md](WRITE_CONTROL_CURRENT_EN.md)
4. **Audit and recovery**: a hash-chained journal with verifier cross-check, read-only `--replay`, an edit-face rollback window, the `orz rollback` undo CLI, and a carrier integrity self-check.

Known boundaries are registered, not hidden: no defense against a compromised signer, same-user malicious processes, kernel-level compromise, or a malicious model service; a ticket proves provenance, not that a command is wise; command review is best-effort lexical matching.

## Developer entry points

- Project-wide routing, status, and stable IDs: [`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md) (Chinese)
- Current design authority: [`ADR-0010`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md) (Chinese)
- Current architecture projection: [`architecture/current/README.md`](../../architecture/current/README.md)
- Unified backlog: [`docs/BACKLOG_AND_PRIORITIES.md`](../BACKLOG_AND_PRIORITIES.md) / [`TODO.md`](../../TODO.md) (Chinese)
- Implementation audits: [`docs/audits/`](../audits/) (Chinese)
- Python reference/conformance: [`assurance/README.md`](../../assurance/README.md)
- Historical README snapshots (Chinese archive directory): [`存档/readme/README.md`](../../存档/readme/README.md)

This repository is committed and released single-maintainer: external developers, please fork and open PRs or issues; no direct write access is granted.

### Current status

- **Design**: ADR-0010 is the sole natural-language design authority, `accepted / evolving` (as of 2026-09-27 the freeze was lifted in favor of a versioned current law — design-layer evolution is free; contract-layer changes must go through revision documents + downstream synchronization; see ADR-0010 §14.79).
- **Implementation**: the Rust production workspace runs; overall status `partial`; open gaps are registered in [CLI_PROJECT_INDEX.md §3.1](../../CLI_PROJECT_INDEX.md) (Chinese) and not expanded here.
- **Releases**: 0.1.0–0.5.1 trial packages under [`releases/`](../../releases/); from 0.5.4 on, dual-platform installers are on [GitHub Releases](https://github.com/SilverWhite/CLI/releases) (current latest v0.8.10, Windows zip / Linux tar.gz + `SHA256SUMS`; 0.6.13 embedded the Web workbench in the carrier — `orz web` starts a local loopback UI; 0.7.0 shipped the retrieval line and the real-machine browser lane; 0.8.0 shipped the write-control line [Linux Landlock / carrier integrity self-check / command review logging / rollback-window undo]; 0.8.2 shipped the ACAF signing assembly-point mismatch fix and the signer-startup-failure bypass; 0.8.7 narrowed write control to the host-machine catastrophic backstop [protected surface = `.gsa` session volume + ACAF keystore root / signer manifest, with a host-state ancestor-chain arm] and shipped model-surface prefix render stabilization; 0.8.8 shipped write-control L3 kernel-granularity precision [file-level allow for safe device nodes + allow for new root-level entries]; 0.8.9 shipped dead-code retirement and wall-clock visibility removal; 0.8.10 shipped blackboard model-surface slimming [description 5,593 → 2,492 chars + guide rewrite] and RLI reference-surface annotations). No native macOS packages at this time.
- All-green tests or a single benchmark run do not establish architectural conformance; conformance status is governed by the index and the audits.

## License

Apache License 2.0. See [`LICENSE`](../../LICENSE) and [`NOTICE`](../../NOTICE).

Third-party and vendored licensing/provenance: [`orz/THIRD-PARTY-NOTICES`](../../orz/THIRD-PARTY-NOTICES) (crate dependencies, vendored source ports, plus the Web workbench's embedded static assets 98.css / XP.css / marked and the Pixelated MS Sans Serif font), the [component register](../../upstream/fusion-component-register-v0.1.yaml), and the per-asset summary of embedded frontend assets [`vendor/MANIFEST.sha256.txt`](../../orz/crates/orz-web/assets/vendor/MANIFEST.sha256.txt).
