//! PlanArtifact — the four-section structured plan.
//!
//! Ported from Python `plan_mode.PlanArtifact` / `verify_plan_artifact`.
//! The sha256 is deterministic over canonical JSON of the plan contents
//! (hash field excluded), so plan versions chain like journal events.

use crate::{canonical_json, sha256_hex};

/// One plan section (four required: investigation / plan / design / implementation).
#[derive(Debug, Clone)]
pub struct PlanSection {
    pub title: String,
    pub content_md: String,
    pub evidence_status: String,
    pub source_refs: Vec<String>,
}

/// The structured plan artifact.
#[derive(Debug, Clone)]
pub struct PlanArtifact {
    pub plan_id: String,
    pub task_id: String,
    pub run_id: String,
    pub workspace_root: String,
    pub created_at: String,
    pub planning_policy: String,
    pub sections: Vec<PlanSection>,
    pub version: u32,
    pub previous_plan_sha256: String,
    pub deferred_decisions: Vec<String>,
    // Approval fields (populated after plan review).
    pub approval_decision: String,
    pub approval_timestamp: String,
    pub approval_authority: String,
    pub selected_approval_policy: String,
}

impl PlanArtifact {
    /// The four canonical section titles (§3.1).
    pub const REQUIRED_SECTION_TITLES: [&'static str; 4] = [
        "前期调查",
        "具体计划",
        "具体设计",
        "实施方案",
    ];

    pub fn new(
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        run_id: impl Into<String>,
        workspace_root: impl Into<String>,
        planning_policy: impl Into<String>,
        sections: Vec<PlanSection>,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            run_id: run_id.into(),
            workspace_root: workspace_root.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            planning_policy: planning_policy.into(),
            sections,
            version: 1,
            previous_plan_sha256: String::new(),
            deferred_decisions: Vec::new(),
            approval_decision: String::new(),
            approval_timestamp: String::new(),
            approval_authority: String::new(),
            selected_approval_policy: String::new(),
        }
    }

    /// Deterministic SHA-256 of the plan contents (Python `compute_sha256`).
    pub fn compute_sha256(&self) -> String {
        let mut source_refs: Vec<Vec<String>> = self
            .sections
            .iter()
            .map(|s| {
                let mut refs = s.source_refs.clone();
                refs.sort();
                refs
            })
            .collect();
        source_refs.sort();

        let mut deferred = self.deferred_decisions.clone();
        deferred.sort();

        let payload = serde_json::json!({
            "plan_id": self.plan_id,
            "task_id": self.task_id,
            "run_id": self.run_id,
            "workspace_root": self.workspace_root,
            "created_at": self.created_at,
            "planning_policy": self.planning_policy,
            "version": self.version,
            "previous_plan_sha256": self.previous_plan_sha256,
            "sections": self.sections.iter().enumerate().map(|(i, s)| {
                serde_json::json!({
                    "title": s.title,
                    "content_md": s.content_md,
                    "evidence_status": s.evidence_status,
                    "source_refs": source_refs[i],
                })
            }).collect::<Vec<_>>(),
            "deferred_decisions": deferred,
            "approval_decision": self.approval_decision,
            "approval_timestamp": self.approval_timestamp,
            "approval_authority": self.approval_authority,
            "selected_approval_policy": self.selected_approval_policy,
        });
        sha256_hex(&canonical_json(&payload).unwrap_or_default())
    }
}

/// Verification result (Python `verify_plan_artifact` receipt — simplified).
#[derive(Debug, Clone, Default)]
pub struct PlanVerification {
    pub valid: bool,
    pub errors: Vec<String>,
}

/// Verify a plan artifact's structural invariants:
/// exactly the four required sections (titles match), non-empty content,
/// non-empty plan/task/run ids, deterministic hash.
pub fn verify_plan_artifact(artifact: &PlanArtifact) -> PlanVerification {
    let mut errors: Vec<String> = Vec::new();

    if artifact.plan_id.is_empty() {
        errors.push("plan_id must be non-empty".to_string());
    }
    if artifact.task_id.is_empty() {
        errors.push("task_id must be non-empty".to_string());
    }
    if artifact.run_id.is_empty() {
        errors.push("run_id must be non-empty".to_string());
    }

    let titles: Vec<&str> = artifact.sections.iter().map(|s| s.title.as_str()).collect();
    if titles != PlanArtifact::REQUIRED_SECTION_TITLES {
        errors.push(format!(
            "sections must be exactly the four required titles in order, got {titles:?}"
        ));
    }
    if artifact.sections.iter().any(|s| s.content_md.trim().is_empty()) {
        errors.push("every section must have non-empty content".to_string());
    }

    // Deterministic hash sanity: recomputing must be stable.
    let h1 = artifact.compute_sha256();
    let h2 = artifact.compute_sha256();
    if h1 != h2 {
        errors.push("plan sha256 must be deterministic".to_string());
    }

    PlanVerification {
        valid: errors.is_empty(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(title: &str) -> PlanSection {
        PlanSection {
            title: title.to_string(),
            content_md: format!("{title} 内容"),
            evidence_status: "observed".to_string(),
            source_refs: vec!["docs/a.md".to_string()],
        }
    }

    fn artifact() -> PlanArtifact {
        PlanArtifact::new(
            "PLAN-1",
            "TASK-1",
            "RUN-1",
            "/workspace",
            "manual",
            vec![
                section("前期调查"),
                section("具体计划"),
                section("具体设计"),
                section("实施方案"),
            ],
        )
    }

    #[test]
    fn valid_plan_passes_verification() {
        let v = verify_plan_artifact(&artifact());
        assert!(v.valid, "{:?}", v.errors);
    }

    #[test]
    fn missing_sections_fail_verification() {
        let mut a = artifact();
        a.sections.pop();
        let v = verify_plan_artifact(&a);
        assert!(!v.valid);
        assert!(v.errors.iter().any(|e| e.contains("four required")));
    }

    #[test]
    fn sha256_is_deterministic_and_content_sensitive() {
        // Fix the timestamp (Python semantics: created_at is caller-supplied).
        let mut a = artifact();
        a.created_at = "2026-08-04T00:00:00Z".to_string();
        let mut b = artifact();
        b.created_at = a.created_at.clone();
        assert_eq!(a.compute_sha256(), b.compute_sha256());
        let mut c = artifact();
        c.created_at = a.created_at.clone();
        c.sections[0].content_md.push_str(" changed");
        assert_ne!(a.compute_sha256(), c.compute_sha256());
    }

    #[test]
    fn empty_ids_fail_verification() {
        let mut a = artifact();
        a.plan_id.clear();
        let v = verify_plan_artifact(&a);
        assert!(!v.valid);
    }
}
