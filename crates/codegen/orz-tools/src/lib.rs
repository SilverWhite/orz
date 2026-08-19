//! Grok tools library.

pub use orz_version::VERSION;

/// Default maximum output size (in bytes) for tool results sent to the model.
/// 40 KB ≈ 10 000 tokens
pub const DEFAULT_TOOL_OUTPUT_BYTES: usize = 40_000;

/// Default maximum output size (in characters) for bash/terminal tool results.
/// 8 000 chars ≈ 2 000 tokens (OUTPUT-DEGENERATION-GUARD 2026-08-19,
/// ADR-0010 §14.33: 终端工具输出统一限值 20K → 8K——「只做一个限值」；
/// 超限截断末尾机械附加 read_file 补读闭环指针，8K 以内正常结果完整可见).
pub const DEFAULT_TOOL_OUTPUT_CHARS: usize = 8_000;

/// MCP inline tool-result cap (`MCP_MAX_OUTPUT_BYTES` and host/env helpers).
pub use util::mcp_truncate::{
    ENV_GROK_MAX_MCP_OUTPUT_BYTES, ENV_MAX_MCP_OUTPUT_BYTES, MCP_MAX_OUTPUT_BYTES,
    mcp_max_output_bytes, mcp_max_output_bytes_from_env, set_mcp_max_output_bytes,
};

pub mod attribution;

pub mod bridge;
pub mod computer;
pub mod gitignore;
pub mod implementations;
pub mod normalization;
pub mod notification;
pub mod persistence;
pub mod registry;
pub mod reminders;
pub mod retry;
pub mod tool_taxonomy;
pub mod types;
pub mod util;
pub mod versions;

pub use attribution::{
    Auth401AttributionCallback, SENT_BEARER_PREFIX_LEN, SharedAttributionCallback, ToolConsumer,
};
