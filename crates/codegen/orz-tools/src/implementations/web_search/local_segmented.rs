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
//!
//! 补强批（2026-09-15，[`RETRIEVAL_LOCAL_SEGMENTED_HARDENING_DESIGN`] §10
//! 裁决，落码顺序 G2 → G1 → G3 → G4）：
//! - **G2** 降级页相关性闸门（默认开，`ORZ_RETRIEVAL_RELEVANCE_GATE`）：
//!   HTTP 200 + 整页无关 `b_algo` 不再判成功——前 3 条 ∩ 查询词集
//!   （ASCII ≥3 字符词 + CJK bigram，25% 阈值 + 词集封顶 12），判负复用
//!   `empty_result` 并继续引擎链，不新增稳定码。
//! - **G1** 计时三段：`T_acquire`（客户端 connect_timeout，5s）⊂
//!   `T_first`（每引擎钟，10s，`ORZ_RETRIEVAL_DEADLINE_MS` 语义收窄为
//!   此）⊂ `T_overall`（30s 兜底）；`T_segment` 每页独立 10s
//!   （`ORZ_RETRIEVAL_SEGMENT_MS`），页级失败不再吃掉已解析命中的交付。
//! - **G3** 引擎面收尾：google/baidu 跳转包装并发解包（6 worker / 单条
//!   6s，`ORZ_RETRIEVAL_UNWRAP_WORKERS`/`_MS`）+ 页抓取最终 URL 回填
//!   （取值序：回填 > 解包 > 包装原样）。
//! - **G4** 代理管道：只加在分段检索专用客户端 `local_http`；读取序
//!   `ORZ_RETRIEVAL_PROXY` → `HTTPS_PROXY` → `HTTP_PROXY`（`none` 显式
//!   关，传输层 `.no_proxy()` 钉死直连）；**不设引擎白名单**（显式
//!   `ORZ_RETRIEVAL_ENGINES` 永远优先），有代理默认链
//!   `bing_cn,bing_global,duckduckgo,google`。
//!
//! 检索侧补强（2026-09-15，QUAD 批深审 RET-B1/B2/B3/B4 裁决）：
//! 有代理默认链改序不改集（DDG/Google 解析器缺位下的止损排序，见
//! [`PROXY_DEFAULT_CHAIN`]）；G3 解包段按剩余整体预算钳制（设计 §5.3）；
//! 探针读数经装配期快照 [`assembled_local_segmented`] 单一源化。

use std::sync::{LazyLock, OnceLock};
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
/// G1：建连/预热预算 `T_acquire`（ms，默认 5_000）——专用客户端
/// `connect_timeout`；失败归 `capability_unreachable`，不吃检索预算。
pub const ENV_ACQUIRE_MS: &str = "ORZ_RETRIEVAL_ACQUIRE_MS";
/// G1：单页抓取预算 `T_segment`（ms/页，默认 10_000）。
pub const ENV_SEGMENT_MS: &str = "ORZ_RETRIEVAL_SEGMENT_MS";
/// G2：降级页相关性闸门（`0`/`false`/`off`/`no` 关；缺省/其余 = 开）。
pub const ENV_RELEVANCE_GATE: &str = "ORZ_RETRIEVAL_RELEVANCE_GATE";
/// G3：跳转包装并发解包 worker 数（默认 6）。
pub const ENV_UNWRAP_WORKERS: &str = "ORZ_RETRIEVAL_UNWRAP_WORKERS";
/// G3：跳转包装单条解包超时（ms，默认 6_000）。
pub const ENV_UNWRAP_MS: &str = "ORZ_RETRIEVAL_UNWRAP_MS";
/// G4：分段检索专用代理（读取序第一位；`none` 显式关闭）。
pub const ENV_PROXY: &str = "ORZ_RETRIEVAL_PROXY";
/// 0bs ⑩：指纹 sidecar 单次调用上限（含 python 启动；超时即回落 reqwest）。
pub const FINGERPRINT_TIMEOUT: Duration = Duration::from_secs(25);

pub const DEFAULT_PER_ENGINE_DEADLINE_MS: u64 = 10_000;
pub const DEFAULT_OVERALL_DEADLINE_MS: u64 = 30_000;
pub const DEFAULT_SEGMENT_PAGES: usize = 3;
pub const DEFAULT_ACQUIRE_MS: u64 = 5_000;
pub const DEFAULT_SEGMENT_MS: u64 = 10_000;
pub const DEFAULT_UNWRAP_WORKERS: usize = 6;
pub const DEFAULT_UNWRAP_MS: u64 = 6_000;
/// G2：查询词集封顶（长查询判据退化为「命中 ≥3」，防过严误杀）。
pub const RELEVANCE_TERM_CAP: usize = 12;
/// G2：只取前 3 条的 title+snippet 判相关（区分「整页无关」与「某条不相关」）。
pub const RELEVANCE_TOP_HITS: usize = 3;
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
/// 0bs ⑨（2026-09-26，用户令 + 裁决）：引擎集改为
/// **`360search` / `baidu` / `duckduckgo`** 三引擎（Bing 家族出集，仍保留
/// 注册项供显式 `ORZ_RETRIEVAL_ENGINES` 链使用）。解析器按 SearXNG 对应
/// 引擎的选择器**移植参考**（来源标注见各 `parse_*_serp`）。
pub const ENGINE_REGISTRY: &[(&str, &str)] = &[
    ("360search", "https://www.so.com/s?q={query}&pn=1"),
    ("baidu", "https://www.baidu.com/s?wd={query}&rn=10"),
    ("duckduckgo", "https://html.duckduckgo.com/html/?q={query}"),
    // Bing 家族（显式链可用；默认链已出集——2026-09-26 裁决）。
    ("bing_cn", "https://cn.bing.com/search?q={query}&count=10"),
    (
        "bing_global",
        "https://www.bing.com/search?q={query}&count=10",
    ),
    ("google", "https://www.google.com/search?q={query}"),
];

/// 0bs ⑫（2026-09-26 裁决）：**垂直源引擎族**——结构化检索 API（JSON /
/// Atom），与通用 SERP 引擎同链同预算。**不塞默认链**（垂直源是特化检索，
/// 通用查询上会稀释命中）：显式 `ORZ_RETRIEVAL_ENGINES=github,arxiv,…`
/// 即启用；id 与通用引擎同域（无前缀，链读数里直接可见）。
pub const VERTICAL_REGISTRY: &[(&str, &str)] = &[
    (
        "github",
        "https://api.github.com/search/repositories?q={query}&per_page=5",
    ),
    (
        "stackexchange",
        "https://api.stackexchange.com/2.3/search/advanced?order=desc&sort=relevance&q={query}&site=stackoverflow&pagesize=5",
    ),
    (
        "arxiv",
        "https://export.arxiv.org/api/query?search_query=all:{query}&max_results=5",
    ),
    (
        "openalex",
        "https://api.openalex.org/works?search={query}&per-page=5",
    ),
    (
        "crossref",
        "https://api.crossref.org/works?query={query}&rows=5",
    ),
    (
        "npm",
        "https://registry.npmjs.org/-/v1/search?text={query}&size=5",
    ),
];

/// 引擎 id → 搜索 URL 模板（通用 SERP 注册表 + 垂直源注册表）。
fn engine_template(id: &str) -> Option<&'static str> {
    ENGINE_REGISTRY
        .iter()
        .chain(VERTICAL_REGISTRY.iter())
        .find(|(known, _)| *known == id)
        .map(|(_, template)| *template)
}

/// One engine in the chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineSpec {
    pub id: String,
    pub search_url: String,
}

/// The local segmented retrieval configuration (switch + engine chain +
/// deadlines + 补强批 G1/G2/G3/G4 knobs). Built from the environment once
/// at tool-registry construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSegmentedConfig {
    enabled: bool,
    pub engines: Vec<EngineSpec>,
    /// G1：`T_first`——请求发出 → 首个可解析结果（每引擎独立钟）。
    pub per_engine_deadline: Duration,
    /// G1：`T_overall`——全程硬上限，三段预算的共同从属。
    pub overall_deadline: Duration,
    pub segment_pages: usize,
    /// G1：`T_acquire`——专用客户端 connect_timeout（建连失败归
    /// `capability_unreachable`，不吃检索预算）。
    pub acquire_timeout: Duration,
    /// G1：`T_segment`——单页抓取预算（页级失败不致命、不占 T_first）。
    pub segment_deadline: Duration,
    /// G2：降级页相关性闸门（默认开；关闭时行为与现状逐字节一致）。
    pub relevance_gate: bool,
    /// G3：跳转包装并发解包 worker 数。
    pub unwrap_workers: usize,
    /// G3：跳转包装单条解包超时。
    pub unwrap_timeout: Duration,
    /// G4：代理（已按读取序解析；`None` = 直连）。只加在专用客户端。
    pub proxy: Option<String>,
    /// 0bs ⑩（2026-09-26）：指纹 sidecar 优先（curl_cffi 子进程）。
    /// **config 门控**——`Default` 为 `false`（单测/离线面保持确定性），
    /// `from_env_with` 按 env 缺省（`auto` = 开）置位。
    pub fingerprint: bool,
}

/// G4→0bs ⑨（2026-09-26 裁决）：有代理时的默认引擎链。**Bing 家族出集**，
/// 改以三引擎（360/百度/DDG）：360 直连即用、百度需指纹伪装（无伪装时如实
/// 失败并顺链降级）、DDG 需代理（有代理才有意义）。显式
/// `ORZ_RETRIEVAL_ENGINES` 永远优先、不构成白名单。
pub const PROXY_DEFAULT_CHAIN: &[&str] = &["360search", "baidu", "duckduckgo"];

/// 0bs ⑨（2026-09-26 裁决）：**直连默认链**——360 直连即用；百度在有指纹
/// 伪装时可用（无伪装时如实失败并顺链降级）。DDG 直连不可达，不入直连链。
pub const DEFAULT_DIRECT_CHAIN: &[&str] = &["360search", "baidu"];

impl Default for LocalSegmentedConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            // 0bs ⑨（2026-09-26 裁决）：直连默认链 = 360search, baidu。
            engines: DEFAULT_DIRECT_CHAIN
                .iter()
                .map(|id| EngineSpec {
                    id: (*id).to_string(),
                    search_url: ENGINE_REGISTRY
                        .iter()
                        .find(|(known, _)| known == id)
                        .map(|(_, template)| (*template).to_string())
                        .unwrap_or_default(),
                })
                .collect(),
            per_engine_deadline: Duration::from_millis(DEFAULT_PER_ENGINE_DEADLINE_MS),
            overall_deadline: Duration::from_millis(DEFAULT_OVERALL_DEADLINE_MS),
            segment_pages: DEFAULT_SEGMENT_PAGES,
            acquire_timeout: Duration::from_millis(DEFAULT_ACQUIRE_MS),
            segment_deadline: Duration::from_millis(DEFAULT_SEGMENT_MS),
            relevance_gate: true,
            unwrap_workers: DEFAULT_UNWRAP_WORKERS,
            unwrap_timeout: Duration::from_millis(DEFAULT_UNWRAP_MS),
            proxy: None,
            // 0bs ⑩：`Default` 保持确定性（单测/离线面无 sidecar）；
            // 进程装配面（from_env_with）按 env 缺省置位。
            fingerprint: false,
        }
    }
}

impl LocalSegmentedConfig {
    /// Read the switch and tuning knobs from the process environment.
    ///
    /// RET-B3：装配期（registry 构造）单次快照——解析完成的配置存入进程级
    /// [`ASSEMBLED_LOCAL_SEGMENTED`]（首次写入获胜），探针只读快照、不重
    /// 读 env，防装配后 env 变化导致读数漂移。
    pub fn from_env() -> Self {
        let config = Self::from_env_with(|key| std::env::var(key).ok());
        let _ = ASSEMBLED_LOCAL_SEGMENTED.set(config.clone());
        config
    }

    /// Environment-independent seam (tests never mutate the process env —
    /// the crate's test binary runs in parallel).
    pub fn from_env_with(get: impl Fn(&str) -> Option<String>) -> Self {
        // F-014③（0ac S3 审记 2026-09-14）：clippy 的
        // `field_reassign_with_default` 警告——enabled 并入初值，
        // 其余旋钮保持就地覆盖。
        let mut config = Self {
            enabled: get(ENV_SWITCH)
                .map(|v| {
                    let v = v.trim().to_ascii_lowercase();
                    matches!(v.as_str(), "1" | "true" | "on" | "yes")
                })
                .unwrap_or(false),
            ..Self::default()
        };
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
        if let Some(ms) = get(ENV_ACQUIRE_MS)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|ms| *ms > 0)
        {
            config.acquire_timeout = Duration::from_millis(ms);
        }
        if let Some(ms) = get(ENV_SEGMENT_MS)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|ms| *ms > 0)
        {
            config.segment_deadline = Duration::from_millis(ms);
        }
        // G2：默认开；显式 falsey 值才关（关闭 = 行为与现状逐字节一致）。
        config.relevance_gate = !get(ENV_RELEVANCE_GATE)
            .map(|v| {
                matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "0" | "false" | "off" | "no"
                )
            })
            .unwrap_or(false);
        if let Some(n) = get(ENV_UNWRAP_WORKERS)
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|n| *n > 0)
        {
            config.unwrap_workers = n;
        }
        if let Some(ms) = get(ENV_UNWRAP_MS)
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|ms| *ms > 0)
        {
            config.unwrap_timeout = Duration::from_millis(ms);
        }
        // G4：代理解析（读取序见 resolve_proxy_from）。显式
        // ORZ_RETRIEVAL_ENGINES 永远优先（不设引擎白名单）；未显式配置时
        // 默认链随代理形态切换：直连 = bing_cn 单引擎（§9.4），有代理 =
        // bing_cn,bing_global,duckduckgo,google（§6.1；RET-B1 止损排序，
        // 见 PROXY_DEFAULT_CHAIN）。
        config.proxy = resolve_proxy_from(&get);
        // 0bs ⑩：指纹 sidecar 模式（env 缺省 `auto` = 开；`off` 关闭）。
        config.fingerprint = super::fingerprint::enabled();
        let explicit_engines = get(ENV_ENGINES).and_then(|list| {
            let engines: Vec<EngineSpec> = list
                .split(',')
                .filter_map(|id| {
                    let id = id.trim().to_ascii_lowercase();
                    engine_template(&id).map(|template| EngineSpec {
                        id: id.clone(),
                        search_url: template.to_string(),
                    })
                })
                .collect();
            if engines.is_empty() {
                None
            } else {
                Some(engines)
            }
        });
        config.engines = explicit_engines.unwrap_or_else(|| {
            if config.proxy.is_some() {
                PROXY_DEFAULT_CHAIN
                    .iter()
                    .filter_map(|id| {
                        ENGINE_REGISTRY.iter().find(|(known, _)| known == id).map(
                            |(known, template)| EngineSpec {
                                id: (*known).to_string(),
                                search_url: (*template).to_string(),
                            },
                        )
                    })
                    .collect()
            } else {
                config.engines.clone()
            }
        });
        config
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The engine-chain reading for the `retrieval_family` probe
    /// (`retrieval_family.search_engine.detail`)。G4：读数带
    /// `proxy=on|off` 与端点（脱敏至 host:port），探针可区分直连/代理
    /// 两种形态（§6.1 形态差异登记）。
    pub fn engine_chain_detail(&self) -> String {
        let chain = self
            .engines
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>()
            .join(",");
        match &self.proxy {
            Some(proxy) => format!("{chain}; proxy=on {}", proxy_display(proxy)),
            None => format!("{chain}; proxy=off"),
        }
    }
}

/// RET-B3：装配期单次快照（`LocalSegmentedConfig::from_env()` 首次调用
/// 写入，首次写入获胜）——探针只读不重读 env，防漂移。
static ASSEMBLED_LOCAL_SEGMENTED: OnceLock<LocalSegmentedConfig> = OnceLock::new();

/// RET-B3 读数面（单一源）：registry 装配期（`registry/types.rs` 构造时
/// 调 [`LocalSegmentedConfig::from_env`]）存下的配置快照；未装配（纯单测
/// /未建 registry）返回 `None`，调用方回退自己的 env 读取。
pub fn assembled_local_segmented() -> Option<&'static LocalSegmentedConfig> {
    ASSEMBLED_LOCAL_SEGMENTED.get()
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
    /// G2：判负时的相关命中数（全链判负取分数最高者定 detail）。
    pub gate_score: Option<u64>,
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
    Regex::new(r#"(?is)<h2[^>]*>\s*<a\b([^>]*)>(.*?)</a>"#).expect("valid SERP title regex")
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
static BING_REDIRECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)[?&]u=a1([A-Za-z0-9_\-]+)"#).expect("valid redirect regex"));

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

// ── 0bs ⑨（2026-09-26 裁决）：引擎解析器移植面 ─────────────────────────
//
// 移植参考：SearXNG 对应引擎的 `parse` 选择器（`searx/engines/360search.py`
// / `baidu.py` / `duckduckgo.py`），在原始 HTML 上用正则复刻同一组语义
// （块级容器 → 标题/链接 → 摘要），来源与行文随注释保留。

static SO_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<li[^>]*class="[^"]*res-list[^"]*"[^>]*>(.*?)</li>"#)
        .expect("valid so block regex")
});
static SO_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<h3[^>]*class="[^"]*res-title[^"]*"[^>]*>.*?<a[^>]*>(.*?)</a>"#)
        .expect("valid so title regex")
});
static SO_HREF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(?:data-mdurl|href)="([^"]+)""#).expect("valid so href regex")
});
/// 0bs ⑨：360 把真实目标塞在 `data-mdurl`（`href` 是 so.com 包装）——
/// 优先取它，缺失才回退 `href`。
static SO_MDURL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)data-mdurl="([^"]+)""#).expect("valid so mdurl regex"));
static SO_SNIPPET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<p[^>]*class="[^"]*res-desc[^"]*"[^>]*>(.*?)</p>"#)
        .expect("valid so snippet regex")
});

/// Parse a 360 搜索（so.com）SERP — SearXNG `360search.py` 语义移植：
/// `li.res-list` 容器、`h3.res-title > a` 标题（链接优先取 `data-mdurl`，
/// 缺失回退 `href`）、`p.res-desc` 摘要。
pub fn parse_360_serp(html: &str) -> Vec<SerpHit> {
    let mut hits = Vec::new();
    for block in SO_BLOCK_RE.captures_iter(html) {
        let block = block.get(1).map(|m| m.as_str()).unwrap_or_default();
        // 标题锚点窗口：从 <a …> 起取其后的一段（标题 + 链接属性同域）。
        let Some(anchor_start) = block.find("<a") else {
            continue;
        };
        let anchor_window = &block[anchor_start..];
        let Some(title_caps) = SO_TITLE_RE.captures(block) else {
            continue;
        };
        let raw_title = title_caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let Some(href) = SO_MDURL_RE
            .captures(anchor_window)
            .or_else(|| SO_HREF_RE.captures(anchor_window))
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
        let snippet = SO_SNIPPET_RE
            .captures(block)
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

static BAIDU_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<div[^>]*class="[^"]*(?:result|result-op)[^"]*"[^>]*>(.*?)</div>"#)
        .expect("valid baidu block regex")
});
/// SearXNG `baidu.py` 的 `data-tools` JSON 通道（标题/URL 直接是结构化
/// 字段，绕开百度对锚点属性的混淆）。
static BAIDU_DATA_TOOLS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)data-tools='([^']*)"#).expect("valid baidu data-tools regex")
});
static BAIDU_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<h3[^>]*>.*?<a[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)
        .expect("valid baidu title regex")
});
static BAIDU_SNIPPET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<span[^>]*class="[^"]*content-right[^"]*"[^>]*>(.*?)</span>"#)
        .expect("valid baidu snippet regex")
});

/// Parse a Baidu SERP — SearXNG `baidu.py` 语义移植：优先 `data-tools`
/// JSON 通道（`{"title":…,"url":…}`，百度把结果元数据塞在这个属性里），
/// 缺失时回退 `div.result h3 > a` 锚点 + `span.content-right*` 摘要。
pub fn parse_baidu_serp(html: &str) -> Vec<SerpHit> {
    let mut hits = Vec::new();
    for caps in BAIDU_DATA_TOOLS_RE.captures_iter(html) {
        let raw = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let decoded = decode_html_entities(raw);
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&decoded) else {
            continue;
        };
        let title = value
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_default();
        let url = value
            .get("url")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_default();
        let snippet = value
            .get("abs")
            .or_else(|| value.get("publish_time"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if title.is_empty() || !url.starts_with("http") {
            continue;
        }
        hits.push(SerpHit {
            title: decode_html_entities(&strip_tags(&title)),
            url,
            snippet,
        });
    }
    if !hits.is_empty() {
        return hits;
    }
    for block in BAIDU_BLOCK_RE.captures_iter(html) {
        let block = block.get(1).map(|m| m.as_str()).unwrap_or_default();
        let Some(title_caps) = BAIDU_TITLE_RE.captures(block) else {
            continue;
        };
        let href = title_caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let raw_title = title_caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        let title = decode_html_entities(&strip_tags(raw_title));
        let url = normalize_hit_url(&decode_html_entities(href));
        if title.is_empty() || !url.starts_with("http") {
            continue;
        }
        let snippet = BAIDU_SNIPPET_RE
            .captures(block)
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

static DDG_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#)
        .expect("valid ddg title regex")
});
/// 摘要锚点在标题锚点**之后的窗口**内单独取——不把可选的摘要并进标题
/// 正则：`(?:snippet)?` 在 leftmost-first 语义下会在窗口为空时直接成功
/// （摘要被静默跳过），这正是 0bs ⑨ 首版 fixture 抓到的缺陷形态。
static DDG_SNIPPET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*>(.*?)</a>"#)
        .expect("valid ddg snippet regex")
});

/// Parse a DuckDuckGo HTML-endpoint SERP — SearXNG `duckduckgo.py` 语义
/// 移植：`a.result__a` 标题锚点（`href` 可能是 `/l/?uddg=<urlencoded>`
/// 跳转包装，离线解包）+ 其后窗口内的 `a.result__snippet` 摘要。
pub fn parse_duckduckgo_serp(html: &str) -> Vec<SerpHit> {
    let mut hits = Vec::new();
    for caps in DDG_TITLE_RE.captures_iter(html) {
        let Some(whole) = caps.get(0) else {
            continue;
        };
        let href = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
        let raw_title = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        // 摘要窗口：标题锚点结尾起的 4KB（无摘要的条目自然拿到空串；窗口
        // 跨进下一条目时语义仍按"最近一条摘要"——与 HTML 端点的排版一致）。
        let window_end = (whole.end() + 4096).min(html.len());
        let window = &html[whole.end()..window_end];
        let snippet = DDG_SNIPPET_RE
            .captures(window)
            .and_then(|c| c.get(1))
            .map(|m| decode_html_entities(&strip_tags(m.as_str())))
            .unwrap_or_default();
        let title = decode_html_entities(&strip_tags(raw_title));
        let url = normalize_ddg_url(&decode_html_entities(href));
        if title.is_empty() || !url.starts_with("http") {
            continue;
        }
        hits.push(SerpHit {
            title,
            url,
            snippet,
        });
    }
    hits
}

/// DDG 的 `/l/?uddg=<percent-encoded>` 跳转包装 → 真实 URL（HTML 端点
/// 的离线解包；非包装形态原样通过）。SearXNG `duckduckgo.py` 同语义。
pub fn normalize_ddg_url(url: &str) -> String {
    let Some(index) = url.find("uddg=") else {
        return url.to_string();
    };
    let rest = &url[index + "uddg=".len()..];
    let encoded = rest.split('&').next().unwrap_or(rest);
    let mut decoded = String::new();
    let bytes = encoded.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = &encoded[i + 1..i + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                decoded.push(byte as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            decoded.push(' ');
        } else {
            decoded.push(bytes[i] as char);
        }
        i += 1;
    }
    if decoded.starts_with("http") {
        decoded
    } else {
        url.to_string()
    }
}

/// 0bs ⑨：按引擎 id 分派解析器（未知 id 走 Bing 形态——google 的解析器
/// 就位前它只会得到空命中，如实计入 `empty_result` 并顺链降级）。
pub fn parse_engine_serp(engine_id: &str, body: &str) -> Vec<SerpHit> {
    if is_vertical_engine(engine_id) {
        return parse_vertical_serp(engine_id, body);
    }
    match engine_id {
        "360search" => parse_360_serp(body),
        "baidu" => parse_baidu_serp(body),
        "duckduckgo" => parse_duckduckgo_serp(body),
        _ => parse_bing_serp(body),
    }
}

/// 0bs ⑫（2026-09-26 裁决）：引擎 id 是否属垂直源族。
pub fn is_vertical_engine(engine_id: &str) -> bool {
    VERTICAL_REGISTRY
        .iter()
        .any(|(known, _)| *known == engine_id)
}

static ARXIV_ENTRY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<entry>(.*?)</entry>"#).expect("valid arxiv entry regex"));
static ARXIV_TITLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<title[^>]*>(.*?)</title>"#).expect("valid arxiv title regex")
});
static ARXIV_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?is)<id>(.*?)</id>"#).expect("valid arxiv id regex"));
static ARXIV_SUMMARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?is)<summary[^>]*>(.*?)</summary>"#).expect("valid arxiv summary regex")
});

/// 0bs ⑫：垂直源解析（结构化 API → [`SerpHit`]）——JSON 通道为各家的
/// 原生字段映射；arXiv 是 Atom XML（正则取 `<entry>` 三元组）。解析失败
/// 一律回空（引擎链按 `empty_result` 如实降级，不造伪命中）。
pub fn parse_vertical_serp(engine_id: &str, body: &str) -> Vec<SerpHit> {
    if engine_id == "arxiv" {
        let mut hits = Vec::new();
        for entry in ARXIV_ENTRY_RE.captures_iter(body) {
            let entry = entry.get(1).map(|m| m.as_str()).unwrap_or_default();
            let title = ARXIV_TITLE_RE
                .captures(entry)
                .and_then(|c| c.get(1))
                .map(|m| WS_RE.replace_all(m.as_str(), " ").trim().to_string())
                .unwrap_or_default();
            let url = ARXIV_ID_RE
                .captures(entry)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let snippet = ARXIV_SUMMARY_RE
                .captures(entry)
                .and_then(|c| c.get(1))
                .map(|m| WS_RE.replace_all(m.as_str(), " ").trim().to_string())
                .unwrap_or_default();
            if title.is_empty() || !url.starts_with("http") {
                continue;
            }
            hits.push(SerpHit {
                title,
                url,
                snippet,
            });
        }
        return hits;
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let str_at = |node: &serde_json::Value, path: &[&str]| -> String {
        let mut current = node;
        for key in path {
            match current.get(*key) {
                Some(next) => current = next,
                None => return String::new(),
            }
        }
        current.as_str().unwrap_or_default().to_string()
    };
    let mut hits = Vec::new();
    let mut push = |title: String, url: String, snippet: String| {
        if !title.is_empty() && url.starts_with("http") {
            hits.push(SerpHit {
                title,
                url,
                snippet,
            });
        }
    };
    match engine_id {
        "github" => {
            for item in value
                .get("items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            {
                push(
                    str_at(&item, &["full_name"]),
                    str_at(&item, &["html_url"]),
                    str_at(&item, &["description"]),
                );
            }
        }
        "npm" => {
            for object in value
                .get("objects")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            {
                let package = object.get("package").cloned().unwrap_or_default();
                push(
                    str_at(&package, &["name"]),
                    str_at(&package, &["links", "npm"]),
                    str_at(&package, &["description"]),
                );
            }
        }
        "stackexchange" => {
            for item in value
                .get("items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            {
                push(
                    str_at(&item, &["title"]),
                    str_at(&item, &["link"]),
                    String::new(),
                );
            }
        }
        "openalex" => {
            for item in value
                .get("results")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            {
                let url = {
                    let landing = str_at(&item, &["primary_location", "landing_page_url"]);
                    if landing.is_empty() {
                        let doi = str_at(&item, &["doi"]);
                        if doi.starts_with("http") {
                            doi
                        } else {
                            String::new()
                        }
                    } else {
                        landing
                    }
                };
                push(str_at(&item, &["display_name"]), url, String::new());
            }
        }
        "crossref" => {
            for item in value
                .get("message")
                .and_then(|m| m.get("items"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            {
                let title = item
                    .get("title")
                    .and_then(|t| t.as_array())
                    .and_then(|t| t.first())
                    .and_then(|t| t.as_str())
                    .unwrap_or_default()
                    .to_string();
                push(title, str_at(&item, &["URL"]), String::new());
            }
        }
        _ => {}
    }
    hits
}

/// Minimal HTML entity decoding (the SERP surface set).
pub fn decode_html_entities(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        rest = &rest[index..];
        // G1（0ac S3 实现审记 2026-09-14）：原实现按字节切 12 字节窗口，
        // 末端落进多字节字符（CJK 标题/摘要极常见）即 char boundary
        // panic。`;` 是 ASCII：整体查找 + 窗口谓词收窄，语义等价。
        let Some(semi) = rest.find(';').filter(|index| *index < rest.len().min(12)) else {
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
            // SERP 正文常见命名实体（fixture 钉住 mdash/hellip）。
            "mdash" => Some("—".to_string()),
            "ndash" => Some("–".to_string()),
            "hellip" => Some("…".to_string()),
            "lsquo" => Some("‘".to_string()),
            "rsquo" => Some("’".to_string()),
            "ldquo" => Some("“".to_string()),
            "rdquo" => Some("”".to_string()),
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
    let padded = format!("{}{}", encoded, "=".repeat((4 - encoded.len() % 4) % 4));
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

// ── G2：降级页相关性闸门 ────────────────────────────────────────────────

fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF)
}

/// G2（设计 §4.2）：查询词集 Q = ASCII 长度 ≥3 的词（小写化）+ CJK
/// 二元组（bigram），去重保序、封顶 [`RELEVANCE_TERM_CAP`]。
/// 纯符号/纯数字查询词集为空 ⇒ 闸门直接放行（防误杀）。
pub fn query_terms(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let mut terms: Vec<String> = Vec::new();
    let mut word = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() {
            word.push(ch);
        } else if !word.is_empty() {
            if word.chars().count() >= 3 {
                terms.push(std::mem::take(&mut word));
            } else {
                word.clear();
            }
        }
    }
    if word.chars().count() >= 3 {
        terms.push(word);
    }
    let chars: Vec<char> = lower.chars().collect();
    for pair in chars.windows(2) {
        if is_cjk(pair[0]) && is_cjk(pair[1]) {
            terms.push(pair.iter().collect());
        }
    }
    let mut seen = std::collections::HashSet::new();
    terms
        .into_iter()
        .filter(|t| seen.insert(t.clone()))
        .take(RELEVANCE_TERM_CAP)
        .collect()
}

/// G2 判定（设计 §4.2）：前 [`RELEVANCE_TOP_HITS`] 条的 title+snippet 与
/// 查询词集求交。`Ok(())` = 放行（词集空或命中 ≥ need）；`Err((score,
/// need))` = 判负。need = max(1, ceil(0.25 × min(|Q|, 12)))——词集封顶
/// 后长查询的判据 = 命中 ≥3（宁可放过、不可误杀，F-007(a) 宽口径）。
pub fn relevance_gate_verdict(hits: &[SerpHit], query: &str) -> Result<(), (usize, usize)> {
    let terms = query_terms(query);
    if terms.is_empty() {
        return Ok(());
    }
    let need = terms.len().div_ceil(4).max(1);
    let text = hits
        .iter()
        .take(RELEVANCE_TOP_HITS)
        .map(|h| format!("{} {}", h.title.to_lowercase(), h.snippet.to_lowercase()))
        .collect::<Vec<_>>()
        .join("\n");
    let score = terms.iter().filter(|t| text.contains(t.as_str())).count();
    if score >= need {
        Ok(())
    } else {
        Err((score, need))
    }
}

// ── G3：跳转包装网络解包 + G4：代理解析 ─────────────────────────────────

/// G3：google/baidu 的不透明跳转包装（bing 的 base64 形态已由
/// `normalize_hit_url` 离线解包）需要跟随一次跳转才能得到真实 URL。
pub fn needs_network_unwrap(url: &str) -> bool {
    url.contains("google.com/goto") || url.contains("baidu.com/link")
}

/// G3：跟随跳转取最终 URL（reqwest 默认跟随重定向，单条受 unwrap
/// 超时约束）；失败返 `None`——调用方保留包装 URL，不阻断交付。
async fn unwrap_redirect(http: reqwest::Client, url: String, timeout: Duration) -> Option<String> {
    let response = http.get(&url).timeout(timeout).send().await.ok()?;
    let final_url = response.url().to_string();
    (final_url.starts_with("http") && final_url != url).then_some(final_url)
}

/// G4 读取序：`ORZ_RETRIEVAL_PROXY` → `HTTPS_PROXY` → `HTTP_PROXY`；
/// `ORZ_RETRIEVAL_PROXY=none` 显式关闭（优先级高于其余来源）——容器/
/// 评测基线借此保证与无代理形态逐字节一致。
pub fn resolve_proxy_from(get: &impl Fn(&str) -> Option<String>) -> Option<String> {
    if let Some(v) = get(ENV_PROXY) {
        let v = v.trim().to_string();
        return if v.is_empty() || v.eq_ignore_ascii_case("none") {
            None
        } else {
            Some(v)
        };
    }
    for key in ["HTTPS_PROXY", "HTTP_PROXY"] {
        if let Some(v) = get(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// G4 读数面脱敏：scheme 与凭据剥除，仅留 `host:port` 形态。
pub fn proxy_display(proxy: &str) -> String {
    let rest = proxy.split("://").nth(1).unwrap_or(proxy);
    let authority = rest.split(['/', '?']).next().unwrap_or(rest);
    match authority.rsplit_once('@') {
        Some((_, host)) => host.to_string(),
        None => authority.to_string(),
    }
}

// ── 执行（G1 三段账：T_acquire ⊂ T_first ⊂ T_overall；T_segment 每页独立）──

/// Run the local segmented retrieval: engine chain → SERP → (G2 闸门) →
/// 逐页抓取 → 逐段抽取.
///
/// G1 计时三段（设计 §3）：`T_acquire` = 专用客户端 connect_timeout
/// （建连失败 → `capability_unreachable`）；`T_first` = 每引擎独立钟
/// （默认 10s；`ORZ_RETRIEVAL_DEADLINE_MS` 语义收窄为此——请求发出 →
/// 首个可解析结果）；`T_segment` = 单页预算（默认 10s/页），页级失败
/// 不致命、已解析命中照常交付；`T_overall`（默认 30s）是三段共同从属
/// 的全程硬上限。G2：闸门判负 = `empty_result`（不新增稳定码）并继续
/// 引擎链；全链判负时 detail 取分数最高的一次尝试。
pub async fn search(
    http: &reqwest::Client,
    config: &LocalSegmentedConfig,
    query: &str,
) -> Result<SegmentedOutcome, SegmentedError> {
    let started = Instant::now();
    let mut attempts: Vec<EngineAttempt> = Vec::new();
    let mut last: Option<SegmentedError> = None;
    let mut best_empty: Option<SegmentedError> = None;
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
                gate_score: None,
            });
            break;
        }
        // G1 `T_first`：只包「请求发出 → SERP 解析完成」；页抓取挪出。
        let budget = config.per_engine_deadline.min(remaining);
        let engine_started = Instant::now();
        let url = engine
            .search_url
            .replace("{query}", &urlencode_query(query));
        let serp =
            tokio::time::timeout(budget, fetch_serp(http, engine, &url, config.fingerprint)).await;
        match serp {
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
                    gate_score: None,
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
            Ok(Ok(serp_hits)) => {
                if serp_hits.is_empty() {
                    attempts.push(EngineAttempt {
                        engine: engine.id.clone(),
                        waited_ms: engine_started.elapsed().as_millis() as u64,
                        outcome: CAUSE_EMPTY_RESULT.to_string(),
                    });
                    last = Some(SegmentedError {
                        cause: CAUSE_EMPTY_RESULT,
                        engine: engine.id.clone(),
                        detail: "SERP parsed but carried no organic hit".to_string(),
                        waited_ms: engine_started.elapsed().as_millis() as u64,
                        attempts: attempts.clone(),
                        gate_score: None,
                    });
                    continue;
                }
                // G2：解析成功但整页无关（引擎兜底页）≠ 成功——判负复用
                // `empty_result` 继续引擎链；detail 带分数与 need，供
                // A/B 与法官核对（不新增稳定码）。
                if config.relevance_gate
                    && let Err((score, need)) = relevance_gate_verdict(&serp_hits, query)
                {
                    attempts.push(EngineAttempt {
                        engine: engine.id.clone(),
                        waited_ms: engine_started.elapsed().as_millis() as u64,
                        outcome: CAUSE_EMPTY_RESULT.to_string(),
                    });
                    let error = SegmentedError {
                        cause: CAUSE_EMPTY_RESULT,
                        engine: engine.id.clone(),
                        detail: format!(
                            "SERP 命中与查询无关（引擎兜底页，非解析故障）；\
                             relevance score={score}/{need}"
                        ),
                        waited_ms: engine_started.elapsed().as_millis() as u64,
                        attempts: attempts.clone(),
                        gate_score: Some(score as u64),
                    };
                    if best_empty
                        .as_ref()
                        .is_none_or(|best| best.gate_score.is_none_or(|s| score as u64 > s))
                    {
                        best_empty = Some(error.clone());
                    }
                    last = Some(error);
                    continue;
                }
                attempts.push(EngineAttempt {
                    engine: engine.id.clone(),
                    waited_ms: engine_started.elapsed().as_millis() as u64,
                    outcome: "ok".to_string(),
                });
                let mut hits: Vec<SegmentedHit> = serp_hits
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
                // G3：跳转包装并发解包——信号量封 worker 上限（6/6s）；
                // 任务先行启动、与页抓取并发推进，段末统一 await（§5）。
                // 解包失败保留包装 URL，不阻断交付。
                let semaphore =
                    std::sync::Arc::new(tokio::sync::Semaphore::new(config.unwrap_workers.max(1)));
                let handles: Vec<tokio::task::JoinHandle<(usize, Option<String>)>> = hits
                    .iter()
                    .enumerate()
                    .filter(|(_, hit)| needs_network_unwrap(&hit.url))
                    .map(|(index, hit)| {
                        let http = http.clone();
                        let semaphore = std::sync::Arc::clone(&semaphore);
                        let url = hit.url.clone();
                        let timeout = config.unwrap_timeout;
                        tokio::spawn(async move {
                            let _permit = semaphore.acquire_owned().await;
                            let final_url = unwrap_redirect(http, url, timeout).await;
                            (index, final_url)
                        })
                    })
                    .collect();
                // G1 `T_segment`：每页独立预算 = min(segment，剩余整体)；
                // 页级失败不致命；成功页回填最终 URL（G3 取值序最高位）。
                let mut backfilled = std::collections::HashSet::new();
                for hit in hits.iter_mut().take(config.segment_pages) {
                    let remaining = config.overall_deadline.saturating_sub(started.elapsed());
                    if remaining.is_zero() {
                        break;
                    }
                    let page_budget = config.segment_deadline.min(remaining);
                    let page =
                        tokio::time::timeout(page_budget, fetch_page_text(http, &hit.url)).await;
                    if let Ok(Ok((html, final_url))) = page {
                        hit.segments = extract_segments(&html);
                        hit.url = final_url;
                        backfilled.insert(hit.index);
                    }
                }
                // RET-B4（2026-09-15 裁决，设计 §5.3「整段取 min(剩余整体
                // 预算)」）：单条 6s 只约束单个请求，await-all 若不按剩余
                // 整体预算钳制，T_overall 可被 ⌈n/worker⌉×6s 突破。每轮
                // 重算 remaining，到点 abort 并停止等待；已收到的解包结果
                // 照常回填。timeout 只借 `&mut handle`（JoinHandle 的
                // Future 实现），超时后仍持有所有权才能 abort。
                for mut handle in handles {
                    let remaining = config.overall_deadline.saturating_sub(started.elapsed());
                    match tokio::time::timeout(remaining, &mut handle).await {
                        Ok(Ok((index, Some(final_url)))) => {
                            if let Some(hit) = hits.get_mut(index)
                                && !backfilled.contains(&index)
                            {
                                hit.url = final_url;
                            }
                        }
                        // 整体预算到点：abort 未完成解包任务，不再等待；
                        // 已收到的结果照常回填。
                        Err(_elapsed) => {
                            handle.abort();
                            break;
                        }
                        // 解包任务自身失败（panic）或未取到最终 URL：保留
                        // 包装 URL，不阻断交付（与既有语义一致）。
                        Ok(Ok((_, None))) | Ok(Err(_)) => {}
                    }
                }
                return Ok(SegmentedOutcome {
                    engine: engine.id.clone(),
                    hits,
                    waited_ms: started.elapsed().as_millis() as u64,
                    attempts,
                });
            }
        }
    }
    // G2：全链判负时 detail 取分数最高的一次尝试（避免只报最后一个引擎）。
    let mut final_error = last.unwrap_or(SegmentedError {
        cause: CAUSE_NO_PROGRESS,
        engine: "none".to_string(),
        detail: "engine chain is empty (no engine configured)".to_string(),
        waited_ms: started.elapsed().as_millis() as u64,
        attempts,
        gate_score: None,
    });
    if final_error.cause == CAUSE_EMPTY_RESULT
        && let Some(best) = &best_empty
        && best
            .gate_score
            .is_some_and(|s| s > final_error.gate_score.unwrap_or(0))
    {
        final_error.engine = best.engine.clone();
        final_error.detail = best.detail.clone();
        final_error.gate_score = best.gate_score;
    }
    Err(final_error)
}

/// G1：SERP 请求 → 首个可解析结果（`T_first` 钟内）。建连超时由专用
/// 客户端的 `connect_timeout`（`T_acquire`）先到先归因：`is_connect`
/// ⇒ `capability_unreachable`，建连完成到不了首个结果 ⇒ 引擎钟到点
/// `network_no_response`。
async fn fetch_serp(
    http: &reqwest::Client,
    engine: &EngineSpec,
    url: &str,
    fingerprint: bool,
) -> Result<Vec<SerpHit>, SegmentedError> {
    let engine_started = Instant::now();
    // 0bs ⑩（2026-09-26 裁决）：**指纹伪装优先**——curl_cffi sidecar（python
    // 子进程，固定内嵌脚本）先试；不可用（python/curl_cffi 缺失）或调用失败
    // 时**如实回落** reqwest（`wreq` 记为目标形态，后续批），不把回落伪装成
    // 指纹成功。引擎解析按 id 分派（⑨ 三引擎 + Bing 家族保留）。
    // 门控＝config.fingerprint（`Default` 为 false，单测保持确定性）。
    if fingerprint
        && super::fingerprint::enabled()
        && let Some((body, _final_url)) = super::fingerprint::fetch(url, FINGERPRINT_TIMEOUT).await
    {
        return Ok(parse_engine_serp(&engine.id, &body));
    }
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
            gate_score: None,
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
            gate_score: None,
        });
    }
    let html = response.text().await.map_err(|e| SegmentedError {
        cause: CAUSE_NETWORK_ERROR,
        engine: engine.id.clone(),
        detail: format!("reading SERP body failed: {e}"),
        waited_ms: engine_started.elapsed().as_millis() as u64,
        attempts: Vec::new(),
        gate_score: None,
    })?;
    Ok(parse_engine_serp(&engine.id, &html))
}

/// 页抓取返回 `(正文, 最终 URL)`——G3：reqwest 跟随重定向后
/// `response.url()` 即最终落点，零额外请求即可回填交付 URL。
async fn fetch_page_text(
    http: &reqwest::Client,
    url: &str,
) -> Result<(String, String), reqwest::Error> {
    let response = http.get(url).send().await?;
    let final_url = response.url().to_string();
    let text = response.text().await?;
    Ok((text, final_url))
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
/// headers)。G1：`connect_timeout` = `T_acquire`（专用客户端装配期建连
/// 预算）；G4：代理只加在此专用客户端上（服务端 `/responses` 客户端
/// 带凭证边界，不受影响；容器不设代理 env ⇒ 行为与现状一致）。
pub fn build_http_client(config: &LocalSegmentedConfig) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(config.acquire_timeout)
        .timeout(config.overall_deadline + Duration::from_secs(5));
    if let Some(proxy) = &config.proxy {
        builder = builder.proxy(reqwest::Proxy::all(proxy)?);
    } else {
        // RET-B2（2026-09-15 裁决）：resolve=None 有两种来源——三个 env
        // 全空，或显式 `ORZ_RETRIEVAL_PROXY=none`。reqwest 0.12 缺省
        // auto_sys_proxy 还会读 resolve 层不读的 ALL_PROXY（及小写变体），
        // env 有代理时显式 `none` 与不设同态走代理，污染 A/B「代理 vs
        // 直连」对照读数——此处 `.no_proxy()` 钉死直连：env 全无时行为
        // 不变（本就无代理可读），显式 `none` 时真正强制直连。reqwest
        // Client 无法内省代理配置，传输面行为不加单测（resolve 层语义
        // 由 `proxy_resolution_order_none_and_fallbacks` 钉住）。
        builder = builder.no_proxy();
    }
    builder.build()
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
    /// 0bs ⑫：垂直源解析（结构化 API → SerpHit 三元组）。
    #[test]
    fn parse_vertical_serp_maps_native_json_shapes() {
        let github = r#"{"items":[{"full_name":"rust-lang/rust","html_url":"https://github.com/rust-lang/rust","description":"empowering everyone"}]}"#;
        let hits = parse_vertical_serp("github", github);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].title, "rust-lang/rust");
        assert_eq!(hits[0].url, "https://github.com/rust-lang/rust");
        assert!(hits[0].snippet.contains("empowering"));
        let npm = r#"{"objects":[{"package":{"name":"vitest","links":{"npm":"https://www.npmjs.com/package/vitest"},"description":"test runner"}}]}"#;
        let hits = parse_vertical_serp("npm", npm);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].url, "https://www.npmjs.com/package/vitest");
        let arxiv = r#"<feed><entry><title>Attention Is All You Need</title><id>http://arxiv.org/abs/1706.03762v7</id><summary> The dominant sequence transduction models. </summary></entry></feed>"#;
        let hits = parse_vertical_serp("arxiv", arxiv);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].title, "Attention Is All You Need");
        assert!(hits[0].url.starts_with("http://arxiv.org/abs/1706"));
        assert!(hits[0].snippet.contains("dominant"));
        // 坏 JSON ⇒ 空命中（如实降级，不造伪命中）。
        assert!(parse_vertical_serp("github", "not json").is_empty());
        // 垂直源 id 也可进显式链（engine_template 覆盖两张注册表）。
        assert!(is_vertical_engine("openalex"));
        assert!(!is_vertical_engine("360search"));
        assert!(engine_template("crossref").is_some());
        let config = LocalSegmentedConfig::from_env_with(|key| match key {
            ENV_ENGINES => Some("github,arxiv,bogus".to_string()),
            _ => None,
        });
        let ids: Vec<&str> = config.engines.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, vec!["github", "arxiv"]);
    }

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

    /// 0bs ⑨：三引擎解析器 fixtures（选择器语义按 SearXNG 对应移植）。
    #[test]
    fn parse_360_serp_reads_res_list_and_prefers_data_mdurl() {
        let html = r#"<!DOCTYPE html><html><body>
<li class="res-list"><h3 class="res-title"><a href="https://www.so.com/link?m=wrapped" data-mdurl="https://example.com/so-target">360 目标站 &amp; 副题</a></h3>
  <p class="res-desc">360 摘要文本。</p></li>
<li class="res-list other"><h3 class="res-title"><a href="https://example.com/second">第二条</a></h3></li>
</body></html>"#;
        let hits = parse_360_serp(html);
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!(hits[0].title, "360 目标站 & 副题");
        assert_eq!(hits[0].url, "https://example.com/so-target");
        assert!(hits[0].snippet.contains("360 摘要"));
        assert_eq!(hits[1].url, "https://example.com/second");
    }

    /// 0bs ⑨：百度走 `data-tools` JSON 通道（SearXNG baidu.py 同语义）。
    #[test]
    fn parse_baidu_serp_reads_data_tools_json_channel() {
        let html = r#"<!DOCTYPE html><html><body>
<div class="result c-container" data-tools='{"title":"百度目标站","url":"https://example.com/baidu-target","abs":"百度摘要"}'>…</div>
<div class="result c-container" data-tools='{"title":"","url":""}'>no</div>
</body></html>"#;
        let hits = parse_baidu_serp(html);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].title, "百度目标站");
        assert_eq!(hits[0].url, "https://example.com/baidu-target");
        assert_eq!(hits[0].snippet, "百度摘要");
        // 锚点回退通道（无 data-tools 时）。
        let fallback = r#"<div class="result c-container"><h3 class="t"><a href="https://www.baidu.com/link?url=abc">回退标题</a></h3><span class="content-right_1">回退摘要</span></div>"#;
        let hits = parse_baidu_serp(fallback);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].title, "回退标题");
        assert!(hits[0].snippet.contains("回退摘要"));
    }

    /// 0bs ⑨：DDG HTML 端点——`/l/?uddg=` 跳转包装离线解包。
    #[test]
    fn parse_duckduckgo_serp_unwraps_uddg_redirect() {
        let html = r##"<!DOCTYPE html><html><body>
<a class="result__a" href="/l/?uddg=https%3A%2F%2Fexample.com%2Fddg-target&amp;rut=1">DDG 目标站</a>
<a class="result__snippet" href="#">DDG 摘要文本</a>
</body></html>"##;
        let hits = parse_duckduckgo_serp(html);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].title, "DDG 目标站");
        assert_eq!(hits[0].url, "https://example.com/ddg-target");
        assert!(hits[0].snippet.contains("DDG 摘要"));
        assert_eq!(
            normalize_ddg_url("/l/?uddg=https%3A%2F%2Fa.example%2Fb&x=1"),
            "https://a.example/b"
        );
        assert_eq!(
            normalize_ddg_url("https://plain.example/"),
            "https://plain.example/"
        );
    }

    #[test]
    fn entity_and_redirect_helpers() {
        assert_eq!(decode_html_entities("a&amp;b&#39;s"), "a&b's");
        assert_eq!(
            decode_html_entities("keep &unknown; text"),
            "keep &unknown; text"
        );
        assert_eq!(
            normalize_hit_url("https://example.com/x"),
            "https://example.com/x"
        );
        assert_eq!(urlencode_query("a b&c"), "a+b%26c");
    }

    #[test]
    fn decode_html_entities_keeps_multibyte_window_boundary_intact() {
        // G1 钉子（0ac S3 实现审记 2026-09-14）：`&` 后 12 字节窗口跨到
        // 多字节字符边界时不得 panic（原先按字节切片，dev/release 下
        // `panic = "abort"` 直接进程中止）。
        assert_eq!(decode_html_entities("A &mdash; 官方站点"), "A — 官方站点");
        assert_eq!(decode_html_entities("B &hellip; 中文"), "B … 中文");
        assert_eq!(decode_html_entities("未闭合 &amp"), "未闭合 &amp");
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
        assert!(
            !default.is_enabled(),
            "switch defaults to off (design §10.3)"
        );
        assert_eq!(default.per_engine_deadline.as_millis(), 10_000);
        assert_eq!(default.overall_deadline.as_millis(), 30_000);
        // 0bs ⑨（2026-09-26 裁决）：直连默认链 ＝ 360search, baidu。
        assert_eq!(default.engines.len(), 2);
        assert_eq!(default.engines[0].id, "360search");
        assert_eq!(default.engines[1].id, "baidu");
        assert_eq!(default.acquire_timeout.as_millis(), 5_000, "G1 T_acquire");
        assert_eq!(default.segment_deadline.as_millis(), 10_000, "G1 T_segment");
        assert!(default.relevance_gate, "G2 闸门默认开");
        assert_eq!(default.unwrap_workers, 6, "G3 worker");
        assert_eq!(default.unwrap_timeout.as_millis(), 6_000, "G3 单条超时");
        assert!(default.proxy.is_none(), "G4 直连缺省");
        assert_eq!(default.engine_chain_detail(), "360search,baidu; proxy=off");
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
        assert_eq!(
            enabled.engine_chain_detail(),
            "bing_global,duckduckgo; proxy=off"
        );
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
            // 单测不联网：只回 SERP 命中，不抓页（既有口径）。
            segment_pages: 0,
            ..LocalSegmentedConfig::default()
        }
    }

    fn test_http() -> reqwest::Client {
        build_http_client(&LocalSegmentedConfig::default()).expect("client")
    }

    /// G2 降级页样本（设计 §8-1）：HTTP 200 + 结构完整的无关 `b_algo`
    /// （§2 实测形态：4399/Steam/知乎/四六级/阿里云族）。
    fn degraded_serp_fixture() -> String {
        let sites = [
            ("https://www.4399.com/", "4399 小游戏大全"),
            ("https://store.steampowered.com/", "Steam 夏季特卖"),
            ("https://www.zhihu.com/hot", "知乎热榜"),
            ("https://cet.neea.edu.cn/", "全国大学英语四六级考试"),
            ("https://www.aliyun.com/", "阿里云服务平台"),
        ];
        let blocks: String = sites
            .iter()
            .map(|(url, title)| {
                format!(
                    "<li class=\"b_algo\"><h2><a href=\"{url}\">{title}</a></h2>\
                     <div class=\"b_caption\"><p class=\"b_lineclamp4\">热门推荐与榜单内容，与任何技术查询无关。</p></div></li>"
                )
            })
            .collect();
        format!("<!DOCTYPE html><html><body><ol id=\"b_results\">{blocks}</ol></body></html>")
    }

    /// 运行期 SERP 样本：自定义命中 URL（G3 解包 / G1 慢页测试用）。
    fn serp_fixture_with_hit(url: &str) -> String {
        format!(
            "<!DOCTYPE html><html><body><ol id=\"b_results\">\
             <li class=\"b_algo\"><h2><a href=\"{url}\">包装命中</a></h2>\
             <div class=\"b_caption\"><p class=\"b_lineclamp4\">rust programming language book 摘要。</p></div></li>\
             </ol></body></html>"
        )
    }

    #[tokio::test]
    async fn search_returns_hits_with_stable_shape() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(SERP_FIXTURE))
            .mount(&server)
            .await;
        let http = test_http();
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
        let http = test_http();
        let mut config = config_for(empty.uri(), 5_000, 30_000);
        config.engines.push(EngineSpec {
            id: "second".to_string(),
            search_url: good.uri(),
        });
        let outcome = search(&http, &config, "rust")
            .await
            .expect("second engine wins");
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
        let http = test_http();
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
        let http = test_http();
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
        let http = test_http();
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

    // ── G2：降级页相关性闸门 ──────────────────────────────────────────

    #[test]
    fn gate_verdict_word_set_bigrams_and_cap() {
        // ASCII ≥3 字符词 + CJK bigram，去重保序。
        let terms = query_terms("Rust theBook 编程入门");
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"thebook".to_string()));
        assert!(terms.contains(&"编程".to_string()));
        assert!(terms.contains(&"程入".to_string()));
        assert!(
            !terms.contains(&"the".to_string()),
            "短词被剥离（<3 字符在词内断开）"
        );
        // 词集封顶 12。
        let long = query_terms("aaa bbb ccc ddd eee fff ggg hhh iii jjj kkk lll mmm nnn");
        assert_eq!(long.len(), RELEVANCE_TERM_CAP, "词集封顶 12");
        // 纯符号/纯数字查询：词集为空。
        assert!(query_terms("v1.2.3 #7 %^&").is_empty());
    }

    #[test]
    fn gate_long_query_capped_need_is_three() {
        // 16 词查询 → 词集封顶 12 → need = ceil(0.25×12) = 3。
        let query = "w01 w02 w03 w04 w05 w06 w07 w08 w09 w10 w11 w12 w13 w14 w15 w16";
        let hit = |words: &[&str]| {
            let title = words.join(" ");
            vec![SerpHit {
                title,
                url: "https://example.com/x".to_string(),
                snippet: String::new(),
            }]
        };
        // 前 3 条合计命中 3 个词 ⇒ 放行。
        assert!(relevance_gate_verdict(&hit(&["w01 x", "w02 x", "w03 x"]), query).is_ok());
        // 命中 2 个词 ⇒ 判负（score=2, need=3）。
        assert_eq!(
            relevance_gate_verdict(&hit(&["w01 x", "w02 x", "无关内容"]), query),
            Err((2, 3))
        );
    }

    #[tokio::test]
    async fn gate_rejects_degraded_page_and_chain_falls_through() {
        let degraded = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_string(degraded_serp_fixture()),
            )
            .mount(&degraded)
            .await;
        let good = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(SERP_FIXTURE))
            .mount(&good)
            .await;
        let http = test_http();
        let mut config = config_for(degraded.uri(), 5_000, 30_000);
        config.engines.push(EngineSpec {
            id: "second".to_string(),
            search_url: good.uri(),
        });
        let outcome = search(&http, &config, "rust book language")
            .await
            .expect("second engine wins after gate rejection");
        assert_eq!(outcome.engine, "second");
        assert_eq!(outcome.attempts[0].outcome, CAUSE_EMPTY_RESULT);
        assert_eq!(outcome.attempts[1].outcome, "ok");
    }

    #[tokio::test]
    async fn gate_disabled_keeps_legacy_behavior() {
        let degraded = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_string(degraded_serp_fixture()),
            )
            .mount(&degraded)
            .await;
        let http = test_http();
        let mut config = config_for(degraded.uri(), 5_000, 30_000);
        config.relevance_gate = false;
        let outcome = search(&http, &config, "rust book language")
            .await
            .expect("闸门关闭 = 现状行为（非空即成功）");
        assert_eq!(outcome.hits.len(), degraded_serp_hits());
    }

    fn degraded_serp_hits() -> usize {
        parse_bing_serp(&degraded_serp_fixture()).len()
    }

    #[tokio::test]
    async fn all_chain_gate_failures_report_the_highest_score_attempt() {
        // 引擎 1：零重叠（score 0）；引擎 2：标题带一个查询词（score 1）。
        // 查询 5 词 → need 2 ⇒ 两引擎都判负；最终 detail 取分数最高者。
        let query = "rust programming language book tutorial";
        let one = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_string(degraded_serp_fixture()),
            )
            .mount(&one)
            .await;
        let two = wiremock::MockServer::start().await;
        let semi = format!(
            "<!DOCTYPE html><html><body><ol id=\"b_results\">\
             <li class=\"b_algo\"><h2><a href=\"https://example.com/a\">rust 教程站</a></h2>\
             <div class=\"b_caption\"><p class=\"b_lineclamp4\">无关推荐位。</p></div></li>\
             </ol></body></html>"
        );
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(semi))
            .mount(&two)
            .await;
        let http = test_http();
        let mut config = config_for(one.uri(), 5_000, 30_000);
        config.engines.push(EngineSpec {
            id: "second".to_string(),
            search_url: two.uri(),
        });
        let error = search(&http, &config, query).await.expect_err("全链判负");
        assert_eq!(error.cause, CAUSE_EMPTY_RESULT);
        assert_eq!(error.engine, "second", "detail 取分数最高的尝试");
        assert!(
            error.detail.contains("score=1/2"),
            "detail={}",
            error.detail
        );
        assert_eq!(error.gate_score, Some(1));
        assert_eq!(error.attempts.len(), 2);
    }

    #[test]
    fn symbol_only_query_passes_the_gate() {
        let hits = parse_bing_serp(&degraded_serp_fixture());
        assert!(
            relevance_gate_verdict(&hits, "v1.2.3 #7 %^&").is_ok(),
            "词集空 ⇒ 放行"
        );
    }

    // ── G3：跳转包装解包 + 最终 URL 回填 ──────────────────────────────

    #[test]
    fn network_unwrap_targets_google_and_baidu_wrappers_only() {
        assert!(needs_network_unwrap(
            "https://www.google.com/goto?url=OPAQUE"
        ));
        assert!(needs_network_unwrap("https://www.baidu.com/link?url=xyz"));
        assert!(!needs_network_unwrap("https://docs.rust-lang.org/book/"));
        assert!(!needs_network_unwrap(
            "https://www.bing.com/ck/a?!&&u=a1aHR0cA"
        ));
    }

    #[tokio::test]
    async fn unwrap_backfills_the_real_url_when_no_page_fetch() {
        let serp = wiremock::MockServer::start().await;
        let redirector = wiremock::MockServer::start().await;
        let final_server = wiremock::MockServer::start().await;
        let fixture =
            serp_fixture_with_hit(&format!("{}/google.com/goto?url=OPAQUE", redirector.uri()));
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(fixture))
            .mount(&serp)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(302)
                    .insert_header("Location", format!("{}/real", final_server.uri())),
            )
            .mount(&redirector)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string("final"))
            .mount(&final_server)
            .await;
        let http = test_http();
        let mut config = config_for(serp.uri(), 5_000, 30_000);
        config.segment_pages = 0;
        let outcome = search(&http, &config, "rust programming language book")
            .await
            .expect("hits");
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(
            outcome.hits[0].url,
            format!("{}/real", final_server.uri()),
            "解包结果回填（无页抓取回填时）"
        );
    }

    #[tokio::test]
    async fn page_fetch_backfill_beats_unwrap_and_wrapper() {
        let serp = wiremock::MockServer::start().await;
        let redirector = wiremock::MockServer::start().await;
        let page = wiremock::MockServer::start().await;
        let fixture =
            serp_fixture_with_hit(&format!("{}/baidu.com/link?url=xyz", redirector.uri()));
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(fixture))
            .mount(&serp)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(302)
                    .insert_header("Location", format!("{}/real", page.uri())),
            )
            .mount(&redirector)
            .await;
        let body = format!("<html><body><p>{}</p></body></html>", "x".repeat(80));
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(body))
            .mount(&page)
            .await;
        let http = test_http();
        let mut config = config_for(serp.uri(), 5_000, 30_000);
        config.segment_pages = 1;
        let outcome = search(&http, &config, "rust programming language book")
            .await
            .expect("hits");
        assert_eq!(
            outcome.hits[0].url,
            format!("{}/real", page.uri()),
            "页抓取回填优先"
        );
        assert_eq!(outcome.hits[0].segments.len(), 1, "正文段照常抽取");
    }

    /// RET-B4（2026-09-15，设计 §5.3「整段取 min(剩余整体预算)」）：解包
    /// 段整体受剩余预算钳制——单条解包钟远长于剩余整体预算时，await 不
    /// 得把 T_overall 拖破；被 abort 的解包保留包装 URL、不阻断交付。
    #[tokio::test]
    async fn unwrap_await_is_clamped_by_remaining_overall_budget() {
        let serp = wiremock::MockServer::start().await;
        let redirector = wiremock::MockServer::start().await;
        let fixture =
            serp_fixture_with_hit(&format!("{}/google.com/goto?url=OPAQUE", redirector.uri()));
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(fixture))
            .mount(&serp)
            .await;
        // 跳转方悬挂：远超整体剩余预算，解包只能被 abort（而非等单条钟）。
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(302)
                    .set_delay(Duration::from_secs(30))
                    .insert_header("Location", "https://docs.rust-lang.org/book/"),
            )
            .mount(&redirector)
            .await;
        let http = test_http();
        let mut config = config_for(serp.uri(), 5_000, 1_000);
        config.segment_pages = 0;
        // 单条解包钟（30s）远长于剩余整体预算（≈1s）——整体钳制必须先到。
        config.unwrap_timeout = Duration::from_secs(30);
        let started = Instant::now();
        let outcome = search(&http, &config, "rust programming language book")
            .await
            .expect("SERP 命中不被解包段拖死");
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "解包 await 必须按剩余整体预算截断，elapsed={:?}",
            started.elapsed()
        );
        assert!(
            outcome.hits[0].url.contains("google.com/goto"),
            "解包被 abort：包装 URL 原样保留",
        );
    }

    // ── G1：T_segment 独立于 T_first（页级超时不吞已解析命中）─────────

    #[tokio::test]
    async fn slow_page_does_not_kill_delivered_hits() {
        let serp = wiremock::MockServer::start().await;
        let page = wiremock::MockServer::start().await;
        let fixture = serp_fixture_with_hit("https://docs.rust-lang.org/book/");
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(fixture))
            .mount(&serp)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_delay(Duration::from_millis(400))
                    .set_body_string("<html><body><p>slow</p></body></html>"),
            )
            .mount(&page)
            .await;
        let http = test_http();
        let mut config = config_for(serp.uri(), 5_000, 30_000);
        config.segment_pages = 1;
        config.engines.push(EngineSpec {
            id: "pages".to_string(),
            search_url: page.uri(),
        });
        // 页级预算 80ms：页抓取超时被吞掉，SERP 命中照常交付。
        let mut config = LocalSegmentedConfig {
            segment_deadline: Duration::from_millis(80),
            engines: config.engines,
            ..config
        };
        config.segment_pages = 1;
        let started = Instant::now();
        let outcome = search(&http, &config, "rust programming language book")
            .await
            .expect("SERP 命中不被页级超时吞掉");
        assert!(started.elapsed() < Duration::from_millis(3_000));
        assert_eq!(outcome.hits[0].url, "https://docs.rust-lang.org/book/");
        assert!(outcome.hits[0].segments.is_empty(), "慢页无段，命中仍交付");
    }

    // ── G4：代理解析与默认链序 ────────────────────────────────────────

    #[test]
    fn proxy_resolution_order_none_and_fallbacks() {
        let none_first = |key: &str| match key {
            ENV_PROXY => Some("none".to_string()),
            "HTTPS_PROXY" => Some("http://leak:1".to_string()),
            _ => None,
        };
        assert_eq!(resolve_proxy_from(&none_first), None, "none 显式关优先");
        let override_wins = |key: &str| match key {
            ENV_PROXY => Some("http://first:1".to_string()),
            "HTTPS_PROXY" => Some("http://second:1".to_string()),
            _ => None,
        };
        assert_eq!(
            resolve_proxy_from(&override_wins).as_deref(),
            Some("http://first:1")
        );
        let https_fallback = |key: &str| match key {
            "HTTPS_PROXY" => Some(" http://secure:1 ".to_string()),
            _ => None,
        };
        assert_eq!(
            resolve_proxy_from(&https_fallback).as_deref(),
            Some("http://secure:1")
        );
        let http_fallback = |key: &str| match key {
            "HTTP_PROXY" => Some("http://plain:1".to_string()),
            _ => None,
        };
        assert_eq!(
            resolve_proxy_from(&http_fallback).as_deref(),
            Some("http://plain:1")
        );
        assert_eq!(resolve_proxy_from(&|_| None), None);
    }

    #[test]
    fn proxy_display_strips_scheme_and_credentials() {
        assert_eq!(proxy_display("http://127.0.0.1:7890"), "127.0.0.1:7890");
        assert_eq!(
            proxy_display("http://user:pass@10.0.0.1:8080"),
            "10.0.0.1:8080",
            "凭据脱敏"
        );
        assert_eq!(proxy_display("socks5://1.2.3.4:1080"), "1.2.3.4:1080");
    }

    /// RET-B1 守卫钉（2026-09-15）：`fetch_serp` 统一走 `parse_bing_serp`，
    /// DDG/Google 恒 0 命中——有代理默认链若以这两引擎领头，链首各烧满
    /// `T_first` 死重。完整解析器就位（S4）前改链，前两引擎必须保持在
    /// Bing 系；集合不变（改序不改集，不构成白名单）。
    #[test]
    fn proxy_default_chain_leads_with_three_engines() {
        // 0bs ⑨（2026-09-26 裁决）：Bing 家族出集，代理默认链改以
        // 360/百度/DDG 三引擎（360 直连即用、百度经指纹伪装、DDG 需代理）。
        let leads: Vec<&str> = PROXY_DEFAULT_CHAIN.iter().take(3).copied().collect();
        assert_eq!(
            leads,
            vec!["360search", "baidu", "duckduckgo"],
            "有代理默认链 = 三引擎（不构成白名单）"
        );
        let mut sorted = PROXY_DEFAULT_CHAIN.to_vec();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            vec!["360search", "baidu", "duckduckgo"],
            "三引擎集合固定（显式 ORZ_RETRIEVAL_ENGINES 永远优先）"
        );
        assert_eq!(
            DEFAULT_DIRECT_CHAIN,
            &["360search", "baidu"],
            "直连默认链：360 + 百度（DDG 直连不可达，不入直连链）"
        );
    }

    #[test]
    fn proxy_switches_default_chain_and_probe_detail() {
        let proxied = LocalSegmentedConfig::from_env_with(|key| match key {
            ENV_SWITCH => Some("on".to_string()),
            ENV_PROXY => Some("http://user:pass@127.0.0.1:7890".to_string()),
            _ => None,
        });
        let ids: Vec<&str> = proxied.engines.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, PROXY_DEFAULT_CHAIN, "有代理默认链（不构成白名单）");
        assert_eq!(
            proxied.engine_chain_detail(),
            "360search,baidu,duckduckgo; proxy=on 127.0.0.1:7890",
            "探针读数带 proxy 状态与脱敏端点"
        );
        // 显式 ORZ_RETRIEVAL_ENGINES 永远优先（不设引擎白名单）。
        let explicit = LocalSegmentedConfig::from_env_with(|key| match key {
            ENV_PROXY => Some("http://127.0.0.1:7890".to_string()),
            ENV_ENGINES => Some("bing_cn".to_string()),
            _ => None,
        });
        assert_eq!(explicit.engines.len(), 1);
        assert_eq!(explicit.engines[0].id, "bing_cn");
        // 显式 none = 关闭（不落回 HTTPS_PROXY）。
        let off = LocalSegmentedConfig::from_env_with(|key| match key {
            ENV_PROXY => Some("none".to_string()),
            "HTTPS_PROXY" => Some("http://leak:1".to_string()),
            _ => None,
        });
        assert!(off.proxy.is_none());
        let off_ids: Vec<&str> = off.engines.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(
            off_ids, DEFAULT_DIRECT_CHAIN,
            "显式 none = 直连 ⇒ 直连默认链（0bs ⑨：360search, baidu）"
        );
    }
}
