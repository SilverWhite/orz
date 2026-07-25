"""Runtime-neutral General Assurance Kernel development fixtures.

P1-P5 provide testable contracts, lifecycle controls, guarded execution,
instruction/capability authorization, retained metadata audit/recovery gates,
a workspace-first integrated path, and synthetic user-task preflight.
GSA-CORE adds a narrow domain-neutral multi-file read-only review. They remain
development/conformance mechanisms rather than a production security boundary.
"""

from .archive import ArchiveController, resume_archived_conversation
from .archive_verifier import verify_archive
from .audit import AuditLedger, verify_audit_seal
from .conversation import ConversationNamespace
from .envelope import create_security_envelope, verify_security_envelope
from .errors import AssuranceError
from .guarded_execution import (
    DockerProcessTracker,
    build_guarded_frozen_context,
    execute_guarded_no_model_action,
    verify_guarded_execution_receipt,
)
from .general_science_review import review_general_science_bundle
from .keystore import MemoryInstallationKeyStore, WindowsDpapiInstallationKeyStore
from .instruction_gate import (
    authorize_action_candidate,
    delegate_capabilities,
    record_instruction_provenance,
    verify_action_authorization,
    verify_capability_delegation,
    verify_instruction_provenance,
)
from .integrated_run import (
    build_workspace_first_frozen_context,
    execute_workspace_first_integrated_run,
    initialize_workspace_marker,
    load_execution_backend_policy,
    select_execution_backend,
    verify_workspace_first_integrated_run,
)
from .permit import (
    consume_sensitive_action_permit,
    issue_sensitive_action_permit,
    verify_sensitive_action_permit,
)
from .profile_registry import (
    load_profile_registry,
    resolve_effective_profile,
    validate_profile_registry_semantics,
)
from .readonly_projection import (
    load_readonly_task_projection,
    project_complex_task_readonly,
    verify_complex_task_readonly_projection,
)
from .recovery import (
    authorize_recovery_candidate,
    create_recovery_candidate,
    recovery_permit_binding,
    verify_recovery_authorization,
    verify_recovery_candidate,
)
from .sandbox import (
    build_sandbox_selection_receipt,
    docker_candidate_from_observation,
    run_docker_sandbox_probe,
    windows_native_strict_candidate,
)
from .sandbox_verifier import (
    verify_docker_observation,
    verify_sandbox_selection_receipt,
)
from .user_task_evaluation import (
    load_synthetic_user_task_suite,
    run_synthetic_user_task_evaluation,
    verify_synthetic_user_task_evaluation,
)
from .validator_bridge import (
    load_general_science_validator_registry,
    run_general_science_validators,
    validate_validator_registry_semantics,
)

__all__ = [
    "ArchiveController",
    "AuditLedger",
    "AssuranceError",
    "ConversationNamespace",
    "DockerProcessTracker",
    "MemoryInstallationKeyStore",
    "WindowsDpapiInstallationKeyStore",
    "build_sandbox_selection_receipt",
    "build_guarded_frozen_context",
    "build_workspace_first_frozen_context",
    "authorize_action_candidate",
    "authorize_recovery_candidate",
    "create_security_envelope",
    "create_recovery_candidate",
    "docker_candidate_from_observation",
    "consume_sensitive_action_permit",
    "delegate_capabilities",
    "execute_guarded_no_model_action",
    "execute_workspace_first_integrated_run",
    "initialize_workspace_marker",
    "issue_sensitive_action_permit",
    "record_instruction_provenance",
    "review_general_science_bundle",
    "load_execution_backend_policy",
    "load_general_science_validator_registry",
    "load_profile_registry",
    "load_readonly_task_projection",
    "load_synthetic_user_task_suite",
    "project_complex_task_readonly",
    "recovery_permit_binding",
    "resolve_effective_profile",
    "resume_archived_conversation",
    "run_docker_sandbox_probe",
    "run_synthetic_user_task_evaluation",
    "run_general_science_validators",
    "select_execution_backend",
    "verify_archive",
    "verify_audit_seal",
    "verify_action_authorization",
    "verify_docker_observation",
    "verify_guarded_execution_receipt",
    "verify_capability_delegation",
    "verify_instruction_provenance",
    "verify_complex_task_readonly_projection",
    "verify_synthetic_user_task_evaluation",
    "verify_workspace_first_integrated_run",
    "verify_recovery_authorization",
    "verify_recovery_candidate",
    "verify_sandbox_selection_receipt",
    "verify_security_envelope",
    "verify_sensitive_action_permit",
    "validate_profile_registry_semantics",
    "validate_validator_registry_semantics",
    "windows_native_strict_candidate",
]
