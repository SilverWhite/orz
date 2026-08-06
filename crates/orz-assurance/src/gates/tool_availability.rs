//! Tool availability gate — mechanical probe + gate decision.
//!
//! Ported from Python `assurance/tool_availability_gate.py` (spec reference).
//! The probe is mechanical: it never invokes a model and never performs
//! network requests. Each tool spec is classified against the probe registry:
//! - tool_id not in registry → `unprobed`
//! - registry value `None` → `degraded`
//! - `true` → `available`; `false` → `unavailable`
//!
//! Gate decision priority (Python `build_tool_availability_gate_receipt`):
//! any unavailable → Block; any degraded → Warn; not all probed → Defer;
//! otherwise → Pass.

use std::collections::HashMap;

use crate::GateDecision;

/// Tool capability categories (Python `CAPABILITY_ENUM`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    Search,
    FileRead,
    FileWrite,
    FileEdit,
    BashExec,
    WebFetch,
    Subagent,
    ToolRegistry,
    NetworkIo,
}

/// Probe result status (Python `VALID_STATUSES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeStatus {
    Available,
    Unavailable,
    Unprobed,
    Degraded,
}

/// One tool declaration to be probed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub tool_id: String,
    pub tool_name: String,
    pub capability: Capability,
    pub probe_method: String,
}

impl ToolSpec {
    pub fn new(
        tool_id: impl Into<String>,
        tool_name: impl Into<String>,
        capability: Capability,
        probe_method: impl Into<String>,
    ) -> Self {
        Self {
            tool_id: tool_id.into(),
            tool_name: tool_name.into(),
            capability,
            probe_method: probe_method.into(),
        }
    }
}

/// Per-tool probe result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProbe {
    pub spec: ToolSpec,
    pub status: ProbeStatus,
}

/// Aggregate probe report with per-status counts.
#[derive(Debug, Clone, Default)]
pub struct ToolAvailabilityReport {
    pub probes: Vec<ToolProbe>,
    pub available: Vec<String>,
    pub unavailable: Vec<String>,
    pub unprobed: Vec<String>,
    pub degraded: Vec<String>,
}

impl ToolAvailabilityReport {
    pub fn is_all_probed(&self) -> bool {
        self.unprobed.is_empty()
    }
}

/// Classify every tool spec against the probe registry.
///
/// `probe_registry` maps `tool_id` → `Option<bool>`:
/// missing id → `unprobed`; `None` → `degraded`; `Some(true)` → `available`;
/// `Some(false)` → `unavailable`.
pub fn probe_tool_availability(
    specs: &[ToolSpec],
    probe_registry: &HashMap<String, Option<bool>>,
) -> ToolAvailabilityReport {
    let mut report = ToolAvailabilityReport::default();
    for spec in specs {
        let status = match probe_registry.get(&spec.tool_id) {
            None => ProbeStatus::Unprobed,
            Some(None) => ProbeStatus::Degraded,
            Some(Some(true)) => ProbeStatus::Available,
            Some(Some(false)) => ProbeStatus::Unavailable,
        };
        match status {
            ProbeStatus::Available => report.available.push(spec.tool_id.clone()),
            ProbeStatus::Unavailable => report.unavailable.push(spec.tool_id.clone()),
            ProbeStatus::Unprobed => report.unprobed.push(spec.tool_id.clone()),
            ProbeStatus::Degraded => report.degraded.push(spec.tool_id.clone()),
        }
        report.probes.push(ToolProbe {
            spec: spec.clone(),
            status,
        });
    }
    report.available.sort();
    report.unavailable.sort();
    report.unprobed.sort();
    report.degraded.sort();
    report
}

/// Gate decision over a probe report (Python `build_tool_availability_gate_receipt`).
pub fn gate_decision(report: &ToolAvailabilityReport) -> GateDecision {
    if !report.unavailable.is_empty() {
        GateDecision::Block {
            reason_codes: vec!["tool_unavailable".to_string()],
        }
    } else if !report.degraded.is_empty() {
        GateDecision::Warn {
            reason_codes: vec!["tool_degraded".to_string()],
        }
    } else if !report.is_all_probed() {
        GateDecision::Defer {
            missing: report
                .unprobed
                .iter()
                .map(|id| format!("probe: {id}"))
                .collect(),
        }
    } else {
        GateDecision::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GateDecision;

    fn spec(id: &str) -> ToolSpec {
        ToolSpec::new(id, id, Capability::FileRead, "acp_tool_registry")
    }

    #[test]
    fn unavailable_tool_blocks() {
        let mut registry = HashMap::new();
        registry.insert("read_file".to_string(), Some(false));
        let report = probe_tool_availability(&[spec("read_file")], &registry);
        assert!(matches!(gate_decision(&report), GateDecision::Block { .. }));
    }

    #[test]
    fn degraded_probe_warns() {
        let mut registry = HashMap::new();
        registry.insert("read_file".to_string(), None);
        let report = probe_tool_availability(&[spec("read_file")], &registry);
        assert!(matches!(gate_decision(&report), GateDecision::Warn { .. }));
    }

    #[test]
    fn unprobed_tools_defer() {
        let report = probe_tool_availability(&[spec("read_file")], &HashMap::new());
        assert!(matches!(gate_decision(&report), GateDecision::Defer { .. }));
    }

    #[test]
    fn all_available_passes() {
        let mut registry = HashMap::new();
        registry.insert("read_file".to_string(), Some(true));
        registry.insert("bash".to_string(), Some(true));
        let report = probe_tool_availability(
            &[spec("read_file"), spec("bash")],
            &registry,
        );
        assert_eq!(gate_decision(&report), GateDecision::Pass);
    }

    #[test]
    fn block_precedes_warn_and_defer() {
        let mut registry = HashMap::new();
        registry.insert("read_file".to_string(), Some(false));
        registry.insert("bash".to_string(), None);
        registry.insert("grep".to_string(), Some(true));
        let report = probe_tool_availability(
            &[spec("read_file"), spec("bash"), spec("grep")],
            &registry,
        );
        assert!(matches!(gate_decision(&report), GateDecision::Block { .. }));
    }

    #[test]
    fn warn_precedes_defer() {
        let mut registry = HashMap::new();
        registry.insert("bash".to_string(), None);
        let report = probe_tool_availability(&[spec("bash")], &registry);
        // degraded (warn) precedes unprobed (defer)
        assert!(matches!(gate_decision(&report), GateDecision::Warn { .. }));
    }
}
