//! MainAgent — the single model-backed agent in the loop.

use std::sync::Arc;

use crate::agent_loop::RoundAgent;
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
    #[allow(clippy::too_many_arguments)] // RoundAgent trait signature — every param is wired
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

/// GAP-SUBAGENT-RUNTIME (2026-08-10): the main agent's uniform round entry —
/// delegates to the inherent method so the shared loop (`run_agent_loop`)
/// drives all three agents through one code path.
#[async_trait::async_trait]
impl RoundAgent for MainAgent {
    fn config_fingerprint(&self) -> String {
        self.gateway.config_fingerprint()
    }

    async fn run_round(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Vec<ToolDef>,
        max_tokens: u32,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        self.run_round(
            system, messages, tools, max_tokens, cancel, heartbeat, on_chunk,
        )
        .await
    }
}
