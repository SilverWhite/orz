//! ACP JSON-RPC stdio server entry point.
//! Uses agent-client-protocol 0.10.4 + xai-acp-lib for gateway/channels.
//! Implements session/new, session/prompt, list_tools, and other ACP methods.
//! Delegates agent turns to orz_loop::AgentLoopController via LoopHost trait.

// TODO: Implement ACP server (Phase 1 — scaffold only)
