from __future__ import annotations

from datetime import datetime
from typing import Any, Iterable

from .layout import PROTOCOL_ROOT
from .schema import validate_instance


TERMINAL_ACTION_STATES = {"succeeded", "failed", "cancelled", "unknown"}


def _parse_timestamp(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def _duplicates(values: Iterable[str]) -> set[str]:
    seen: set[str] = set()
    duplicates: set[str] = set()
    for value in values:
        if value in seen:
            duplicates.add(value)
        seen.add(value)
    return duplicates


def validate_session_record(record: dict[str, Any]) -> dict[str, Any]:
    """Validate the mechanically decidable v0.1 kernel invariants.

    JSON Schema validation runs first. This spike deliberately does not decide
    scientific sufficiency, task acceptance, or claim-strength eligibility.
    """

    validate_instance(
        record,
        PROTOCOL_ROOT / "agent-protocol-v0.1.schema.json",
        label="session record",
    )

    violations: list[dict[str, str]] = []

    def add(code: str, location: str, message: str) -> None:
        violations.append({"code": code, "location": location, "message": message})

    collections = {
        "sources": ("source_id", record["sources"]),
        "actions": ("action_id", record["actions"]),
        "artifacts": ("artifact_id", record["artifacts"]),
        "evidence": ("evidence_id", record["evidence"]),
        "gate_decisions": ("decision_id", record["gate_decisions"]),
        "case_retrievals": ("retrieval_id", record["case_retrievals"]),
        "claims": ("claim_id", record["claims"]),
    }
    ids_by_kind = {
        name: {item[id_field] for item in items}
        for name, (id_field, items) in collections.items()
    }
    all_record_ids: list[str] = []
    for name, (id_field, items) in collections.items():
        values = [item[id_field] for item in items]
        for duplicate in sorted(_duplicates(values)):
            add("ID-DUPLICATE-001", f"/{name}", f"duplicate ID: {duplicate}")
        all_record_ids.extend(values)
    for duplicate in sorted(_duplicates(all_record_ids)):
        add(
            "ID-AMBIGUOUS-001",
            "/",
            f"ID is reused across record collections: {duplicate}",
        )

    source_ids = ids_by_kind["sources"]
    action_ids = ids_by_kind["actions"]
    artifact_ids = ids_by_kind["artifacts"]
    evidence_ids = ids_by_kind["evidence"]
    claim_ids = ids_by_kind["claims"]
    subject_ids = set(all_record_ids)
    subject_ids.update(
        {
            record["session"]["session_id"],
            record["task_contract"]["task_id"],
        }
    )
    precommitment = record.get("reasoning_precommitment")
    if precommitment:
        subject_ids.add(precommitment["precommitment_id"])

    def require_refs(
        values: Iterable[str], known: set[str], location: str, code: str
    ) -> None:
        for value in values:
            if value not in known:
                add(code, location, f"unknown reference: {value}")

    require_refs(
        record["task_contract"]["source_of_truth"],
        source_ids,
        "/task_contract/source_of_truth",
        "ROUTE-SOURCE-001",
    )

    source_by_id = {item["source_id"]: item for item in record["sources"]}
    for source_id, source in source_by_id.items():
        if (
            source["epistemic_role"] == "source_grounded"
            and source["usage"] not in {"read", "cited"}
        ):
            add(
                "ROUTE-SOURCE-001",
                f"/sources/{source_id}",
                "source_grounded requires usage read or cited",
            )

    if precommitment:
        for index, fact in enumerate(precommitment["minimum_facts"]):
            require_refs(
                fact["source_ids"],
                source_ids,
                f"/reasoning_precommitment/minimum_facts/{index}/source_ids",
                "ROUTE-SOURCE-001",
            )
        for index, hypothesis in enumerate(precommitment["candidate_hypotheses"]):
            require_refs(
                hypothesis.get("source_ids", []),
                source_ids,
                f"/reasoning_precommitment/candidate_hypotheses/{index}/source_ids",
                "ROUTE-SOURCE-001",
            )

    event_ids: list[str] = []
    event_sequences: list[int] = []
    for action_index, action in enumerate(record["actions"]):
        events = action["events"]
        terminal_events = [event for event in events if event["event_kind"] == "terminal"]
        event_ids.extend(event["event_id"] for event in events)
        event_sequences.extend(event["sequence"] for event in events)
        for event_index, event in enumerate(events):
            location = f"/actions/{action_index}/events/{event_index}"
            if event["action_id"] != action["action_id"]:
                add("PROC-EVENT-ACTION-001", location, "event action_id differs from parent action")
            if event["event_kind"] == "terminal" and event.get("terminal_state") is None:
                add("PROC-TERMINAL-001", location, "terminal event lacks terminal_state")
            if event["event_kind"] == "progress" and event.get("terminal_state") is not None:
                add("PROC-TERMINAL-001", location, "progress event carries terminal_state")
            require_refs(
                event.get("produced_artifact_ids", []),
                artifact_ids,
                f"{location}/produced_artifact_ids",
                "ART-STALE-001",
            )
        if action["state"] in TERMINAL_ACTION_STATES:
            if len(terminal_events) != 1:
                add(
                    "PROC-TERMINAL-001",
                    f"/actions/{action_index}/events",
                    f"terminal action requires exactly one terminal event; found {len(terminal_events)}",
                )
            elif terminal_events[0].get("terminal_state") != action["state"]:
                add(
                    "PROC-TERMINAL-001",
                    f"/actions/{action_index}",
                    "terminal event state differs from action state",
                )
            if terminal_events and terminal_events[0]["sequence"] != max(
                event["sequence"] for event in events
            ):
                add(
                    "PROC-TERMINAL-001",
                    f"/actions/{action_index}/events",
                    "terminal event is not the final event for the action",
                )
        elif terminal_events:
            add(
                "PROC-TERMINAL-001",
                f"/actions/{action_index}/events",
                "non-terminal action already has a terminal event",
            )

    for duplicate in sorted(_duplicates(event_ids)):
        add("ID-DUPLICATE-001", "/actions/*/events", f"duplicate event ID: {duplicate}")
    for duplicate in sorted(_duplicates(str(value) for value in event_sequences)):
        add(
            "PROC-SEQUENCE-001",
            "/actions/*/events",
            f"duplicate event sequence: {duplicate}",
        )
    for action_index, action in enumerate(record["actions"]):
        sequences = [event["sequence"] for event in action["events"]]
        if sequences != sorted(sequences):
            add(
                "PROC-SEQUENCE-001",
                f"/actions/{action_index}/events",
                "event sequences are not monotonically increasing",
            )

    for index, artifact in enumerate(record["artifacts"]):
        require_refs(
            [artifact["producer_action_id"]],
            action_ids,
            f"/artifacts/{index}/producer_action_id",
            "ART-STALE-001",
        )
        if artifact.get("manifest_artifact_id") is not None:
            require_refs(
                [artifact["manifest_artifact_id"]],
                artifact_ids,
                f"/artifacts/{index}/manifest_artifact_id",
                "ART-STALE-001",
            )

    for index, evidence in enumerate(record["evidence"]):
        require_refs(
            evidence["artifact_ids"], artifact_ids, f"/evidence/{index}/artifact_ids", "ART-STALE-001"
        )
        require_refs(
            evidence["claim_ids"], claim_ids, f"/evidence/{index}/claim_ids", "CLM-STRENGTH-001"
        )
        require_refs(
            evidence["source_ids"], source_ids, f"/evidence/{index}/source_ids", "ROUTE-SOURCE-001"
        )

    for index, claim in enumerate(record["claims"]):
        require_refs(
            claim["supporting_evidence_ids"],
            evidence_ids,
            f"/claims/{index}/supporting_evidence_ids",
            "CLM-STRENGTH-001",
        )
        require_refs(
            claim["refuting_evidence_ids"],
            evidence_ids,
            f"/claims/{index}/refuting_evidence_ids",
            "CLM-STRENGTH-001",
        )
        require_refs(
            claim["prior_existence_scan"]["source_ids"],
            source_ids,
            f"/claims/{index}/prior_existence_scan/source_ids",
            "ROUTE-PRIOR-EXISTENCE-001",
        )

    for index, decision in enumerate(record["gate_decisions"]):
        require_refs(
            decision["subject_refs"],
            subject_ids,
            f"/gate_decisions/{index}/subject_refs",
            "GATE-SUBJECT-001",
        )
        require_refs(
            decision["source_ids"],
            source_ids,
            f"/gate_decisions/{index}/source_ids",
            "ROUTE-SOURCE-001",
        )

    if record["case_retrievals"] and not precommitment:
        add(
            "BIAS-PRECOMMIT-001",
            "/case_retrievals",
            "case retrieval exists without a reasoning precommitment",
        )
    if precommitment:
        precommitment_time = _parse_timestamp(precommitment["created_at"])
        for index, retrieval in enumerate(record["case_retrievals"]):
            if not retrieval["performed_after_precommitment"]:
                add(
                    "BIAS-PRECOMMIT-001",
                    f"/case_retrievals/{index}",
                    "retrieval is marked as preceding precommitment",
                )
            if _parse_timestamp(retrieval["performed_at"]) <= precommitment_time:
                add(
                    "BIAS-PRECOMMIT-001",
                    f"/case_retrievals/{index}/performed_at",
                    "retrieval timestamp is not after precommitment",
                )

    violations.sort(key=lambda item: (item["location"], item["code"], item["message"]))
    return {
        "schema_version": "0.1.0-prototype",
        "valid": not violations,
        "violation_count": len(violations),
        "violations": violations,
        "limitations": [
            "This spike checks mechanically decidable session invariants only.",
            "It does not establish task acceptance, scientific validity, evidence sufficiency, or claim eligibility.",
        ],
    }
