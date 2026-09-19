//! 0ar S2（2026-09-19，检索批次回送与轮级单席位设计 v1.0）：阈值回送 /
//! 单批墙钟 / 合并优先的共享机械常数与纯函数。
//!
//! 设计权威：[`RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19`]
//! （docs/RETRIEVAL_BATCH_HANDOFF_AND_ROUND_SEAT_DESIGN_2026-09-19.md）。
//! 三件套共用同一把**宽口径**尺（§3.3）：`visibility ∈ {full_text_observed,
//! partial_text_observed}` 的工具证据、按 `content_sha256` 去重——阈值
//! （5）、机械护栏（10）、可见倒数（每批检索结果尾行）三处读数必须一致，
//! 不一致落 anomaly、不静默取其一（设计 §8 判据 5/7）。

use crate::retrieval::evidence::EvidenceRecord;

/// 报告阈值（§3.2 `sufficiency_target`）：可用来源满 5 条即收尾回送
/// （判定权归子代理——机械计数是共用尺，子代理以 β 收尾回合产出总结）。
pub(crate) const SUFFICIENCY_TARGET: u64 = 5;

/// 机械护栏（§3.2 `mechanical_cap`）：子代理未自行收尾时的强制收尾上限
/// （「最起码留出二取一的余地」，用户 2026-09-19 裁决定值 10）。
pub(crate) const MECHANICAL_CAP: u64 = 10;

/// 合并上限（§5.4）：同轮多检索先合并为单激活多 query，超出部分以
/// 「未派发」交回（无 `ToolStarted` 的预派发拒绝）。
pub(crate) const MERGE_MAX_QUERIES: usize = 3;

/// D3 溢出拒绝的稳定 cause（§5.4/§5.5）：无 `ToolStarted` 的预派发拒绝，
/// 模板同族 `refuse_inject_budget`／`plan_round_denied`。
pub(crate) const DEFERRED_CAUSE: &str = "retrieval_dispatch_deferred_one_per_round";

/// 提前交付（§3.7）显式标记：子代理不满 5 条可主动提交，标记行其余部分
/// 即证据指针（缺指针按普通收尾处理＋anomaly，fail-open）。
pub(crate) const EARLY_DELIVERY_MARKER: &str = "[EARLY_DELIVERY]";

/// 连续提前交付（可用计数 <5）的 anomaly 观测线（§3.7/§10-5：可审计、
/// 不阻断；第 N 次起在 assessment `reason_codes` 落观测码）。
pub(crate) const EARLY_DELIVERY_STREAK_ANOMALY: u32 = 3;

/// 本批收尾成因（`LoopOutcome` 携带，dispatch 映射到
/// `retrieval_close_record.terminal_reason`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BatchCloseKind {
    /// 可用证据满报告阈值（5）——β 收尾回合后正常收口。
    EvidenceThresholdMet,
    /// 可用证据触机械护栏（10）——强制进入同一 β 收尾形态。
    MechanicalCapForceClose,
    /// 单批墙钟到点（§4，默认 300 s）——以「部分证据＋缺口」正常交回，
    /// 不走 `subagent_failed` 失败臂。
    WallclockBound,
}

impl BatchCloseKind {
    /// 映射到 `retrieval_close_record.terminal_reason`（S1 契约面 12 值
    /// 闭枚举；`auto_close`／`subagent_early_delivery` 在 dispatch 侧
    /// 另行判定）。
    pub(crate) fn terminal_reason(self) -> &'static str {
        match self {
            BatchCloseKind::EvidenceThresholdMet | BatchCloseKind::MechanicalCapForceClose => {
                "evidence_threshold_met"
            }
            BatchCloseKind::WallclockBound => "dispatch_wallclock_bound",
        }
    }
}

/// 宽口径可用计数（§3.3 定稿口径）：`visibility ∈ {full_text_observed,
/// partial_text_observed}` 的工具证据、按 `content_sha256` 去重。
/// 机械层从不采信模型自报；声明行恒 metadata-only，天然不入数。
pub(crate) fn usable_source_count(evidence: &[EvidenceRecord]) -> u64 {
    let mut seen = std::collections::HashSet::new();
    let mut usable = 0u64;
    for ev in evidence {
        if !matches!(
            ev.visibility.as_str(),
            "full_text_observed" | "partial_text_observed"
        ) {
            continue;
        }
        // 证据记录恒带 digest（PDF 取文档摘要、其余取输出摘要）；digest
        // 缺失时退回 identity——去重键必须稳定，宁可用身份也不漏计。
        let key = ev
            .content_sha256
            .clone()
            .unwrap_or_else(|| format!("identity:{}", ev.identity));
        if seen.insert(key) {
            usable += 1;
        }
    }
    usable
}

/// 逐 query 宽口径可用计数（§5.4 判据 6「逐 query 可用计数可核」）：
/// 按 `EvidenceRecord::search_query` 精确匹配归因。web_fetch/browser_read
/// 等派生证据不带 query（`search_query=None`）⇒ 多 query 合并激活下归属
/// 「未归因」，单 query 激活全额归属该 query（与总数一致）。
pub(crate) fn per_query_usable_counts(queries: &[String], evidence: &[EvidenceRecord]) -> Vec<u64> {
    if queries.len() <= 1 {
        return vec![usable_source_count(evidence); queries.len()];
    }
    queries
        .iter()
        .map(|q| {
            let owned: Vec<EvidenceRecord> = evidence
                .iter()
                .filter(|ev| ev.search_query.as_deref() == Some(q.as_str()))
                .cloned()
                .collect();
            usable_source_count(&owned)
        })
        .collect()
}

/// 可见倒数行（§3.6）：每批检索结果末尾由机械层追加（模型不可写；随该次
/// 工具结果入链，不新增事件类型）。**条目额度与已发起调用数分开报**——
/// 护栏 10 计的是可用条目而非检索调用次数，混报会把上限误读成配额。
pub(crate) fn countdown_line(usable: u64, retrieval_calls: u64) -> String {
    let remaining = MECHANICAL_CAP.saturating_sub(usable);
    format!(
        "[机械] 本批可用证据 {usable}/{SUFFICIENCY_TARGET}；距强制报告额度上限还剩 \
         {remaining} 条（上限 {MECHANICAL_CAP}）。本轮已发起检索调用 {retrieval_calls} 次。"
    )
}

/// β 收尾回合注入块（§3.4 定案形态）：中性事实＋机械指令——工具面收空、
/// 唯一收尾回合。不教学、不评价（FP-2 纪律）。
pub(crate) fn close_round_block(kind: BatchCloseKind, usable: u64) -> String {
    let fact = match kind {
        BatchCloseKind::EvidenceThresholdMet => {
            format!("本批可用证据 {usable} 条，已满足报告阈值（{SUFFICIENCY_TARGET}）")
        }
        BatchCloseKind::MechanicalCapForceClose => {
            format!("本批可用证据 {usable} 条，已到强制报告上限（{MECHANICAL_CAP}）")
        }
        BatchCloseKind::WallclockBound => String::from("本批派发墙钟到点"),
    };
    format!(
        "[机械] 检索批次收尾：{fact}；工具面已收空，本回合为唯一收尾回合——\
         请基于已得证据产出本批结果总结（含 [DOC]/[SOURCE] 声明行），不再发起新的动作。"
    )
}

/// 解析提前交付声明（§3.7）：标记行其余部分即证据指针。仅认独立成行的
/// 标记（行首），行内提及不触发。
pub(crate) fn parse_early_delivery(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(EARLY_DELIVERY_MARKER))
        .map(str::trim)
        .map(str::to_string)
}

/// 检索族工具判定（车道内可见的检索/抓取面）：web 族 + 本地浏览器族 +
/// 项目文档/PDF 读族。可见倒数与调用计数只落这些工具（工作区 read_file
/// 等虽产出证据，但不计入「已发起检索调用」——那是检索面不是抓取面）。
pub(crate) fn is_lane_retrieval_tool(name: &str) -> bool {
    crate::relay::is_web_retrieval_tool(name)
        || matches!(
            name,
            "browser_read" | "browser_control" | "pdf_read" | "project_doc_index"
        )
}

/// 把可见倒数行追加到指定 call_id 的 Tool 消息尾部（模型面＋会话侧车
/// 载体；不新增事件类型——随该次工具结果入链）。按 call_id 自后向前定
/// 位（runner 已把结果消息推入 `messages`）。找不到目标消息时静默跳过
/// （倒计时是附面，绝不因附面缺席而失败）。
pub(crate) fn append_countdown_to_tool_message(
    messages: &mut [crate::gateway::model::Message],
    call_id: &str,
    evidence: &[EvidenceRecord],
    retrieval_calls: u64,
) {
    let line = countdown_line(usable_source_count(evidence), retrieval_calls);
    for m in messages.iter_mut().rev() {
        if m.role == crate::gateway::model::Role::Tool && m.tool_call_id.as_deref() == Some(call_id)
        {
            m.content.push('\n');
            m.content.push_str(&line);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(visibility: &str, digest: &str, query: Option<&str>) -> EvidenceRecord {
        EvidenceRecord {
            tool: "web_fetch".to_string(),
            identity: format!("https://example.com/{digest}"),
            title: digest.to_string(),
            source_type: "web_page".to_string(),
            visibility: visibility.to_string(),
            content_sha256: Some(format!("{digest:0>64}").chars().take(64).collect()),
            observed_scope: "test".to_string(),
            missing_scope: "none".to_string(),
            candidate_urls: Vec::new(),
            search_query: query.map(str::to_string),
            accessed_at: "2026-09-19T00:00:00Z".to_string(),
        }
    }

    /// 宽口径三钉：full/partial 入数、metadata 不入数、digest 去重
    /// （设计 §3.3——七批次严口径最大 7 的反证 ⇒ 必须宽口径）。
    #[test]
    fn usable_count_is_wide_scope_with_dedup() {
        let evidence = vec![
            ev("full_text_observed", "a", None),
            ev("partial_text_observed", "b", None),
            ev("metadata_only", "c", None),
            // 同一 digest 重复观察（同页二次抓取）只计 1。
            ev("full_text_observed", "a", None),
        ];
        assert_eq!(usable_source_count(&evidence), 2);
    }

    /// 逐 query 归因：search_query 精确匹配；单 query 全额归属。
    #[test]
    fn per_query_counts_attribute_by_search_query() {
        let evidence = vec![
            ev("full_text_observed", "a", Some("q1")),
            ev("partial_text_observed", "b", Some("q2")),
            ev("full_text_observed", "c", None), // 派生证据：多 query 下未归因
        ];
        let multi = per_query_usable_counts(&["q1".into(), "q2".into()], &evidence);
        assert_eq!(multi, vec![1, 1]);
        let single = per_query_usable_counts(&["q1".into()], &evidence);
        assert_eq!(single, vec![3]);
    }

    /// 可见倒数行：条目额度与调用次数分开报（§3.6 口径钉）。
    #[test]
    fn countdown_line_reports_entries_and_calls_separately() {
        let line = countdown_line(3, 12);
        assert!(line.contains("本批可用证据 3/5"), "{line}");
        assert!(line.contains("还剩 7 条（上限 10）"), "{line}");
        assert!(line.contains("已发起检索调用 12 次"), "{line}");
        // 到上限：剩余 0，不 panic、不出现负数。
        let at_cap = countdown_line(10, 20);
        assert!(at_cap.contains("还剩 0 条"), "{at_cap}");
    }

    /// 提前交付标记：整行前缀解析；行内提及不触发。
    #[test]
    fn early_delivery_marker_parses_pointer_line() {
        let text = "总结……\n[EARLY_DELIVERY] ledger SRC-001..SRC-003; archive: x\n后续";
        assert_eq!(
            parse_early_delivery(text).as_deref(),
            Some("ledger SRC-001..SRC-003; archive: x")
        );
        assert_eq!(
            parse_early_delivery("提到 [EARLY_DELIVERY] 但不在行首"),
            None
        );
        assert_eq!(
            parse_early_delivery("[EARLY_DELIVERY]"),
            Some(String::new())
        );
    }

    /// terminal_reason 映射钉（S1 契约面闭枚举值）。
    #[test]
    fn batch_close_kind_maps_to_contract_reasons() {
        assert_eq!(
            BatchCloseKind::EvidenceThresholdMet.terminal_reason(),
            "evidence_threshold_met"
        );
        assert_eq!(
            BatchCloseKind::MechanicalCapForceClose.terminal_reason(),
            "evidence_threshold_met"
        );
        assert_eq!(
            BatchCloseKind::WallclockBound.terminal_reason(),
            "dispatch_wallclock_bound"
        );
    }
}
