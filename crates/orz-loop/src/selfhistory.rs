//! 模型自信息面按需查询（0p S1，2026-09-07，ADR-0010 §14.61 设计 A1/A2；
//! W2 压测 D-1 需求链：长会话模型需要恢复自身早期失败细节，折叠视图默认
//! 不含早期行、黑板不可 grep——本模块给 `blackboard_read section=exec` 补
//! 两个 PULL 查询面，全有界、零注入、不改预算面）：
//!
//! - **A1 `failures_only=true`**：F4 失败目标聚合行集（复用 P2-12 行语义：
//!   (kind,id) 身份、epoch 内累计计数、错误码集、首末墙钟、行内域段），
//!   整响应 ≤3K、截断显式标注、指针行恒可见。
//! - **A2 `search=<literal>`**：大小写不敏感**字面子串**（非正则）扫
//!   exec 动作/结果摘要 + actions receipt 摘要（terminal 订单输出在
//!   receipt 的 `response.output`——actions 结果栏仅留最近 50 条，本面
//!   是其唯一可检索视图），命中 ≤20 行（行 = 轮号 + exit + 摘要 +
//!   order_id 指针），截断显式标注。
//!
//! 口径（机械、确定性）：匹配在**全量文本**上判定、渲染行按既有 200 字符
//! 行截断（与折叠视图 FOLDABLE_ROW_MAX_CHARS 同口径）；互斥组合守卫在
//! host_exec 参数层 fail loud（同非法 epoch/receipt_id 纪律）。

use crate::blackboard::{ActionBoard, ExecSection};

/// A1 整响应字符预算（设计 A1：上限 3K 截断标注；与压缩注意事项槽
/// SUMMARY_SLOT_LIMITS[3] 同值级）。
pub const FAILURE_FACE_MAX_CHARS: usize = 3_000;

/// A2 命中行上限（设计 A2：命中 ≤20 行）。
pub const SEARCH_HIT_MAX_ROWS: usize = 20;

/// A2 单命中行渲染截断（与折叠视图行 200 字符同口径）。
pub const SEARCH_ROW_MAX_CHARS: usize = 200;

/// A1 失败聚合按需面——按存储序渲染 failure_agg 行集（P2-12 行语义），
/// 整响应 ≤3K；放不下的行计入「其余 N 条」；回查指针行恒可见。
/// 空聚合 = 「（无）」（P2-13 B3 空槽纪律）。
///
/// 0p S1 复审 F-B（2026-09-07）：设计原列的行级「receipt 指针」删除——
/// FailureAgg 不存 receipt 数据（record 无此参数），指针仅保留真实可得
/// 的 domain/round 展开面（设计文档 §3.A1 已随批勘误）。
pub fn render_failures_only(agg: &crate::failure_agg::FailureAgg) -> String {
    let header = "== exec · 失败聚合（failures_only=true）==";
    if agg.rows.is_empty() {
        return format!("{header}\n（无）");
    }
    let rows: Vec<String> = agg
        .rows
        .iter()
        .map(crate::summary::render_failure_target_row)
        .collect();
    let total = rows.len();
    let pointer = "回查：域段 r 范围用 section=exec + domain/round_from/round_to 展开原文";
    // 尾部预留 = 「\n」+ 截断注记上限（~80，含位数）——指针行与注记都
    // 必须可见且计入 3K 预算（F-H 复审修正：注记不入预算会让整响应
    // 超出上限）。
    const FACE_TAIL_RESERVE: usize = 96;
    let mut out = String::from(header);
    let mut shown = 0usize;
    for line in &rows {
        let candidate = format!("{out}\n{line}");
        if candidate.chars().count() + 1 + pointer.chars().count() + FACE_TAIL_RESERVE
            <= FAILURE_FACE_MAX_CHARS
        {
            out = candidate;
            shown += 1;
        }
    }
    let hidden = total - shown;
    if hidden > 0 {
        out.push_str(&format!(
            "\n（其余 {hidden} 条失败目标未显示——failures_only 整响应 3K 上限；\
             按 preview 关键词用 search 精确定位）"
        ));
    }
    out.push('\n');
    out.push_str(pointer);
    out
}

/// A2 自历史字面检索——exec results / errors / actions receipts 三源按
/// 存储序扫描；大小写不敏感字面子串（非正则）；匹配在全量文本、渲染行
/// ≤200 字符；命中 ≤20 行，溢出显式标注。
///
/// exit 标签（0p S1 复审 F-A，2026-09-07）：
/// - exec 行 `exit=N` = 命令真实退出码（run_terminal_cmd/run_tests Ok 臂
///   回填；N≠0 即命令级失败）；`exit=0` = 命令成功；
/// - exec 行 `exit=ok`/`exit=err` = 工具级成功（无命令退出语义，如
///   read_file）/ host 级 ToolError；
/// - receipt 行按 `receipt.ok` 标 ok/err，行含 order_id 指针。
pub fn render_exec_search(exec: &ExecSection, actions: &ActionBoard, query: &str) -> String {
    let needle = query.to_lowercase();
    let mut hits: Vec<String> = Vec::new();
    let mut total = 0usize;
    for e in &exec.results {
        if e.text.to_lowercase().contains(&needle) {
            total += 1;
            hits.push(format!(
                "r{} | {} | {}",
                e.round,
                exec_exit_label(e.exit_code, false),
                crate::summary::truncate_chars(&e.text, SEARCH_ROW_MAX_CHARS)
            ));
        }
    }
    for e in &exec.errors {
        if e.text.to_lowercase().contains(&needle) {
            total += 1;
            hits.push(format!(
                "r{} | {} | {}",
                e.round,
                exec_exit_label(e.exit_code, true),
                crate::summary::truncate_chars(&e.text, SEARCH_ROW_MAX_CHARS)
            ));
        }
    }
    for r in &actions.results {
        let summary = receipt_summary(r);
        if summary.to_lowercase().contains(&needle) {
            total += 1;
            hits.push(format!(
                "r{} | exit={} | {} | {}",
                r.round,
                if r.ok { "ok" } else { "err" },
                r.order_id,
                crate::summary::truncate_chars(&summary, SEARCH_ROW_MAX_CHARS)
            ));
        }
    }
    // F-D（0p S1 复审，2026-09-07）：表头回显 query 截断 ≤100 字符——
    // 匹配仍用全量 query，响应有界纪律不因 echo 破坏。
    let header = format!(
        "== exec · search \"{}\"（命中 {total} 条）==",
        crate::summary::truncate_chars(query, 100)
    );
    if total == 0 {
        return format!("{header}\n（无命中）");
    }
    let mut out = header;
    let shown = hits.len().min(SEARCH_HIT_MAX_ROWS);
    for line in hits.iter().take(shown) {
        out.push('\n');
        out.push_str(line);
    }
    let hidden = total - shown;
    if hidden > 0 {
        out.push_str(&format!(
            "\n（另有 {hidden} 条命中未显示——search 命中上限 {SEARCH_HIT_MAX_ROWS} 行；\
             缩小关键词，或按行首轮号用 domain/round_from/round_to 展开原文）"
        ));
    }
    out
}

/// exec 行 exit 标签：`exit=N`（命令真实退出码，N≠0 = 命令级失败）/
/// `exit=0`（命令成功）/ `exit=ok`（工具级成功，无命令退出语义）/
/// `exit=err`（host 级 ToolError）。None 时按 results/errors 臂回退
/// 工具级标签——旧板/pre-stamp 行与无退出语义工具的诚实形态。
fn exec_exit_label(exit_code: Option<i32>, is_error_arm: bool) -> String {
    match exit_code {
        Some(n) => format!("exit={n}"),
        None if is_error_arm => "exit=err".to_string(),
        None => "exit=ok".to_string(),
    }
}

/// actions receipt 的可检索摘要——ok 走 `response.output`（terminal /
/// run_tests 文本输出信封形状 `{"output": string}`），无该字段时整信封
/// JSON（确定性序列化，匹配面不漏）；err 走 error 信封 step/code/message
/// （message 是早期失败细节的主要载体，必须进匹配面）。
fn receipt_summary(result: &crate::blackboard::ActionResult) -> String {
    let action = result.action.as_deref().unwrap_or("?");
    if result.ok {
        let output = result
            .response
            .as_ref()
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("output"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                result
                    .response
                    .as_ref()
                    .map(|j| serde_json::to_string(j).unwrap_or_else(|_| "<unserializable>".into()))
                    .unwrap_or_else(|| "(no response)".into())
            });
        format!("{action}: {output}")
    } else {
        let (step, code) = crate::summary::failure_envelope_fields(&result.error);
        let message = result
            .error
            .as_ref()
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        format!("{action}: step={step} code={code} {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{ActionResult, ExecEntry};
    use crate::failure_agg::FailureAgg;
    use orz_assurance::lif::Domain;

    fn agg_with_rows() -> FailureAgg {
        let mut agg = FailureAgg::default();
        agg.record(
            "cmd_target",
            "id-a",
            "python train.py",
            "tool_timeout",
            10.0,
            3,
            Domain::Normal,
        );
        agg.record(
            "cmd_target",
            "id-a",
            "python train.py",
            "execution_failed",
            90.0,
            7,
            Domain::Pressure,
        );
        agg
    }

    // ---- A1 failures_only ----

    #[test]
    fn failures_only_renders_p2_12_rows_and_empty_slot() {
        let out = render_failures_only(&agg_with_rows());
        assert!(out.starts_with("== exec · 失败聚合（failures_only=true）=="));
        // P2-12 行语义：身份预览 + 计数 + 错误码集 + 首末墙钟 + 域段序列。
        assert!(out.contains("[失败目标 cmd_target] python train.py ×2"));
        assert!(out.contains("codes=[tool_timeout×1, execution_failed×1]"));
        assert!(out.contains("首末 10s–90s"));
        assert!(out.contains("normal(r3)→pressure(r7)"));
        // 指针行恒可见（回查教学；F-B 后仅保留 domain/round 展开面）。
        assert!(out.contains("domain/round_from/round_to"));
        assert!(!out.contains("receipt_id"));
        // 空聚合 = 空槽「（无）」。
        let empty = render_failures_only(&FailureAgg::default());
        assert!(empty.contains("（无）"));
    }

    #[test]
    fn failures_only_caps_at_3k_with_explicit_truncation() {
        let mut agg = FailureAgg::default();
        // 每行 ~120 字符 × 60 行 ≈ 7.2K > 3K：必然触发截断。
        for i in 0..60 {
            agg.record(
                "cmd_target",
                &format!("id-{i:03}"),
                &format!("python train_step_{i:03}.py --epoch 32"),
                "tool_timeout",
                10.0 + i as f64,
                i + 1,
                Domain::Normal,
            );
        }
        let out = render_failures_only(&agg);
        assert!(out.chars().count() <= FAILURE_FACE_MAX_CHARS);
        assert!(out.contains("条失败目标未显示"));
        assert!(out.contains("3K 上限"));
        // 指针行仍在（截断不吞指针；F-B 后无 receipt 字样）。
        assert!(out.contains("domain/round_from/round_to"));
        assert!(!out.contains("receipt_id"));
    }

    #[test]
    fn failures_only_single_oversized_row_stays_bounded_with_pointer() {
        let mut agg = FailureAgg::default();
        // 单行远超 3K 预算的极端形态：0 行可见 + 截断注记 + 指针仍在，
        // 输出保持有界（F-H 边界）。
        agg.record(
            "cmd_target",
            "id-huge",
            &"x".repeat(FAILURE_FACE_MAX_CHARS * 2),
            "tool_timeout",
            10.0,
            1,
            Domain::Normal,
        );
        let out = render_failures_only(&agg);
        assert!(out.chars().count() <= FAILURE_FACE_MAX_CHARS);
        assert!(out.contains("1 条失败目标未显示"));
        assert!(out.contains("domain/round_from/round_to"));
    }

    // ---- A2 search ----

    fn exec_entry(round: u64, text: &str) -> ExecEntry {
        let mut e = ExecEntry::stamped(
            text.to_string(),
            round,
            Domain::Normal,
            "2026-09-07T00:00:00Z".to_string(),
        );
        e.exit_code = None;
        e
    }

    fn exec_entry_exit(round: u64, text: &str, exit_code: Option<i32>) -> ExecEntry {
        let mut e = exec_entry(round, text);
        e.exit_code = exit_code;
        e
    }

    // ---- F-A exit 标签三态（0p S1 复审）----

    #[test]
    fn exec_rows_render_real_exit_code_not_tool_level_ok() {
        let mut exec = ExecSection::default();
        // 命令级失败：工具 Ok 臂 + exit_code=1 → exit=1（不再是 exit=ok）。
        exec.results.push(exec_entry_exit(
            12,
            "[run_terminal_cmd] pip install fasttext\r\nMemoryError: bad allocation",
            Some(1),
        ));
        // 命令成功：exit_code=0 → exit=0。
        exec.results
            .push(exec_entry_exit(13, "[run_terminal_cmd] echo done", Some(0)));
        // 无命令退出语义（read_file 类）：None → 工具级 exit=ok。
        exec.results
            .push(exec_entry(14, "[read_file] src/main.rs 200 lines"));
        let out = render_exec_search(&exec, &ActionBoard::default(), "exit-probe-no-hit");
        assert!(out.contains("（无命中）"));
        let out = render_exec_search(&exec, &ActionBoard::default(), "pip install");
        assert!(out.contains("r12 | exit=1 | "), "{out}");
        let out = render_exec_search(&exec, &ActionBoard::default(), "echo done");
        assert!(out.contains("r13 | exit=0 | "), "{out}");
        let out = render_exec_search(&exec, &ActionBoard::default(), "src/main.rs");
        assert!(out.contains("r14 | exit=ok | "), "{out}");
    }

    // ---- F-D 表头 query 截断 ----

    #[test]
    fn search_header_truncates_long_query_echo() {
        let long_query = "k".repeat(500);
        let out = render_exec_search(
            &ExecSection::default(),
            &ActionBoard::default(),
            &long_query,
        );
        assert!(
            out.chars().count() < 300,
            "header echo must be bounded: {out}"
        );
        assert!(out.contains("（无命中）"));
    }

    fn receipt(order_id: &str, ok: bool, output: &str) -> ActionResult {
        ActionResult {
            order_id: order_id.to_string(),
            action: Some("workspace.run_terminal".to_string()),
            ok,
            response: if ok {
                Some(serde_json::json!({ "output": output }))
            } else {
                None
            },
            round: 5,
            domain: Some(Domain::Normal),
            error: if ok {
                None
            } else {
                Some(serde_json::json!({
                    "step": "execute",
                    "code": "execution_failed",
                    "message": output,
                }))
            },
            trace_id: "t-test".to_string(),
            timestamp: "2026-09-07T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn search_hits_early_failure_rows_case_insensitive_literal() {
        let mut exec = ExecSection::default();
        // 早期轮失败行（折叠视图默认不含的行）。
        exec.errors.push(exec_entry(
            2,
            "[workspace.run_terminal] pip install fasttext → MemoryError: bad allocation",
        ));
        // 无关行。
        exec.results.push(exec_entry(3, "[read_file] ok 200 chars"));
        let actions = ActionBoard::default();
        let out = render_exec_search(&exec, &actions, "MEMORYERROR");
        // 大小写不敏感命中早期失败行；行 = 轮号 + exit + 摘要。
        assert!(out.contains("== exec · search \"MEMORYERROR\"（命中 1 条）=="));
        assert!(out.contains("r2 | exit=err |"));
        assert!(out.contains("MemoryError: bad allocation"));
    }

    #[test]
    fn search_hits_action_receipts_and_carries_order_id_pointer() {
        let exec = ExecSection::default();
        let mut actions = ActionBoard::default();
        actions
            .results
            .push(receipt("ORD-000042", true, "g++ -O2 train.cpp -o train"));
        let out = render_exec_search(&exec, &actions, "train.cpp");
        assert!(out.contains("命中 1 条"));
        // receipt 行带 order_id 指针（receipt_id 点读入口）。
        assert!(out.contains("r5 | exit=ok | ORD-000042 |"));
        assert!(out.contains("g++ -O2 train.cpp -o train"));
    }

    #[test]
    fn search_caps_at_20_rows_with_explicit_overflow() {
        let mut exec = ExecSection::default();
        for i in 0..25 {
            exec.results.push(exec_entry(
                i + 1,
                &format!("[workspace.run_terminal] cargo build fasttext-step-{i}"),
            ));
        }
        let out = render_exec_search(&exec, &ActionBoard::default(), "fasttext-step");
        assert!(out.contains("命中 25 条"));
        assert_eq!(out.lines().count() - 1, 20 + 1); // 表头 + 20 行 + 溢出注记
        assert!(out.contains("另有 5 条命中未显示"));
        assert!(out.contains("命中上限 20 行"));
    }

    #[test]
    fn search_no_hit_is_explicit_and_literal_not_regex() {
        let mut exec = ExecSection::default();
        exec.results.push(exec_entry(
            1,
            "[workspace.run_terminal] pip install fasttext",
        ));
        // 字面检索：`.` 不展开为任意字符。
        let out = render_exec_search(&exec, &ActionBoard::default(), "pip.instalx");
        assert!(out.contains("（无命中）"));
        let out2 = render_exec_search(&exec, &ActionBoard::default(), "fasttext");
        assert!(out2.contains("命中 1 条"));
    }
}
