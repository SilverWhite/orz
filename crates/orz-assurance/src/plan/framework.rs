//! PLAN-FIRST execution-style framework block (D2).
//!
//! ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD_DESIGN §8: the mechanical
//! plan-first execution-style framework. This is the canonical copy,
//! shared by:
//! - orz-loop system-prompt injection (plan-first sessions, main +
//!   retrieval subagent — unconditional, independent of AGENTS.md);
//! - orz-agent `render_agents_md` fixed prefix (external AGENTS.md
//!   consumers, user content first).
//! 唯一机制：不做规范模板、不做 schema 校验（§10 非目标不变）。

pub const PLAN_FRAMEWORK_BLOCK: &str = "\
<plan_framework>
计划框架（harness 注入；用户直接指令最高优先，本框架优先于项目文件中的执行风格描述）：
1. 复杂任务建议用 plan_write 记录目标、步骤、验收标准与证据要求；简单任务可直接行动，不强制先写计划。
2. 计划可分步：每步含目标、执行方式、验收标准与证据要求；末步建议为
   「递交/完成」（step id 用 deliver 或 submit）。
3. 计划是执行顺序与状态展示，不是首轮门槛；第 1 轮不要求先提交计划，
   也不要求动作绑定 step_id。
4. 递交前先核查实际结果（工具结果/变更清单/证据），确认后再递交；禁止
   按计划惯性推进或虚假声明完成。
本框架只约束执行风格（计划可用、分步执行、按序核查、末步递交），不覆盖项目文件中的事实与约束。
</plan_framework>";

/// 兼容旧名。
#[deprecated(note = "use PLAN_FRAMEWORK_BLOCK")]
pub const PLAN_FIRST_FRAMEWORK_BLOCK: &str = PLAN_FRAMEWORK_BLOCK;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_block_keeps_mechanical_contract() {
        let text = PLAN_FRAMEWORK_BLOCK;
        assert!(text.contains("<plan_framework>"));
        assert!(text.contains("不强制先写计划"));
        assert!(text.contains("禁止"));
        assert!(text.contains("末步建议为"));
        assert!(text.contains("deliver 或 submit"));
        assert!(text.contains("计划是执行顺序与状态展示"));
        assert!(text.contains("不覆盖项目文件中的事实与约束"));
        assert!(!text.contains("You are"));
        assert!(!text.contains("friendly"));
    }
}
