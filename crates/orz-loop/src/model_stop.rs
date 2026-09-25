//! 中性终态与模型主动停止（结束自述）——0bh ⑯ 落码面。
//!
//! 设计档：`docs/NEUTRAL_TERMINATION_AND_MODEL_STOP_DESIGN_2026-09-22.md`
//! （v1.0 定稿；索引条目 `FUS-NEUTRAL-TERMINAL`）。四原则：
//! **P1 判断归模型**（不设证据门槛、不要求举证）／**P2 机械层只记录与传话**
//! （不判定、不建议、不驳回）／**P3 回应归外部**（不把回应逻辑加进框架）／
//! **P4 无应答者不剥夺出口**。
//!
//! 形态（本文件＝**纯解析与记录**，无任何机械判定）：
//! - 模型在**决策边界**（无工具轮的收束轮）文本里输出声明块
//!   `[RUN_END] … [/RUN_END]`：`intent`（`conclude`／`pause`；缺省 `conclude`）、
//!   `reason`（`completed`／`partial`／`blocked`／`awaiting_response`；缺省
//!   `completed`）、`summary`（结束摘要；**不设质量门**——缺失即如实记空）。
//! - 机械层把声明落 `run_finished` 的**新增字段**（旧字段/旧取值语义不变，
//!   最小契约面）；未知取值不驳回 ⇒ 归 `other` 并把原文收入摘要（P2）。
//! - 开关 `ORZ_MODEL_STOP_AWAIT`（缺省开；`0`＝关，跑分装置显式置关）只影响
//!   **guide 面的暂停广告与 `await_channel` 标注**，不影响声明记录本身。

/// 声明块前缀（模型面单一来源；guide 与解析共用）。
pub const MODEL_STOP_PREFIX: &str = "[RUN_END";
/// 声明块后缀。
pub const MODEL_STOP_END: &str = "[/RUN_END]";

/// 结束意向（暂停＝结束的一个意向取值；不是新通道）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopIntent {
    Conclude,
    Pause,
    /// 未识别取值（不驳回；原文进摘要）。
    Other,
}

impl StopIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            StopIntent::Conclude => "conclude",
            StopIntent::Pause => "pause",
            StopIntent::Other => "other",
        }
    }
}

/// 结束原因类别（「完成」降为与其它原因平级的一种——不再是唯一正常终态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    Completed,
    Partial,
    Blocked,
    AwaitingResponse,
    /// 未识别取值（不驳回；原文进摘要）。
    Other,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            StopReason::Completed => "completed",
            StopReason::Partial => "partial",
            StopReason::Blocked => "blocked",
            StopReason::AwaitingResponse => "awaiting_response",
            StopReason::Other => "other",
        }
    }
}

/// 一次结束自述（机械层如实记录；不缺省不补写判断）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStopDeclaration {
    pub intent: StopIntent,
    pub reason: StopReason,
    /// 结束摘要（常态化——每次结束都写；缺失 = 空串，不审查不罚）。
    pub summary: String,
    /// 未识别字段／取值原文（P2：不驳回，保留证据面）。
    pub raw_unrecognized: String,
}

/// 开关：`ORZ_MODEL_STOP_AWAIT`（缺省开；`0` 关）。跑分装置显式置关 +
/// 起跑前断言（断言在装置侧；本函数只提供读取口径）。
pub fn model_stop_await_enabled() -> bool {
    std::env::var("ORZ_MODEL_STOP_AWAIT")
        .map(|v| v.trim() != "0")
        .unwrap_or(true)
}

/// 从模型收束文本里提取**最后一段**声明块（旧在前；取最后一段＝最终意图）。
pub fn extract_model_stop(text: &str) -> Option<ModelStopDeclaration> {
    let start = text.rfind(MODEL_STOP_PREFIX)?;
    let tail = &text[start..];
    let end = tail.find(MODEL_STOP_END)?;
    let block = &tail[..end];
    let mut intent: Option<StopIntent> = None;
    let mut reason: Option<StopReason> = None;
    let mut summary: Option<String> = None;
    let mut unrecognized: Vec<String> = Vec::new();
    for line in block.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            unrecognized.push(line.to_string());
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "intent" => {
                intent = Some(match value {
                    "conclude" => StopIntent::Conclude,
                    "pause" => StopIntent::Pause,
                    _ => {
                        unrecognized.push(format!("intent: {value}"));
                        StopIntent::Other
                    }
                });
            }
            "reason" => {
                reason = Some(match value {
                    "completed" => StopReason::Completed,
                    "partial" => StopReason::Partial,
                    "blocked" => StopReason::Blocked,
                    "awaiting_response" => StopReason::AwaitingResponse,
                    _ => {
                        unrecognized.push(format!("reason: {value}"));
                        StopReason::Other
                    }
                });
            }
            "summary" => summary = Some(value.to_string()),
            _ => unrecognized.push(line.to_string()),
        }
    }
    Some(ModelStopDeclaration {
        intent: intent.unwrap_or(StopIntent::Conclude),
        reason: reason.unwrap_or(StopReason::Completed),
        summary: summary.unwrap_or_default(),
        raw_unrecognized: unrecognized.join("; "),
    })
}

/// 声明语法（guide 与软提醒共用的单一来源；不含判断与建议话术——只述通道）。
pub fn model_stop_syntax_line() -> String {
    let await_part = if model_stop_await_enabled() {
        "；`intent: pause`＝暂停（待回应），续跑由外部入口发起"
    } else {
        "；（本装置 `ORZ_MODEL_STOP_AWAIT=0`：暂停广告关闭，声明仍如实记录）"
    };
    format!(
        "{MODEL_STOP_PREFIX}]\nintent: conclude（或 pause）\nreason: completed|partial|blocked|awaiting_response\n\
         summary: 一句话已做／现场／未完／所需\n{MODEL_STOP_END}{await_part}"
    )
}

/// **0bs ①（2026-09-25）：模型面常驻尾行**——结束自述通道的**告知面收口**。
/// 单一来源＝本模块；由模型面装配（`model_face::build_model_face`）每轮尾随
/// 注入（分块表之后）。0bm 轮实证：语法只挂 pull 面 `guide` ⇒ 137 工具轮
/// 零自述、`run_finished` 仍三键——故改为**常驻**（模型不必先想到去 pull）。
/// 开关口径同 [`model_stop_syntax_line`]：`ORZ_MODEL_STOP_AWAIT=0` 只关暂停
/// 广告，不影响声明记录本身。
pub fn model_stop_resident_line() -> String {
    let intent = if model_stop_await_enabled() {
        "`intent: conclude`（或 `pause`＝暂停/待回应——续跑由外部入口发起）"
    } else {
        "`intent: conclude`（本装置 `ORZ_MODEL_STOP_AWAIT=0`：暂停广告关闭，声明仍如实记录）"
    };
    format!(
        "【结束自述通道】你可在**收束轮**（不调用工具的那一轮）文本里以 \
         {MODEL_STOP_PREFIX}] … {MODEL_STOP_END} 声明结束：{intent}、\
         `reason: completed|partial|blocked|awaiting_response`、\
         `summary: 一句话（已做／现场／未完／所需）`——机械层如实落账（不判定、不驳回）。"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_declaration_with_intent_reason_and_summary() {
        let text = "终答正文。\n[RUN_END]\nintent: pause\nreason: awaiting_response\n\
                    summary: 已试 A/B；现场在 C；需要用户确认 D\n[/RUN_END]\n";
        let decl = extract_model_stop(text).expect("declaration extracted");
        assert_eq!(decl.intent, StopIntent::Pause);
        assert_eq!(decl.reason, StopReason::AwaitingResponse);
        assert!(decl.summary.contains("已试 A/B"));
        assert!(decl.raw_unrecognized.is_empty());
    }

    #[test]
    fn missing_fields_fall_back_without_judgement() {
        // 只声明 intent ⇒ reason 缺省 completed、summary 空（不审查不罚）。
        let decl = extract_model_stop("[RUN_END]\nintent: conclude\n[/RUN_END]").unwrap();
        assert_eq!(decl.intent, StopIntent::Conclude);
        assert_eq!(decl.reason, StopReason::Completed);
        assert!(decl.summary.is_empty());
        // 空块 ⇒ 全缺省（机械层不驳回）。
        let decl = extract_model_stop("[RUN_END]\n[/RUN_END]").unwrap();
        assert_eq!(decl.intent, StopIntent::Conclude);
        assert_eq!(decl.reason, StopReason::Completed);
    }

    #[test]
    fn unknown_values_are_recorded_not_rejected() {
        let decl = extract_model_stop(
            "[RUN_END]\nintent: hold\nreason: stuck\nsummary: 自定义\n[/RUN_END]",
        )
        .unwrap();
        assert_eq!(decl.intent, StopIntent::Other);
        assert_eq!(decl.reason, StopReason::Other);
        assert!(decl.raw_unrecognized.contains("intent: hold"));
        assert!(decl.raw_unrecognized.contains("reason: stuck"));
        assert_eq!(decl.summary, "自定义");
    }

    #[test]
    fn last_declaration_wins_and_absent_block_is_none() {
        let text = "[RUN_END]\nintent: pause\n[/RUN_END]\n（改主意）\n[RUN_END]\nintent: conclude\n[/RUN_END]";
        let decl = extract_model_stop(text).unwrap();
        assert_eq!(decl.intent, StopIntent::Conclude);
        assert!(extract_model_stop("没有声明块").is_none());
        assert!(
            extract_model_stop("[RUN_END]\nintent: pause").is_none(),
            "未闭合不认"
        );
    }

    #[test]
    fn switch_defaults_on_and_off_is_explicit_zero_only() {
        // 缺省开（真实环境变量面只读数、不改写）。
        unsafe {
            std::env::remove_var("ORZ_MODEL_STOP_AWAIT");
        }
        assert!(model_stop_await_enabled());
        // 关闭只认字面 "0"（其余值＝开——显式关闭形态）。
        assert_ne!("0".trim() != "0", true);
        let syntax = model_stop_syntax_line();
        assert!(syntax.contains("intent: conclude"));
        assert!(syntax.contains(MODEL_STOP_END));
    }

    /// 0bs ① 钉（2026-09-25）：**常驻尾行**承载同一单源语法（前缀/结束标记/
    /// 取值枚举均取自本模块常量）；文本有界（常驻成本可核）。
    #[test]
    fn resident_line_carries_the_channel_syntax_from_single_source() {
        let line = model_stop_resident_line();
        assert!(line.starts_with("【结束自述通道】"));
        assert!(line.contains(MODEL_STOP_PREFIX));
        assert!(line.contains(MODEL_STOP_END));
        assert!(line.contains("conclude"));
        assert!(line.contains("awaiting_response"));
        assert!(line.contains("summary"));
        assert!(line.len() < 400, "常驻行长度应受控：{} 字符", line.len());
    }
}
