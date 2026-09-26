//! Axum server: static three-draft UI + token-gated WS/API routes
//! (0br S2, survey §5/§6).
//!
//! Route surface:
//! - `GET /`, `GET /assets/{*rest}` — static, tokenless, Host-checked
//! - `GET /ws/acp?token=` — ACP pump (single session, Origin-checked)
//! - `GET /ws/journal/{run_id}?token=&from=` — live journal tail
//!   (Origin-checked)
//! - `GET /api/runs?token=` — run listing (light metadata)
//! - `GET /api/run/{run_id}/events?token=&from=&max=` — journal slice
//! - `GET /api/conversations?token=` — conversation listing
//! - `GET /api/archives?token=` — session-archive listing (0br S3 新增面)
//! - `GET /api/archives/{session8}?token=` — in-gzip summary + bounded
//!   transcript
//! - `POST /api/archives/{session8}?token=` — archive action (0br S3)
//! - `DELETE /api/archives/{session8}?token=` — 回档 / unarchive (0br S3
//!   批六: removes the package + watermark; session data stays)
//! - `DELETE /api/sessions/{session8}?token=` — 删除 / purge (0br S3 批六:
//!   removes ALL session data — package, watermark, sidecar, runs)
//!
//! Everything except static assets requires the per-boot token; WS
//! upgrades additionally check Origin/Host (DNS-rebinding / drive-by
//! defence), and static routes run the Host check tokenlessly. The bind
//! address is validated loopback-only at startup.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::acp_pump::SessionSlot;
use crate::security;

/// Inbound ACP frame cap (16 MiB): prompts are text; a paste beyond this
/// fails fast instead of buffering unbounded bytes per connection.
const ACP_WS_MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

/// On-demand archive child cap: packaging is local gzip + a small journal
/// append; a hang beyond this means the child is stuck and the request
/// should fail visibly instead of pinning the explorer action.
const ARCHIVE_CHILD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Clone)]
pub struct ServerState {
    pub token: Arc<String>,
    pub cwd: Arc<PathBuf>,
    pub agent_binary: Arc<PathBuf>,
    pub session: SessionSlot,
}

pub struct BindError(pub String);

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Validate a bind target: loopback only, ever.
pub fn validate_bind(addr: SocketAddr) -> Result<SocketAddr, BindError> {
    if security::bind_addr_allowed(addr.ip()) {
        Ok(addr)
    } else {
        Err(BindError(format!(
            "refusing to bind {addr}: the workbench bridge is loopback-only (design: 0br S1 §6)"
        )))
    }
}

fn unauthorized(reason: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": {"code": 401, "message": reason}})),
    )
        .into_response()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({"error": {"code": status.as_u16(), "message": message}})),
    )
        .into_response()
}

type Response = axum::response::Response;

fn host_ok(headers: &HeaderMap) -> bool {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok());
    security::host_header_ok(host)
}

fn origin_ok(headers: &HeaderMap) -> bool {
    let origin = headers
        .get(axum::http::header::ORIGIN)
        .and_then(|v| v.to_str().ok());
    security::origin_allowed(origin)
}

fn gate(state: &ServerState, headers: &HeaderMap, token: Option<&String>) -> Result<(), Response> {
    if !security::token_ok(token.map(String::as_str), &state.token) {
        return Err(unauthorized("missing or invalid token"));
    }
    // Host header check guards against DNS rebinding (the token remains the
    // primary gate); static routes run the same Host check tokenlessly.
    if !host_ok(headers) {
        return Err(unauthorized("host header must name the loopback interface"));
    }
    Ok(())
}

/// Gate chain for every WS upgrade: token + Host, then a loopback-or-absent
/// Origin check. Extracted as a pure function so the full chain is unit
/// testable without hyper's `OnUpgrade` extension (the `WebSocketUpgrade`
/// extractor cannot run under `tower::ServiceExt::oneshot`).
fn ws_gate(
    state: &ServerState,
    headers: &HeaderMap,
    token: Option<&String>,
) -> Result<(), Response> {
    gate(state, headers, token)?;
    if !origin_ok(headers) {
        return Err(unauthorized("origin rejected"));
    }
    Ok(())
}

async fn index(headers: HeaderMap) -> Response {
    if !host_ok(&headers) {
        return unauthorized("host header must name the loopback interface");
    }
    serve_asset("/")
}

// The whole static face (index + assets) runs the Host check tokenlessly:
// no secrets live here, but there is no reason to leave the only ungated
// routes open to DNS-rebinding probes either.
async fn assets(headers: HeaderMap, Path(rest): Path<String>) -> Response {
    if !host_ok(&headers) {
        return unauthorized("host header must name the loopback interface");
    }
    serve_asset(&format!("/assets/{rest}"))
}

fn serve_asset(path: &str) -> Response {
    match crate::assets::lookup(path) {
        Some(asset) => (
            [
                (axum::http::header::CONTENT_TYPE, asset.content_type),
                // Embedded assets change with every carrier rebuild; without
                // a cache policy the browser heuristic-caches them and serves
                // a stale UI after an upgrade (observed live 2026-09-25).
                (axum::http::header::CACHE_CONTROL, "no-store"),
            ],
            asset.bytes,
        )
            .into_response(),
        None => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct TokenQuery {
    token: Option<String>,
}

#[derive(serde::Deserialize)]
struct TailQuery {
    token: Option<String>,
    #[serde(default)]
    from: Option<u64>,
}

async fn ws_acp(
    State(state): State<ServerState>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = ws_gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    // Explicit inbound frame caps (defaults are far larger): a prompt is
    // text, and an oversized paste should fail fast instead of buffering
    // tens of MiB per connection.
    let upgrade = upgrade
        .max_message_size(ACP_WS_MAX_MESSAGE_BYTES)
        .max_frame_size(ACP_WS_MAX_MESSAGE_BYTES);
    if state.session.is_held().await {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": {"code": 409, "message": "此桥为单会话：已有活动连接（关闭该页后重试）"}})),
        )
            .into_response();
    }
    let slot = state.session.clone();
    upgrade.on_upgrade(move |socket| async move {
        // Acquire inside the upgrade so a failed upgrade never leaks the
        // slot; a race loser gets its socket closed with an explanation.
        if !slot.try_acquire().await {
            // Inherent WebSocket::send + drop-close (axum semantics).
            let mut socket = socket;
            let _ = socket
                .send(axum::extract::ws::Message::text(
                    r#"{"error":{"code":409,"message":"此桥为单会话：已有活动连接"}}"#,
                ))
                .await;
            return;
        }
        // RAII release: the slot frees on every exit path of `pump`,
        // including an unexpected panic unwinding through this future.
        let guard = crate::acp_pump::SlotGuard::new(slot.clone());
        crate::acp_pump::pump(
            socket,
            (*state.agent_binary).clone(),
            (*state.cwd).clone(),
            guard.releaser(),
        )
        .await;
        drop(guard);
    })
}

async fn ws_journal(
    State(state): State<ServerState>,
    Path(run_id): Path<String>,
    Query(q): Query<TailQuery>,
    headers: HeaderMap,
    upgrade: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = ws_gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    let upgrade = upgrade.max_message_size(1 << 20).max_frame_size(1 << 20);
    let Some(path) = crate::journal_tail::resolve(&state.cwd, &run_id) else {
        return (StatusCode::BAD_REQUEST, "invalid run id").into_response();
    };
    let from = q.from.unwrap_or(0);
    upgrade.on_upgrade(move |socket| crate::journal_tail::run_tail(socket, path, from))
}

async fn api_runs(
    State(state): State<ServerState>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    Json(json!({ "runs": crate::runs::list_runs(&state.cwd) })).into_response()
}

#[derive(serde::Deserialize)]
struct EventsQuery {
    token: Option<String>,
    #[serde(default)]
    from: Option<u64>,
    #[serde(default)]
    max: Option<u64>,
}

async fn api_run_events(
    State(state): State<ServerState>,
    Path(run_id): Path<String>,
    Query(q): Query<EventsQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    let from = q.from.unwrap_or(0);
    match crate::runs::slice_events(&state.cwd, &run_id, from, q.max.unwrap_or(u64::MAX)) {
        Some((lines, next_offset)) => Json(json!({
            "run_id": run_id,
            "from": from,
            "next_offset": next_offset,
            "lines": lines,
        }))
        .into_response(),
        None => (StatusCode::BAD_REQUEST, "invalid run id").into_response(),
    }
}

async fn api_conversations(
    State(state): State<ServerState>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    Json(json!({ "conversations": crate::runs::list_conversations(&state.cwd) })).into_response()
}

/// Session-archive listing (0br S3 新增面): `.gsa/archives/{s8}.json.gz`
/// packages with best-effort milestone facts.
async fn api_archives(
    State(state): State<ServerState>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    Json(json!({ "archives": crate::archives::list_archives(&state.cwd) })).into_response()
}

/// In-gzip summary of one archive package: conversation facts +
/// `archive_keys` passthrough + bounded transcript. Errors map 1:1 to the
/// typed read taxonomy (400 id / 404 missing / 413 too large / 500 corrupt).
async fn api_archive_detail(
    State(state): State<ServerState>,
    Path(session8): Path<String>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    match crate::archives::archive_detail(&state.cwd, &session8) {
        Ok(detail) => Json(detail).into_response(),
        Err(e) => match e {
            crate::archives::ArchiveReadError::InvalidId => {
                error_response(StatusCode::BAD_REQUEST, &e.message())
            }
            crate::archives::ArchiveReadError::Missing => {
                error_response(StatusCode::NOT_FOUND, &e.message())
            }
            crate::archives::ArchiveReadError::TooLarge => {
                error_response(StatusCode::PAYLOAD_TOO_LARGE, &e.message())
            }
            crate::archives::ArchiveReadError::Corrupt(_) => {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.message())
            }
        },
    }
}

/// On-demand archive action (0br S3, user directive 2026-09-25「归档活跃
/// 会话」): the UI files the request; EXECUTION stays with the agent — the
/// bridge spawns `<agent_binary> archive <session8>` (the same binary's
/// dedicated subcommand, which reuses the close-archive primitive) and
/// relays the outcome. Zero execution facts in the browser (ADR-0010
/// §14.78 条 5).
async fn api_archive_session(
    State(state): State<ServerState>,
    Path(session8): Path<String>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    if !security::session_id_ok(&session8) {
        return error_response(StatusCode::BAD_REQUEST, "无效的会话标识");
    }
    match run_agent_session_tool(&state, "archive", &session8).await {
        Ok(message) => Json(json!({ "ok": true, "session8": session8, "message": message }))
            .into_response(),
        Err((status, message)) => error_response(status, &message),
    }
}

/// 回档（0br S3 批六，2026-09-25 第七用户令）：`DELETE /api/archives/{s8}`
/// → 桥 spawn `<agent_binary> unarchive <s8>`（移除归档包＋水位，会话回
/// 活跃组；数据保留）。执行仍在 agent 侧，浏览器零执行事实。
async fn api_archive_delete(
    State(state): State<ServerState>,
    Path(session8): Path<String>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    if !security::session_id_ok(&session8) {
        return error_response(StatusCode::BAD_REQUEST, "无效的会话标识");
    }
    match run_agent_session_tool(&state, "unarchive", &session8).await {
        Ok(message) => Json(json!({ "ok": true, "session8": session8, "message": message }))
            .into_response(),
        Err((status, message)) => error_response(status, &message),
    }
}

/// 删除（0br S3 批六，2026-09-25 第七用户令）：`DELETE /api/sessions/{s8}`
/// → 桥 spawn `<agent_binary> delete-session <s8>`（彻底移除该会话的
/// 归档包/水位/侧车/运行 journal——不可恢复，UI 侧先经确认弹窗）。执行
/// 仍在 agent 侧，浏览器零执行事实。
async fn api_session_delete(
    State(state): State<ServerState>,
    Path(session8): Path<String>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    if !security::session_id_ok(&session8) {
        return error_response(StatusCode::BAD_REQUEST, "无效的会话标识");
    }
    match run_agent_session_tool(&state, "delete-session", &session8).await {
        Ok(message) => Json(json!({ "ok": true, "session8": session8, "message": message }))
            .into_response(),
        Err((status, message)) => error_response(status, &message),
    }
}

/// 信任授信（0br S3 批七，用户报告 prompt「workspace not trusted」）：
/// `POST /api/trust?token=` → 桥 spawn `<agent_binary> trust <workspace>`
/// ——把工作区授信写入与 prompt 子进程同一 GROK_HOME 下的信任存储
/// （TUI 信任窗的 Web 等价物；授信决定由 UI 弹窗确认在先，工作区路径
/// 取桥自身的 state.cwd，浏览器零执行事实、零路径输入）。
async fn api_trust(
    State(state): State<ServerState>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    let cwd = state.cwd.display().to_string();
    match run_agent_session_tool(&state, "trust", &cwd).await {
        Ok(message) => Json(json!({ "ok": true, "cwd": cwd, "message": message })).into_response(),
        Err((status, message)) => error_response(status, &message),
    }
}

/// Shared spawn-relay for the session maintenance actions (archive /
/// unarchive / delete-session / trust): one child shape, one timeout, one
/// error mapping — no second implementation per route. The bridge only
/// points the same binary at its dedicated subcommand; the semantics live
/// in orz-host.
async fn run_agent_session_tool(
    state: &ServerState,
    subcommand: &str,
    arg: &str,
) -> Result<String, (StatusCode, String)> {
    // 面向用户的中文动词（启动失败/超时文案）；子命令名本身保持英文。
    let label = match subcommand {
        "archive" => "归档",
        "unarchive" => "回档",
        "delete-session" => "删除",
        "trust" => "信任",
        other => other,
    };
    let mut child = tokio::process::Command::new((*state.agent_binary).clone());
    child
        .arg(subcommand)
        .arg(arg)
        .current_dir((*state.cwd).clone())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let output = match tokio::time::timeout(ARCHIVE_CHILD_TIMEOUT, child.output()).await {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{label}进程启动失败: {e}"),
            ));
        }
        Err(_) => {
            return Err((
                StatusCode::GATEWAY_TIMEOUT,
                format!("{label}超时（120 s）——子进程已被终止"),
            ));
        }
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        Err((StatusCode::INTERNAL_SERVER_ERROR, detail.to_string()))
    }
}

/// Session bootstrap facts (token-gated): the workspace cwd the frontend
/// must pass to `session/new` (the browser cannot know it), plus the
/// bridge's agent binary identity.
async fn api_boot(
    State(state): State<ServerState>,
    Query(q): Query<TokenQuery>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = gate(&state, &headers, q.token.as_ref()) {
        return resp;
    }
    Json(json!({
        "cwd": state.cwd.display().to_string(),
        "agent": state.agent_binary.display().to_string(),
        // 用户全局信任存储（~/.grok/trusted_folders.toml）的授予清单——
        // 探索器「工作区 → 已信任工作区」子级的数据源（0br S3）。
        "trusted_workspaces": crate::trust::list_trusted_workspaces(),
    }))
    .into_response()
}

/// Build the router (exposed for tests).
pub fn router(state: ServerState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/assets/{*rest}", get(assets))
        .route("/ws/acp", get(ws_acp))
        .route("/ws/journal/{run_id}", get(ws_journal))
        .route("/api/runs", get(api_runs))
        .route("/api/run/{run_id}/events", get(api_run_events))
        .route("/api/conversations", get(api_conversations))
        .route("/api/archives", get(api_archives))
        .route(
            "/api/archives/{session8}",
            get(api_archive_detail)
                .post(api_archive_session)
                .delete(api_archive_delete),
        )
        .route("/api/sessions/{session8}", delete(api_session_delete))
        .route("/api/trust", post(api_trust))
        .route("/api/boot", get(api_boot))
        .with_state(state)
}

/// Serve forever on the validated loopback address.
pub async fn serve(state: ServerState, bind: SocketAddr) -> Result<(), BindError> {
    let bind = validate_bind(bind)?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| BindError(format!("bind {bind} failed: {e}")))?;
    // Print the entry URL only once the ACTUAL port is known (`--addr
    // …:0` picks an ephemeral one); format matches main.js boot().
    let actual = listener
        .local_addr()
        .map_err(|e| BindError(format!("local_addr failed: {e}")))?;
    println!("orz web: 打开 http://{actual}/#token={}", state.token);
    let app = router(state);
    axum::serve(listener, app)
        .await
        .map_err(|e| BindError(format!("server error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> ServerState {
        ServerState {
            token: Arc::new("tok-123".into()),
            cwd: Arc::new(std::env::temp_dir()),
            agent_binary: Arc::new(PathBuf::from("orz-test-binary")),
            session: SessionSlot::default(),
        }
    }

    #[tokio::test]
    async fn api_rejects_missing_token() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs")
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_rejects_wrong_token() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs?token=wrong")
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_rejects_non_loopback_host_header() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs?token=tok-123")
                    .header("host", "attacker.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_accepts_token_and_loopback_host() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/runs?token=tok-123")
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn static_assets_serve_without_token() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn static_assets_reject_non_loopback_host_header() {
        let app = router(test_state());
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/app.css")
                    .header("host", "evil.example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    /// Minimal headers for the WS-gate unit tests (the pure `ws_gate`
    /// chain, not the `WebSocketUpgrade` extractor, which requires
    /// hyper's `OnUpgrade` extension and therefore 426s under oneshot).
    fn ws_header_map(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                axum::http::HeaderValue::from_str(v).unwrap(),
            );
        }
        map
    }

    #[tokio::test]
    async fn ws_gate_rejects_foreign_origin() {
        let state = test_state();
        let headers = ws_header_map(&[("host", "127.0.0.1:1"), ("origin", "https://evil.example")]);
        let resp = ws_gate(&state, &headers, Some(&"tok-123".to_string())).unwrap_err();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn ws_gate_accepts_loopback_origin_or_absent() {
        let state = test_state();
        let token = "tok-123".to_string();
        for pairs in [
            vec![("host", "127.0.0.1:1")],
            vec![("host", "127.0.0.1:1"), ("origin", "http://localhost:3000")],
        ] {
            let headers = ws_header_map(&pairs);
            assert!(
                ws_gate(&state, &headers, Some(&token)).is_ok(),
                "loopback/absent origin must pass: {pairs:?}"
            );
        }
    }

    #[tokio::test]
    async fn ws_gate_still_requires_token_and_host() {
        let state = test_state();
        let headers = ws_header_map(&[("host", "127.0.0.1:1")]);
        assert!(ws_gate(&state, &headers, None).is_err(), "token required");
        let headers = ws_header_map(&[
            ("host", "evil.example"),
            ("origin", "http://localhost:3000"),
        ]);
        assert!(
            ws_gate(&state, &headers, Some(&"tok-123".to_string())).is_err(),
            "host gate precedes origin"
        );
    }

    #[tokio::test]
    async fn archive_api_gates_reject_then_detail_serves() {
        // Fixture: a real package under a temp workspace (flate2 GzEncoder,
        // same writer shape as the agent side).
        let root = std::env::temp_dir().join(format!(
            "orz-web-archive-api-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join(".gsa/archives")).unwrap();
        {
            use std::io::Write;
            let envelope = serde_json::json!({
                "schema": "session-archive-package-v0.2",
                "conversation": {"messages": [{"role": "user", "content": "问题"}]},
                "archive_keys": {"lif": {"round_start": 1, "round_end": 2}},
            });
            let file = std::fs::File::create(root.join(".gsa/archives/6aab1234.json.gz")).unwrap();
            let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            enc.write_all(envelope.to_string().as_bytes()).unwrap();
            enc.finish().unwrap();
        }
        let state = ServerState {
            token: Arc::new("tok-123".into()),
            cwd: Arc::new(root.clone()),
            agent_binary: Arc::new(PathBuf::from("orz-test-binary")),
            session: SessionSlot::default(),
        };
        let get = |app: axum::Router, uri: String| async move {
            app.oneshot(
                Request::builder()
                    .uri(uri)
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap()
        };

        // No token / forged host → 401 on both archive routes.
        let app = router(test_state());
        assert_eq!(
            get(app, "/api/archives".into()).await.status(),
            StatusCode::UNAUTHORIZED
        );
        let app = router(test_state());
        assert_eq!(
            get(app, "/api/archives/6aab1234".into()).await.status(),
            StatusCode::UNAUTHORIZED
        );

        // Traversal / reserved-name session ids → 400, filesystem untouched
        // (the token gate precedes the id check — 401 without one).
        let app = router(state.clone());
        for bad in ["..", "a%2Fb", "NUL", "x.y"] {
            assert_eq!(
                get(app.clone(), format!("/api/archives/{bad}?token=tok-123"))
                    .await
                    .status(),
                StatusCode::BAD_REQUEST,
                "session id {bad:?} must be rejected"
            );
        }

        // Missing package → 404.
        assert_eq!(
            get(
                router(state.clone()),
                "/api/archives/6aab9999?token=tok-123".into()
            )
            .await
            .status(),
            StatusCode::NOT_FOUND
        );

        // Happy path: summary facts + transcript + archive_keys passthrough.
        let resp = get(
            router(state.clone()),
            "/api/archives/6aab1234?token=tok-123".into(),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["session8"], "6aab1234");
        assert_eq!(v["message_count"], 1);
        assert_eq!(v["archive_keys"]["lif"]["round_end"], 2);
        assert_eq!(v["messages"][0]["role"], "user");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn archive_action_gates_id_and_reports_spawn_failure() {
        let app = router(test_state());
        // Token gate first (401), then id gate (400).
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/archives/6aab1234")
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let resp = router(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/archives/a%2Fb?token=tok-123")
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        // The state's agent binary does not exist → the child fails to spawn
        // and the error surfaces verbatim (no silent success).
        let resp = router(test_state())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/archives/6aab1234?token=tok-123")
                    .header("host", "127.0.0.1:1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap()
                .starts_with("归档进程启动失败"),
            "{v}"
        );
    }

    /// 0br S3 批六钉子：回档（DELETE /api/archives/{s8}）与删除（DELETE
    /// /api/sessions/{s8}）走同一令牌/路径门与同一 spawn 错误透传面。
    #[tokio::test]
    async fn unarchive_and_delete_routes_gate_id_and_relay_spawn_failure() {
        for (method, uri) in [
            ("DELETE", "/api/archives/6aab1234"),
            ("DELETE", "/api/sessions/6aab1234"),
        ] {
            // 无令牌 ⇒ 401。
            let resp = router(test_state())
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .header("host", "127.0.0.1:1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{method} {uri}");

            // 令牌在、id 非法 ⇒ 400。
            let bad = uri.replacen("6aab1234", "a%2Fb", 1);
            let resp = router(test_state())
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(format!("{bad}?token=tok-123"))
                        .header("host", "127.0.0.1:1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{method} {uri}");

            // agent 二进制不存在 ⇒ 子进程启动失败逐字透传（不静默成功）。
            let resp = router(test_state())
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(format!("{uri}?token=tok-123"))
                        .header("host", "127.0.0.1:1")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR, "{method} {uri}");
            let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
            let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let message = v["error"]["message"].as_str().unwrap();
            let expected = if method == "DELETE" && uri.contains("archives") {
                "回档进程启动失败"
            } else {
                "删除进程启动失败"
            };
            assert!(message.starts_with(expected), "{method} {uri}: {v}");
        }
    }

    #[test]
    fn bind_validation_refuses_non_loopback() {
        assert!(validate_bind("127.0.0.1:21487".parse().unwrap()).is_ok());
        assert!(validate_bind("[::1]:21487".parse().unwrap()).is_ok());
        assert!(validate_bind("0.0.0.0:21487".parse().unwrap()).is_err());
        assert!(validate_bind("192.168.0.9:21487".parse().unwrap()).is_err());
    }
}
