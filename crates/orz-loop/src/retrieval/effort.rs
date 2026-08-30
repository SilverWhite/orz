//! 委托契约复杂度分档（RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批，
//! 2026-08-30 设计轮定稿 + 用户裁决采纳）。
//!
//! 机械层、模型面零改动：`classify_retrieval_effort` 是纯函数，把
//! query/scope/max_results/lane 确定性映射到三档 effort；档位只调
//! run 级执行预算（墙钟/轮数/候选默认/同轮浏览器并行），不扩大
//! [DOC]/[SOURCE] 回传与注入预算上限（第一批 16 行/8K 既定边界不动）。
//! 设计文档：`docs/RETRIEVAL_ORCHESTRATION_MECHANICAL_BATCH2_DESIGN_2026-08-30.md`。

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// 委托契约 effort 档位。预算语义=上限（`budget_exhausted` 终态不变）；
/// 既有显式 env（`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS` /
/// `ORZ_RETRIEVAL_MAX_TOOL_ROUNDS`）优先于档位默认。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffortTier {
    Standard,
    Extended,
    Deep,
}

impl EffortTier {
    pub fn as_str(self) -> &'static str {
        match self {
            EffortTier::Standard => "standard",
            EffortTier::Extended => "extended",
            EffortTier::Deep => "deep",
        }
    }

    /// 档位墙钟上限（秒）。设计定案：standard 240s / extended 600s（旧
    /// 默认）/ deep 900s。`0` 禁用语义仍由显式 env 提供，档位默认恒启用。
    pub fn wallclock_default(self) -> Duration {
        match self {
            EffortTier::Standard => Duration::from_secs(240),
            EffortTier::Extended => Duration::from_secs(600),
            EffortTier::Deep => Duration::from_secs(900),
        }
    }

    /// 档位工具轮上限（与主车道 `max_tool_rounds` 取 min；ADR-0010
    /// §3.7.7 子代理 120 轮预算不变）。
    pub fn max_tool_rounds_default(self) -> u32 {
        match self {
            EffortTier::Standard => 30,
            EffortTier::Extended => 60,
            EffortTier::Deep => 90,
        }
    }

    /// 档位 `max_results` 默认（模型未显式传参时机械并入 goal 文本）。
    pub fn max_results_default(self) -> u64 {
        match self {
            EffortTier::Standard => 5,
            EffortTier::Extended => 8,
            EffortTier::Deep => 12,
        }
    }

    /// 档位同轮 `browser_read` 并行上限（loop 侧信号量；host tab 池仍是
    /// 最终物理上限）。deep = 不限（池大小决定）。
    pub fn browser_read_concurrency(self) -> Option<usize> {
        match self {
            EffortTier::Standard => Some(2),
            EffortTier::Extended => Some(4),
            EffortTier::Deep => None,
        }
    }
}

/// 广度/聚合词面（命中即 +0.5，上限 +1）——「调研/比较/总结 全部/多个」等
/// 形态提示跨来源聚合任务，机械信号与 Anthropic effort 分档同向。
const BREADTH_WORDS: &[&str] = &[
    "调研",
    "调查",
    "比较",
    "对比",
    "分析",
    "总结",
    "综述",
    "全部",
    "所有",
    "多个",
    "各方面",
    "各种",
];

/// 委托契约复杂度分档——纯函数（确定性、无模型参与、表驱动单测）。
///
/// 计分规则（设计定案 §4.2）：
/// - query 长度（字符）：≤200 → +0；200–800 → +1；>800 → +2；
/// - 广度/聚合词面命中：每命中 +0.5（上限 +1）；
/// - scope：显式窄 scope → +0；含通配/多目录/逗号分号 → +1；无 scope
///   （全工作区）→ +1；
/// - max_results：>10 → +1；5–10 → +0.5；<5/缺省 → +0；
/// - lane：外部 web 检索 → +1；内部文档 → +0。
///
/// 边界（登记于设计文档）：总分 ≤1.0 → standard；≤3.0 → extended；其余 → deep。
pub fn classify_retrieval_effort(
    query: &str,
    scope: Option<&str>,
    max_results: Option<u64>,
    external_lane: bool,
) -> EffortTier {
    let mut score = 0.0f64;
    let qlen = query.chars().count();
    if qlen > 800 {
        score += 2.0;
    } else if qlen > 200 {
        score += 1.0;
    }
    let breadth_hits = BREADTH_WORDS.iter().filter(|w| query.contains(**w)).count();
    score += (breadth_hits as f64 * 0.5).min(1.0);
    match scope.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) if s.contains('*') || s.contains(',') || s.contains(';') => score += 1.0,
        Some(_) => {}
        None => score += 1.0,
    }
    match max_results {
        Some(n) if n > 10 => score += 1.0,
        Some(n) if (5..=10).contains(&n) => score += 0.5,
        _ => {}
    }
    if external_lane {
        score += 1.0;
    }
    if score <= 1.0 {
        EffortTier::Standard
    } else if score <= 3.0 {
        EffortTier::Extended
    } else {
        EffortTier::Deep
    }
}

/// `ORZ_RETRIEVAL_EFFORT=standard|extended|deep` 强制覆盖（评测/对拍用；
/// 显式设置优先于机械映射）。
pub fn retrieval_effort_override() -> Option<EffortTier> {
    std::env::var("ORZ_RETRIEVAL_EFFORT")
        .ok()
        .and_then(|s| parse_retrieval_effort(&s))
}

/// 档位 env 值的纯解析（与 `parse_retrieval_subagent_wallclock` 同约定：
/// 无效/缺失 → None，忽略不报错）。独立函数使单测不依赖进程环境
/// （并行测试/评测环境设置了该变量也不互相污染）。
pub fn parse_retrieval_effort(s: &str) -> Option<EffortTier> {
    match s.trim() {
        "standard" => Some(EffortTier::Standard),
        "extended" => Some(EffortTier::Extended),
        "deep" => Some(EffortTier::Deep),
        _ => None,
    }
}

/// 从检索工具调用参数机械提取分档输入（与 `build_retrieval_task_goal`
/// 的参数口径一致）。
pub fn effort_inputs_from_args(args: &serde_json::Value) -> (String, Option<String>, Option<u64>) {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let scope = args
        .get("scope")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let max_results = args.get("max_results").and_then(|v| v.as_u64());
    (query, scope, max_results)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tier(query: &str, scope: Option<&str>, max: Option<u64>, external: bool) -> EffortTier {
        classify_retrieval_effort(query, scope, max, external)
    }

    #[test]
    fn simple_internal_query_is_standard() {
        assert_eq!(
            tier("查找缓存层文档", None, None, false),
            EffortTier::Standard
        );
        assert_eq!(
            tier("readme 关键词", Some("docs/readme.md"), Some(3), false),
            EffortTier::Standard
        );
    }

    #[test]
    fn external_lane_bumps_short_queries_to_extended() {
        // 外部 lane 基础 +1：短查询（无 scope、max<5）→ 1.0 → extended。
        assert_eq!(
            tier("查找 API 文档", None, None, true),
            EffortTier::Extended
        );
    }

    #[test]
    fn query_length_and_breadth_words_scale() {
        // >800 字符 → +2。
        let long = "x".repeat(801);
        assert_eq!(tier(&long, None, None, false), EffortTier::Extended);
        // 长 + 聚合词 → deep。
        let long_breadth = format!("调研{}", "x".repeat(900));
        assert_eq!(tier(&long_breadth, None, None, false), EffortTier::Deep);
        // 200 字符边界：恰好 200 → standard。
        assert_eq!(
            tier(&"x".repeat(200), None, None, false),
            EffortTier::Standard
        );
        // 201 字符 → extended（+1）。
        assert_eq!(
            tier(&"x".repeat(201), None, None, false),
            EffortTier::Extended
        );
    }

    #[test]
    fn scope_and_max_results_shape_tier() {
        // 无 scope（全工作区）+1 且 max>10 +1 → extended。
        assert_eq!(tier("q", None, Some(20), false), EffortTier::Extended);
        // 通配 scope + 聚合词 + max>10 + 外部 → deep。
        assert_eq!(
            tier("调研", Some("src/**"), Some(20), true),
            EffortTier::Deep
        );
        // 窄 scope + max 5 + 外部 → 1.5 → extended。
        assert_eq!(
            tier("q", Some("src/a.rs"), Some(5), true),
            EffortTier::Extended
        );
    }

    #[test]
    fn boundary_scores_map_deterministically() {
        // 恰好 1.0 → standard；2.0 → extended；4.0 → deep。
        assert_eq!(tier("q", Some("src"), Some(3), false), EffortTier::Standard);
        assert_eq!(tier("q", None, Some(20), false), EffortTier::Extended);
        assert_eq!(tier("调研", None, Some(20), true), EffortTier::Deep);
    }

    #[test]
    fn tier_parameter_defaults_match_design_table() {
        assert_eq!(
            EffortTier::Standard.wallclock_default(),
            Duration::from_secs(240)
        );
        assert_eq!(
            EffortTier::Extended.wallclock_default(),
            Duration::from_secs(600)
        );
        assert_eq!(
            EffortTier::Deep.wallclock_default(),
            Duration::from_secs(900)
        );
        assert_eq!(EffortTier::Standard.max_tool_rounds_default(), 30);
        assert_eq!(EffortTier::Extended.max_tool_rounds_default(), 60);
        assert_eq!(EffortTier::Deep.max_tool_rounds_default(), 90);
        assert_eq!(EffortTier::Standard.max_results_default(), 5);
        assert_eq!(EffortTier::Extended.max_results_default(), 8);
        assert_eq!(EffortTier::Deep.max_results_default(), 12);
        assert_eq!(EffortTier::Standard.browser_read_concurrency(), Some(2));
        assert_eq!(EffortTier::Extended.browser_read_concurrency(), Some(4));
        assert_eq!(EffortTier::Deep.browser_read_concurrency(), None);
    }

    #[test]
    fn effort_override_parses_shapes_without_env() {
        // 只测纯解析函数（不触碰进程环境——并行测试/评测环境设置了
        // ORZ_RETRIEVAL_EFFORT 也不影响本测试）。
        assert_eq!(
            parse_retrieval_effort("standard"),
            Some(EffortTier::Standard)
        );
        assert_eq!(
            parse_retrieval_effort(" extended "),
            Some(EffortTier::Extended)
        );
        assert_eq!(parse_retrieval_effort("deep"), Some(EffortTier::Deep));
        assert_eq!(parse_retrieval_effort("turbo"), None);
        assert_eq!(parse_retrieval_effort(""), None);
    }

    #[test]
    fn effort_inputs_from_args_extracts_shape() {
        let args = serde_json::json!({
            "query": "调研 X",
            "scope": "src/**",
            "max_results": 20,
        });
        let (q, s, m) = effort_inputs_from_args(&args);
        assert_eq!(q, "调研 X");
        assert_eq!(s.as_deref(), Some("src/**"));
        assert_eq!(m, Some(20));
        // 非法 max_results（字符串）→ None（与 build_retrieval_task_goal 口径一致）。
        let (_, _, m) = effort_inputs_from_args(&serde_json::json!({ "max_results": "10" }));
        assert_eq!(m, None);
    }
}
