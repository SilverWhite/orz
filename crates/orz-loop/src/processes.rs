//! TER T1.6 (2026-09-04): 黑板 `section=processes` live 分区渲染——
//! 读取时现算快照的文本化（纯函数；行结构 task_id / 命令摘要 ≤80B /
//! elapsed / status / 字节 / CPU / pid / killable）。整分区 ≤ 8KiB 字符
//! （同 session 面先例），超限机械截断并标注。

use crate::host::LiveProcessFact;

/// 整分区渲染字符预算（同 session 面先例，T0.2 §5.1）。
pub const PROCESSES_RENDER_BUDGET_CHARS: usize = 8 * 1024;

/// 命令摘要长度上限（T0.2 §5.1：≤80B；沿用 F4 cmd_preview 摘要纪律，
/// 不落全命令）。
const MAX_COMMAND_PREVIEW_CHARS: usize = 80;

fn preview(command: &str) -> String {
    let trimmed = command.trim();
    if trimmed.chars().count() <= MAX_COMMAND_PREVIEW_CHARS {
        return trimmed.to_string();
    }
    let mut s: String = trimmed.chars().take(MAX_COMMAND_PREVIEW_CHARS).collect();
    s.push('…');
    s
}

fn render_row(fact: &LiveProcessFact) -> String {
    let command = fact.display_command.as_deref().unwrap_or(&fact.command);
    let pid = fact
        .pid
        .map(|p| p.to_string())
        .unwrap_or_else(|| "-".to_string());
    let killable = if fact.killable { "yes" } else { "no" };
    let elapsed_secs = fact.elapsed_ms as f64 / 1000.0;
    let cpu_ms = fact.cpu_micros as f64 / 1000.0;
    format!(
        "[{}] {} | status: {} | elapsed: {:.1}s | bytes: {} | cpu: {:.1}ms | pid: {} | killable: {}",
        fact.task_id,
        preview(command),
        fact.status,
        elapsed_secs,
        fact.total_bytes,
        cpu_ms,
        pid,
        killable,
    )
}

/// 渲染 processes live 分区（≤ `PROCESSES_RENDER_BUDGET_CHARS` 字符）。
pub fn render_processes_text(facts: &[LiveProcessFact]) -> String {
    let mut out = String::from("== processes (live) ==\n");
    if facts.is_empty() {
        out.push_str("（无）\n");
        return out;
    }
    let mut rendered = 0usize;
    for fact in facts {
        let row = render_row(fact);
        let row_with_newline = row.chars().count() + 1;
        if out.chars().count() + row_with_newline > PROCESSES_RENDER_BUDGET_CHARS {
            out.push_str(&format!(
                "\n[truncated: processes 分区超出 {} 字符预算，仅显示前 {} 行]\n",
                PROCESSES_RENDER_BUDGET_CHARS, rendered
            ));
            break;
        }
        out.push_str(&row);
        out.push('\n');
        rendered += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(task_id: &str, command: &str, status: &str, killable: bool) -> LiveProcessFact {
        LiveProcessFact {
            task_id: task_id.to_string(),
            command: command.to_string(),
            display_command: None,
            pid: Some(4242),
            elapsed_ms: 1234,
            status: status.to_string(),
            total_bytes: 8192,
            cpu_micros: 25_000,
            killable,
            owner_session_id: None,
            description: None,
        }
    }

    #[test]
    fn empty_section_renders_no_active_rows() {
        let text = render_processes_text(&[]);
        assert!(text.starts_with("== processes (live) =="), "{text}");
        assert!(text.contains("（无）"), "{text}");
    }

    #[test]
    fn rows_carry_task_status_pid_and_killable() {
        let text = render_processes_text(&[
            fact("t-1", "python train.py --epochs 100", "idle", true),
            fact("t-2", "sleep 60", "running", true),
            fact("t-3", "make build", "killed", false),
        ]);
        assert!(
            text.contains("[t-1] python train.py --epochs 100"),
            "{text}"
        );
        assert!(text.contains("status: idle"), "{text}");
        assert!(text.contains("pid: 4242"), "{text}");
        assert!(text.contains("killable: yes"), "{text}");
        assert!(text.contains("killable: no"), "{text}");
        assert!(text.contains("elapsed: 1.2s"), "{text}");
        assert!(text.contains("bytes: 8192"), "{text}");
    }

    #[test]
    fn command_preview_elides_to_80_chars() {
        let long = format!("python -c '{}'", "x".repeat(300));
        let fact = fact("t-long", &long, "running", true);
        let text = render_processes_text(&[fact]);
        let row = text
            .lines()
            .find(|l| l.starts_with("[t-long]"))
            .expect("row");
        let body = row
            .strip_prefix("[t-long] ")
            .expect("row prefix")
            .split(" | status:")
            .next()
            .expect("preview part");
        assert_eq!(body.chars().count(), 80 + 1, "80 摘要 + 省略号: {body}");
        assert!(body.ends_with('…'), "{body}");
    }

    #[test]
    fn oversized_section_is_truncated_with_marker() {
        let facts: Vec<LiveProcessFact> = (0..500)
            .map(|i| {
                fact(
                    &format!("t-{i}"),
                    &format!("cmd-{i}-{}", "payload".repeat(40)),
                    "running",
                    true,
                )
            })
            .collect();
        let text = render_processes_text(&facts);
        assert!(
            text.chars().count() <= PROCESSES_RENDER_BUDGET_CHARS + 200,
            "must respect budget (marker 可少量超出): {}",
            text.chars().count()
        );
        assert!(text.contains("[truncated:"), "{text}");
    }
}
