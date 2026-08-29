//! Retrieval mode surface — `RetrievalMode` / `RetrievalCapability` —
//! batch N4 of the controller split second round
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29 §3.5).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use serde::{Deserialize, Serialize};

/// GAP-RETRIEVAL-TOOLS (2026-08-10) — ADR-0010 §3.7.1: the explicit
/// session/task-contract retrieval mode. `off` is the unauthenticated
/// default; `local_browser` is the preferred enabled mode; `framework_fallback`
/// may only be entered by explicit user / parent-task-contract selection.
/// Mode changes are NEVER implicit — a failure (timeout/login/CAPTCHA) must
/// surface explicitly, not switch modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    Off,
    LocalBrowser,
    FrameworkFallback,
}

impl RetrievalMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RetrievalMode::Off => "off",
            RetrievalMode::LocalBrowser => "local_browser",
            RetrievalMode::FrameworkFallback => "framework_fallback",
        }
    }

    /// Parse the session-level mode from its wire form (ACP session/new).
    pub fn from_wire(value: Option<&str>) -> Option<RetrievalMode> {
        match value {
            Some("local_browser") => Some(RetrievalMode::LocalBrowser),
            Some("framework_fallback") => Some(RetrievalMode::FrameworkFallback),
            Some("off") => Some(RetrievalMode::Off),
            _ => None,
        }
    }
}

/// GAP-RETRIEVAL-TOOLS: the capability probe result for the selected mode.
/// Never a silent fallback — `Unsupported`/`Degraded` record WHY a mode
/// cannot serve (e.g. local_browser automation not implemented in this slice;
/// web client not configured). `Available` is constructed by the web client
/// probe once a client is configured (S5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetrievalCapability {
    #[allow(dead_code)] // constructed by the S5 web-client probe
    Available,
    Unsupported(String),
    #[allow(dead_code)] // reserved for degraded transports (S5)
    Degraded(String),
}

impl RetrievalCapability {
    /// The `capability_status` value for the mode-transition payload.
    pub(crate) fn status_str(&self) -> &'static str {
        match self {
            RetrievalCapability::Available => "available",
            RetrievalCapability::Unsupported(_) => "unsupported",
            RetrievalCapability::Degraded(_) => "degraded",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use orz_assurance::{EventType, JournalRecorder, RunEvent};
    use std::sync::Arc;

    // ── GAP-DENIAL-POLICY-REVISION / goal wiring (2026-08-12) ──

    /// `update_goal` swaps the run-level goal digest and bumps the version;
    /// `set_goal_digest` (run start) resets the version — a fresh run
    /// starts a fresh goal epoch. The version/digest pair moves as one
    /// lock-protected snapshot.
    #[tokio::test]
    async fn goal_context_update_increments_version_and_digest() {
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())));
        controller.set_goal_digest("goal-1");
        let (d1, v0) = {
            let g = controller.goal_context.lock().unwrap();
            (g.digest.clone().unwrap(), g.version)
        };
        assert_eq!(v0, 0);
        assert_eq!(d1, AgentLoopController::goal_digest_of("goal-1"));
        controller.update_goal("goal-2");
        let g = controller.goal_context.lock().unwrap();
        assert_eq!(g.version, 1);
        assert_eq!(
            g.digest.as_deref(),
            Some(AgentLoopController::goal_digest_of("goal-2").as_str())
        );
        assert_ne!(g.digest.as_deref(), Some(d1.as_str()));
        drop(g);
        // Run start re-pins and resets the version to 0.
        controller.set_goal_digest("goal-3");
        let g = controller.goal_context.lock().unwrap();
        assert_eq!(g.version, 0);
        assert_eq!(
            g.digest.as_deref(),
            Some(AgentLoopController::goal_digest_of("goal-3").as_str())
        );
    }

    /// `bump_policy_revision` increments the live value; a fresh run resets
    /// it to 0 (same per-run window as the denial breaker).
    #[tokio::test]
    async fn policy_revision_bump_increments_and_run_resets() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("直接回答"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway);
        assert_eq!(controller.policy_revision(), 0);
        controller.bump_policy_revision();
        assert_eq!(controller.policy_revision(), 1);
        controller
            .run_turn(&host, "hi", "RUN-POL", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        assert_eq!(controller.policy_revision(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval mode authority (§3.7.1) ──

    /// mode=off (the default) refuses a retrieval dispatch with an explicit
    /// error — WITHOUT a ToolStarted (the verifier's mode rule forbids any
    /// retrieval dispatch after a transition to off; the refusal is the
    /// terminal ToolCompleted(error) alone). The tool projection also hides
    /// the retrieval family from the model's declarations.
    #[tokio::test]
    async fn mode_off_refuses_retrieval_dispatch() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        // Default controller — mode=off, no bootstrap transition.
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "查找文档", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let types = event_types(&dir);
        assert!(!types.contains(&EventType::ToolStarted), "{types:?}");
        let all_events = events(&dir);
        let refused: Vec<&RunEvent> = all_events
            .iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .collect();
        assert_eq!(refused.len(), 1, "{types:?}");
        assert_eq!(refused[0].payload["error"], "retrieval_mode_off");
        // No subagent side effect: the blackboard internal section stays empty.
        let r = controller.blackboard().read();
        assert!(r.internal_ret.project_docs.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The mode=off projection removes the retrieval family from the
    /// model-visible declarations (the model never sees the tools), and
    /// the tool_availability_check probe partition covers work tools only.
    #[tokio::test]
    async fn mode_off_removes_retrieval_tools_from_declarations() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("直接回答"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "hi", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let all_events = events(&dir);
        let availability = all_events
            .iter()
            .find(|e| e.event_type == EventType::ToolAvailabilityCheck)
            .unwrap();
        let p = &availability.payload;
        assert_eq!(p["probe_scope"], "main_agent_work_tools");
        assert_eq!(p["gate_decision"], "pass");
        let complete: Vec<&str> = p["complete"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        let incomplete: Vec<&str> = p["incomplete"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v["tool"].as_str()).collect())
            .unwrap_or_default();
        // The probe partition covers work tools only — the retrieval
        // dispatch family never appears in it (retrieval_disposition IS a
        // work tool and rides the partition with its own probe).
        for tool in complete.iter().chain(incomplete.iter()) {
            assert!(
                !crate::relay::is_retrieval_dispatch_name(tool)
                    && !crate::relay::is_retrieval_mode_gated_host_tool(tool),
                "{tool} in probe partition"
            );
        }
        // Declarations: the first model request hides the retrieval family
        // under mode=off.
        let received = fake.received_requests();
        let declared: Vec<&str> = received[0].tools.iter().map(|t| t.name.as_str()).collect();
        for tool in [
            "retrieve_project_docs",
            "retrieve_project_source_ledger",
            "web_search",
            "web_fetch",
            // H1 (review 2026-08-10): the host-routed internal retrieval
            // tool is hidden too — off means no retrieval tools at all.
            "project_doc_index",
        ] {
            assert!(
                !declared.iter().any(|t| *t == tool),
                "{tool} in {declared:?}"
            );
        }
        // P0-A-2: the disposition control tool is a work tool like any
        // other — it stays only while a live activation exists (probe
        // `检索会话未激活` otherwise removes it). This test has no
        // activation, so it is NOT declared.
        assert!(
            !declared.iter().any(|t| *t == "retrieval_disposition"),
            "{declared:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A session bootstrap with an explicit mode journals exactly one
    /// `retrieval_mode_transition` (old=off, new=mode, authority/session_
    /// bootstrap, capability) on the run's startup, BEFORE the availability
    /// gate. A second run under the same controller does not repeat it.
    #[tokio::test]
    async fn bootstrap_transition_journaled_once_before_availability() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-1")]),
            ScriptedResponse::text("[SOURCE] x.com\n完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway))
            .with_retrieval_mode(
                RetrievalMode::FrameworkFallback,
                RetrievalCapability::Unsupported("web_client_not_configured".to_string()),
                true,
                Some("sess-test12345".to_string()),
                None,
                None,
            );
        controller
            .run_turn(&host, "查", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let transitions: Vec<&RunEvent> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalModeTransition)
            .collect();
        assert_eq!(transitions.len(), 1);
        let p = &transitions[0].payload;
        assert_eq!(p["old_mode"], "off");
        assert_eq!(p["new_mode"], "framework_fallback");
        assert_eq!(p["authority"], "session_bootstrap");
        assert_eq!(p["reason_code"], "session_default");
        assert_eq!(p["capability_status"], "unsupported");
        assert_eq!(p["session_id"], "sess-test12345");
        // The transition precedes the availability gate.
        let t_index = events
            .iter()
            .position(|e| e.event_type == EventType::RetrievalModeTransition)
            .unwrap();
        let a_index = events
            .iter()
            .position(|e| e.event_type == EventType::ToolAvailabilityCheck)
            .unwrap();
        assert!(t_index < a_index);
        // Pending cleared after the journal.
        assert!(controller.bootstrap_transition_journaled());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：模式 A
    /// 自动降级 transition 携带机械元数据——authority=mechanical_probe、
    /// reason_code=browser_launch_failed（local_browser probe 失败 →
    /// framework_fallback），old_mode=local_browser。
    #[tokio::test]
    async fn mode_a_degrade_transition_carries_mechanical_metadata() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-1")]),
            ScriptedResponse::text("[SOURCE] x.com\n完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway))
            .with_retrieval_mode(
                RetrievalMode::FrameworkFallback,
                RetrievalCapability::Available,
                true,
                Some("sess-mode-a".to_string()),
                Some(RetrievalMode::LocalBrowser),
                Some((
                    "mechanical_probe".to_string(),
                    "browser_launch_failed".to_string(),
                )),
            );
        controller
            .run_turn(&host, "查", "RUN-MODEA", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let transitions: Vec<&RunEvent> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalModeTransition)
            .collect();
        assert_eq!(transitions.len(), 1);
        let p = &transitions[0].payload;
        assert_eq!(p["old_mode"], "local_browser");
        assert_eq!(p["new_mode"], "framework_fallback");
        assert_eq!(p["authority"], "mechanical_probe");
        assert_eq!(p["reason_code"], "browser_launch_failed");
        assert_eq!(p["capability_status"], "available");
        assert_eq!(p["session_id"], "sess-mode-a");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// M4 (review 2026-08-10): an explicit change TO off is a transition
    /// like any other — it journals with the REAL persisted old_mode (never
    /// a hardcoded "off") and a null capability_status (schema allOf).
    #[tokio::test]
    async fn explicit_change_to_off_journals_real_old_mode() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("直接回答"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_retrieval_mode(
            RetrievalMode::Off,
            RetrievalCapability::Unsupported("off".to_string()),
            true,
            Some("sess-test12345".to_string()),
            Some(RetrievalMode::FrameworkFallback),
            None,
        );
        controller
            .run_turn(&host, "hi", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let all_events = events(&dir);
        let transitions: Vec<&RunEvent> = all_events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalModeTransition)
            .collect();
        assert_eq!(transitions.len(), 1, "{transitions:?}");
        assert_eq!(transitions[0].payload["old_mode"], "framework_fallback");
        assert_eq!(transitions[0].payload["new_mode"], "off");
        assert!(transitions[0].payload["capability_status"].is_null());
        assert_eq!(transitions[0].payload["authority"], "session_bootstrap");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// mode=local_browser with an unsupported capability fails EVERY
    /// retrieval dispatch explicitly (ToolStarted → ToolCompleted(error)),
    /// never degrading silently to the framework tools.
    #[tokio::test]
    async fn local_browser_unsupported_fails_explicitly() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-1")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_retrieval_mode(
            RetrievalMode::LocalBrowser,
            RetrievalCapability::Unsupported(
                "local_browser_automation_not_implemented".to_string(),
            ),
            true,
            None,
            None,
            None,
        );
        controller
            .run_turn(&host, "查", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let all_events = events(&dir);
        let refused: Vec<&RunEvent> = all_events
            .iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .collect();
        assert_eq!(refused.len(), 1);
        assert_eq!(
            refused[0].payload["error"],
            "retrieval_capability_unavailable"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// local_browser (2026-08-10): `browser_read` is mode-gated — under
    /// framework_fallback (the web-tool lane) it is refused with
    /// `retrieval_mode_requires_local_browser`, never silently falling back
    /// to web tools (ADR-0010 §3.7.1).
    #[tokio::test]
    async fn framework_fallback_refuses_browser_read() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("browser_read", "call-1")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "读网页", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let types = event_types(&dir);
        assert!(!types.contains(&EventType::ToolStarted), "{types:?}");
        let all_events = events(&dir);
        let refused: Vec<&RunEvent> = all_events
            .iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .collect();
        assert_eq!(refused.len(), 1);
        assert_eq!(
            refused[0].payload["error"],
            "retrieval_mode_requires_local_browser"
        );
        // P0-C S3 前置审查修复 (F1/F3): refusal completions must stay
        // self-describing — non-zero exit_code + status=error + structured
        // denial (the Python verifier cross-check locks this shape).
        assert_eq!(refused[0].payload["exit_code"], serde_json::json!(1));
        assert_eq!(refused[0].payload["status"], serde_json::json!("error"));
        assert_eq!(
            refused[0].payload["policy_denial"]["source"],
            serde_json::json!("retrieval_mode")
        );
        assert_eq!(
            refused[0].payload["policy_denial"]["code"],
            serde_json::json!("retrieval_mode_requires_local_browser")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
