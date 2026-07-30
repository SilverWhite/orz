from __future__ import annotations

from collections import Counter
from copy import deepcopy
from datetime import datetime, timezone
import json
import re
from typing import Any
import uuid

from .archive_verifier import verify_archive
from .contracts import validate_contract
from .conversation import ConversationNamespace
from .envelope import verify_security_envelope
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_json,
    canonical_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
)


SOURCE_KINDS = {
    "acp",
    "session",
    "provider",
    "supervisor",
    "tool",
    "runtime",
    "kernel",
}
CORE_SOURCE_KINDS = {"acp", "session", "provider", "supervisor"}
COMPLETENESS_STATES = {"complete", "partial", "unknown"}
PROVENANCE_STATES = {"direct", "derived_unverified", "unknown"}
RAW_RETENTION_CATEGORIES = {
    "raw_provider_payload",
    "private_reasoning",
    "raw_tool_result",
    "full_stdout_stderr",
    "network_body",
}
TERMINAL_OUTCOMES = {"succeeded", "failed", "cancelled", "unknown"}
SAFE_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
SAFE_TYPE = re.compile(r"^[a-z][a-z0-9_.:-]{0,127}$")
SAFE_FACT_NAME = re.compile(r"^[a-z][a-z0-9_]{0,63}$")
SAFE_FACT_STRING = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$")
SAFE_SOURCE_REF = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{0,255}$")
SAFE_REASON = re.compile(r"^[A-Z][A-Z0-9._-]{1,127}$")
FORBIDDEN_FACT_FRAGMENTS = {
    "authorization",
    "body",
    "content",
    "credential",
    "data",
    "input",
    "message",
    "output",
    "prompt",
    "reasoning",
    "secret",
    "token",
}


def _timestamp(value: datetime) -> str:
    return value.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _validate_timestamp(value: str | None) -> None:
    if value is None:
        return
    try:
        datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (TypeError, ValueError) as exc:
        raise AssuranceError("audit event timestamp is invalid") from exc


def _sign(
    body: dict[str, Any], *, key_store: InstallationKeyStore
) -> dict[str, Any]:
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": key_store.key_id,
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": key_store.sign(payload),
        },
    }


def _verify_signature(
    receipt: dict[str, Any], *, key_store: InstallationKeyStore
) -> list[str]:
    errors: list[str] = []
    integrity = receipt["integrity"]
    body = {key: value for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    if integrity["key_id"] != key_store.key_id:
        errors.append("audit seal key mismatch")
    if integrity["signed_payload_sha256"] != sha256_bytes(payload):
        errors.append("audit seal signed payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        errors.append("audit seal signature verification failed")
    return errors


def _normalize_correlation(
    value: dict[str, str | None] | None,
) -> dict[str, str | None]:
    selected = value or {
        "turn_id": None,
        "tool_call_id": None,
        "request_id": None,
    }
    if set(selected) != {"turn_id", "tool_call_id", "request_id"}:
        raise AssuranceError("audit correlation has an invalid field set")
    for item in selected.values():
        if item is not None and (
            not isinstance(item, str) or not SAFE_ID.fullmatch(item)
        ):
            raise AssuranceError("audit correlation ID is invalid")
    return deepcopy(selected)


def _normalize_facts(value: dict[str, Any] | None) -> dict[str, Any]:
    selected = value or {}
    if not isinstance(selected, dict):
        raise AssuranceError("audit facts must be an object")
    normalized: dict[str, Any] = {}
    for key, item in selected.items():
        if not isinstance(key, str) or not SAFE_FACT_NAME.fullmatch(key):
            raise AssuranceError("audit fact name is invalid")
        if any(fragment in key for fragment in FORBIDDEN_FACT_FRAGMENTS):
            raise AssuranceError(
                "audit fact name could carry content-bearing data"
            )
        if isinstance(item, bool) or item is None:
            normalized[key] = item
        elif isinstance(item, int) and not isinstance(item, bool):
            normalized[key] = item
        elif isinstance(item, str) and SAFE_FACT_STRING.fullmatch(item):
            normalized[key] = item
        else:
            raise AssuranceError(
                "audit facts accept only metadata-safe booleans, integers, "
                "nulls, identifiers, and status tokens"
            )
    return normalized


def _normalize_ranges(
    value: dict[str, list[dict[str, Any]]],
) -> dict[str, list[dict[str, Any]]]:
    if set(value) != {"retained", "discarded", "unknown"}:
        raise AssuranceError("compaction ranges have an invalid field set")
    normalized: dict[str, list[dict[str, Any]]] = {}
    classified_indexes: set[int] = set()
    for disposition in ("retained", "discarded", "unknown"):
        ranges = value[disposition]
        if not isinstance(ranges, list):
            raise AssuranceError("compaction ranges must be arrays")
        normalized[disposition] = []
        for item in ranges:
            if not isinstance(item, dict) or set(item) != {
                "first_index",
                "last_index",
                "reason_code",
            }:
                raise AssuranceError("compaction range has an invalid field set")
            first = item["first_index"]
            last = item["last_index"]
            reason = item["reason_code"]
            if (
                not isinstance(first, int)
                or isinstance(first, bool)
                or not isinstance(last, int)
                or isinstance(last, bool)
                or first < 0
                or last < first
                or not isinstance(reason, str)
                or not SAFE_REASON.fullmatch(reason)
            ):
                raise AssuranceError("compaction range is invalid")
            indexes = set(range(first, last + 1))
            if classified_indexes & indexes:
                raise AssuranceError(
                    "compaction retained/discarded/unknown ranges overlap"
                )
            classified_indexes.update(indexes)
            normalized[disposition].append(deepcopy(item))
    if not any(normalized.values()):
        raise AssuranceError(
            "compaction must explicitly classify at least one source range"
        )
    return normalized


def _normalize_compaction(
    value: dict[str, Any] | None,
) -> dict[str, Any] | None:
    if value is None:
        return None
    if not isinstance(value, dict) or set(value) != {
        "summary_status",
        "summary_sha256",
        "source_span_sha256",
        "ranges",
    }:
        raise AssuranceError("compaction metadata has an invalid field set")
    if value["summary_status"] != "derived_unverified":
        raise AssuranceError("compaction summary must remain derived_unverified")
    for field in ("summary_sha256", "source_span_sha256"):
        item = value[field]
        if (
            not isinstance(item, str)
            or not re.fullmatch(r"[a-f0-9]{64}", item)
        ):
            raise AssuranceError(f"compaction {field} is invalid")
    return {
        "summary_status": "derived_unverified",
        "summary_sha256": value["summary_sha256"],
        "source_span_sha256": value["source_span_sha256"],
        "ranges": _normalize_ranges(value["ranges"]),
    }


def _event_hash(event: dict[str, Any]) -> str:
    body = {key: value for key, value in event.items() if key != "event_sha256"}
    return sha256_bytes(canonical_bytes(body))


class AuditLedger:
    """Build a retained metadata-only ledger from ephemeral source payloads."""

    def __init__(
        self,
        *,
        namespace: ConversationNamespace,
        key_store: InstallationKeyStore,
    ) -> None:
        self.namespace = namespace
        self.key_store = key_store
        self.envelope = namespace.load_active_envelope()
        verify_security_envelope(self.envelope, key_store=key_store)
        self.ledger_id = f"AUDIT-{uuid.uuid4().hex.upper()}"
        self.events: list[dict[str, Any]] = []
        self._source_sequences: dict[tuple[str, str], int] = {}
        self._sealed = False

    def append(
        self,
        *,
        source_kind: str,
        source_id: str,
        source_sequence: int,
        source_completeness: str,
        canonical_event_type: str,
        raw_payload: bytes,
        raw_retention_category: str,
        provenance_status: str,
        source_refs: list[str],
        occurred_at: str | None = None,
        correlation: dict[str, str | None] | None = None,
        facts: dict[str, Any] | None = None,
        compaction: dict[str, Any] | None = None,
        terminal_outcome: str | None = None,
    ) -> dict[str, Any]:
        if self._sealed:
            raise AssuranceError("cannot append to a sealed audit ledger")
        if source_kind not in SOURCE_KINDS:
            raise AssuranceError("audit source kind is invalid")
        if not isinstance(source_id, str) or not SAFE_ID.fullmatch(source_id):
            raise AssuranceError("audit source ID is invalid")
        key = (source_kind, source_id)
        expected_sequence = self._source_sequences.get(key, 0)
        if source_sequence != expected_sequence:
            raise AssuranceError(
                "audit source sequence must be contiguous per source"
            )
        if source_completeness not in COMPLETENESS_STATES:
            raise AssuranceError("audit source completeness is invalid")
        if (
            not isinstance(canonical_event_type, str)
            or not SAFE_TYPE.fullmatch(canonical_event_type)
        ):
            raise AssuranceError("canonical audit event type is invalid")
        if not isinstance(raw_payload, bytes):
            raise AssuranceError("audit source payload must be bytes")
        if raw_retention_category not in RAW_RETENTION_CATEGORIES:
            raise AssuranceError(
                "audit raw payload must use an archive-deleted category"
            )
        if provenance_status not in PROVENANCE_STATES:
            raise AssuranceError("audit provenance status is invalid")
        if (
            not isinstance(source_refs, list)
            or not source_refs
            or len(source_refs) != len(set(source_refs))
            or any(
                not isinstance(item, str)
                or not SAFE_SOURCE_REF.fullmatch(item)
                for item in source_refs
            )
        ):
            raise AssuranceError("audit source references are invalid")
        _validate_timestamp(occurred_at)
        normalized_compaction = _normalize_compaction(compaction)
        if (
            normalized_compaction is not None
            and provenance_status != "derived_unverified"
        ):
            raise AssuranceError(
                "compaction events must use derived_unverified provenance"
            )
        if terminal_outcome is not None and terminal_outcome not in TERMINAL_OUTCOMES:
            raise AssuranceError("audit terminal outcome is invalid")

        sequence = len(self.events)
        event_id = f"AEV-{sequence:06d}"
        self.namespace.write_artifact(
            raw_retention_category,
            f"audit/{self.ledger_id}/{event_id}.bin",
            raw_payload,
        )
        body = {
            "schema_version": "0.1.0-draft",
            "ledger_id": self.ledger_id,
            "conversation_id": self.namespace.conversation_id,
            "envelope_id": self.envelope["envelope_id"],
            "sequence": sequence,
            "event_id": event_id,
            "canonical_event_type": canonical_event_type,
            "occurred_at": occurred_at,
            "source": {
                "kind": source_kind,
                "source_id": source_id,
                "source_sequence": source_sequence,
                "completeness": source_completeness,
            },
            "correlation": _normalize_correlation(correlation),
            "facts": _normalize_facts(facts),
            "payload": {
                "sha256": sha256_bytes(raw_payload),
                "bytes": len(raw_payload),
                "raw_payload_embedded": False,
                "retention_category": raw_retention_category,
            },
            "provenance": {
                "status": provenance_status,
                "source_refs": sorted(source_refs),
            },
            "compaction": normalized_compaction,
            "terminal": {
                "is_terminal": terminal_outcome is not None,
                "outcome": terminal_outcome or "nonterminal",
            },
            "previous_event_sha256": (
                self.events[-1]["event_sha256"] if self.events else None
            ),
        }
        event = {**body, "event_sha256": sha256_bytes(canonical_bytes(body))}
        validate_contract(
            event,
            "audit-event-v0.1.schema.json",
            label="metadata audit event",
        )
        self.events.append(event)
        self._source_sequences[key] = expected_sequence + 1
        return deepcopy(event)

    def seal(
        self,
        *,
        source_completeness: dict[str, str],
        now: datetime | None = None,
        shadow_refs: dict[str, str] | None = None,
    ) -> dict[str, Any]:
        if self._sealed:
            raise AssuranceError("audit ledger is already sealed")
        if not self.events:
            raise AssuranceError("cannot seal an empty audit ledger")
        if (
            not isinstance(source_completeness, dict)
            or not CORE_SOURCE_KINDS <= set(source_completeness)
            or not set(source_completeness) <= SOURCE_KINDS
            or any(
                value not in COMPLETENESS_STATES
                for value in source_completeness.values()
            )
        ):
            raise AssuranceError(
                "audit seal must declare ACP/session/provider/supervisor completeness"
            )
        terminals = [item for item in self.events if item["terminal"]["is_terminal"]]
        if len(terminals) != 1 or terminals[0] is not self.events[-1]:
            raise AssuranceError(
                "audit ledger requires exactly one final terminal event"
            )
        by_source = Counter(item["source"]["kind"] for item in self.events)
        for kind, declared in source_completeness.items():
            observed_states = {
                item["source"]["completeness"]
                for item in self.events
                if item["source"]["kind"] == kind
            }
            if observed_states and observed_states != {declared}:
                raise AssuranceError(
                    f"audit source completeness mismatch for {kind}"
                )
            if by_source.get(kind, 0) == 0 and declared != "unknown":
                raise AssuranceError(
                    f"unobserved audit source {kind} must remain unknown"
                )

        journal_bytes = b"".join(
            canonical_bytes(item) + b"\n" for item in self.events
        )
        relative_name = f"audit/{self.ledger_id}.events.jsonl"
        journal_path = self.namespace.write_artifact(
            "redacted_conversation",
            relative_name,
            journal_bytes,
        )
        sources = [
            {
                "kind": kind,
                "declared_completeness": source_completeness[kind],
                "observed_event_count": by_source.get(kind, 0),
            }
            for kind in sorted(source_completeness)
        ]
        compaction_events = [
            item for item in self.events if item["compaction"] is not None
        ]
        selected_now = now or datetime.now(timezone.utc)
        body = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "metadata_audit_seal",
            "receipt_id": f"ASR-{uuid.uuid4().hex.upper()}",
            "ledger_id": self.ledger_id,
            "conversation_id": self.namespace.conversation_id,
            "envelope_id": self.envelope["envelope_id"],
            "envelope_sha256": sha256_bytes(canonical_bytes(self.envelope)),
            "created_at": _timestamp(selected_now),
            "journal": {
                "relative_path": journal_path.relative_to(
                    self.namespace.root
                ).as_posix(),
                "sha256": sha256_bytes(journal_bytes),
                "bytes": len(journal_bytes),
                "event_count": len(self.events),
                "head_sha256": self.events[-1]["event_sha256"],
            },
            "sources": sources,
            "counts": {
                "events": len(self.events),
                "by_source": dict(sorted(by_source.items())),
                "by_type": dict(
                    sorted(
                        Counter(
                            item["canonical_event_type"]
                            for item in self.events
                        ).items()
                    )
                ),
            },
            "terminal": {
                "exactly_once": True,
                "event_id": terminals[0]["event_id"],
                "outcome": terminals[0]["terminal"]["outcome"],
            },
            "compaction": {
                "event_count": len(compaction_events),
                "summary_statuses": sorted(
                    {
                        item["compaction"]["summary_status"]
                        for item in compaction_events
                    }
                ),
            },
        }
        if shadow_refs is not None:
            commit_sha = shadow_refs.get("commit_sha", "")
            tree_sha = shadow_refs.get("tree_sha", "")
            if not isinstance(commit_sha, str) or not re.match(
                r"^[a-f0-9]{40}$", commit_sha
            ):
                raise AssuranceError(
                    "shadow_refs.commit_sha must be a 40-char Git commit SHA"
                )
            if not isinstance(tree_sha, str) or not re.match(
                r"^[a-f0-9]{40}$", tree_sha
            ):
                raise AssuranceError(
                    "shadow_refs.tree_sha must be a 40-char Git tree SHA"
                )
            body["shadow_refs"] = {
                "commit_sha": commit_sha,
                "tree_sha": tree_sha,
            }
        receipt = _sign(body, key_store=self.key_store)
        validate_contract(
            receipt,
            "audit-seal-receipt-v0.1.schema.json",
            label="metadata audit seal",
        )
        atomic_write_json(
            self.namespace.receipts_root
            / f"audit-seal-{self.ledger_id}.json",
            receipt,
        )
        verification = verify_audit_seal(
            receipt,
            namespace=self.namespace,
            key_store=self.key_store,
        )
        if not verification["valid"]:
            raise AssuranceError(
                "metadata audit seal failed verification: "
                + "; ".join(verification["errors"])
            )
        self._sealed = True
        return receipt


def _load_events(journal_bytes: bytes) -> tuple[list[dict[str, Any]], list[str]]:
    events: list[dict[str, Any]] = []
    errors: list[str] = []
    for line_number, raw_line in enumerate(journal_bytes.splitlines(), 1):
        if not raw_line.strip():
            continue
        try:
            event = json.loads(raw_line.decode("utf-8"))
            validate_contract(
                event,
                "audit-event-v0.1.schema.json",
                label=f"metadata audit event line {line_number}",
            )
        except Exception as exc:
            errors.append(str(exc))
            continue
        events.append(event)
    return events, errors


def verify_audit_seal(
    receipt: dict[str, Any],
    *,
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    require_archive_complete: bool = False,
) -> dict[str, Any]:
    errors: list[str] = []
    try:
        validate_contract(
            receipt,
            "audit-seal-receipt-v0.1.schema.json",
            label="metadata audit seal",
        )
        errors.extend(_verify_signature(receipt, key_store=key_store))
        relative = receipt["journal"]["relative_path"]
        prefix = "artifacts/redacted_conversation/"
        if not relative.startswith(prefix):
            raise AssuranceError("audit journal is outside retained category")
        journal_bytes = namespace.read_artifact(
            namespace.conversation_id,
            "redacted_conversation",
            relative[len(prefix) :],
        )
    except Exception as exc:
        return {
            "valid": False,
            "archive_complete": False,
            "raw_payload_absence_proven": False,
            "errors": [str(exc)],
        }
    if receipt["conversation_id"] != namespace.conversation_id:
        errors.append("audit seal conversation mismatch")
    if (
        receipt["journal"]["sha256"] != sha256_bytes(journal_bytes)
        or receipt["journal"]["bytes"] != len(journal_bytes)
    ):
        errors.append("audit journal artifact mismatch")
    events, event_errors = _load_events(journal_bytes)
    errors.extend(event_errors)

    previous: str | None = None
    sequences: dict[tuple[str, str], int] = {}
    for index, event in enumerate(events):
        if (
            event["ledger_id"] != receipt["ledger_id"]
            or event["conversation_id"] != receipt["conversation_id"]
            or event["envelope_id"] != receipt["envelope_id"]
        ):
            errors.append(f"audit event identity mismatch at sequence {index}")
        if event["sequence"] != index or event["event_id"] != f"AEV-{index:06d}":
            errors.append(f"audit event sequence mismatch at sequence {index}")
        if (
            event["previous_event_sha256"] != previous
            or event["event_sha256"] != _event_hash(event)
        ):
            errors.append(f"audit event hash-chain mismatch at sequence {index}")
        previous = event["event_sha256"]
        source_key = (event["source"]["kind"], event["source"]["source_id"])
        expected_source_sequence = sequences.get(source_key, 0)
        if event["source"]["source_sequence"] != expected_source_sequence:
            errors.append(
                f"audit source sequence mismatch at sequence {index}"
            )
        sequences[source_key] = expected_source_sequence + 1

    terminals = [item for item in events if item["terminal"]["is_terminal"]]
    by_source = Counter(item["source"]["kind"] for item in events)
    by_type = Counter(item["canonical_event_type"] for item in events)
    sources = [
        {
            "kind": item["kind"],
            "declared_completeness": item["declared_completeness"],
            "observed_event_count": by_source.get(item["kind"], 0),
        }
        for item in receipt["sources"]
    ]
    expected_counts = {
        "events": len(events),
        "by_source": dict(sorted(by_source.items())),
        "by_type": dict(sorted(by_type.items())),
    }
    if receipt["counts"] != expected_counts:
        errors.append("audit event counts do not rebuild")
    if receipt["sources"] != sources:
        errors.append("audit source completeness projection mismatch")
    declared_source_kinds = [item["kind"] for item in receipt["sources"]]
    if (
        not CORE_SOURCE_KINDS <= set(declared_source_kinds)
        or len(declared_source_kinds) != len(set(declared_source_kinds))
    ):
        errors.append("audit core source declaration is incomplete or duplicated")
    for source in receipt["sources"]:
        observed_states = {
            item["source"]["completeness"]
            for item in events
            if item["source"]["kind"] == source["kind"]
        }
        if observed_states and observed_states != {
            source["declared_completeness"]
        }:
            errors.append(
                f"audit source state mismatch: {source['kind']}"
            )
        if (
            source["observed_event_count"] == 0
            and source["declared_completeness"] != "unknown"
        ):
            errors.append(
                f"unobserved audit source promoted: {source['kind']}"
            )
    if (
        len(terminals) != 1
        or not events
        or terminals[0] is not events[-1]
        or receipt["terminal"]["event_id"] != terminals[0]["event_id"]
        or receipt["terminal"]["outcome"]
        != terminals[0]["terminal"]["outcome"]
    ):
        errors.append("audit terminal is not exactly-once and final")
    compaction_events = [
        item for item in events if item["compaction"] is not None
    ]
    for item in compaction_events:
        try:
            if _normalize_compaction(item["compaction"]) != item["compaction"]:
                errors.append(
                    f"audit compaction normalization mismatch: {item['event_id']}"
                )
        except AssuranceError as exc:
            errors.append(
                f"audit compaction invalid at {item['event_id']}: {exc}"
            )
    expected_compaction = {
        "event_count": len(compaction_events),
        "summary_statuses": sorted(
            {
                item["compaction"]["summary_status"]
                for item in compaction_events
            }
        ),
    }
    if receipt["compaction"] != expected_compaction:
        errors.append("audit compaction projection mismatch")
    if events and receipt["journal"]["head_sha256"] != events[-1]["event_sha256"]:
        errors.append("audit journal head mismatch")
    if receipt["journal"]["event_count"] != len(events):
        errors.append("audit journal event count mismatch")

    envelope_path = (
        namespace.artifacts_root
        / "active_security_envelope"
        / f"{receipt['envelope_id']}.json"
    )
    if envelope_path.exists():
        try:
            envelope = load_json(envelope_path)
            verify_security_envelope(envelope, key_store=key_store)
            if receipt["envelope_sha256"] != sha256_bytes(
                canonical_bytes(envelope)
            ):
                errors.append("audit seal envelope digest mismatch")
        except Exception as exc:
            errors.append(str(exc))

    archive_complete = False
    raw_payload_absence_proven = False
    if require_archive_complete:
        archive = verify_archive(namespace, key_store=key_store)
        archive_complete = bool(
            archive["valid"] and archive["archive_complete"]
        )
        raw_payload_absence_proven = archive_complete
        if not archive_complete:
            errors.extend(
                f"archive: {item}" for item in archive["errors"]
            )
            if not archive["errors"]:
                errors.append("archive is not verified complete")
    return {
        "valid": not errors,
        "archive_complete": archive_complete,
        "raw_payload_absence_proven": raw_payload_absence_proven,
        "event_count": len(events),
        "terminal_outcome": (
            terminals[0]["terminal"]["outcome"]
            if len(terminals) == 1
            else "unknown"
        ),
        "errors": errors,
        "limitations": [
            "The ledger proves recorded metadata, not missing unrecorded events.",
            "Archive verification is limited to the declared conversation namespace.",
        ],
    }
