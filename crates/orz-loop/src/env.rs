//! TER T1.12 (W-F11, 2026-09-04)：黑板 `section=env` live 分区渲染——
//! 机械层代码工具环境快照（工具/语言/包/版本、关键输入在场、连通性）。
//! 纯函数：白名单 kind 登记（越权 kind = 渲染层拒绝）、kind+key 排序、
//! 整分区 ≤8KiB 字符（同 processes/session 预算先例）、空态「（无）」。

use crate::host::EnvSnapshotFact;

/// 整分区字符预算（同黑板其它分区先例）。
pub const ENV_RENDER_BUDGET_CHARS: usize = 8 * 1024;

/// env 白名单 kind 集合（T0.2 §5.2/§5.3-4：实现侧登记，越权键=渲染层
/// 拒绝）。
pub const ENV_ALLOWED_KINDS: &[&str] = &["tool", "language", "package", "input", "connectivity"];

/// 渲染 env live 分区：只输出白名单 kind；按 kind→key 排序；超预算截断
/// 标注。绝不输出任务专属结论 / allowlist 内容（facts 生产侧亦不产生）。
pub fn render_env_text(facts: &[EnvSnapshotFact]) -> String {
    let mut out = String::from("== env (live) ==\n");
    let mut rows: Vec<&EnvSnapshotFact> = facts
        .iter()
        .filter(|f| ENV_ALLOWED_KINDS.contains(&f.kind.as_str()))
        .collect();
    rows.sort_by(|a, b| a.kind.cmp(&b.kind).then_with(|| a.key.cmp(&b.key)));
    if rows.is_empty() {
        out.push_str("（无）\n");
        return out;
    }
    let mut rendered = 0usize;
    for fact in rows {
        let line = format!("{}: {} = {}\n", fact.kind, fact.key, fact.value);
        if out.chars().count() + line.chars().count() > ENV_RENDER_BUDGET_CHARS {
            out.push_str(&format!(
                "\n[truncated: env 分区超出 {} 字符预算，仅显示前 {} 行]\n",
                ENV_RENDER_BUDGET_CHARS, rendered
            ));
            break;
        }
        out.push_str(&line);
        rendered += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fact(kind: &str, key: &str, value: &str) -> EnvSnapshotFact {
        EnvSnapshotFact {
            kind: kind.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn unknown_kinds_are_rejected_at_render() {
        let text = render_env_text(&[
            fact("tool", "git", "present (2.45)"),
            fact("secret_allowlist", "hf_endpoint", "https://…"),
            fact("task_conclusion", "mips", "missing"),
        ]);
        assert!(text.contains("tool: git"), "{text}");
        assert!(!text.contains("secret_allowlist"), "{text}");
        assert!(!text.contains("hf_endpoint"), "{text}");
        assert!(!text.contains("task_conclusion"), "{text}");
        assert!(!text.contains("mips"), "{text}");
    }

    #[test]
    fn rows_sorted_and_empty_state() {
        let text = render_env_text(&[
            fact("package", "pip", "present"),
            fact("tool", "cargo", "present"),
            fact("language", "python", "present"),
        ]);
        let language = text.find("language: python").expect("language row");
        let package = text.find("package: pip").expect("package row");
        let tool = text.find("tool: cargo").expect("tool row");
        assert!(language < package && package < tool, "{text}");
        assert!(render_env_text(&[]).contains("（无）"));
    }

    #[test]
    fn oversized_env_is_truncated_with_marker() {
        let facts: Vec<EnvSnapshotFact> = (0..500)
            .map(|i| {
                fact(
                    "tool",
                    &format!("t{i}"),
                    &format!("present ({})", "v".repeat(80)),
                )
            })
            .collect();
        let text = render_env_text(&facts);
        assert!(
            text.chars().count() <= ENV_RENDER_BUDGET_CHARS + 200,
            "{}",
            text.chars().count()
        );
        assert!(text.contains("[truncated:"), "{text}");
    }
}
