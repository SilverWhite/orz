//! Action-ledger mechanical collapse (P0-D S2, 2026-08-14).
//!
//! ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §3: after every model tool
//! round, the MODEL-VISIBLE context collapses completed OLD rounds into one
//! deterministic action-ledger block — tool name, target path/URL, result
//! pointer (sha256 of the tool-result content; the full content stays in
//! the journal / conversation sidecar / P4 audit) and the round's non-empty
//! final reply. Zero model calls, no cooldown, whole-round pairing
//! discipline (a round is never split — every surviving tool reply keeps a
//! matching declaration).
//!
//! The persisted `messages` (the conversation) is never mutated here: the
//! collapsed view is built per model request, so the sidecar keeps the full
//! tool records for audit and the model context keeps only ledger rows plus
//! the bounded recent tail.

use crate::gateway::model::{Message, Role, ToolCall};
use orz_assurance::journal::sha256_hex;

/// Prefix of the model-visible action-ledger block (`[动作台账 v0.1] …`).
/// Registered in `is_injected_block_text` (mechanical injected text).
pub const ACTION_LEDGER_PREFIX: &str = "[动作台账";

/// Version marker of the ledger block format.
pub const ACTION_LEDGER_VERSION: &str = "v0.1";

/// Newest tool rounds kept verbatim in the model-visible context (bounded
/// exact tail; older rounds collapse into ledger rows).
pub const DEFAULT_RECENT_TAIL_ROUNDS: usize = 2;

/// FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
/// §14.28): the fixed pointer-message prefix injected into the folded
/// request view (`[U0][preamble][pointer][桥]`). The message is byte-fixed
/// for the session — the external ledger file is the only growing part, so
/// the request prefix stays byte-stable across advances (the per-window
/// rewrite is gone; measured hit rate 81.9% → ~91–93%). FUS-LEDGER-FOLD-
/// BRIDGE (2026-08-19, ADR-0010 §14.32): the bridge is the newest complete
/// rounds within the 8K real-token budget (形态甲: 先定裁剪、再内容截断);
/// the pointer text was updated once for the bridge wording (deployment
/// fingerprint change accepted by §3.4).
/// 滑块上下文 v8（2026-09-16 勘误批）：模型面文案纪律——不用「折叠／折叠桥」
/// 等内部沿革词，改为「上下文分块」（模型可见的正名是「当前上下文窗口」＋
/// 「主滑块」＋「分块」）。常量名保留（大量调用面沿用），仅前缀文案更名。
pub const LEDGER_FOLD_POINTER_PREFIX: &str = "【上下文分块】";

/// The append-only model-readable ledger projection file (per session;
/// run dirs are naturally isolated). Design: 追加式写入、只增不轮转（压缩
/// 不重置）；序号续点以文件为准——推进时读文件尾行取最大序号 +1。
pub fn ledger_file_path(session_cwd: &std::path::Path) -> std::path::PathBuf {
    session_cwd.join(".gsa").join("ledger").join("current.md")
}

/// The byte-fixed pointer message of the model face. 路径/文本均固定（本会话内
/// 不变），不含任何变化 ID/序号 ⇒ 两次压缩之间前缀字节稳定（不变量 I6）。
///
/// **v8 勘误（2026-09-16）**：v7 文案称「当前上下文窗口只保留最近若干完整轮」
/// ——那是**机械驱逐**语义（随勘误退役）。v8 模型面**累积**全部内容：更早轮次
/// 按**分块**留在窗口内（分块表为索引），机械摘要行只是外挂台账里的结构化
/// 投影；被压缩／被截断的分块可按块回放。文案不含任何参数值（改 env 即一次性
/// 前缀失效；配置固定性优先于精确表述，同 v7 §3.1 纪律）。
pub fn build_pointer_message(ledger_path: &std::path::Path) -> String {
    format!(
        "{LEDGER_FOLD_POINTER_PREFIX}更早轮次的**机械摘要行**（工具／命令／结果类的结构化投影）\
         外挂存档于 {}（本会话内固定）。\n\
         需要回顾时按行检索该文件，例如 grep \"轮次\" {}、grep <工具名> {}。\n\
         你当前**上下文窗口**里的旧内容按**分块**累积（分块表是索引，见其后）；\
         被压缩或被截断的分块仍**全量留档**、可按块回放（回放路径见分块表与压缩告知块）。",
        ledger_path.display(),
        ledger_path.display(),
        ledger_path.display(),
    )
}

/// 0ae D4（2026-09-15，设计 §7）：run 起始基线捕获——`git rev-parse HEAD`
/// + `git status --porcelain`（零模型调用，run 起始一次）；非 git 工作区
/// 或 git 失败 = `None`（不渲染基线段，其余两段照常）。解决「362 行
/// 自产代码被表述为上一轮遗留」的出处失忆与基线失忆。
pub fn capture_run_baseline(cwd: &std::path::Path) -> Option<String> {
    let head = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(cwd)
        .output()
        .ok()?;
    if !head.status.success() {
        return None;
    }
    let dirty = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(cwd)
        .output()
        .map(|out| out.status.success() && !out.stdout.is_empty())
        .unwrap_or(false);
    Some(format!(
        "HEAD={}（run 起始 worktree：{}）",
        String::from_utf8_lossy(&head.stdout).trim(),
        if dirty {
            "含未提交改动"
        } else {
            "干净"
        }
    ))
}

/// 最近编辑指纹条数（设计 §7③「最近 N 次」，首版 N=5）。
pub const RUN_CONTEXT_RECENT_EDITS: usize = 5;

/// 0ae D4：折叠桥机械段渲染——①本 run 自编辑文件清单（路径 + 次数 +
/// 末次时间）②会话历史自编辑文件（此前 run / 无章旧行）③run 起始基线
/// ④最近 N 次编辑指纹。数据源 = 黑板 edits 分区（机械单写者，模型不可
/// 伪造）。0AE-C11 处置（2026-09-17，盘点 FR-C04 采②分 run 标注）：清单
/// 按 run 章（`EditRecord.run`）分组——「本 run」段只含当前 run 的编辑，
/// 跨 run 残留与旧无章行落「会话历史」段，多 run 会话不再被「本 run」
/// 标签夸大归属；`current_run = None`（无章可对）时全部归历史段。
pub fn render_run_context_block(
    baseline: Option<&str>,
    edits: &[crate::blackboard::EditRecord],
    current_run: Option<&str>,
) -> String {
    let mut lines = Vec::new();
    if let Some(baseline) = baseline {
        lines.push(format!("【本 run 基线】{baseline}"));
    }
    // 文件聚合（路径 + 次数 + 末次时间；保序）——本 run / 历史两组共用。
    let summarize = |records: &[&crate::blackboard::EditRecord]| -> Option<String> {
        if records.is_empty() {
            return None;
        }
        let mut order: Vec<&str> = Vec::new();
        let mut counts: std::collections::HashMap<&str, (usize, &str)> =
            std::collections::HashMap::new();
        for record in records {
            match counts.get_mut(record.file.as_str()) {
                Some((count, last)) => {
                    *count += 1;
                    *last = record.timestamp.as_str();
                }
                None => {
                    order.push(record.file.as_str());
                    counts.insert(record.file.as_str(), (1, record.timestamp.as_str()));
                }
            }
        }
        Some(
            order
                .iter()
                .map(|file| {
                    let (count, last) = counts.get(file).copied().expect("registered above");
                    format!("{file} ×{count}（末次 {last}）")
                })
                .collect::<Vec<_>>()
                .join("；"),
        )
    };
    let is_current = |record: &crate::blackboard::EditRecord| {
        matches!(current_run, Some(run) if !run.is_empty()) && record.run == current_run.unwrap()
    };
    let current: Vec<&crate::blackboard::EditRecord> =
        edits.iter().filter(|r| is_current(r)).collect();
    let history: Vec<&crate::blackboard::EditRecord> =
        edits.iter().filter(|r| !is_current(r)).collect();
    if let Some(summary) = summarize(&current) {
        lines.push(format!("【本 run 自编辑文件】{summary}"));
    }
    if let Some(summary) = summarize(&history) {
        let prior_runs: std::collections::BTreeSet<&str> = history
            .iter()
            .map(|record| record.run.as_str())
            .filter(|run| !run.is_empty())
            .collect();
        let has_legacy = history.iter().any(|record| record.run.is_empty());
        let mut label = String::from("【会话历史自编辑文件");
        match (prior_runs.len(), has_legacy) {
            (0, true) => label.push_str("（无章旧行）"),
            (n, false) => label.push_str(&format!("（此前 {n} 个 run）")),
            (n, true) => label.push_str(&format!("（此前 {n} 个 run ＋ 无章旧行）")),
        }
        label.push('】');
        lines.push(format!("{label}{summary}"));
    }
    if !edits.is_empty() {
        let recent: Vec<String> = edits
            .iter()
            .rev()
            .take(RUN_CONTEXT_RECENT_EDITS)
            .map(|record| {
                format!(
                    "[{}] {} +{}/-{}",
                    record.timestamp, record.file, record.new_lines, record.old_lines
                )
            })
            .collect();
        lines.push(format!("【最近编辑指纹】{}", recent.join("；")));
    }
    lines.join(
        "
",
    )
}

/// 用户 2026-09-15 裁定（窗口内溢出处置）：**超大工具结果指针化**的正文前缀
/// ——`Role::Tool` 消息内容被换成「短头部 ＋ 回读指针」后的可识别标记
/// （幂等判定用；Tool 消息不属注入块过滤面，无需注册 `is_injected_block_text`）。
pub const TOOL_RESULT_POINTERIZED_PREFIX: &str = "[工具结果已机械指针化";

/// 指针化时保留的原文头部字符数（让模型仍认得这个结果是什么）。
const TOOL_RESULT_POINTER_HEAD_CHARS: usize = 400;

/// **回放块（可再生）**指针前缀（2026-09-16 实现批，设计 §6/§12「先裁回放块」）：
/// 模型读按块档案产生的结果，其原文**仍在按块档案里**，重读即恢复 ⇒ 线上溢出时
/// 它优先于原始内容被移出模型上下文。
pub const REPLAY_TOOL_RESULT_PREFIX: &str = "[回放块（可再生）]";

/// 按块回放档案的路径特征（判定一次工具调用是否为「回放读」）。
const BLOCK_ARCHIVE_PATH_MARKERS: [&str; 2] = ["compaction/blocks", "compaction\\blocks"];

/// 「超大结果」门槛（估计口径 chars/2）：**超过**此值才是指针化候选。
/// 依据：实测多数读/计划轮 1–3K、终端执行轮 5–7K（08-19 桥预算同源读数）
/// ⇒ 8K 以下不指针化，避免把正常工作现场磨成指针。
pub const OVERSIZED_TOOL_RESULT_CAP_TOKENS: u64 = 8_000;

/// 指针化统计（用户 2026-09-15 裁定；落账与告知均用这两个读数）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PointerizeStats {
    /// 被换成指针的工具结果条数。
    pub replaced: u32,
    /// 释放的估算 token（估计口径 chars/2）。
    pub freed_tokens: u64,
}

/// **窗口内超大工具结果指针化**（用户 2026-09-15 裁定；先例＝
/// OUTPUT-DEGENERATION-GUARD／ADR-0010 §14.33「落盘 ＋ 回读指针 ＋
/// `read_file` 分页」）：硬线截留（移出窗口外轮次）之后仍在线之上、而溢出
/// 体量位于**当前上下文窗口内**（单轮／单结果过大）时，把超过
/// [`OVERSIZED_TOOL_RESULT_CAP_TOKENS`] 的**工具结果正文**换成
/// 「原文头部（≤400 字符）＋ 回读指针」，直到估算 ≤ `target_estimate` 或没有
/// 候选；**大者优先**（用最少条数换最多空间），不动非工具消息。
///
/// 不变量：① **配对不破坏**——只改 `content`，`role`／`tool_call_id` 一律不动
/// （否则重放必 400）；② **幂等**——已带
/// [`TOOL_RESULT_POINTERIZED_PREFIX`] 的结果不再入选；③ **不假装新鲜**——
/// 指针行显式声明「这是读取当时的快照，可能已陈旧，编辑/决策前以新鲜读取为准」。
pub fn pointerize_oversized_tool_results(
    messages: &mut [Message],
    target_estimate: u64,
    per_result_cap: u64,
    archive_dir: &std::path::Path,
    archive_tag: &str,
    journal_path: Option<&std::path::Path>,
    run_id: &str,
) -> PointerizeStats {
    let mut stats = PointerizeStats::default();
    if per_result_cap == 0 {
        return stats;
    }
    // 落盘失败而被跳过的候选（保留正文，不指针化）。
    let mut blocked: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    let mut estimate = crate::controller::estimate_messages_tokens(messages);
    while estimate > target_estimate {
        // 大者优先：选当前最大的合格候选（已指针化/未超门槛者不入围）。
        let mut best: Option<(usize, u64)> = None;
        for (idx, m) in messages.iter().enumerate() {
            if blocked.contains(&idx) {
                continue;
            }
            if m.role != Role::Tool || m.content.starts_with(TOOL_RESULT_POINTERIZED_PREFIX) {
                continue;
            }
            let est = crate::controller::estimate_message_tokens(m);
            if est <= per_result_cap {
                continue;
            }
            if best.is_none_or(|(_, b)| est > b) {
                best = Some((idx, est));
            }
        }
        let Some((idx, before)) = best else { break };
        let original = messages[idx].content.clone();
        let call_id = messages[idx].tool_call_id.clone().unwrap_or_default();
        // 审查 R-7 修复（2026-09-16）：**改前先落逐字原文**——journal 的
        // `tool_completed` 不带工具结果正文（只有工具/调用 id/退出码），旧文案
        // 「原文仍在本地档案、按 call_id 从 journal 回读」对读类结果不成立。
        // 落盘失败 ⇒ **不指针化**（不能让唯一副本消失）。
        let archive_path = oversized_archive_path(archive_dir, archive_tag, &call_id);
        let Some(parent) = archive_path.parent().map(|p| p.to_path_buf()) else {
            break;
        };
        let body = format!(
            "# ORZ 窗口内超大工具结果原文（指针化前落盘）\n\n\
             - run: {run_id}\n- call_id: {}\n- 估算: ≈{before}tk token（chars/2）\n\
             - 生成时间: {}\n\n\
             > 该结果正文已从模型上下文移出（换成「头部 ＋ 回读指针」）；本文件是\n\
             > **移出当时的逐字原文**，按 read_file offset/limit 分页回读。\n\n{original}\n",
            if call_id.is_empty() {
                "（无）"
            } else {
                call_id.as_str()
            },
            crate::controller::chrono_utc_now(),
        );
        if !crate::summary::write_archive_retry(parent.as_path(), &archive_path, &body) {
            // 落盘失败：跳过本条（保留正文），尝试下一条候选；全失败即收手。
            blocked.insert(idx);
            continue;
        }
        let replacement = tool_result_pointer_line(
            &original,
            &archive_path,
            journal_path,
            run_id,
            messages[idx].tool_call_id.as_deref(),
        );
        let after = replacement.chars().count() as u64 / 2;
        messages[idx].content = replacement;
        stats.replaced += 1;
        let freed = before.saturating_sub(after);
        stats.freed_tokens = stats.freed_tokens.saturating_add(freed);
        estimate = estimate.saturating_sub(freed);
    }
    stats
}

/// 指针化原文的落盘路径（`.gsa/compaction/pointerized/<tag>-<call_id>.md`）。
fn oversized_archive_path(
    archive_dir: &std::path::Path,
    archive_tag: &str,
    call_id: &str,
) -> std::path::PathBuf {
    let safe: String = call_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let safe = if safe.is_empty() {
        "call".to_string()
    } else {
        safe
    };
    archive_dir.join(format!("{archive_tag}-{safe}.md"))
}

/// 指针行文本（头部 ＋ **逐字原文落盘指针**）。
///
/// 2026-09-16 实现批（审查 R-7）：v7/v8 早期文案称「原文仍在本地档案、按
/// `call_id` 从 run journal 回读」——但 journal 的 `tool_completed` 只带
/// 工具名/调用 id/退出码，**不带结果正文**（只有终端等截断输出族才有
/// `output_object_id`）⇒ 对读类结果该承诺不成立。现在正文改前先落盘，指针
/// 指向真实载体；journal 仍作为关联索引（`call_id`）如实并列。
fn tool_result_pointer_line(
    original: &str,
    archive_path: &std::path::Path,
    journal_path: Option<&std::path::Path>,
    run_id: &str,
    call_id: Option<&str>,
) -> String {
    let chars = original.chars().count();
    let head: String = original
        .chars()
        .take(TOOL_RESULT_POINTER_HEAD_CHARS)
        .collect();
    let call = call_id.unwrap_or("（无）");
    let journal = match journal_path {
        Some(p) => format!(
            "run journal {}（按 `call_id={call}` 检索该次调用的事件面；\
             journal 不含结果正文，逐字原文以上面落盘文件为准）",
            p.display()
        ),
        None => format!("run journal（{run_id}；按 `call_id={call}` 检索该次调用）"),
    };
    format!(
        "{TOOL_RESULT_POINTERIZED_PREFIX}（窗口内超大结果）] 正文 {chars} 字符已移出模型上下文\
         ——**逐字原文已落盘**：{archive}（read_file offset/limit 分页；\
         首读若收到 session_volume_notice 通知信封，再读一次即放行）。\n\
         关联索引：{journal}；终端输出的完整日志另见其结果尾部原有指针。\n\
         这是**读取当时的快照**，\
        可能已陈旧：编辑或据此决策前请以新鲜读取为准。\n\
         == 原文头部（前 {head_chars} 字符） ==\n{head}",
        archive = archive_path.display(),
        head_chars = TOOL_RESULT_POINTER_HEAD_CHARS
    )
}

/// **回放块优先指针化**（2026-09-16 实现批；设计 §6/§12「回放到线时先裁回放块
/// ——可再生」）：模型读按块档案产生的结果，其原文仍在按块档案里 ⇒ 线上仍溢出
/// 时**优先**把它换成指针（重读原路径即恢复），再考虑更旧的原始内容。
///
/// 判定＝按 `tool_call_id` 找回想该结果的声明消息，其 `arguments` 里出现
/// `compaction/blocks` 即视为回放读；**大者优先**，直到 ≤ `target_estimate`
/// 或没有候选。不变量：只改 `content`（配对不动）、已指针化者不再入选、
/// 声明缺失时保守跳过（不猜）。
pub fn pointerize_replay_tool_results(
    messages: &mut [Message],
    target_estimate: u64,
    archive_root: &std::path::Path,
) -> PointerizeStats {
    let mut stats = PointerizeStats::default();
    // tool_call_id → 回放档案路径（判定 + 指针文案均用真实路径）。
    let mut replay_calls: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for m in messages.iter() {
        for tc in &m.tool_calls {
            let rendered = tc.arguments.to_string();
            if !BLOCK_ARCHIVE_PATH_MARKERS
                .iter()
                .any(|marker| rendered.contains(marker))
            {
                continue;
            }
            let path = string_values(&tc.arguments)
                .into_iter()
                .find(|v| {
                    BLOCK_ARCHIVE_PATH_MARKERS
                        .iter()
                        .any(|marker| v.contains(marker))
                })
                .unwrap_or_else(|| rendered.clone());
            replay_calls.insert(tc.call_id.clone(), path);
        }
    }
    if replay_calls.is_empty() {
        return stats;
    }
    let mut estimate = crate::controller::estimate_messages_tokens(messages);
    while estimate > target_estimate {
        let mut best: Option<(usize, u64)> = None;
        for (idx, m) in messages.iter().enumerate() {
            if m.role != Role::Tool
                || m.content.starts_with(TOOL_RESULT_POINTERIZED_PREFIX)
                || m.content.starts_with(REPLAY_TOOL_RESULT_PREFIX)
            {
                continue;
            }
            let Some(call) = m.tool_call_id.as_deref() else {
                continue;
            };
            if !replay_calls.contains_key(call) {
                continue;
            }
            let est = crate::controller::estimate_message_tokens(m);
            if best.is_none_or(|(_, b)| est > b) {
                best = Some((idx, est));
            }
        }
        let Some((idx, before)) = best else { break };
        let call = messages[idx].tool_call_id.clone().unwrap_or_default();
        let path = replay_calls.get(&call).cloned().unwrap_or_default();
        let replacement = replay_pointer_line(&path, archive_root, &call);
        let after = replacement.chars().count() as u64 / 2;
        messages[idx].content = replacement;
        stats.replaced += 1;
        stats.freed_tokens = stats
            .freed_tokens
            .saturating_add(before.saturating_sub(after));
        estimate = estimate.saturating_sub(before.saturating_sub(after));
    }
    stats
}

fn replay_pointer_line(path: &str, archive_root: &std::path::Path, call: &str) -> String {
    let display = if path.is_empty() {
        archive_root.display().to_string()
    } else {
        path.to_string()
    };
    format!(
        "{REPLAY_TOOL_RESULT_PREFIX} 本条是**回放块**（当时按块读回的历史原文），已在溢出时\
         优先移出模型上下文（可再生）：原文仍在该按块档案里，重读即恢复。\n\
         重读指针：{display}（read_file offset/limit 分页；\
         首读若收到 session_volume_notice 通知信封，再读一次即放行）。\n\
         关联索引：call_id={call}。这是**读取当时的快照**，可能已陈旧。"
    )
}

/// 递归收集 JSON 里的字符串值（回放路径提取用）。
fn string_values(value: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(items) => {
            for item in items {
                out.extend(string_values(item));
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                out.extend(string_values(item));
            }
        }
        _ => {}
    }
    out
}

/// One external-file row: `[<全局序号>] 轮次 <窗口内轮次>: <工具> 目标=…
/// 结果=… 最终回复=…`。`[<全局序号>]` 为按行续点的 per-row 全局序号
/// （跨压缩连续；多结果轮按条各占一行、序号连续递增），文件只增，读尾行
/// 即可续号；`轮次` 沿用现有台账行语义 = `round_index + 1`（窗口内 0 基
/// 声明序号 +1，跨压缩重置，与 `build_ledger_block` 标注一致）——2026-08-18
/// 审查修复：此前两者共用一个 per-row 序号，多工具轮会把轮次标错。
/// 2026-08-18 S4 复验修复：嵌入字段（目标/结果/最终回复）先做换行转义 +
/// 长度上限——否则多行最终回复会把一条逻辑记录拆成多个物理行，`tail_seq`
/// 把续行当损坏尾巴（InvalidData）→ 追加失败 → 连续 3 次后折叠被禁用
/// （实测 `[31] 轮次 27: blackboard_read ...` 记录跨 15 行）。单行契约是
/// 「追加续号」正确性的前提；上限符合设计「摘要行 ~200–350 字符/行」。
pub fn external_row_line(seq: u64, row: &ActionLedgerRow) -> String {
    /// Per-field cap for the single-line row contract（设计 §2.3 摘要行规模）。
    const LEDGER_ROW_FIELD_CAP: usize = 300;
    /// Escape embedded line breaks so a logical row is exactly one physical
    /// line, then bound the field so the file stays a compact grep-able
    /// summary (full content remains in `messages` / journal).
    fn sanitize_field(value: &str) -> String {
        let escaped = value.replace(['\r', '\n'], "\\n");
        if escaped.chars().count() <= LEDGER_ROW_FIELD_CAP {
            return escaped;
        }
        let mut truncated: String = escaped.chars().take(LEDGER_ROW_FIELD_CAP).collect();
        truncated.push('…');
        truncated
    }
    format!(
        "[{seq}] 轮次 {}: {} 目标={} 结果={} 最终回复={}",
        row.round_index + 1,
        row.tool,
        if row.target.is_empty() {
            "（无）".to_string()
        } else {
            sanitize_field(&row.target)
        },
        sanitize_field(&row.pointer),
        if row.final_reply.is_empty() {
            "（无）".to_string()
        } else {
            sanitize_field(&row.final_reply)
        },
    )
}

/// Read the last complete row's leading `[seq]` — the next per-row seq to
/// assign. The file is append-only with monotonically increasing seqs, so
/// the LAST row carries the max; a missing/empty file starts at 1. Reads
/// only the file tail — the window grows until the last row is fully
/// captured (a single row can exceed 4 KiB: long command targets / verbatim
/// final replies; parsing a partial row would silently restart the numbering
/// and break the 跨压缩连续 contract). A non-empty file whose last row has no
/// parseable `[seq]` is corruption — reported as an error so the caller rolls
/// back and degrades instead of renumbering from 1 (2026-08-18 审查修复).
fn tail_seq(path: &std::path::Path) -> std::io::Result<u64> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    const TAIL_BYTES: u64 = 4096;
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(0);
    }
    let mut window = TAIL_BYTES;
    loop {
        let start = len.saturating_sub(window);
        file.seek(SeekFrom::Start(start))?;
        let mut buf = Vec::new();
        (&mut file).take(window).read_to_end(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        // Walk newline-separated segments from the end; a segment is a
        // complete row iff it is preceded by a newline inside the window
        // (k > 0) or the window covers the file head (start == 0).
        let mut last_row: Option<&str> = None;
        let segments: Vec<&str> = text.split('\n').collect();
        for (k, segment) in segments.iter().enumerate().rev() {
            if segment.trim().is_empty() {
                continue;
            }
            if k > 0 || start == 0 {
                last_row = Some(*segment);
            }
            break;
        }
        if start == 0 && last_row.is_none() {
            // Empty or blank-only file — nothing written yet.
            return Ok(0);
        }
        if let Some(row) = last_row {
            let seq = row
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_start_matches('[')
                .split(']')
                .next()
                .and_then(|s| s.trim().parse::<u64>().ok());
            return match seq {
                Some(n) => Ok(n),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("external ledger tail row has no parseable [seq]: {row:?}"),
                )),
            };
        }
        // The last row starts before the window — grow and retry.
        window = window.saturating_mul(2).max(len);
    }
}

/// Append the newly folded rows to the external ledger file (atomic
/// O_APPEND single write + flush). Assigns the per-row global seqs from
/// the file tail (`tail_seq` + 1, +2, …). Failure is reported to the
/// caller — a failed append must NOT advance the fold state (the rows
/// would be lost from both the view and the file; the next trigger retries).
pub fn append_ledger_rows(path: &std::path::Path, rows: &[ActionLedgerRow]) -> std::io::Result<()> {
    append_ledger_rows_range(path, rows).map(|_| ())
}

/// v7 原文定位指针（S1 修订批，2026-09-15，设计 §3.5.1）：与
/// [`append_ledger_rows`] 同一实现，但把本次实际分配的 `[seq]` 闭区间
/// 返回给调用方（压缩 marker 的「台账 `[seq]` 区间」指针来源；空输入
/// 返回 `None`）。行序号跨压缩连续，故区间可直接用于回读定位。
///
/// 审查修正批（2026-09-15，审查 P3⑩）：序号分配是「读尾号 → 追加」两步，
/// **非跨进程原子**——同一 `session_cwd` 下若有并行主车道会话同时折叠，两者
/// 可能交错追加（深审 §3-4 记录过同工作区并发事实）。故本区间按「**本 epoch
/// 观测到的行跨度**」读：指针仍可定位，但并发场景下可能覆盖到别的会话写入的
/// 行（后续若要严格隔离，须给台账文件加会话域或写锁——本批不动）。
pub fn append_ledger_rows_range(
    path: &std::path::Path,
    rows: &[ActionLedgerRow],
) -> std::io::Result<Option<(u64, u64)>> {
    use std::io::Write;
    if rows.is_empty() {
        return Ok(None);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut seq = tail_seq(path)?;
    let first_assigned = seq + 1;
    let mut buf = String::new();
    for row in rows {
        seq += 1;
        buf.push_str(&external_row_line(seq, row));
        buf.push('\n');
    }
    // 0p S2 B5（2026-09-07，ADR-0010 §14.61 设计 B5）：台账落
    // `.gsa/ledger/`，是会话卷持久化面——行文本（命令摘要/订单摘要）经
    // orz-secrets 机械脱敏后写盘（key 不落卷不变量）。
    let buf = orz_secrets::redact_secrets(&buf);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(buf.as_bytes())?;
    file.flush()?;
    Ok(Some((first_assigned, seq)))
}

/// One deterministic ledger row for a single executed tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionLedgerRow {
    /// 0-based round index among the assistant tool declarations.
    pub round_index: usize,
    /// Tool name (mechanical, from the assistant's tool call).
    pub tool: String,
    /// Best-effort target extracted from the call arguments (path/file/
    /// file_path/target_file/url/target/document_id/directory/command);
    /// empty when none is present. FR-A07（2026-09-17）：`file_path` 与
    /// `target_file` 为 harness 实际参数名（search_replace / read_file），
    /// 缺失时行退化为「目标=（无）」、不可按路径检索。
    pub target: String,
    /// `sha256:<hex>` of the tool-result content — the journal/sidecar
    /// pointer for audit look-back; `no_result` when the result is missing.
    pub pointer: String,
    /// The round's non-empty final assistant reply, verbatim (empty when
    /// the round produced no text).
    pub final_reply: String,
}

/// Index ranges of complete tool rounds: each range starts at an assistant
/// tool-call declaration and ends at the next declaration (or the end).
pub(crate) fn round_ranges(messages: &[Message]) -> Vec<(usize, usize)> {
    let starts: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .map(|(i, _)| i)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, &start)| (start, starts.get(k + 1).copied().unwrap_or(messages.len())))
        .collect()
}

/// Every assistant tool call in the round has a matching tool reply — a
/// round missing a result is never collapsed (an orphaned declaration would
/// 400 on replay; D2-1 pairing discipline).
pub(crate) fn is_round_complete(messages: &[Message], range: (usize, usize)) -> bool {
    let (start, end) = range;
    let declared: Vec<&str> = messages[start..end]
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.as_str()))
        .collect();
    let replied: Vec<&str> = messages[start..end]
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.tool_call_id.as_deref())
        .collect();
    declared.iter().all(|id| replied.contains(id))
}

/// Mechanical best-effort target extraction from tool-call arguments.
pub(crate) fn target_of_call(tool_call: &ToolCall) -> String {
    const TARGET_FIELDS: &[&str] = &[
        "path",
        "file",
        "file_path",
        "target_file",
        "url",
        "target",
        "document_id",
        "directory",
        "command",
    ];
    let Some(args) = tool_call.arguments.as_object() else {
        return String::new();
    };
    for field in TARGET_FIELDS {
        if let Some(s) = args.get(*field).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

/// `sha256:<hex>` pointer of the tool-result content (or `no_result`).
fn pointer_of(tool_result: Option<&Message>) -> String {
    match tool_result {
        Some(m) => format!("sha256:{}", sha256_hex(m.content.as_bytes())),
        None => "no_result".to_string(),
    }
}

/// The round's last non-empty assistant text (verbatim), if any.
fn final_reply_of(messages: &[Message], range: (usize, usize)) -> String {
    messages[range.0..range.1]
        .iter()
        .filter(|m| m.role == Role::Assistant && !m.content.is_empty())
        .map(|m| m.content.clone())
        .last()
        .unwrap_or_default()
}

/// One row per tool call in the round ("多结果轮按条登记").
pub(crate) fn rows_for_round(
    messages: &[Message],
    range: (usize, usize),
    round_index: usize,
) -> Vec<ActionLedgerRow> {
    let final_reply = final_reply_of(messages, range);
    let mut rows = Vec::new();
    for m in &messages[range.0..range.1] {
        if m.role != Role::Assistant {
            continue;
        }
        for tc in &m.tool_calls {
            let tool_result = messages[range.0..range.1].iter().find(|r| {
                r.role == Role::Tool && r.tool_call_id.as_deref() == Some(tc.call_id.as_str())
            });
            rows.push(ActionLedgerRow {
                round_index,
                tool: tc.name.clone(),
                target: target_of_call(tc),
                pointer: pointer_of(tool_result),
                final_reply: final_reply.clone(),
            });
        }
    }
    rows
}

/// Render the deterministic ledger block (one line per row).
pub fn build_ledger_block(rows: &[ActionLedgerRow]) -> String {
    let mut lines = vec![format!("{ACTION_LEDGER_PREFIX} {ACTION_LEDGER_VERSION}]")];
    for row in rows {
        lines.push(format!(
            "轮次 {}: {} 目标={} 结果={} 最终回复={}",
            row.round_index + 1,
            row.tool,
            if row.target.is_empty() {
                "（无）"
            } else {
                &row.target
            },
            row.pointer,
            if row.final_reply.is_empty() {
                "（无）"
            } else {
                &row.final_reply
            },
        ));
    }
    lines.push("[/动作台账]".to_string());
    lines.join("\n")
}

/// Build the model-visible request view: preamble + one action-ledger block
/// for the collapsed OLD rounds + the newest `keep_recent_rounds` rounds
/// verbatim. The source `messages` slice is never modified.
///
/// Collapse stops before the first incomplete round — a round missing a
/// tool result is kept verbatim (it becomes part of the exact tail).
pub fn collapsed_cut(messages: &[Message], keep_recent_rounds: usize) -> Option<usize> {
    let ranges = round_ranges(messages);
    if ranges.is_empty() {
        return None;
    }
    let tail = keep_recent_rounds.max(1);
    let mut collapse_count = ranges.len().saturating_sub(tail);
    // FUS-LEDGER-FOLD-STATE review fix (2026-08-18): the completeness check
    // covers EVERY round to be collapsed — not just the boundary round — so
    // a mid-history incomplete round (declaration without a matching tool
    // reply) is never folded into ledger rows (`no_result` rows would lose
    // the in-flight declaration from the model view). Matches the doc
    // contract "a round missing a tool result is kept verbatim".
    while collapse_count > 0
        && !ranges[..collapse_count]
            .iter()
            .all(|&r| is_round_complete(messages, r))
    {
        collapse_count -= 1;
    }
    if collapse_count == 0 {
        return None;
    }
    Some(ranges[collapse_count].0)
}

/// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.2): 折叠
/// 推进的保留起点——桥预算版（取代外挂文件设计 §2.2「最近 1 轮」）。
/// 从最新完整轮往回累加整轮估计（`estimate_messages_tokens`，chars/2
/// 口径），直至再纳入更早一轮会超出桥预算；**最新一轮恒保留**（单独超预算
/// 时由视图内容截断 §3.3 处理），预算剩余时按时间从新到旧容纳更早完整轮。
/// `fold_tail_rounds` 语义退役（桥恒含 ≥1 轮结构，无需轮数下限）。
///
/// 完整性纪律不变：被折叠区（cut 之前）必须全为完整轮——第一个不完整轮
/// 及其后全部保留原文（绝不折叠，同 `collapsed_cut` 的 completeness
/// back-off；不完整轮进桥造成的预算超出由视图截断兜底，声明内容通常很小）。
pub fn bridge_cut(messages: &[Message], budget_estimate: u64) -> Option<usize> {
    let ranges = round_ranges(messages);
    if ranges.is_empty() {
        return None;
    }
    let mut kept_total: u64 = 0;
    let mut kept_from = ranges.len();
    for k in (0..ranges.len()).rev() {
        let estimate =
            crate::controller::estimate_messages_tokens(&messages[ranges[k].0..ranges[k].1]);
        // `kept_from == ranges.len()` 表示尚未计入任何轮——最新一轮恒入桥。
        if kept_from != ranges.len() && kept_total.saturating_add(estimate) > budget_estimate {
            break;
        }
        kept_total += estimate;
        kept_from = k;
    }
    if let Some(first_incomplete) = ranges.iter().position(|&r| !is_round_complete(messages, r))
        && kept_from > first_incomplete
    {
        kept_from = first_incomplete;
    }
    if kept_from == 0 {
        return None;
    }
    Some(ranges[kept_from].0)
}

/// Number of complete round ranges whose start index is strictly before
/// `cut` — the rounds a compaction draining `[first_declaration..cut)`
/// would drop (FUS-LEDGER-FOLD-STATE: the cut may be the frozen
/// `fold_cut` rather than a recomputed stateless tail).
pub fn rounds_before(messages: &[Message], cut: usize) -> usize {
    round_ranges(messages)
        .iter()
        .take_while(|&&(start, _)| start < cut)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    fn round(call_id: &str, tool: &str, target: &str, result: &str) -> Vec<Message> {
        vec![
            Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    name: tool.to_string(),
                    arguments: serde_json::json!({"path": target}),
                    call_id: call_id.to_string(),
                }],
                reasoning_content: None,
                round: None,
            },
            Message {
                role: Role::Tool,
                content: result.to_string(),
                tool_call_id: Some(call_id.to_string()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        ]
    }

    fn declared_ids(messages: &[Message]) -> Vec<String> {
        messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.clone()))
            .collect()
    }

    /// 桥预算测试助手：直接给出 `estimate_messages_tokens` 估计口径的整段
    /// 预算（与生产换算 `fold_tail_estimate_budget` 解耦，便于按轮精确控制
    /// 桥内轮数）。
    fn est_budget(messages: &[Message], from: usize) -> u64 {
        crate::controller::estimate_messages_tokens(&messages[from..])
    }

    // ---- 无状态坍缩语义（0ah 收口清理批）：渲染器 `build_collapsed_request`
    // 已随 v7 视图退役；保留起点语义由 `collapsed_cut` 承接、行装配由
    // `rows_for_round`/`build_ledger_block` 承接——以下直测这三个存留件。----

    #[test]
    fn zero_rounds_is_identity() {
        let messages = vec![msg(Role::User, "你好")];
        assert_eq!(
            collapsed_cut(&messages, DEFAULT_RECENT_TAIL_ROUNDS),
            None,
            "no complete rounds ⇒ nothing to collapse"
        );
    }

    #[test]
    fn old_rounds_collapse_into_ledger_rows() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "内容A"));
        messages.extend(round("c2", "search_replace", "b.rs", "已编辑"));
        messages.extend(round("c3", "web_fetch", "https://x.dev", "页面"));
        // tail=2 ⇒ 只有 c1 落在保留起点之前（c2/c3 留在保留尾）。
        let cut = collapsed_cut(&messages, 2).expect("c1 is collapsible");
        let ranges = round_ranges(&messages);
        let mut rows = Vec::new();
        for (idx, &range) in ranges.iter().enumerate().take_while(|&(_, r)| r.0 < cut) {
            rows.extend(rows_for_round(&messages, range, idx));
        }
        let ledger = build_ledger_block(&rows);
        assert!(ledger.contains("read_file"), "{ledger:?}");
        assert!(ledger.contains("目标=a.py"), "{ledger:?}");
        assert!(ledger.contains("sha256:"), "{ledger:?}");
        assert!(ledger.contains("轮次 1:"), "{ledger:?}");
        assert!(
            !ledger.contains("b.rs"),
            "kept rounds must not collapse: {ledger:?}"
        );
        // c3 的原文留在保留尾（cut 之后）。
        assert!(messages[cut..].iter().any(|m| m.content == "页面"));
    }

    #[test]
    fn multi_result_round_registers_one_row_per_call() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![
                ToolCall {
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path": "a.py"}),
                    call_id: "c1".into(),
                },
                ToolCall {
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path": "b.py"}),
                    call_id: "c2".into(),
                },
            ],
            reasoning_content: None,
            round: None,
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果A".into(),
            tool_call_id: Some("c1".into()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果B".into(),
            tool_call_id: Some("c2".into()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        messages.extend(round("c3", "read_file", "c.py", "结果C"));
        let ranges = round_ranges(&messages);
        let rows = rows_for_round(&messages, ranges[0], 0);
        assert_eq!(rows.len(), 2, "one row per declared call");
        assert_eq!(build_ledger_block(&rows).matches("轮次 1:").count(), 2);
    }

    #[test]
    fn incomplete_round_is_kept_verbatim() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "结果A"));
        // Round 2 declares c2 but the tool result is missing.
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "b.py"}),
                call_id: "c2".into(),
            }],
            reasoning_content: None,
            round: None,
        });
        let cut = collapsed_cut(&messages, 1).expect("c1 is collapsible");
        assert_eq!(cut, 3, "cut = c2 轮起点（不完整轮及其后全保留）");
        let rows = rows_for_round(&messages, (1, 3), 0);
        let ledger = build_ledger_block(&rows);
        assert!(
            ledger.contains("a.py"),
            "complete round collapses: {ledger:?}"
        );
        assert!(
            !ledger.contains("c2"),
            "the incomplete round must not enter the ledger: {ledger:?}"
        );
        assert!(
            messages[cut..]
                .iter()
                .any(|m| m.tool_calls.iter().any(|tc| tc.call_id == "c2")),
            "incomplete declaration stays verbatim"
        );
    }

    #[test]
    fn mid_history_incomplete_round_never_folds() {
        // FUS-LEDGER-FOLD-STATE review fix (2026-08-18): the completeness
        // back-off must cover EVERY collapsed round, not only the boundary
        // round — a mid-history incomplete declaration must stay verbatim
        // (folding it would drop the in-flight call from the model view
        // and register a `no_result` row).
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "结果A"));
        // Round 2 (mid-history) declares c2 without a tool result.
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "b.py"}),
                call_id: "c2".into(),
            }],
            reasoning_content: None,
            round: None,
        });
        messages.extend(round("c3", "web_fetch", "https://x.dev", "页面"));
        messages.extend(round("c4", "read_file", "d.py", "结果D"));
        let cut = collapsed_cut(&messages, 1).expect("c1 is collapsible");
        assert_eq!(
            cut, 3,
            "completeness back-off stops at the incomplete round"
        );
        let ranges = round_ranges(&messages);
        let mut rows = Vec::new();
        for (idx, &range) in ranges.iter().enumerate().take_while(|&(_, r)| r.0 < cut) {
            rows.extend(rows_for_round(&messages, range, idx));
        }
        let ledger = build_ledger_block(&rows);
        // Only the complete round before the incomplete one may fold; the
        // incomplete declaration and everything after stays verbatim.
        assert!(ledger.contains("a.py"), "{ledger:?}");
        assert!(!ledger.contains("b.py"), "{ledger:?}");
        assert!(!ledger.contains("c3"), "{ledger:?}");
        assert!(
            messages[cut..]
                .iter()
                .any(|m| m.tool_calls.iter().any(|tc| tc.call_id == "c2")),
            "the mid-history incomplete declaration stays"
        );
        assert!(messages[cut..].iter().any(|m| m.content == "页面"));
    }

    #[test]
    fn deterministic_output() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "结果A"));
        messages.extend(round("c2", "web_fetch", "https://x.dev", "页面"));
        let build = || {
            let cut = collapsed_cut(&messages, 1).unwrap();
            let ranges = round_ranges(&messages);
            let mut rows = Vec::new();
            for (idx, &range) in ranges.iter().enumerate().take_while(|&(_, r)| r.0 < cut) {
                rows.extend(rows_for_round(&messages, range, idx));
            }
            build_ledger_block(&rows)
        };
        assert_eq!(build(), build());
    }

    /// FR-A07（2026-09-17 复核）：台账行「目标」提取须覆盖 harness 实际参数名
    /// ——`read_file` 用 `target_file`、`search_replace` 用 `file_path`；缺失
    /// 时行退化为「目标=（无）」，不可按路径检索。
    #[test]
    fn target_of_call_covers_harness_argument_names() {
        let call = |args: serde_json::Value| ToolCall {
            name: "x".to_string(),
            arguments: args,
            call_id: "c".to_string(),
        };
        assert_eq!(
            target_of_call(&call(serde_json::json!({"target_file": "docs/a.md"}))),
            "docs/a.md"
        );
        assert_eq!(
            target_of_call(&call(
                serde_json::json!({"file_path": "src/b.rs", "old_string": "o"})
            )),
            "src/b.rs"
        );
        assert_eq!(
            target_of_call(&call(serde_json::json!({"command": "cargo test"}))),
            "cargo test"
        );
        assert_eq!(
            target_of_call(&call(serde_json::json!({"section": "plan"}))),
            "",
            "无目标字段的调用维持空（行渲染为「目标=（无）」）"
        );
    }

    #[test]
    fn external_file_append_continues_sequence_across_windows() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        let rows = |targets: &[&str]| {
            targets
                .iter()
                .enumerate()
                .map(|(i, t)| ActionLedgerRow {
                    round_index: i,
                    tool: "read_file".to_string(),
                    target: (*t).to_string(),
                    pointer: format!("sha256:{i:064x}"),
                    final_reply: format!("已读 {t}"),
                })
                .collect::<Vec<_>>()
        };
        append_ledger_rows(&path, &rows(&["a.py", "b.py", "c.py"])).unwrap();
        append_ledger_rows(&path, &rows(&["d.py", "e.py"])).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(
            lines[0].starts_with("[1] 轮次 1: read_file 目标=a.py"),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("[2] 轮次 2: read_file 目标=b.py"),
            "{}",
            lines[1]
        );
        assert!(
            lines[3].starts_with("[4] 轮次 1: read_file 目标=d.py"),
            "{}",
            lines[3]
        );
        assert!(
            lines[4].starts_with("[5] 轮次 2: read_file 目标=e.py"),
            "{}",
            lines[4]
        );
        assert!(lines[0].contains("结果=sha256:") && lines[0].contains("最终回复=已读 a.py"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 审查修复：`[seq]`（per-row 全局序号）与 `轮次`（窗口内
    /// round_index+1）解耦——多工具轮各行轮次标注一致，跨窗口时全局序号
    /// 继续而轮次从 1 重新计数。
    #[test]
    fn external_row_round_label_uses_round_index_not_global_seq() {
        let row = ActionLedgerRow {
            round_index: 2,
            tool: "read_file".to_string(),
            target: "a.py".to_string(),
            pointer: "sha256:ab".to_string(),
            final_reply: "已读".to_string(),
        };
        let line = external_row_line(7, &row);
        assert!(
            line.starts_with("[7] 轮次 3: read_file 目标=a.py"),
            "{line}"
        );
        // 同一轮的第二条（同 round_index）全局序号 +1、轮次不变。
        let line2 = external_row_line(8, &row);
        assert!(
            line2.starts_with("[8] 轮次 3: read_file 目标=a.py"),
            "{line2}"
        );
    }

    /// 2026-08-18 审查修复：尾行超过 4KiB 时 `tail_seq` 仍取到完整末行，
    /// 续号不因截断而重置（此前 4KiB 尾窗会静默从 1 重新编号）。
    /// 2026-08-18 S4 复验修复后：`external_row_line` 对嵌入字段做换行转义 +
    /// 300 字符上限，正常追加路径不再产生超长行；本测试改为直接写盘一条
    /// >4KiB 单行记录，保留对 `tail_seq` 尾窗逐级倍增路径的防御覆盖。
    #[test]
    fn tail_seq_survives_long_last_row() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-long-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // 单行 >4KiB：尾窗必须逐级倍增取到完整末行，不得截断续号。
        let long_row = format!(
            "[41] 轮次 7: run_terminal 目标=make 结果=sha256:1 最终回复={}",
            "x".repeat(36_000)
        );
        std::fs::write(&path, long_row).unwrap();
        assert_eq!(
            tail_seq(&path).unwrap(),
            41,
            "long single-line row keeps its seq"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 S4 复验修复：嵌入字段（目标/结果/最终回复）中的换行必须
    /// 转义——多行最终回复会把一条逻辑记录拆成多个物理行，`tail_seq` 将续行
    /// 当损坏尾巴（InvalidData）导致追加失败、连续 3 次后折叠被禁用。单行
    /// 契约是「追加续号」正确性的前提；字段上限符合设计「摘要行 ~200–350
    /// 字符/行」。
    #[test]
    fn external_row_line_escapes_embedded_newlines_and_caps() {
        let row = ActionLedgerRow {
            round_index: 3,
            tool: "blackboard_read".to_string(),
            target: "多行\n目标".to_string(),
            pointer: "sha256:ab".to_string(),
            final_reply: "[blackboard_read] == registration ==\nstep1: build\nstep2: verify\n"
                .to_string(),
        };
        let line = external_row_line(31, &row);
        assert_eq!(
            line.lines().count(),
            1,
            "row must be a single physical line: {line}"
        );
        assert!(
            line.starts_with("[31] 轮次 4: blackboard_read 目标=多行\\n目标"),
            "{line}"
        );
        assert!(
            line.contains(
                "最终回复=[blackboard_read] == registration ==\\nstep1: build\\nstep2: verify\\n"
            ),
            "{line}"
        );
        // 字段上限：>300 字符截断并以 … 标记。
        let row2 = ActionLedgerRow {
            round_index: 4,
            tool: "grep".to_string(),
            target: "a.py".to_string(),
            pointer: "sha256:cd".to_string(),
            final_reply: "很长".repeat(500),
        };
        let line2 = external_row_line(32, &row2);
        assert_eq!(line2.lines().count(), 1);
        assert!(
            line2.ends_with('…'),
            "truncated field ends with marker: {line2}"
        );
        assert!(
            line2.chars().count() < 400,
            "row bounded by field cap + prefix: {}",
            line2.chars().count()
        );
    }

    /// 0p S2 B5（2026-09-07，ADR-0010 §14.61 设计 B5）：台账落
    /// `.gsa/ledger/`（会话卷持久化面）——append 漏斗对行文本做
    /// orz-secrets 机械脱敏（sk-shape → 占位符），key 不落卷不变量。
    #[test]
    fn append_ledger_rows_scrubs_secret_shaped_targets() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-scrub-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = ledger_file_path(&dir);
        let secret = "sk-abcdefghijklmnopqrstuvwxyz012345";
        let row = ActionLedgerRow {
            round_index: 0,
            tool: "run_terminal_cmd".to_string(),
            target: format!("pip install --api-key {secret}"),
            pointer: "sha256:ef".to_string(),
            final_reply: String::new(),
        };
        append_ledger_rows(&path, &[row]).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(
            !content.contains(secret),
            "secret-shaped strings must not reach the ledger: {content}"
        );
        assert!(content.contains("[REDACTED_SECRET]"), "{content}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 S4 复验回归：多行最终回复的批次写入后，下一次追加必须
    /// 继续续号（`tail_seq` 不被续行误导），文件保持一行一条逻辑记录。
    #[test]
    fn append_ledger_rows_continues_after_multiline_reply() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-multiline-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        let batch1 = vec![
            ActionLedgerRow {
                round_index: 26,
                tool: "blackboard_read".to_string(),
                target: "（无）".to_string(),
                pointer: "sha256:1".to_string(),
                final_reply: "[blackboard_read] == registration ==\nstep1: build\nstep2: verify"
                    .to_string(),
            },
            ActionLedgerRow {
                round_index: 27,
                tool: "run_terminal".to_string(),
                target: "make".to_string(),
                pointer: "sha256:2".to_string(),
                final_reply: "exit: 0\ncompiled ok".to_string(),
            },
        ];
        append_ledger_rows(&path, &batch1).unwrap();
        let batch2 = vec![ActionLedgerRow {
            round_index: 28,
            tool: "read_file".to_string(),
            target: "vm.js".to_string(),
            pointer: "sha256:3".to_string(),
            final_reply: "ok".to_string(),
        }];
        append_ledger_rows(&path, &batch2).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "one physical line per logical row:\n{text}");
        assert!(
            lines[2].starts_with("[3] 轮次 29: read_file 目标=vm.js"),
            "续号必须从文件尾行继续: {}",
            lines[2]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 审查修复：非空文件尾行无 `[seq]` 视为损坏——返回错误让
    /// 调用方回滚并降级，而不是静默从 1 重新编号造成重复序号。
    #[test]
    fn tail_seq_rejects_unparseable_tail() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-corrupt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "不是台账行的垃圾内容\n").unwrap();
        let row = ActionLedgerRow {
            round_index: 0,
            tool: "read_file".to_string(),
            target: "a.py".to_string(),
            pointer: "sha256:1".to_string(),
            final_reply: "已读".to_string(),
        };
        assert!(
            append_ledger_rows(&path, std::slice::from_ref(&row)).is_err(),
            "corrupt tail must fail loudly, not restart numbering at 1"
        );
        // 空文件（或仅空行）仍从 1 开始。
        let empty = ledger_file_path(&dir.join("empty"));
        append_ledger_rows(&empty, std::slice::from_ref(&row)).unwrap();
        let text = std::fs::read_to_string(&empty).unwrap();
        assert!(
            text.starts_with("[1] 轮次 1: read_file 目标=a.py"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 审查修复：`safe_fold_cut` 防御在推进侧生效——最新保留轮
    /// 不完整（声明无回复）时 cut 回退到最近平衡轮起点，未折叠轮次留在
    /// 原文视图、不入外挂文件，行与视图一致。
    #[test]
    fn bridge_cut_walks_newest_first_until_budget() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "run_terminal", "make", &"out".repeat(200)));
        messages.extend(round("c3", "read_file", "c.py", "C"));
        // 预算恰容纳 c2+c3：从最新往回累加，c1 必须折进台账。
        let cut = bridge_cut(&messages, est_budget(&messages, 3)).unwrap();
        assert_eq!(cut, 3, "cut = c2 轮起点（c2+c3 入桥）");
    }

    #[test]
    fn bridge_cut_keeps_newest_round_when_alone_exceeds_budget() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "run_terminal", "make", &"x".repeat(20_000)));
        // 极小预算：最新一轮单独超预算仍恒入桥，更早轮照常折叠。
        let cut = bridge_cut(&messages, 10).unwrap();
        assert_eq!(cut, 3, "c2 单独入桥、c1 折叠");
    }

    #[test]
    fn bridge_cut_defers_to_first_incomplete_round() {
        // c1 巨大（单独占满预算）→ 预算行走会折掉 c1 保留 c2/c3；
        // c2 不完整 → cut 必须停在 c2 轮起点，不得把不完整轮折进台账。
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "run_terminal", "make", &"x".repeat(20_000)));
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "b.py"}),
                call_id: "c2".into(),
            }],
            reasoning_content: None,
            round: None,
        });
        messages.extend(round("c3", "read_file", "c.py", "C"));
        let cut = bridge_cut(&messages, 10).unwrap();
        assert_eq!(cut, 3, "cut = c2 轮起点（不完整轮永不折叠）");
    }

    #[test]
    fn oversized_tool_results_are_pointerized_idempotently() {
        let mut messages = vec![msg(Role::User, "任务")];
        // 小结果（200 字符 ≈100 估计）与超大结果（60K 字符 ≈30K 估计）。
        messages.extend(round("c1", "read_file", "small.py", &"s".repeat(200)));
        messages.extend(round("c2", "read_file", "big.py", &"B".repeat(60_000)));
        let target = 1_000;
        let dir = std::env::temp_dir().join(format!(
            "orz-pointerize-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let archive_dir = dir.join(".gsa").join("compaction").join("pointerized");
        let stats = pointerize_oversized_tool_results(
            &mut messages,
            target,
            OVERSIZED_TOOL_RESULT_CAP_TOKENS,
            archive_dir.as_path(),
            "probe",
            Some(std::path::Path::new(".gsa/runs/RUN-X/events.jsonl")),
            "RUN-X",
        );
        assert_eq!(stats.replaced, 1, "只替换超门槛的那一条");
        assert!(
            stats.freed_tokens > 25_000,
            "释放读数: {}",
            stats.freed_tokens
        );
        assert!(
            crate::controller::estimate_messages_tokens(&messages) <= target,
            "达线即停"
        );
        let big = messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("c2"))
            .expect("超大工具结果仍在（配对未破坏）");
        assert!(big.content.starts_with(TOOL_RESULT_POINTERIZED_PREFIX));
        assert!(big.content.contains("call_id=c2"), "{}", big.content);
        assert!(big.content.contains("events.jsonl"), "{}", big.content);
        assert!(
            big.content.contains("新鲜读取"),
            "不假装新鲜: {}",
            big.content
        );
        assert!(
            big.content.contains("原文头部"),
            "保留头部: {}",
            big.content
        );
        let small = messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("c1"))
            .expect("小结果");
        assert!(
            !small.content.starts_with(TOOL_RESULT_POINTERIZED_PREFIX),
            "未超门槛不得指针化"
        );
        assert_eq!(declared_ids(&messages).len(), 2, "声明配对未破坏");
        assert!(
            messages
                .iter()
                .any(|m| m.role == Role::Assistant && m.tool_calls.len() == 1),
            "assistant 声明原样"
        );
        // 幂等：重复调用不再替换（不重复消耗/不重复改写）。
        let again = pointerize_oversized_tool_results(
            &mut messages,
            target,
            OVERSIZED_TOOL_RESULT_CAP_TOKENS,
            archive_dir.as_path(),
            "probe",
            None,
            "RUN-X",
        );
        assert_eq!(again.replaced, 0);
        assert_eq!(again.freed_tokens, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pointer_message_is_parameter_free_and_points_at_the_ledger() {
        let pointer = build_pointer_message(std::path::Path::new(".gsa/ledger/current.md"));
        assert!(pointer.starts_with(LEDGER_FOLD_POINTER_PREFIX), "{pointer}");
        assert!(pointer.contains(".gsa/ledger/current.md"), "{pointer}");
        // v8（2026-09-16 勘误批，设计 §1 §4 §6）：v7 的「窗口只保留最近若干
        // 完整轮」（机械驱逐语义）随勘误退役——文案改为「旧内容按**分块**累积
        // ＋被压缩/截断的分块可按块回放」；参数（x/y/阶梯）一律不进文案。
        assert!(
            pointer.contains("当前**上下文窗口**里的旧内容按**分块**累积"),
            "{pointer}"
        );
        assert!(
            pointer.contains("可按块回放"),
            "v8 指针文案须声明按块回放（I5）: {pointer}"
        );
        assert!(!pointer.contains("8K"), "参数不得进指针文案: {pointer}");
        assert!(
            pointer.contains("外挂存档于 .gsa/ledger/current.md"),
            "指针文案必须给出外挂台账落点: {pointer}"
        );
        assert!(!pointer.contains("归档在该文件中"), "{pointer}");
        assert!(!pointer.contains("最近 1 轮原文"), "{pointer}");
    }

    /// 0AE-C11 处置钉（2026-09-17，盘点 FR-C04 采②）：「本 run」段只含
    /// 当前 run 章的编辑；此前 run 与无章旧行落「会话历史」段——多 run
    /// 会话不再被「本 run」标签夸大归属。
    #[test]
    fn run_context_block_splits_current_run_from_history() {
        let edits = vec![
            edit("shared.rs", "RUN-2", "t3"),
            edit("old_a.rs", "RUN-1", "t1"),
            edit("legacy.rs", "", "t0"),
            edit("shared.rs", "RUN-1", "t2"),
        ];
        let block = render_run_context_block(None, &edits, Some("RUN-2"));
        let this_line = block
            .lines()
            .find(|l| l.starts_with("【本 run 自编辑文件】"))
            .expect("本 run 段须存在");
        assert!(this_line.contains("shared.rs"), "{this_line}");
        assert!(
            !this_line.contains("old_a.rs") && !this_line.contains("legacy.rs"),
            "本 run 段不得混入历史 run 编辑: {this_line}"
        );
        let hist_line = block
            .lines()
            .find(|l| l.starts_with("【会话历史自编辑文件"))
            .expect("历史段须存在");
        assert!(
            hist_line.contains("（此前 1 个 run ＋ 无章旧行）"),
            "历史段标签须如实并列此前 run 数与无章旧行: {hist_line}"
        );
        assert!(hist_line.contains("old_a.rs"), "{hist_line}");
        assert!(hist_line.contains("legacy.rs"), "{hist_line}");
        assert!(hist_line.contains("shared.rs"), "{hist_line}");
    }

    /// 单 run 会话（全部编辑属当前 run）：不渲染历史段——不虚设「历史」。
    #[test]
    fn run_context_block_omits_history_when_all_edits_are_current() {
        let edits = vec![edit("a.rs", "RUN-1", "t1"), edit("b.rs", "RUN-1", "t2")];
        let block = render_run_context_block(None, &edits, Some("RUN-1"));
        assert!(block.contains("【本 run 自编辑文件】"), "{block}");
        assert!(!block.contains("会话历史"), "{block}");
    }

    /// 无当前 run 可对（current_run = None）：全部归历史段、不渲染「本
    /// run」段——标签不夸大。
    #[test]
    fn run_context_block_without_current_run_marks_all_history() {
        let edits = vec![edit("a.rs", "RUN-1", "t1")];
        let block = render_run_context_block(None, &edits, None);
        assert!(!block.contains("【本 run 自编辑文件】"), "{block}");
        assert!(
            block.contains("【会话历史自编辑文件（此前 1 个 run）】"),
            "{block}"
        );
    }

    fn edit(file: &str, run: &str, timestamp: &str) -> crate::blackboard::EditRecord {
        crate::blackboard::EditRecord {
            file: file.to_string(),
            old_lines: 1,
            new_lines: 2,
            timestamp: timestamp.to_string(),
            round: 1,
            domain: None,
            run: run.to_string(),
        }
    }
}
