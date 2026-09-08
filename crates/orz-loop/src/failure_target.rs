//! Failure-target identity (P2-10 MECHANICAL-LAYER-MATH-CALCULUS F4 §5.2,
//! TODO I2).
//!
//! The journal `tool_completed` failure events of the command / anchor /
//! file / URL tool families carry an optional `failure_target: {kind, id,
//! ...}` where `id` is a deterministic SHA-256 digest of the canonical
//! target — never the full command text (only a ≤ 80 B preview travels).
//!
//! Families (§5.2):
//! - `cmd_target`   ← run_terminal_cmd / run_tests (canonical command text);
//! - `anchor_target`← search_replace with expected_anchor (GetPut triple
//!   {path, anchor_hash, size});
//! - `file_target`  ← search_replace without anchor / read_file / grep (path);
//! - `url_target`   ← web_fetch / browser_read (ACAF canonical URL).
//!
//! 0q 统一失败事件管线（2026-09-08，ADR-0010 §14.63）新增第五族：
//! - `action_target`← console 订单失败收据（`id = sha256(order_id)`，
//!   preview = action 名）。订单身份在收据装配点从既有字段确定性导出，
//!   仅入黑板 `failure_agg` 聚合与 receipt 错误信封，不进 journal 事件面
//!   （收据错误信封不是 journal 事件——法官族面不校验本族）。

use orz_assurance::journal::sha256_hex;
use serde_json::{Value, json};

/// Canonical command text for digesting: trim + normalize CRLF. The digest
/// covers exactly these bytes.
pub fn canonical_command(cmd: &str) -> String {
    cmd.trim().replace("\r\n", "\n").to_string()
}

/// Canonical URL: the ACAF network-target normalisation (default ports
/// dropped, fragment removed, scheme/host lowercased) when parseable;
/// otherwise the trimmed input (still deterministic).
pub fn canonical_url(url: &str) -> String {
    let trimmed = url.trim();
    match orz_assurance::acaf::target::resolve_network_url(trimmed) {
        Ok(canonical) => canonical,
        Err(_) => trimmed.to_string(),
    }
}

/// First `max_bytes` bytes of `text`, never splitting a UTF-8 code point.
pub fn bounded_preview(text: &str, max_bytes: usize) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if out.len() + ch.len_utf8() > max_bytes {
            break;
        }
        out.push(ch);
    }
    out
}

/// Human-facing display preview for an F4 identity object (≤ 80 B, never
/// log-level detail) — the P2-12 failure-target aggregation's per-row label.
/// cmd targets use the command preview, anchor/file targets the path, URL
/// targets the canonical URL, action targets the action name; unknown kinds
/// yield `None`.
pub fn target_preview(ft: &Value) -> Option<String> {
    let kind = ft.get("kind").and_then(Value::as_str)?;
    let raw = match kind {
        "cmd_target" => ft.get("cmd_preview").and_then(Value::as_str)?,
        "anchor_target" | "file_target" => ft.get("path").and_then(Value::as_str)?,
        "url_target" => ft.get("canonical_url").and_then(Value::as_str)?,
        "action_target" => ft.get("action").and_then(Value::as_str)?,
        _ => return None,
    };
    Some(bounded_preview(raw, 80))
}

/// 0q（2026-09-08，ADR-0010 §14.63）：失败聚合覆盖面工具集——四族身份
/// 可及的工具并集。漏斗只对该集合的 error 形状完成做盖章/标记判定；其余
/// 工具（计划轮黑板面、console 内建、检索派发包装等）身份定义性 None，
/// 聚合面维持不进现状，法官族面也按同表跳过。
pub fn identity_capable(tool: &str) -> bool {
    matches!(
        tool,
        "run_terminal_cmd"
            | "run_tests"
            | "search_replace"
            | "read_file"
            | "grep"
            | "web_fetch"
            | "browser_read"
    )
}

/// 0q 第五族：console 订单失败的 F4 身份（SARIF ruleId×指纹同构）——
/// `id = sha256(order_id)`，`action` 字段承载动作名（渲染 preview 的源）。
pub fn action_failure_target(order_id: &str, action: &str) -> Value {
    json!({
        "kind": "action_target",
        "id": sha256_hex(order_id.as_bytes()),
        "action": action,
    })
}

/// Build the `failure_target` JSON object for a failing tool call, or `None`
/// when the call carries no identifiable target (workspace-wide grep, missing
/// arguments, unknown tool).
pub fn failure_target(tool: &str, arguments: &Value) -> Option<Value> {
    match tool {
        "run_terminal_cmd" | "run_tests" => {
            let cmd = arguments
                .get("command")
                .or_else(|| arguments.get("cmd"))
                .and_then(Value::as_str)?;
            let canonical = canonical_command(cmd);
            if canonical.is_empty() {
                return None;
            }
            let id = sha256_hex(canonical.as_bytes());
            Some(json!({
                "kind": "cmd_target",
                "id": id,
                "cmd_preview": bounded_preview(&canonical, 80),
            }))
        }
        "search_replace" => {
            let path = arguments.get("file_path").and_then(Value::as_str)?;
            if path.is_empty() {
                return None;
            }
            // GetPut anchor identity when the call carried expected_anchor.
            if let Some(anchor) = arguments.get("expected_anchor") {
                let anchor_hash = anchor
                    .get("sha256")
                    .or_else(|| anchor.get("hash"))
                    .and_then(Value::as_str);
                let size = anchor.get("size").and_then(Value::as_u64);
                if let (Some(anchor_hash), Some(size)) = (anchor_hash, size) {
                    let canonical = format!(
                        "{{\"path\":{},\"anchor_hash\":{},\"size\":{}}}",
                        json!(path),
                        json!(anchor_hash),
                        size
                    );
                    let id = sha256_hex(canonical.as_bytes());
                    return Some(json!({
                        "kind": "anchor_target",
                        "id": id,
                        "path": path,
                        "anchor_hash": anchor_hash,
                        "size": size,
                    }));
                }
            }
            let id = sha256_hex(path.as_bytes());
            Some(json!({
                "kind": "file_target",
                "id": id,
                "path": path,
            }))
        }
        "read_file" | "grep" => {
            let path = arguments
                .get("file_path")
                .or_else(|| arguments.get("path"))
                .and_then(Value::as_str)?;
            if path.is_empty() {
                return None;
            }
            let id = sha256_hex(path.as_bytes());
            Some(json!({
                "kind": "file_target",
                "id": id,
                "path": path,
            }))
        }
        "web_fetch" | "browser_read" => {
            let url = arguments.get("url").and_then(Value::as_str)?;
            let canonical = canonical_url(url);
            if canonical.is_empty() {
                return None;
            }
            let id = sha256_hex(canonical.as_bytes());
            Some(json!({
                "kind": "url_target",
                "id": id,
                "canonical_url": canonical,
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_command_trims_and_normalizes_crlf() {
        assert_eq!(canonical_command("  make -j8  "), "make -j8");
        assert_eq!(canonical_command("a\r\nb"), "a\nb");
        assert_eq!(canonical_command("a\nb"), "a\nb");
    }

    #[test]
    fn canonical_url_uses_acaf_normalization() {
        assert_eq!(
            canonical_url("HTTP://Example.COM:80/a/b?q=1#frag"),
            "http://example.com/a/b?q=1"
        );
        // Unparseable input degrades to the trimmed string deterministically.
        assert_eq!(canonical_url("  not a url  "), "not a url");
    }

    #[test]
    fn preview_is_byte_bounded_and_utf8_safe() {
        assert_eq!(bounded_preview("abcdef", 3), "abc");
        assert_eq!(bounded_preview("你好世界", 4), "你");
        assert_eq!(bounded_preview("hello", 100), "hello");
    }

    #[test]
    fn cmd_target_shape() {
        let ft = failure_target(
            "run_terminal_cmd",
            &json!({ "command": "  python train.py --epochs 50  " }),
        )
        .expect("cmd target");
        assert_eq!(ft["kind"], "cmd_target");
        assert_eq!(ft["cmd_preview"], "python train.py --epochs 50");
        let id = ft["id"].as_str().unwrap();
        assert_eq!(id.len(), 64);
        let expected = sha256_hex("python train.py --epochs 50".as_bytes());
        assert_eq!(id, expected);
    }

    #[test]
    fn anchor_target_uses_getput_triple() {
        let ft = failure_target(
            "search_replace",
            &json!({
                "file_path": "src/lib.rs",
                "expected_anchor": { "size": 4096, "sha256": "ab".repeat(32) },
            }),
        )
        .expect("anchor target");
        assert_eq!(ft["kind"], "anchor_target");
        assert_eq!(ft["path"], "src/lib.rs");
        assert_eq!(ft["size"], 4096);
        assert_eq!(ft["anchor_hash"], "ab".repeat(32));
    }

    #[test]
    fn search_replace_without_anchor_falls_back_to_file_target() {
        let ft = failure_target("search_replace", &json!({ "file_path": "src/lib.rs" }))
            .expect("file target");
        assert_eq!(ft["kind"], "file_target");
        assert_eq!(ft["path"], "src/lib.rs");
    }

    #[test]
    fn file_target_for_read_and_grep() {
        let ft = failure_target("read_file", &json!({ "file_path": "a.py" })).unwrap();
        assert_eq!(ft["kind"], "file_target");
        assert_eq!(ft["path"], "a.py");
        let ft = failure_target("grep", &json!({ "path": "src" })).unwrap();
        assert_eq!(ft["kind"], "file_target");
        assert_eq!(ft["path"], "src");
        // Workspace-wide grep without a path has no target identity.
        assert!(failure_target("grep", &json!({ "pattern": "x" })).is_none());
    }

    #[test]
    fn url_target_uses_canonical_url() {
        let ft = failure_target("web_fetch", &json!({ "url": "HTTP://EXAMPLE.com/a#f" })).unwrap();
        assert_eq!(ft["kind"], "url_target");
        assert_eq!(ft["canonical_url"], "http://example.com/a");
        let id = ft["id"].as_str().unwrap();
        assert_eq!(id.len(), 64);
    }

    #[test]
    fn unknown_tool_and_missing_args_yield_none() {
        assert!(failure_target("plan_write", &json!({})).is_none());
        assert!(failure_target("run_terminal_cmd", &json!({})).is_none());
        assert!(failure_target("web_fetch", &json!({})).is_none());
    }

    #[test]
    fn identity_capable_is_exactly_the_four_family_union() {
        for tool in [
            "run_terminal_cmd",
            "run_tests",
            "search_replace",
            "read_file",
            "grep",
            "web_fetch",
            "browser_read",
        ] {
            assert!(identity_capable(tool), "{tool} must be capable");
        }
        for tool in [
            "plan_write",
            "blackboard_read",
            "list_dir",
            "assistant.trace",
            "retrieve_project_docs",
            "web_search",
        ] {
            assert!(!identity_capable(tool), "{tool} must not be capable");
        }
    }

    #[test]
    fn action_target_carries_sha256_order_identity_and_action_preview() {
        let ft = action_failure_target("ORD-000042", "workspace.run_terminal");
        assert_eq!(ft["kind"], "action_target");
        let id = ft["id"].as_str().unwrap();
        assert_eq!(id, sha256_hex(b"ORD-000042"));
        assert_eq!(ft["action"], "workspace.run_terminal");
        assert_eq!(
            target_preview(&ft).as_deref(),
            Some("workspace.run_terminal")
        );
        // Deterministic: same order id → same identity (SARIF fingerprint
        // isomorphism — identity derives from order_id alone, not the
        // action name); distinct orders never collide on identity.
        assert_eq!(action_failure_target("ORD-000042", "x")["id"], ft["id"]);
        assert_ne!(action_failure_target("ORD-000043", "x")["id"], ft["id"]);
    }
}
