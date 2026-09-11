//! Pending-round mechanism shared by the MAIN lane's pause points
//! (ADR-0010 §4.2/§14.16 + §14.17⑱; P2-11 DC 清理 2026-08-31 拆分).
//!
//! After the Diagnostic Coverage forced-template round was removed
//! (MODEL-RESIDUAL-PRESSURE-FOLLOWUP 裁决 2, 2026-08-31), the shared
//! checkpoint machinery keeps exactly two pending rounds:
//!
//! - `Orientation` — the soft gate (THIN-HARNESS-REDESIGN-V2 §9.2): the
//!   fire event is journaled at the safe gap, the orientation block is
//!   injected, and the NEXT model round is a normal round (tools stay
//!   offered). A pure-text answer is consumed and the loop continues; a
//!   tool round commits and falls through to the normal dispatch path.
//!   No `checkpoint_response` event is produced (no template to verify;
//!   the fire event + the following model_output/tool events form the
//!   audit chain).
//!
//! - `ConsoleModeInquiry` — the console dual-mode explicit inquiry round
//!   (PLAN-FIRST 阶段 C, ADR-0010 §14.17⑱ / 设计 §7.3): a tool-free round
//!   after the assistant-side failure streak; the model answers the
//!   `{"decision": "switch"|"stay", "reason": "…"}` template. One re-fill,
//!   then degrade to stay. The decision is journaled by the transition
//!   event; the mode state is updated by the caller (agent_loop).

use crate::orientation::{OrientationFireRecord, OrientationSessionState};

/// A pending checkpoint — the fire event was journaled and the block
/// injected; the pending round has not completed yet.
#[derive(Debug, Clone)]
pub(crate) enum PendingCheckpoint {
    Orientation {
        record: OrientationFireRecord,
    },
    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.3):
    /// console 双模式显式询问轮——3 连败助理层故障面触发后的无工具轮；
    /// 模型只回答 `{"decision": "switch"|"stay", "reason": "…"}` 模板。
    /// 优先级低于 orientation（触发点同 gap，orientation > 本项）。
    ConsoleModeInquiry {
        attempt: u32,
        streak: u32,
        order_ids: Vec<String>,
    },
}

impl PendingCheckpoint {
    pub(crate) fn with_attempt(&self, attempt: u32) -> Self {
        match self {
            PendingCheckpoint::Orientation { record } => PendingCheckpoint::Orientation {
                record: record.clone(),
            },
            PendingCheckpoint::ConsoleModeInquiry {
                streak, order_ids, ..
            } => PendingCheckpoint::ConsoleModeInquiry {
                attempt,
                streak: *streak,
                order_ids: order_ids.clone(),
            },
        }
    }
}

/// Commit the pending fire AFTER the pending round completed. A `None`
/// orientation state (grill/one-shot) is a no-op for the orientation lane.
pub(crate) fn commit_pending(
    pending: PendingCheckpoint,
    orientation: Option<&mut OrientationSessionState>,
) {
    match pending {
        PendingCheckpoint::Orientation { record } => {
            if let Some(state) = orientation {
                // P0-0x S1: two fire shapes share this pending round —
                // the one-shot initial-round inquiry (sets its flag only,
                // never resets the periodic counter) and the periodic
                // threshold inquiry (resets the counter). The `trigger`
                // value on the record is the dispatcher.
                if record.is_initial_round() {
                    state.commit_initial_round_fire(record.agent_role, &record);
                } else {
                    state.commit_fire(record.agent_role, &record);
                }
            }
        }
        // Console inquiry commits nothing to orientation state — the
        // decision is journaled by the transition event (switch/stay) and
        // the mode state is updated by the caller (agent_loop).
        PendingCheckpoint::ConsoleModeInquiry { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orientation::AgentRole;

    #[test]
    fn console_inquiry_tracks_attempt_and_preserves_streak() {
        let p = PendingCheckpoint::ConsoleModeInquiry {
            attempt: 1,
            streak: 3,
            order_ids: vec!["ORD-1".to_string()],
        };
        let p2 = p.with_attempt(2);
        match &p2 {
            PendingCheckpoint::ConsoleModeInquiry {
                attempt,
                streak,
                order_ids,
                ..
            } => {
                assert_eq!(*attempt, 2);
                assert_eq!(*streak, 3);
                assert_eq!(order_ids, &vec!["ORD-1".to_string()]);
            }
            _ => panic!("variant changed"),
        }
    }

    #[test]
    fn commit_pending_orientation_is_noop_without_state() {
        let mut orientation = OrientationSessionState::new_with_threshold("sess-2", 7);
        for _ in 0..7 {
            orientation.feed_round(AgentRole::Main);
        }
        let rec = orientation
            .build_fire_record(AgentRole::Main, "RUN-2", "loop_top")
            .unwrap();
        commit_pending(PendingCheckpoint::Orientation { record: rec }, None);
        // No state → no crash, no commit.
    }
}
