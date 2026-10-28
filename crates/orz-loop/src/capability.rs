//! 0ds M1 v1 (2026-10-28, 366 charter §1 item 4): the thin capability
//! registry.
//!
//! Scope is deliberately mechanical and session-scoped:
//! - a fixed tool-name whitelist (the canonical `tool_probe::WORK_TOOLS`);
//! - one preopened session root (recorded, never written into state/journal
//!   as content beyond the path label);
//! - a rights label derived from the existing risk-class lattice.
//!
//! The registry answers "is this tool declared for this session?" before the
//! permission/ticket gates. Permission policy remains the policy decision;
//! this layer exists so an undeclared tool call is denied *and audited* even
//! when a host would otherwise accept it. It never stores secrets.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::host::RiskClass;
use crate::tool::ToolDispatcher;
use crate::tool_probe::WORK_TOOLS;

/// Payload decision values (single source for producer/schema/fixtures).
pub const CAPABILITY_DECISION_GRANTED: &str = "granted";
pub const CAPABILITY_DECISION_DENIED: &str = "denied";
/// Stable audit reason; never includes argument/secret content.
pub const CAPABILITY_REASON_NOT_DECLARED: &str = "capability_not_declared";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDecision {
    pub allowed: bool,
    pub tool: String,
    pub rights: Vec<&'static str>,
    pub decision: &'static str,
    pub reason: Option<&'static str>,
    pub preopen_root: Option<String>,
}

impl CapabilityDecision {
    /// Journal payload (no raw tool arguments, no secrets).
    pub fn payload(&self) -> serde_json::Value {
        let mut payload = serde_json::json!({
            "tool": self.tool,
            "rights": self.rights,
            "decision": self.decision,
        });
        if let Some(reason) = self.reason {
            payload["reason"] = serde_json::Value::String(reason.to_string());
        }
        if let Some(root) = &self.preopen_root {
            payload["preopen_root"] = serde_json::Value::String(root.clone());
        }
        payload
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    allowed: BTreeSet<&'static str>,
    preopen_root: Option<PathBuf>,
}

/// Tools that are declared outside the work-tool probe set but still part
/// of the loop/host surface. Keep this list mechanical; adding a new tool
/// must add its name here (the deny path is the safety net).
const SUPPLEMENTAL_TOOLS: &[&str] = &[
    "web_search",
    "web_fetch",
    "bash",
    "submit",
    "project_doc_index",
    "browser_read",
    "browser_control",
    "spawn_scout",
    "self_check",
];

impl Default for CapabilityRegistry {
    fn default() -> Self {
        let mut allowed: BTreeSet<&'static str> = WORK_TOOLS.iter().copied().collect();
        allowed.extend(ToolDispatcher::READ_ONLY_EXEMPT_TOOLS.iter().copied());
        allowed.extend(SUPPLEMENTAL_TOOLS.iter().copied());
        Self {
            allowed,
            preopen_root: None,
        }
    }
}

impl CapabilityRegistry {
    /// Session preopen: the root is recorded with every decision. v1 does
    /// not inspect tool arguments here; path-level enforcement stays with
    /// the existing permission/snapshot gates so this thin layer cannot
    /// silently widen or narrow them.
    pub fn for_session(root: impl AsRef<Path>) -> Self {
        Self {
            preopen_root: Some(root.as_ref().to_path_buf()),
            ..Self::default()
        }
    }

    pub fn allowed_tools(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.allowed.iter().copied()
    }

    pub fn preopen_root(&self) -> Option<&Path> {
        self.preopen_root.as_deref()
    }

    pub fn evaluate(&self, tool: &str) -> CapabilityDecision {
        let rights = rights_for(tool).to_vec();
        if !self.allowed.contains(tool) {
            return CapabilityDecision {
                allowed: false,
                tool: tool.to_string(),
                rights,
                decision: CAPABILITY_DECISION_DENIED,
                reason: Some(CAPABILITY_REASON_NOT_DECLARED),
                preopen_root: self
                    .preopen_root
                    .as_ref()
                    .map(|path| path.display().to_string()),
            };
        }
        CapabilityDecision {
            allowed: true,
            tool: tool.to_string(),
            rights,
            decision: CAPABILITY_DECISION_GRANTED,
            reason: None,
            preopen_root: self
                .preopen_root
                .as_ref()
                .map(|path| path.display().to_string()),
        }
    }
}

/// Mechanical rights label; intentionally not a policy grant.
pub fn rights_for(tool: &str) -> &'static [&'static str] {
    match ToolDispatcher::risk_class(tool) {
        RiskClass::ReadOnly => &["read"],
        RiskClass::LocalMutation => &["write"],
        RiskClass::NetworkCall => &["network"],
        RiskClass::SandboxEscape => &["execute"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_tools_are_granted_with_mechanical_rights() {
        let registry = CapabilityRegistry::for_session("/tmp/session-root");
        let read = registry.evaluate("read_file");
        assert!(read.allowed);
        assert_eq!(read.rights, vec!["read"]);
        assert_eq!(read.decision, CAPABILITY_DECISION_GRANTED);
        assert_eq!(read.preopen_root.as_deref(), Some("/tmp/session-root"));

        let write = registry.evaluate("search_replace");
        assert!(write.allowed);
        assert_eq!(write.rights, vec!["write"]);
        let shell = registry.evaluate("run_terminal_cmd");
        assert!(shell.allowed);
        assert_eq!(shell.rights, vec!["write"]);
        assert_eq!(rights_for("bash"), &["execute"]);
    }

    #[test]
    fn undeclared_tool_is_denied_with_stable_reason() {
        let registry = CapabilityRegistry::default();
        let decision = registry.evaluate("not_a_real_tool");
        assert!(!decision.allowed);
        assert_eq!(decision.decision, CAPABILITY_DECISION_DENIED);
        assert_eq!(decision.reason, Some(CAPABILITY_REASON_NOT_DECLARED));
        let payload = decision.payload();
        assert_eq!(payload["tool"], "not_a_real_tool");
        assert_eq!(payload["decision"], "denied");
        assert!(payload.get("preopen_root").is_none());
    }

    #[test]
    fn payload_never_contains_argument_or_secret_fields() {
        let registry = CapabilityRegistry::default();
        let decision = registry.evaluate("read_file");
        let payload = decision.payload();
        assert!(payload.get("arguments").is_none());
        assert!(payload.get("token").is_none());
        assert!(payload.get("secret").is_none());
    }
}
