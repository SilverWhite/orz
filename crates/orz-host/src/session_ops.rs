//! Session lifecycle primitives (0dk S2, 332 批) — transport-neutral.
//!
//! 设计权威：[`docs/SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md`]。
//! 本模块只依赖 [`SessionHost`]（查态／取消／释放／标记检查），**不依赖
//! ACP、不 import `xai-acp-lib`**：换底座时重写 `acp_server.rs` 里的传输
//! 适配器，状态机与错误段（-32010..-32014）随 orz 走（设计 §11）。
//!
//! 状态机（设计 §3）：
//! `created → idle → active → closing → closed`，另有 `archived/deleted`
//! 两个持久态由归档/删除路径维护；本模块驱动 close/evict 的同步部分，
//! 「等 run 终态」的等待上限由调用方传入（生产 10s，测试可缩短）。

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// 生产 close 等待 run 终态的上限（设计 §6.3）。
pub const DEFAULT_CLOSE_TIMEOUT: Duration = Duration::from_secs(10);

/// close 等待期间的轮询间隔——足够细（终态通常 <1 个 tick），又不空转。
pub const CLOSE_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// 会话状态（设计 §3.1）。线格式为 snake_case 字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Created,
    Idle,
    Active,
    Closing,
    Closed,
    Archived,
    Deleted,
}

impl SessionState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Idle => "idle",
            Self::Active => "active",
            Self::Closing => "closing",
            Self::Closed => "closed",
            Self::Archived => "archived",
            Self::Deleted => "deleted",
        }
    }
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 关闭原因（设计 §5.1）。`evict` 是容量驱逐，与用户主动关闭区分审计。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    UserClose,
    Switch,
    Evict,
    Shutdown,
}

impl CloseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserClose => "user_close",
            Self::Switch => "switch",
            Self::Evict => "evict",
            Self::Shutdown => "shutdown",
        }
    }

    /// 宽进：`close` / `user` / `user-close` 都归到 `user_close`，未知值
    /// 也按用户主动关闭处理（绝不把未知原因升级为 evict/审计特例）。
    pub fn parse(raw: Option<&str>) -> Self {
        match raw.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
            "evict" => Self::Evict,
            "switch" => Self::Switch,
            "shutdown" => Self::Shutdown,
            _ => Self::UserClose,
        }
    }
}

/// 传输无关的命令枚举（设计 §5.1）。ACP 适配器只做方法名↔命令的映射。
#[derive(Debug, Clone)]
pub enum SessionCommand {
    Close {
        session_id: String,
        reason: CloseReason,
    },
    State {
        session_id: String,
    },
    Delete {
        session_id: String,
    },
}

impl SessionCommand {
    pub fn session_id(&self) -> &str {
        match self {
            Self::Close { session_id, .. }
            | Self::State { session_id }
            | Self::Delete { session_id } => session_id,
        }
    }
}

/// close 的成功结果（设计 §5.2 线格式）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOutcome {
    pub session_id: String,
    pub state: SessionState,
    pub closed: bool,
    pub already: bool,
}

impl SessionOutcome {
    pub fn closed(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            state: SessionState::Closed,
            closed: true,
            already: false,
        }
    }

    pub fn already_closed(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            state: SessionState::Closed,
            closed: true,
            already: true,
        }
    }
}

/// 私有错误段（设计 §5.3）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    NotFound {
        session_id: String,
    },
    Busy {
        session_id: String,
        evidence: String,
        waited_ms: u64,
    },
    InvalidState {
        session_id: String,
        state: SessionState,
        verb: &'static str,
    },
    LeaseConflict {
        session_id: String,
        detail: String,
    },
    AttachConflict {
        workspace: String,
    },
    /// 文件/持久化层失败（删除原语）；不是协议状态机的一部分，但必须
    /// 以「内部错误 + 证据」上浮，不能静默成功。
    Internal {
        session_id: String,
        detail: String,
    },
}

impl SessionError {
    /// JSON-RPC 扩展错误码（orz 自有段，设计 §5.3）。
    pub fn code(&self) -> i32 {
        match self {
            Self::NotFound { .. } => -32010,
            Self::Busy { .. } => -32011,
            Self::InvalidState { .. } => -32012,
            Self::LeaseConflict { .. } => -32013,
            Self::AttachConflict { .. } => -32014,
            Self::Internal { .. } => -32603,
        }
    }

    /// 短名（error.data.code）——Web `copy.js` 的键。
    pub fn name(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "session_not_found",
            Self::Busy { .. } => "session_busy",
            Self::InvalidState { .. } => "invalid_state",
            Self::LeaseConflict { .. } => "lease_conflict",
            Self::AttachConflict { .. } => "attach_conflict",
            Self::Internal { .. } => "internal",
        }
    }

    /// 六类之一（设计 §9.1）。会话错误全部属于 `session`，仅有传输占用
    /// 归 `connection`。
    pub fn class(&self) -> &'static str {
        match self {
            Self::AttachConflict { .. } => "connection",
            Self::Internal { .. } => "internal",
            _ => "session",
        }
    }

    /// 四要素之「下一步」（设计 §9.2）。
    pub fn hint(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "该会话已删除或从未创建；请新建会话，或从归档恢复",
            Self::Busy { .. } => "等待当前回复结束后重试；需要立即停止请先点「停止」",
            Self::InvalidState { .. } => "先结束当前操作（停止/等待回复），再重试",
            Self::LeaseConflict { .. } => "关闭占用该会话的另一个 orz 窗口后重试",
            Self::AttachConflict { .. } => "关闭占用此工作区的其他 orz 窗口后刷新页面",
            Self::Internal { .. } => "重试；仍失败请记录证据并重启 orz web",
        }
    }

    /// 四要素之「证据在哪」。
    pub fn evidence(&self) -> String {
        match self {
            Self::NotFound { session_id } => format!("session={session_id}"),
            Self::Busy {
                session_id,
                evidence,
                ..
            } => format!("session={session_id}; {evidence}"),
            Self::InvalidState {
                session_id, state, ..
            } => format!("session={session_id}; state={state}"),
            Self::LeaseConflict { session_id, detail } => format!("session={session_id}; {detail}"),
            Self::AttachConflict { workspace } => format!("workspace={workspace}"),
            Self::Internal { session_id, detail } => format!("session={session_id}; {detail}"),
        }
    }

    /// JSON-RPC `error.data`（四要素；设计 §9.3）。
    pub fn data(&self) -> serde_json::Value {
        serde_json::json!({
            "code": self.name(),
            "class": self.class(),
            "hint": self.hint(),
            "evidence": self.evidence(),
        })
    }
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { session_id } => write!(f, "session not found: {session_id}"),
            Self::Busy {
                session_id,
                evidence,
                waited_ms,
            } => write!(
                f,
                "session busy: {session_id} still has an in-flight run after {waited_ms}ms ({evidence})"
            ),
            Self::InvalidState {
                session_id,
                state,
                verb,
            } => write!(
                f,
                "invalid state: cannot {verb} a {state} session ({session_id})"
            ),
            Self::LeaseConflict { session_id, detail } => write!(
                f,
                "lease conflict: {session_id} is attached elsewhere ({detail})"
            ),
            Self::AttachConflict { workspace } => {
                write!(f, "workspace occupied: {workspace}")
            }
            Self::Internal { session_id, detail } => {
                write!(f, "session operation failed: {session_id}: {detail}")
            }
        }
    }
}

impl std::error::Error for SessionError {}

/// `release` 的原子结果。`Busy` 表示「查态与移除之间出现了 run」——状态
/// 必须保持 `active`，绝不半关闭（设计 §6.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseOutcome {
    Released,
    Busy,
    AlreadyClosed,
    NotFound,
}

/// 会话宿主的最小面（查/取消/释放/持久标记）。实现方拥有锁与文件布局；
/// 本模块只做状态机裁决。
pub trait SessionHost {
    fn state_of(&self, session_id: &str) -> Option<SessionState>;
    /// 取消在途 run；返回是否命中活动 run。
    fn cancel(&self, session_id: &str) -> bool;
    /// 原子「标记 closing + 移除」；活动 run 存在时返回 [`ReleaseOutcome::Busy`]。
    fn release(&self, session_id: &str, reason: CloseReason) -> ReleaseOutcome;
    /// 在途 run 的审计证据（如 `RUN-<s8>-3`）。
    fn run_evidence(&self, session_id: &str) -> Option<String>;
    fn is_deleted(&self, session_id: &str) -> bool;
    fn is_closed(&self, session_id: &str) -> bool;
    /// 另一进程持 lease（fail-closed）。
    fn lease_held_elsewhere(&self, session_id: &str) -> bool;
}

/// close 驱动（设计 §6）：先 cancel，等终态，最多 `timeout`；超时返回
/// `session_busy` 且**不**移除会话。
pub async fn close<H: SessionHost + ?Sized>(
    host: &H,
    session_id: &str,
    reason: CloseReason,
    timeout: Duration,
) -> Result<SessionOutcome, SessionError> {
    match host.state_of(session_id) {
        Some(SessionState::Closing) => {
            return Err(SessionError::InvalidState {
                session_id: session_id.to_string(),
                state: SessionState::Closing,
                verb: "close",
            });
        }
        Some(SessionState::Active) => {
            host.cancel(session_id);
            let started = std::time::Instant::now();
            while matches!(
                host.state_of(session_id),
                Some(SessionState::Active) | Some(SessionState::Closing)
            ) {
                if started.elapsed() >= timeout {
                    return Err(SessionError::Busy {
                        session_id: session_id.to_string(),
                        evidence: host
                            .run_evidence(session_id)
                            .unwrap_or_else(|| "run=unknown".to_string()),
                        waited_ms: started.elapsed().as_millis() as u64,
                    });
                }
                tokio::time::sleep(CLOSE_POLL_INTERVAL).await;
            }
        }
        Some(_) => {}
        None => {
            if host.is_deleted(session_id) {
                return Err(SessionError::NotFound {
                    session_id: session_id.to_string(),
                });
            }
            if host.lease_held_elsewhere(session_id) {
                return Err(SessionError::LeaseConflict {
                    session_id: session_id.to_string(),
                    detail: "lease locked by another orz process".to_string(),
                });
            }
            if host.is_closed(session_id) {
                return Ok(SessionOutcome::already_closed(session_id));
            }
            return Err(SessionError::NotFound {
                session_id: session_id.to_string(),
            });
        }
    }

    match host.release(session_id, reason) {
        ReleaseOutcome::Released => Ok(SessionOutcome::closed(session_id)),
        ReleaseOutcome::AlreadyClosed => Ok(SessionOutcome::already_closed(session_id)),
        ReleaseOutcome::Busy => Err(SessionError::Busy {
            session_id: session_id.to_string(),
            evidence: host
                .run_evidence(session_id)
                .unwrap_or_else(|| "run=unknown".to_string()),
            waited_ms: 0,
        }),
        ReleaseOutcome::NotFound => {
            if host.is_closed(session_id) {
                Ok(SessionOutcome::already_closed(session_id))
            } else {
                Err(SessionError::NotFound {
                    session_id: session_id.to_string(),
                })
            }
        }
    }
}

/// `session_state` 查询：内存态优先；未知但留有 closed/tombstone 标记时
/// 返回持久态，仍是未知则 `session_not_found`。
pub fn state_of<H: SessionHost + ?Sized>(
    host: &H,
    session_id: &str,
) -> Result<SessionOutcome, SessionError> {
    if host.is_deleted(session_id) {
        return Ok(SessionOutcome {
            session_id: session_id.to_string(),
            state: SessionState::Deleted,
            closed: true,
            already: true,
        });
    }
    if let Some(state) = host.state_of(session_id) {
        return Ok(SessionOutcome {
            session_id: session_id.to_string(),
            state,
            closed: matches!(
                state,
                SessionState::Closed | SessionState::Archived | SessionState::Deleted
            ),
            already: false,
        });
    }
    if host.is_closed(session_id) {
        return Ok(SessionOutcome::already_closed(session_id));
    }
    Err(SessionError::NotFound {
        session_id: session_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeHost {
        state: Mutex<Option<SessionState>>,
        cancels: Mutex<u32>,
        /// `cancel` 是否真的收敛（false = 卡在 active，用于超时钉）。
        cancel_converges: Mutex<bool>,
        deleted: Mutex<bool>,
        closed_marker: Mutex<bool>,
        lease_elsewhere: Mutex<bool>,
    }

    impl FakeHost {
        fn with_state(state: SessionState) -> Self {
            Self {
                state: Mutex::new(Some(state)),
                cancel_converges: Mutex::new(true),
                ..Self::default()
            }
        }
    }

    impl SessionHost for FakeHost {
        fn state_of(&self, _session_id: &str) -> Option<SessionState> {
            *self.state.lock().unwrap()
        }

        fn cancel(&self, _session_id: &str) -> bool {
            *self.cancels.lock().unwrap() += 1;
            if *self.cancel_converges.lock().unwrap() {
                *self.state.lock().unwrap() = Some(SessionState::Idle);
            }
            true
        }

        fn release(&self, _session_id: &str, _reason: CloseReason) -> ReleaseOutcome {
            let mut state = self.state.lock().unwrap();
            match *state {
                Some(SessionState::Active) | Some(SessionState::Closing) => ReleaseOutcome::Busy,
                Some(_) => {
                    *state = None;
                    *self.closed_marker.lock().unwrap() = true;
                    ReleaseOutcome::Released
                }
                None => ReleaseOutcome::NotFound,
            }
        }

        fn run_evidence(&self, _session_id: &str) -> Option<String> {
            Some("RUN-deadbeef-3".to_string())
        }

        fn is_deleted(&self, _session_id: &str) -> bool {
            *self.deleted.lock().unwrap()
        }

        fn is_closed(&self, _session_id: &str) -> bool {
            *self.closed_marker.lock().unwrap()
        }

        fn lease_held_elsewhere(&self, _session_id: &str) -> bool {
            *self.lease_elsewhere.lock().unwrap()
        }
    }

    #[tokio::test]
    async fn close_idle_releases_once() {
        let host = FakeHost::with_state(SessionState::Idle);
        let out = close(
            &host,
            "sess-a",
            CloseReason::UserClose,
            Duration::from_secs(1),
        )
        .await
        .expect("close idle");
        assert!(out.closed && !out.already);
        assert_eq!(out.state, SessionState::Closed);
        assert_eq!(*host.state.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn close_active_cancels_then_closes() {
        let host = FakeHost::with_state(SessionState::Active);
        let out = close(&host, "sess-a", CloseReason::Switch, Duration::from_secs(1))
            .await
            .expect("close active");
        assert!(out.closed);
        assert_eq!(*host.cancels.lock().unwrap(), 1);
        assert_eq!(*host.state.lock().unwrap(), None);
    }

    #[tokio::test]
    async fn close_times_out_and_keeps_active() {
        let host = FakeHost::with_state(SessionState::Active);
        *host.cancel_converges.lock().unwrap() = false;
        let err = close(
            &host,
            "sess-a",
            CloseReason::UserClose,
            Duration::from_millis(40),
        )
        .await
        .expect_err("must time out");
        assert!(matches!(err, SessionError::Busy { .. }), "{err:?}");
        assert_eq!(err.code(), -32011);
        assert_eq!(*host.state.lock().unwrap(), Some(SessionState::Active));
    }

    #[tokio::test]
    async fn close_is_idempotent_via_closed_marker() {
        let host = FakeHost::with_state(SessionState::Idle);
        close(
            &host,
            "sess-a",
            CloseReason::UserClose,
            Duration::from_secs(1),
        )
        .await
        .expect("first close");
        let again = close(
            &host,
            "sess-a",
            CloseReason::UserClose,
            Duration::from_secs(1),
        )
        .await
        .expect("second close");
        assert!(again.already && again.closed);
    }

    #[tokio::test]
    async fn unknown_session_is_not_found_but_tombstone_wins() {
        let host = FakeHost::default();
        let err = close(
            &host,
            "sess-a",
            CloseReason::UserClose,
            Duration::from_secs(1),
        )
        .await
        .expect_err("unknown");
        assert!(matches!(err, SessionError::NotFound { .. }));

        *host.deleted.lock().unwrap() = true;
        let err = close(
            &host,
            "sess-a",
            CloseReason::UserClose,
            Duration::from_secs(1),
        )
        .await
        .expect_err("deleted");
        assert!(matches!(err, SessionError::NotFound { .. }));
        assert_eq!(err.code(), -32010);
    }

    #[tokio::test]
    async fn lease_conflict_is_reported_before_not_found() {
        let host = FakeHost::default();
        *host.lease_elsewhere.lock().unwrap() = true;
        let err = close(
            &host,
            "sess-a",
            CloseReason::UserClose,
            Duration::from_secs(1),
        )
        .await
        .expect_err("lease conflict");
        assert!(matches!(err, SessionError::LeaseConflict { .. }));
        assert_eq!(err.code(), -32013);
        assert_eq!(err.class(), "session");
    }

    #[test]
    fn error_data_carries_four_elements() {
        let err = SessionError::NotFound {
            session_id: "abc".into(),
        };
        let data = err.data();
        assert_eq!(data["code"], "session_not_found");
        assert_eq!(data["class"], "session");
        assert!(!data["hint"].as_str().unwrap().is_empty());
        assert!(data["evidence"].as_str().unwrap().contains("abc"));
    }
}
