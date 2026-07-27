"""Runtime-neutral General Assurance Kernel development fixtures.

P1-P5 provide testable contracts, lifecycle controls, guarded execution,
instruction/capability authorization, retained metadata audit/recovery gates,
a workspace-first integrated path, and synthetic user-task preflight.
GSA-CORE adds a narrow domain-neutral multi-file read-only review. They remain
development/conformance mechanisms rather than a production security boundary.
"""

from .archive import ArchiveController, resume_archived_conversation
from .archive_verifier import verify_archive
from .artifact_registry import (
    index_artifact_schemas,
    load_general_science_artifact_registry,
    validate_artifact_registry_semantics,
)
from .audit import AuditLedger, verify_audit_seal
from .canonical_cli import (
    build_canonical_cli_run_manifest,
    build_fake_answer_packet,
    run_canonical_guarded_cli,
    run_canonical_guarded_cli_real,
    verify_canonical_guarded_cli_run,
)
from .conversation import ConversationNamespace
from .deepseek_adapter import (
    build_real_deepseek_answer_packet,
    build_real_deepseek_context,
    call_deepseek_api,
)
from .deepseek_api_observation import (
    run_deepseek_api_observation_pipeline,
    verify_deepseek_api_observation_pipeline,
)
from .deepseek_stream_observation import (
    run_deepseek_stream_observation_fixture,
    verify_deepseek_stream_observation_fixture,
)
from .disposable_reproduction import (
    build_disposable_reproduction_run_proof,
    run_disposable_reproduction,
    verify_disposable_reproduction_receipt,
    verify_disposable_reproduction_run_proof,
)
from .execution_lock import (
    build_disposable_reproduction_execution_lock,
    verify_disposable_reproduction_execution_lock,
)
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
from .orientation_runtime_guard import (
    build_orientation_checkpoint,
    build_tool_availability_infused_orientation_block,
    build_tool_availability_infused_orientation_checkpoint,
    evaluate_runtime_stagnation_guard,
    evaluate_tool_belief_stagnation,
    verify_orientation_response,
)
from .orientation_runtime_integration import (
    run_orientation_stagnation_integration_fixture,
    verify_orientation_stagnation_integration_fixture,
)
from .orientation_runtime_journal import (
    verify_orientation_stagnation_runtime_journal,
    write_orientation_stagnation_runtime_journal,
)
from .instruction_gate import (
    authorize_action_candidate,
    delegate_capabilities,
    record_instruction_provenance,
    verify_action_authorization,
    verify_capability_delegation,
    verify_instruction_provenance,
)
from .instruction_provenance_gate import (
    build_instruction_provenance_gate_context,
    evaluate_instruction_provenance_gate,
    run_instruction_provenance_gate_fixture,
    verify_instruction_provenance_gate_fixture,
    verify_instruction_provenance_gate_receipt,
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
from .retrieval_subagent import (
    build_fake_retrieval_result,
    build_retrieval_session_close_receipt,
    build_retrieval_task_contract,
    run_retrieval_subagent_fixture,
    validate_retrieval_result,
    verify_retrieval_result_sources,
    verify_retrieval_subagent_fixture,
)
from .runtime_preflight import (
    build_gsa_runtime_preflight_projection,
    verify_gsa_runtime_preflight_journal,
    verify_gsa_runtime_preflight_projection,
    write_gsa_runtime_preflight_journal,
)
from .runner_public_output import (
    extract_public_outputs_from_runner_stream,
    run_runner_public_output_extraction_fixture,
    verify_runner_public_output_extraction_fixture,
)
from .source_visibility import (
    evaluate_source_visibility_gate,
    evaluate_source_visibility_ledger_file,
    render_visibility_summary,
)
from .task_contract import (
    build_task_contract_from_ask,
    load_task_contract,
)
from .tool_availability_gate import (
    build_tool_availability_context_block,
    build_tool_availability_gate_receipt,
    evaluate_tool_belief_mismatch,
    probe_tool_availability,
)
from .runner import (
    inspect_gsa_runner_journal_recovery,
    replay_gsa_runner_journal,
    repair_gsa_runner_journal,
    resume_gsa_no_model_runner_skeleton,
    run_gsa_no_model_runner_skeleton,
    verify_gsa_no_model_runner_skeleton,
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
    verify_windows_native_observation,
)
from .windows_sandbox import (
    run_windows_native_sandbox_probe,
    windows_native_candidate_from_observation,
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
    "build_instruction_provenance_gate_context",
    "build_disposable_reproduction_run_proof",
    "build_disposable_reproduction_execution_lock",
    "build_canonical_cli_run_manifest",
    "build_fake_answer_packet",
    "build_fake_retrieval_result",
    "build_orientation_checkpoint",
    "build_tool_availability_infused_orientation_block",
    "build_tool_availability_infused_orientation_checkpoint",
    "build_retrieval_session_close_receipt",
    "build_retrieval_task_contract",
    "build_task_contract_from_ask",
    "build_gsa_runtime_preflight_projection",
    "build_workspace_first_frozen_context",
    "build_tool_availability_context_block",
    "build_tool_availability_gate_receipt",
    "authorize_action_candidate",
    "authorize_recovery_candidate",
    "create_security_envelope",
    "create_recovery_candidate",
    "docker_candidate_from_observation",
    "consume_sensitive_action_permit",
    "delegate_capabilities",
    "execute_guarded_no_model_action",
    "execute_workspace_first_integrated_run",
    "extract_public_outputs_from_runner_stream",
    "evaluate_instruction_provenance_gate",
    "evaluate_runtime_stagnation_guard",
    "evaluate_source_visibility_gate",
    "evaluate_source_visibility_ledger_file",
    "evaluate_tool_belief_mismatch",
    "evaluate_tool_belief_stagnation",
    "initialize_workspace_marker",
    "index_artifact_schemas",
    "inspect_gsa_runner_journal_recovery",
    "issue_sensitive_action_permit",
    "record_instruction_provenance",
    "replay_gsa_runner_journal",
    "repair_gsa_runner_journal",
    "resume_gsa_no_model_runner_skeleton",
    "review_general_science_bundle",
    "load_execution_backend_policy",
    "load_general_science_artifact_registry",
    "load_general_science_validator_registry",
    "load_profile_registry",
    "load_readonly_task_projection",
    "load_task_contract",
    "load_synthetic_user_task_suite",
    "project_complex_task_readonly",
    "probe_tool_availability",
    "recovery_permit_binding",
    "resolve_effective_profile",
    "resume_archived_conversation",
    "render_visibility_summary",
    "run_disposable_reproduction",
    "run_canonical_guarded_cli",
    "run_canonical_guarded_cli_real",
    "build_real_deepseek_answer_packet",
    "build_real_deepseek_context",
    "call_deepseek_api",
    "run_deepseek_api_observation_pipeline",
    "run_deepseek_stream_observation_fixture",
    "run_docker_sandbox_probe",
    "run_gsa_no_model_runner_skeleton",
    "run_instruction_provenance_gate_fixture",
    "run_orientation_stagnation_integration_fixture",
    "run_retrieval_subagent_fixture",
    "run_runner_public_output_extraction_fixture",
    "run_synthetic_user_task_evaluation",
    "run_windows_native_sandbox_probe",
    "run_general_science_validators",
    "select_execution_backend",
    "verify_archive",
    "verify_audit_seal",
    "verify_action_authorization",
    "verify_docker_observation",
    "verify_windows_native_observation",
    "verify_guarded_execution_receipt",
    "verify_instruction_provenance_gate_fixture",
    "verify_instruction_provenance_gate_receipt",
    "verify_capability_delegation",
    "verify_instruction_provenance",
    "verify_orientation_response",
    "verify_orientation_stagnation_integration_fixture",
    "verify_orientation_stagnation_runtime_journal",
    "verify_retrieval_result_sources",
    "verify_retrieval_subagent_fixture",
    "verify_runner_public_output_extraction_fixture",
    "verify_complex_task_readonly_projection",
    "verify_disposable_reproduction_receipt",
    "verify_canonical_guarded_cli_run",
    "verify_deepseek_api_observation_pipeline",
    "verify_deepseek_stream_observation_fixture",
    "verify_disposable_reproduction_execution_lock",
    "verify_disposable_reproduction_run_proof",
    "verify_gsa_runtime_preflight_projection",
    "verify_gsa_runtime_preflight_journal",
    "verify_gsa_no_model_runner_skeleton",
    "verify_synthetic_user_task_evaluation",
    "verify_workspace_first_integrated_run",
    "verify_recovery_authorization",
    "verify_recovery_candidate",
    "verify_sandbox_selection_receipt",
    "verify_security_envelope",
    "verify_sensitive_action_permit",
    "write_gsa_runtime_preflight_journal",
    "write_orientation_stagnation_runtime_journal",
    "validate_profile_registry_semantics",
    "validate_artifact_registry_semantics",
    "validate_retrieval_result",
    "validate_validator_registry_semantics",
    "windows_native_candidate_from_observation",
    "windows_native_strict_candidate",
]
