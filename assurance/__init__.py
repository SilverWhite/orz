"""Runtime-neutral General Assurance Kernel development fixtures.

P1-P5 provide testable contracts, lifecycle controls, guarded execution,
instruction/capability authorization, retained metadata audit/recovery gates,
a workspace-first integrated path, and synthetic user-task preflight.
GSA-CORE adds a narrow domain-neutral multi-file read-only review. They remain
development/conformance mechanisms rather than a production security boundary.
"""

from .adapter_gate import (
    AdapterGateBlockedError,
    AdapterGateContext,
    enforce_adapter_call,
    run_adapter_gate_bypass_fixture,
    verify_adapter_gate_enforcement,
)
from .adapter_failure_classifier import (
    build_failure_recovery_plan,
    classify_adapter_error,
)
from .adapter_output_validator import validate_adapter_output
from .adapter_preflight import run_adapter_preflight, verify_adapter_preflight
from .archive import ArchiveController, resume_archived_conversation
from .archive_journal import (
    ArchiveJournalWriter,
    cleanup_stale_archive_lock,
    detect_stale_archive_lock,
    inspect_archive_journal,
    recover_archive_journal,
    replay_archive_journal,
)
from .archive_recovery import classify_archive_failure, recover_archive
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
from .child_capability_enforcer import (
    ChildCapabilityEscalationError,
    enforce_child_capabilities,
    spawn_child_context,
    verify_child_capability_enforcement,
)
from .conversation import ConversationNamespace
from .credential_scrub import (
    CredentialGuard,
    assert_no_credential_in_dict,
    assert_safe_container_mount,
    audit_child_environment,
    audit_container_mount,
    audit_credential_scrub_sites,
    get_scrub_audit,
    sanitize_child_environment,
    scan_dict_for_credentials,
)
from .deepseek_adapter import (
    build_real_deepseek_answer_packet,
    build_real_deepseek_context,
    call_deepseek_api,
)
from .endpoint_canonicalizer import (
    canonicalize_filesystem_path,
    canonicalize_network_endpoint,
    validate_endpoint_list,
    validate_filesystem_targets,
)
from .deepseek_api_observation import (
    run_deepseek_api_observation_pipeline,
    verify_deepseek_api_observation_pipeline,
)
from .deepseek_stream_observation import (
    run_deepseek_stream_observation_fixture,
    verify_deepseek_stream_observation_fixture,
)
from .browser_retrieval import (
    BrowserCDPClient,
    PageContent,
    PageLink,
    TabInfo,
)
from .retrieval_workflow import (
    RetrievalProgress,
    RetrievalResult,
    WebPageResult,
    WebRetrievalResult,
    retrieve_search,
    retrieve_urls,
    run_retrieval,
)
from .evidence_store import (
    EVIDENCE_ROOT,
    document_exists,
    get_document_path,
    get_original_pdf_path,
    list_documents,
    read_metadata,
    read_pages_jsonl,
    store_pdf,
    store_source_record,
)
from .pdf_evidence import (
    PageEntry,
    PageIndex,
    PdfValidation,
    build_page_index,
    extract_text,
    find_in_pages,
    guess_version,
    read_pages,
    validate_pdf,
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
from .envelope import create_security_envelope, migrate_envelope, verify_security_envelope
from .errors import AssuranceError
from .guarded_execution import (
    DockerProcessTracker,
    build_guarded_frozen_context,
    execute_guarded_no_model_action,
    verify_guarded_execution_receipt,
)
from .general_science_review import review_general_science_bundle
from .key_lifecycle import KeyLifecycleController, verify_key_history
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
    detect_obfuscated_injection,
    evaluate_instruction_provenance_gate,
    evaluate_instruction_provenance_gate_with_canonicalizer,
    parse_instruction_content,
    run_instruction_provenance_gate_fixture,
    validate_all_entry_points_consume_same_gate,
    verify_instruction_provenance_gate_fixture,
    verify_instruction_provenance_gate_receipt,
)
from .network_permit_gateway import (
    NetworkPermitBlockedError,
    build_network_permit_policy,
    evaluate_network_permit,
    verify_network_permit_receipt,
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
    verify_profile_registry_completeness,
)
from .readonly_projection import (
    load_readonly_task_projection,
    project_complex_task_readonly,
    verify_complex_task_readonly_projection,
)
from .project_doc_index import ProjectDocIndex
from .retrieval_subagent import (
    build_fake_retrieval_result,
    build_retrieval_session_close_receipt,
    build_retrieval_task_contract,
    DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET,
    dispatch_external_retrieval_subagent,
    dispatch_retrieval_subagent,
    dispatch_retrieval_subagent_online,
    DEFAULT_INTERNAL_RETRIEVAL_CREDENTIAL_TARGET,
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
from .runner_scoring_handoff import (
    build_runner_action_manifest,
    build_scoring_input,
    compute_claim_boundary_score,
    compute_deterministic_scores,
    compute_gate_scores,
    compute_integrity_score,
    validate_action_dag,
)
from .recovery import (
    authorize_recovery_candidate,
    create_recovery_candidate,
    recovery_permit_binding,
    verify_recovery_authorization,
    verify_recovery_candidate,
)
from .session_governor import SessionGovernor
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
from .storage_adapter import LocalStorageAdapter, StorageAdapter
from .windows_sandbox import (
    run_windows_native_sandbox_probe,
    windows_native_candidate_from_observation,
)
from .workspace_trust import (
    establish_workspace_trust,
    validate_all_entry_points_establish_trust,
    verify_workspace_trust,
    workspace_trust_for_adapter,
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
    "AdapterGateBlockedError",
    "AdapterGateContext",
    "ArchiveController",
    "ArchiveJournalWriter",
    "AuditLedger",
    "AssuranceError",
    "ConversationNamespace",
    "CredentialGuard",
    "DockerProcessTracker",
    "MemoryInstallationKeyStore",
    "WindowsDpapiInstallationKeyStore",
    "build_sandbox_selection_receipt",
    "build_guarded_frozen_context",
    "build_instruction_provenance_gate_context",
    "build_disposable_reproduction_run_proof",
    "build_disposable_reproduction_execution_lock",
    "build_canonical_cli_run_manifest",
    "build_failure_recovery_plan",
    "canonicalize_filesystem_path",
    "canonicalize_network_endpoint",
    "classify_adapter_error",
    "compute_claim_boundary_score",
    "compute_deterministic_scores",
    "compute_gate_scores",
    "compute_integrity_score",
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
    "assert_no_credential_in_dict",
    "assert_safe_container_mount",
    "audit_child_environment",
    "audit_container_mount",
    "audit_credential_scrub_sites",
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
    "enforce_adapter_call",
    "evaluate_instruction_provenance_gate",
    "evaluate_instruction_provenance_gate_with_canonicalizer",
    "evaluate_runtime_stagnation_guard",
    "evaluate_source_visibility_gate",
    "evaluate_source_visibility_ledger_file",
    "evaluate_tool_belief_mismatch",
    "evaluate_tool_belief_stagnation",
    "initialize_workspace_marker",
    "index_artifact_schemas",
    "inspect_archive_journal",
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
    "LocalStorageAdapter",
    "parse_instruction_content",
    "project_complex_task_readonly",
    "ProjectDocIndex",
    "probe_tool_availability",
    "classify_archive_failure",
    "cleanup_stale_archive_lock",
    "detect_obfuscated_injection",
    "detect_stale_archive_lock",
    "DEFAULT_EXTERNAL_RETRIEVAL_CREDENTIAL_TARGET",
    "dispatch_external_retrieval_subagent",
    "dispatch_retrieval_subagent",
    "dispatch_retrieval_subagent_online",
    "DEFAULT_INTERNAL_RETRIEVAL_CREDENTIAL_TARGET",
    "get_scrub_audit",
    "recover_archive",
    "recover_archive_journal",
    "recovery_permit_binding",
    "replay_archive_journal",
    "resolve_effective_profile",
    "resume_archived_conversation",
    "render_visibility_summary",
    "run_disposable_reproduction",
    "run_adapter_gate_bypass_fixture",
    "run_adapter_preflight",
    "run_canonical_guarded_cli",
    "run_canonical_guarded_cli_real",
    "build_real_deepseek_answer_packet",
    "build_real_deepseek_context",
    "build_runner_action_manifest",
    "build_scoring_input",
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
    "sanitize_child_environment",
    "scan_dict_for_credentials",
    "select_execution_backend",
    "SessionGovernor",
    "StorageAdapter",
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
    "verify_profile_registry_completeness",
    "validate_artifact_registry_semantics",
    "validate_endpoint_list",
    "validate_filesystem_targets",
    "validate_retrieval_result",
    "validate_validator_registry_semantics",
    "validate_action_dag",
    "validate_adapter_output",
    "validate_all_entry_points_consume_same_gate",
    "verify_adapter_gate_enforcement",
    "verify_adapter_preflight",
    "windows_native_candidate_from_observation",
    "windows_native_strict_candidate",
]
