//! MainAgent — the single model-backed agent in the loop.

use std::sync::Arc;

use crate::gateway::model::{
    ActivityClock, GatewayError, Message, ModelGateway, ModelRequest, ModelResponse,
};
use crate::host::ToolDef;
use crate::prompt::PromptBuilder;

/// The single main agent — drives the model↔tool loop.
#[derive(Clone)]
pub struct MainAgent {
    pub gateway: Arc<dyn ModelGateway>,
    pub prompt_builder: PromptBuilder,
}

impl MainAgent {
    pub fn new(gateway: Arc<dyn ModelGateway>) -> Self {
        Self {
            gateway,
            prompt_builder: PromptBuilder::new(),
        }
    }

    /// Run one model generation round, streaming the text to `on_chunk` as
    /// the gateway produces it (live `text_delta` delivery — never journaled).
    ///
    /// `system` is the assembled system prompt (assurance blocks injected by
    /// the caller/controller); `messages` carries the turn history including
    /// tool results; `tools` is the current tool registry projection.
    /// P1-1 (2026-08-08 stall guards): `heartbeat` is forwarded to the
    /// gateway — the transport stamps it on every wire frame (reasoning
    /// deltas included) so a long max-effort thinking round counts as
    /// activity for the stall watchdog.
    pub async fn run_round(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Vec<ToolDef>,
        max_tokens: u32,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        self.gateway
            .generate_stream(
                ModelRequest {
                    system: system.to_string(),
                    messages,
                    tools,
                    max_tokens,
                    thinking: None,
                },
                cancel,
                heartbeat,
                on_chunk,
            )
            .await
    }
}
