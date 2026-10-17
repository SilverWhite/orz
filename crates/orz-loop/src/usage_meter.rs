//! 0df 任务级计费机械记录点（2026-10-17，ADR-0010 §14.86 /
//! PER_TASK_METERING_DESIGN D3/D6）：run 级 usage 折算器——主车道与检索
//! 子代理（同 journal 同 [`crate::agent_loop::SharedLoopServices`]）把每次
//! `ModelResponse` 到达折入同一 meter，controller 在终态前落**恰一条**
//! `run_usage_metered` 事件。
//!
//! 折算点选在**内存 ModelResponse 到达点**而非 journal 重放：`model_output`
//! 事件不携带 `prompt_tokens`（A6 压缩触发专用内存读数），事后重放会丢
//! prompt 口径。金额不入机械记录（journal 哈希链不可改写，金额归因走派生
//! 台账配对——设计 D4）；`effort_tier` 是 gateway 配置侧设计值（trait 单读，
//! 空响应链降级是 per-call 局部量不改 config），与观测 `reasoning_tokens`
//! 分列（r4 实证：档位恒定 ≠ 推理量恒定）。

use crate::gateway::model::ModelResponse;

/// Run 级 usage 累计（per-run 隔离：`run_turn_inner` 起点复位，沿
/// `for_new_run()` 每 run 换新的隔离先例）。
#[derive(Debug, Default)]
pub(crate) struct RunUsageMeter {
    model_calls: u64,
    prompt_tokens: u64,
    cache_hit_tokens: u64,
    cache_miss_tokens: u64,
    completion_tokens: u64,
    reasoning_tokens: u64,
    /// 任一折算轮五项 usage 有缺（流中断臂等）⇒ true；非 completed 终态臂
    /// 由 controller 保守置位（终态前在途轮消耗不可知，如实登记不装完整）。
    usage_incomplete: bool,
    /// 终态计量行已成功落链的幂等旗（288 处置批）：`journal_usage_meter`
    /// 成功 record 后置位，此后本 run 的任何再入（completed 臂落行后
    /// `record_terminal` 罕见硬错上抛进 Err 臂）不再补写第二条——「恰
    /// 一条」由生产侧封死，而不只靠法官族事后报。record 失败不置位，
    /// Err 臂仍可重试落行。
    journal_line_written: bool,
}

impl RunUsageMeter {
    /// 折入一次 `ModelResponse` 到达（全部车道共用 run 的单一 meter）。
    /// Option 语义：任一项缺席不计和（求和保持诚实）并置
    /// `usage_incomplete`。
    pub(crate) fn fold_response(&mut self, response: &ModelResponse) {
        self.model_calls = self.model_calls.saturating_add(1);
        let mut complete = true;
        macro_rules! fold {
            ($field:ident) => {
                match response.$field {
                    Some(v) => self.$field = self.$field.saturating_add(u64::from(v)),
                    None => complete = false,
                }
            };
        }
        fold!(prompt_tokens);
        fold!(cache_hit_tokens);
        fold!(cache_miss_tokens);
        fold!(completion_tokens);
        fold!(reasoning_tokens);
        if !complete {
            self.usage_incomplete = true;
        }
    }

    /// 无 usage 可观测的轮（流中断／取消中断）——只置不完整位。
    pub(crate) fn mark_incomplete(&mut self) {
        self.usage_incomplete = true;
    }

    /// 终态计量行是否已成功落链（幂等旗读取——`journal_usage_meter`
    /// 臂间防双写，288 处置批）。
    pub(crate) fn journal_line_written(&self) -> bool {
        self.journal_line_written
    }

    /// 置位幂等旗（仅在计量行成功 record 之后调用——record 失败保持
    /// 未置位，Err 臂仍可重试落行）。
    pub(crate) fn mark_journal_line_written(&mut self) {
        self.journal_line_written = true;
    }

    /// `run_usage_metered` 事件 payload（每 run 终态前恰一条）。
    pub(crate) fn metered_payload(&self, model: &str, effort_tier: &str) -> serde_json::Value {
        serde_json::json!({
            "model": model,
            "model_calls": self.model_calls,
            "prompt_tokens": self.prompt_tokens,
            "cache_hit_tokens": self.cache_hit_tokens,
            "cache_miss_tokens": self.cache_miss_tokens,
            "completion_tokens": self.completion_tokens,
            "reasoning_tokens": self.reasoning_tokens,
            "effort_tier": effort_tier,
            "usage_incomplete": self.usage_incomplete,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::{FinishReason, ToolCall, TransportRetryInfo};

    fn response(
        prompt: Option<u64>,
        hit: Option<u64>,
        miss: Option<u64>,
        completion: Option<u32>,
        reasoning: Option<u32>,
    ) -> ModelResponse {
        ModelResponse {
            text: None,
            tool_calls: Vec::<ToolCall>::new(),
            finish_reason: FinishReason::Stop,
            reasoning_content: None,
            reasoning_tokens: reasoning,
            completion_tokens: completion,
            cache_hit_tokens: hit,
            cache_miss_tokens: miss,
            prompt_tokens: prompt,
            transport_retry: TransportRetryInfo::default(),
        }
    }

    /// 0df 钉：五项逐轮饱和求和；全齐 ⇒ incomplete 恒 false。
    #[test]
    fn fold_sums_five_fields_and_stays_complete() {
        let mut meter = RunUsageMeter::default();
        meter.fold_response(&response(Some(100), Some(80), Some(20), Some(30), Some(10)));
        meter.fold_response(&response(
            Some(200),
            Some(190),
            Some(10),
            Some(50),
            Some(40),
        ));
        let payload = meter.metered_payload("deepseek-chat", "max");
        assert_eq!(payload["model_calls"], 2);
        assert_eq!(payload["prompt_tokens"], 300);
        assert_eq!(payload["cache_hit_tokens"], 270);
        assert_eq!(payload["cache_miss_tokens"], 30);
        assert_eq!(payload["completion_tokens"], 80);
        assert_eq!(payload["reasoning_tokens"], 50);
        assert_eq!(payload["usage_incomplete"], false);
        assert_eq!(payload["effort_tier"], "max");
    }

    /// 0df 钉：任一项缺席 ⇒ 该项不计和＋incomplete 置位（honest sums）。
    #[test]
    fn fold_missing_field_marks_incomplete() {
        let mut meter = RunUsageMeter::default();
        meter.fold_response(&response(Some(100), Some(80), Some(20), Some(30), Some(10)));
        meter.fold_response(&response(Some(200), Some(190), None, Some(50), None));
        let payload = meter.metered_payload("m", "low");
        assert_eq!(payload["prompt_tokens"], 300);
        assert_eq!(payload["cache_miss_tokens"], 20);
        assert_eq!(payload["reasoning_tokens"], 10);
        assert_eq!(payload["usage_incomplete"], true);
    }

    /// 0df 钉：mark_incomplete（流中断臂）＋零轮形态（model_calls 0 ⇒
    /// 五项和 0，法官族不变量）。
    #[test]
    fn mark_incomplete_and_zero_round_shape() {
        let mut meter = RunUsageMeter::default();
        meter.mark_incomplete();
        let payload = meter.metered_payload("m", "unknown");
        assert_eq!(payload["model_calls"], 0);
        assert_eq!(payload["prompt_tokens"], 0);
        assert_eq!(payload["usage_incomplete"], true);
    }
}
