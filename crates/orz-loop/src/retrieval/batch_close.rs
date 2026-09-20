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

/// 0au（2026-09-20 立项，S3 摩擦 N2）：派发前 run 余量判定——批墙钟之外
/// 还须容纳**一次收尾回合**的余量（秒）。remaining < batch_wallclock ＋
/// 本余量 ⇒ 本批保留不派发（`WALLCLOCK_RESERVED_CAUSE`）。
pub(crate) const CLOSE_ROUND_MARGIN_SECS: u64 = 60;

/// 0au 尾部保留（秒）：距 run 墙钟不足此数**一律**不派发新批，尾部留给
/// 落盘（方案建议 90–120 s，取上沿 120）。与档位表 180／300／450 绑定
/// 判定（`wallclock_reserved`），不另立第二把尺。
pub(crate) const RUN_TAIL_RESERVE_SECS: u64 = 120;

/// 0au 保留拒绝的稳定 cause：无 `ToolStarted` 的预派发拒绝（模板同 D3）；
/// 语义是「保留」不是「失败」——run 余量不足以容纳完整检索批＋收尾。
pub(crate) const WALLCLOCK_RESERVED_CAUSE: &str = "retrieval_dispatch_wallclock_reserved";

/// 0au 纯函数：本批应否因 run 墙钟余量不足而保留（不派发）。
///
/// `remaining = None`（无已知 run 上限——`ORZ_MAX_WALLCLOCK` 未设/为 0）
/// ⇒ 永不保留（自然收工的 run 无预算可判，与 S3 取证口径一致：摩擦例全
/// 部产生于墙钟到点的 run）。判据两支任一即保留：
/// ① `remaining < batch_wallclock + CLOSE_ROUND_MARGIN_SECS`（批跑不满＋
///   无收尾回合——S3 五次 trailing 的直接成因）；
/// ② `remaining < RUN_TAIL_RESERVE_SECS`（尾部保留，落盘窗口）。
pub(crate) fn wallclock_reserved(
    remaining: Option<std::time::Duration>,
    batch_wallclock: std::time::Duration,
) -> bool {
    let Some(remaining) = remaining else {
        return false;
    };
    let need = batch_wallclock + std::time::Duration::from_secs(CLOSE_ROUND_MARGIN_SECS);
    remaining < need || remaining < std::time::Duration::from_secs(RUN_TAIL_RESERVE_SECS)
}

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

/// S3 前去噪（设计 §10-2「空批不唤醒」）：可用计数为 0 的批次绝不武装
/// β 收尾回合——不产生收尾模型调用；1–4 条未达阈值同样不武装（等子代理
/// 继续或由墙钟 D2 交回）。阈值/护栏共用一段判定，空批显式短路。
pub(crate) fn should_arm_close(usable: u64) -> Option<BatchCloseKind> {
    if usable == 0 {
        return None;
    }
    if usable >= MECHANICAL_CAP {
        Some(BatchCloseKind::MechanicalCapForceClose)
    } else if usable >= SUFFICIENCY_TARGET {
        Some(BatchCloseKind::EvidenceThresholdMet)
    } else {
        None
    }
}

/// 宽口径可用计数（§3.3 定稿口径）：`visibility ∈ {full_text_observed,
/// partial_text_observed}` 的工具证据、按 `content_sha256` 去重。
/// 机械层从不采信模型自报；声明行恒 metadata-only，天然不入数。
///
/// 0ax S1（2026-09-20 立项，S3 摩擦 N5）：**无 URL 合成答案不入数**——
/// `web_search_result` 证据的引用面（`candidate_urls`）为空即是合成文本
/// （DeepSeek 侧 `url_citation` 恒空 ⇒ 无 fetch 目标、不可引用），不再计入
/// 阈值 5／护栏 10 的可用额度，改由 [`synthetic_answer_count`] 单列。
/// S3 实测该口径曾被「不可引用的文本」凑满（act 01 的 5/5 全为无 URL
/// 合成文本）⇒ 子代理按规则提前收尾、判分物未落盘。阈值/护栏/倒数仍共用
/// 这一把尺（§3.3 一致性判据不受影响）。
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
        if is_synthetic_answer(ev) {
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

/// 0ax S1：判定一条证据是否为「无 URL 合成答案」——`web_search_result`
/// 且引用池为空（机械形态：宿主结构化 seam 未带来任何 citation URL）。
/// 其余证据（web_page/pdf_document/local_file/project_doc/web_search 带
/// 引用池）不在此列。
///
/// 0ay S1（2026-09-20，GAP-RETRIEVAL-SYNTHETIC-JUDGEMENT-AUDITABILITY）：
/// 判定输入（原引用池大小）与落盘面共用 [`citation_url_count`] 这一把尺
/// ——`None` ⇔ 原池为空 ⇔ 合成答案；`Some(n)` ⇒ 该条目按 `n` 落
/// `citation_url_count`（装配点 `evidence.rs` 同一 helper 调用）。
pub(crate) fn is_synthetic_answer(ev: &EvidenceRecord) -> bool {
    ev.source_type == "web_search_result" && citation_url_count(ev).is_none()
}

/// 0ay S1：条目**原**（前置过滤前）web_search 引用池大小——判定输入的唯一
/// 单源。原池为空时返回 `None`（缺席即空池，落盘面据此判合成）；非空时
/// 返回原池条数（`candidate_urls` 字段是**前置过滤后**的保留池，故
/// `citation_url_count ≥ candidate_urls.len()`，且
/// `citation_url_count == 保留池 ＋ prefilter_log 移除数`）。
pub(crate) fn citation_url_count(ev: &EvidenceRecord) -> Option<usize> {
    (!ev.candidate_urls.is_empty()).then_some(ev.candidate_urls.len())
}

/// 0ax S1：本批「无 URL 合成答案」条数（宽口径可见性内、按去重键去重）。
/// 单列披露用——不入可用额度，但必须在倒数行与 committed payload 里可见
/// （「如实标注全为合成」的机械面）。
pub(crate) fn synthetic_answer_count(evidence: &[EvidenceRecord]) -> u64 {
    let mut seen = std::collections::HashSet::new();
    let mut synthetic = 0u64;
    for ev in evidence {
        if !matches!(
            ev.visibility.as_str(),
            "full_text_observed" | "partial_text_observed"
        ) {
            continue;
        }
        if !is_synthetic_answer(ev) {
            continue;
        }
        let key = ev
            .content_sha256
            .clone()
            .unwrap_or_else(|| format!("identity:{}", ev.identity));
        if seen.insert(key) {
            synthetic += 1;
        }
    }
    synthetic
}

/// 0at B 面（2026-09-20 立项，S3 摩擦 N1）：未归因可用计数＝批级可用 −
/// Σ 逐 query 可用（逐 query 归因键＝逐字 `search_query` 精确匹配）。多
/// query 批披露本值（单 query 批全额归属、无缺口，不落字段）。
///
/// 恒等式 `Σ 逐 query ＋ unattributed ＝ 批级可用`（判据 ①）成立的条件：
/// 逐 query 桶按 digest **互斥**（同一内容只落在派发序的一个桶）。同一
/// 内容被两个派发 query 都检索到时，桶内各自去重令 Σ 逐 query > 批级，
/// 本函数饱和到 0——恒等式在该批不成立，装配点（`evidence.rs`）以机械
/// 告警留痕（不阻断、不改 payload 形状）。
pub(crate) fn unattributed_usable_count(queries: &[String], evidence: &[EvidenceRecord]) -> u64 {
    let batch = usable_source_count(evidence);
    let per_query: u64 = per_query_usable_counts(queries, evidence).iter().sum();
    batch.saturating_sub(per_query)
}

/// 0at A 面（2026-09-20 立项，S3 摩擦 N1）：派发谱系归因——为每条证据
/// 指定其来源的派发 query id（与 `query_summary` 的 `query_id` 同一生成
/// 域）。归因规则（机械、有序短路）：
/// ① 单 query 批 ⇒ 全 `None`（整批天然归属该 query，payload 逐字节不变）；
/// ② 逐字匹配 `search_query` == 派发 query ⇒ 该 query；
/// ③ 归一化匹配（trim＋小写＋空白折叠）⇒ 该 query；
/// ④ 派生证据（`search_query=None`）且 identity 命中某条已归因
///    `web_search` 证据的候选池 ⇒ 归该证据所属 query（「派生证据归发起
///    它的那次检索」）；
/// ⑤ 其余（子代理自查串/无法定位来源的派生证据）⇒ **leader 谱系**（本批
///    首个派发 query——子代理自查服务整个激活）。
///
/// 返回值与 `evidence` 等长对齐。判据：归一化后逐 query 覆盖率 ≥90 %
/// （S3 语料回放）。
pub(crate) fn origin_query_assignments(
    queries: &[String],
    evidence: &[EvidenceRecord],
    query_ids: &[String],
) -> Vec<Option<String>> {
    if queries.len() <= 1 || evidence.is_empty() {
        return vec![None; evidence.len()];
    }
    let normalize = |s: &str| {
        s.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let id_of = |index: usize| -> Option<String> { query_ids.get(index).cloned() };

    // 每条证据的「主归因」序号（None = 尚未归因）。
    let mut assigned: Vec<Option<usize>> = evidence
        .iter()
        .map(|ev| {
            let q = ev.search_query.as_deref()?;
            if let Some(i) = queries.iter().position(|declared| declared == q) {
                return Some(i);
            }
            queries
                .iter()
                .position(|declared| normalize(declared) == normalize(q))
        })
        .collect();
    // ④ 派生证据的候选池回溯：identity 出现在某条已归因 web_search 证据
    // 的候选池里 ⇒ 跟随该证据。
    for (index, ev) in evidence.iter().enumerate() {
        if assigned[index].is_some() || ev.search_query.is_some() {
            continue;
        }
        for (other, candidate) in evidence.iter().enumerate() {
            let Some(host_index) = assigned[other] else {
                continue;
            };
            if other == index || candidate.search_query.is_none() {
                continue;
            }
            if candidate
                .candidate_urls
                .iter()
                .any(|url| url == &ev.identity)
            {
                assigned[index] = Some(host_index);
                break;
            }
        }
    }
    // ⑤ leader 兜底：子代理自查/未定位派生证据归首个派发 query（谱系
    // 不留空——每条证据都归属到某个派发 query；字面匹配缺口由 B 面
    // `unattributed_usable_count` 披露，两层面语义互补）。
    assigned
        .into_iter()
        .map(|slot| match slot {
            Some(i) => id_of(i),
            None => id_of(0),
        })
        .collect()
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
///
/// 0ax S1（2026-09-20）：`synthetic_answers` > 0 时附「可引用性」披露——
/// 无 URL 合成答案不计入可用额度（单列），=0 时整行与旧形态逐字节一致。
pub(crate) fn countdown_line(usable: u64, retrieval_calls: u64, synthetic_answers: u64) -> String {
    let remaining = MECHANICAL_CAP.saturating_sub(usable);
    let mut line = format!(
        "[机械] 本批可用证据 {usable}/{SUFFICIENCY_TARGET}；距强制报告额度上限还剩 \
         {remaining} 条（上限 {MECHANICAL_CAP}）。本轮已发起检索调用 {retrieval_calls} 次。"
    );
    if synthetic_answers > 0 {
        line.push_str(&format!(
            "另有 {synthetic_answers} 条为无 URL 合成文本，未计入可用额度（不可引用）。"
        ));
    }
    line
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
    let synthetic = synthetic_answer_count(evidence);
    let line = countdown_line(usable_source_count(evidence), retrieval_calls, synthetic);
    for m in messages.iter_mut().rev() {
        if m.role == crate::gateway::model::Role::Tool && m.tool_call_id.as_deref() == Some(call_id)
        {
            m.content.push('\n');
            m.content.push_str(&line);
            return;
        }
    }
}

/// S3 前去噪（设计 §5.6「重复 query 回踩」）：取检索调用的去重键——
/// `query` 优先、`url` 兜底（web_fetch/browser_read），trim 后为空视同无键。
/// 精确字符串匹配（不做大小写/语义归一，避免越权改写检索意图）。
pub(crate) fn query_key(arguments: &serde_json::Value) -> Option<String> {
    arguments
        .get("query")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            arguments
                .get("url")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .map(str::to_string)
}

/// 重复 query 的中性指针回执（不改写检索语义、不教学）：明确未重复检索，
/// 指向本 run 内首次派发的 call_id（模型消息面已有该结果）。
pub(crate) fn duplicate_query_note(query: &str, original_call_id: &str) -> String {
    format!(
        "[机械] 本 query（\"{query}\"）已于本 run 派发过（原 call_id={original_call_id}）；\
         未重复检索，结果见该次调用的既有返回。"
    )
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
        let line = countdown_line(3, 12, 0);
        assert!(line.contains("本批可用证据 3/5"), "{line}");
        assert!(line.contains("还剩 7 条（上限 10）"), "{line}");
        assert!(line.contains("已发起检索调用 12 次"), "{line}");
        // 到上限：剩余 0，不 panic、不出现负数。
        let at_cap = countdown_line(10, 20, 0);
        assert!(at_cap.contains("还剩 0 条"), "{at_cap}");
        // 0ax S1：合成数为 0 ⇒ 无可引用性披露（行形态不变）。
        assert!(!line.contains("合成"), "{line}");
        // 合成数 > 0 ⇒ 附「可引用性」披露（单列、不入可用额度）。
        let with_synthetic = countdown_line(2, 12, 5);
        assert!(
            with_synthetic.contains("另有 5 条为无 URL 合成文本，未计入可用额度"),
            "{with_synthetic}"
        );
    }

    // ── 0ax S1：无 URL 合成答案单列（不入可用额度） ─────────────────────

    /// 无 URL 的 web_search 证据（candidate_urls 空）不入可用计数、单列
    /// 合成计数；带引用池的 web_search 与普通 web_page 照常入数。
    #[test]
    fn synthetic_answers_are_counted_separately_and_excluded_from_usable() {
        // search_ev 产 web_search_result 形态（带 query、无引用池）。
        let no_url_search = search_ev("deepseek checkpoint format", "s1");
        assert!(is_synthetic_answer(&no_url_search));

        let mut with_url_search = search_ev("deepseek checkpoint format", "s2");
        with_url_search.candidate_urls = vec!["https://a.example".to_string()];
        assert!(!is_synthetic_answer(&with_url_search));

        let evidence = vec![
            no_url_search.clone(),
            no_url_search, // 同 digest 重复 ⇒ 合成计数也去重
            with_url_search,
            ev("full_text_observed", "p1", None), // web_page：照常入数
        ];
        assert_eq!(usable_source_count(&evidence), 2, "synthetic excluded");
        assert_eq!(synthetic_answer_count(&evidence), 1, "same-digest dedup");
    }

    // ── 0at A 面：派发谱系归因（origin_query_id） ───────────────────────

    fn search_ev(query: &str, digest: &str) -> EvidenceRecord {
        let mut e = ev("partial_text_observed", digest, Some(query));
        e.tool = "web_search".to_string();
        e.source_type = "web_search_result".to_string();
        e
    }

    /// 归因五规则的纯函数钉：逐字 ＞ 归一化 ＞ 候选池回溯 ＞ leader 兜底；
    /// 单 query 批恒 None。
    #[test]
    fn origin_query_assignment_follows_the_rule_ladder() {
        let queries = vec![
            "rust channel docs".to_string(),
            "tokio select pitfalls".to_string(),
        ];
        let ids = vec!["QRY-a".to_string(), "QRY-a-2".to_string()];
        // ① 逐字命中 q1。
        let exact = search_ev("rust channel docs", "e1");
        // ② 归一化命中 q2（大小写＋空白折叠）。
        let normalized = search_ev("TOKIO   select\npitfalls", "e2");
        // ④ 派生证据（无 query）identity 命中 ② 的候选池 ⇒ 跟随 q2。
        let mut pool_host = search_ev("tokio select pitfalls", "e3");
        pool_host.candidate_urls = vec!["https://tokio.rs/doc".to_string()];
        let derived = {
            let mut e = ev("full_text_observed", "e4", None);
            e.identity = "https://tokio.rs/doc".to_string();
            e.source_type = "web_page".to_string();
            e
        };
        // ⑤ 子代理自查串（改写、无池）⇒ leader 谱系（q1）。
        let rewritten = search_ev("DeepSpeed _scale_loss_by_gas", "e5");

        let evidence = vec![exact, normalized, pool_host, derived, rewritten];
        let got = origin_query_assignments(&queries, &evidence, &ids);
        assert_eq!(got[0].as_deref(), Some("QRY-a"), "exact → its query");
        assert_eq!(got[1].as_deref(), Some("QRY-a-2"), "normalized → its query");
        assert_eq!(got[2].as_deref(), Some("QRY-a-2"), "declared-match search");
        assert_eq!(
            got[3].as_deref(),
            Some("QRY-a-2"),
            "derived evidence follows the search whose pool it came from"
        );
        assert_eq!(
            got[4].as_deref(),
            Some("QRY-a"),
            "unmatched self-search falls back to the leader lineage"
        );

        // ① 单 query 批：全部 None（payload 逐字节不变的前提）。
        let single =
            origin_query_assignments(&["only".to_string()], &evidence, &["QRY-s".to_string()]);
        assert!(single.iter().all(|o| o.is_none()));
    }

    /// 0at B 面：未归因可用计数恒等式 `Σ 逐 query ＋ unattributed ＝ 批级`。
    #[test]
    fn unattributed_count_completes_the_per_query_identity() {
        let queries = vec!["q1".to_string(), "q2".to_string()];
        // ev() 产 web_page 形态（非合成口径）——带 search_query 的页面证据
        // 可按逐字 query 归因。
        let evidence = vec![
            ev("full_text_observed", "a", Some("q1")),
            ev("partial_text_observed", "b", Some("q1")),
            // 子代理自查串（不逐字匹配任何派发 query）。
            ev("partial_text_observed", "c", Some("rewritten query")),
            ev("full_text_observed", "d", Some("rewritten query")),
        ];
        let batch = usable_source_count(&evidence);
        let per_query: u64 = per_query_usable_counts(&queries, &evidence).iter().sum();
        let unattributed = unattributed_usable_count(&queries, &evidence);
        assert_eq!(batch, 4);
        assert_eq!(per_query, 2);
        assert_eq!(unattributed, 2);
        assert_eq!(per_query + unattributed, batch, "恒等式（判据 ①）");
    }

    /// 0at B 面（2026-09-20 审查修复批钉）：跨 query 重叠的饱和边界——
    /// 同一 digest 在两个派发 query 桶各出现一次时，桶内各自去重令
    /// Σ 逐 query 超过批级，unattributed 饱和到 0（恒等式在该批不成立；
    /// 装配点以机械告警留痕）。桶互斥时恒等式照常成立。
    #[test]
    fn unattributed_count_saturates_when_content_overlaps_two_queries() {
        let queries = vec!["q1".to_string(), "q2".to_string()];
        // 同 digest "x" 同时归 q1 与 q2；另有一条 q1 独享与一条未归因。
        let evidence = vec![
            ev("full_text_observed", "x", Some("q1")),
            ev("full_text_observed", "x", Some("q2")),
            ev("partial_text_observed", "y", Some("q1")),
            ev("full_text_observed", "z", None),
        ];
        assert_eq!(usable_source_count(&evidence), 3, "批级按 digest 全局去重");
        let per_query = per_query_usable_counts(&queries, &evidence);
        assert_eq!(per_query, vec![2, 1], "桶内各自去重（x 两桶各计 1）");
        let unattributed = unattributed_usable_count(&queries, &evidence);
        assert_eq!(unattributed, 0, "3 − 3 饱和到 0");
        assert_eq!(
            per_query.iter().sum::<u64>() + unattributed,
            usable_source_count(&evidence),
            "Σ(3) + 0 ≠ 批级(3) 的重叠形态：恒等式不成立（装配点告警）"
        );
    }

    /// S3 前去噪：空批不武装 β 收尾回合；阈值/护栏边界不变。
    #[test]
    fn empty_batch_never_arms_close_round() {
        assert_eq!(should_arm_close(0), None);
        assert_eq!(should_arm_close(1), None);
        assert_eq!(should_arm_close(4), None);
        assert_eq!(
            should_arm_close(SUFFICIENCY_TARGET),
            Some(BatchCloseKind::EvidenceThresholdMet)
        );
        assert_eq!(
            should_arm_close(MECHANICAL_CAP),
            Some(BatchCloseKind::MechanicalCapForceClose)
        );
    }

    /// S3 前去噪：query/url 去重键与空值语义。
    #[test]
    fn query_key_prefers_query_then_url_and_rejects_blanks() {
        assert_eq!(
            query_key(&serde_json::json!({"query": "  rust policy  "})).as_deref(),
            Some("rust policy")
        );
        assert_eq!(
            query_key(&serde_json::json!({"url": "https://example.com/a"})).as_deref(),
            Some("https://example.com/a")
        );
        assert_eq!(
            query_key(&serde_json::json!({"query": "   ", "url": "https://example.com/b"}))
                .as_deref(),
            Some("https://example.com/b")
        );
        assert_eq!(query_key(&serde_json::json!({"query": ""})), None);
        assert_eq!(query_key(&serde_json::json!({})), None);
    }

    /// S3 前去噪：重复 query 指针回执含原 call_id，且明确「未重复检索」。
    #[test]
    fn duplicate_query_note_points_back_to_original_call() {
        let note = duplicate_query_note("rust policy", "call-1");
        assert!(note.contains("call-1"), "{note}");
        assert!(note.contains("未重复检索"), "{note}");
        assert!(note.contains("rust policy"), "{note}");
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

    // ── 0au：派发前 run 墙钟余量判定（S1 落码钉子） ─────────────────────

    use std::time::Duration as StdDuration;

    /// 无已知 run 上限（ORZ_MAX_WALLCLOCK 缺席/为 0）⇒ 永不保留——自然
    /// 收工的 run 无预算可判（判据 ③：不误伤剩余充足/无上限路径）。
    #[test]
    fn wallclock_reserve_never_fires_without_a_known_limit() {
        assert_eq!(wallclock_reserved(None, StdDuration::from_secs(300)), false);
    }

    /// 判据 ①：余量 < 批墙钟＋收尾回合 ⇒ 保留——S3 四轮 trailing 实录
    /// 逐例回放（extract 26s、torch 117s、r1 torch 183s、r2 gpt2 123s 全被
    /// 拦；r3 的 462s 正常批不受扰）。
    #[test]
    fn wallclock_reserve_replays_the_s3_trailing_corpus() {
        let extended = StdDuration::from_secs(300);
        let standard = StdDuration::from_secs(180);
        // S3 extract 26s / torch 117s（近失）：< 300+60 ⇒ 保留。
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(26)),
            extended
        ));
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(117)),
            extended
        ));
        // r1 torch 183s、r2 gpt2 123s、r2 extract 191s：extended 下全保留。
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(183)),
            extended
        ));
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(123)),
            extended
        ));
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(191)),
            extended
        ));
        // r3 462s extended：360 需求 ⇒ 不保留（该批正常收口）。
        assert!(!wallclock_reserved(
            Some(StdDuration::from_secs(462)),
            extended
        ));
        // standard 180+60=240：300s 余量不保留（边界外）；239s 保留（边界内）。
        assert!(!wallclock_reserved(
            Some(StdDuration::from_secs(240)),
            standard
        ));
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(239)),
            standard
        ));
    }

    /// 尾部保留支：即便批墙钟极短（standard），距 run 终点不足 120s 一律
    /// 保留（S3 extract 26s 即使 standard 也必须拦住）。
    #[test]
    fn tail_reserve_holds_even_for_the_shortest_tier() {
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(119)),
            StdDuration::from_secs(180)
        ));
        // 恰好 120s：不小于保留额；但 < 180+60 ⇒ 仍由判据 ① 保留。
        assert!(wallclock_reserved(
            Some(StdDuration::from_secs(120)),
            StdDuration::from_secs(180)
        ));
    }
}
