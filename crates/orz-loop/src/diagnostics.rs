//! 半助理层失败自动诊断（THIN-HARNESS-REDESIGN V2 §4.2，R2）。
//!
//! 触发：命令执行失败（非零退出 / 超时 / 目标缺失 / 工具不可用）。
//! 动作：结构化签名词典匹配 → 调用物状态检查（确定性 fs 探针）→
//! 有界原始尾部兜底 → ≤2KB 极简记录 + 全量日志指针（trace_id）。
//!
//! P5 纪律：签名匹配只消费结构化信号（`exit_code` / `timed_out` /
//! `output_encoding` / `ToolError` 类别 / `PolicyDenial{source,code}`），
//! **禁止对输出文本做子串判定**；无签名命中时降级为 raw 尾部，不产生
//! 错误分类。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 诊断信封序列化总上限（§4.2 定案：≤2KB 摘要 + 指针）。
pub const DIAGNOSTIC_MAX_SERIALIZED_BYTES: usize = 2048;
/// 有界原始尾部行数上限。
pub const DIAGNOSTIC_TAIL_MAX_LINES: usize = 12;
/// 有界原始尾部字符上限（单尾）。
pub const DIAGNOSTIC_TAIL_MAX_CHARS: usize = 2048;
/// 结构化 key_fields 上限。
pub const DIAGNOSTIC_KEY_FIELDS_MAX: usize = 8;
/// 调用物状态（target_state）字段上限。
pub const DIAGNOSTIC_TARGET_STATE_MAX: usize = 4;

/// 诊断域：决定签名词典范围与调用物状态检查方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticDomain {
    Terminal,
    File,
    Process,
    Environment,
}

/// 结构化工具错误类别（与 `crate::host::ToolErrorKind` 统一，不承载文本；
/// 2026-08-28 全面审查处理：单一枚举来源，消除平行枚举手写映射漂移）。
pub use crate::host::ToolErrorKind as ErrorKind;

/// stat 探针结构化信号（2026-08-28 全面审查处理补明 §4.2：fs 探针结果
/// 属结构化信号，签名可声明 stat 谓词）。
#[derive(Debug, Clone, Copy, Default)]
pub struct TargetStat<'a> {
    /// 目标是否存在（三态：None=无探针或目标路径缺失）。
    pub exists: Option<bool>,
    /// 实际类型（file/directory/other；仅 exists=true 时有值）。
    pub kind: Option<&'a str>,
    /// 期望类型（由参数键推断；None=不限）。
    pub expected_kind: Option<&'a str>,
}

/// 单条结构化 key field（键值恒为字符串，稳定排序）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyField {
    pub key: String,
    pub value: String,
}

/// 诊断记录（§4.2 定案格式）：
/// `{exit_code, matched_signature?, key_fields[], target_state[], tail[], log_pointer, truncated}`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosticRecord {
    pub exit_code: Option<i32>,
    /// 命中的结构化签名 id（None = 无签名命中，tail 为 raw 兜底）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_signature: Option<String>,
    /// 签名声明的 key_fields（≤8 项）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_fields: Vec<KeyField>,
    /// 调用物状态（path/exists/kind/size/mtime；≤4 项）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub target_state: Vec<KeyField>,
    /// 有界原始尾部（raw，不参与错误分类；≤12 行）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tail: Vec<String>,
    /// tail 是否 raw 兜底（无签名命中时 true；有签名命中时 tail 仅为
    /// 补充上下文）。设计 §4.2：无签名命中附 stderr 尾部并标记 raw。
    #[serde(default)]
    pub tail_is_raw: bool,
    /// 全量日志指针（trace_id → `assistant.trace` 续读）。
    pub log_pointer: String,
    /// 序列化总上限 2KB 截断标记。
    #[serde(default)]
    pub truncated: bool,
}

/// 结构化签名（静态词典；顺序即优先级——更具体签名在前）。
#[derive(Debug, Clone, Copy)]
pub struct DiagnosticSignature {
    pub id: &'static str,
    pub domain: DiagnosticDomain,
    /// exit_code 精确集合（`None` = 不约束）。
    pub exit_codes: Option<&'static [i32]>,
    /// 非成功退出（exit_code != Some(0)，含 None）。
    pub non_success: bool,
    /// `timed_out == true` 匹配。
    pub require_timed_out: bool,
    /// `output_encoding` 含 `lossy` 标记匹配。
    pub require_encoding_lossy: bool,
    /// `ToolError` 类别匹配。
    pub error_kind: Option<ErrorKind>,
    /// `PolicyDenial.source` 集合匹配（结构化）。
    pub policy_sources: Option<&'static [&'static str]>,
    /// stat 探针断言（P5 补明：fs 探针结果属结构化信号；None=不约束）。
    /// `Some(true)` 要求 exists==true；`Some(false)` 要求 exists==false。
    pub requires_exists: Option<bool>,
    /// 类型不符断言（`target_type_mismatch` 用）：exists==true 且实际
    /// kind ≠ 期望 kind（期望由参数键推断，见 `expected_kind_from_arguments`）。
    pub requires_kind_mismatch: bool,
    /// 匹配后抽取的 key_fields 键。
    pub key_fields: &'static [&'static str],
}

/// 首批结构化签名词典（§4.2 明细表；R2 S1 落地）。
pub static DIAGNOSTIC_SIGNATURES: &[DiagnosticSignature] = &[
    // terminal 域
    DiagnosticSignature {
        id: "tool_timeout",
        domain: DiagnosticDomain::Terminal,
        exit_codes: None,
        non_success: false,
        require_timed_out: true,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["timed_out"],
    },
    DiagnosticSignature {
        id: "command_not_found",
        domain: DiagnosticDomain::Terminal,
        exit_codes: Some(&[127, 9009]),
        non_success: false,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["exit_code"],
    },
    DiagnosticSignature {
        id: "exit_nonzero",
        domain: DiagnosticDomain::Terminal,
        exit_codes: None,
        non_success: true,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["exit_code"],
    },
    // file 域
    DiagnosticSignature {
        id: "target_missing",
        domain: DiagnosticDomain::File,
        exit_codes: None,
        non_success: true,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        // §4.2 表：exit≠0 + stat exists=false。目标存在时不得命中。
        requires_exists: Some(false),
        requires_kind_mismatch: false,
        key_fields: &["target_path", "exists"],
    },
    DiagnosticSignature {
        id: "target_type_mismatch",
        domain: DiagnosticDomain::File,
        exit_codes: None,
        non_success: true,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        // §4.2 表：exit≠0 + stat 类型与工具期望不符（exists=true 且
        // kind != 期望；期望由参数键推断，无期望时不命中）。
        requires_exists: Some(true),
        requires_kind_mismatch: true,
        key_fields: &["target_path", "kind"],
    },
    DiagnosticSignature {
        id: "not_executable",
        domain: DiagnosticDomain::File,
        exit_codes: Some(&[126]),
        non_success: false,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["exit_code"],
    },
    // process 域
    DiagnosticSignature {
        id: "process_timeout",
        domain: DiagnosticDomain::Process,
        exit_codes: None,
        non_success: false,
        require_timed_out: true,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["timed_out"],
    },
    DiagnosticSignature {
        id: "exit_nonzero",
        domain: DiagnosticDomain::Process,
        exit_codes: None,
        non_success: true,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["exit_code"],
    },
    // environment 域
    DiagnosticSignature {
        id: "tool_unavailable",
        domain: DiagnosticDomain::Environment,
        exit_codes: None,
        non_success: false,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: None,
        policy_sources: Some(&["permission", "acaf", "retrieval_mode"]),
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["source", "code", "reason"],
    },
    DiagnosticSignature {
        id: "tool_not_found",
        domain: DiagnosticDomain::Environment,
        exit_codes: None,
        non_success: false,
        require_timed_out: false,
        require_encoding_lossy: false,
        error_kind: Some(ErrorKind::NotFound),
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["message"],
    },
    DiagnosticSignature {
        id: "encoding_lossy",
        domain: DiagnosticDomain::Environment,
        exit_codes: None,
        non_success: false,
        require_timed_out: false,
        require_encoding_lossy: true,
        error_kind: None,
        policy_sources: None,
        requires_exists: None,
        requires_kind_mismatch: false,
        key_fields: &["output_encoding"],
    },
];

/// 结构化签名匹配（P5：只消费结构化信号，不做文本子串判定）。
///
/// `stat` 为调用物状态探针结果（`TargetStat`：exists 三态、kind 仅在
/// exists=true 时有值、expected_kind 由参数键推断——`target_directory`→
/// directory、`target_file`/`file_path`→file、`path`→不限）。stat 探针
/// 结果属结构化信号（2026-08-28 全面审查处理补明 §4.2）。
pub fn match_signature(
    domain: DiagnosticDomain,
    exit_code: Option<i32>,
    timed_out: bool,
    output_encoding: Option<&str>,
    error_kind: Option<ErrorKind>,
    policy_source: Option<&str>,
    stat: TargetStat<'_>,
) -> Option<&'static DiagnosticSignature> {
    DIAGNOSTIC_SIGNATURES.iter().find(|sig| {
        if sig.domain != domain {
            return false;
        }
        if let Some(codes) = sig.exit_codes
            && !exit_code.is_some_and(|c| codes.contains(&c))
        {
            return false;
        }
        if sig.non_success && exit_code == Some(0) {
            return false;
        }
        if sig.require_timed_out && !timed_out {
            return false;
        }
        if sig.require_encoding_lossy && !output_encoding.is_some_and(|e| e.contains("lossy")) {
            return false;
        }
        if let Some(kind) = sig.error_kind
            && error_kind != Some(kind)
        {
            return false;
        }
        if let Some(sources) = sig.policy_sources
            && !policy_source.is_some_and(|s| sources.contains(&s))
        {
            return false;
        }
        if let Some(required) = sig.requires_exists
            && stat.exists != Some(required)
        {
            return false;
        }
        if sig.requires_kind_mismatch {
            let (Some(actual), Some(expected)) = (stat.kind, stat.expected_kind) else {
                return false;
            };
            if actual == expected {
                return false;
            }
        }
        true
    })
}

/// 从工具参数中提取文件目标路径（file 域；结构化，不做文本解析）。
fn target_path_from_arguments(arguments: &Value) -> Option<String> {
    ["target_file", "file_path", "path", "target_directory"]
        .iter()
        .find_map(|key| arguments.get(*key).and_then(Value::as_str))
        .map(crate::entities::normalize_entity_path)
}

/// 期望的目标类型（由参数键推断，结构化；无期望=类型不限）。
fn expected_kind_from_arguments(arguments: &Value) -> Option<&'static str> {
    if arguments.get("target_directory").is_some() {
        Some("directory")
    } else if arguments.get("target_file").is_some() || arguments.get("file_path").is_some() {
        Some("file")
    } else {
        // `path`（grep）：file/directory 均可，不做类型断言。
        None
    }
}

/// 调用物状态检查（确定性 fs 探针；bounded 且无模型参与）。file 域做
/// stat（path/exists/kind/size）；process 域附命令摘要与派生状态
/// （command/status）；environment 域由环境实体摘要承载，诊断信封不重复。
fn target_state_check(
    domain: DiagnosticDomain,
    arguments: &Value,
    exit_code: Option<i32>,
    timed_out: bool,
) -> Vec<KeyField> {
    match domain {
        DiagnosticDomain::File => {
            let Some(path) = target_path_from_arguments(arguments) else {
                return Vec::new();
            };
            let mut fields = vec![KeyField {
                key: "target_path".to_string(),
                value: path.clone(),
            }];
            match std::fs::metadata(&path) {
                Ok(metadata) => {
                    let kind = if metadata.is_dir() {
                        "directory"
                    } else if metadata.is_file() {
                        "file"
                    } else {
                        "other"
                    };
                    fields.push(KeyField {
                        key: "exists".to_string(),
                        value: "true".to_string(),
                    });
                    fields.push(KeyField {
                        key: "kind".to_string(),
                        value: kind.to_string(),
                    });
                    fields.push(KeyField {
                        key: "size".to_string(),
                        value: metadata.len().to_string(),
                    });
                }
                Err(_) => {
                    fields.push(KeyField {
                        key: "exists".to_string(),
                        value: "false".to_string(),
                    });
                    fields.push(KeyField {
                        key: "kind".to_string(),
                        value: "missing".to_string(),
                    });
                }
            }
            fields.truncate(DIAGNOSTIC_TARGET_STATE_MAX);
            fields
        }
        DiagnosticDomain::Process => {
            let mut fields = Vec::new();
            if let Some(command) = arguments.get("command").and_then(Value::as_str) {
                fields.push(KeyField {
                    key: "command".to_string(),
                    value: truncate_chars(command, 200),
                });
            }
            let status = if timed_out {
                "timed_out"
            } else if exit_code == Some(0) {
                "exited_ok"
            } else if exit_code.is_some() {
                "exited_error"
            } else {
                "unknown"
            };
            fields.push(KeyField {
                key: "status".to_string(),
                value: status.to_string(),
            });
            fields.truncate(DIAGNOSTIC_TARGET_STATE_MAX);
            fields
        }
        DiagnosticDomain::Terminal | DiagnosticDomain::Environment => Vec::new(),
    }
}

/// 有界原始尾部（raw；≤12 行 / ≤2KB）。
pub fn bounded_tail(output: &str, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .rev()
        .take(max_lines)
        .map(|l| truncate_chars(l, DIAGNOSTIC_TAIL_MAX_CHARS / max_lines.max(1)))
        .collect();
    lines.reverse();
    lines
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let head: String = s.chars().take(max_chars).collect();
        format!("{head}…[truncated]")
    }
}

/// 构造诊断记录（确定性、有界、无模型参与）。
#[allow(clippy::too_many_arguments)] // 镜像 ToolResult 全量结构化信号
pub fn diagnose_failure(
    domain: DiagnosticDomain,
    arguments: &Value,
    exit_code: Option<i32>,
    timed_out: bool,
    output_encoding: Option<&str>,
    error_kind: Option<ErrorKind>,
    policy_source: Option<&str>,
    policy_code: Option<&str>,
    policy_reason: Option<&str>,
    output: &str,
    log_pointer: &str,
) -> DiagnosticRecord {
    // 调用物状态探针先行（一次 stat，供签名匹配与 key_fields 共用，
    // 避免重复 fs 访问；stat 结果属结构化信号）。
    let target_state = target_state_check(domain, arguments, exit_code, timed_out);
    let exists = target_state
        .iter()
        .find(|f| f.key == "exists")
        .map(|f| f.value == "true");
    let kind = target_state
        .iter()
        .find(|f| f.key == "kind")
        .filter(|f| f.value != "missing")
        .map(|f| f.value.as_str());
    let stat = TargetStat {
        exists,
        kind,
        expected_kind: expected_kind_from_arguments(arguments),
    };
    let matched = match_signature(
        domain,
        exit_code,
        timed_out,
        output_encoding,
        error_kind,
        policy_source,
        stat,
    );
    let mut key_fields: Vec<KeyField> = Vec::new();
    if let Some(sig) = matched {
        for key in sig.key_fields {
            let value = match *key {
                "exit_code" => exit_code.map(|c| c.to_string()),
                "timed_out" => Some(timed_out.to_string()),
                "output_encoding" => output_encoding.map(str::to_string),
                "message" => error_kind.map(|k| format!("{k:?}")),
                "source" => policy_source.map(str::to_string),
                "code" => policy_code.map(str::to_string),
                "reason" => policy_reason.map(str::to_string),
                // stat 探针字段直接从 target_state 复制（同一份证据）。
                "target_path" | "exists" | "kind" => target_state
                    .iter()
                    .find(|f| f.key == *key)
                    .map(|f| f.value.clone()),
                _ => None,
            };
            if let Some(value) = value {
                key_fields.push(KeyField {
                    key: (*key).to_string(),
                    value,
                });
            }
            if key_fields.len() >= DIAGNOSTIC_KEY_FIELDS_MAX {
                break;
            }
        }
    }
    let tail = bounded_tail(output, DIAGNOSTIC_TAIL_MAX_LINES);
    let mut record = DiagnosticRecord {
        exit_code,
        matched_signature: matched.map(|s| s.id.to_string()),
        key_fields,
        target_state,
        tail,
        // raw 兜底标记：无签名命中时尾部为 raw（设计 §4.2）。
        tail_is_raw: matched.is_none(),
        log_pointer: log_pointer.to_string(),
        truncated: false,
    };
    enforce_serialized_cap(&mut record);
    record
}

/// 序列化总上限 ≤2KB 强制（超出按 tail → key_fields → target_state 顺序
/// 截断；exit_code / matched_signature / log_pointer 恒保留）。
pub fn enforce_serialized_cap(record: &mut DiagnosticRecord) {
    let over = |r: &DiagnosticRecord| -> bool {
        serde_json::to_string(r)
            .map(|s| s.len() > DIAGNOSTIC_MAX_SERIALIZED_BYTES)
            .unwrap_or(true)
    };
    if !over(record) {
        return;
    }
    while over(record) && !record.tail.is_empty() {
        record.tail.pop();
    }
    while over(record) && !record.key_fields.is_empty() {
        record.key_fields.pop();
    }
    while over(record) && !record.target_state.is_empty() {
        record.target_state.pop();
    }
    if over(record) {
        // 极不可能（指针+退出码仍超 2KB）：尾部整体清空保底。
        record.tail.clear();
        record.key_fields.clear();
        record.target_state.clear();
    }
    record.truncated = true;
}

/// 极简文本渲染（固定字段顺序；供失败消息附着，同构稳定）。
pub fn render_diagnostic(record: &DiagnosticRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "诊断: exit_code={}",
        record
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    if let Some(sig) = &record.matched_signature {
        out.push_str(&format!(" signature={sig}"));
    }
    if !record.key_fields.is_empty() {
        let joined: Vec<String> = record
            .key_fields
            .iter()
            .map(|f| format!("{}={}", f.key, f.value))
            .collect();
        out.push_str(&format!(" | {}", joined.join(" ")));
    }
    if !record.target_state.is_empty() {
        let joined: Vec<String> = record
            .target_state
            .iter()
            .map(|f| format!("{}={}", f.key, f.value))
            .collect();
        out.push_str(&format!(" | [target] {}", joined.join(" ")));
    }
    out.push_str(&format!(" | log: {}", record.log_pointer));
    if !record.tail.is_empty() {
        if record.tail_is_raw {
            out.push_str(" | tail(raw): ");
        } else {
            out.push_str(" | tail: ");
        }
        out.push_str(&record.tail.join(" ⏎ "));
    }
    if record.truncated {
        out.push_str(" | truncated");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 默认无 stat 探针的快捷匹配（既有信号面测试用；stat 断言测试显式传参）。
    fn match_sig(
        domain: DiagnosticDomain,
        exit_code: Option<i32>,
        timed_out: bool,
        encoding: Option<&str>,
        err: Option<ErrorKind>,
        policy: Option<&str>,
    ) -> Option<&'static DiagnosticSignature> {
        match_signature(
            domain,
            exit_code,
            timed_out,
            encoding,
            err,
            policy,
            TargetStat::default(),
        )
    }

    #[test]
    fn command_not_found_matches_posix_and_windows_exit_codes() {
        for code in [127, 9009] {
            let sig = match_sig(
                DiagnosticDomain::Terminal,
                Some(code),
                false,
                Some("utf-8"),
                None,
                None,
            )
            .expect("signature");
            assert_eq!(sig.id, "command_not_found");
        }
    }

    #[test]
    fn timeout_signature_precedes_nonzero_for_timed_out() {
        let sig = match_sig(DiagnosticDomain::Terminal, Some(1), true, None, None, None)
            .expect("signature");
        assert_eq!(sig.id, "tool_timeout");
    }

    #[test]
    fn timed_out_precedes_command_not_found() {
        // 2026-08-28 全面审查处理：timed_out 是更确定的宿主信号，即使
        // exit_code 命中 command_not_found 集合（127）也优先 tool_timeout。
        let sig = match_sig(
            DiagnosticDomain::Terminal,
            Some(127),
            true,
            None,
            None,
            None,
        )
        .expect("signature");
        assert_eq!(sig.id, "tool_timeout");
    }

    #[test]
    fn nonzero_without_other_signal_matches_exit_nonzero() {
        let sig = match_sig(
            DiagnosticDomain::Terminal,
            Some(2),
            false,
            Some("utf-8"),
            None,
            None,
        )
        .expect("signature");
        assert_eq!(sig.id, "exit_nonzero");
        let sig = match_sig(DiagnosticDomain::Process, Some(3), false, None, None, None)
            .expect("signature");
        assert_eq!(sig.id, "exit_nonzero");
    }

    #[test]
    fn success_exit_matches_no_signature() {
        assert!(match_sig(DiagnosticDomain::Terminal, Some(0), false, None, None, None).is_none());
    }

    #[test]
    fn policy_denial_matches_tool_unavailable_only_with_structured_source() {
        let sig = match_sig(
            DiagnosticDomain::Environment,
            None,
            false,
            None,
            None,
            Some("acaf"),
        )
        .expect("signature");
        assert_eq!(sig.id, "tool_unavailable");
        // 结构化字段外的自由文本来源不匹配（P5：无子串判定）。
        assert!(
            match_sig(
                DiagnosticDomain::Environment,
                None,
                false,
                None,
                None,
                Some("hash-400-noise"),
            )
            .is_none()
        );
    }

    #[test]
    fn tool_not_found_matches_error_kind_only() {
        let sig = match_sig(
            DiagnosticDomain::Environment,
            None,
            false,
            None,
            Some(ErrorKind::NotFound),
            None,
        )
        .expect("signature");
        assert_eq!(sig.id, "tool_not_found");
        assert!(
            match_sig(
                DiagnosticDomain::Environment,
                None,
                false,
                None,
                Some(ErrorKind::ExecutionFailed),
                None,
            )
            .is_none()
        );
    }

    #[test]
    fn encoding_lossy_matches_structured_encoding_marker() {
        let sig = match_sig(
            DiagnosticDomain::Environment,
            Some(0),
            false,
            Some("utf-8-lossy"),
            None,
            None,
        )
        .expect("signature");
        assert_eq!(sig.id, "encoding_lossy");
        assert!(
            match_sig(
                DiagnosticDomain::Environment,
                Some(0),
                false,
                Some("utf-8"),
                None,
                None,
            )
            .is_none()
        );
    }

    #[test]
    fn diagnose_builds_bounded_record_with_raw_tail_fallback() {
        let arguments = json!({ "target_file": "definitely_missing_xyz.txt" });
        let record = diagnose_failure(
            DiagnosticDomain::File,
            &arguments,
            Some(1),
            false,
            Some("utf-8"),
            None,
            None,
            None,
            None,
            "error line one\nerror line two\n",
            "TRC-1",
        );
        assert_eq!(record.matched_signature.as_deref(), Some("target_missing"));
        assert!(
            record
                .key_fields
                .iter()
                .any(|f| f.key == "exists" && f.value == "false")
        );
        assert!(
            record
                .target_state
                .iter()
                .any(|f| f.key == "exists" && f.value == "false")
        );
        assert_eq!(record.tail.len(), 2);
        assert_eq!(record.log_pointer, "TRC-1");
        assert!(!record.truncated);
        let rendered = render_diagnostic(&record);
        assert!(rendered.contains("signature=target_missing"));
        assert!(rendered.contains("log: TRC-1"));
        assert!(rendered.contains("tail: "));
        assert!(!record.tail_is_raw);
    }

    #[test]
    fn file_target_missing_requires_stat_missing() {
        // 目标不存在 → target_missing。
        let missing = json!({ "target_file": "definitely_missing_xyz_2.txt" });
        let record = diagnose_failure(
            DiagnosticDomain::File,
            &missing,
            Some(1),
            false,
            None,
            None,
            None,
            None,
            None,
            "no such file",
            "TRC-5",
        );
        assert_eq!(record.matched_signature.as_deref(), Some("target_missing"));
        // 目标存在（如权限拒绝/内容校验失败）→ 不得误报 target_missing。
        let path = std::env::temp_dir().join(format!("rzdiag-exists-{}", std::process::id()));
        std::fs::write(&path, b"x").expect("write temp file");
        let exists_args = json!({ "target_file": path.to_string_lossy() });
        let record = diagnose_failure(
            DiagnosticDomain::File,
            &exists_args,
            Some(1),
            false,
            None,
            None,
            None,
            None,
            None,
            "permission denied",
            "TRC-6",
        );
        let _ = std::fs::remove_file(&path);
        assert_ne!(record.matched_signature.as_deref(), Some("target_missing"));
        assert!(record.tail_is_raw, "no signature → raw fallback");
        assert!(record.tail.iter().any(|l| l.contains("permission denied")));
    }

    #[test]
    fn not_executable_reachable_for_126() {
        // 126 精确命中（stat 可用时 target_missing/type_mismatch 均不吞）。
        let sig = match_signature(
            DiagnosticDomain::File,
            Some(126),
            false,
            None,
            None,
            None,
            TargetStat {
                exists: Some(true),
                kind: Some("file"),
                expected_kind: Some("file"),
            },
        )
        .expect("signature");
        assert_eq!(sig.id, "not_executable");
        // stat 不可用（无路径/无探针）时 126 仍精确命中。
        let sig = match_sig(DiagnosticDomain::File, Some(126), false, None, None, None)
            .expect("signature");
        assert_eq!(sig.id, "not_executable");
    }

    #[test]
    fn target_type_mismatch_matches_wrong_kind() {
        // 目录被当文件读（target_file + 期望 file，实际 directory）。
        let dir = std::env::temp_dir().join(format!("rzdiag-dir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let arguments = json!({ "target_file": dir.to_string_lossy() });
        let record = diagnose_failure(
            DiagnosticDomain::File,
            &arguments,
            Some(1),
            false,
            None,
            None,
            None,
            None,
            None,
            "is a directory",
            "TRC-7",
        );
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            record.matched_signature.as_deref(),
            Some("target_type_mismatch")
        );
        assert!(
            record
                .target_state
                .iter()
                .any(|f| f.key == "kind" && f.value == "directory")
        );
    }

    #[test]
    fn process_domain_target_state_has_status() {
        let record = diagnose_failure(
            DiagnosticDomain::Process,
            &json!({ "command": "cargo test" }),
            Some(1),
            false,
            None,
            None,
            None,
            None,
            None,
            "failed",
            "TRC-8",
        );
        assert_eq!(record.matched_signature.as_deref(), Some("exit_nonzero"));
        assert!(
            record
                .target_state
                .iter()
                .any(|f| f.key == "status" && f.value == "exited_error")
        );
        assert!(
            record
                .target_state
                .iter()
                .any(|f| f.key == "command" && f.value == "cargo test")
        );
    }

    #[test]
    fn tail_is_raw_flag_reflects_signature_match() {
        // 无签名命中（file 域目标存在但无更具体签名，如权限拒绝）→ raw。
        let path = std::env::temp_dir().join(format!("rzdiag-raw-{}", std::process::id()));
        std::fs::write(&path, b"x").expect("write temp file");
        let raw = diagnose_failure(
            DiagnosticDomain::File,
            &json!({ "target_file": path.to_string_lossy() }),
            Some(1),
            false,
            None,
            None,
            None,
            None,
            None,
            "unclassified",
            "TRC-9",
        );
        let _ = std::fs::remove_file(&path);
        assert!(raw.matched_signature.is_none());
        assert!(raw.tail_is_raw);
        assert!(render_diagnostic(&raw).contains("tail(raw)"));
        // 有签名命中 → tail 仅为补充上下文。
        let sig = diagnose_failure(
            DiagnosticDomain::Terminal,
            &json!({}),
            Some(1),
            true,
            None,
            None,
            None,
            None,
            None,
            "timed out",
            "TRC-10",
        );
        assert_eq!(sig.matched_signature.as_deref(), Some("tool_timeout"));
        assert!(!sig.tail_is_raw);
        assert!(render_diagnostic(&sig).contains("tail: "));
        assert!(!render_diagnostic(&sig).contains("tail(raw)"));
    }

    #[test]
    fn encoding_lossy_full_chain() {
        let record = diagnose_failure(
            DiagnosticDomain::Environment,
            &json!({}),
            Some(0),
            false,
            Some("utf-8-lossy"),
            None,
            None,
            None,
            None,
            "replacement chars",
            "TRC-11",
        );
        assert_eq!(record.matched_signature.as_deref(), Some("encoding_lossy"));
        assert!(
            record
                .key_fields
                .iter()
                .any(|f| f.key == "output_encoding" && f.value == "utf-8-lossy")
        );
        assert!(!record.tail_is_raw);
    }

    #[test]
    fn expected_kind_from_arguments_by_field() {
        assert_eq!(
            expected_kind_from_arguments(&json!({ "target_directory": "x" })),
            Some("directory")
        );
        assert_eq!(
            expected_kind_from_arguments(&json!({ "target_file": "x" })),
            Some("file")
        );
        assert_eq!(
            expected_kind_from_arguments(&json!({ "file_path": "x" })),
            Some("file")
        );
        // grep 的 path：file/directory 均可，不约束。
        assert_eq!(expected_kind_from_arguments(&json!({ "path": "x" })), None);
        assert_eq!(expected_kind_from_arguments(&json!({})), None);
    }

    #[test]
    fn hash_containing_400_is_never_classified_by_substring() {
        // P5 回归：哈希串/文本含 "400" 不产生任何签名判定。
        let output = "artifact_sha256=abc400def hash-400-noise payload\n";
        let arguments = json!({ "command": "noop" });
        let record = diagnose_failure(
            DiagnosticDomain::Terminal,
            &arguments,
            Some(1),
            false,
            Some("utf-8"),
            None,
            None,
            None,
            None,
            output,
            "TRC-2",
        );
        assert_eq!(record.matched_signature.as_deref(), Some("exit_nonzero"));
        assert!(!record.tail.iter().any(|l| l.contains("matched")));
        // tail 保留 raw 内容（有界），但不参与分类。
        assert!(record.tail.iter().any(|l| l.contains("artifact_sha256")));
    }

    #[test]
    fn tail_is_bounded_and_capped() {
        let output = (0..50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let record = diagnose_failure(
            DiagnosticDomain::Terminal,
            &json!({}),
            Some(1),
            false,
            None,
            None,
            None,
            None,
            None,
            &output,
            "TRC-3",
        );
        assert!(record.tail.len() <= DIAGNOSTIC_TAIL_MAX_LINES);
        // 最近的行优先。
        assert_eq!(record.tail.last().map(|l| l.as_str()), Some("line 49"));
    }

    #[test]
    fn serialized_record_never_exceeds_2kb() {
        let huge = "x".repeat(6000);
        let output = (0..40).map(|_| huge.clone()).collect::<Vec<_>>().join("\n");
        let record = diagnose_failure(
            DiagnosticDomain::Terminal,
            &json!({}),
            Some(1),
            true,
            None,
            None,
            None,
            None,
            None,
            &output,
            "TRC-LONG",
        );
        let serialized = serde_json::to_string(&record).expect("serialize");
        assert!(serialized.len() <= DIAGNOSTIC_MAX_SERIALIZED_BYTES);
        assert!(record.truncated);
        assert_eq!(record.log_pointer, "TRC-LONG");
    }

    #[test]
    fn target_state_checks_file_kind() {
        let path = std::env::temp_dir().join(format!("rzdiag-{}", std::process::id()));
        std::fs::write(&path, b"x").expect("write temp file");
        let arguments = json!({ "target_file": path.to_string_lossy() });
        let record = diagnose_failure(
            DiagnosticDomain::File,
            &arguments,
            Some(1),
            false,
            None,
            None,
            None,
            None,
            None,
            "",
            "TRC-4",
        );
        let _ = std::fs::remove_file(&path);
        assert!(
            record
                .target_state
                .iter()
                .any(|f| f.key == "kind" && f.value == "file")
        );
        assert!(
            record
                .target_state
                .iter()
                .any(|f| f.key == "size" && f.value == "1")
        );
    }
}
