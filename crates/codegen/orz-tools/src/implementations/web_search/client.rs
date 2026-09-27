use super::local_segmented::{self, LocalSegmentedConfig};
use super::types::WebSearchConfig;
use crate::attribution::{SharedAttributionCallback, ToolConsumer};
use crate::types::SharedApiKeyProvider;
use async_openai::types::responses as rs;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderName, HeaderValue};
use std::time::Duration;
/// A minimal, purpose-built HTTP client for calling the Responses API
/// with web search capability.
#[derive(Clone)]
pub struct WebSearchClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
    api_key_provider: Option<SharedApiKeyProvider>,
    /// Optional 401-attribution hook. Callers can wire this so a 401
    /// from the Responses API emits an `auth_401_attribution` event
    /// with `consumer == "WebSearch"`.
    attribution_callback: Option<SharedAttributionCallback>,
    /// 0ac S3① (2026-09-13, design §9/§10): `Some` ⇒ the local segmented
    /// retrieval path (SERP 引擎链 → 逐页抓取 → 逐段抽取) is active and the
    /// server-side `/responses` search path is never used.
    local_segmented: Option<LocalSegmentedConfig>,
    /// The plain HTTP client of the local path (no backend auth headers).
    local_http: Option<reqwest::Client>,
}
impl WebSearchClient {
    /// Create a new web search client from `WebSearchConfig::Enabled`.
    ///
    /// Returns `Err` if the config is `Disabled` or if header values are invalid.
    pub fn new(
        config: &WebSearchConfig,
        api_key_provider: Option<SharedApiKeyProvider>,
    ) -> Result<Self, xai_tool_runtime::ToolError> {
        let WebSearchConfig::Enabled {
            api_key,
            base_url,
            model,
            extra_headers,
            alpha_test_key,
        } = config
        else {
            return Err(xai_tool_runtime::ToolError::execution(
                xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                "Cannot create WebSearchClient from disabled config".to_string(),
            ));
        };
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}")).map_err(|e| {
                xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Invalid API key for header: {e}"),
                )
            })?,
        );
        for (key, value) in extra_headers {
            let header_name = HeaderName::from_bytes(key.as_bytes()).map_err(|e| {
                xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Invalid header name '{key}': {e}"),
                )
            })?;
            let header_value = HeaderValue::from_str(value).map_err(|e| {
                xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Invalid header value for '{key}': {e}"),
                )
            })?;
            headers.insert(header_name, header_value);
        }
        let _ = alpha_test_key;
        let http = reqwest::Client::builder()
            // THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1): DeepSeek
            // /responses 生成式搜索是服务端多轮（search → open →
            // re-search），社区实测单次 15–70s 属正常、120s 为共识下限——
            // 客户端总超时 120s + connect 10s；超时经
            // `map_transport_error` 转为结构化 Timeout 错误，不再挂死
            // （S4 实测单次最高 1365s 的时间黑洞）。
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()
            .map_err(|e| {
                xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Failed to build HTTP client: {e}"),
                )
            })?;
        Ok(Self {
            http,
            base_url: base_url.clone(),
            model: model.clone(),
            api_key_provider,
            attribution_callback: None,
            local_segmented: None,
            local_http: None,
        })
    }

    /// 0ac S3① (2026-09-13, design §9/§10): point this client at the **local
    /// segmented retrieval** front-end. Enabled ⇒ every `search` /
    /// `search_with_titles` call short-circuits to the local path; the
    /// server-side `/responses` search (withdrawn by the provider, S1 probe
    /// §5/§6) is not called at all. A config that is switched off is a no-op.
    pub fn with_local_segmented(mut self, config: LocalSegmentedConfig) -> Self {
        if config.is_enabled() {
            match local_segmented::build_http_client(&config) {
                Ok(http) => {
                    self.local_http = Some(http);
                    self.local_segmented = Some(config);
                }
                Err(e) => {
                    tracing::warn!("local segmented retrieval http client build failed: {e}");
                }
            }
        }
        self
    }

    /// The active local-path config (`None` ⇒ the server-side path).
    pub fn local_segmented(&self) -> Option<&LocalSegmentedConfig> {
        self.local_segmented.as_ref()
    }
    /// Wire a 401-attribution callback into this client. Idempotent;
    /// safe to call before or after the first request.
    pub fn with_attribution_callback(
        mut self,
        callback: Option<SharedAttributionCallback>,
    ) -> Self {
        self.attribution_callback = callback;
        self
    }
    async fn current_bearer(&self) -> Option<String> {
        crate::types::api_key_provider::resolve_bearer(self.api_key_provider.as_ref()).await
    }
    fn record_401_attribution(&self, sent_bearer: Option<&str>) {
        crate::attribution::emit_401(
            self.attribution_callback.as_ref(),
            ToolConsumer::WebSearch,
            sent_bearer,
        );
    }
    /// Perform a web search query using the Responses API.
    ///
    /// THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1; 审查处理 P3-1/P4):
    /// map a reqwest transport failure to a structured tool error —
    /// client-side total / connect timeouts surface as
    /// `ToolErrorKind::Timeout` with the elapsed time and a concrete next
    /// step (retry with a narrower query / read a known URL via
    /// web_fetch). `stage` names the failing transport phase ("sending
    /// request" / "reading response body") so non-timeout failures keep
    /// their phase context; connect timeouts get their own wording (the
    /// server was not reached — connect budget 10s). `elapsed` is measured
    /// from the request start (S4: web_search single calls hung up to
    /// 1365s before the 120s budget).
    fn map_transport_error(
        e: reqwest::Error,
        elapsed: std::time::Duration,
        stage: &str,
    ) -> xai_tool_runtime::ToolError {
        let tool_id = xai_tool_protocol::ToolId::new("web_search").expect("valid");
        if e.is_timeout() {
            let budget_note = if e.is_connect() {
                "connection timed out (connect budget 10s) — the server was not reached"
            } else {
                "the Responses API server-side search did not complete within \
                 the client budget (120s)"
            };
            xai_tool_runtime::ToolError::timeout(
                tool_id,
                format!(
                    "web_search timed out after {:.1}s — {budget_note} ({stage}). \
                     Retry with a narrower query, or read a known URL directly \
                     with web_fetch.",
                    elapsed.as_secs_f64(),
                ),
            )
        } else {
            xai_tool_runtime::ToolError::execution(
                tool_id,
                format!("HTTP request failed while {stage}: {e}"),
            )
        }
    }

    /// Returns `(content, citations)` where content is the assistant's text
    /// and citations are unique URLs found in the response annotations.
    ///
    /// 0bv（2026-09-26）：等价于不带浏览器 SERP 链首的
    /// [`Self::search_with_serp`]（资源缺席形态——链＝本地 HTTP → provider）。
    pub async fn search(
        &self,
        query: &str,
        allowed_domains: Option<Vec<String>>,
    ) -> Result<(String, Vec<String>), xai_tool_runtime::ToolError> {
        let (content, citations, _facts) =
            self.search_with_serp(query, allowed_domains, None).await?;
        Ok((content, citations))
    }

    /// 0bv（2026-09-26）：带**浏览器 SERP 链首**的检索链——浏览器 SERP
    /// （若可用）→ 本地 HTTP 分段 → provider 合成。
    ///
    /// 让渡注记纪律与 0bs ⑧ 同：任一段失败都不静默——注记随内容前缀（后续
    /// 段交付时）或失败 `details`（全链失败时）如实回传；浏览器车道的**用量
    /// 事实**经返回值第三项交出（loop 层单账本结算；`None` = 本次未触及该
    /// 车道）。
    pub async fn search_with_serp(
        &self,
        query: &str,
        allowed_domains: Option<Vec<String>>,
        browser_serp: Option<&dyn crate::types::resources::BrowserSerpBackend>,
    ) -> Result<
        (
            String,
            Vec<String>,
            Option<crate::types::resources::BrowserSerpFacts>,
        ),
        xai_tool_runtime::ToolError,
    > {
        // —— 链首：浏览器 SERP（若可用；失败/零命中都让渡、不静默）——
        let mut serp_facts: Option<crate::types::resources::BrowserSerpFacts> = None;
        let mut browser_note: Option<String> = None;
        if let Some(backend) = browser_serp {
            match backend.search(query).await {
                Ok(outcome) => {
                    let mut outcome = outcome;
                    let delivered = crate::types::resources::BrowserSerpFacts {
                        navigations: outcome.navigations,
                        delivered_by_browser: true,
                        engine: Some(outcome.engine.clone()),
                        session_used: outcome.session_used,
                        session_cap: outcome.session_cap,
                    };
                    let fell_back = crate::types::resources::BrowserSerpFacts {
                        navigations: outcome.navigations,
                        delivered_by_browser: false,
                        engine: Some(outcome.engine.clone()),
                        session_used: outcome.session_used,
                        session_cap: outcome.session_cap,
                    };
                    if let Some(domains) = allowed_domains.as_deref().filter(|d| !d.is_empty()) {
                        outcome
                            .hits
                            .retain(|hit| domains.iter().any(|d| host_matches(&hit.url, d)));
                    }
                    if outcome.hits.is_empty() {
                        // 命中被域名过滤耗尽与「零命中」同径让渡（不虚构）。
                        browser_note = Some(format!(
                            "[browser_serp] fell back to local_http: cause=empty engine={} \
                             navigations={} session={}/{} waited={}ms",
                            outcome.engine,
                            outcome.navigations,
                            outcome.session_used,
                            outcome.session_cap,
                            outcome.waited_ms
                        ));
                        serp_facts = Some(fell_back);
                    } else {
                        return Ok((
                            render_browser_serp_content(&outcome),
                            browser_serp_citations(&outcome),
                            Some(delivered),
                        ));
                    }
                }
                Err(failure) => {
                    serp_facts = Some(crate::types::resources::BrowserSerpFacts {
                        navigations: failure.navigations,
                        delivered_by_browser: false,
                        engine: failure.engine.clone(),
                        session_used: failure.session_used,
                        session_cap: failure.session_cap,
                    });
                    browser_note = Some(format!(
                        "[browser_serp] fell back to local_http: cause={} engine={} \
                         navigations={} session={}/{} detail={}",
                        failure.cause,
                        failure.engine.as_deref().unwrap_or("-"),
                        failure.navigations,
                        failure.session_used,
                        failure.session_cap,
                        failure.detail
                    ));
                }
            }
        }
        // 0bs ⑧（2026-09-26 裁决）：本地分段车道由"**整条短路**"改为
        // **链中的一段**——`ORZ_WEB_SEARCH_LOCAL` 语义＝"允许本地 HTTP 车道
        // 进入链路"，不再独占。成功即交付（原语义不变）；失败时**让渡**给
        // provider 合成，让渡注记随内容/失败 details 如实回传（不静默）。
        // （浏览器 SERP 车道为链首的接缝＝session-scoped resource，见报告
        // 记账；此前置未装配时本地 HTTP 就是链首。）
        let mut local_note: Option<String> = None;
        if let (Some(config), Some(http)) =
            (self.local_segmented.as_ref(), self.local_http.as_ref())
        {
            match local_segmented::search(http, config, query).await {
                Ok(mut outcome) => {
                    if let Some(domains) = allowed_domains.as_deref().filter(|d| !d.is_empty()) {
                        outcome
                            .hits
                            .retain(|hit| domains.iter().any(|d| host_matches(&hit.url, d)));
                    }
                    // 链序注记：浏览器段让渡（若有）随本段内容前缀如实回传。
                    let mut content = local_segmented::render_content(&outcome);
                    if let Some(note) = &browser_note {
                        content = format!("{note}\n{content}");
                    }
                    return Ok((content, local_segmented::citations(&outcome), serp_facts));
                }
                Err(local_error) => {
                    local_note = Some(format!(
                        "[local_segmented] fell back to provider: cause={} engine={} \
                         waited={}ms detail={}",
                        local_error.cause,
                        local_error.engine,
                        local_error.waited_ms,
                        local_error.detail
                    ));
                }
            }
        }
        let provider = async move {
            let web_search = rs::WebSearchToolArgs::default()
                .filters(rs::WebSearchToolFilters { allowed_domains })
                .build()
                .map_err(|e| {
                    xai_tool_runtime::ToolError::execution(
                        xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                        format!("Failed to build web search tool: {e}"),
                    )
                })?;
            let request = rs::CreateResponseArgs::default()
                .model(self.model.clone())
                .input(query.to_string())
                .tools(vec![rs::Tool::WebSearch(web_search)])
                .store(false)
                .temperature(0.1_f32)
                .top_p(0.95_f32)
                .max_output_tokens(8192u32)
                .build()
                .map_err(|e| {
                    xai_tool_runtime::ToolError::execution(
                        xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                        format!("Failed to build request: {e}"),
                    )
                })?;
            let url = format!("{}/responses", self.base_url.trim_end_matches('/'));
            let sent_bearer = self.current_bearer().await;
            let mut req = self.http.post(&url).json(&request);
            if let Some(ref key) = sent_bearer {
                req = req.header(AUTHORIZATION, format!("Bearer {key}"));
            }
            let started = std::time::Instant::now();
            let response = req
                .send()
                .await
                .map_err(|e| Self::map_transport_error(e, started.elapsed(), "sending request"))?;
            let status = response.status();
            if status == reqwest::StatusCode::UNAUTHORIZED {
                self.record_401_attribution(sent_bearer.as_deref());
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to read error body".to_string());
                return Err(xai_tool_runtime::ToolError::unauthorized(format!(
                    "Responses API returned 401 Unauthorized: {body}"
                ))
                .with_details(serde_json::json!({
                    "tool_id": "web_search",
                    "status": 401,
                })));
            }
            if !status.is_success() {
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to read error body".to_string());
                return Err(xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Responses API returned {status}: {body}"),
                ));
            }
            let bytes = response.bytes().await.map_err(|e| {
                Self::map_transport_error(e, started.elapsed(), "reading response body")
            })?;
            // 2026-08-11 (direction correction): parsed as raw JSON — the
            // typed `rs::Response` shape does not match the DeepSeek backend
            // (its `web_search_call` search action carries `queries`, while
            // async-openai requires `query`; the typed parse would fail).
            // `response_content`/`extract_citations` walk the raw output.
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Failed to parse response: {e}"),
                )
            })?;
            let content = response_content(&value);
            let content = if content.is_empty() {
                "No search results found.".to_string()
            } else {
                content
            };
            let citations = extract_citations(&value);
            Ok((content, citations))
        }
        .await;
        // 注记链（链序：浏览器 SERP → 本地 HTTP）：任一存在则随交付/失败如实
        // 回传（0bs ⑧ 让渡纪律 + 0bv 链首；不静默、不合成）。
        let notes: Vec<String> = browser_note.into_iter().chain(local_note).collect();
        match (provider, notes.is_empty()) {
            (Ok((content, citations)), false) => Ok((
                format!("{}\n{content}", notes.join("\n")),
                citations,
                serp_facts,
            )),
            (Ok((content, citations)), true) => Ok((content, citations, serp_facts)),
            (Err(error), false) => {
                let mut details = serde_json::Map::new();
                for note in &notes {
                    let key = if note.starts_with("[browser_serp]") {
                        "browser_serp_fallback"
                    } else {
                        "local_segmented_fallback"
                    };
                    details.insert(key.to_string(), serde_json::Value::String(note.clone()));
                }
                Err(error.with_details(serde_json::Value::Object(details)))
            }
            (Err(error), true) => Err(error),
        }
    }
    /// Same as [`Self::search`] but also extracts per-citation titles when
    /// the Responses API surfaces them. Returns `(content, citations_with_titles)`
    /// where each citation is `(title, url)`. Empty `title` strings indicate
    /// the upstream didn't supply one for that URL.
    ///
    /// Used by the cursor-compat `WebSearch` adapter to render a
    /// `Links:\n1. [title](url)` list instead of the LLM synthesis text.
    pub async fn search_with_titles(
        &self,
        query: &str,
        allowed_domains: Option<Vec<String>>,
    ) -> Result<(String, Vec<(String, String)>), xai_tool_runtime::ToolError> {
        // 0bs ⑧：与 [`Self::search`] 同语义——本地 HTTP 车道是链中的一段，
        // 失败让渡 provider（带注记），不再整条短路。
        let mut local_note: Option<String> = None;
        if let (Some(config), Some(http)) =
            (self.local_segmented.as_ref(), self.local_http.as_ref())
        {
            match local_segmented::search(http, config, query).await {
                Ok(mut outcome) => {
                    if let Some(domains) = allowed_domains.as_deref().filter(|d| !d.is_empty()) {
                        outcome
                            .hits
                            .retain(|hit| domains.iter().any(|d| host_matches(&hit.url, d)));
                    }
                    let pairs = outcome
                        .hits
                        .iter()
                        .map(|hit| (hit.title.clone(), hit.url.clone()))
                        .collect();
                    return Ok((local_segmented::render_content(&outcome), pairs));
                }
                Err(local_error) => {
                    local_note = Some(format!(
                        "[local_segmented] fell back to provider: cause={} engine={} \
                         waited={}ms detail={}",
                        local_error.cause,
                        local_error.engine,
                        local_error.waited_ms,
                        local_error.detail
                    ));
                }
            }
        }
        let provider = async move {
            let web_search = rs::WebSearchToolArgs::default()
                .filters(rs::WebSearchToolFilters { allowed_domains })
                .build()
                .map_err(|e| {
                    xai_tool_runtime::ToolError::execution(
                        xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                        format!("Failed to build web search tool: {e}"),
                    )
                })?;
            let request = rs::CreateResponseArgs::default()
                .model(self.model.clone())
                .input(query.to_string())
                .tools(vec![rs::Tool::WebSearch(web_search)])
                .store(false)
                .temperature(0.1_f32)
                .top_p(0.95_f32)
                .max_output_tokens(8192u32)
                .build()
                .map_err(|e| {
                    xai_tool_runtime::ToolError::execution(
                        xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                        format!("Failed to build request: {e}"),
                    )
                })?;
            let url = format!("{}/responses", self.base_url.trim_end_matches('/'));
            let sent_bearer = self.current_bearer().await;
            let mut req = self.http.post(&url).json(&request);
            if let Some(ref key) = sent_bearer {
                req = req.header(AUTHORIZATION, format!("Bearer {key}"));
            }
            let started = std::time::Instant::now();
            let response = req
                .send()
                .await
                .map_err(|e| Self::map_transport_error(e, started.elapsed(), "sending request"))?;
            let status = response.status();
            if status == reqwest::StatusCode::UNAUTHORIZED {
                self.record_401_attribution(sent_bearer.as_deref());
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to read error body".to_string());
                return Err(xai_tool_runtime::ToolError::unauthorized(format!(
                    "Responses API returned 401 Unauthorized: {body}"
                ))
                .with_details(serde_json::json!({
                    "tool_id": "web_search",
                    "status": 401,
                })));
            }
            if !status.is_success() {
                let body = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Failed to read error body".to_string());
                return Err(xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Responses API returned {status}: {body}"),
                ));
            }
            let bytes = response.bytes().await.map_err(|e| {
                Self::map_transport_error(e, started.elapsed(), "reading response body")
            })?;
            // Raw-JSON parse — same rationale as [`Self::search`].
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
                xai_tool_runtime::ToolError::execution(
                    xai_tool_protocol::ToolId::new("web_search").expect("valid"),
                    format!("Failed to parse response: {e}"),
                )
            })?;
            let content = response_content(&value);
            let content = if content.is_empty() {
                "No search results found.".to_string()
            } else {
                content
            };
            let pairs = extract_citation_pairs(&value);
            Ok((content, pairs))
        }
        .await;
        match (provider, local_note) {
            (Ok((content, pairs)), Some(note)) => Ok((format!("{note}\n{content}"), pairs)),
            (Ok(result), None) => Ok(result),
            (Err(error), Some(note)) => Err(error.with_details(serde_json::json!({
                "local_segmented_fallback": note,
            }))),
            (Err(error), None) => Err(error),
        }
    }
}
/// 0ac S3① (2026-09-13): `allowed_domains` filtering for the local segmented
/// path — a hit matches when the URL host equals the domain or is one of its
/// subdomains (leading dot tolerated). The server-side path leaves this to the
/// API filter; locally the contract is enforced here.
fn host_matches(url: &str, domain: &str) -> bool {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('@')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let domain = domain.trim().trim_start_matches('.').to_ascii_lowercase();
    if host.is_empty() || domain.is_empty() {
        return false;
    }
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// 0bv（2026-09-26）：浏览器 SERP 车道的模型面渲染（与本地分段车道同纪律：
/// 逐条 URL、不合成、不补写；头部携带引擎与用量读数，模型可直接引用）。
fn render_browser_serp_content(outcome: &crate::types::resources::BrowserSerpOutcome) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "[browser_serp] engine={} hits={} navigations={} session={}/{}\n",
        outcome.engine,
        outcome.hits.len(),
        outcome.navigations,
        outcome.session_used,
        outcome.session_cap
    ));
    for (index, hit) in outcome.hits.iter().enumerate() {
        out.push_str(&format!("\n{}. {} — {}\n", index + 1, hit.title, hit.url));
        if !hit.snippet.is_empty() {
            out.push_str(&format!("   {}\n", hit.snippet));
        }
    }
    out
}

/// 浏览器车道的引用池（URL，按命中序，去重）。
fn browser_serp_citations(outcome: &crate::types::resources::BrowserSerpOutcome) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for hit in &outcome.hits {
        if !seen.contains(&hit.url) {
            seen.push(hit.url.clone());
        }
    }
    seen
}

/// The `output` array of a Responses API payload (as raw JSON).
fn response_output(response: &serde_json::Value) -> Vec<&serde_json::Value> {
    response
        .get("output")
        .and_then(|o| o.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

/// Concatenated `output_text` content from all message items (mirrors the
/// typed `output_text()` helper on raw JSON).
fn response_content(response: &serde_json::Value) -> String {
    let mut parts: Vec<String> = Vec::new();
    for item in response_output(response) {
        if item.get("type").and_then(|v| v.as_str()) != Some("message") {
            continue;
        }
        if let Some(contents) = item.get("content").and_then(|c| c.as_array()) {
            for c in contents {
                if c.get("type").and_then(|v| v.as_str()) == Some("output_text")
                    && let Some(t) = c.get("text").and_then(|v| v.as_str())
                {
                    parts.push(t.to_string());
                }
            }
        }
    }
    parts.join("")
}

/// Extract citation URLs from the raw Response payload.
///
/// Two backend shapes are merged (2026-08-11 direction correction — the
/// executor targets the DeepSeek Responses API, which serves server-side
/// web search with the SAME key as the main transport):
/// - xAI style: `url_citation` annotations on the output text;
/// - DeepSeek style: `web_search_call` items whose `open_page` action
///   succeeded (annotations come back empty — the opened URLs ARE the
///   citations). A `#ws_call_id=...` fragment DeepSeek appends is stripped.
fn extract_citations(response: &serde_json::Value) -> Vec<String> {
    let mut citations = Vec::new();
    for item in response_output(response) {
        match item.get("type").and_then(|v| v.as_str()) {
            Some("message") => {
                // xAI path: url_citation annotations (DeepSeek: empty).
                for c in item
                    .get("content")
                    .and_then(|c| c.as_array())
                    .into_iter()
                    .flatten()
                {
                    for ann in c
                        .get("annotations")
                        .and_then(|a| a.as_array())
                        .into_iter()
                        .flatten()
                    {
                        if ann.get("type").and_then(|v| v.as_str()) == Some("url_citation")
                            && let Some(url) = ann.get("url").and_then(|v| v.as_str())
                        {
                            citations.push(url.to_string());
                        }
                    }
                }
            }
            Some("web_search_call") => {
                if item.get("status").and_then(|v| v.as_str()) == Some("completed")
                    && item
                        .get("action")
                        .and_then(|a| a.get("type"))
                        .and_then(|v| v.as_str())
                        == Some("open_page")
                    && let Some(url) = item
                        .get("action")
                        .and_then(|a| a.get("url"))
                        .and_then(|v| v.as_str())
                {
                    citations.push(url.split('#').next().unwrap_or(url).trim().to_string());
                }
            }
            _ => {}
        }
    }
    let mut seen = std::collections::HashSet::new();
    citations.retain(|url| !url.is_empty() && seen.insert(url.clone()));
    citations
}
/// Extract `(title, url)` pairs from the Responses API output.
///
/// Same backend merge as [`extract_citations`] (2026-08-11): annotation
/// titles (xAI) plus DeepSeek `web_search_call` `open_page` URLs (title is
/// empty — DeepSeek does not surface per-page titles). `title` may be an
/// empty string when upstream doesn't supply one. URLs are deduplicated
/// while preserving the first-seen order so the rendered `Links:` list is
/// stable and free of duplicates.
fn extract_citation_pairs(response: &serde_json::Value) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for item in response_output(response) {
        match item.get("type").and_then(|v| v.as_str()) {
            Some("message") => {
                // xAI path: annotation titles + urls.
                for c in item
                    .get("content")
                    .and_then(|c| c.as_array())
                    .into_iter()
                    .flatten()
                {
                    for ann in c
                        .get("annotations")
                        .and_then(|a| a.as_array())
                        .into_iter()
                        .flatten()
                    {
                        if ann.get("type").and_then(|v| v.as_str()) == Some("url_citation")
                            && let Some(url) = ann.get("url").and_then(|v| v.as_str())
                        {
                            if url.is_empty() {
                                continue;
                            }
                            let title = ann
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            pairs.push((title, url.to_string()));
                        }
                    }
                }
            }
            Some("web_search_call") => {
                // DeepSeek path: opened page URLs (no titles upstream).
                if item.get("status").and_then(|v| v.as_str()) == Some("completed")
                    && item
                        .get("action")
                        .and_then(|a| a.get("type"))
                        .and_then(|v| v.as_str())
                        == Some("open_page")
                    && let Some(url) = item
                        .get("action")
                        .and_then(|a| a.get("url"))
                        .and_then(|v| v.as_str())
                {
                    let clean = url.split('#').next().unwrap_or(url).trim().to_string();
                    if !clean.is_empty() {
                        pairs.push((String::new(), clean));
                    }
                }
            }
            _ => {}
        }
    }
    let mut seen = std::collections::HashSet::new();
    pairs.retain(|(_t, url)| seen.insert(url.clone()));
    pairs
}
#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    /// Helper to create a raw Response payload for testing (the parse is
    /// raw JSON since the typed shape does not match DeepSeek).
    fn response_from_json(json: serde_json::Value) -> serde_json::Value {
        json
    }
    #[test]
    fn test_new_client_uses_configured_model() {
        let config = WebSearchConfig::Enabled {
            api_key: "test-key".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            model: "custom-enterprise-model".to_string(),
            extra_headers: IndexMap::new(),
            alpha_test_key: None,
        };
        let client = WebSearchClient::new(&config, None).expect("client should build");
        assert_eq!(client.model, "custom-enterprise-model");
    }
    /// THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1 审查处理 P3-1):
    /// transport 超时经 `map_transport_error` 映射为结构化 Timeout 类别，
    /// 携带已用时长与后续建议——本地 listener 只收不应答 + 50ms 客户端
    /// 总超时确定性构造（不依赖不可路由地址；1ms 在慢速 CI/沙箱下会与
    /// connect 阶段竞态，导致「非超时」传输错误）。
    #[tokio::test]
    async fn map_transport_error_marks_client_timeouts() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            // Accept ONE connection and HOLD it open without responding —
            // dropping the accepted socket would close the connection
            // server-side and fail the request with a non-timeout
            // `IncompleteMessage` instead of the client's total timeout.
            // The task ends when the test runtime shuts down.
            let _conn = listener.accept().await.ok();
            std::future::pending::<()>().await;
        });
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(50))
            .connect_timeout(Duration::from_millis(1000))
            .build()
            .expect("client");
        let started = std::time::Instant::now();
        let err = client
            .get(format!("http://{addr}/"))
            .send()
            .await
            .expect_err("request must time out");
        assert!(err.is_timeout(), "expected a timeout error: {err}");
        let mapped =
            WebSearchClient::map_transport_error(err, started.elapsed(), "sending request");
        assert_eq!(mapped.kind, xai_tool_runtime::ToolErrorKind::Timeout);
        let text = mapped.to_string();
        assert!(text.contains("timed out after"), "{text}");
        assert!(text.contains("Retry with a narrower query"), "{text}");
    }

    /// 非超时 transport 失败保持 Execution 类别并携带阶段上下文
    /// （构造性非法 URL 在请求构建/发送阶段即失败）。
    #[tokio::test]
    async fn map_transport_error_keeps_execution_kind_with_stage() {
        let client = reqwest::Client::builder().build().expect("client");
        let started = std::time::Instant::now();
        let err = client
            .get("http://[::1") // invalid URL — transport failure, not timeout
            .send()
            .await
            .expect_err("request must fail");
        assert!(!err.is_timeout(), "{err}");
        let mapped =
            WebSearchClient::map_transport_error(err, started.elapsed(), "sending request");
        assert_eq!(mapped.kind, xai_tool_runtime::ToolErrorKind::Execution);
        let text = mapped.to_string();
        assert!(text.contains("while sending request"), "{text}");
    }
    /// Counts attribution callback invocations for the test below.
    #[derive(Default, Debug)]
    struct CountingCallback {
        invocations: std::sync::Mutex<Vec<(ToolConsumer, Option<String>)>>,
    }
    impl crate::attribution::Auth401AttributionCallback for CountingCallback {
        fn record_401(&self, consumer: ToolConsumer, sent_bearer_prefix: Option<&str>) {
            self.invocations
                .lock()
                .unwrap()
                .push((consumer, sent_bearer_prefix.map(|s| s.to_string())));
        }
    }
    /// `record_401_attribution` invokes the wired callback with
    /// `ToolConsumer::WebSearch` and the truncated bearer prefix.
    /// The full bearer never crosses the trait boundary.
    #[test]
    fn record_401_attribution_passes_truncated_prefix_to_callback() {
        let cb = std::sync::Arc::new(CountingCallback::default());
        let cb_dyn: crate::attribution::SharedAttributionCallback = cb.clone();
        let config = WebSearchConfig::Enabled {
            api_key: "ignored".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            model: "test-model".to_string(),
            extra_headers: IndexMap::new(),
            alpha_test_key: None,
        };
        let client = WebSearchClient::new(&config, None)
            .expect("client should build")
            .with_attribution_callback(Some(cb_dyn));
        client.record_401_attribution(Some("bearer-with-long-tail-aaaaaaaaaa"));
        let calls = cb.invocations.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, ToolConsumer::WebSearch);
        assert_eq!(calls[0].1.as_deref(), Some("bearer-with-"));
        assert_eq!(
            calls[0].1.as_deref().map(str::len),
            Some(crate::attribution::SENT_BEARER_PREFIX_LEN),
        );
    }
    /// `record_401_attribution` is a no-op when no callback is wired
    /// -- the BYOK / standalone case must not panic or allocate.
    #[test]
    fn record_401_attribution_is_noop_without_callback() {
        let config = WebSearchConfig::Enabled {
            api_key: "test-key".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            model: "test-model".to_string(),
            extra_headers: IndexMap::new(),
            alpha_test_key: None,
        };
        let client = WebSearchClient::new(&config, None).expect("client should build");
        client.record_401_attribution(Some("any-bearer"));
        client.record_401_attribution(None);
    }
    #[test]
    fn test_extract_citations_empty_response() {
        let response = response_from_json(serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "output": [],
            "model": "test-model"
        }));
        let citations = extract_citations(&response);
        assert!(citations.is_empty());
    }
    #[test]
    fn test_extract_citations_with_url_citations() {
        let response = response_from_json(serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "model": "test-model",
            "output": [
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Here is some info about Rust.",
                            "annotations": [
                                {
                                    "type": "url_citation",
                                    "url": "https://www.rust-lang.org/",
                                    "title": "Rust Programming Language",
                                    "start_index": 0,
                                    "end_index": 10
                                },
                                {
                                    "type": "url_citation",
                                    "url": "https://docs.rs/",
                                    "title": "Docs.rs",
                                    "start_index": 11,
                                    "end_index": 20
                                }
                            ]
                        }
                    ]
                }
            ]
        }));
        let citations = extract_citations(&response);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0], "https://www.rust-lang.org/");
        assert_eq!(citations[1], "https://docs.rs/");
    }
    #[test]
    fn test_extract_citations_deduplicates() {
        let response = response_from_json(serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "model": "test-model",
            "output": [
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Info with duplicate citations.",
                            "annotations": [
                                {
                                    "type": "url_citation",
                                    "url": "https://example.com/page1",
                                    "title": "Page 1",
                                    "start_index": 0,
                                    "end_index": 5
                                },
                                {
                                    "type": "url_citation",
                                    "url": "https://example.com/page2",
                                    "title": "Page 2",
                                    "start_index": 6,
                                    "end_index": 10
                                },
                                {
                                    "type": "url_citation",
                                    "url": "https://example.com/page1",
                                    "title": "Page 1 Again",
                                    "start_index": 11,
                                    "end_index": 15
                                }
                            ]
                        }
                    ]
                }
            ]
        }));
        let citations = extract_citations(&response);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0], "https://example.com/page1");
        assert_eq!(citations[1], "https://example.com/page2");
    }
    #[test]
    fn test_extract_citations_multiple_messages() {
        let response = response_from_json(serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "model": "test-model",
            "output": [
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "First message",
                            "annotations": [
                                {
                                    "type": "url_citation",
                                    "url": "https://first.com/",
                                    "title": "First",
                                    "start_index": 0,
                                    "end_index": 5
                                }
                            ]
                        }
                    ]
                },
                {
                    "type": "message",
                    "id": "msg_2",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Second message",
                            "annotations": [
                                {
                                    "type": "url_citation",
                                    "url": "https://second.com/",
                                    "title": "Second",
                                    "start_index": 0,
                                    "end_index": 6
                                }
                            ]
                        }
                    ]
                }
            ]
        }));
        let citations = extract_citations(&response);
        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0], "https://first.com/");
        assert_eq!(citations[1], "https://second.com/");
    }
    #[test]
    fn test_extract_citations_ignores_non_url_annotations() {
        let response = response_from_json(serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "model": "test-model",
            "output": [
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Some text",
                            "annotations": [
                                {
                                    "type": "url_citation",
                                    "url": "https://valid.com/",
                                    "title": "Valid",
                                    "start_index": 0,
                                    "end_index": 4
                                }
                            ]
                        }
                    ]
                }
            ]
        }));
        let citations = extract_citations(&response);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0], "https://valid.com/");
    }
    /// A provider that always returns `None`, simulating an API-key user
    /// whose token has aged past the client-side TTL.
    struct NoneProvider;
    impl crate::types::ApiKeyProvider for NoneProvider {
        fn current_api_key(&self) -> Option<String> {
            None
        }
    }
    /// When the dynamic provider returns `None`, the static `api_key`
    /// from config must still be sent as the Authorization header.
    /// This is a regression scenario: API-key users
    /// past the 30-day client TTL saw 401 because no auth was sent.
    #[tokio::test]
    async fn static_api_key_is_fallback_when_provider_returns_none() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(header("Authorization", "Bearer static-key-from-config"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_test",
                "object": "response",
                "created_at": 1234567890,
                "status": "completed",
                "model": "test-model",
                "output": [{
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [{
                        "type": "output_text",
                        "text": "search result",
                        "annotations": []
                    }]
                }]
            })))
            .mount(&server)
            .await;
        let config = WebSearchConfig::Enabled {
            api_key: "static-key-from-config".to_string(),
            base_url: server.uri(),
            model: "test-model".to_string(),
            extra_headers: IndexMap::new(),
            alpha_test_key: None,
        };
        let provider: SharedApiKeyProvider = std::sync::Arc::new(NoneProvider);
        let client = WebSearchClient::new(&config, Some(provider)).expect("client should build");
        let (content, _citations) = client
            .search("test query", None)
            .await
            .expect("search must succeed with static key fallback");
        assert_eq!(content, "search result");
    }
    /// When the provider returns a fresh key, it overrides the static one.
    #[tokio::test]
    async fn provider_key_overrides_static_key() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        struct FreshProvider;
        impl crate::types::ApiKeyProvider for FreshProvider {
            fn current_api_key(&self) -> Option<String> {
                Some("fresh-key-from-provider".to_string())
            }
        }
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(header("Authorization", "Bearer fresh-key-from-provider"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_test",
                "object": "response",
                "created_at": 1234567890,
                "status": "completed",
                "model": "test-model",
                "output": [{
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [{
                        "type": "output_text",
                        "text": "fresh result",
                        "annotations": []
                    }]
                }]
            })))
            .mount(&server)
            .await;
        let config = WebSearchConfig::Enabled {
            api_key: "stale-static-key".to_string(),
            base_url: server.uri(),
            model: "test-model".to_string(),
            extra_headers: IndexMap::new(),
            alpha_test_key: None,
        };
        let provider: SharedApiKeyProvider = std::sync::Arc::new(FreshProvider);
        let client = WebSearchClient::new(&config, Some(provider)).expect("client should build");
        let (content, _citations) = client
            .search("test query", None)
            .await
            .expect("search must succeed with provider key");
        assert_eq!(content, "fresh result");
    }
    #[test]
    fn test_extract_citations_no_annotations() {
        let response = response_from_json(serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "model": "test-model",
            "output": [
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Plain text with no annotations",
                            "annotations": []
                        }
                    ]
                }
            ]
        }));
        let citations = extract_citations(&response);
        assert!(citations.is_empty());
    }

    /// 2026-08-11 (direction correction): the DeepSeek Responses backend
    /// returns citations in `web_search_call` items (annotations come back
    /// empty) — completed `open_page` URLs are the citations, `search`
    /// actions and failed pages are not, and the appended
    /// `#ws_call_id=...` fragment is stripped.
    #[test]
    fn test_extract_citations_deepseek_web_search_call() {
        let response = response_from_json(serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "created_at": 1234567890,
            "status": "completed",
            "model": "deepseek-v4-flash",
            "output": [
                {
                    "type": "web_search_call",
                    "id": "call_00",
                    "status": "completed",
                    "action": {"type": "search", "queries": ["xAI 2026 release"]}
                },
                {
                    "type": "web_search_call",
                    "id": "call_01",
                    "status": "completed",
                    "action": {"type": "open_page", "url": "https://example.com/a#ws_call_id=call_01"}
                },
                {
                    "type": "web_search_call",
                    "id": "call_02",
                    "status": "failed",
                    "action": {"type": "open_page", "url": "https://blocked.example.com"}
                },
                {
                    "type": "web_search_call",
                    "id": "call_03",
                    "status": "completed",
                    "action": {"type": "open_page", "url": "https://example.com/a"}
                },
                {
                    "type": "message",
                    "id": "msg_1",
                    "status": "completed",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "综合结果", "annotations": []}]
                }
            ]
        }));
        let citations = extract_citations(&response);
        assert_eq!(citations, vec!["https://example.com/a"]);
    }

    /// 2026-08-11: end-to-end search() over a DeepSeek-shaped response —
    /// content from the final output_text, citations from the completed
    /// open_page calls.
    #[tokio::test]
    async fn deepseek_search_parses_web_search_call_citations() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "resp_ds",
                "object": "response",
                "created_at": 1234567890,
                "status": "completed",
                "model": "deepseek-v4-flash",
                "output": [
                    {
                        "type": "web_search_call",
                        "id": "call_00",
                        "status": "completed",
                        "action": {"type": "search", "queries": ["query"]}
                    },
                    {
                        "type": "web_search_call",
                        "id": "call_01",
                        "status": "completed",
                        "action": {"type": "open_page", "url": "https://deepseek-served.example/1#ws_call_id=call_01"}
                    },
                    {
                        "type": "message",
                        "id": "msg_1",
                        "status": "completed",
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": "搜索结果综合", "annotations": []}]
                    }
                ]
            })))
            .mount(&server)
            .await;
        let config = WebSearchConfig::Enabled {
            api_key: "deepseek-key".to_string(),
            base_url: server.uri(),
            model: "deepseek-v4-flash".to_string(),
            extra_headers: IndexMap::new(),
            alpha_test_key: None,
        };
        let client = WebSearchClient::new(&config, None).expect("client should build");
        let (content, citations) = client
            .search("test query", None)
            .await
            .expect("search must succeed");
        assert_eq!(content, "搜索结果综合");
        assert_eq!(citations, vec!["https://deepseek-served.example/1"]);
    }
}
