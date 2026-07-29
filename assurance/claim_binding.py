"""Claim-evidence binding — LBR-001 §12.

Structured binding between answer claims and the evidence sources that
support them.  Each claim records:

- What assertion is being made (claim_text, claim_type)
- Which sources support it (evidence list with document_id, page, section)
- How strongly it is supported (support_level)

This is separate from source records — a source record describes where
evidence came from; a claim binding describes how it is used.
"""

from __future__ import annotations

import hashlib
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from .utils import utc_now


# ── enums ────────────────────────────────────────────────────────────────────


class ClaimType(str, Enum):
    """What kind of assertion a claim represents."""

    DEFINITION = "definition"
    METHOD = "method"
    RESULT = "result"
    LIMITATION = "limitation"
    BACKGROUND = "background"
    IMPLEMENTATION_DETAIL = "implementation_detail"
    CURRENT_FACT = "current_fact"
    COMPARISON = "comparison"


class EvidenceRole(str, Enum):
    """How a piece of evidence supports a claim."""

    DEFINITION = "definition"
    METHOD = "method"
    RESULT = "result"
    LIMITATION = "limitation"
    BACKGROUND = "background"
    IMPLEMENTATION_DETAIL = "implementation_detail"
    CURRENT_FACT = "current_fact"


class SupportLevel(str, Enum):
    """How strongly the evidence supports the claim."""

    DIRECT = "direct"            # exact text, same section
    INFERRED = "inferred"        # reasonable interpretation
    PARTIAL = "partial"          # some aspects supported, others not
    WEAK = "weak"               # tangential or low-confidence match


# ── data types ───────────────────────────────────────────────────────────────


@dataclass
class EvidenceBinding:
    """A single piece of evidence backing a claim."""

    source_id: str
    document_id: str = ""
    page: int | None = None
    section: str = ""
    text_span_hash: str = ""     # SHA-256 of the exact supporting text
    evidence_role: EvidenceRole = EvidenceRole.BACKGROUND


@dataclass
class Claim:
    """A verifiable claim backed by evidence."""

    claim_id: str = ""
    claim_text: str = ""
    claim_type: ClaimType = ClaimType.CURRENT_FACT
    evidence: list[EvidenceBinding] = field(default_factory=list)
    support_level: SupportLevel = SupportLevel.DIRECT
    created_at: str = field(default_factory=utc_now)
    notes: str = ""


# ── public API ───────────────────────────────────────────────────────────────


def make_claim_id() -> str:
    """Generate a unique claim ID."""
    import uuid
    return f"CLAIM-{uuid.uuid4().hex[:12].upper()}"


def hash_text_span(text: str) -> str:
    """SHA-256 of a text span, used for evidence binding."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def bind_claim_to_evidence(
    *,
    claim_text: str,
    claim_type: ClaimType | str = ClaimType.CURRENT_FACT,
    source_id: str = "",
    document_id: str = "",
    page: int | None = None,
    section: str = "",
    text_span: str = "",
    evidence_role: EvidenceRole | str = EvidenceRole.BACKGROUND,
    support_level: SupportLevel | str = SupportLevel.DIRECT,
    notes: str = "",
    claim_id: str | None = None,
) -> Claim:
    """Create a :class:`Claim` bound to a single piece of evidence.

    For claims supported by multiple sources, call this once per source
    and merge the evidence lists manually.
    """
    ct = ClaimType(claim_type) if isinstance(claim_type, str) else claim_type
    er = EvidenceRole(evidence_role) if isinstance(evidence_role, str) else evidence_role
    sl = SupportLevel(support_level) if isinstance(support_level, str) else support_level

    text_hash = hash_text_span(text_span) if text_span else ""

    return Claim(
        claim_id=claim_id or make_claim_id(),
        claim_text=claim_text,
        claim_type=ct,
        evidence=[
            EvidenceBinding(
                source_id=source_id,
                document_id=document_id,
                page=page,
                section=section,
                text_span_hash=text_hash,
                evidence_role=er,
            )
        ],
        support_level=sl,
        notes=notes,
    )


def claims_to_dict(claim: Claim) -> dict[str, Any]:
    """Serialize a :class:`Claim` to a JSON-compatible dict."""
    return {
        "claim_id": claim.claim_id,
        "claim_text": claim.claim_text,
        "claim_type": claim.claim_type.value,
        "evidence": [
            {
                "source_id": e.source_id,
                "document_id": e.document_id,
                "page": e.page,
                "section": e.section,
                "text_span_hash": e.text_span_hash,
                "evidence_role": e.evidence_role.value,
            }
            for e in claim.evidence
        ],
        "support_level": claim.support_level.value,
        "created_at": claim.created_at,
        "notes": claim.notes,
    }


def attach_citation_to_source_record(
    source_record: dict[str, Any],
    *,
    page: int,
    section: str = "",
    evidence_role: str = "",
    quote_text: str = "",
) -> dict[str, Any]:
    """Attach a citation location to an existing source record.

    Populates the ``citation_locations`` field that was previously
    schema-defined but never written to by any code path.

    Returns the mutated *source_record* (also modified in place).
    """
    entry: dict[str, Any] = {
        "page": page,
        "section": section,
        "evidence_role": evidence_role,
    }
    if quote_text:
        entry["quote_hash"] = hash_text_span(quote_text)

    locations: list[dict[str, Any]] = source_record.get("citation_locations", [])
    if locations is None:
        locations = []
    locations.append(entry)
    source_record["citation_locations"] = locations
    return source_record
