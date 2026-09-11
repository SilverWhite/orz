//! Neutral orientation checkpoint — construction + response verification.
//!
//! Ported from Python `assurance/orientation_runtime_guard.py`:
//! `build_orientation_checkpoint` / `build_tool_availability_infused_orientation_checkpoint`
//! / `verify_orientation_response`. Receipt/context jsonschema validation stays on
//! the Python side (conformance suite + schema authority).

/// Soft-gate orientation prompt (THIN-HARNESS-REDESIGN-V2 §9.2,
/// 2026-08-29 用户裁决) — 简短方向检查，非强制模板、不打断动作：
/// 模型可简要回答后继续，也可直接继续动作；纯文本回答由 loop 消费后
/// 明确续跑，终答仍只由模型自发。
///
/// GAP-INQUIRY-SPLIT (2026-08-09): re-tagged `[ORIENTATION v0.2]` — the v0.2
/// producer actually injects the block (the old monitor wrote the event but
/// never injected it), so the marker must match the `ORIENTATION_INJECTED_PREFIX`
/// registration in the injected-block filter. v0.3 (2026-08-15) carried the
/// forced-template JSON answer contract; v0.4 (2026-08-29) 软门——移除
/// "只输出 JSON 模板 / 不要调用任何工具"措辞，恢复简短方向检查三问。
pub const ORIENTATION_BLOCK: &str = concat!(
    "[ORIENTATION v0.4]\n",
    "方向检查（非强制模板，不打断动作）：\n",
    "1. 当前正在做什么？\n",
    "2. 当前任务定位是什么？\n",
    "3. 下一步输出应该服务哪个用户目标？\n",
    "\n[/ORIENTATION]"
);

/// 初始轮中立问询块（P0-0x / ADR-0010 §14.66，2026-09-11 设计定稿）：
/// **一次性**开局问询，只在**首个含工具调用的动作批次结束**
/// （`post_tool_batch_gap`）由机械层独立注入一次——不携带机械审查报告
/// （审查依旧是结尾的事），不与周期 `ORIENTATION_BLOCK` 共用文本。
///
/// 三问取自用户口径原文（设计 §3.3）：只请求模型**想一次**
/// （口径 / 方向 / 做法评估），不校验回答、不因回答触发额外轮次。
/// 措辞纪律：不设结构化字段、不用追责式提问（首版草稿的
/// 「有没有更短路径？为什么没选它？」已否——属动作中的回查与质疑）。
///
/// 注入块前缀 `[INITIAL_ROUND_INQUIRY` 注册进 `prompt::is_injected_block_text`
/// （机械注入文本，绝不持久化回会话，与 `ORIENTATION_BLOCK` 同待遇）。
pub const INITIAL_ROUND_INQUIRY_BLOCK: &str = concat!(
    "[INITIAL_ROUND_INQUIRY v0.1]\n",
    "开局问询（一次性，非强制模板，不打断动作）：\n",
    "1. 本任务实际要交付什么、会被按什么判定？\n",
    "2. 大方向是什么？当前处在什么阶段、下一步要解决什么？\n",
    "3. 当前做法优劣如何？你对任务有何评估？\n",
    "[/INITIAL_ROUND_INQUIRY]"
);

/// Checklist context prefix template (Python `CHECKLIST_CONTEXT_TEMPLATE`).
pub const CHECKLIST_CONTEXT_TEMPLATE: &str = "[CHECKLIST_CONTEXT v0.1]\n\
当前步骤: {step_id} — {title}\n\
任务位置: 步骤 {position} / 共 {total} 项\n\
步骤状态: {status}\n\
[/CHECKLIST_CONTEXT]";

/// Allowed response fields (Python `ALLOWED_ORIENTATION_FIELDS`).
pub const ALLOWED_ORIENTATION_FIELDS: [&str; 4] = [
    "orientation_summary",
    "current_task_position",
    "next_output_target",
    "available_tools_acknowledged",
];

/// Forbidden response fields (Python `FORBIDDEN_ORIENTATION_FIELDS`).
pub const FORBIDDEN_ORIENTATION_FIELDS: [&str; 5] = [
    "counterexample_candidate",
    "claim_disposition",
    "claim_promotion",
    "negative_fixture_candidate",
    "tool_belief_mismatch_statement",
];

/// Checkpoint trigger — schema enum `["fixed_step_interval", "pre_handoff"]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointTrigger {
    FixedStepInterval { step_index: u64 },
    PreHandoff { step_index: u64 },
}

impl CheckpointTrigger {
    pub fn trigger_type(&self) -> &'static str {
        match self {
            CheckpointTrigger::FixedStepInterval { .. } => "fixed_step_interval",
            CheckpointTrigger::PreHandoff { .. } => "pre_handoff",
        }
    }

    pub fn step_index(&self) -> u64 {
        match self {
            CheckpointTrigger::FixedStepInterval { step_index }
            | CheckpointTrigger::PreHandoff { step_index } => *step_index,
        }
    }
}

/// Neutral checklist positional context (Python `checklist_context` dict).
#[derive(Debug, Clone)]
pub struct ChecklistContext {
    pub step_id: String,
    pub title: String,
    pub position: u64,
    pub total: u64,
    pub status: String,
}

/// Claim policy — constant in Python; kept as a struct for schema parity.
#[derive(Debug, Clone)]
pub struct ClaimPolicy {
    pub may_generate_counterexample_candidate: bool,
    pub may_set_claim_disposition: bool,
    pub claim_strength_effect: &'static str,
}

impl Default for ClaimPolicy {
    fn default() -> Self {
        Self {
            may_generate_counterexample_candidate: false,
            may_set_claim_disposition: false,
            claim_strength_effect: "none",
        }
    }
}

/// A built orientation checkpoint.
#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub checkpoint_id: String,
    pub task_id: String,
    pub trigger: CheckpointTrigger,
    pub message_block: String,
    pub allowed_fields: [&'static str; 4],
    pub forbidden_fields: [&'static str; 5],
    pub claim_policy: ClaimPolicy,
    pub task_contract_sha256: Option<String>,
    pub tool_availability_sha256: Option<String>,
}

impl Checkpoint {
    pub fn checkpoint_id(&self) -> &str {
        &self.checkpoint_id
    }

    pub fn message_block(&self) -> &str {
        &self.message_block
    }
}

/// Build a checkpoint (Python `build_orientation_checkpoint`).
///
/// `checkpoint_id` = `ORIENT-{task_id}-{step:04d}`. Step is `u64` so negative
/// steps (Python `AssuranceError`) are unrepresentable.
pub fn build_checkpoint(
    task_id: &str,
    trigger: CheckpointTrigger,
    task_contract_sha256: Option<String>,
    tool_availability_sha256: Option<String>,
    checklist_context: Option<&ChecklistContext>,
) -> Checkpoint {
    let message_block = match checklist_context {
        Some(ctx) => {
            let ctx_block = CHECKLIST_CONTEXT_TEMPLATE
                .replace("{step_id}", &ctx.step_id)
                .replace("{title}", &ctx.title)
                .replace("{position}", &ctx.position.to_string())
                .replace("{total}", &ctx.total.to_string())
                .replace("{status}", &ctx.status);
            format!("{ctx_block}\n\n{ORIENTATION_BLOCK}")
        }
        None => ORIENTATION_BLOCK.to_string(),
    };

    Checkpoint {
        checkpoint_id: format!("ORIENT-{task_id}-{:04}", trigger.step_index()),
        task_id: task_id.to_string(),
        trigger,
        message_block,
        allowed_fields: ALLOWED_ORIENTATION_FIELDS,
        forbidden_fields: FORBIDDEN_ORIENTATION_FIELDS,
        claim_policy: ClaimPolicy::default(),
        task_contract_sha256,
        tool_availability_sha256,
    }
}

/// Infuse a tool-availability context block ahead of the orientation block
/// (Python `build_tool_availability_infused_orientation_block`).
pub fn infused_message_block(tool_availability_context_block: &str) -> String {
    format!("{tool_availability_context_block}\n\n{ORIENTATION_BLOCK}")
}

/// Verify a structured orientation response (Python `verify_orientation_response`).
///
/// The response is a set of field names (keys of the model's structured
/// response dict). Valid iff no forbidden field is present and no field outside
/// the allowed ∪ forbidden vocabulary is present.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CheckpointVerification {
    pub valid: bool,
    pub allowed_observed: Vec<String>,
    pub forbidden_observed: Vec<String>,
    pub unexpected_observed: Vec<String>,
}

pub fn verify_orientation_response(
    checkpoint: &Checkpoint,
    response_fields: &[String],
) -> CheckpointVerification {
    let mut allowed_observed: Vec<String> = Vec::new();
    let mut forbidden_observed: Vec<String> = Vec::new();
    let mut unexpected_observed: Vec<String> = Vec::new();

    for field in response_fields {
        if checkpoint.allowed_fields.contains(&field.as_str()) {
            allowed_observed.push(field.clone());
        } else if checkpoint.forbidden_fields.contains(&field.as_str()) {
            forbidden_observed.push(field.clone());
        } else {
            unexpected_observed.push(field.clone());
        }
    }

    allowed_observed.sort();
    forbidden_observed.sort();
    unexpected_observed.sort();

    CheckpointVerification {
        valid: forbidden_observed.is_empty() && unexpected_observed.is_empty(),
        allowed_observed,
        forbidden_observed,
        unexpected_observed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cp(task_id: &str, step: u64) -> Checkpoint {
        build_checkpoint(
            task_id,
            CheckpointTrigger::FixedStepInterval { step_index: step },
            None,
            None,
            None,
        )
    }

    #[test]
    fn checkpoint_id_format() {
        let c = cp("TASK", 1);
        assert_eq!(c.checkpoint_id, "ORIENT-TASK-0001");
        let c = cp("RUN-ABC123", 42);
        assert_eq!(c.checkpoint_id, "ORIENT-RUN-ABC123-0042");
    }

    #[test]
    fn message_block_carries_soft_direction_check() {
        let c = cp("TASK", 0);
        // THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29)：v0.4 软门——简短
        // 方向检查三问，无强制模板措辞、无工具禁令。
        assert!(c.message_block.starts_with("[ORIENTATION v0.4]"));
        assert!(c.message_block.ends_with("[/ORIENTATION]"));
        assert!(c.message_block.starts_with("[ORIENTATION"));
        for question in [
            "当前正在做什么",
            "当前任务定位是什么",
            "下一步输出应该服务哪个用户目标",
        ] {
            assert!(
                c.message_block.contains(question),
                "direction-check question missing from block: {question}"
            );
        }
        // 已退役的强制模板措辞不得回潮。
        for retired in [
            "只输出",
            "JSON 模板",
            "不要调用任何工具",
            "请暂停动作",
            "task_position",
            "progress_evidence",
            "next_action",
            "gather_evidence",
        ] {
            assert!(
                !c.message_block.contains(retired),
                "retired forced-template wording must not return: {retired}"
            );
        }
    }

    #[test]
    fn forbidden_fields_absent_from_block() {
        let c = cp("TASK", 0);
        for field in FORBIDDEN_ORIENTATION_FIELDS {
            assert!(
                !c.message_block.contains(field),
                "forbidden field leaked into message block: {field}"
            );
        }
    }

    /// P0-0x S1: 初始轮中立问询块形状——独立版本标识、三问原文、可识别的
    /// 注入前缀；且与周期 `ORIENTATION_BLOCK` 互不包含（两块必须能各自
    /// 被 `is_injected_block_text` 前缀登记区分）。
    #[test]
    fn initial_round_inquiry_block_shape() {
        assert!(INITIAL_ROUND_INQUIRY_BLOCK.starts_with("[INITIAL_ROUND_INQUIRY v0.1]"));
        assert!(INITIAL_ROUND_INQUIRY_BLOCK.ends_with("[/INITIAL_ROUND_INQUIRY]"));
        for question in [
            "本任务实际要交付什么、会被按什么判定？",
            "大方向是什么？当前处在什么阶段、下一步要解决什么？",
            "当前做法优劣如何？你对任务有何评估？",
        ] {
            assert!(
                INITIAL_ROUND_INQUIRY_BLOCK.contains(question),
                "initial-round question missing from block: {question}"
            );
        }
        // 一次性软门——不得出现强制模板 / 工具禁令 / 追责式措辞。
        for retired in [
            "只输出",
            "JSON",
            "不要调用任何工具",
            "请暂停动作",
            "为什么没有选",
            "为什么没选",
        ] {
            assert!(
                !INITIAL_ROUND_INQUIRY_BLOCK.contains(retired),
                "forbidden initial-round wording must not appear: {retired}"
            );
        }
        // 两块互不包含：周期问询的三问不得泄漏进初始轮块（反之亦然，
        // 由文本整体不相等 + 各自版本标识锁定）。
        for q in [
            "当前正在做什么？",
            "当前任务定位是什么？",
            "下一步输出应该服务哪个用户目标？",
        ] {
            assert!(
                !INITIAL_ROUND_INQUIRY_BLOCK.contains(q),
                "periodic question leaked into the initial-round block: {q}"
            );
        }
        assert_ne!(ORIENTATION_BLOCK, INITIAL_ROUND_INQUIRY_BLOCK);
        // 注入块过滤按前缀匹配——初始轮块不得被周期前缀命中，反之亦然。
        assert!(!INITIAL_ROUND_INQUIRY_BLOCK.starts_with("[ORIENTATION"));
        assert!(!ORIENTATION_BLOCK.starts_with("[INITIAL_ROUND_INQUIRY"));
    }

    #[test]
    fn pre_handoff_trigger_shape() {
        let c = build_checkpoint(
            "TASK",
            CheckpointTrigger::PreHandoff { step_index: 7 },
            None,
            None,
            None,
        );
        assert_eq!(c.trigger.trigger_type(), "pre_handoff");
        assert_eq!(c.trigger.step_index(), 7);
        assert_eq!(c.checkpoint_id, "ORIENT-TASK-0007");
    }

    #[test]
    fn checklist_context_injected_before_orientation() {
        let ctx = ChecklistContext {
            step_id: "S3".to_string(),
            title: "验证证据链".to_string(),
            position: 3,
            total: 5,
            status: "in_progress".to_string(),
        };
        let c = build_checkpoint(
            "TASK",
            CheckpointTrigger::FixedStepInterval { step_index: 1 },
            None,
            None,
            Some(&ctx),
        );
        assert!(c.message_block.starts_with("[CHECKLIST_CONTEXT v0.1]"));
        assert!(c.message_block.contains("当前步骤: S3 — 验证证据链"));
        assert!(c.message_block.contains("步骤 3 / 共 5 项"));
    }

    #[test]
    fn verification_accepts_allowed_fields_only() {
        let c = cp("TASK", 0);
        let fields = vec![
            "orientation_summary".to_string(),
            "current_task_position".to_string(),
        ];
        let v = verify_orientation_response(&c, &fields);
        assert!(v.valid);
        assert_eq!(v.allowed_observed.len(), 2);
    }

    #[test]
    fn verification_rejects_forbidden_field() {
        let c = cp("TASK", 0);
        let fields = vec!["counterexample_candidate".to_string()];
        let v = verify_orientation_response(&c, &fields);
        assert!(!v.valid);
        assert_eq!(v.forbidden_observed, vec!["counterexample_candidate"]);
    }

    #[test]
    fn verification_rejects_unexpected_field() {
        let c = cp("TASK", 0);
        let fields = vec!["some_unknown_field".to_string()];
        let v = verify_orientation_response(&c, &fields);
        assert!(!v.valid);
        assert_eq!(v.unexpected_observed, vec!["some_unknown_field"]);
    }
}
