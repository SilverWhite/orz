//! 0ac S3① (2026-09-13, design §9/§10): the **local segmented retrieval**
//! front-end — `web_search` 的本地后端。
//!
//! 裁决链（用户 2026-09-13）：
//! - 服务端 web_search（`/responses` + 服务端工具）已被 DeepSeek 下架
//!   （`0AC_S1_PROBE_RECORD` §5/§6）⇒ 主路径改**本地分段检索**：
//!   SERP 引擎链 → 逐页抓取 → 逐段抽取；每段独立结果/截止
//!   （design §9.1）。
//! - 引擎集：**Bing HTML 直连单引擎为默认**（直连 TTFB 0.4s，S1′ 代理
//!   假象更正；DDG/Google 直连不可达，仅在显式配置时进入引擎链，§9.4）。
//! - 截止：**每引擎独立计时 10s**（`ORZ_RETRIEVAL_DEADLINE_MS` 可配）+
//!   **整体兜底 30s**（`ORZ_RETRIEVAL_OVERALL_DEADLINE_MS` 可配）——
//!   不共用一个钟（§9.4 用户裁决原文）。
//! - SERP 提取器**按现行 `b_algo` 结构重写**（S1′ 实测：h2/a 属性序变了，
//!   旧正则过期；旧 CDP 选择器语义 `#b_results > li.b_algo` 保留）。
//! - 失败自描述：稳定码 `capability_unreachable` / `network_no_response` /
//!   `network_error` / `empty_result` / `no_progress`（S2 schema 闭枚举，
//!   §10.1），随 `cause` 附进 `ToolError` details，供 S3①的
//!   `tool_completed.cause` 写点读取（F-003 验收样本）。
//! - **开关**：`ORZ_WEB_SEARCH_LOCAL`（默认关闭直至复验；§10.3 item 1）。

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use base64::Engine as _;
use regex::Regex;

/// 开关（默认关闭；设计 §10.3：全部带开关，默认关闭直至复验）。
pub const ENV_SWITCH: &str = "ORZ_WEB_SEARCH_LOCAL";
/// 每引擎独立截止（ms），默认 10_000（§9.4）。
pub const ENV_PER_ENGINE_DEADLINE_MS: &str = "ORZ_RETRIEVAL_DEADLINE_MS";
/// 整体兜底截止（ms），默认 30_000（§9.4）。
pub const ENV_OVERALL_DEADLINE_MS: &str = "ORZ_RETRIEVAL_OVERALL_DEADLINE_MS";
/// 引擎链（逗号分隔 id；默认 `bing_cn` 单引擎，§9.4/§9.6）。
pub const ENV_ENGINES: &str = "ORZ_RETRIEVAL_ENGINES";
/// 逐页抓取段抽取的页数上限（默认 3；0 = 只回 SERP 命中，不抓页）。
pub const ENV_SEGMENT_PAGES: &str = "ORZ_RETRIEVAL_SEGMENT_PAGES";

pub const DEFAULT_PER_ENGINE_DEADLINE_MS: u64 = 10_000;
pub const DEFAULT_OVERALL_DEADLINE_MS: u64 = 30_000;
pub const DEFAULT_SEGMENT_PAGES: usize = 3;
/// 单页段数上限与单段字节上限（防超大页拖垮整体兜底）。
pub const MAX_SEGMENTS_PER_PAGE: usize = 5;
pub const MAX_SEGMENT_CHARS: usize = 1_200;

// ── 稳定码（S2 schema 闭枚举；§10.1）─────────────────────────────────────
pub const CAUSE_CAPABILITY_UNREACHABLE: &str = "capability_unreachable";
pub const CAUSE_NETWORK_NO_RESPONSE: &str = "network_no_response";
pub const CAUSE_NETWORK_ERROR: &str = "network_error";
pub const CAUSE_EMPTY_RESULT: &str = "empty_result";
pub const CAUSE_NO_PROGRESS: &str = "no_progress";

/// The engine registry: id → search URL template (`{query}` placeholder).
///
/// `bing_cn`/`bing_global` are the 0v SERP 语义资产（`#b_results > li.b_algo`
/// 同族页面）；`duckduckgo`/`google` 直连不可达，仅在显式
/// `ORZ_RETRIEVAL_ENGINES`（代理形态）下进入引擎链（§9.4 第 4 条/§9.6）。
pub const ENGINE_REGISTRY: &[(&str, &str)] = &[
    ("bing_cn", "https://cn.bing.com/search?q={query}&count=10"),
    ("bing_global", "https://www.bing.com/search?q={query}&count=10"),
    (
        "duckduckgo",
        "https://html.duckduckgo.com/html/?q={query}",
    ),
    ("google", "https://www.google.com/search?q={query}"),
];

/// One engine in the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineSpec {
    pub id: String,
    pub search_url: String,
}

/// The local segmented retrieval configuration (switch + engine chain +
/// deadlines). Built from the environment once at tool-registry construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSegmentedConfig {
    enabled: bool,
    pub engines: Vec<EngineSpec>,
    pub per_engine_deadline: Duration,
    pub overall_deadline: Duration,
    pub segment_pages: usize,
}

impl Default for LocalSegmentedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            engines: vec![EngineSpec {
                id: "bing_cn".to_string(),
                search_url: ENGINE_REGISTRY[0].1.to_string(),
            }],
            per_engine_deadline: Duration::from_millis(DEFAULT_PER_ENGINE_DEADLINE_MS),
            overall_deadline: Duration::from_millis(DEFAULT_OVERALL_DEADLINE_MS),
            segment_pages: DEFAULT_SEGMENT_PAGES,
        }
    }
}

impl LocalSegmentedConfig {
    /// Read the switch and tuning knobs from the process environment.
    pub fn from_env() -> Self {
        Self::from_env_with(|key| std::env::var(key).ok())
    }

    /// Environment-independent seam (tests never mutate the process env —
    /// the crate's test binary runs in parallel).
    pub fn from_env_with(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut config = Self::default();
        config.enabled = get(ENV_SWITCH)
            .map(|v| {
                let v = v.trim().to_ascii_lowercase();
                matches!(v.as_str(), "1" | "true" | "on" | "yes")
            })
            .unwrap_or(false);
        if let Some(ms) = get(ENV_PER_ENGINE_DEADLINE_MS)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|ms| *ms > 0)
        {
            config.per_engine_deadline = Duration::from_millis(ms);
        }
        if let Some(ms) = get(ENV_OVERALL_DEADLINE_MS)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|ms| *ms > 0)
        {
            config.overall_deadline = Duration::from_millis(ms);
        }
        if let Some(spec) = get(ENV_SEGMENT_PAGES).and_then(|v| v.trim().parse::<usize>().ok()) {
            config.segment_pages = spec;
        }
        if let Some(list) = get(ENV_ENGINES) {
            let engines: Vec<EngineSpec> = list
                .split(',')
                .filter_map(|id| {
                    let id = id.trim().to_ascii_lowercase();
                    ENGINE_REGISTRY
                        .iter()
                        .find(|(known, _)| *known == id)
                        .map(|(known, template)| EngineSpec {
                            id: (*known).to_string(),
                            search_url: (*template).to_string(),
                        })
                })
                .collect();
            if !engines.is_empty() {
                config.engines = engines;
            }
        }
        config
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The engine-chain reading for the `retrieval_family` probe
    /// (`retrieval_family.search_engine.detail`).
    pub fn engine_chain_detail(&self) -> String {
        self.engines
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }
}

// ── SERP 提取（按现行 b_algo 结构重写）──────────────────────────────────

/// One organic SERP hit (pre page-fetch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerpHit {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// A delivered result with its extracted segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedHit {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub snippet: String,
    /// 逐段抽取的正文段（空 = 页抓取失败/无段；SERP 命中仍交付）。
    pub segments: Vec<String>,
}

/// One engine attempt (the A/B 记录面 + `no_progress` 判据的输入)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineAttempt {
    pub engine: String,
    pub waited_ms: u64,
    /// `ok` | `empty` | 稳定码。
    pub outcome: String,
}

/// A successful local segmented retrieval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedOutcome {
    pub engine: String,
    pub hits: Vec<SegmentedHit>,
    pub waited_ms: u64,
    pub attempts: Vec<EngineAttempt>,
}

/// A failed local segmented retrieval — self-describing: stable code, engine,
/// human detail, waited time (§10.3 item 1 / F-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedError {
    pub cause: &'static str,
    pub engine: String,
    pub detail: String,
    pub waited_ms: u64,
    pub attempts: Vec<EngineAttempt>,
}

impl SegmentedError {
    pub fn code(&self) -> &'static str {
        self.cause
    }

    /// Structured tool error: the stable code rides `details.cause` so the
    /// loop-side `tool_completed` writer can self-describe the failure
    /// (壳码拒绝集见 S2 schema `not.enum`).
    pub fn to_tool_error(&self) -> xai_tool_runtime::ToolError {
        let tool_id = xai_tool_protocol::ToolId::new("web_search").expect("valid");
        let message = format!(
            "local segmented retrieval failed ({}) — engine={} waited={}ms: {}",
            self.cause, self.engine, self.waited_ms, self.detail
        );
        let error = if self.cause == CAUSE_NETWORK_NO_RESPONSE {
            xai_tool_runtime::ToolError::timeout(tool_id, message)
        } else {
            xai_tool_runtime::ToolError::execution(tool_id, message)
        };
        error.with_details(serde_json::json!({
            "tool_id": "web_search",
            "retrieval_path": "local_segmented",
            "cause": self.cause,
            "engine": self.engine,
            "waited_ms": self.waited_ms,
            "attempts": self
                .attempts
                .iter()
                .map(|a| serde_json::json!({
                    "engine": a.engine,
                    "waited_ms": a.waited_ms,
                    "outcome": a.outcome,
                }))
                .collect::<Vec<_>>(),
        }))
    }
}

static BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    // 现行结构（S1′ 实测）：`<li class="b_algo" …>` 有机块；广告块
    // `b_ad` 不含 `b_algo`（`class="b_ad"`/`b_adLast`），天然被排除。
    Regex::new(r#"(?is)<li[^>]*class="[^"]*\bb_algo\b[^"]*"[^>]*>(.*?)</li>"#)
        .expect("valid SERP block regex")
});
static TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    // 属性序不再固定（S1′ 教训）：h2 → 第一个带 href 的 a，属性任意序。
    Regex::new(r#"(?is)<h2[^>]*>\s*<a\b([^>]*)>(.*?)</a>"#)
        .expect("valid SERP title regex")
});
static HREF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)href="([^"]*)""#).expect("valid href regex"));
static SNIPPET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<p[^>]*class="[^"]*(?:b_lineclamp|b_paractl)[^"]*"[^>]*>(.*?)</p>"#)
        .expect("valid snippet regex")
});
static CAPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<div[^>]*class="[^"]*b_caption[^"]*"[^>]*>(.*?)</div>"#)
        .expect("valid caption regex")
});
static P_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<p[^>]*>(.*?)</p>"#).expect("valid p regex"));
static TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<[^>]+>"#).expect("valid tag regex"));
static WS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").expect("valid whitespace regex"));
static BING_REDIRECT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)[?&]u=a1([A-Za-z0-9_\-]+)"#).expect("valid redirect regex")
});

/// Parse the organic results out of a Bing SERP HTML page (the 0v selector
/// semantics `#b_results > li.b_algo`, on raw HTML instead of the DOM).
pub fn parse_bing_serp(html: &str) -> Vec<SerpHit> {
    let mut hits = Vec::new();
    for block in BLOCK_RE.captures_iter(html) {
        let block = block.get(1).map(|m| m.as_str()).unwrap_or_default();
        let Some(title_caps) = TITLE_RE.captures(block) else {
            continue;
        };
        let attrs = title_caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let raw_title = title_caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        let Some(href) = HREF_RE
            .captures(attrs)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
        else {
            continue;
        };
        let title = decode_html_entities(&strip_tags(raw_title));
        let url = normalize_hit_url(&decode_html_entities(href));
        if title.is_empty() || !url.starts_with("http") {
            continue;
        }
        let snippet = SNIPPET_RE
            .captures(block)
            .or_else(|| CAPTION_RE.captures(block))
            .or_else(|| P_RE.captures(block))
            .and_then(|c| c.get(1))
            .map(|m| decode_html_entities(&strip_tags(m.as_str())))
            .unwrap_or_default();
        hits.push(SerpHit {
            title,
            url,
            snippet,
        });
    }
    hits
}

/// Tags → single spaces, whitespace collapsed.
pub fn strip_tags(fragment: &str) -> String {
    let without_tags = TAG_RE.replace_all(fragment, " ");
    WS_RE.replace_all(&without_tags, " ").trim().to_string()
}

/// Minimal HTML entity decoding (the SERP surface set).
pub fn decode_html_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        let Some(semi) = rest[..rest.len().min(12)].find(';') else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "amp" => Some("&".to_string()),
            "lt" => Some("<".to_string()),
            "gt" => Some(">".to_string()),
            "quot" => Some("\"".to_string()),
            "apos" | "#39" | "#x27" | "#X27" => Some("'".to_string()),
            "nbsp" => Some(" ".to_string()),
            other if other.starts_with('#') => other[1..]
                .trim_start_matches(['x', 'X'])
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .map(|c| c.to_string()),
            _ => None,
        };
        match decoded {
            Some(decoded) => {
                out.push_str(&decoded);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Unwrap Bing's `bing.com/ck/a?…&u=a1<base64url>` redirect wrapper into the
/// real target URL; everything else passes through unchanged.
pub fn normalize_hit_url(url: &str) -> String {
    if !url.contains("bing.com/ck/a") {
        return url.to_string();
    }
    let Some(caps) = BING_REDIRECT_RE.captures(url) else {
        return url.to_string();
    };
    let encoded = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
    let padded = format!(
        "{}{}",
        encoded,
        "=".repeat((4 - encoded.len() % 4) % 4)
    );
    for engine in [
        base64::engine::general_purpose::URL_SAFE,
        base64::engine::general_purpose::URL_SAFE_NO_PAD,
        base64::engine::general_purpose::STANDARD,
    ] {
        if let Ok(bytes) = engine.decode(padded.as_bytes())
            && let Ok(text) = String::from_utf8(bytes)
            && text.starts_with("http")
        {
            return text;
        }
    }
    url.to_string()
}

/// Extract readable text segments from a fetched page (`<p>` blocks, bounded).
pub fn extract_segments(html: &str) -> Vec<String> {
    let document = scraper::Html::parse_document(html);
    let Ok(selector) = scraper::Selector::parse("p") else {
        return Vec::new();
    };
    let mut segments = Vec::new();
    for node in document.select(&selector) {
        let text = node.text().collect::<String>();
        let text = WS_RE.replace_all(text.trim(), " ").to_string();
        if text.chars().count() < 40 {
            continue;
        }
        let mut bounded: String = text.chars().take(MAX_SEGMENT_CHARS).collect();
        if text.chars().count() > MAX_SEGMENT_CHARS {
            bounded.push('…');
        }
        segments.push(bounded);
        if segments.len() >= MAX_SEGMENTS_PER_PAGE {
            break;
        }
    }
    segments
}

/// `application/x-www-form-urlencoded`-style query encoding (no extra deps).
pub fn urlencode_query(query: &str) -> String {
    let mut out = String::with_capacity(query.len());
    for byte in query.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

// ── 执行（每引擎独立截止 + 整体兜底）────────────────────────────────────

/// Run the local segmented retrieval: engine chain → SERP → 逐页抓取 → 逐段抽取.
///
/// Deadlines (design §9.4): 每个引擎**独立** `per_engine_deadline`（默认
/// 10s，`ORZ_RETRIEVAL_DEADLINE_MS` 可配）；整体兜底 `overall_deadline`
/// （默认 30s）。整体预算耗尽即返结构化错误（`network_no_response`），
/// 不等引擎自身的 120s 黑洞。
pub async fn search(
    http: &reqwest::Client,
    config: &LocalSegmentedConfig,
    query: &str,
) -> Result<SegmentedOutcome, SegmentedError> {
    let started = Instant::now();
    let mut attempts: Vec<EngineAttempt> = Vec::new();
    let mut last: Option<SegmentedError> = None;
    for engine in &config.engines {
        let remaining = config.overall_deadline.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            last = Some(SegmentedError {
                cause: CAUSE_NETWORK_NO_RESPONSE,
                engine: engine.id.clone(),
                detail: format!(
                    "overall deadline {}ms exhausted before engine {} started",
                    config.overall_deadline.as_millis(),
                    engine.id
                ),
                waited_ms: started.elapsed().as_millis() as u64,
                attempts: attempts.clone(),
            });
            break;
        }
        let budget = config.per_engine_deadline.min(remaining);
        let engine_started = Instant::now();
        let url = engine
            .search_url
            .replace("{query}", &urlencode_query(query));
        let outcome =
            tokio::time::timeout(budget, fetch_engine(http, config, engine, &url, started)).await;
        match outcome {
            Err(_elapsed) => {
                attempts.push(EngineAttempt {
                    engine: engine.id.clone(),
                    waited_ms: engine_started.elapsed().as_millis() as u64,
                    outcome: CAUSE_NETWORK_NO_RESPONSE.to_string(),
                });
                last = Some(SegmentedError {
                    cause: CAUSE_NETWORK_NO_RESPONSE,
                    engine: engine.id.clone(),
                    detail: format!(
                        "engine deadline {}ms exhausted (per-engine clock, no shared budget)",
                        budget.as_millis()
                    ),
                    waited_ms: engine_started.elapsed().as_millis() as u64,
                    attempts: attempts.clone(),
                });
            }
            Ok(Ok(hits)) => {
                if hits.is_empty() {
                    attempts.push(EngineAttempt {
                        engine: engine.id.clone(),
                        waited_ms: engine_started.elapsed().as_millis() as u64,
                        outcome: CAUSE_EMPTY_RESULT.to_string(),
                    });
                    last = Some(SegmentedError {
                        cause: CAUSE_EMPTY_RESULT,
                        engine: engine.id.clone(),
                        detail: "SERP parsed but carried no organic b_algo hit".to_string(),
                        waited_ms: engine_started.elapsed().as_millis() as u64,
                        attempts: attempts.clone(),
                    });
                    continue;
                }
                attempts.push(EngineAttempt {
                    engine: engine.id.clone(),
                    waited_ms: engine_started.elapsed().as_millis() as u64,
                    outcome: "ok".to_string(),
                });
                return Ok(SegmentedOutcome {
                    engine: engine.id.clone(),
                    hits,
                    waited_ms: started.elapsed().as_millis() as u64,
                    attempts,
                });
            }
            Ok(Err(error)) => {
                attempts.push(EngineAttempt {
                    engine: engine.id.clone(),
                    waited_ms: error.waited_ms,
                    outcome: error.cause.to_string(),
                });
                last = Some(SegmentedError {
                    attempts: attempts.clone(),
                    ..error
                });
            }
        }
    }
    Err(last.unwrap_or(SegmentedError {
        cause: CAUSE_NO_PROGRESS,
        engine: "none".to_string(),
        detail: "engine chain is empty (no engine configured)".to_string(),
        waited_ms: started.elapsed().as_millis() as u64,
        attempts,
    }))
}

async fn fetch_engine(
    http: &reqwest::Client,
    config: &LocalSegmentedConfig,
    engine: &EngineSpec,
    url: &str,
    overall_started: Instant,
) -> Result<Vec<SegmentedHit>, SegmentedError> {
    let engine_started = Instant::now();
    let response = http.get(url).send().await.map_err(|e| {
        let cause = if e.is_connect() {
            CAUSE_CAPABILITY_UNREACHABLE
        } else {
            CAUSE_NETWORK_ERROR
        };
        SegmentedError {
            cause,
            engine: engine.id.clone(),
            detail: format!("request failed: {e}"),
            waited_ms: engine_started.elapsed().as_millis() as u64,
            attempts: Vec::new(),
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(SegmentedError {
            cause: CAUSE_NETWORK_ERROR,
            engine: engine.id.clone(),
            detail: format!("SERP returned HTTP {status}"),
            waited_ms: engine_started.elapsed().as_millis() as u64,
            attempts: Vec::new(),
        });
    }
    let html = response.text().await.map_err(|e| SegmentedError {
        cause: CAUSE_NETWORK_ERROR,
        engine: engine.id.clone(),
        detail: format!("reading SERP body failed: {e}"),
        waited_ms: engine_started.elapsed().as_millis() as u64,
        attempts: Vec::new(),
    })?;
    let serp = parse_bing_serp(&html);
    let mut hits: Vec<SegmentedHit> = serp
        .into_iter()
        .enumerate()
        .map(|(index, hit)| SegmentedHit {
            index,
            title: hit.title,
            url: hit.url,
            snippet: hit.snippet,
            segments: Vec::new(),
        })
        .collect();
    if hits.is_empty() {
        return Ok(hits);
    }
    // 逐页抓取 → 逐段抽取。页级失败不致命（SERP 命中仍交付）；
    // 每页截止 = per-engine 预算，且不越过整体兜底。
    for hit in hits.iter_mut().take(config.segment_pages) {
        let remaining = config.overall_deadline.saturating_sub(overall_started.elapsed());
        if remaining.is_zero() {
            break;
        }
        let budget = config.per_engine_deadline.min(remaining);
        let page = tokio::time::timeout(budget, fetch_page_text(http, &hit.url)).await;
        if let Ok(Ok(html)) = page {
            hit.segments = extract_segments(&html);
        }
    }
    Ok(hits)
}

async fn fetch_page_text(http: &reqwest::Client, url: &str) -> Result<String, reqwest::Error> {
    let response = http.get(url).send().await?;
    response.text().await
}

/// Render the outcome as the model-visible content (per-segment, 去重纪律：
/// 每段自带 URL，模型可直接引用；不合成、不补写).
pub fn render_content(outcome: &SegmentedOutcome) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "[local_segmented] engine={} hits={} waited={}ms\n",
        outcome.engine,
        outcome.hits.len(),
        outcome.waited_ms
    ));
    for hit in &outcome.hits {
        out.push_str(&format!(
            "\n{}. {} — {}\n",
            hit.index + 1,
            hit.title,
            hit.url
        ));
        if !hit.snippet.is_empty() {
            out.push_str(&format!("   {}\n", hit.snippet));
        }
        for (index, segment) in hit.segments.iter().enumerate() {
            out.push_str(&format!("   [§{}] {}\n", index + 1, segment));
        }
    }
    out
}

/// The citation pool (URLs, in hit order, deduped).
pub fn citations(outcome: &SegmentedOutcome) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for hit in &outcome.hits {
        if !seen.contains(&hit.url) {
            seen.push(hit.url.clone());
        }
    }
    seen
}

/// Build the plain HTTP client used by the local path (no backend auth
/// headers; connect budget 5s; total = overall deadline + slack).
pub fn build_http_client(overall: Duration) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(overall + Duration::from_secs(5))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 现行 Bing 结构样本（S1′ 实测形态：h2/a 属性序与旧假设不同，
    /// 广告块 `class="b_ad"` 与有机块并存；标题带实体）。
    const SERP_FIXTURE: &str = r#"<!DOCTYPE html><html><body>
<ol id="b_results" class="">
  <li class="b_algo" data-bm="6"><h2 class=""><a class="tilk" href="https://www.rust-lang.org/">Rust &amp; Cargo &mdash; 官方站点</a></h2>
    <div class="b_caption"><p class="b_lineclamp4">Rust is a systems language &hellip; fast &amp; memory-safe.</p></div></li>
  <li class="b_ad b_adLast"><h2><a href="https://ads.example.com/promo">被排除的广告块</a></h2><p>ad</p></li>
  <li class="b_algo"><h2><a href="https://www.bing.com/ck/a?!&amp;&amp;p=1&amp;u=a1aHR0cHM6Ly9kb2NzLnJ1c3QtbGFuZy5vcmcvYm9vay8&amp;ntb=1">The Rust Book</a></h2>
    <div class="b_caption"><div><p>Learn Rust with the official book.</p></div></div></li>
</ol></body></html>"#;

    #[test]
    fn parse_bing_serp_reads_current_structure_and_skips_ads() {
        let hits = parse_bing_serp(SERP_FIXTURE);
        assert_eq!(hits.len(), 2, "ad block must be excluded: {hits:?}");
        assert_eq!(hits[0].title, "Rust & Cargo — 官方站点");
        assert_eq!(hits[0].url, "https://www.rust-lang.org/");
        assert!(hits[0].snippet.contains("memory-safe"));
        assert_eq!(hits[1].title, "The Rust Book");
        // Bing ck/a redirect → real target.
        assert_eq!(hits[1].url, "https://docs.rust-lang.org/book/");
    }

    #[test]
    fn entity_and_redirect_helpers() {
        assert_eq!(decode_html_entities("a&amp;b&#39;s"), "a&b's");
        assert_eq!(decode_html_entities("keep &unknown; text"), "keep &unknown; text");
        assert_eq!(normalize_hit_url("https://example.com/x"), "https://example.com/x");
        assert_eq!(urlencode_query("a b&c"), "a+b%26c");
    }

    #[test]
    fn segment_extraction_bounds_and_filters() {
        let html = format!(
            "<html><body><p>{}</p><p>short</p><p>{}</p></body></html>",
            "x".repeat(80),
            "y".repeat(MAX_SEGMENT_CHARS + 50)
        );
        let segments = extract_segments(&html);
        assert_eq!(segments.len(), 2);
        assert!(segments[1].chars().count() <= MAX_SEGMENT_CHARS + 1);
        assert!(segments[1].ends_with('…'));
    }

    #[test]
    fn config_defaults_and_env_seam() {
        let default = LocalSegmentedConfig::from_env_with(|_| None);
        assert!(!default.is_enabled(), "switch defaults to off (design §10.3)");
        assert_eq!(default.per_engine_deadline.as_millis(), 10_000);
        assert_eq!(default.overall_deadline.as_millis(), 30_000);
        assert_eq!(default.engines.len(), 1);
        assert_eq!(default.engines[0].id, "bing_cn");
        let enabled = LocalSegmentedConfig::from_env_with(|key| match key {
            ENV_SWITCH => Some("on".to_string()),
            ENV_PER_ENGINE_DEADLINE_MS => Some("7000".to_string()),
            ENV_OVERALL_DEADLINE_MS => Some("21000".to_string()),
            ENV_ENGINES => Some("bing_global,bogus,duckduckgo".to_string()),
            _ => None,
        });
        assert!(enabled.is_enabled());
        assert_eq!(enabled.per_engine_deadline.as_millis(), 7_000);
        assert_eq!(enabled.overall_deadline.as_millis(), 21_000);
        let ids: Vec<&str> = enabled.engines.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["bing_global", "duckduckgo"]);
        assert_eq!(enabled.engine_chain_detail(), "bing_global,duckduckgo");
    }

    fn config_for(url: String, per_engine_ms: u64, overall_ms: u64) -> LocalSegmentedConfig {
        LocalSegmentedConfig {
            enabled: true,
            engines: vec![EngineSpec {
                id: "mock_bing".to_string(),
                search_url: url,
            }],
            per_engine_deadline: Duration::from_millis(per_engine_ms),
            overall_deadline: Duration::from_millis(overall_ms),
            segment_pages: 0,
        }
    }

    #[tokio::test]
    async fn search_returns_hits_with_stable_shape() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(SERP_FIXTURE))
            .mount(&server)
            .await;
        let http = build_http_client(Duration::from_secs(30)).expect("client");
        let config = config_for(server.uri(), 5_000, 30_000);
        let outcome = search(&http, &config, "rust book").await.expect("hits");
        assert_eq!(outcome.engine, "mock_bing");
        assert_eq!(outcome.hits.len(), 2);
        assert_eq!(outcome.attempts.len(), 1);
        assert_eq!(outcome.attempts[0].outcome, "ok");
        let content = render_content(&outcome);
        assert!(content.contains("[local_segmented] engine=mock_bing hits=2"));
        assert_eq!(
            citations(&outcome),
            vec![
                "https://www.rust-lang.org/".to_string(),
                "https://docs.rust-lang.org/book/".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn empty_serp_is_empty_result_and_chain_falls_through() {
        let empty = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("<html></html>"))
            .mount(&empty)
            .await;
        let good = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(SERP_FIXTURE))
            .mount(&good)
            .await;
        let http = build_http_client(Duration::from_secs(30)).expect("client");
        let mut config = config_for(empty.uri(), 5_000, 30_000);
        config.engines.push(EngineSpec {
            id: "second".to_string(),
            search_url: good.uri(),
        });
        let outcome = search(&http, &config, "rust").await.expect("second engine wins");
        assert_eq!(outcome.engine, "second");
        assert_eq!(outcome.attempts.len(), 2);
        assert_eq!(outcome.attempts[0].outcome, CAUSE_EMPTY_RESULT);
        assert_eq!(outcome.attempts[1].outcome, "ok");
    }

    #[tokio::test]
    async fn engine_failure_carries_self_describing_cause() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let http = build_http_client(Duration::from_secs(30)).expect("client");
        let config = config_for(server.uri(), 5_000, 30_000);
        let error = search(&http, &config, "rust").await.expect_err("503");
        assert_eq!(error.code(), CAUSE_NETWORK_ERROR);
        assert!(error.detail.contains("HTTP 503"));
        let tool_error = error.to_tool_error();
        let details = tool_error.details.clone().unwrap_or_default();
        assert_eq!(details["cause"], CAUSE_NETWORK_ERROR);
        assert_eq!(details["retrieval_path"], "local_segmented");
    }

    #[tokio::test]
    async fn per_engine_deadline_is_independent_of_the_whole_budget() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(400))
                    .set_body_string(SERP_FIXTURE),
            )
            .mount(&server)
            .await;
        let http = build_http_client(Duration::from_secs(30)).expect("client");
        // 每引擎 80ms 截止、整体 30s：引擎到期即返回，不等整体钟。
        let config = config_for(server.uri(), 80, 30_000);
        let started = Instant::now();
        let error = search(&http, &config, "rust").await.expect_err("deadline");
        assert_eq!(error.code(), CAUSE_NETWORK_NO_RESPONSE);
        assert!(
            started.elapsed() < Duration::from_millis(3_000),
            "per-engine clock must fire early, elapsed={:?}",
            started.elapsed()
        );
        assert_eq!(error.attempts.len(), 1);
        assert_eq!(error.attempts[0].outcome, CAUSE_NETWORK_NO_RESPONSE);
    }

    #[tokio::test]
    async fn overall_deadline_bounds_the_chain() {
        let slow = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(500))
                    .set_body_string(SERP_FIXTURE),
            )
            .mount(&slow)
            .await;
        let http = build_http_client(Duration::from_secs(30)).expect("client");
        // 整体兜底 60ms：即便引擎钟更长，整体钟先到 → 结构化错误。
        let config = config_for(slow.uri(), 5_000, 60);
        let started = Instant::now();
        let error = search(&http, &config, "rust").await.expect_err("overall");
        assert_eq!(error.code(), CAUSE_NETWORK_NO_RESPONSE);
        assert!(
            started.elapsed() < Duration::from_millis(3_000),
            "overall bound must clip the engine budget, elapsed={:?}",
            started.elapsed()
        );
    }
}
