# async-openai (vendored fork)

Vendored copy of `async-openai` 0.33.1 with a DeepSeek `reasoning_content`
patch. The dependency is wired through the workspace
`[patch.crates-io]` in `Cargo.toml` (`path = "third_party/async-openai"`),
so the orz workspace — and the inherited crates (`orz-tools`,
`orz-sampling-types`) — compile against this single local source. The
patch is a **file in this repository**: no external fork, no rev pinning,
every clone reproduces the exact dependency graph.

## Provenance

- **Upstream**: [async-openai](https://github.com/async-openai/async-openai) 0.33.1
- **Base commit**: `95b52ebdedf42143083cf3d6f0e0be7c84e9c808`
  (the rev previously pinned as `[patch.crates-io]` git dependency
  `our-forks/async-openai.git`)
- **License**: MIT (`LICENSE`, Copyright (c) 2022 Himanshu Neema)
- **Trimmed**: `examples/`, `openapi.documented.yml`, `.github/`,
  `CONTRIBUTING.md`, `.git` — none are part of crate compilation.

## Local changes (vs base commit)

`async-openai/src/types/chat/chat_.rs` — three types gain a
`reasoning_content: Option<String>` field:

1. `ChatCompletionRequestAssistantMessage` — serialized only when `Some`
   (`skip_serializing_if`), so providers that never emit it see no change;
2. `ChatCompletionResponseMessage` — preserved from the non-streamed
   response so transports can replay it;
3. `ChatCompletionStreamResponseDelta` — accumulated across stream chunks.

DeepSeek returns `reasoning_content` on every completion (even without a
thinking option; live probe 2026-08-06 showed 318 chars on a plain
prompt) and expects it replayed with the assistant turn on multi-turn
conversations. The orz transport (`orz-loop/src/gateway/transport.rs`)
preserves it on `ModelResponse` and echoes it back on the assistant
declaration message. See the main repo audit
`docs/REASONING_CONTENT_REPLAY_SLICE_2026-08-06.md` for the full closure.

## Updating

Edit here, bump the patch if the upstream rev moves. The fork is a
deliberately small delta; keep the diff to this file minimal.
