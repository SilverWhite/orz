//! 候选计数门：`candidate_gate`（agent_candidates 注入判定）与拒绝路径 `refuse_candidate`。
//! 0ai (2026-09-16) 拆分自 `host_exec.rs`（机械搬移，行为不变）。

use super::ToolFailureOutcome;
use crate::controller::{
    AgentLoopController, AgentLoopError, CandidateGateDecision, DenialKey, EventWriter,
    PolicyFeedback, candidate_tool_prefix,
};
use crate::gateway::model::{Message, Role, ToolCall};
use crate::host::ToolResult;
use orz_assurance::EventType;
use std::sync::Mutex;

impl AgentLoopController {
    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): candidate count gate
    /// — count domain lookup, exact-string URL dedup and cap check (design
    /// §1), shared by the web_fetch family and `browser_read` (local_browser
    /// second segment). Runs BEFORE any fetch/read action and BEFORE
    /// ToolStarted / ACAF ticketing (a refused call needs no ticket). The
    /// decision is consumption-free: the caller commits the URL at the
    /// execution boundary after the permission/ACAF gates pass (review fix
    /// 2026-08-14).
    ///
    /// Fail-closed arms (per tool family, stable `{family}_candidate_*`
    /// codes):
    /// - no count domain (main/grill lane — retrieval tools never execute
    ///   there; belt-and-braces): `{family}_candidate_count_unbound`;
    /// - missing `url` argument (no count identity):
    ///   `{family}_candidate_url_missing`;
    /// - new URL at/over the cap: `{family}_candidate_cap_exceeded` —
    ///   no ToolStarted, neutral statement, Denied feedback (the
    ///   consecutive-denial breaker gives no retry space, ADR-0010
    ///   §3.5.4).
    pub(crate) async fn candidate_gate(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tc: &ToolCall,
        fetch_candidates: Option<&Mutex<Vec<String>>>,
    ) -> Result<CandidateGateDecision, AgentLoopError> {
        let prefix = candidate_tool_prefix(&tc.name);
        let Some(counter) = fetch_candidates else {
            return self
                .refuse_candidate(
                    writer,
                    messages,
                    tc,
                    &format!("{prefix}_candidate_count_unbound"),
                    &format!("{prefix} 已拒绝 — 候选核验计数域不可用"),
                    None,
                    false,
                )
                .await;
        };
        let Some(url) = tc
            .arguments
            .get("url")
            .and_then(|u| u.as_str())
            .map(str::to_string)
        else {
            return self
                .refuse_candidate(
                    writer,
                    messages,
                    tc,
                    &format!("{prefix}_candidate_url_missing"),
                    &format!("{prefix} 已拒绝 — 缺少 url 参数，候选核验无法计数"),
                    None,
                    true,
                )
                .await;
        };
        let cap = self.candidate_cap as usize;
        // 0k 审查处理 (P2-2, 2026-08-30)：决策+预留原子化——锁内检查 cap
        // 并立即占位，消除并行批次下「决策/提交分离」的竞态（两个调用
        // 基于同一旧计数同时通过 → 硬 cap 超限最多 +批次大小）。后续
        // permission/ACAF 门拒绝时由调用方 `rollback_candidate` 回滚，
        // 保持「被权限/票据拒绝的调用不消耗候选」语义（review fix
        // 2026-08-14 不变）；`commit_candidate` 对已预留 url 为去重幂等
        // （返回计数）。The std MutexGuard must not cross the async
        // refusal below (Send).
        let outcome = {
            let mut seen = counter.lock().unwrap_or_else(|e| e.into_inner());
            let count = seen.len();
            let is_new = !seen.iter().any(|u| u == &url);
            if is_new && count >= cap {
                Err((count, cap))
            } else {
                if is_new {
                    seen.push(url.clone());
                }
                Ok(())
            }
        };
        match outcome {
            Ok(()) => Ok(CandidateGateDecision::Allowed { url, cap }),
            Err((count, cap)) => {
                self.refuse_candidate(
                    writer,
                    messages,
                    tc,
                    &format!("{prefix}_candidate_cap_exceeded"),
                    &format!("{prefix} 已拒绝 — 候选核验数量已达上限 {cap}（当前 {count}/{cap}）"),
                    Some((count, cap)),
                    true,
                )
                .await
            }
        }
    }

    /// Shared no-ToolStarted refusal for the candidate gate — event +
    /// neutral tool message + Denied feedback (the breaker aggregates at
    /// round granularity and blocks repeated refusals).
    async fn refuse_candidate(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tc: &ToolCall,
        code: &str,
        msg: &str,
        counts: Option<(usize, usize)>,
        lane: bool,
    ) -> Result<CandidateGateDecision, AgentLoopError> {
        let mut payload = serde_json::json!({
            "tool": tc.name,
            "call_id": tc.call_id,
            "status": "error",
            "error": code,
        });
        // 0q（ADR-0010 §14.63）：原「P2-10 F4 身份挂载 + P2-12 写时盖章」
        // 散布写点退役——语义由单一漏斗等价覆盖（写点 ④：候选门拒单，
        // `{family}_candidate_*` 码族）。
        self.stamp_failure(
            &mut payload,
            &tc.name,
            &tc.arguments,
            ToolFailureOutcome::Refused(code),
        );
        // Only lane refusals carry the dispatch target: `count_unbound`
        // fires in a lane with no count domain (main/grill belt-and-braces),
        // where no dispatch occurred (review fix 2026-08-14).
        if lane {
            payload["target"] = serde_json::json!("external_retrieval");
        }
        if let Some((count, cap)) = counts {
            payload["candidate_count"] = serde_json::json!(count);
            payload["candidate_cap"] = serde_json::json!(cap);
        }
        // P2-10 R2 (2026-08-31): candidate-gate refusal = deny event.
        // 0am 审查处置（2026-10-03）：拒绝码喂入点解析 → GateGuard
        // （`*_candidate_*` 码族由 `DenyClass::of_code` 判定）。
        self.feed_lif_deny(None, code);
        writer.record(EventType::ToolCompleted, payload).await?;
        messages.push(Message {
            role: Role::Tool,
            content: msg.to_string(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        Ok(CandidateGateDecision::Refused(
            ToolResult {
                output: msg.to_string(),
                exit_code: Some(1),
                output_encoding: None,
                // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24)：拒绝信封带
                // 结构化错误码与计数——机械审查层据此精确识别候选超限/
                // 计数域未绑定异常，而非靠剩余池近似。
                structured: {
                    let mut s = serde_json::json!({ "error": code });
                    if let Some((count, cap)) = counts {
                        s["candidate_count"] = serde_json::json!(count);
                        s["candidate_cap"] = serde_json::json!(cap);
                    }
                    Some(s)
                },
                ..Default::default()
            },
            Some(PolicyFeedback::Denied(DenialKey {
                tool_name: tc.name.clone(),
                reason_code: code.to_string(),
                policy_revision: self.policy_revision(),
            })),
        ))
    }
}
