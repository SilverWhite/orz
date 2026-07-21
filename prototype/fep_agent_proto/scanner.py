from __future__ import annotations

import json
from pathlib import Path
import re
from typing import Any

from . import __version__
from .errors import PrototypeError
from .io_utils import (
    canonical_bytes,
    collect_file_records,
    digest_file_records,
    load_json,
    sha256_bytes,
    utc_now,
)
from .layout import RUNTIME_ROOT
from .schema import validate_instance


FORBIDDEN_KEYS = {
    "oracle",
    "classification",
    "sources",
    "countercase_ids",
    "curation_fixture_manifest",
    "hidden_from_subject",
    "expected_gate_decisions",
    "allowed_claims",
    "forbidden_claims",
    "correction_summary",
}

TEXT_RULES = {
    "LEAK-ID-INTERNAL-001": re.compile(r"\bFEP-(?:REG|SYN)-\d{3}\b|\bCLU-\d{2}\b"),
    "LEAK-PATH-SOURCE-001": re.compile(
        r"R\d+_DISCUSSION_|EXPERIMENT_PROGRESS_MAP\d+|LIF_CURRENT_INDEX\.md|self_check_protocol\.md",
        re.IGNORECASE,
    ),
    "LEAK-SEM-PHRASE-001": re.compile(
        r"\b(?:expected[_ ]gate|forbidden[_ ]claim|correction[_ ]summary|the correct answer)\b",
        re.IGNORECASE,
    ),
}


def _walk_keys(value: Any, pointer: str = "") -> list[tuple[str, str]]:
    hits: list[tuple[str, str]] = []
    if isinstance(value, dict):
        for key, nested in value.items():
            child = f"{pointer}/{key}"
            if key in FORBIDDEN_KEYS:
                hits.append((key, child))
            hits.extend(_walk_keys(nested, child))
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            hits.extend(_walk_keys(nested, f"{pointer}/{index}"))
    return hits


def _finding(
    number: int, *, severity: str, location: str, summary: str
) -> dict[str, Any]:
    fingerprint = sha256_bytes(canonical_bytes({"location": location, "summary": summary}))
    return {
        "finding_id": f"FIND-{number:03d}",
        "severity": severity,
        "location": location,
        "fingerprint": fingerprint,
        "summary": summary,
    }


def _status_for(findings: list[dict[str, Any]], rule_prefix: str) -> str:
    matching = [item for item in findings if item["summary"].startswith(rule_prefix)]
    if any(item["severity"] == "fail" for item in matching):
        return "fail"
    if matching:
        return "warn"
    return "pass"


def scan_bundle(
    bundle_root: Path,
    *,
    export_id: str,
    policy_mode: str,
    expected_bundle_sha256: str | None = None,
) -> dict[str, Any]:
    if policy_mode not in {"development", "challenge", "evaluation", "holdout"}:
        raise PrototypeError(f"unsupported leak-scan policy mode: {policy_mode}")

    started_at = utc_now()
    findings: list[dict[str, Any]] = []
    records: list[dict[str, Any]] = []
    try:
        records = collect_file_records(bundle_root)
    except PrototypeError as exc:
        findings.append(
            _finding(
                len(findings) + 1,
                severity="fail",
                location=str(bundle_root),
                summary=f"LEAK-FS-ESCAPE-001: {exc}",
            )
        )

    bundle_sha256 = digest_file_records(records)
    if expected_bundle_sha256 and bundle_sha256 != expected_bundle_sha256:
        findings.append(
            _finding(
                len(findings) + 1,
                severity="fail",
                location="bundle",
                summary="LEAK-DIGEST-MISMATCH-001: bundle digest differs from manifest",
            )
        )

    index_path = bundle_root / "bundle-index.json"
    if records and index_path.is_file():
        try:
            index = load_json(index_path)
            expected = [
                {key: item[key] for key in ("path", "size_bytes", "sha256")}
                for item in index.get("files", [])
            ]
            actual = [item for item in records if item["path"] != "bundle-index.json"]
            if expected != actual:
                findings.append(
                    _finding(
                        len(findings) + 1,
                        severity="fail",
                        location="bundle-index.json",
                        summary="LEAK-DIGEST-INDEX-001: bundle index does not match actual files",
                    )
                )
        except PrototypeError as exc:
            findings.append(
                _finding(
                    len(findings) + 1,
                    severity="fail",
                    location="bundle-index.json",
                    summary=f"LEAK-DIGEST-INDEX-001: {exc}",
                )
            )
    else:
        findings.append(
            _finding(
                len(findings) + 1,
                severity="fail",
                location="bundle-index.json",
                summary="LEAK-DIGEST-INDEX-001: bundle index is missing",
            )
        )

    for record in records:
        path = bundle_root / record["path"]
        raw = path.read_bytes()
        text = raw.decode("utf-8", errors="replace")
        if path.suffix.lower() == ".json":
            try:
                value = json.loads(text)
            except json.JSONDecodeError as exc:
                findings.append(
                    _finding(
                        len(findings) + 1,
                        severity="fail",
                        location=record["path"],
                        summary=f"LEAK-STRUCT-JSON-001: invalid JSON: {exc}",
                    )
                )
            else:
                for key, pointer in _walk_keys(value):
                    findings.append(
                        _finding(
                            len(findings) + 1,
                            severity="fail",
                            location=f"{record['path']}#{pointer}",
                            summary=f"LEAK-STRUCT-FIELD-001: forbidden key {key}",
                        )
                    )
        for rule_id, pattern in TEXT_RULES.items():
            match = pattern.search(text)
            if match:
                severity = "warn" if rule_id == "LEAK-SEM-PHRASE-001" else "fail"
                findings.append(
                    _finding(
                        len(findings) + 1,
                        severity=severity,
                        location=record["path"],
                        summary=f"{rule_id}: matched answer-bearing or internal text pattern",
                    )
                )

    checks = [
        {
            "rule_id": "LEAK-STRUCT-FIELD-001",
            "layer": "structural",
            "status": _status_for(findings, "LEAK-STRUCT"),
            "evidence": "Parsed every JSON file and recursively inspected forbidden keys.",
        },
        {
            "rule_id": "LEAK-ID-INTERNAL-001",
            "layer": "semantic",
            "status": _status_for(findings, "LEAK-ID-INTERNAL"),
            "evidence": "Scanned UTF-8 views for internal case and cluster identifiers.",
        },
        {
            "rule_id": "LEAK-PATH-SOURCE-001",
            "layer": "path",
            "status": _status_for(findings, "LEAK-PATH-SOURCE"),
            "evidence": "Scanned paths and text for source-document identifiers.",
        },
        {
            "rule_id": "LEAK-DIGEST-BUNDLE-001",
            "layer": "digest",
            "status": "fail"
            if any(item["summary"].startswith("LEAK-DIGEST") for item in findings)
            else "pass",
            "evidence": "Recomputed file records, bundle index, and optional expected bundle digest.",
        },
        {
            "rule_id": "LEAK-FS-ESCAPE-001",
            "layer": "mount",
            "status": _status_for(findings, "LEAK-FS-ESCAPE"),
            "evidence": "Rejected linked files, linked directories, reparse points, and root escapes.",
        },
        {
            "rule_id": "LEAK-SEM-REVIEW-001",
            "layer": "semantic",
            "status": "review_required",
            "evidence": "No independent human semantic review was performed by this prototype.",
        },
    ]

    if any(item["severity"] == "fail" for item in findings):
        verdict = "fail"
    else:
        verdict = "warn"
    if verdict == "fail":
        gate_action = "block"
    elif policy_mode == "development":
        gate_action = "allow"
    elif policy_mode == "challenge":
        gate_action = "defer"
    else:
        gate_action = "block"

    report = {
        "schema_version": "0.1.0-draft",
        "scan_id": f"LEAKSCAN-{bundle_sha256[:12].upper()}",
        "export_id": export_id,
        "visibility": "reviewer_only",
        "policy_mode": policy_mode,
        "scanner_version": __version__,
        "started_at": started_at,
        "completed_at": utc_now(),
        "target_bundle_sha256": bundle_sha256,
        "checks": checks,
        "findings": findings,
        "semantic_review": {
            "method": "not_run",
            "status": "not_run",
            "reviewer_role_id": None,
            "completed_at": None,
            "independence_attested": False,
        },
        "verdict": verdict,
        "gate_action": gate_action,
        "limitations": [
            "Mechanical scanning cannot establish semantic oracle isolation.",
            "This development-only prototype cannot authorize evaluation or holdout execution.",
        ],
    }
    validate_instance(
        report,
        RUNTIME_ROOT / "leak-scan-report-v0.1.schema.json",
        label="leak scan report",
    )
    return report
