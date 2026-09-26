//! `local_browser` lane (ADR-0010 §3.7) — real browser retrieval via a
//! headless Chrome/Edge on an isolated profile (GAP-RETRIEVAL-TOOLS
//! boundary, 2026-08-10: capability was explicitly `Unsupported`).
//!
//! MVP scope (user ruling 2026-08-10): pure webpage reading — gate the URL,
//! navigate, extract rendered text, record evidence. The PDF evidence
//! pipeline (2026-08-11) adds `download_or_read` — an owned-tab download
//! into an isolated staging dir for whitelisted paper-library domains
//! (`ORZ_PDF_BROWSER_DOMAINS`), consumed by the host's PDF evidence store.
//! 0bs ⑪ (2026-09-26): the action face is opened — typing / key presses /
//! clicks / scrolling act on the page like a user, with input simulation
//! (delay jitter, typo+correction, WindMouse trajectory, wheel cadence)
//! applied MECHANICALLY and permanently by [`input_sim`] (fixed standard v1,
//! single source): the model issues plain commands and never tunes the
//! rhythm. Only two actions keep gates: downloads and script execution
//! (user ruling 2026-09-25 — verify dialog + explicit user approval).
//! Arbitrary JS evaluation stays host-fixed-expression-only until that
//! approval channel is wired (recorded as open in the 0bs report).
//!
//! Architecture:
//! - [`BrowserSession`] trait — the tool side depends on this, so tests and
//!   conformance captures inject fakes (same pattern as `ModelGateway` /
//!   `LoopHost` / `connect_inprocess`).
//! - [`LocalBrowserManager`] — owns one [`CdpBrowserSession`] per session
//!   (launch once, reuse across runs, kill on shutdown) plus profile-dir
//!   cleanup; fail-closed with [`UnavailableBrowserSession`] when nothing
//!   was launched.
//! - `browser_read` — the single host-owned tool (project_doc_index
//!   pattern): `{ "url": "https://…", "mode": "full|preview|keywords" }` →
//!   read-only page text (P0-B step 4: full / preview / keyword excerpts).
//!   One tab per call, never handed to the model (§3.7.6).

mod cdp;
mod discovery;
mod input_sim;
mod serp;
mod url_gate;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use orz_loop::host::{ToolDef, ToolError, ToolResult};
use serde_json::json;

pub use cdp::{CdpBrowserSession, CdpConfig, CdpError, PageReadOutcome};
pub use discovery::{
    BrowserBinary, DiscoveryError, DiscoveryOrigin, ORZ_BROWSER_PATH_ENV, find_browser,
};
pub use serp::{SERP_MAX_SEARCH_QUERY_CHARS, SerpEngineAttempt, SerpResult};
pub use url_gate::{UrlGateError, check_navigation_url, check_navigation_url_sync};

/// `browser_read` read-scope values (P0-B step 4, 2026-08-14).
pub const MODE_FULL: &str = "full";
pub const MODE_PREVIEW: &str = "preview";
pub const MODE_KEYWORDS: &str = "keywords";

/// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：browser_read 读取
/// 模式——下传 CDP 层以决定资源拦截与等待语义（preview/keywords 用
/// 「可用文本就绪」+ 图片/字体/媒体拦截；full 保持 loadEventFired 终态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    Full,
    Preview,
    Keywords,
}

impl ReadMode {
    pub fn from_mode_str(s: &str) -> Option<Self> {
        match s {
            MODE_FULL => Some(Self::Full),
            MODE_PREVIEW => Some(Self::Preview),
            MODE_KEYWORDS => Some(Self::Keywords),
            _ => None,
        }
    }
}

/// S2-R P3 / P1-2b（2026-09-09, P1 设计 §2.1）：`browser_control` Phase 1
/// 最小动作集——导航级动作 + 状态观测，**不含正文读取**（正文读取仍走
/// `browser_read`；候选与 evidence 纪律不扩）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserControlAction {
    /// 直导航（URL gate / ACAF 与 browser_read 同级，CDP 层执行）。
    Navigate { url: String },
    /// 历史后退。
    Back,
    /// 历史前进。
    Forward,
    /// 刷新当前页。
    Refresh,
    /// 等待加载/文本就绪（有界超时）。
    WaitLoad,
    /// 当前页状态观测（url/title/nav_phase + 日志特征）。
    Snapshot,
    /// 检索引擎 SERP 链（Google → Bing → DuckDuckGo；机械固定 en-US 区域
    /// 参数）。搜索引擎需求由本动作承载，不加新工具。
    Search { query: String },
    // —— 0bs ⑪（2026-09-26）：输入动作面（用户令「模型对浏览器的使用是
    // 完全的……输入动作常驻，机械层自动做拟真」）。拟真时序全部由
    // `input_sim` v1 固定标准常驻施加，模型只下发语义指令、不调参。
    /// 键入到当前焦点（`submit=true` 时文本后按 Enter 提交）。
    Type { text: String, submit: bool },
    /// 单键（CDP 键名：`Enter` / `Tab` / `Escape` / `Backspace` /
    /// `ArrowDown` …）。
    Key { key: String },
    /// 点击：`selector`（CSS 选择器，主机自持 JS 模板解析元素中心）与
    /// 视口像素坐标 `x,y`（整数）二选一。轨迹＝WindMouse + 按下-抬起驻留。
    Click {
        selector: Option<String>,
        x: Option<i64>,
        y: Option<i64>,
    },
    /// 滚动：视口像素增量（正 = 向右/下；负 = 向左/上）。
    Scroll { dx: i64, dy: i64 },
    // —— 0bs ⑪（2026-09-26）：唯二门禁动作（用户令：下载与脚本执行一律
    // 弹验证＋用户明确批准）。二者恒过 ApprovalAlways 权限门（桥侧
    // `browser_action_requires_approval` → `force_prompt`）：逐次交互提示，
    // yolo／会话授予／auto 分类器／policy-allow 一切自动放行短路对其失效；
    // 无客户端应答 fail-closed。其余动作面全面放开。
    /// 下载一个公共 http(s) URL 到会话暂存目录
    /// （`.gsa/browser-downloads-<session8>/<call8>/`），回传落盘路径与
    /// 字节数。URL 门与 `browser_read` 同级（入口与每次重定向都重检）。
    Download { url: String },
    /// 在当前控制 tab 上执行一段 JavaScript 表达式（CDP
    /// `Runtime.evaluate`，`returnByValue`+`awaitPromise`；输出有界截断）。
    /// 执行面**只**经此批准动作可达——其余内部表达式仍是主机固定式
    /// （模型文本只从这个已批准入口进入 evaluate）。
    Script { code: String },
}

/// 每动作统一返回的机械段（P1 设计 §2.2）——状态 + 真实类别 + 导航阶段 +
/// url/title + 有界日志特征。`action_status=error` 是**状态化失败**（正常
/// Ok 回传，FP-2 真实类别，无教学句）；启动层失败才走 `ToolError`。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BrowserControlOutcome {
    /// `ok` / `error`。
    pub action_status: String,
    /// error 时的真实类别：dns / connection_reset / timeout / blocked /
    /// certificate / other（FP-2 口径）。
    pub error_class: Option<String>,
    /// idle / loading / completed / error。
    pub nav_phase: String,
    /// 当前落点 URL（识别重定向；无控制 tab 时为 None）。
    pub url: Option<String>,
    pub title: Option<String>,
    /// 有界日志特征——console error/warning + network error 节选，
    /// `[browser_log]` 容器、引用语域、脱敏（P1 设计 §2.2）。
    pub log: String,
    /// search 动作成功时命中的引擎（非 search 动作恒为 None）。
    pub engine: Option<String>,
    /// P2-1（2026-09-10）：search 动作的每引擎尝试明细（成功与失败都携带）
    /// ——失败时模型据此知道是哪个引擎、以何类别失败。非 search 动作恒为
    /// None。
    pub engine_attempts: Option<Vec<SerpEngineAttempt>>,
    /// search 动作成功时的结构化结果（≤10 条；tier/weight/reason 为
    /// 低质量来源机械标注，不硬过滤）。非 search 动作恒为 None。
    pub results: Option<Vec<SerpResult>>,
    /// 0bs ⑪（2026-09-26）：本会话所用浏览器**类型留档**（`headless` /
    /// `headed`）——每次使用时随信封回传（journal 侧即落档）；单会话单实例
    /// ⇒ 天然不混用（用户令 §4.6②）。
    pub browser_type: Option<String>,
    /// 0bs ⑪：输入动作实际施加的拟真事件数（输入动作恒 Some；非输入动作
    /// None）——留档与可核性（不是可调参数）。
    pub input_events: Option<usize>,
    /// 0bs ⑪：download 动作成功时的落盘信息（非 download 动作恒 None）。
    pub download: Option<BrowserDownloadInfo>,
    /// 0bs ⑪：script 动作的结果文本（`Runtime.evaluate` returnByValue 的
    /// JSON 取值，有界截断；非 script 动作恒 None）。
    pub script_output: Option<String>,
}

/// 0bs ⑪（2026-09-26）：`download` 动作的落盘读数——路径 + 字节数 +
/// 重定向后的落点 URL（URL 门在入口与重定向处均已重检）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserDownloadInfo {
    /// 绝对路径（会话暂存目录 `.gsa/browser-downloads-<session8>/<call8>/`）。
    pub path: String,
    pub bytes: u64,
    pub final_url: String,
}

/// 特征回传边界（P1 设计 §2.2 / §3.2-1）：≤12 行 / ≤2 KiB，超出截断标记。
pub const BROWSER_LOG_MAX_LINES: usize = 12;
pub const BROWSER_LOG_MAX_BYTES: usize = 2048;

/// Max characters of page text returned to the model in full mode (tool
/// contract — §3.7.7 download-size limits; aligned with the Python LBR
/// precedent of 100k chars). Truncation appends a mechanical footer so the
/// evidence layer maps it to `partial_text_observed`.
pub const MAX_READ_CHARS: usize = 100_000;

/// Max characters returned in preview mode — the first portion of the
/// rendered page text (P0-B step 4). A page short enough to fit is returned
/// complete (evidence stays `full_text_observed`); otherwise a mechanical
/// footer marks the preview as partial.
pub const PREVIEW_READ_CHARS: usize = 4_000;

/// Keyword-extraction bounds (P0-B step 4): at most this many terms, each
/// at most this many chars, excerpts of ±this many chars around a match, at
/// most this many merged excerpts per term, and this many total excerpt
/// chars returned to the model.
pub const MAX_KEYWORDS: usize = 16;
pub const MAX_KEYWORD_CHARS: usize = 64;
pub const KEYWORD_EXCERPT_RADIUS: usize = 160;
pub const KEYWORD_EXCERPTS_PER_TERM: usize = 3;
pub const KEYWORD_TOTAL_CHARS: usize = 12_000;

/// Footer prefix when the page text is truncated (evidence layer matches
/// this prefix to downgrade visibility — same shape as web_fetch's
/// `"\n\n[web_fetch content truncated: ...]"`: a leading blank line
/// separates the footer from page text).
pub const TRUNCATED_FOOTER_PREFIX: &str = "\n\n[browser_read content truncated:";

/// Outcome of one `download_or_read` call.
#[derive(Debug, Clone)]
pub enum BrowserDownloadOutcome {
    /// A download completed into the staging dir. The file is NOT yet
    /// validated — the PDF evidence pipeline verifies magic/parse.
    Pdf { path: PathBuf, final_url: String },
    /// The navigation produced a rendered page instead of a download.
    Page(PageReadOutcome),
}

/// The read-only browser surface the tools and probe rely on.
#[async_trait]
pub trait BrowserSession: Send + Sync {
    /// Read one URL. Implementations must enforce the URL gate themselves
    /// (fail-closed) and return explicit errors — never a silent fallback.
    /// `mode` selects the read scope: preview/keywords use resource blocking
    /// + text-ready wait, full waits for the load event (0k 2026-08-30).
    async fn read_page(&self, url: &str, mode: ReadMode) -> Result<PageReadOutcome, CdpError>;

    /// Download-or-read one URL into `download_dir` (PDF evidence pipeline,
    /// 2026-08-11). The URL gate applies on entry and on every redirect; a
    /// canceled download is an explicit error, never a fallback to a read.
    /// Implementations must clean the staging dir before downloading so
    /// "newest file" semantics are exact.
    async fn download_or_read(
        &self,
        url: &str,
        download_dir: &Path,
    ) -> Result<BrowserDownloadOutcome, CdpError>;

    /// S2-R P3 / P1-2b（2026-09-09, P1 设计 §2.1–§2.3）：执行一个浏览器
    /// 控制动作（导航/历史/刷新/等待/快照），作用于会话持续控制 tab——
    /// back/forward 历史依赖同一 target 不销毁。`timeout` 是导航/等待的
    /// 有界超时。实现必须自行执行 URL gate（fail-closed）并返回状态化
    /// 失败（`error_class` + 日志），不静默回退。
    async fn control(
        &self,
        action: BrowserControlAction,
        timeout: Duration,
    ) -> Result<BrowserControlOutcome, CdpError>;

    /// True when a browser is actually available (drives tool declaration).
    fn ready(&self) -> bool;

    /// 0bs ⑪（2026-09-26）：本会话所用浏览器**类型留档**（`headless` /
    /// `headed`）——每次使用时随信封回传（journal 落档；用户令 §4.6②：
    /// 单次子代理进程只用一类）。fake/stub 实现默认 `unknown`。
    fn browser_type(&self) -> &'static str {
        "unknown"
    }

    /// P2-3（2026-09-10）：本会话已消耗的 SERP **引擎导航次数**与会话上限
    /// （`(navigations, ceiling)`）。宿主把它作为机械事实上报给 loop，loop
    /// 据此为检索车道保留底线额度（0v 设计 §3.2.5）——车道策略不在本层
    /// （宿主没有车道身份）。`None`（默认）= 该实现没有 SERP 会话计数器
    /// （unavailable/stub 形态）；返回的是**跨车道共享**的同一计数器。
    async fn serp_session_navigations(&self) -> Option<(u32, u32)> {
        None
    }

    /// Session teardown: kill the process tree, best-effort remove the
    /// profile directory.
    async fn shutdown(&self);
}

/// The real implementation: one headless browser per session, launched
/// lazily by the capability probe and reused across runs (same profile dir →
/// no profile-lock conflicts).
pub struct LocalBrowserManager {
    /// v2 (RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批, 2026-08-30)：
    /// 会话以 Arc 共享；`read_page` 只短取 Arc（锁内 clone、锁外读取）——
    /// 同轮多个 `browser_read` 真正并发（tab 池在会话内部约束并发）。
    inner: tokio::sync::Mutex<Option<Arc<CdpBrowserSession>>>,
}

impl LocalBrowserManager {
    /// Wrap an already-launched session (the probe does the launching).
    pub fn new(session: CdpBrowserSession) -> Self {
        Self {
            inner: tokio::sync::Mutex::new(Some(Arc::new(session))),
        }
    }

    /// The profile directory this manager would use — kept here so the
    /// probe and the manager agree on one layout.
    pub fn profile_dir_for(workspace: &std::path::Path, session_id: &str) -> PathBuf {
        let suffix: String = session_id.chars().take(8).collect();
        workspace
            .join(".gsa")
            .join(format!("chrome-profile-{suffix}"))
    }
}

#[async_trait]
impl BrowserSession for LocalBrowserManager {
    async fn read_page(&self, url: &str, mode: ReadMode) -> Result<PageReadOutcome, CdpError> {
        let session = self
            .inner
            .lock()
            .await
            .clone()
            .ok_or_else(|| CdpError::Io("browser session not launched".into()))?;
        session.read_page(url, mode).await
    }

    async fn control(
        &self,
        action: BrowserControlAction,
        timeout: Duration,
    ) -> Result<BrowserControlOutcome, CdpError> {
        let session = self
            .inner
            .lock()
            .await
            .clone()
            .ok_or_else(|| CdpError::Io("browser session not launched".into()))?;
        session.control(action, timeout).await
    }

    async fn download_or_read(
        &self,
        url: &str,
        download_dir: &Path,
    ) -> Result<BrowserDownloadOutcome, CdpError> {
        let session = self
            .inner
            .lock()
            .await
            .clone()
            .ok_or_else(|| CdpError::Io("browser session not launched".into()))?;
        session.download_or_read(url, download_dir).await
    }

    fn ready(&self) -> bool {
        // try_lock: a read in progress must not block declaration checks.
        // The process must be alive too — a browser killed out-of-band (crash,
        // OOM, external kill) must report not-ready so the next probe
        // relaunches it (D-3 self-healing covers process death, not just
        // launch failure).
        self.inner
            .try_lock()
            .map(|g| g.as_ref().is_some_and(|s| s.is_alive()))
            .unwrap_or(false)
    }

    /// 0bs ⑪：类型留档——短取锁读会话（try_lock：读进行中不阻塞留档）。
    fn browser_type(&self) -> &'static str {
        self.inner
            .try_lock()
            .ok()
            .and_then(|guard| guard.as_ref().map(|session| session.browser_type()))
            .unwrap_or("unknown")
    }

    /// P2-3：转发会话内部的 SERP 计数器（跨 run 存活；锁内 clone、锁外
    /// 读取，与 read_page/control 同一纪律）。
    async fn serp_session_navigations(&self) -> Option<(u32, u32)> {
        let session = self.inner.lock().await.clone()?;
        Some(session.serp_session_navigations())
    }

    async fn shutdown(&self) {
        let mut guard = self.inner.lock().await;
        if let Some(session) = guard.take() {
            session.shutdown().await;
        }
    }
}

/// Fail-closed placeholder — returned when no browser was launched (probe
/// failed or mode is off). `browser_read` must report an explicit error
/// through this, never silently succeed.
pub struct UnavailableBrowserSession {
    reason: String,
}

impl UnavailableBrowserSession {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

#[async_trait]
impl BrowserSession for UnavailableBrowserSession {
    async fn read_page(&self, _url: &str, _mode: ReadMode) -> Result<PageReadOutcome, CdpError> {
        Err(CdpError::Io(format!(
            "browser unavailable: {}",
            self.reason
        )))
    }

    async fn control(
        &self,
        _action: BrowserControlAction,
        _timeout: Duration,
    ) -> Result<BrowserControlOutcome, CdpError> {
        Err(CdpError::Io(format!(
            "browser unavailable: {}",
            self.reason
        )))
    }

    async fn download_or_read(
        &self,
        _url: &str,
        _download_dir: &Path,
    ) -> Result<BrowserDownloadOutcome, CdpError> {
        Err(CdpError::Io(format!(
            "browser unavailable: {}",
            self.reason
        )))
    }

    fn ready(&self) -> bool {
        false
    }

    async fn shutdown(&self) {}
}

/// Shared handle type used by `OrzHost` and the probe.
pub type SharedBrowser = Arc<dyn BrowserSession>;

/// Serializes browser launch vs teardown on the SAME profile directory
/// (2026-08-10 review M4): `close_session` detaches `shutdown()`, so a
/// same-session immediate re-open could race the old teardown (taskkill +
/// profile-dir delete with up to 2s of retries) against the new Chrome's
/// launch on the same profile — the delete could pull the new browser's
/// DevToolsActivePort, or the two Chromes collide on the profile
/// SingletonLock. Both sides hold the profile's async lock, so the probe
/// simply waits out the detached teardown. Entries are one tiny Arc per
/// distinct profile path (one per session) and are never evicted — bounded
/// by the session table in practice.
static PROFILE_LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<tokio::sync::Mutex<()>>>>> =
    OnceLock::new();

fn profile_lock(profile_dir: &Path) -> Arc<tokio::sync::Mutex<()>> {
    let map = PROFILE_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    map.lock()
        .unwrap()
        .entry(profile_dir.to_path_buf())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// Tool definition for `browser_read` (host-owned; declared only when the
/// browser is ready — declaration and probe are the same source of truth).
pub fn browser_read_tool_def() -> ToolDef {
    ToolDef {
        name: "browser_read".to_string(),
        description: "Read a webpage with the local headless browser: navigates \
             to the URL (http/https public pages only — file://, localhost, \
             private IPs and cloud metadata are blocked by policy), waits for \
             load, and returns the rendered page text + title + final URL. \
             mode=full (default) returns up to 100000 chars of page text; \
             mode=preview returns the first 4000 chars; mode=keywords returns \
             bounded excerpts around the given keywords. \
             One tab per call, closed after reading; page content is evidence \
             only. PDFs, forms and JavaScript evaluation are not supported."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Public http(s) URL to read" },
                "mode": {
                    "type": "string",
                    "enum": ["full", "preview", "keywords"],
                    "description": "Read scope (default full): full returns up to \
                        100000 chars of page text; preview returns the first 4000 \
                        chars; keywords returns bounded excerpts around the given \
                        keywords.",
                },
                "keywords": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": MAX_KEYWORD_CHARS,
                    },
                    "maxItems": 16,
                    "description": "Required when mode=keywords: 1..16 non-empty \
                        terms (each up to 64 chars) to extract excerpts for.",
                },
            },
            "required": ["url"],
        }),
    }
}

/// The `browser_read` tool execution: parse args strictly, delegate to the
/// session, map outcomes to `ToolResult` (exit 0 with the read text + a
/// structured footer) and errors to explicit `ToolError`s.
pub async fn handle_browser_read(
    browser: &dyn BrowserSession,
    args: &serde_json::Value,
) -> Result<ToolResult, ToolError> {
    // Strict argument parsing: exactly one `url` string, non-empty. Unknown
    // keys are rejected (not silently ignored) — every failure carries a
    // stable `[browser_read_*]` error code (§3.7.2 explicit errors).
    let obj = args.as_object().ok_or_else(|| {
        ToolError::ExecutionFailed(
            "browser_read failed [browser_read_invalid_arguments]: arguments must be a JSON object"
                .to_string(),
        )
    })?;
    let unknown: Vec<&String> = obj
        .keys()
        .filter(|k| !matches!(k.as_str(), "url" | "mode" | "keywords"))
        .collect();
    if !unknown.is_empty() {
        return Err(ToolError::ExecutionFailed(format!(
            "browser_read failed [browser_read_invalid_arguments]: unknown argument(s): {}",
            unknown
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let url = obj
        .get("url")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed(
                "browser_read failed [browser_read_missing_url]: requires a non-empty `url` argument"
                    .to_string(),
            )
        })?;
    let mode = match obj.get("mode") {
        None => MODE_FULL,
        Some(v) => match v.as_str() {
            Some(m @ (MODE_FULL | MODE_PREVIEW | MODE_KEYWORDS)) => m,
            _ => {
                return Err(ToolError::ExecutionFailed(
                    "browser_read failed [browser_read_invalid_arguments]: `mode` must be \
                     one of \"full\", \"preview\" or \"keywords\""
                        .to_string(),
                ));
            }
        },
    };
    let keywords = match obj.get("keywords") {
        Some(_) if mode != MODE_KEYWORDS => {
            return Err(ToolError::ExecutionFailed(
                "browser_read failed [browser_read_invalid_arguments]: `keywords` is only \
                 allowed in keywords mode"
                    .to_string(),
            ));
        }
        None if mode == MODE_KEYWORDS => {
            return Err(ToolError::ExecutionFailed(
                "browser_read failed [browser_read_invalid_arguments]: keywords mode requires \
                 a non-empty `keywords` array"
                    .to_string(),
            ));
        }
        None => Vec::new(),
        Some(v) => {
            let arr = v.as_array().ok_or_else(|| {
                ToolError::ExecutionFailed(
                    "browser_read failed [browser_read_invalid_arguments]: `keywords` must \
                     be an array of non-empty strings"
                        .to_string(),
                )
            })?;
            if arr.is_empty() || arr.len() > MAX_KEYWORDS {
                return Err(ToolError::ExecutionFailed(format!(
                    "browser_read failed [browser_read_invalid_arguments]: `keywords` must \
                     contain 1..{MAX_KEYWORDS} terms"
                )));
            }
            let mut kws = Vec::with_capacity(arr.len());
            for item in arr {
                let term = item
                    .as_str()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        ToolError::ExecutionFailed(
                            "browser_read failed [browser_read_invalid_arguments]: \
                             `keywords` entries must be non-empty strings"
                                .to_string(),
                        )
                    })?;
                if term.chars().count() > MAX_KEYWORD_CHARS {
                    return Err(ToolError::ExecutionFailed(format!(
                        "browser_read failed [browser_read_invalid_arguments]: keyword \
                         exceeds {MAX_KEYWORD_CHARS} chars"
                    )));
                }
                if kws.iter().any(|k| k == term) {
                    return Err(ToolError::ExecutionFailed(
                        "browser_read failed [browser_read_invalid_arguments]: \
                         `keywords` must be unique"
                            .to_string(),
                    ));
                }
                kws.push(term.to_string());
            }
            kws
        }
    };

    // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：模式下传——
    // preview/keywords 由 CDP 层做资源拦截 + 文本就绪等待。
    let read_mode = ReadMode::from_mode_str(mode).expect("mode validated above");
    match browser.read_page(url, read_mode).await {
        Ok(outcome) => {
            let mut text = outcome.text;
            let mut truncated = false;
            let content = match mode {
                MODE_FULL => {
                    if text.chars().count() > MAX_READ_CHARS {
                        text = text.chars().take(MAX_READ_CHARS).collect();
                        truncated = true;
                    }
                    let mut output = text;
                    if truncated {
                        output.push_str(TRUNCATED_FOOTER_PREFIX);
                        output.push_str(&format!(" {} chars, page text only]", MAX_READ_CHARS));
                    }
                    output
                }
                MODE_PREVIEW => {
                    if text.chars().count() > PREVIEW_READ_CHARS {
                        text = text.chars().take(PREVIEW_READ_CHARS).collect();
                        truncated = true;
                        text.push_str(TRUNCATED_FOOTER_PREFIX);
                        text.push_str(&format!(
                            " preview, first {} chars, page text only]",
                            PREVIEW_READ_CHARS
                        ));
                    }
                    text
                }
                MODE_KEYWORDS => {
                    // Review fix (2026-08-14): bound the extraction INPUT
                    // too — full mode caps at MAX_READ_CHARS, but keywords
                    // previously scanned the whole rendered page (a huge
                    // page cost a full copy + scan). Matches beyond the
                    // cap are out of scope for the excerpt; the footer
                    // says so.
                    let mut input_capped = false;
                    if text.chars().count() > MAX_READ_CHARS {
                        text = text.chars().take(MAX_READ_CHARS).collect();
                        input_capped = true;
                    }
                    let (excerpts, emitted_terms, excerpt_count) =
                        keyword_excerpts(&text, &keywords);
                    let mut output = if excerpts.is_empty() {
                        format!(
                            "[browser_read keywords: no matching excerpts for {} term(s)]",
                            keywords.len()
                        )
                    } else {
                        excerpts
                    };
                    output.push_str(TRUNCATED_FOOTER_PREFIX);
                    let cap_note = if input_capped {
                        format!(", input capped at {MAX_READ_CHARS} chars")
                    } else {
                        String::new()
                    };
                    output.push_str(&format!(
                        " keyword excerpts ({} terms, {} excerpts){cap_note}, page text only]",
                        emitted_terms, excerpt_count
                    ));
                    truncated = true;
                    output
                }
                _ => unreachable!("mode validated above"),
            };
            let out = json!({
                "url": outcome.final_url,
                "title": outcome.title,
                "mode": mode,
                "content": content,
                "truncated": truncated,
            });
            let mut out = out;
            if mode == MODE_KEYWORDS {
                out["keywords"] = json!(keywords);
            }
            if mode == MODE_PREVIEW {
                out["preview_chars"] = json!(PREVIEW_READ_CHARS);
            }
            Ok(ToolResult {
                output: serde_json::to_string(&out).map_err(|e| {
                    ToolError::ExecutionFailed(format!("browser_read serialize: {e}"))
                })?,
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            })
        }
        Err(CdpError::UrlGate(gate_err)) => Err(ToolError::ExecutionFailed(format!(
            "browser_read refused [{}]: {}",
            gate_err.tool_error_code(),
            gate_err
        ))),
        Err(CdpError::EmptyContent) => Err(ToolError::ExecutionFailed(
            "browser_read failed [browser_read_empty_content]: page produced no \
             readable text (empty page, or a login/CAPTCHA wall — not a silent \
             success)"
                .to_string(),
        )),
        Err(CdpError::LoadTimeout { .. }) => Err(ToolError::ExecutionFailed(
            "browser_read failed [browser_read_load_timeout]: page load timed out".to_string(),
        )),
        Err(CdpError::TotalTimeout { .. }) => Err(ToolError::ExecutionFailed(
            "browser_read failed [browser_read_total_timeout]: total page-read \
             budget exceeded"
                .to_string(),
        )),
        Err(e) => Err(ToolError::ExecutionFailed(format!(
            "browser_read failed [browser_read_failed]: {e}"
        ))),
    }
}

/// S2-R P3 / P1-2b（2026-09-09, P1 设计 §2.1/§2.4）：`browser_control`
/// 工具定义——导航级控制（external lane、检索启用会话声明）。静态车道
/// 标注由 projection 追加（与 browser_read 同路径）；动作返回状态 + 有界
/// 日志特征，正文读取仍走 browser_read。
pub fn browser_control_tool_def() -> ToolDef {
    ToolDef {
        name: "browser_control".to_string(),
        description: "Control the local browser lane: navigate to a URL, \
             go back/forward in history, refresh, wait for load, snapshot \
             the current page state, run a bounded search-engine SERP \
             lookup (Google → Bing → DuckDuckGo, fixed en-US region), or act \
             on the page like a user — type text (type), press a key (key), \
             click (click), scroll (scroll). Input actions are driven through \
             a resident human-like simulation (per-char delays, typo and \
             correction, mouse trajectory, wheel cadence); you issue plain \
             commands and never tune the rhythm. Each action returns \
             action_status + a real error class (dns / connection_reset / \
             timeout / blocked / certificate / other) + navigation phase + \
             current url/title + a bounded [browser_log] excerpt \
             (console/network errors only, never page prose) + browser_type \
             (headless/headed; input actions also carry input_events). \
             Reading page content is done with browser_read — this tool never \
             returns full page text. Public http(s) URLs only (file://, \
             localhost, private IPs and cloud metadata are blocked by policy)."
            .to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "navigate", "back", "forward", "refresh", "wait_load", "snapshot",
                        "search", "type", "key", "click", "scroll", "download", "script",
                    ],
                    "description": "Action to perform: navigate(url) direct \
                        navigation; back/forward history; refresh reloads the \
                        current page; wait_load waits up to timeout_secs for \
                        load/text readiness; snapshot reports the current page \
                        state without navigating; search(query) runs the fixed \
                        SERP chain and returns up to 10 weighted organic \
                        results; type/key/click/scroll act on the page like a \
                        user (resident human-like simulation, no tuning); \
                        download(url) saves a public http(s) URL to the \
                        session download dir (ALWAYS asks the user for \
                        explicit approval — download is one of the two gated \
                        actions); script evaluates a JavaScript expression on \
                        the current control tab (ALWAYS asks the user for \
                        explicit approval — script execution is the other \
                        gated action; output is bounded).",
                },
                "url": {
                    "type": "string",
                    "description": "Required when action=navigate or \
                        action=download: public http(s) URL (file://, \
                        localhost, private IPs and cloud metadata are blocked).",
                },
                "query": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 500,
                    "description": "Required when action=search: the search \
                        query sent to the fixed engine chain.",
                },
                "timeout_secs": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 30,
                    "description": "Optional bounded wait for navigate / \
                        wait_load / download (default 30s, capped at 30s).",
                },
                "text": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": CONTROL_TYPE_MAX_CHARS,
                    "description": "Required when action=type: text to type \
                        into the focused element.",
                },
                "submit": {
                    "type": "boolean",
                    "description": "action=type: press Enter after typing \
                        (default false).",
                },
                "key": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": CONTROL_KEY_MAX_CHARS,
                    "description": "Required when action=key: CDP key name \
                        (Enter / Tab / Escape / Backspace / ArrowDown / …).",
                },
                "selector": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": CONTROL_SELECTOR_MAX_CHARS,
                    "description": "action=click: CSS selector; the host \
                        resolves the element center and moves the mouse there \
                        (takes precedence over x/y).",
                },
                "x": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": CONTROL_COORD_MAX,
                    "description": "action=click: viewport x in CSS pixels \
                        (use with y when selector is omitted).",
                },
                "y": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": CONTROL_COORD_MAX,
                    "description": "action=click: viewport y in CSS pixels \
                        (use with x when selector is omitted).",
                },
                "dx": {
                    "type": "integer",
                    "minimum": -CONTROL_SCROLL_MAX_PX,
                    "maximum": CONTROL_SCROLL_MAX_PX,
                    "description": "action=scroll: horizontal wheel delta in \
                        pixels (negative = left).",
                },
                "dy": {
                    "type": "integer",
                    "minimum": -CONTROL_SCROLL_MAX_PX,
                    "maximum": CONTROL_SCROLL_MAX_PX,
                    "description": "action=scroll: vertical wheel delta in \
                        pixels (negative = up).",
                },
                "script": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": CONTROL_SCRIPT_MAX_CHARS,
                    "description": "Required when action=script: JavaScript \
                        expression evaluated on the current control tab \
                        (returnByValue; output bounded to \
                        4000 chars). Requires explicit user approval every \
                        time.",
                },
            },
            "required": ["action"],
        }),
    }
}

/// S2-R P3 / P1-2b（2026-09-09, P1 设计 §2.1–§2.2）：`browser_control`
/// 执行——严格解析参数，调会话层动作，统一返回机械信封（成功或状态化
/// 失败都 Ok 回传；仅参数错误/序列化错误走 `ToolError`）。启动层失败由
/// host 懒启动路径负责（`BrowserLaunchFailed`），不进本函数。
pub async fn handle_browser_control(
    browser: &dyn BrowserSession,
    args: &serde_json::Value,
) -> Result<ToolResult, ToolError> {
    let obj = args.as_object().ok_or_else(|| {
        ToolError::ExecutionFailed(
            "browser_control failed [browser_control_invalid_arguments]: \
             arguments must be a JSON object"
                .to_string(),
        )
    })?;
    let unknown: Vec<&String> = obj
        .keys()
        .filter(|k| {
            !matches!(
                k.as_str(),
                "action"
                    | "url"
                    | "query"
                    | "timeout_secs"
                    // 0bs ⑪（2026-09-26）：输入动作参数（type/key/click/scroll）。
                    | "text"
                    | "submit"
                    | "key"
                    | "selector"
                    | "x"
                    | "y"
                    | "dx"
                    | "dy"
                    // 0bs ⑪：唯二门禁动作参数（script 本体的源码字段）。
                    | "script"
            )
        })
        .collect();
    if !unknown.is_empty() {
        return Err(ToolError::ExecutionFailed(format!(
            "browser_control failed [browser_control_invalid_arguments]: \
             unknown argument(s): {}",
            unknown
                .iter()
                .map(|k| k.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    let action_name = obj
        .get("action")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            ToolError::ExecutionFailed(
                "browser_control failed [browser_control_missing_action]: \
                 requires a non-empty `action` argument"
                    .to_string(),
            )
        })?;
    let url_arg = obj.get("url").and_then(|v| v.as_str()).map(str::to_string);
    let action = match action_name {
        "navigate" => {
            // 0bs ⑪：输入参数只在输入动作上（navigate 不例外）。
            reject_input_params(action_name, &obj)?;
            if obj.contains_key("url") && url_arg.is_none() {
                return Err(ToolError::ExecutionFailed(
                    "browser_control failed [browser_control_invalid_arguments]: \
                     `url` must be a string"
                        .to_string(),
                ));
            }
            let url = url_arg
                .as_ref()
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_missing_url]: \
                         action=navigate requires a non-empty `url` argument"
                            .to_string(),
                    )
                })?;
            BrowserControlAction::Navigate { url }
        }
        "back" => {
            reject_url_for_action(action_name, &obj)?;
            BrowserControlAction::Back
        }
        "forward" => {
            reject_url_for_action(action_name, &obj)?;
            BrowserControlAction::Forward
        }
        "refresh" => {
            reject_url_for_action(action_name, &obj)?;
            BrowserControlAction::Refresh
        }
        "wait_load" => {
            reject_url_for_action(action_name, &obj)?;
            BrowserControlAction::WaitLoad
        }
        "snapshot" => {
            reject_url_for_action(action_name, &obj)?;
            BrowserControlAction::Snapshot
        }
        "search" => {
            reject_url_for_action(action_name, &obj)?;
            let query = obj
                .get("query")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_missing_query]: \
                         action=search requires a non-empty `query` argument"
                            .to_string(),
                    )
                })?;
            if query.chars().count() > SERP_MAX_SEARCH_QUERY_CHARS {
                return Err(ToolError::ExecutionFailed(format!(
                    "browser_control failed [browser_control_invalid_arguments]: \
                     `query` must be at most {SERP_MAX_SEARCH_QUERY_CHARS} chars"
                )));
            }
            BrowserControlAction::Search {
                query: query.to_string(),
            }
        }
        // —— 0bs ⑪（2026-09-26）：输入动作（拟真时序由机械层常驻施加） ——
        "type" => {
            reject_url_for_action(action_name, &obj)?;
            reject_keys(
                action_name,
                &obj,
                &["query", "key", "selector", "x", "y", "dx", "dy"],
            )?;
            let text = obj
                .get("text")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_missing_text]: \
                         action=type requires a non-empty `text` argument"
                            .to_string(),
                    )
                })?;
            if text.chars().count() > CONTROL_TYPE_MAX_CHARS {
                return Err(ToolError::ExecutionFailed(format!(
                    "browser_control failed [browser_control_invalid_arguments]: \
                     `text` must be at most {CONTROL_TYPE_MAX_CHARS} chars \
                     (realistic typing is ~200ms per char)"
                )));
            }
            let submit = match obj.get("submit") {
                None => false,
                Some(v) => v.as_bool().ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_invalid_arguments]: \
                         `submit` must be a boolean"
                            .to_string(),
                    )
                })?,
            };
            BrowserControlAction::Type { text, submit }
        }
        "key" => {
            reject_url_for_action(action_name, &obj)?;
            reject_keys(
                action_name,
                &obj,
                &["query", "text", "submit", "selector", "x", "y", "dx", "dy"],
            )?;
            let key = obj
                .get("key")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .filter(|s| !s.is_empty() && s.chars().count() <= CONTROL_KEY_MAX_CHARS)
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(format!(
                        "browser_control failed [browser_control_missing_key]: \
                         action=key requires a non-empty `key` of at most \
                         {CONTROL_KEY_MAX_CHARS} chars (CDP key name, e.g. Enter)"
                    ))
                })?;
            BrowserControlAction::Key { key }
        }
        "click" => {
            reject_url_for_action(action_name, &obj)?;
            reject_keys(action_name, &obj, &["query", "text", "submit", "key", "dx", "dy"])?;
            let selector = match obj.get("selector") {
                None => None,
                Some(v) => {
                    let s = v.as_str().map(str::to_string).filter(|s| !s.is_empty());
                    let Some(s) = s else {
                        return Err(ToolError::ExecutionFailed(
                            "browser_control failed [browser_control_invalid_arguments]: \
                             `selector` must be a non-empty string"
                                .to_string(),
                        ));
                    };
                    if s.chars().count() > CONTROL_SELECTOR_MAX_CHARS {
                        return Err(ToolError::ExecutionFailed(format!(
                            "browser_control failed [browser_control_invalid_arguments]: \
                             `selector` must be at most {CONTROL_SELECTOR_MAX_CHARS} chars"
                        )));
                    }
                    Some(s)
                }
            };
            let coord = |name: &str| -> Result<Option<i64>, ToolError> {
                match obj.get(name) {
                    None => Ok(None),
                    Some(v) => {
                        let n = v.as_i64().filter(|n| (0..=CONTROL_COORD_MAX).contains(n));
                        n.map(Some).ok_or_else(|| {
                            ToolError::ExecutionFailed(format!(
                                "browser_control failed [browser_control_invalid_arguments]: \
                                 `{name}` must be an integer in 0..={CONTROL_COORD_MAX}"
                            ))
                        })
                    }
                }
            };
            let (x, y) = (coord("x")?, coord("y")?);
            match (&selector, x, y) {
                // 二选一（严格）：selector 独占，或 x+y 成对；混给也拒绝。
                (Some(_), None, None) | (None, Some(_), Some(_)) => {}
                _ => {
                    return Err(ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_invalid_arguments]: \
                         action=click requires exactly one of `selector` or `x`+`y`"
                            .to_string(),
                    ));
                }
            }
            BrowserControlAction::Click { selector, x, y }
        }
        "scroll" => {
            reject_url_for_action(action_name, &obj)?;
            reject_keys(
                action_name,
                &obj,
                &["query", "text", "submit", "key", "selector", "x", "y"],
            )?;
            let delta = |name: &str| -> Result<i64, ToolError> {
                match obj.get(name) {
                    None => Ok(0),
                    Some(v) => v
                        .as_i64()
                        .filter(|n| n.abs() <= CONTROL_SCROLL_MAX_PX)
                        .ok_or_else(|| {
                            ToolError::ExecutionFailed(format!(
                                "browser_control failed [browser_control_invalid_arguments]: \
                                 `{name}` must be an integer in \
                                 ±{CONTROL_SCROLL_MAX_PX}"
                            ))
                        }),
                }
            };
            let (dx, dy) = (delta("dx")?, delta("dy")?);
            if dx == 0 && dy == 0 {
                return Err(ToolError::ExecutionFailed(
                    "browser_control failed [browser_control_invalid_arguments]: \
                     action=scroll requires a non-zero `dx` or `dy`"
                        .to_string(),
                ));
            }
            BrowserControlAction::Scroll { dx, dy }
        }
        // —— 0bs ⑪（2026-09-26）：唯二门禁动作 ——
        "download" => {
            reject_keys(
                action_name,
                &obj,
                &["query", "text", "submit", "key", "selector", "x", "y", "dx", "dy", "script"],
            )?;
            if obj.contains_key("url") && url_arg.is_none() {
                return Err(ToolError::ExecutionFailed(
                    "browser_control failed [browser_control_invalid_arguments]: \
                     `url` must be a string"
                        .to_string(),
                ));
            }
            let url = url_arg
                .as_ref()
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_missing_url]: \
                         action=download requires a non-empty `url` argument"
                            .to_string(),
                    )
                })?;
            BrowserControlAction::Download { url }
        }
        "script" => {
            reject_url_for_action(action_name, &obj)?;
            reject_keys(
                action_name,
                &obj,
                &["query", "text", "submit", "key", "selector", "x", "y", "dx", "dy"],
            )?;
            let code = obj
                .get("script")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_missing_script]: \
                         action=script requires a non-empty `script` argument"
                            .to_string(),
                    )
                })?;
            if code.chars().count() > CONTROL_SCRIPT_MAX_CHARS {
                return Err(ToolError::ExecutionFailed(format!(
                    "browser_control failed [browser_control_invalid_arguments]: \
                     `script` must be at most {CONTROL_SCRIPT_MAX_CHARS} chars"
                )));
            }
            BrowserControlAction::Script { code }
        }
        _ => {
            return Err(ToolError::ExecutionFailed(format!(
                "browser_control failed [browser_control_invalid_arguments]: \
                 `action` must be one of \"navigate\", \"back\", \"forward\", \
                 \"refresh\", \"wait_load\", \"snapshot\", \"search\", \"type\", \
                 \"key\", \"click\", \"scroll\", \"download\" or \"script\" \
                 (got {action_name})"
            )));
        }
    };
    let timeout = match obj.get("timeout_secs") {
        None => DEFAULT_CONTROL_TIMEOUT,
        Some(v) => {
            let secs = v
                .as_u64()
                .filter(|&s| (1..=30).contains(&s))
                .ok_or_else(|| {
                    ToolError::ExecutionFailed(
                        "browser_control failed [browser_control_invalid_arguments]: \
                     `timeout_secs` must be an integer in 1..=30"
                            .to_string(),
                    )
                })?;
            Duration::from_secs(secs)
        }
    };
    let outcome = browser
        .control(action.clone(), timeout)
        .await
        .map_err(|e| {
            // 状态机/连接层意外错误按普通失败回传（FP-2 真实类别文本）。
            ToolError::ExecutionFailed(format!(
                "browser_control failed [browser_control_failed]: {e}"
            ))
        })?;
    let mut out = json!({
        "action": action_name,
        "action_status": outcome.action_status,
        "error_class": outcome.error_class,
        "nav_phase": outcome.nav_phase,
        "log": outcome.log,
        // 0bs ⑪（2026-09-26）：类型留档——每次使用随信封回传（journal 落档；
        // 单会话单实例 ⇒ 天然不混用）。
        "browser_type": outcome
            .browser_type
            .clone()
            .unwrap_or_else(|| browser.browser_type().to_string()),
    });
    if let Some(events) = outcome.input_events {
        out["input_events"] = json!(events);
    }
    // 0bs ⑪（2026-09-26）：唯二门禁动作的机械读数——download 落盘信息与
    // script 结果文本（有界）。两者只在对应动作上出现。
    if let Some(download) = outcome.download {
        out["download"] = json!({
            "path": download.path,
            "bytes": download.bytes,
            "final_url": download.final_url,
        });
    }
    if let Some(script_output) = outcome.script_output {
        out["script_output"] = json!(script_output);
    }
    if action_name == "search" {
        if let Some(engine) = outcome.engine {
            out["engine"] = json!(engine);
        }
        if let Some(attempts) = outcome.engine_attempts {
            out["engine_attempts"] = json!(attempts);
        }
        if let Some(results) = outcome.results {
            out["results"] = json!(results);
        }
    } else {
        out["url"] = json!(outcome.url);
        out["title"] = json!(outcome.title);
    }
    Ok(ToolResult {
        output: serde_json::to_string(&out)
            .map_err(|e| ToolError::ExecutionFailed(format!("browser_control serialize: {e}")))?,
        exit_code: Some(0),
        output_encoding: None,
        structured: None,
        ..Default::default()
    })
}

/// navigate 以外的动作不接受 `url` 参数（严格解析纪律与 browser_read 同）。
fn reject_url_for_action(
    action: &str,
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), ToolError> {
    if obj.contains_key("url") {
        return Err(ToolError::ExecutionFailed(format!(
            "browser_control failed [browser_control_invalid_arguments]: \
             `url` is only allowed when action=navigate (action={action})"
        )));
    }
    // 0bs ⑪（2026-09-26）：输入参数只允许出现在输入动作上（严格解析——
    // 非输入动作携带 text/submit/key/selector/x/y/dx/dy 一律拒绝）。
    if !matches!(action, "type" | "key" | "click" | "scroll") {
        reject_input_params(action, obj)?;
    }
    Ok(())
}

/// 控制动作默认等待上限（P1 设计 §2.1：默认 ≤30s，可配）。
pub const DEFAULT_CONTROL_TIMEOUT: Duration = Duration::from_secs(30);

// —— 0bs ⑪（2026-09-26）：输入动作参数边界（机械层拟真时序 ⇒ 语义面只需
// 有界常量；模型的输入本体不做动作级限制，边界只防误用/滥用） ——
/// `type` 动作文本上限（逐字符 200ms 拟真 ⇒ 800 字 ≈ 160–224s 真实耗时）。
pub const CONTROL_TYPE_MAX_CHARS: usize = 800;
/// `key` 动作键名上限（CDP 键名，如 `Enter`）。
pub const CONTROL_KEY_MAX_CHARS: usize = 32;
/// `click` 动作 CSS 选择器上限。
pub const CONTROL_SELECTOR_MAX_CHARS: usize = 500;
/// `script` 动作源码上限（0bs ⑪ 唯二门禁动作；模型下发的表达式本体）。
pub const CONTROL_SCRIPT_MAX_CHARS: usize = 8_000;
/// `script` 动作回传输出上限（`Runtime.evaluate` returnByValue 的 JSON
/// 取值，超出机械截断——输出进对话正文，必须有界）。
pub const CONTROL_SCRIPT_MAX_OUTPUT_CHARS: usize = 4_000;
/// `click` 动作视口坐标上限（0..=20000）。
pub const CONTROL_COORD_MAX: i64 = 20_000;
/// `scroll` 动作单轴像素上限（±20000；110px/格 ⇒ ≤182 格）。
pub const CONTROL_SCROLL_MAX_PX: i64 = 20_000;

/// 0bs ⑪：动作专属参数纪律——`keys` 中任一键出现即拒绝（严格解析；输入
/// 参数只允许出现在对应动作上）。
fn reject_keys(
    action: &str,
    obj: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Result<(), ToolError> {
    let present: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|k| obj.contains_key(*k))
        .collect();
    if !present.is_empty() {
        return Err(ToolError::ExecutionFailed(format!(
            "browser_control failed [browser_control_invalid_arguments]: \
             argument(s) {} are not allowed for action={action}",
            present.join(", ")
        )));
    }
    Ok(())
}

/// 0bs ⑪：非输入动作的输入参数拒绝（navigate/search/历史/等待/快照）。
fn reject_input_params(action: &str, obj: &serde_json::Map<String, serde_json::Value>) -> Result<(), ToolError> {
    reject_keys(
        action,
        obj,
        &["text", "submit", "key", "selector", "x", "y", "dx", "dy"],
    )
}

/// 0bs ⑪：`script` 动作输出截断——超过 [`CONTROL_SCRIPT_MAX_OUTPUT_CHARS`]
/// 字符即截断并附机械标记（输出进对话正文，必须有界；截断是显式的，绝不
/// 静默）。截断标记与 `browser_read` 的 footer 同族形态。
pub(crate) fn bounded_script_output(text: &str) -> String {
    let total = text.chars().count();
    if total <= CONTROL_SCRIPT_MAX_OUTPUT_CHARS {
        return text.to_string();
    }
    let head: String = text
        .chars()
        .take(CONTROL_SCRIPT_MAX_OUTPUT_CHARS)
        .collect();
    format!(
        "{head}\n[script output truncated: {CONTROL_SCRIPT_MAX_OUTPUT_CHARS} of {total} chars shown]"
    )
}

/// S2-R P3 / P1-2b（2026-09-09, P1 设计 §2.2/§3.2-1）：把 CDP 层收集的
/// 日志行（已按严重度过滤）折叠成有界 `[browser_log]` 容器——≤12 行 /
/// ≤2 KiB，超限机械截断标记；每行带引用语域前缀，页面文本不得以原声进
/// 对话正文。
pub(crate) fn bounded_browser_log(lines: &[String]) -> String {
    let kept: Vec<&String> = lines.iter().take(BROWSER_LOG_MAX_LINES).collect();
    let mut truncated = lines.len() > BROWSER_LOG_MAX_LINES;
    let mut out = String::from("[browser_log]");
    for line in kept.iter() {
        let redacted = redact_browser_log_line(line);
        let piece = format!("\n  {redacted}");
        if out.len() + piece.len() > BROWSER_LOG_MAX_BYTES {
            truncated = true;
            break;
        }
        out.push_str(&piece);
    }
    if truncated {
        out.push_str(&format!(
            "\n  [truncated: >{} lines or >{} bytes]",
            BROWSER_LOG_MAX_LINES, BROWSER_LOG_MAX_BYTES
        ));
    }
    out
}

/// 日志行保守脱敏（凭据 pattern 值掩码；页面文本不经本函数进正文）。
pub(crate) fn redact_browser_log_line(line: &str) -> String {
    const MARKERS: &[&str] = &[
        "sk-",
        "Bearer ",
        "api_key=",
        "apikey=",
        "token=",
        "password=",
        "passwd=",
        "secret=",
        "authorization=",
    ];
    let mut out = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let mut marker_found = None;
        for marker in MARKERS {
            if bytes[i..].starts_with(marker.as_bytes()) {
                marker_found = Some(marker);
                break;
            }
        }
        if let Some(marker) = marker_found {
            out.push_str(marker);
            i += marker.len();
            // 掩码到下一个空白/引号/逗号（保守，不吞结构性字符）。
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'"'
                && bytes[i] != b'\''
                && bytes[i] != b','
            {
                i += 1;
            }
            out.push_str("***REDACTED***");
        } else {
            let ch = line[i..].chars().next().expect("char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

/// Nearest char boundary at or before `idx` (never splits a UTF-8 char).
fn floor_char_boundary(text: &str, mut idx: usize) -> usize {
    if idx >= text.len() {
        return text.len();
    }
    while idx > 0 && !text.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

/// Nearest char boundary at or after `idx` (never splits a UTF-8 char).
fn ceil_char_boundary(text: &str, mut idx: usize) -> usize {
    let len = text.len();
    if idx >= len {
        return len;
    }
    while idx < len && !text.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

/// Mechanical keyword-excerpt extraction (P0-B step 4): case-insensitive
/// substring matching for pure-ASCII text+terms, exact substring matching
/// otherwise; overlapping/adjacent matches merge into one excerpt; each term
/// contributes at most [`KEYWORD_EXCERPTS_PER_TERM`] merged spans and the
/// returned excerpt body is STRICTLY bounded by [`KEYWORD_TOTAL_CHARS`]
/// characters — separators and ellipsis markers count against the budget
/// (review fix 2026-08-14). Returns the joined excerpts, the number of
/// terms actually represented in the returned output, and the excerpt count.
fn keyword_excerpts(text: &str, keywords: &[String]) -> (String, usize, usize) {
    // Per-keyword merged spans (up to KEYWORD_EXCERPTS_PER_TERM each) —
    // kept for emitted-term attribution after the budget clip.
    let mut keyword_spans: Vec<Vec<(usize, usize)>> = Vec::with_capacity(keywords.len());
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for kw in keywords {
        let matches: Vec<(usize, usize)> = if text.is_ascii() && kw.is_ascii() {
            let lower_text = text.to_ascii_lowercase();
            let lower_kw = kw.to_ascii_lowercase();
            lower_text
                .match_indices(&lower_kw)
                .map(|(i, m)| (i, i + m.len()))
                .collect()
        } else {
            text.match_indices(kw.as_str())
                .map(|(i, m)| (i, i + m.len()))
                .collect()
        };
        // Merge overlapping/adjacent matches within one term.
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (s, e) in matches {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 {
                    last.1 = last.1.max(e);
                } else {
                    merged.push((s, e));
                }
            } else {
                merged.push((s, e));
            }
        }
        let kept: Vec<(usize, usize)> =
            merged.into_iter().take(KEYWORD_EXCERPTS_PER_TERM).collect();
        keyword_spans.push(kept.clone());
        spans.extend(kept);
    }
    // Global merge across terms (overlapping excerpts collapse).
    spans.sort_unstable();
    let mut merged_spans: Vec<(usize, usize)> = Vec::new();
    for (s, e) in spans {
        if let Some(last) = merged_spans.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
            } else {
                merged_spans.push((s, e));
            }
        } else {
            merged_spans.push((s, e));
        }
    }

    let mut out = String::new();
    let mut total_chars = 0usize;
    let mut excerpt_count = 0usize;
    let mut emitted_spans: Vec<(usize, usize)> = Vec::new();
    for (s, e) in merged_spans {
        if total_chars >= KEYWORD_TOTAL_CHARS {
            break;
        }
        let start = floor_char_boundary(text, s.saturating_sub(KEYWORD_EXCERPT_RADIUS));
        let end = ceil_char_boundary(text, (e + KEYWORD_EXCERPT_RADIUS).min(text.len()));
        let excerpt: String = text[start..end].chars().collect();
        // The whole piece (separator + ellipsis markers + excerpt) counts
        // against the strict budget — no uncounted characters (review fix
        // 2026-08-14).
        let mut piece = String::new();
        if !out.is_empty() {
            piece.push_str("\n---\n");
        }
        if start > 0 {
            piece.push('…');
        }
        piece.push_str(&excerpt);
        if end < text.len() {
            piece.push('…');
        }
        let piece_chars = piece.chars().count();
        let remaining = KEYWORD_TOTAL_CHARS.saturating_sub(total_chars);
        if piece_chars > remaining {
            // The separator must fit cleanly before we fill the tail; a
            // dangling separator-only tail is not an excerpt.
            let separator = if out.is_empty() { 0 } else { 5 };
            if remaining <= separator {
                break;
            }
            out.push_str(&piece.chars().take(remaining).collect::<String>());
            emitted_spans.push((s, e));
            excerpt_count += 1;
            break;
        }
        out.push_str(&piece);
        total_chars += piece_chars;
        emitted_spans.push((s, e));
        excerpt_count += 1;
    }

    // Terms actually represented in the emitted output (after the budget
    // clip) — a matched term whose spans were all dropped by the budget
    // does not appear in the footer count (review fix 2026-08-14).
    let emitted_terms = keyword_spans
        .iter()
        .filter(|term| {
            term.iter()
                .any(|(s1, e1)| emitted_spans.iter().any(|(s2, e2)| s1 <= e2 && s2 <= e1))
        })
        .count();
    (out, emitted_terms, excerpt_count)
}

/// Whether `browser_read` should be declared/run (same source of truth as
/// the capability probe).
pub fn browser_ready(browser: &dyn BrowserSession) -> bool {
    browser.ready()
}

/// The capability probe's launch step (ADR-0010 §3.7.1): discover a browser
/// binary, start a headless instance on the session's isolated profile, and
/// wait for a reachable devtools endpoint. Failures return a `Degraded`
/// reason string with a subdivided cause — never a silent fallback.
pub async fn probe_launch(
    workspace: &std::path::Path,
    session_id: &str,
) -> Result<LocalBrowserManager, String> {
    let binary = find_browser(
        std::env::var(discovery::ORZ_BROWSER_PATH_ENV)
            .ok()
            .as_deref(),
    )
    .map_err(|e| format!("browser_not_found: {e}"))?;
    tracing::info!(
        browser = %binary.path.display(),
        origin = ?binary.origin,
        "local_browser: launching headless browser"
    );
    let profile_dir = LocalBrowserManager::profile_dir_for(workspace, session_id);
    // Hold the profile lock across the launch (review M4): a detached
    // teardown of the previous session on this profile must finish first.
    let profile = profile_lock(&profile_dir);
    let _profile_guard = profile.lock().await;
    let session = CdpBrowserSession::launch(binary.path, profile_dir, CdpConfig::default())
        .await
        .map_err(|e| e.to_string())?;
    Ok(LocalBrowserManager::new(session))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Env gate for the live-browser e2e (mirrors the Python LBR e2e
    /// `GSA_RUN_LIVE_BROWSER_TESTS` precedent): the test spawns a REAL
    /// headless browser on the machine — never in CI.
    const LIVE_BROWSER_ENV: &str = "GSA_RUN_LIVE_BROWSER_TESTS";

    /// local_browser (2026-08-10): end-to-end against a real headless
    /// Chrome/Edge — discovery → launch → DevToolsActivePort → read a public
    /// page → rendered text → teardown (process tree + profile dir). Env-
    /// gated; requires network access to example.com.
    #[tokio::test]
    #[ignore = "live browser e2e — GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored local_browser_e2e"]
    async fn local_browser_e2e() {
        if std::env::var_os(LIVE_BROWSER_ENV).is_none() {
            return; // env-gated; the #[ignore] marker is the primary switch
        }
        let workspace =
            std::env::temp_dir().join(format!("orz-browser-e2e-{:?}", std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(&workspace).unwrap();

        let session = CdpBrowserSession::launch(
            find_browser(None)
                .expect("a Chrome/Edge binary must be discoverable")
                .path,
            LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-TEST"),
            CdpConfig {
                load_timeout: std::time::Duration::from_secs(30),
                total_budget: std::time::Duration::from_secs(60),
                tab_pool_size: 4,
                dns_ttl: std::time::Duration::from_secs(300),
            },
        )
        .await
        .expect("headless browser must launch");
        assert!(
            session.is_alive(),
            "browser process must be alive after launch"
        );
        let outcome = session
            .read_page("https://example.com/", ReadMode::Full)
            .await
            .expect("example.com must be readable");
        assert!(outcome.text.contains("Example Domain"), "{}", outcome.text);
        assert_eq!(outcome.final_url, "https://example.com/");
        assert!(!outcome.title.is_empty());

        // Teardown: kill the tree + delete the profile dir.
        let profile = LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-TEST");
        assert!(profile.exists(), "profile dir must have been created");
        session.shutdown().await;
        assert!(!profile.exists(), "profile dir must be cleaned up");
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// The URL gate must reject a private/metadata URL before the browser
    /// ever navigates — even in a live session, and even when the private
    /// address is hidden behind a DNS name (the DNS layer of the gate, not
    /// just IP literals: `*.nip.io` resolves `169.254.169.254.nip.io` to the
    /// cloud-metadata address).
    #[tokio::test]
    #[ignore = "live browser e2e — GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored local_browser_e2e"]
    async fn local_browser_e2e_gate_blocks_metadata_url() {
        if std::env::var_os(LIVE_BROWSER_ENV).is_none() {
            return;
        }
        let workspace =
            std::env::temp_dir().join(format!("orz-browser-e2e-{:?}", std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(&workspace).unwrap();

        let session = CdpBrowserSession::launch(
            find_browser(None)
                .expect("a Chrome/Edge binary must be discoverable")
                .path,
            LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-GATE"),
            CdpConfig::default(),
        )
        .await
        .expect("headless browser must launch");
        // DNS-resolving metadata hostname — the sync layer alone would pass
        // this; the DNS layer must reject it before navigation. Either a
        // private-address block (DNS resolved — the intended case) or a
        // fail-closed DNS failure proves the async gate engaged; both block
        // the navigation. (A literal-IP PrivateAddress case is covered
        // deterministically by the unit test `full_check_rejects_private_...`.)
        let err = session
            .read_page(
                "http://169.254.169.254.nip.io/latest/meta-data/",
                ReadMode::Full,
            )
            .await
            .unwrap_err();
        assert!(
            matches!(
                err,
                CdpError::UrlGate(UrlGateError::PrivateAddress { .. })
                    | CdpError::UrlGate(UrlGateError::DnsFailure { .. })
            ),
            "{err:?}"
        );
        session.shutdown().await;
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// S2-R P3 / P1-2b（2026-09-09, P1 设计 §2.1–§2.3）：browser_control
    /// 真协议 e2e——导航/快照/历史后退前进/刷新/wait_load/日志特征 + URL
    /// gate 拦截形态 + 同 URL 免重复导航（read_page 命中控制 tab）。Env-
    /// gated（真实 Chrome + 网络），同 local_browser e2e 先例。
    #[tokio::test]
    #[ignore = "live browser e2e — GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored browser_control_live_e2e"]
    async fn browser_control_live_e2e() {
        if std::env::var_os(LIVE_BROWSER_ENV).is_none() {
            return;
        }
        let workspace = std::env::temp_dir().join(format!(
            "orz-browser-control-e2e-{:?}",
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(&workspace).unwrap();
        let session = CdpBrowserSession::launch(
            find_browser(None)
                .expect("a Chrome/Edge binary must be discoverable")
                .path,
            LocalBrowserManager::profile_dir_for(&workspace, "RUN-CTRL-E2E"),
            CdpConfig::default(),
        )
        .await
        .expect("headless browser must launch");

        // navigate → completed + 落点识别（重定向后的 canonical URL）。
        let out = session
            .control(
                BrowserControlAction::Navigate {
                    url: "https://example.com/".to_string(),
                },
                Duration::from_secs(30),
            )
            .await
            .expect("control navigate");
        assert_eq!(out.action_status, "ok", "{out:?}");
        assert_eq!(out.nav_phase, "completed", "{out:?}");
        assert_eq!(out.url.as_deref(), Some("https://example.com/"), "{out:?}");
        assert!(out.title.as_deref().is_some_and(|t| !t.is_empty()));

        // 同 URL 免重复导航：read_page 命中控制 tab（不新建导航）→ 内容
        // 返回且控制 tab 状态不变。
        let read = session
            .read_page("https://example.com/", ReadMode::Full)
            .await
            .expect("same-url read via control tab");
        assert!(read.text.contains("Example Domain"), "{}", read.text);

        // 第二次导航 → 历史可后退/前进。
        let out = session
            .control(
                BrowserControlAction::Navigate {
                    url: "https://example.com/?second=1".to_string(),
                },
                Duration::from_secs(30),
            )
            .await
            .expect("second navigate");
        assert_eq!(out.action_status, "ok", "{out:?}");

        let back = session
            .control(BrowserControlAction::Back, Duration::from_secs(30))
            .await
            .expect("back");
        assert_eq!(back.action_status, "ok", "{back:?}");
        assert_eq!(
            back.url.as_deref(),
            Some("https://example.com/"),
            "{back:?}"
        );

        let fwd = session
            .control(BrowserControlAction::Forward, Duration::from_secs(30))
            .await
            .expect("forward");
        assert_eq!(fwd.action_status, "ok", "{fwd:?}");
        assert_eq!(
            fwd.url.as_deref(),
            Some("https://example.com/?second=1"),
            "{fwd:?}"
        );

        // refresh + snapshot。
        let refresh = session
            .control(BrowserControlAction::Refresh, Duration::from_secs(30))
            .await
            .expect("refresh");
        assert_eq!(refresh.action_status, "ok", "{refresh:?}");
        let snap = session
            .control(BrowserControlAction::Snapshot, Duration::from_secs(30))
            .await
            .expect("snapshot");
        assert_eq!(snap.action_status, "ok", "{snap:?}");
        assert!(snap.url.as_deref().is_some());

        // wait_load（已加载页面 → 文本就绪即返回）。
        let wait = session
            .control(BrowserControlAction::WaitLoad, Duration::from_secs(30))
            .await
            .expect("wait_load");
        assert_eq!(wait.action_status, "ok", "{wait:?}");

        // URL gate 拦截 = blocked 状态化失败（fail-closed，非导航尝试）。
        let blocked = session
            .control(
                BrowserControlAction::Navigate {
                    url: "http://169.254.169.254/latest/meta-data/".to_string(),
                },
                Duration::from_secs(30),
            )
            .await
            .expect("gate failure is a stateful outcome");
        assert_eq!(blocked.action_status, "error", "{blocked:?}");
        assert_eq!(
            blocked.error_class.as_deref(),
            Some("blocked"),
            "{blocked:?}"
        );

        // 不可解析域名 → gate 前置 DNS 预检 fail-closed 拦截（blocked +
        // 真实 gate 文本；导航层 dns 类别逻辑由 classify_log_class 单测
        // 覆盖——gate 放行后的 Chrome 层 DNS 失败在沙箱内不可稳定构造）。
        let dns_gate = session
            .control(
                BrowserControlAction::Navigate {
                    url: "http://nonexistent-0t-browser-lane.invalid/".to_string(),
                },
                Duration::from_secs(30),
            )
            .await
            .expect("dns failure is a stateful outcome");
        assert_eq!(dns_gate.action_status, "error", "{dns_gate:?}");
        assert_eq!(
            dns_gate.error_class.as_deref(),
            Some("blocked"),
            "{dns_gate:?}"
        );
        assert!(
            dns_gate.log.contains("DNS resolution failed"),
            "{dns_gate:?}"
        );

        session.shutdown().await;
        let _ = std::fs::remove_dir_all(&workspace);
    }

    /// PDF evidence (2026-08-11 review C1-1): real-Chrome download e2e —
    /// download a public test PDF through the CDP channel, assert the file
    /// lands in the staging dir with %PDF magic, and that the final URL
    /// comes from the download events (not about:blank). This is the live
    /// check the browser download channel needs: the scripted ws tests
    /// cannot catch a silently-suppressed event family (P1-1) or a lost
    /// source_url (P1-2).
    #[tokio::test]
    #[ignore = "live browser e2e — GSA_RUN_LIVE_BROWSER_TESTS=1 cargo test -p orz-host -- --ignored local_browser_e2e_pdf_download"]
    async fn local_browser_e2e_pdf_download() {
        if std::env::var_os(LIVE_BROWSER_ENV).is_none() {
            return; // env-gated; the #[ignore] marker is the primary switch
        }
        let workspace = std::env::temp_dir().join(format!(
            "orz-browser-e2e-pdf-{:?}",
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(&workspace).unwrap();

        let session = CdpBrowserSession::launch(
            find_browser(None)
                .expect("a Chrome/Edge binary must be discoverable")
                .path,
            LocalBrowserManager::profile_dir_for(&workspace, "RUN-E2E-PDF"),
            CdpConfig::default(),
        )
        .await
        .expect("headless browser must launch");

        let staging = workspace.join("staging");
        let url = "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf";
        let outcome = session
            .download_or_read(url, &staging)
            .await
            .expect("real Chrome must download the PDF");
        match outcome {
            BrowserDownloadOutcome::Pdf { path, final_url } => {
                assert!(path.exists(), "downloaded file must exist");
                let bytes = std::fs::read(&path).unwrap();
                assert!(
                    bytes.starts_with(b"%PDF-"),
                    "downloaded bytes must be a real PDF (magic)"
                );
                assert!(
                    final_url.contains("dummy.pdf"),
                    "final_url must be the downloaded resource URL, got: {final_url}"
                );
                assert_ne!(
                    final_url, "about:blank",
                    "P1-2: source_url must not degrade"
                );
            }
            other => panic!("expected a PDF download, got {other:?}"),
        }

        session.shutdown().await;
        let _ = std::fs::remove_dir_all(&workspace);
    }

    struct StubBrowser {
        outcome: Result<PageReadOutcome, CdpError>,
        download: Result<BrowserDownloadOutcome, CdpError>,
    }

    #[async_trait]
    impl BrowserSession for StubBrowser {
        async fn read_page(
            &self,
            _url: &str,
            _mode: ReadMode,
        ) -> Result<PageReadOutcome, CdpError> {
            self.outcome.clone()
        }

        async fn control(
            &self,
            _action: BrowserControlAction,
            _timeout: Duration,
        ) -> Result<BrowserControlOutcome, CdpError> {
            // 只读 stub：control 默认返回空闲快照（具体动作语义由
            // ControlScriptedBrowser / CDP live 路径覆盖）。
            Ok(BrowserControlOutcome {
                action_status: "ok".to_string(),
                error_class: None,
                nav_phase: "idle".to_string(),
                url: None,
                title: None,
                log: String::new(),
                ..Default::default()
            })
        }

        async fn download_or_read(
            &self,
            _url: &str,
            _download_dir: &Path,
        ) -> Result<BrowserDownloadOutcome, CdpError> {
            self.download.clone()
        }

        fn ready(&self) -> bool {
            true
        }

        async fn shutdown(&self) {}
    }

    fn sample_outcome(text: &str) -> PageReadOutcome {
        PageReadOutcome {
            final_url: "https://example.com/page".to_string(),
            title: "Example".to_string(),
            text: text.to_string(),
        }
    }

    /// Cross-module test helper (crate tests inject a ready browser lane).
    pub(crate) fn ready_stub_browser() -> SharedBrowser {
        Arc::new(StubBrowser {
            outcome: Ok(sample_outcome("hello page")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("hello page"))),
        })
    }

    /// S2-R P3 / P1-2b：可脚本化 `control` 结果的 stub——browser_control
    /// 信封与特征回传的确定性测试面（CDP 真协议行为由 env-gated live
    /// e2e 覆盖，见 `control_actions_live_e2e` 先例边界）。
    pub(crate) struct ControlScriptedBrowser {
        outcome: Result<BrowserControlOutcome, CdpError>,
    }

    #[async_trait]
    impl BrowserSession for ControlScriptedBrowser {
        async fn read_page(
            &self,
            _url: &str,
            _mode: ReadMode,
        ) -> Result<PageReadOutcome, CdpError> {
            unreachable!("control tests never read pages")
        }
        async fn control(
            &self,
            _action: BrowserControlAction,
            _timeout: Duration,
        ) -> Result<BrowserControlOutcome, CdpError> {
            self.outcome.clone()
        }
        async fn download_or_read(
            &self,
            _url: &str,
            _download_dir: &Path,
        ) -> Result<BrowserDownloadOutcome, CdpError> {
            unreachable!("control tests never download")
        }
        fn ready(&self) -> bool {
            true
        }
        async fn shutdown(&self) {}
    }

    fn sample_control_outcome(
        status: &str,
        error_class: Option<&str>,
        url: Option<&str>,
        log_lines: Vec<String>,
    ) -> BrowserControlOutcome {
        BrowserControlOutcome {
            action_status: status.to_string(),
            error_class: error_class.map(str::to_string),
            nav_phase: if status == "ok" {
                "completed".to_string()
            } else {
                "error".to_string()
            },
            url: url.map(str::to_string),
            title: Some("Example".to_string()),
            log: bounded_browser_log(&log_lines),
            ..Default::default()
        }
    }

    /// S2-R P3 / P1-2b（P1 设计 §2.2）：browser_control 每动作信封——成功
    /// 与状态化失败都以 Ok 回传，字段齐全（action_status/error_class/
    /// nav_phase/url/title/log），日志只进工具结果文本。
    #[tokio::test]
    async fn browser_control_envelope_carries_action_state_and_log() {
        let log_lines = vec![
            "network: net::ERR_CONNECTION_RESET".to_string(),
            "console error: fetch failed".to_string(),
        ];
        let ok_browser = ControlScriptedBrowser {
            outcome: Ok(sample_control_outcome(
                "ok",
                None,
                Some("https://example.com/redirected"),
                log_lines.clone(),
            )),
        };
        let result = handle_browser_control(
            &ok_browser,
            &json!({"action": "navigate", "url": "https://example.com"}),
        )
        .await
        .unwrap();
        assert_eq!(result.exit_code, Some(0));
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["action"], "navigate");
        assert_eq!(parsed["action_status"], "ok");
        assert!(parsed["error_class"].is_null());
        assert_eq!(parsed["nav_phase"], "completed");
        assert_eq!(parsed["url"], "https://example.com/redirected");
        assert_eq!(parsed["title"], "Example");
        assert!(parsed["log"].as_str().unwrap().contains("[browser_log]"));
        assert!(
            parsed["log"]
                .as_str()
                .unwrap()
                .contains("ERR_CONNECTION_RESET"),
            "log feature rides the tool result: {}",
            parsed["log"]
        );

        // 状态化失败（error_class 真实类别）同样 Ok 回传（FP-2 无教学句）。
        let err_browser = ControlScriptedBrowser {
            outcome: Ok(sample_control_outcome(
                "error",
                Some("connection_reset"),
                Some("https://example.com"),
                log_lines,
            )),
        };
        let result = handle_browser_control(
            &err_browser,
            &json!({"action": "navigate", "url": "https://example.com"}),
        )
        .await
        .unwrap();
        assert_eq!(result.exit_code, Some(0));
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["action_status"], "error");
        assert_eq!(parsed["error_class"], "connection_reset");
        assert_eq!(parsed["nav_phase"], "error");
    }

    /// P0-0v：search 信封不携带 navigate 的 url/title，携带 engine/results；
    /// low_quality 结果保留（不硬过滤），且带 tier/weight/reason 标注。
    #[tokio::test]
    async fn browser_control_search_envelope_carries_weighted_results() {
        let browser = ControlScriptedBrowser {
            outcome: Ok(BrowserControlOutcome {
                action_status: "ok".to_string(),
                error_class: None,
                nav_phase: "completed".to_string(),
                url: Some("https://example.com/should-not-appear".to_string()),
                title: Some("should-not-appear".to_string()),
                log: String::new(),
                engine: Some("bing".to_string()),
                engine_attempts: Some(vec![
                    SerpEngineAttempt {
                        engine: super::serp::SerpEngine::Google,
                        status: "failed",
                        error_class: Some("network".to_string()),
                        reason: Some("timeout".to_string()),
                        wall_ms: Some(1200),
                    },
                    SerpEngineAttempt {
                        engine: super::serp::SerpEngine::Bing,
                        status: "ok",
                        error_class: None,
                        reason: None,
                        wall_ms: Some(30),
                    },
                ]),
                results: Some(vec![SerpResult {
                    title: "low quality".to_string(),
                    url: "https://blog.csdn.net/a".to_string(),
                    snippet: "still visible".to_string(),
                    tier: "low_quality",
                    weight: 0.7,
                    reason: "low_quality_platform:csdn.net".to_string(),
                }]),
                ..Default::default()
            }),
        };
        let result = handle_browser_control(
            &browser,
            &json!({"action": "search", "query": "rust borrow checker"}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["action"], "search");
        assert_eq!(parsed["action_status"], "ok");
        assert_eq!(parsed["engine"], "bing");
        assert_eq!(parsed["results"][0]["tier"], "low_quality");
        assert_eq!(parsed["results"][0]["weight"], 0.7);
        assert_eq!(parsed["engine_attempts"][0]["engine"], "google");
        assert_eq!(parsed["engine_attempts"][0]["status"], "failed");
        assert_eq!(parsed["engine_attempts"][0]["wall_ms"], 1200);
        assert_eq!(parsed["engine_attempts"][1]["engine"], "bing");
        assert_eq!(parsed["engine_attempts"][1]["wall_ms"], 30);
        assert!(parsed.get("url").is_none(), "{parsed}");
        assert!(parsed.get("title").is_none(), "{parsed}");
    }

    /// P2-1：search 失败信封必须标注引擎——失败时模型要能分辨「是哪个
    /// 引擎、以何类别失败」，而不是只看到一个笼统的 all_engines_failed。
    #[tokio::test]
    async fn browser_control_search_failure_names_the_engine() {
        let browser = ControlScriptedBrowser {
            outcome: Ok(BrowserControlOutcome {
                action_status: "error".to_string(),
                error_class: Some("all_engines_failed".to_string()),
                nav_phase: "error".to_string(),
                log: "[browser_log]\n  error: all three SERP engines were attempted and failed"
                    .to_string(),
                engine_attempts: Some(vec![
                    SerpEngineAttempt {
                        engine: super::serp::SerpEngine::Google,
                        status: "failed",
                        error_class: Some("network".to_string()),
                        reason: Some("timeout".to_string()),
                        wall_ms: Some(1200),
                    },
                    SerpEngineAttempt {
                        engine: super::serp::SerpEngine::Bing,
                        status: "failed",
                        error_class: Some("captcha".to_string()),
                        reason: Some("search bing hit CAPTCHA/consent".to_string()),
                        wall_ms: Some(45),
                    },
                ]),
                ..Default::default()
            }),
        };
        let result =
            handle_browser_control(&browser, &json!({"action": "search", "query": "rust"}))
                .await
                .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["action_status"], "error");
        assert_eq!(parsed["error_class"], "all_engines_failed");
        assert_eq!(parsed["engine_attempts"][0]["engine"], "google");
        assert_eq!(parsed["engine_attempts"][1]["engine"], "bing");
        assert_eq!(parsed["engine_attempts"][1]["error_class"], "captcha");
    }

    /// S2-R P3 / P1-2b：严格参数解析——缺 action/url、非 navigate 带
    /// url、非法 timeout 均显式拒绝（稳定码前缀，无静默容错）。
    #[tokio::test]
    async fn browser_control_validation_rejects_bad_arguments() {
        let browser = ControlScriptedBrowser {
            outcome: Ok(sample_control_outcome("ok", None, None, vec![])),
        };
        let cases = [
            (json!({}), "browser_control_missing_action"),
            (json!({"action": "navigate"}), "browser_control_missing_url"),
            (
                json!({"action": "navigate", "url": 42}),
                "browser_control_invalid_arguments",
            ),
            (
                json!({"action": "back", "url": "https://example.com"}),
                "browser_control_invalid_arguments",
            ),
            (
                json!({"action": "snapshot", "timeout_secs": 99}),
                "browser_control_invalid_arguments",
            ),
            (json!({"action": "search"}), "browser_control_missing_query"),
            (
                json!({"action": "search", "query": "rust", "url": "https://example.com"}),
                "browser_control_invalid_arguments",
            ),
            // 0bs ⑪（2026-09-26）：输入动作的专属错误码与参数纪律。
            (json!({"action": "type"}), "browser_control_missing_text"),
            (
                json!({"action": "click"}),
                "browser_control_invalid_arguments",
            ),
            (
                json!({"action": "click", "selector": "a", "x": 1, "y": 2}),
                "browser_control_invalid_arguments",
            ),
            (
                json!({"action": "scroll", "dx": 0, "dy": 0}),
                "browser_control_invalid_arguments",
            ),
            (
                json!({"action": "key", "key": ""}),
                "browser_control_missing_key",
            ),
            (
                json!({"action": "navigate", "url": "https://example.com", "text": "x"}),
                "browser_control_invalid_arguments",
            ),
            (
                json!({"action": "scroll", "dy": 100000}),
                "browser_control_invalid_arguments",
            ),
            (
                json!({"action": "navigate", "url": "https://example.com", "extra": 1}),
                "browser_control_invalid_arguments",
            ),
        ];
        for (args, code) in cases {
            let err = handle_browser_control(&browser, &args).await.unwrap_err();
            assert!(
                err.to_string().contains(code),
                "case {args}: expected {code}, got {err}"
            );
        }
    }

    /// S2-R P3 / P1-2b（P1 设计 §2.2 边界）：日志节选 ≤12 行 / ≤2 KiB，
    /// 超出机械截断标记；凭据 pattern 保守脱敏。
    #[test]
    fn bounded_browser_log_caps_and_redacts() {
        let lines: Vec<String> = (0..20)
            .map(|i| format!("console error: line {i}"))
            .collect();
        let log = bounded_browser_log(&lines);
        assert!(log.contains("[browser_log]"));
        assert!(log.contains("line 0"));
        assert!(!log.contains("line 15"), "over-cap lines dropped");
        assert!(log.contains("[truncated:"));
        assert!(log.lines().count() <= BROWSER_LOG_MAX_LINES + 2);
        assert!(
            log.len() <= BROWSER_LOG_MAX_BYTES + 64,
            "byte cap with truncation marker"
        );

        let secret = "network: https://user:pass@example.com?token=sk-abc1234567890def";
        let redacted = redact_browser_log_line(secret);
        assert!(!redacted.contains("sk-abc1234567890def"), "{redacted}");
        assert!(redacted.contains("***REDACTED***"), "{redacted}");
    }

    /// S2-R P3 / P1-2b：工具定义形状——单工具多动作。0bs ⑪ 起动作面为
    /// 完整 13 动作（读/导航 7 + 输入 4 + 唯二门禁 2）；url 仅在
    /// navigate/download 场景语义存在（schema 无法表达条件必需，由执行层
    /// 严格解析兜底）。
    #[test]
    fn browser_control_tool_def_shape() {
        let def = browser_control_tool_def();
        assert_eq!(def.name, "browser_control");
        let props = &def.parameters["properties"];
        let actions: Vec<&str> = props["action"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            actions,
            vec![
                "navigate",
                "back",
                "forward",
                "refresh",
                "wait_load",
                "snapshot",
                "search",
                "type",
                "key",
                "click",
                "scroll",
                "download",
                "script"
            ]
        );
        assert!(props["url"].is_object());
        assert!(props["query"].is_object());
        assert_eq!(props["query"]["maxLength"], 500);
        assert_eq!(def.parameters["required"][0], "action");
        // 0bs ⑪：输入与门禁动作的参数面必须随动作一起出现在 schema 上
        // （b 轮遗留缺口：enum 与属性未随动作更新）。
        assert_eq!(props["text"]["maxLength"], CONTROL_TYPE_MAX_CHARS);
        assert!(props["submit"].is_object());
        assert_eq!(props["key"]["maxLength"], CONTROL_KEY_MAX_CHARS);
        assert_eq!(props["selector"]["maxLength"], CONTROL_SELECTOR_MAX_CHARS);
        assert_eq!(props["x"]["maximum"], CONTROL_COORD_MAX);
        assert_eq!(props["y"]["maximum"], CONTROL_COORD_MAX);
        assert_eq!(props["dx"]["minimum"], -CONTROL_SCROLL_MAX_PX);
        assert_eq!(props["dy"]["minimum"], -CONTROL_SCROLL_MAX_PX);
        assert_eq!(props["script"]["maxLength"], CONTROL_SCRIPT_MAX_CHARS);
    }

    /// 0bs ⑪：script 输出截断是显式的机械标记（不是静默截断）。
    #[test]
    fn script_output_is_bounded_with_explicit_marker() {
        let short = "42";
        assert_eq!(bounded_script_output(short), "42");
        let long: String = std::iter::repeat_n('x', CONTROL_SCRIPT_MAX_OUTPUT_CHARS + 10).collect();
        let bounded = bounded_script_output(&long);
        assert!(
            bounded.contains("[script output truncated:"),
            "truncation must be visible: {bounded}"
        );
        assert!(
            bounded.starts_with(&"x".repeat(CONTROL_SCRIPT_MAX_OUTPUT_CHARS)),
            "head must be the first N chars"
        );
    }

    /// 0bs ⑪：唯二门禁动作的解析纪律——download 要 url、script 要非空
    /// script、两者拒绝彼此与输入参数、超限拒绝、catch-all 文案含新动作。
    #[tokio::test]
    async fn download_and_script_actions_parse_strictly() {
        async fn err_of(args: serde_json::Value) -> String {
            let browser = StubBrowser {
                outcome: Ok(sample_outcome("x")),
                download: Ok(BrowserDownloadOutcome::Page(sample_outcome("x"))),
            };
            handle_browser_control(&browser, &args)
                .await
                .err()
                .expect("must reject")
                .to_string()
        }
        // download 缺 url。
        let e = err_of(json!({"action": "download"})).await;
        assert!(e.contains("browser_control_missing_url"), "{e}");
        // download 带输入参数。
        let e = err_of(json!({"action": "download", "url": "https://example.com/a", "text": "x"}))
            .await;
        assert!(e.contains("browser_control_invalid_arguments"), "{e}");
        // script 缺 script。
        let e = err_of(json!({"action": "script"})).await;
        assert!(e.contains("browser_control_missing_script"), "{e}");
        // script 带 url（url 只在 navigate/download 上）。
        let e = err_of(json!({"action": "script", "script": "1+1", "url": "https://example.com/"}))
            .await;
        assert!(e.contains("browser_control_invalid_arguments"), "{e}");
        // script 超限。
        let long = "x".repeat(CONTROL_SCRIPT_MAX_CHARS + 1);
        let e = err_of(json!({"action": "script", "script": long})).await;
        assert!(e.contains("browser_control_invalid_arguments"), "{e}");
        // catch-all 文案覆盖新动作（未知动作）。
        let e = err_of(json!({"action": "eval"})).await;
        assert!(e.contains("download"), "{e}");
        assert!(e.contains("script"), "{e}");
    }

    #[tokio::test]
    async fn read_success_wraps_text_with_meta() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("hello world")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("hello world"))),
        };
        let result = handle_browser_read(&browser, &json!({"url": "https://example.com/"}))
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["title"], "Example");
        assert_eq!(parsed["content"], "hello world");
        assert_eq!(parsed["truncated"], false);
    }

    #[tokio::test]
    async fn read_truncates_with_mechanical_footer() {
        let long = "x".repeat(MAX_READ_CHARS + 500);
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(&long)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(&long))),
        };
        let result = handle_browser_read(&browser, &json!({"url": "https://example.com/"}))
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["truncated"], true);
        assert!(
            parsed["content"]
                .as_str()
                .unwrap()
                .contains(TRUNCATED_FOOTER_PREFIX),
            "truncated output must carry the mechanical footer"
        );
        assert!(
            parsed["content"].as_str().unwrap().chars().count() < MAX_READ_CHARS + 200,
            "output must stay near the cap"
        );
    }

    /// P0-B step 4 (2026-08-14): preview mode returns the first
    /// `PREVIEW_READ_CHARS` chars with a mechanical footer + `truncated:
    /// true` when the page is longer than the preview budget; a short page
    /// returns complete (no footer, `truncated: false`).
    #[tokio::test]
    async fn preview_mode_truncates_long_pages_and_keeps_short_pages() {
        let long = "x".repeat(PREVIEW_READ_CHARS + 500);
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(&long)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(&long))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "preview"}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["mode"], "preview");
        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["preview_chars"], PREVIEW_READ_CHARS);
        assert!(
            parsed["content"]
                .as_str()
                .unwrap()
                .contains(TRUNCATED_FOOTER_PREFIX),
            "long preview must carry the mechanical footer"
        );

        let short = "short page text";
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(short)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(short))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "preview"}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["truncated"], false);
        assert_eq!(parsed["content"], short);
        assert!(
            !parsed["content"]
                .as_str()
                .unwrap()
                .contains(TRUNCATED_FOOTER_PREFIX),
            "short preview must stay footer-free (complete content)"
        );
    }

    /// P0-B step 4 (2026-08-14): keywords mode returns bounded excerpts
    /// around matches (case-insensitive for ASCII), always `truncated: true`
    /// with the mechanical footer, and echoes the requested keywords.
    #[tokio::test]
    async fn keywords_mode_extracts_bounded_excerpts() {
        let page = "The quick brown fox jumps over the lazy dog. \
                    The QUICK fox is fast.";
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(page)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(page))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["quick"]}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["mode"], "keywords");
        assert_eq!(parsed["truncated"], true);
        assert_eq!(parsed["keywords"][0], "quick");
        let content = parsed["content"].as_str().unwrap();
        assert!(content.contains("quick brown fox"), "{content}");
        assert!(content.contains("QUICK fox"), "{content}");
        assert!(
            content.contains(TRUNCATED_FOOTER_PREFIX),
            "keywords output must carry the mechanical footer"
        );
        assert!(content.contains("keyword excerpts"), "{content}");
    }

    /// P0-B step 4 (2026-08-14): keywords mode without any match returns a
    /// neutral no-match note (still a successful read, still partial).
    #[tokio::test]
    async fn keywords_mode_no_match_returns_neutral_note() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("page without the term")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(
                "page without the term",
            ))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["absent"]}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["truncated"], true);
        assert!(
            parsed["content"]
                .as_str()
                .unwrap()
                .contains("no matching excerpts"),
            "{parsed}"
        );
    }

    /// Review fix (2026-08-14): the excerpt BODY is strictly bounded by
    /// KEYWORD_TOTAL_CHARS — separators and ellipsis markers count against
    /// the budget, so the model-visible body never exceeds it even with
    /// many matches across all 16 terms.
    #[tokio::test]
    async fn keywords_mode_output_stays_within_total_chars_budget() {
        let mut page = String::new();
        for _block in 0..20 {
            for term in 0..16 {
                page.push_str(&format!("needle{term} "));
                page.push_str(&"x".repeat(200));
            }
        }
        let keywords: Vec<String> = (0..16).map(|i| format!("needle{i}")).collect();
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(&page)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(&page))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({
                "url": "https://example.com/",
                "mode": "keywords",
                "keywords": keywords,
            }),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["truncated"], true);
        let content = parsed["content"].as_str().unwrap();
        let body = content.split(TRUNCATED_FOOTER_PREFIX).next().unwrap();
        let body_chars = body.chars().count();
        assert!(
            body_chars <= KEYWORD_TOTAL_CHARS,
            "excerpt body exceeds the strict budget: {body_chars} > {KEYWORD_TOTAL_CHARS}"
        );
    }

    /// Review fix (2026-08-14): keywords extraction caps its INPUT at
    /// MAX_READ_CHARS like full mode — a keyword that only appears past
    /// the cap is out of scope, reported via the no-match note and the
    /// input-cap footer.
    #[tokio::test]
    async fn keywords_mode_caps_input_before_extraction() {
        let mut page = String::new();
        page.push_str(&"x".repeat(MAX_READ_CHARS + 100));
        page.push_str("needle");
        let browser = StubBrowser {
            outcome: Ok(sample_outcome(&page)),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome(&page))),
        };
        let result = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["needle"]}),
        )
        .await
        .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        let content = parsed["content"].as_str().unwrap();
        assert!(
            content.contains("no matching excerpts"),
            "match past the input cap must be out of scope: {content}"
        );
        assert!(content.contains("input capped"), "{content}");
        assert_eq!(parsed["truncated"], true);
    }

    /// P0-B step 4 (2026-08-14): strict argument validation for the new
    /// mode/keywords surface — invalid mode, keywords outside keywords mode,
    /// empty/duplicate/oversized terms and non-array keywords all carry the
    /// stable `[browser_read_invalid_arguments]` code.
    #[tokio::test]
    async fn mode_and_keywords_arguments_are_strictly_validated() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("x")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("x"))),
        };
        let cases = [
            json!({"url": "https://example.com/", "mode": "summary"}),
            json!({"url": "https://example.com/", "keywords": ["a"]}),
            json!({"url": "https://example.com/", "mode": "keywords"}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": []}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["a", ""]}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": ["a", "a"]}),
            json!({"url": "https://example.com/", "mode": "keywords", "keywords": "a"}),
            json!({
                "url": "https://example.com/",
                "mode": "keywords",
                "keywords": ["x".repeat(MAX_KEYWORD_CHARS + 1)],
            }),
            json!({
                "url": "https://example.com/",
                "mode": "keywords",
                "keywords": (0..=MAX_KEYWORDS).map(|i| format!("k{i}")).collect::<Vec<_>>(),
            }),
        ];
        for args in cases {
            let err = handle_browser_read(&browser, &args).await.unwrap_err();
            assert!(
                err.to_string().contains("[browser_read_invalid_arguments]"),
                "args {args}: {err}"
            );
        }
    }

    /// P0-B step 4 (2026-08-14): the tool definition declares the mode
    /// enum and the keywords array so the model can select read scope.
    #[test]
    fn tool_def_declares_scope_modes() {
        let def = browser_read_tool_def();
        assert_eq!(def.parameters["properties"]["mode"]["enum"][0], "full");
        assert_eq!(def.parameters["properties"]["mode"]["enum"][1], "preview");
        assert_eq!(def.parameters["properties"]["mode"]["enum"][2], "keywords");
        assert_eq!(
            def.parameters["properties"]["keywords"]["maxItems"],
            MAX_KEYWORDS
        );
        assert_eq!(
            def.parameters["properties"]["keywords"]["items"]["maxLength"],
            MAX_KEYWORD_CHARS
        );
        assert_eq!(def.parameters["required"][0], "url");
    }

    #[tokio::test]
    async fn url_gate_error_maps_to_stable_code() {
        let browser = StubBrowser {
            outcome: Err(CdpError::UrlGate(UrlGateError::PrivateAddress {
                host: "10.0.0.1".into(),
            })),
            download: Err(CdpError::UrlGate(UrlGateError::PrivateAddress {
                host: "10.0.0.1".into(),
            })),
        };
        let err = handle_browser_read(&browser, &json!({"url": "http://10.0.0.1/"}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("refused"), "{err}");
        // The stable code must be findable in the error text.
        let code = UrlGateError::PrivateAddress { host: "x".into() }.tool_error_code();
        assert!(!code.is_empty());
    }

    #[tokio::test]
    async fn empty_content_is_explicit_error_not_success() {
        let browser = StubBrowser {
            outcome: Err(CdpError::EmptyContent),
            download: Err(CdpError::EmptyContent),
        };
        let err = handle_browser_read(&browser, &json!({"url": "https://example.com/"}))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no readable text"), "{err}");
    }

    #[tokio::test]
    async fn missing_url_is_invalid_arguments() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("x")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("x"))),
        };
        let err = handle_browser_read(&browser, &json!({})).await.unwrap_err();
        assert!(err.to_string().contains("`url`"), "{err}");
        // Every argument failure carries a stable error code.
        assert!(
            err.to_string().contains("[browser_read_missing_url]"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn unknown_arguments_are_rejected() {
        let browser = StubBrowser {
            outcome: Ok(sample_outcome("x")),
            download: Ok(BrowserDownloadOutcome::Page(sample_outcome("x"))),
        };
        let err = handle_browser_read(
            &browser,
            &json!({"url": "https://example.com/", "timeout": 5}),
        )
        .await
        .unwrap_err();
        assert!(err.to_string().contains("timeout"), "{err}");
        assert!(
            err.to_string().contains("[browser_read_invalid_arguments]"),
            "{err}"
        );
        let err = handle_browser_read(&browser, &json!(42)).await.unwrap_err();
        assert!(
            err.to_string().contains("[browser_read_invalid_arguments]"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn unavailable_session_fails_closed() {
        let browser = UnavailableBrowserSession::new("browser_launch_failed: test".into());
        assert!(!browser.ready());
        let err = browser
            .read_page("https://example.com/", ReadMode::Full)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("browser unavailable"), "{err}");
    }

    /// P2-3（2026-09-10）：无浏览器会话不报 SERP 事实——loop 因此不施加
    /// "检索保留额度"规则（浏览器缺席的调用按普通失败回传，不由额度面兜）。
    #[tokio::test]
    async fn unavailable_session_reports_no_serp_facts() {
        let browser = UnavailableBrowserSession::new("browser_launch_failed: test".into());
        assert_eq!(browser.serp_session_navigations().await, None);
    }

    /// Windows 盘符路径语义（Linux 上 `C:\` 是相对路径，行为不同）。
    #[cfg(windows)]
    #[test]
    fn profile_dir_layout_is_gsa_scoped() {
        let dir =
            LocalBrowserManager::profile_dir_for(std::path::Path::new(r"C:\work"), "RUN12345678");
        assert!(dir.starts_with(r"C:\work\.gsa"));
        // session_id.chars().take(8) — "RUN12345678" → "RUN12345"
        assert!(
            dir.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("chrome-profile-RUN12345")
        );
    }
}
