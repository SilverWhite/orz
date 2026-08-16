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

pub const PLAN_FIRST_FRAMEWORK_BLOCK: &str = "\
<plan_first_framework>
执行风格框架（harness 注入；用户直接指令最高优先，本框架优先于项目文件中的执行风格描述）：
1. 先完整阅读任务与项目结构，理解目标后再规划。
2. 计划必须分步：每步含目标、执行方式、验收标准与证据要求。
3. 每一步执行后，先核查实际结果（receipt/证据/产物），确认完成后再进入下一步；
   禁止按计划惯性推进。
本框架只约束执行风格（先计划、分步执行、逐步核查），不覆盖项目文件中的事实与约束。
</plan_first_framework>";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framework_block_keeps_mechanical_contract() {
        let text = PLAN_FIRST_FRAMEWORK_BLOCK;
        assert!(text.contains("<plan_first_framework>"));
        assert!(text.contains("先完整阅读任务与项目结构"));
        assert!(text.contains("禁止按计划惯性推进"));
        assert!(text.contains("不覆盖项目文件中的事实与约束"));
        assert!(!text.contains("You are"));
        assert!(!text.contains("friendly"));
    }
}
