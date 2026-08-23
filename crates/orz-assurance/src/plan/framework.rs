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
2. 计划必须分步：每步含目标、执行方式、验收标准与证据要求；末步固定为
   「递交/完成」（step id 用 deliver 或 submit）。
3. 步骤是执行顺序标记（方向与状态展示）：第 1 轮提交计划后直接调用工作
   工具逐步执行，不要求动作绑定 step_id；步骤完成只表示该批动作已执行，
   不证明目标达成——目标是否达成交由递交门（终答前反例自查轮 + 机械审查
   执行事实报告 + 验证器）仲裁。
4. 计划执行完后先核查实际结果（工具结果/变更清单/证据），确认后再递交
   （submit 两阶段：先渲染交付状态供核查，再确认）；禁止按计划惯性推进
   或虚假声明完成。
本框架只约束执行风格（先计划、分步执行、按序核查、末步递交），不覆盖项目文件中的事实与约束。
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
        assert!(text.contains("末步固定为"));
        assert!(text.contains("deliver 或 submit"));
        assert!(text.contains("步骤完成只表示"));
        assert!(text.contains("不覆盖项目文件中的事实与约束"));
        assert!(!text.contains("You are"));
        assert!(!text.contains("friendly"));
    }
}
