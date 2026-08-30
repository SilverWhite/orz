//! CDP (Chrome DevTools Protocol) client for the `local_browser` lane
//! (ADR-0010 §3.7.3/§3.7.6).
//!
//! Minimal read-only capability set:
//! - Only [`ALLOWED_CDP_METHODS`] may ever be sent (enforced at `send` time,
//!   fail-closed, plus a test locks the set).
//! - `Runtime.evaluate` only ever runs host-owned fixed expressions (title /
//!   innerText / location.href) — the model cannot inject JavaScript.
//! - Tabs live only for the duration of one `read_page` call (created and
//!   closed inside it): the session never hands a tab handle to the model
//!   (§3.7.6 tab ownership).
//! - Cookies, passwords, localStorage and other tabs never leave the
//!   browser; only the rendered text + title + final URL are returned.
//!
//! The browser is launched with `--remote-debugging-port=0` (ephemeral port)
//! and the actual port is read from `DevToolsActivePort` in the profile dir —
//! no port races. The debug endpoint binds 127.0.0.1 only.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use super::BrowserDownloadOutcome;
use super::url_gate::{UrlGateError, check_navigation_url};

/// Every CDP method this client may send. Anything else fails closed at the
/// send boundary (a test asserts the set is exactly what `read_page` and
/// `download_or_read` need).
///
/// `Browser.setDownloadBehavior` (2026-08-11, PDF evidence pipeline) is the
/// single download affordance: it points the browser's default context at an
/// isolated staging dir. It is NOT a `Network.`/`Input.`/`Storage.` method —
/// no network interception, no input synthesis, no storage access. Download
/// *events* (`Page.downloadWillBegin`/`Page.downloadProgress`) are received,
/// not sent, so they need no entry here.
pub const ALLOWED_CDP_METHODS: &[&str] = &[
    "Target.createTarget",
    "Target.closeTarget",
    "Page.enable",
    "Page.navigate",
    "Browser.setDownloadBehavior",
    // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30, TODO P0-0k 第一批
    // 第 4 项)：preview/keywords 模式资源拦截——只阻断图片/字体/媒体等
    // 与文本读取无关的请求，不读写任何用户数据（网络域只发
    // setBlockedURLs 列表，主机自有固定参数）。
    "Network.enable",
    "Network.setBlockedURLs",
    "Runtime.enable",
    "Runtime.evaluate",
];

/// Host-owned, fixed evaluation expressions. The model never supplies an
/// expression — `Runtime.evaluate` is only reachable through these.
const EXPR_TITLE: &str = "document.title";
const EXPR_TEXT: &str = "document.body ? document.body.innerText : ''";
const EXPR_FINAL_URL: &str = "window.location.href";

/// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：preview/keywords
/// 模式拦截的资源后缀（Chrome URL pattern，`*` 通配、匹配完整 URL）。
/// 样式表保留——阻断 CSS 会破坏布局导致 innerText 缺内容；三方脚本不拦
/// （JS 渲染是 browser_read 相对静态直连的核心价值，全拦会伤可用性）。
/// 0k 审查处理 (P3)：尾随 `*` 让带 query string 的资源（`a.png?v=3`）
/// 也命中——纯后缀模式（`*.png`）要求 URL 以 `.png` 结尾，query 逃逸。
const BLOCKED_URL_PATTERNS: &[&str] = &[
    "*.png*", "*.jpg*", "*.jpeg*", "*.gif*", "*.webp*", "*.avif*", "*.svg*", "*.woff*", "*.woff2*",
    "*.ttf*", "*.otf*", "*.eot*", "*.mp4*", "*.webm*", "*.mp3*", "*.ogg*", "*.wav*",
];

/// Default timeouts (2026-08-10; ADR-0010 §3.7.2 explicitly does not freeze
/// old-document timeouts, these are implementation choices).
const DEFAULT_LOAD_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_TOTAL_BUDGET: Duration = Duration::from_secs(60);
/// Discovery-endpoint HTTP bound (loopback devtools, sub-second in health).
const HTTP_JSON_TIMEOUT: Duration = Duration::from_secs(5);

/// Page-read timing knobs (injectable for tests).
#[derive(Debug, Clone)]
pub struct CdpConfig {
    pub load_timeout: Duration,
    pub total_budget: Duration,
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：会话级
    /// tab 池大小（有界 CDP target 常驻，租约独占、归还复用、LRU 选空闲；
    /// 0 = 关闭池，回退每调用 create/close 现状）。默认 4，
    /// `ORZ_BROWSER_TAB_POOL_SIZE` 可配。
    pub tab_pool_size: usize,
    /// DNS 预检会话级缓存 TTL（按 host；redirect 重检门保留）。默认 300s，
    /// `ORZ_BROWSER_DNS_TTL_SECS` 可配。
    pub dns_ttl: Duration,
}

impl Default for CdpConfig {
    fn default() -> Self {
        let tab_pool_size = std::env::var("ORZ_BROWSER_TAB_POOL_SIZE")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(4);
        let dns_ttl_secs = std::env::var("ORZ_BROWSER_DNS_TTL_SECS")
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(300);
        Self {
            load_timeout: DEFAULT_LOAD_TIMEOUT,
            total_budget: DEFAULT_TOTAL_BUDGET,
            tab_pool_size,
            dns_ttl: Duration::from_secs(dns_ttl_secs),
        }
    }
}

/// The read-only outcome of one `browser_read` call.
#[derive(Debug, Clone)]
pub struct PageReadOutcome {
    /// Final URL after navigation (redirects followed; verified by the gate).
    pub final_url: String,
    pub title: String,
    /// Rendered visible text (innerText) — never raw HTML, never JS results.
    pub text: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CdpError {
    #[error("browser process failed to start: {0}")]
    Spawn(String),

    #[error("devtools port file {path} not written within {timeout}s")]
    DevToolsActivePortTimeout { path: String, timeout: u64 },

    #[error("CDP discovery endpoint failed: {0}")]
    Discovery(String),

    #[error("websocket error: {0}")]
    Ws(String),

    #[error("CDP command {method} failed: {message}")]
    Command { method: String, message: String },

    #[error("CDP method {0} is not in the allowed set")]
    DisallowedMethod(String),

    #[error("timed out waiting for page load ({timeout}s)")]
    LoadTimeout { timeout: u64 },

    #[error("timed out after {timeout}s (total page-read budget)")]
    TotalTimeout { timeout: u64 },

    #[error("navigation blocked by URL policy: {0}")]
    UrlGate(#[from] UrlGateError),

    #[error("page produced no readable text (empty innerText)")]
    EmptyContent,

    #[error("browser download was canceled")]
    DownloadCanceled,

    #[error("browser download failed: {0}")]
    DownloadFailed(String),

    #[error("{0}")]
    Io(String),
}

type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Map of in-flight CDP command ids to their response oneshots.
type PendingResponses = HashMap<u32, oneshot::Sender<Result<Value, CdpError>>>;

/// One CDP websocket connection: a write half, a reader task that routes
/// responses to pending command oneshots and events to the event channel.
struct WsSession {
    write: futures::stream::SplitSink<WsStream, WsMessage>,
    events: mpsc::Receiver<Value>,
    pending: Arc<Mutex<PendingResponses>>,
    next_id: u32,
    /// Debug label for error messages ("browser ws" / "page ws").
    label: &'static str,
}

impl WsSession {
    async fn connect(url: &str, label: &'static str) -> Result<Self, CdpError> {
        let (ws, _) = tokio_tungstenite::connect_async(url)
            .await
            .map_err(|e| CdpError::Ws(format!("{label}: {e}")))?;
        let (write, read) = ws.split();
        // 256: bursty multi-frame pages emit dozens of events per navigation.
        // Overflow policy lives in `ws_reader_task` — `loadEventFired` is
        // never dropped; other events are best-effort under a flood.
        let (event_tx, event_rx) = mpsc::channel(256);
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let reader_pending = pending.clone();
        tokio::spawn(ws_reader_task(read, reader_pending, event_tx));
        Ok(Self {
            write,
            events: event_rx,
            pending,
            next_id: 1,
            label,
        })
    }

    /// Send one command and await its response (events keep flowing to the
    /// event channel in the background). One command in flight per ws.
    async fn send_command(&mut self, method: &str, params: Value) -> Result<Value, CdpError> {
        if !ALLOWED_CDP_METHODS.contains(&method) {
            return Err(CdpError::DisallowedMethod(method.to_string()));
        }
        let id = self.next_id;
        self.next_id += 1;
        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap();
            pending.insert(id, tx);
        }
        let msg = json!({ "id": id, "method": method, "params": params });
        self.write
            .send(WsMessage::Text(msg.to_string().into()))
            .await
            .map_err(|e| CdpError::Ws(format!("{}: send: {e}", self.label)))?;
        // The sender channel carries Result<Value, CdpError> — one more layer.
        let resp = rx
            .await
            .map_err(|_| CdpError::Ws(format!("{}: response channel closed", self.label)))?;
        let resp = resp?;
        if let Some(err) = resp.get("error") {
            return Err(CdpError::Command {
                method: method.to_string(),
                message: err["message"]
                    .as_str()
                    .unwrap_or("unknown CDP error")
                    .to_string(),
            });
        }
        Ok(resp["result"].clone())
    }

    /// Run one fixed host expression and return its string value.
    async fn evaluate_string(&mut self, expression: &str) -> Result<String, CdpError> {
        let result = self
            .send_command(
                "Runtime.evaluate",
                json!({ "expression": expression, "returnByValue": true }),
            )
            .await?;
        // An evaluate exception (host constants must never throw) is an
        // explicit failure, not an empty result — §3.7.2, no silent fallback.
        if let Some(exc) = result.get("exceptionDetails").and_then(|e| e.as_object()) {
            let text = exc
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("evaluate exception");
            let desc = exc
                .get("exception")
                .and_then(|e| e.get("description"))
                .and_then(|d| d.as_str());
            return Err(CdpError::Command {
                method: "Runtime.evaluate".to_string(),
                message: desc.unwrap_or(text).to_string(),
            });
        }
        Ok(result["result"]["value"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_default())
    }
}

/// Reader task: routes `{id}` responses to pending oneshots, `{method}`
/// events to the event channel. Response routing must never block on the
/// event channel — a full event queue would stall in-flight commands.
async fn ws_reader_task(
    mut read: futures::stream::SplitStream<WsStream>,
    pending: Arc<Mutex<PendingResponses>>,
    event_tx: mpsc::Sender<Value>,
) {
    while let Some(msg) = read.next().await {
        let Ok(WsMessage::Text(text)) = msg else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        if let Some(id) = v.get("id").and_then(|i| i.as_u64()) {
            if let Some(tx) = pending.lock().unwrap().remove(&(id as u32)) {
                let _ = tx.send(Ok(v));
            }
        } else if let Some(method) = v.get("method").and_then(|m| m.as_str()) {
            // Only events the client consumes are worth queueing; drop the
            // rest without touching the channel.
            //
            // `loadEventFired` is the terminal signal of `wait_for_load` and
            // must never be lost — its delivery is awaited. Bounded: the
            // event loop consumes it promptly and no commands are in flight
            // while it waits (the only later commands run after load), so
            // the await cannot stall response routing in practice.
            //
            // All other events are best-effort: under a flood the reader
            // must keep routing responses, so a full channel drops them. A
            // dropped `frameNavigated` only skips one redirect re-check —
            // the pre-navigation gate and the post-load final-URL gate still
            // bound the landing page. A dropped `downloadProgress{completed}`
            // surfaces as an explicit download timeout (never a silent
            // success) — the download loop re-polls the staging dir for a
            // bounded window, so a single lost event is self-healing there.
            if method == "Page.loadEventFired" {
                let _ = event_tx.send(v).await;
            } else if method == "Page.frameNavigated"
                || method == "Page.downloadWillBegin"
                || method == "Page.downloadProgress"
                // PDF evidence (2026-08-11 review P1-1): newer Chrome
                // versions mark the `Page.*` download events deprecated and
                // gate them behind `eventsEnabled: true` on
                // `Browser.setDownloadBehavior`. The `Browser.*` variants
                // are the current protocol surface — forward BOTH families
                // so the download loop works across Chrome generations.
                || method == "Browser.downloadWillBegin"
                || method == "Browser.downloadProgress"
            {
                let _ = event_tx.try_send(v);
            }
        }
    }
}

/// The local_browser lane's browser session: one headless Chrome process on
/// an isolated profile, one lazily-opened browser ws, per-call page tabs.
///
/// v2 (RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批, 2026-08-30)：会话级
/// tab 池——有界 `tab_pool_size` 个 CDP target 常驻（create 一次、navigate
/// 复用、LRU 选空闲、租约独占、归还复用），替代「每调用 create+close」；
/// 同轮多个 `browser_read` 真正并发（manager 不再整读持锁）。隔离语义：
/// 每次租约仍是完整 navigate + 读 + 归还，模型永不接触 tab 句柄；同一
/// session 单 profile 的 cookie/localStorage 跨调用共享是既有事实，池化
/// 不引入新的状态泄漏维度。`tab_pool_size=0` 回退每调用 create/close。
pub struct CdpBrowserSession {
    port: u16,
    /// Browser-level ws endpoint (from `/json/version`); page ws endpoints
    /// are built from the port + target id.
    browser_ws_url: String,
    profile_dir: PathBuf,
    config: CdpConfig,
    /// 可变 session 状态（browser ws + 子进程）——tokio Mutex：仅在
    /// create_target/shutdown 短持有，绝不跨 std Mutex await。
    inner: tokio::sync::Mutex<SessionInner>,
    /// v2：tab 池——target 列表 + 忙碌标记。std Mutex 只做「检查/取用/
    /// 归还」的短临界区（无 await 在其内）。
    pool: Mutex<Vec<PooledTab>>,
    /// v2：池上限信号量（permits = tab_pool_size；池满等待计入总预算）。
    pool_sem: tokio::sync::Semaphore,
    /// v2：创建串行化——保证池有界 ≤ pool_size（tokio Mutex，可跨 await）。
    creation_lock: tokio::sync::Mutex<()>,
    /// v2：DNS 预检结果缓存（按 host + TTL；只缓存成功结果）。
    dns: Mutex<HashMap<String, DnsCacheEntry>>,
}

#[derive(Default)]
struct SessionInner {
    browser_ws: Option<WsSession>,
    child: Option<tokio::process::Child>,
}

/// v2：池中的一个 CDP target。`busy=true` 期间被一次读取独占（租约）；
/// 归还后 `page_ws` 保留复用（读取失败则丢弃，下次租约重连）。
struct PooledTab {
    target_id: String,
    page_ws: Option<WsSession>,
    busy: bool,
    last_used: std::time::Instant,
}

/// v2：DNS 预检缓存条目（只缓存成功结果；失败不缓存、下次重试）。
#[derive(Debug, Clone)]
struct DnsCacheEntry {
    at: std::time::Instant,
}

impl CdpBrowserSession {
    /// Launch a headless browser on an isolated profile and wait for its
    /// devtools port (fail-closed: anything short of a reachable debug
    /// endpoint is an error — ADR-0010 §3.7.2, no silent fallback).
    pub async fn launch(
        binary: PathBuf,
        profile_dir: PathBuf,
        config: CdpConfig,
    ) -> Result<Self, CdpError> {
        let mut cmd = tokio::process::Command::new(&binary);
        // Headed by default (user ruling 2026-08-10 — the operator can log
        // in through the visible window); ORZ_BROWSER_HEADLESS=1 forces
        // headless for display-less environments.
        let headless = std::env::var_os(super::discovery::ORZ_BROWSER_HEADLESS_ENV).is_some();
        cmd.args(super::discovery::browser_launch_args(
            &profile_dir,
            headless,
        ));
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        let child = cmd
            .spawn()
            .map_err(|e| CdpError::Spawn(format!("{}: {e}", binary.display())))?;

        let port_file = profile_dir.join("DevToolsActivePort");
        let port = poll_devtools_active_port(&port_file, DEFAULT_LOAD_TIMEOUT.as_secs()).await?;

        // The probe verifies the endpoint actually answers (not just the
        // port file). The browser ws itself is opened lazily per first call.
        let version = http_get_json(port, "/json/version").await?;
        let ws_url = version["webSocketDebuggerUrl"]
            .as_str()
            .ok_or_else(|| CdpError::Discovery("no webSocketDebuggerUrl in /json/version".into()))?
            .to_string();

        let pool_size = config.tab_pool_size;
        Ok(Self {
            port,
            browser_ws_url: ws_url,
            profile_dir,
            config,
            inner: tokio::sync::Mutex::new(SessionInner {
                browser_ws: None,
                child: Some(child),
            }),
            pool: Mutex::new(Vec::new()),
            pool_sem: tokio::sync::Semaphore::new(pool_size.max(1)),
            creation_lock: tokio::sync::Mutex::new(()),
            dns: Mutex::new(HashMap::new()),
        })
    }

    /// Whether the process is still alive (probe/lifecycle use).
    ///
    /// `try_wait` (not a pid check) so a reaped/exited process reports dead
    /// immediately — `ready()` relies on this for probe self-healing when
    /// the browser was killed out-of-band. tokio caches the reap status, so
    /// the later `wait()` in `shutdown`/`kill_process_tree` still returns it.
    pub fn is_alive(&self) -> bool {
        match self.inner.try_lock() {
            Ok(mut inner) => match inner.child.as_mut() {
                Some(child) => child.try_wait().map(|s| s.is_none()).unwrap_or(false),
                None => false,
            },
            // 锁忙（create_target/shutdown 进行中）→ 保守视为存活。
            Err(_) => true,
        }
    }

    /// One read of one URL: gate (shape + DNS, 会话级缓存) → 租约 tab →
    /// navigate → wait (full: loadEventFired 终态；preview/keywords：
    /// 「可用文本就绪」+ 资源拦截) → final-URL gate → extract rendered text
    /// → 归还 tab。整体受 [`CdpConfig::total_budget`] 约束。
    ///
    /// v2：`tab_pool_size > 0` 走池化租约（并发读各自独占 tab、归还复用）；
    /// `0` 回退每调用 create/close（legacy）。
    pub async fn read_page(
        &self,
        url: &str,
        mode: super::ReadMode,
    ) -> Result<PageReadOutcome, CdpError> {
        // Pre-navigation gate: URL shape + DNS resolution（fail-closed——
        // 私有/metadata 地址在浏览器导航前拒绝，ADR-0010 §3.7.3；DNS 结果
        // 会话级缓存，redirect 重检门保留）。
        self.check_navigation_url_cached(url).await?;
        if self.config.tab_pool_size == 0 {
            self.read_page_legacy(url, mode).await
        } else {
            self.read_page_pooled(url, mode).await
        }
    }

    /// v2：池化读取——信号量（池上限）→ 租约（独占 busy）→ 读取 → 归还
    /// （busy=false + 保留/丢弃 page ws）。创建经 `creation_lock` 串行化，
    /// 保证池有界 ≤ tab_pool_size。
    async fn read_page_pooled(
        &self,
        url: &str,
        mode: super::ReadMode,
    ) -> Result<PageReadOutcome, CdpError> {
        let total_budget = self.config.total_budget;
        let deadline = tokio::time::Instant::now() + total_budget;
        let budget_err = CdpError::TotalTimeout {
            timeout: total_budget.as_secs(),
        };
        // 池上限信号量：池满且全 busy 时等待（计入总预算，防饥饿）。
        let _permit = tokio::time::timeout_at(deadline, self.pool_sem.acquire())
            .await
            .map_err(|_| budget_err.clone())?;
        // 租约：独占一个 tab（busy=true）。审查处理（2026-08-30）：不再
        // 把 lease 错误统一映射为 TotalTimeout——create_target/浏览器 ws
        // 连接失败等真实错误原样传播（lease_tab 内部仅把创建超时映射为
        // TotalTimeout），「浏览器不可达」与「预算超时」可区分。
        let target_id = self.lease_tab(deadline).await?;
        let page_ws_url = format!("ws://127.0.0.1:{}/devtools/page/{target_id}", self.port);
        // 取出（若有）既存 page ws（归还时可复用连接；读取失败则丢弃，
        // 下次租约重连）。
        let mut page = self.pool_take_ws(&target_id);
        let read_result = tokio::time::timeout_at(deadline, async {
            if page.is_none() {
                page = Some(WsSession::connect(&page_ws_url, "page ws").await?);
            }
            read_in_page(page.as_mut().unwrap(), url, self.config.load_timeout, mode).await
        })
        .await;
        // 归还：成功保留 page ws 复用；失败/超时丢弃（断开连接不进入池）。
        let returned_ws = match &read_result {
            Ok(_) => page,
            Err(_) => None,
        };
        self.release_tab(&target_id, returned_ws);
        read_result.map_err(|_| budget_err)?
    }

    /// v2：legacy 读取（`ORZ_BROWSER_TAB_POOL_SIZE=0`）——每调用
    /// create/close（既有语义；teardown every path 固定 5s cap）。
    async fn read_page_legacy(
        &self,
        url: &str,
        mode: super::ReadMode,
    ) -> Result<PageReadOutcome, CdpError> {
        let total_budget = self.config.total_budget;
        let deadline = tokio::time::Instant::now() + total_budget;
        let budget_err = CdpError::TotalTimeout {
            timeout: total_budget.as_secs(),
        };
        let target_id = tokio::time::timeout_at(deadline, self.create_target())
            .await
            .map_err(|_| budget_err.clone())??;
        let page_ws_url = format!("ws://127.0.0.1:{}/devtools/page/{target_id}", self.port);
        let outcome = tokio::time::timeout_at(deadline, async {
            let mut page = WsSession::connect(&page_ws_url, "page ws").await?;
            read_in_page(&mut page, url, self.config.load_timeout, mode).await
        })
        .await;
        // Tab teardown on EVERY path（成功/错误/超时）——固定 5s cap。
        let _ = tokio::time::timeout(Duration::from_secs(5), self.close_target(&target_id)).await;
        outcome.map_err(|_| budget_err)?
    }

    /// v2：DNS 预检——形状检查每次执行（无 IO）；DNS/SSRF 结果按 host
    /// 会话级缓存（TTL 内成功结果直接复用；失败不缓存、下次重试；
    /// redirect 重检走未缓存路径）。审查处理（2026-08-30）：登记 TTL 内
    /// 重绑定取舍——同一 host 在 TTL 内从公开解析变为私有/metadata 地址
    /// 时，初始导航预检门被跳过（浏览器仍会自行解析）；redirect 重检门
    /// 不受影响（ADR-0010 §3.7.3 主门保留）。收敛方案（缓存解析 IP 集合
    /// + 变化重检）不在本批，登记为已知边界。
    async fn check_navigation_url_cached(&self, raw: &str) -> Result<(), CdpError> {
        let url =
            url::Url::parse(raw).map_err(|e| CdpError::UrlGate(UrlGateError::InvalidUrl(e)))?;
        super::check_navigation_url_sync(&url).map_err(CdpError::UrlGate)?;
        let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
        let now = std::time::Instant::now();
        let cached = self.dns.lock().unwrap().get(&host).cloned();
        if let Some(entry) = cached
            && now.duration_since(entry.at) < self.config.dns_ttl
        {
            return Ok(());
        }
        super::check_navigation_url(raw)
            .await
            .map_err(CdpError::UrlGate)?;
        self.dns.lock().unwrap().insert(
            host,
            DnsCacheEntry {
                at: std::time::Instant::now(),
            },
        );
        Ok(())
    }

    /// v2：创建 CDP target（browser ws 惰性连接；tokio Mutex 短持有，
    /// 可跨 await）。
    async fn create_target(&self) -> Result<String, CdpError> {
        let mut inner = self.inner.lock().await;
        if inner.browser_ws.is_none() {
            inner.browser_ws = Some(WsSession::connect(&self.browser_ws_url, "browser ws").await?);
        }
        let browser = inner.browser_ws.as_mut().unwrap();
        let resp = browser
            .send_command("Target.createTarget", json!({ "url": "about:blank" }))
            .await?;
        Ok(resp["targetId"]
            .as_str()
            .ok_or_else(|| CdpError::Discovery("no targetId from Target.createTarget".into()))?
            .to_string())
    }

    /// v2：关闭 CDP target（best-effort，用于 legacy 路径与池 evict）。
    async fn close_target(&self, target_id: &str) -> Result<(), CdpError> {
        let mut inner = self.inner.lock().await;
        let Some(browser) = inner.browser_ws.as_mut() else {
            return Ok(());
        };
        browser
            .send_command("Target.closeTarget", json!({ "targetId": target_id }))
            .await
            .map(|_| ())
    }

    /// v2：租约——空闲 tab 直接取（LRU：最早 last_used）；全忙则经
    /// creation_lock 串行创建（池有界 ≤ pool_size）。
    async fn lease_tab(&self, deadline: tokio::time::Instant) -> Result<String, CdpError> {
        let now = std::time::Instant::now();
        let free_tab = || {
            let mut pool = self.pool.lock().unwrap();
            pool.iter()
                .enumerate()
                .filter(|(_, t)| !t.busy)
                .min_by_key(|(_, t)| t.last_used)
                .map(|(idx, _)| idx)
                .map(|idx| {
                    pool[idx].busy = true;
                    pool[idx].last_used = now;
                    pool[idx].target_id.clone()
                })
        };
        if let Some(id) = free_tab() {
            return Ok(id);
        }
        // 创建串行化（tokio Mutex，可跨 await；避免 std Mutex 跨 await
        // 阻塞单线程 runtime）。
        let _creation = self.creation_lock.lock().await;
        if let Some(id) = free_tab() {
            return Ok(id);
        }
        let target_id = tokio::time::timeout_at(deadline, self.create_target())
            .await
            .map_err(|_| CdpError::TotalTimeout {
                timeout: self.config.total_budget.as_secs(),
            })??;
        self.pool.lock().unwrap().push(PooledTab {
            target_id: target_id.clone(),
            page_ws: None,
            busy: true,
            last_used: now,
        });
        Ok(target_id)
    }

    /// v2：取走池中 tab 的 page ws（短临界区，无 await）。
    fn pool_take_ws(&self, target_id: &str) -> Option<WsSession> {
        let mut pool = self.pool.lock().unwrap();
        pool.iter_mut()
            .find(|t| t.target_id == target_id)
            .and_then(|t| t.page_ws.take())
    }

    /// v2：归还 tab（busy=false + page ws 回存；短临界区，无 await）。
    fn release_tab(&self, target_id: &str, page_ws: Option<WsSession>) {
        let mut pool = self.pool.lock().unwrap();
        if let Some(t) = pool.iter_mut().find(|t| t.target_id == target_id) {
            t.busy = false;
            t.page_ws = page_ws;
            t.last_used = std::time::Instant::now();
        }
    }

    /// One download-or-read of one URL: gate → staging-dir reset → create
    /// tab → arm download behavior → navigate → wait for either a completed
    /// download (file in the staging dir) or a rendered page. Shares the
    /// read path's tab lifecycle: one owned tab per call, teardown on every
    /// path under a fixed 5s cap, total-budget bounded (ADR-0010 §3.7.2 —
    /// a download that neither completes nor navigates is an explicit
    /// timeout, never a silent fallback).
    pub async fn download_or_read(
        &self,
        url: &str,
        download_dir: &Path,
    ) -> Result<BrowserDownloadOutcome, CdpError> {
        self.check_navigation_url_cached(url).await?;

        // Fresh staging dir per call: the "newest file in dir" semantics of
        // `wait_for_new_file` must not pick up a leftover from an earlier
        // call (or a crash before retention swept it).
        let _ = std::fs::remove_dir_all(download_dir);
        std::fs::create_dir_all(download_dir).map_err(|e| CdpError::Io(e.to_string()))?;
        // Absolute path without a UNC prefix — Chrome rejects `\\?\`-style
        // download paths (dunce precedent).
        let download_path = dunce::canonicalize(download_dir)
            .unwrap_or_else(|_| download_dir.to_path_buf())
            .to_string_lossy()
            .into_owned();

        let total_budget = self.config.total_budget;
        let deadline = tokio::time::Instant::now() + total_budget;
        let budget_err = CdpError::TotalTimeout {
            timeout: total_budget.as_secs(),
        };

        let target_id = tokio::time::timeout_at(deadline, self.create_target())
            .await
            .map_err(|_| budget_err.clone())??;

        let page_ws_url = format!("ws://127.0.0.1:{}/devtools/page/{target_id}", self.port);
        let outcome = tokio::time::timeout_at(deadline, async {
            let mut page = WsSession::connect(&page_ws_url, "page ws").await?;
            // Arm the default browser context for downloads, pointing at the
            // staging dir. Re-sent per call: idempotent, and the browser may
            // have been restarted between calls. A navigation that produces
            // a rendered page instead of a download falls through to the
            // read path.
            page.send_command(
                "Browser.setDownloadBehavior",
                // `eventsEnabled: true` (review 2026-08-11 P1-1) — without it
                // Chromium's `download_events_enabled_` flag stays false and
                // the browser-domain download events are silently suppressed
                // on modern Chrome.
                json!({ "behavior": "allow", "downloadPath": download_path, "eventsEnabled": true }),
            )
            .await?;
            page.send_command("Page.enable", json!({})).await?;
            page.send_command("Runtime.enable", json!({})).await?;
            page.send_command("Page.navigate", json!({ "url": url })).await?;
            wait_download_or_load(&mut page, url, download_dir, self.config.load_timeout).await
        })
        .await;

        // Tab teardown on EVERY path（成功/错误/超时）——固定 5s cap。
        let _ = tokio::time::timeout(Duration::from_secs(5), self.close_target(&target_id)).await;
        outcome.map_err(|_| budget_err)?
    }

    /// Kill the process tree and best-effort delete the profile dir.
    ///
    /// Profile deletion retries for ~2s: Windows releases file locks held by
    /// the just-killed Chrome tree asynchronously (observed 2026-08-10 e2e —
    /// an immediate `remove_dir_all` right after taskkill failed). Still
    /// best-effort — a leftover dir is covered by the A5 retention sweep.
    pub async fn shutdown(&self) {
        // Serialize with any concurrent launch on the same profile (review
        // M4): this teardown must not delete files a new browser is writing.
        let profile = super::profile_lock(&self.profile_dir);
        let _profile_guard = profile.lock().await;
        let mut inner = self.inner.lock().await;
        inner.browser_ws = None;
        if let Some(mut child) = inner.child.take() {
            crate::kill_process_tree(&mut child).await;
        }
        drop(inner);
        let profile = self.profile_dir.clone();
        let _ = tokio::task::spawn_blocking(move || {
            for _ in 0..10 {
                if std::fs::remove_dir_all(&profile).is_ok() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        })
        .await;
    }
}

/// One read inside an already-connected page ws: enable domains, navigate,
/// wait for load (re-checking the gate on top-frame redirects), final-URL
/// gate, then extract the three host-owned expressions. Free function (not
/// a method) because the caller holds `&mut self.browser_ws` while this runs
/// — it must not borrow `self` again.
async fn read_in_page(
    page: &mut WsSession,
    url: &str,
    load_timeout: Duration,
    mode: super::ReadMode,
) -> Result<PageReadOutcome, CdpError> {
    page.send_command("Page.enable", json!({})).await?;
    page.send_command("Runtime.enable", json!({})).await?;
    if mode != super::ReadMode::Full {
        // 资源拦截（图片/字体/媒体）——preview/keywords 只需文本，
        // 阻断无关请求缩短加载窗（样式表与三方脚本保留，见
        // BLOCKED_URL_PATTERNS 注释）。
        page.send_command("Network.enable", json!({})).await?;
        page.send_command(
            "Network.setBlockedURLs",
            json!({ "urls": BLOCKED_URL_PATTERNS }),
        )
        .await?;
    }
    page.send_command("Page.navigate", json!({ "url": url }))
        .await?;
    match mode {
        super::ReadMode::Full => wait_for_load(page, load_timeout).await?,
        super::ReadMode::Preview | super::ReadMode::Keywords => {
            wait_for_text_ready(page, load_timeout).await?
        }
    }

    extract_page(page, url).await
}

/// Extract the three host-owned expressions from a loaded page. The
/// final-URL gate runs BEFORE any text extraction — nothing is read from a
/// page whose landing location failed the policy (defense in depth: a
/// redirect chain the event stream did not surface is caught here, and a
/// gated page never has expressions evaluated against it). Shared by the
/// read path and the download-or-read path's non-download branch.
async fn extract_page(
    page: &mut WsSession,
    fallback_url: &str,
) -> Result<PageReadOutcome, CdpError> {
    let final_url = page.evaluate_string(EXPR_FINAL_URL).await?;
    let final_url = if final_url.is_empty() || final_url == "about:blank" {
        fallback_url.to_string()
    } else {
        check_navigation_url(&final_url).await?;
        final_url
    };

    let title = page.evaluate_string(EXPR_TITLE).await?;
    let text = page.evaluate_string(EXPR_TEXT).await?;

    if text.trim().is_empty() {
        return Err(CdpError::EmptyContent);
    }
    Ok(PageReadOutcome {
        final_url,
        title,
        text,
    })
}

impl Drop for CdpBrowserSession {
    fn drop(&mut self) {
        // Sync fallback when shutdown() was never awaited (task cancel etc.):
        // the child may keep running until the session object is dropped —
        // the A5 retention sweep on `chrome-profile-*` covers the dir.
        if let Ok(mut inner) = self.inner.try_lock()
            && let Some(child) = inner.child.as_mut()
        {
            let _ = child.start_kill();
        }
    }
}

/// Top-frame redirect gate (ADR-0010 §3.7.3: initial navigation AND every
/// redirect) — shared by the full-load wait and the text-ready poll.
async fn gate_top_frame_redirect(ev: &serde_json::Value) -> Result<(), CdpError> {
    if ev["method"].as_str() == Some("Page.frameNavigated") {
        let frame = &ev["params"]["frame"];
        // Top-level frame only (subframes carry page assets, not the
        // navigation target).
        if frame["parentId"].is_null()
            && let Some(new_url) = frame["url"].as_str()
        {
            // A fresh target emits an initial top-frame
            // `frameNavigated` for `about:blank` before Page.navigate
            // takes effect (some Chrome versions replay it after
            // Page.enable). It is the target's pre-navigation
            // identity, not a navigation outcome — exempt it; real
            // navigations (and the final location.href) are gated.
            if new_url != "about:blank" {
                check_navigation_url(new_url).await?;
            }
        }
    }
    Ok(())
}

/// Wait for the top-level load to finish, re-checking the full URL gate
/// (shape + DNS) on every top-frame redirect (ADR-0010 §3.7.3: initial
/// navigation AND every redirect).
async fn wait_for_load(page: &mut WsSession, timeout: Duration) -> Result<(), CdpError> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let ev = tokio::time::timeout(remaining, page.events.recv())
            .await
            .map_err(|_| CdpError::LoadTimeout {
                timeout: timeout.as_secs(),
            })?
            .ok_or_else(|| CdpError::Io("CDP event channel closed".into()))?;
        gate_top_frame_redirect(&ev).await?;
        if ev["method"].as_str() == Some("Page.loadEventFired") {
            return Ok(());
        }
    }
}

/// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：preview/keywords
/// 等待「可用文本就绪」——轮询 innerText 非空即返回，不等到
/// loadEventFired（慢三方资源不再拖长读取）；导航中的求值错误按
/// 「未就绪」处理，由 deadline 兜底为 LoadTimeout。重定向门与 full
/// 路径同源（ADR-0010 §3.7.3）。
async fn wait_for_text_ready(page: &mut WsSession, timeout: Duration) -> Result<(), CdpError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let poll = Duration::from_millis(150);
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err(CdpError::LoadTimeout {
                timeout: timeout.as_secs(),
            });
        }
        // 0k 审查处理 (P2-4)：`evaluate_string` 本身无内部超时——reader
        // 任务退出而 write 半开时，send_command 的响应 oneshot 永不完成，
        // 裸 await 会永久 pending（deadline 检查在其后永不执行）。用
        // remaining 包裹：断连场景由 deadline 兜底为 LoadTimeout，或由
        // 下方通道关闭分支显式报 Io。
        let evaluate = tokio::time::timeout(remaining, page.evaluate_string(EXPR_TEXT)).await;
        if let Ok(Ok(text)) = evaluate
            && !text.trim().is_empty()
        {
            return Ok(());
        }
        tokio::select! {
            ev = page.events.recv() => {
                if let Some(ev) = ev {
                    gate_top_frame_redirect(&ev).await?;
                } else {
                    // 0k 审查处理 (P2-4)：事件通道关闭（WS 断连、reader
                    // 任务退出）→ 立即显式报错。此前此处空转导致忙循环到
                    // deadline（最长 30s CPU 空转）；full 路径的
                    // `wait_for_load` 对通道关闭是显式 `CDP event channel
                    // closed`，两条路径保持一致。
                    return Err(CdpError::Io("CDP event channel closed".into()));
                }
            }
            _ = tokio::time::sleep(poll.min(remaining)) => {}
        }
    }
}

/// Wait for either a completed download (file in the staging dir) or a
/// rendered page, re-checking the full URL gate on every top-frame redirect.
///
/// Event semantics (both `Page.*` and the current-protocol `Browser.*`
/// families are matched — review 2026-08-11 P1-1):
/// - `downloadWillBegin` → remember the download guid AND the resource URL
///   (`params.url` — the real downloaded resource; the tab's
///   `location.href` stays `about:blank` because the download navigation is
///   canceled in the renderer). From that point on, `loadEventFired` is NOT
///   the terminal signal (a download navigation also fires a load event on
///   its empty frame) — only a completed download is.
/// - `downloadProgress {state:"completed"}` → re-poll the staging dir for
///   the new file (Chrome flushes it asynchronously; Windows file locks are
///   retried) and return `Pdf` with the gated `downloadWillBegin` URL.
/// - `{state:"canceled"}` → explicit `DownloadCanceled` (ADR-0010 §3.7.2,
///   no silent fallback to a page read).
/// - `loadEventFired` with no pending download → the ordinary read path
///   (`extract_page`), gated as usual.
/// - Polling backstop (review 2026-08-11 P2-4): a lost `completed` event
///   (best-effort channel) must not hang the call — while a download is
///   pending, the staging dir is polled every ~500ms; a file that is
///   stable across two samples counts as complete.
async fn wait_download_or_load(
    page: &mut WsSession,
    request_url: &str,
    download_dir: &Path,
    timeout: Duration,
) -> Result<BrowserDownloadOutcome, CdpError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut pending_guid: Option<String> = None;
    let mut pending_url: Option<String> = None;
    // Stable-file polling backstop: previous sample's size for the newest
    // file. `None` = no file observed yet.
    let mut last_sample: Option<u64> = None;
    loop {
        // Polling backstop first: a download already in progress (guid seen)
        // with a stable file in the staging dir is complete even if the
        // `completed` event was dropped. The dir is per-call fresh, so any
        // file is this call's download.
        if pending_guid.is_some()
            && let Some((path, size)) = newest_file(download_dir)
        {
            if last_sample == Some(size) {
                // Stable across two samples (~500ms apart) — flushed.
                let final_url = pending_url
                    .clone()
                    .unwrap_or_else(|| request_url.to_string());
                gate_download_final_url(&final_url).await?;
                return Ok(BrowserDownloadOutcome::Pdf { path, final_url });
            }
            last_sample = Some(size);
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        // Fragmented wait: the backstop poll runs even when no events arrive
        // (a flood that dropped the terminal event). A fragment timeout is
        // NOT a LoadTimeout — only the total budget is.
        let ev =
            match tokio::time::timeout(remaining.min(DOWNLOAD_POLL_INTERVAL), page.events.recv())
                .await
            {
                Ok(Some(ev)) => ev,
                Ok(None) => return Err(CdpError::Io("CDP event channel closed".into())),
                Err(_elapsed) => {
                    if tokio::time::Instant::now() >= deadline {
                        return Err(CdpError::LoadTimeout {
                            timeout: timeout.as_secs(),
                        });
                    }
                    // Fragment elapsed: back to the top of the loop — the poll
                    // backstop runs before the next wait.
                    continue;
                }
            };
        match ev["method"].as_str() {
            Some("Page.downloadWillBegin") | Some("Browser.downloadWillBegin") => {
                if let Some(guid) = ev["params"]["guid"].as_str() {
                    pending_guid = Some(guid.to_string());
                }
                if let Some(url) = ev["params"]["url"].as_str() {
                    pending_url = Some(url.to_string());
                }
            }
            Some("Page.downloadProgress") | Some("Browser.downloadProgress") => {
                match ev["params"]["state"].as_str() {
                    Some("completed") => {
                        // The file may not be flushed yet — Chrome writes it
                        // asynchronously and may briefly hold a Windows lock.
                        let path = wait_for_new_file(download_dir, DOWNLOAD_FLUSH_TIMEOUT).await?;
                        let final_url = pending_url
                            .clone()
                            .unwrap_or_else(|| request_url.to_string());
                        gate_download_final_url(&final_url).await?;
                        return Ok(BrowserDownloadOutcome::Pdf { path, final_url });
                    }
                    Some("canceled") => return Err(CdpError::DownloadCanceled),
                    _ => {} // "in_progress": keep waiting.
                }
            }
            Some("Page.frameNavigated") => {
                let frame = &ev["params"]["frame"];
                if frame["parentId"].is_null()
                    && let Some(new_url) = frame["url"].as_str()
                    && new_url != "about:blank"
                {
                    check_navigation_url(new_url).await?;
                }
            }
            // A download navigation also fires a load event; only a
            // download-less load is the terminal read signal. The real URL
            // is re-read from location.href inside extract_page.
            Some("Page.loadEventFired") if pending_guid.is_none() => {
                let outcome = extract_page(page, request_url).await?;
                return Ok(BrowserDownloadOutcome::Page(outcome));
            }
            _ => {}
        }
    }
}

/// Same final-URL gate as the read path (`extract_page`): a download's
/// landing URL must pass policy before its bytes are ingested (review
/// 2026-08-11 P1-1 — a whitelisted-but-compromised site must not redirect a
/// download to an internal address and smuggle its bytes into evidence).
async fn gate_download_final_url(final_url: &str) -> Result<(), CdpError> {
    if final_url != "about:blank" && !final_url.is_empty() {
        check_navigation_url(final_url).await?;
    }
    Ok(())
}

/// Staging-dir poll cadence for the backstop (see `wait_download_or_load`).
const DOWNLOAD_POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Poll the staging dir for the newest regular file. Windows Chrome may
/// hold the file open briefly after `downloadProgress{completed}` — the
/// poll retries until a file is visible or the window elapses.
const DOWNLOAD_FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// Newest regular file in `dir` (path + size), or `None` when empty or
/// unreadable. Shared by the completed-event flush wait and the poll
/// backstop — "newest" is exact because the staging dir is per-call fresh.
fn newest_file(dir: &Path) -> Option<(PathBuf, u64)> {
    let mut newest: Option<(std::time::SystemTime, PathBuf, u64)> = None;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for entry in rd.flatten() {
            if let Ok(meta) = entry.metadata()
                && meta.is_file()
                && let Ok(modified) = meta.modified()
                && newest.as_ref().is_none_or(|(t, _, _)| modified > *t)
            {
                newest = Some((modified, entry.path(), meta.len()));
            }
        }
    }
    newest.map(|(_, path, size)| (path, size))
}

async fn wait_for_new_file(dir: &Path, timeout: Duration) -> Result<PathBuf, CdpError> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Some((path, _)) = newest_file(dir) {
            return Ok(path);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(CdpError::DownloadFailed(
                "no file appeared in the staging dir".to_string(),
            ));
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Poll the profile's `DevToolsActivePort` file (Chrome writes
/// `<port>\n<ws-path>` there when `--remote-debugging-port=0` is used).
async fn poll_devtools_active_port(port_file: &Path, timeout_s: u64) -> Result<u16, CdpError> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_s);
    loop {
        if let Ok(content) = tokio::fs::read_to_string(port_file).await
            && let Some(port) = content
                .lines()
                .next()
                .and_then(|l| l.trim().parse::<u16>().ok())
                .filter(|p| *p != 0)
        {
            return Ok(port);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(CdpError::DevToolsActivePortTimeout {
                path: port_file.display().to_string(),
                timeout: timeout_s,
            });
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Minimal local HTTP GET over plain TCP (16KB cap) — used only for the
/// loopback devtools discovery endpoints (`/json/version`, `/json/list`).
/// orz-host intentionally stays free of a reqwest/TLS stack.
///
/// Bounded: the whole exchange runs under a 5s timeout (a stalled devtools
/// endpoint must not hang the probe/launch). Reads are Content-Length aware:
/// Chrome ignores `Connection: close` on its devtools HTTP endpoints
/// (HTTP/1.1 keep-alive — observed 2026-08-10 e2e: the response arrives in
/// one chunk and the connection then stays open, so an unbounded read loop
/// hangs on the second read); we stop exactly at the declared body length.
/// `ConnectionReset` right after a response is treated as end-of-body
/// (Windows Chrome RSTs short-lived connections, os error 10054 — the bytes
/// already received are still parsed).
pub(crate) async fn http_get_json(port: u16, path: &str) -> Result<Value, CdpError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    tokio::time::timeout(HTTP_JSON_TIMEOUT, async {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .map_err(|e| CdpError::Discovery(format!("connect: {e}")))?;
        let req =
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
        stream
            .write_all(req.as_bytes())
            .await
            .map_err(|e| CdpError::Discovery(format!("write: {e}")))?;
        let mut buf = Vec::with_capacity(4096);
        let mut chunk = [0u8; 4096];
        // Declared body length once the headers are in — `None` until then.
        let mut content_length: Option<usize> = None;
        loop {
            match stream.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => {
                    buf.extend_from_slice(&chunk[..n]);
                    if buf.len() > 16 * 1024 {
                        return Err(CdpError::Discovery("response exceeds 16KB".into()));
                    }
                    if content_length.is_none() {
                        let text = String::from_utf8_lossy(&buf);
                        if let Some(hdr_end) = text.find("\r\n\r\n") {
                            let mut lines = text[..hdr_end].lines();
                            // Only HTTP 200 is expected from the devtools
                            // endpoint — anything else is an explicit error
                            // (fail-closed, not a JSON-parse surprise).
                            let status_code = lines
                                .next()
                                .and_then(|l| l.split_whitespace().nth(1))
                                .unwrap_or("");
                            if status_code != "200" {
                                return Err(CdpError::Discovery(format!(
                                    "unexpected HTTP status {status_code:?} from {path}"
                                )));
                            }
                            // An invalid Content-Length is an explicit error —
                            // silently treating it as "absent" would truncate
                            // or over-read the body.
                            content_length = match lines.find_map(|l| {
                                let (k, v) = l.split_once(':')?;
                                k.trim()
                                    .eq_ignore_ascii_case("content-length")
                                    .then(|| v.trim())
                            }) {
                                Some(raw) => Some(raw.parse::<usize>().map_err(|_| {
                                    CdpError::Discovery(format!(
                                        "invalid Content-Length {raw:?} from {path}"
                                    ))
                                })?),
                                None => None,
                            };
                        }
                    }
                    // Stop exactly at the declared body length — Chrome keeps
                    // the connection open (keep-alive) after the response.
                    let hdr_end = String::from_utf8_lossy(&buf).find("\r\n\r\n");
                    let complete = match content_length {
                        Some(len) => hdr_end.is_some_and(|i| buf.len() - (i + 4) >= len),
                        None => hdr_end.is_some(), // no CL → one chunk is the body
                    };
                    if complete {
                        break;
                    }
                }
                // Connection reset right after the response (Windows Chrome
                // RSTs short-lived connections): the received bytes are the
                // whole body.
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => return Err(CdpError::Discovery(format!("read: {e}"))),
            }
        }
        let text = String::from_utf8_lossy(&buf);
        let body = text.split_once("\r\n\r\n").map(|(_, b)| b).unwrap_or(&text);
        serde_json::from_str(body.trim())
            .map_err(|e| CdpError::Discovery(format!("json parse: {e}")))
    })
    .await
    .map_err(|_| CdpError::Discovery(format!("http GET {path} timed out after 5s")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CdpConfig {
        CdpConfig {
            load_timeout: Duration::from_millis(500),
            total_budget: Duration::from_secs(5),
            tab_pool_size: 4,
            dns_ttl: Duration::from_secs(300),
        }
    }

    /// 测试用会话构造（无需真实浏览器进程；gate/池语义可离线断言）。
    fn test_session(config: CdpConfig) -> CdpBrowserSession {
        let pool_size = config.tab_pool_size;
        CdpBrowserSession {
            port: 1,
            browser_ws_url: "ws://127.0.0.1:1/devtools/browser/unused".to_string(),
            profile_dir: PathBuf::from("unused"),
            config,
            inner: tokio::sync::Mutex::new(SessionInner {
                browser_ws: None,
                child: None,
            }),
            pool: Mutex::new(Vec::new()),
            pool_sem: tokio::sync::Semaphore::new(pool_size.max(1)),
            creation_lock: tokio::sync::Mutex::new(()),
            dns: Mutex::new(HashMap::new()),
        }
    }

    #[test]
    fn allowed_method_set_is_exactly_the_readonly_surface() {
        let mut set = ALLOWED_CDP_METHODS.to_vec();
        set.sort_unstable();
        assert_eq!(
            set,
            vec![
                "Browser.setDownloadBehavior",
                "Network.enable",
                "Network.setBlockedURLs",
                "Page.enable",
                "Page.navigate",
                "Runtime.enable",
                "Runtime.evaluate",
                "Target.closeTarget",
                "Target.createTarget",
            ]
        );
        // The set must never include mutation/input/storage domains.
        // `Browser.` is allowed EXACTLY as the download affordance — no
        // other Browser-domain command may ever be sent. `Network.` is
        // allowed EXACTLY as the resource-blocking pair for preview/
        // keywords reads (0k 2026-08-30) — blocking-only, no request
        // interception or data access.
        for m in ALLOWED_CDP_METHODS {
            assert!(
                !m.starts_with("Input.") && !m.starts_with("Storage."),
                "{m}"
            );
            if m.starts_with("Browser.") {
                assert_eq!(*m, "Browser.setDownloadBehavior", "{m}");
            }
            if m.starts_with("Network.") {
                assert!(
                    matches!(*m, "Network.enable" | "Network.setBlockedURLs"),
                    "{m}"
                );
            }
        }
    }

    #[tokio::test]
    async fn disallowed_methods_fail_closed() {
        // Any method outside ALLOWED_CDP_METHODS must be refused at the send
        // boundary — before any bytes hit the wire (the ws pair proves the
        // gate fires without a real server round-trip).
        let (_, mut ws) = inprocess_ws_pair().await;
        for method in [
            "Network.getAllCookies",
            "Storage.getCookies",
            "Browser.getVersion",
            "Browser.close",
            "Runtime.evaluate",
        ] {
            // Runtime.evaluate is allowed BY NAME — the model-text injection
            // vector is closed at the host-owned-expression layer, not here.
            if method == "Runtime.evaluate" {
                continue;
            }
            let err = ws.send_command(method, json!({})).await;
            assert!(
                matches!(err, Err(CdpError::DisallowedMethod(_))),
                "{method}: {err:?}"
            );
        }
    }

    /// A loopback ws pair for send-command tests (no server logic needed).
    async fn inprocess_ws_pair() -> (tokio::task::JoinHandle<()>, WsSession) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws.split();
            while let Some(Ok(WsMessage::Text(_))) = read.next().await {
                let _ = write
                    .send(WsMessage::Text("{\"id\":999,\"result\":{}}".into()))
                    .await;
            }
        });
        let ws = WsSession::connect(&format!("ws://{addr}/devtools/browser/test"), "test ws")
            .await
            .unwrap();
        (server, ws)
    }

    #[tokio::test]
    async fn command_responses_route_by_id() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let port = addr.port();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws.split();
            // Script: echo the id back with a per-method marker.
            while let Some(Ok(WsMessage::Text(t))) = read.next().await {
                let v: Value = serde_json::from_str(&t).unwrap();
                let id = v["id"].as_u64().unwrap();
                let method = v["method"].as_str().unwrap().to_string();
                let resp = json!({"id": id, "result": {"method": method}});
                let _ = write.send(WsMessage::Text(resp.to_string().into())).await;
            }
        });
        let mut ws = WsSession::connect(&format!("ws://{addr}/devtools/browser/test"), "test ws")
            .await
            .unwrap();
        let r1 = ws.send_command("Page.enable", json!({})).await.unwrap();
        assert_eq!(r1["method"], "Page.enable");
        let r2 = ws
            .send_command("Page.navigate", json!({"url": "x"}))
            .await
            .unwrap();
        assert_eq!(r2["method"], "Page.navigate");
        let _ = port;
        server.abort();
    }

    #[tokio::test]
    async fn load_wait_consumes_events_and_times_out() {
        // A server that accepts a page ws and never sends loadEventFired →
        // wait_for_load must time out at the configured limit.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            // Never respond, never send events; hold the connection open.
            std::future::pending::<()>().await;
        });
        let mut ws = WsSession::connect(&format!("ws://{addr}/devtools/page/t1"), "page ws")
            .await
            .unwrap();
        let start = tokio::time::Instant::now();
        let err = wait_for_load(&mut ws, Duration::from_millis(200)).await;
        assert!(matches!(err, Err(CdpError::LoadTimeout { .. })));
        assert!(
            start.elapsed() >= Duration::from_millis(150),
            "must wait near the limit"
        );
        server.abort();
    }

    #[tokio::test]
    async fn frame_navigated_to_blocked_url_aborts_load() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut write, mut _read) = ws.split();
            // The client is waiting on events only — push a blocked
            // top-frame redirect immediately, no commands required.
            let ev = json!({
                "method": "Page.frameNavigated",
                "params": {"frame": {"id": "f1", "parentId": Value::Null, "url": "file:///etc/passwd"}}
            });
            let _ = write.send(WsMessage::Text(ev.to_string().into())).await;
            std::future::pending::<()>().await;
        });
        let mut ws = WsSession::connect(&format!("ws://{addr}/devtools/page/t1"), "page ws")
            .await
            .unwrap();
        let err = wait_for_load(&mut ws, Duration::from_secs(2)).await;
        assert!(matches!(
            err,
            Err(CdpError::UrlGate(UrlGateError::NotHttp { .. }))
        ));
        server.abort();
    }

    #[tokio::test]
    async fn subframe_navigation_is_not_checked() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut write, mut _read) = ws.split();
            // Subframe navigating to a blocked URL must NOT abort the load.
            let ev = json!({
                "method": "Page.frameNavigated",
                "params": {"frame": {"id": "f2", "parentId": "f1", "url": "file:///etc/passwd"}}
            });
            let _ = write.send(WsMessage::Text(ev.to_string().into())).await;
            std::future::pending::<()>().await;
        });
        let mut ws = WsSession::connect(&format!("ws://{addr}/devtools/page/t1"), "page ws")
            .await
            .unwrap();
        // No error within a short window means subframe events were ignored
        // (the server keeps sending; loadEventFired never arrives → timeout,
        // NOT a URL gate error).
        let err = wait_for_load(&mut ws, Duration::from_millis(250)).await;
        assert!(matches!(err, Err(CdpError::LoadTimeout { .. })), "{err:?}");
        server.abort();
    }

    #[tokio::test]
    async fn http_get_json_parses_loopback_response() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            // Request body unread by design (Connection: close).
            let mut chunk = [0u8; 1024];
            let _ = stream.read(&mut chunk).await.unwrap();
            let body = r#"{"webSocketDebuggerUrl":"ws://127.0.0.1:9222/devtools/browser/x"}"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(resp.as_bytes()).await.unwrap();
        });
        let v = http_get_json(port, "/json/version").await.unwrap();
        assert!(
            v["webSocketDebuggerUrl"]
                .as_str()
                .unwrap()
                .contains("devtools/browser")
        );
        server.abort();
    }

    #[tokio::test]
    async fn read_page_pre_gate_rejects_private_url_before_any_network() {
        // No server at all: the pre-navigation gate must fail before the
        // client ever opens a socket.
        let session = test_session(test_config());
        let err = session
            .read_page(
                "http://169.254.169.254/latest/meta-data/",
                crate::local_browser::ReadMode::Full,
            )
            .await;
        assert!(matches!(
            err,
            Err(CdpError::UrlGate(UrlGateError::PrivateAddress { .. }))
        ));
    }

    /// v2：tab 池租约互斥——租约期间 busy=true（独占）、归还后空闲；
    /// LRU 复用（最早 last_used 优先）。全程无网络（空闲分支直达）。
    #[tokio::test]
    async fn tab_pool_lease_is_exclusive_and_lru() {
        let config = CdpConfig {
            load_timeout: Duration::from_millis(500),
            total_budget: Duration::from_secs(5),
            tab_pool_size: 2,
            dns_ttl: Duration::from_secs(300),
        };
        let session = test_session(config);
        {
            let mut pool = session.pool.lock().unwrap();
            pool.push(PooledTab {
                target_id: "t-old".to_string(),
                page_ws: None,
                busy: false,
                last_used: std::time::Instant::now() - Duration::from_secs(10),
            });
            pool.push(PooledTab {
                target_id: "t-new".to_string(),
                page_ws: None,
                busy: false,
                last_used: std::time::Instant::now(),
            });
        }
        // LRU：先取最早 last_used。
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        let id = session.lease_tab(deadline).await.unwrap();
        assert_eq!(id, "t-old");
        {
            let pool = session.pool.lock().unwrap();
            let old = pool.iter().find(|t| t.target_id == "t-old").unwrap();
            assert!(old.busy, "leased tab must be busy (exclusive)");
            let new = pool.iter().find(|t| t.target_id == "t-new").unwrap();
            assert!(!new.busy, "other tab stays free");
        }
        // 归还后空闲；再次租约按 LRU 取「最近最少使用」——t-old 刚被
        // 租用（last_used 刷新为最新），t-new 空闲更久 → 先复用 t-new。
        session.release_tab("t-old", None);
        let id2 = session.lease_tab(deadline).await.unwrap();
        assert_eq!(id2, "t-new");
        session.release_tab("t-new", None);
        let id3 = session.lease_tab(deadline).await.unwrap();
        assert_eq!(id3, "t-old");
        // 池上限信号量：permits = tab_pool_size。
        assert_eq!(session.pool_sem.available_permits(), 2);
        session.release_tab("t-old", None);
    }

    /// v2：tab 池有界——信号量 permits 恒等于池大小；池空（无空闲且未
    /// 达上限）创建路径受 creation_lock 串行化（此处池已满、全忙 → 创建
    /// 分支被信号量挡住，永不超池）。
    #[tokio::test]
    async fn tab_pool_semaphore_bounds_concurrency() {
        let config = CdpConfig {
            load_timeout: Duration::from_millis(500),
            total_budget: Duration::from_secs(5),
            tab_pool_size: 2,
            dns_ttl: Duration::from_secs(300),
        };
        let session = test_session(config);
        {
            let mut pool = session.pool.lock().unwrap();
            pool.push(PooledTab {
                target_id: "t1".to_string(),
                page_ws: None,
                busy: true,
                last_used: std::time::Instant::now(),
            });
            pool.push(PooledTab {
                target_id: "t2".to_string(),
                page_ws: None,
                busy: true,
                last_used: std::time::Instant::now(),
            });
        }
        // 池满且全忙：semaphore 无 permit——第三个并发读必须等待
        // （有界等待由调用方 total_budget 兜底），池不可能超 2。
        let p1 = session.pool_sem.acquire();
        let p2 = session.pool_sem.acquire();
        let p1 = tokio::time::timeout(Duration::from_millis(500), p1)
            .await
            .unwrap()
            .unwrap();
        let p2 = tokio::time::timeout(Duration::from_millis(500), p2)
            .await
            .unwrap()
            .unwrap();
        // 第三个 acquire 超时（全部 permit 被占）——池有界。
        let third = session.pool_sem.acquire();
        assert!(
            tokio::time::timeout(Duration::from_millis(200), third)
                .await
                .is_err(),
            "third concurrent read must wait — pool is bounded"
        );
        drop(p1);
        drop(p2);
        assert_eq!(session.pool_sem.available_permits(), 2);
    }

    #[tokio::test]
    async fn final_url_gate_runs_before_any_extraction() {
        // A page whose event stream never surfaced the redirect: the
        // post-load location.href check must reject the landed URL BEFORE
        // title/text are extracted (nothing is read from a gated page).
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let extracted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let extracted_assert = extracted.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws.split();
            let mut eval_count = 0;
            while let Some(Ok(WsMessage::Text(t))) = read.next().await {
                let v: Value = serde_json::from_str(&t).unwrap();
                let id = v["id"].as_u64().unwrap();
                match v["method"].as_str().unwrap() {
                    "Page.enable" | "Runtime.enable" => {
                        let _ = write
                            .send(WsMessage::Text(
                                json!({"id": id, "result": {}}).to_string().into(),
                            ))
                            .await;
                    }
                    "Page.navigate" => {
                        let _ = write
                            .send(WsMessage::Text(
                                json!({"id": id, "result": {}}).to_string().into(),
                            ))
                            .await;
                        // Load completes without any frameNavigated event
                        // (the redirect was never surfaced on the stream).
                        let ev = json!({"method": "Page.loadEventFired"});
                        let _ = write.send(WsMessage::Text(ev.to_string().into())).await;
                    }
                    "Runtime.evaluate" => {
                        eval_count += 1;
                        let value = if eval_count == 1 {
                            // window.location.href landed on a blocked scheme.
                            "file:///etc/passwd"
                        } else {
                            // title/text: must never be reached.
                            extracted_assert.store(true, std::sync::atomic::Ordering::SeqCst);
                            "leaked text"
                        };
                        let resp = json!({
                            "id": id,
                            "result": {"result": {"type": "string", "value": value}}
                        });
                        let _ = write.send(WsMessage::Text(resp.to_string().into())).await;
                    }
                    _ => {}
                }
            }
        });
        let mut page = WsSession::connect(&format!("ws://{addr}/devtools/page/t1"), "page ws")
            .await
            .unwrap();
        let err = read_in_page(
            &mut page,
            "https://example.com/",
            Duration::from_secs(2),
            crate::local_browser::ReadMode::Full,
        )
        .await;
        assert!(
            matches!(err, Err(CdpError::UrlGate(UrlGateError::NotHttp { .. }))),
            "{err:?}"
        );
        assert!(
            !extracted.load(std::sync::atomic::Ordering::SeqCst),
            "title/text must not be evaluated before the final URL gate"
        );
        server.abort();
    }

    #[tokio::test]
    async fn initial_about_blank_frame_is_exempt_from_the_gate() {
        // Some Chrome versions replay the initial top-frame about:blank
        // navigation after Page.enable; it must not abort the load (the
        // exemption treats it as the target's pre-navigation identity).
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut write, mut _read) = ws.split();
            let ev = json!({
                "method": "Page.frameNavigated",
                "params": {"frame": {"id": "f0", "parentId": Value::Null, "url": "about:blank"}}
            });
            let _ = write.send(WsMessage::Text(ev.to_string().into())).await;
            std::future::pending::<()>().await;
        });
        let mut ws = WsSession::connect(&format!("ws://{addr}/devtools/page/t1"), "page ws")
            .await
            .unwrap();
        // about:blank must pass silently — the wait ends in LoadTimeout (the
        // server never sends loadEventFired), NOT a URL gate error.
        let err = wait_for_load(&mut ws, Duration::from_millis(250)).await;
        assert!(matches!(err, Err(CdpError::LoadTimeout { .. })), "{err:?}");
        server.abort();
    }

    #[tokio::test]
    async fn http_get_json_rejects_non_200_status() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut chunk = [0u8; 1024];
            let _ = stream.read(&mut chunk).await.unwrap();
            let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
            stream.write_all(resp.as_bytes()).await.unwrap();
        });
        let err = http_get_json(port, "/json/version").await.unwrap_err();
        assert!(err.to_string().contains("404"), "{err}");
        server.abort();
    }

    #[tokio::test]
    async fn http_get_json_rejects_invalid_content_length() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut chunk = [0u8; 1024];
            let _ = stream.read(&mut chunk).await.unwrap();
            let resp = "HTTP/1.1 200 OK\r\nContent-Length: abc\r\n\r\n{}";
            stream.write_all(resp.as_bytes()).await.unwrap();
        });
        let err = http_get_json(port, "/json/version").await.unwrap_err();
        assert!(err.to_string().contains("Content-Length"), "{err}");
        server.abort();
    }

    // ── download-or-read event loop ──────────────────────────────────────

    /// A loopback ws pair whose server replays an event script, then answers
    /// commands. When `download_dir` is set, the file is pre-created before
    /// the events are sent (Chrome's async flush is the client's problem).
    async fn ws_pair_with_events(
        events: Vec<Value>,
        download_dir: Option<std::path::PathBuf>,
        evaluate_value: String,
    ) -> (tokio::task::JoinHandle<()>, WsSession) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
            let (mut write, mut read) = ws.split();
            if let Some(dir) = &download_dir {
                std::fs::create_dir_all(dir).unwrap();
                std::fs::write(dir.join("paper.pdf"), b"%PDF-1.4\nx").unwrap();
            }
            for ev in events {
                let _ = write.send(WsMessage::Text(ev.to_string().into())).await;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            while let Some(Ok(WsMessage::Text(t))) = read.next().await {
                let v: Value = serde_json::from_str(&t).unwrap();
                let id = v["id"].as_u64().unwrap();
                let method = v["method"].as_str().unwrap_or("");
                let resp = if method == "Runtime.evaluate" {
                    json!({"id": id, "result": {"result": {"type": "string", "value": evaluate_value.clone()}}})
                } else {
                    json!({"id": id, "result": {}})
                };
                let _ = write.send(WsMessage::Text(resp.to_string().into())).await;
            }
        });
        let ws = WsSession::connect(&format!("ws://{addr}/devtools/page/test"), "page ws")
            .await
            .unwrap();
        (server, ws)
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：preview/keywords
    /// 等待「可用文本就绪」——innerText 非空即返回，即使服务端从不发
    /// loadEventFired（慢三方资源不再拖长读取）。
    #[tokio::test]
    async fn text_ready_returns_without_load_event() {
        let (server, mut page) = ws_pair_with_events(vec![], None, "usable text".to_string()).await;
        let start = tokio::time::Instant::now();
        wait_for_text_ready(&mut page, Duration::from_secs(2))
            .await
            .unwrap();
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "text-ready must not wait for the load event"
        );
        server.abort();
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：空文本轮询到
    /// deadline → LoadTimeout（与 full 路径同型显式失败，非挂死）。
    #[tokio::test]
    async fn text_ready_times_out_when_no_text() {
        let (server, mut page) = ws_pair_with_events(vec![], None, "".to_string()).await;
        let start = tokio::time::Instant::now();
        let err = wait_for_text_ready(&mut page, Duration::from_millis(300)).await;
        assert!(matches!(err, Err(CdpError::LoadTimeout { .. })), "{err:?}");
        assert!(start.elapsed() >= Duration::from_millis(250));
        server.abort();
    }

    /// 0k 审查处理 (P2-4)：事件通道关闭（WS 断连、reader 任务退出）→
    /// 有界失败、绝不永久挂/忙循环。断连有两种形态：① write 半开 →
    /// evaluate 挂起，由 `remaining` 兜底为 LoadTimeout（旧实现裸 await
    /// 永久 pending，deadline 检查永不执行）；② write 全断 → evaluate
    /// 快速失败 → recv() 返回 None → 显式 `CDP event channel closed`
    /// （旧实现 select 每次都选中 None 分支忙循环到 deadline）。两种形态
    /// 都必须在 deadline 附近有界返回。
    #[tokio::test]
    async fn text_ready_channel_close_is_bounded_not_hang() {
        let (server, mut page) = ws_pair_with_events(vec![], None, "".to_string()).await;
        // 关闭服务端连接 → reader 任务退出 → 事件通道关闭。
        server.abort();
        let start = tokio::time::Instant::now();
        let err = wait_for_text_ready(&mut page, Duration::from_millis(500)).await;
        assert!(err.is_err(), "channel close must fail, not hang: {err:?}");
        // 有界失败：不超过 deadline + 一个轮询余量。旧实现 evaluate 裸
        // await 永久挂（测试会 hang）或 recv None 忙循环烧满 deadline。
        assert!(
            start.elapsed() < Duration::from_millis(1500),
            "text-ready must be bounded on channel close (elapsed {:?})",
            start.elapsed()
        );
    }

    /// Unique temp dir per test label (no tempfile dev-dep — codebase
    /// pattern: `std::env::temp_dir` + pid + label).
    fn test_tmp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!("orz-cdp-{label}-{}", std::process::id()))
    }

    #[tokio::test]
    async fn download_completed_returns_pdf_file() {
        let dir = test_tmp_dir("dl-completed");
        let (server, mut page) = ws_pair_with_events(
            vec![
                // final_url comes from the event's params.url (the real
                // resource URL) — NOT from evaluate_string, which reports
                // about:blank for a canceled download navigation (P1-2).
                json!({"method": "Page.downloadWillBegin", "params": {"guid": "g1", "suggestedFilename": "paper.pdf", "url": "https://example.com/paper.pdf"}}),
                json!({"method": "Page.downloadProgress", "params": {"guid": "g1", "state": "in_progress"}}),
                json!({"method": "Page.downloadProgress", "params": {"guid": "g1", "state": "completed"}}),
            ],
            Some(dir.clone()),
            "https://example.com/paper.pdf".to_string(),
        )
        .await;
        let outcome = wait_download_or_load(
            &mut page,
            "https://example.com/paper.pdf",
            &dir,
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        match outcome {
            BrowserDownloadOutcome::Pdf { path, final_url } => {
                assert_eq!(path.file_name().unwrap().to_str().unwrap(), "paper.pdf");
                assert_eq!(final_url, "https://example.com/paper.pdf");
            }
            other => panic!("expected Pdf, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn download_missing_event_url_falls_back_to_request_url() {
        let dir = test_tmp_dir("dl-fallback-url");
        let (server, mut page) = ws_pair_with_events(
            vec![
                // No params.url on downloadWillBegin — fall back to the
                // request URL (never evaluate_string's about:blank).
                json!({"method": "Page.downloadWillBegin", "params": {"guid": "g4"}}),
                json!({"method": "Page.downloadProgress", "params": {"guid": "g4", "state": "completed"}}),
            ],
            Some(dir.clone()),
            "https://example.com/other.pdf".to_string(),
        )
        .await;
        let outcome = wait_download_or_load(
            &mut page,
            "https://example.com/requested.pdf",
            &dir,
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        match outcome {
            BrowserDownloadOutcome::Pdf { final_url, .. } => {
                assert_eq!(final_url, "https://example.com/requested.pdf");
            }
            other => panic!("expected Pdf, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn browser_domain_download_events_are_matched() {
        // Current-protocol event family (P1-1): Browser.* events drive the
        // same loop as Page.*.
        let dir = test_tmp_dir("dl-browser-events");
        let (server, mut page) = ws_pair_with_events(
            vec![
                json!({"method": "Browser.downloadWillBegin", "params": {"guid": "g5", "url": "https://example.com/paper.pdf"}}),
                json!({"method": "Browser.downloadProgress", "params": {"guid": "g5", "state": "completed"}}),
            ],
            Some(dir.clone()),
            "https://example.com/paper.pdf".to_string(),
        )
        .await;
        let outcome = wait_download_or_load(
            &mut page,
            "https://example.com/paper.pdf",
            &dir,
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        match outcome {
            BrowserDownloadOutcome::Pdf { final_url, .. } => {
                assert_eq!(final_url, "https://example.com/paper.pdf");
            }
            other => panic!("expected Pdf, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn lost_completed_event_is_recovered_by_stable_file_polling() {
        // P2-4 backstop: only downloadWillBegin arrives (the completed
        // event is dropped) — the staging file stabilizes and the loop
        // returns Pdf instead of timing out.
        let dir = test_tmp_dir("dl-poll-backstop");
        let (server, mut page) = ws_pair_with_events(
            vec![json!({"method": "Page.downloadWillBegin", "params": {"guid": "g6", "url": "https://example.com/paper.pdf"}})],
            Some(dir.clone()),
            "https://example.com/paper.pdf".to_string(),
        )
        .await;
        let outcome = wait_download_or_load(
            &mut page,
            "https://example.com/paper.pdf",
            &dir,
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        match outcome {
            BrowserDownloadOutcome::Pdf { final_url, .. } => {
                assert_eq!(final_url, "https://example.com/paper.pdf");
            }
            other => panic!("expected Pdf, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn download_canceled_is_explicit_error() {
        let dir = test_tmp_dir("dl-canceled");
        let (server, mut page) = ws_pair_with_events(
            vec![
                json!({"method": "Page.downloadWillBegin", "params": {"guid": "g2"}}),
                json!({"method": "Page.downloadProgress", "params": {"guid": "g2", "state": "canceled"}}),
            ],
            None,
            "https://example.com/".to_string(),
        )
        .await;
        let err = wait_download_or_load(
            &mut page,
            "https://example.com/",
            &dir,
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CdpError::DownloadCanceled), "{err:?}");
        server.abort();
    }

    #[tokio::test]
    async fn load_without_pending_download_falls_through_to_page_read() {
        let dir = test_tmp_dir("dl-load-page");
        let (server, mut page) = ws_pair_with_events(
            vec![json!({"method": "Page.loadEventFired"})],
            None,
            "https://example.com/page".to_string(),
        )
        .await;
        let outcome = wait_download_or_load(
            &mut page,
            "https://example.com/requested",
            &dir,
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        match outcome {
            BrowserDownloadOutcome::Page(outcome) => {
                assert_eq!(outcome.final_url, "https://example.com/page");
            }
            other => panic!("expected Page, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn load_after_download_will_begin_still_waits_for_completion() {
        let dir = test_tmp_dir("dl-load-skip");
        let (server, mut page) = ws_pair_with_events(
            vec![
                json!({"method": "Page.downloadWillBegin", "params": {"guid": "g3"}}),
                // The download navigation's empty-frame load must NOT be
                // mistaken for a terminal page read.
                json!({"method": "Page.loadEventFired"}),
                json!({"method": "Page.downloadProgress", "params": {"guid": "g3", "state": "completed"}}),
            ],
            Some(dir.clone()),
            "https://example.com/paper.pdf".to_string(),
        )
        .await;
        let outcome = wait_download_or_load(
            &mut page,
            "https://example.com/paper.pdf",
            &dir,
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        match outcome {
            BrowserDownloadOutcome::Pdf { path, .. } => {
                assert_eq!(path.file_name().unwrap().to_str().unwrap(), "paper.pdf");
            }
            other => panic!("expected Pdf, got {other:?}"),
        }
        server.abort();
    }

    #[tokio::test]
    async fn download_loop_times_out_when_no_terminal_event() {
        let dir = test_tmp_dir("dl-timeout");
        let (server, mut page) = ws_pair_with_events(vec![], None, "".to_string()).await;
        let err = wait_download_or_load(
            &mut page,
            "https://example.com/",
            &dir,
            Duration::from_millis(250),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, CdpError::LoadTimeout { .. }), "{err:?}");
        server.abort();
    }

    #[tokio::test]
    async fn wait_for_new_file_returns_newest_file() {
        let dir = test_tmp_dir("dl-new-file");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.pdf"), b"a").unwrap();
        let path = wait_for_new_file(&dir, Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "a.pdf");
    }
}
