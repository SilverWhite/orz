"""Content-addressed PDF evidence store — Phase 1.

Implements the storage layout from §10.1 of
:file:`architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`::

    EVIDENCE_ROOT/
      papers/
        {sha256_prefix}/       # first 2 hex chars
          {sha256_full}/        # full 64-char hex
            original.pdf
            metadata.json
            pages.jsonl
            source_record.json

All writes are atomic (via :func:`assurance.utils.atomic_write_json` and
:func:`assurance.utils.atomic_write_bytes`).  Reads are plain filesystem
operations.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

from .contracts import ASSURANCE_ROOT
from .errors import AssuranceError
from .pdf_evidence import build_page_index, guess_version, validate_pdf
from .utils import (
    atomic_write_bytes,
    atomic_write_json,
    exclusive_create_bytes,
    load_json,
    sha256_file,
    utc_now,
)

# ── root path ────────────────────────────────────────────────────────────────

EVIDENCE_ROOT: Path = ASSURANCE_ROOT.parent / "evidence"
PAPERS_DIR: Path = EVIDENCE_ROOT / "papers"


def _ensure_dirs() -> None:
    PAPERS_DIR.mkdir(parents=True, exist_ok=True)


# ── path helpers ─────────────────────────────────────────────────────────────


def _document_id_from_sha256(sha256_hex: str) -> str:
    """Return the canonical document-id string for a SHA-256 hex digest."""
    if len(sha256_hex) != 64:
        raise AssuranceError(
            f"SHA-256 hex must be 64 characters, got {len(sha256_hex)}"
        )
    return f"sha256:{sha256_hex}"


def _parse_document_id(document_id: str) -> str:
    """Extract the SHA-256 hex from a ``sha256:...`` document ID."""
    if not document_id.startswith("sha256:"):
        raise AssuranceError(
            f"document_id must start with 'sha256:', got {document_id!r}"
        )
    hex_part = document_id[7:]
    if len(hex_part) != 64:
        raise AssuranceError(
            f"document_id SHA-256 must be 64 hex chars, got {len(hex_part)}"
        )
    return hex_part


def get_document_path(document_id: str) -> Path:
    """Resolve a ``sha256:...`` document ID to its storage directory."""
    hex_part = _parse_document_id(document_id)
    return PAPERS_DIR / hex_part[:2] / hex_part


# ── store & retrieve ─────────────────────────────────────────────────────────


def document_exists(document_id: str) -> bool:
    """Return ``True`` if *document_id* is already in the store."""
    doc_path = get_document_path(document_id)
    return doc_path.is_dir() and (doc_path / "original.pdf").is_file()


def store_pdf(
    pdf_path: Path,
    *,
    source_url: str = "",
    doi: str = "",
    title: str = "",
    authors: list[str] | None = None,
    year: int | None = None,
) -> str:
    """Copy *pdf_path* into the content-addressed store.

    Parameters
    ----------
    pdf_path:
        Path to the PDF file on disk (will be copied, not moved).
    source_url:
        Original URL where the PDF was found.
    doi:
        DOI of the work (e.g. ``"10.1234/example"``).
    title:
        Paper title (used for version guessing).
    authors:
        List of author name strings.
    year:
        Publication year.

    Returns
    -------
    :
        The canonical ``"sha256:..."`` document ID.
    """
    _ensure_dirs()

    # Validate
    validation = validate_pdf(pdf_path)
    if not validation.valid_pdf:
        raise AssuranceError(
            f"PDF validation failed: {validation.status} — "
            f"{'; '.join(validation.warnings)}"
        )

    sha256_hex = sha256_file(pdf_path)
    document_id = _document_id_from_sha256(sha256_hex)
    doc_path = get_document_path(document_id)

    # If already stored, return existing ID
    if doc_path.is_dir() and (doc_path / "original.pdf").is_file():
        return document_id

    doc_path.mkdir(parents=True, exist_ok=True)

    # Copy PDF bytes
    pdf_bytes = pdf_path.read_bytes()
    atomic_write_bytes(doc_path / "original.pdf", pdf_bytes, overwrite=False)

    # Build page index
    page_index = build_page_index(pdf_path, document_id)

    # Write pages.jsonl
    pages_lines: list[str] = []
    for entry in page_index.pages:
        pages_lines.append(
            json.dumps(
                {"page": entry.page, "text": entry.text, "char_count": entry.char_count},
                ensure_ascii=False,
                sort_keys=True,
            )
        )
    pages_bytes = "\n".join(pages_lines).encode("utf-8") + b"\n"
    atomic_write_bytes(doc_path / "pages.jsonl", pages_bytes, overwrite=False)

    # Extract PDF metadata
    try:
        from pypdf import PdfReader
        reader = PdfReader(str(pdf_path))
        pdf_meta = reader.metadata or {}
    except Exception:
        pdf_meta = {}

    # Guess version
    meta_text = str(pdf_meta)
    version_guess, version_confidence = guess_version(
        title=title, doi=doi, metadata_text=meta_text,
    )

    # Write metadata.json
    metadata = {
        "schema_version": "0.1.0-draft",
        "document_id": document_id,
        "work_id": f"doi:{doi}" if doi else "",
        "title": title,
        "authors": authors or [],
        "year": year,
        "version": version_guess,
        "version_confidence": version_confidence,
        "source_url": source_url,
        "downloaded_at": utc_now(),
        "bytes": pdf_path.stat().st_size,
        "sha256": sha256_hex,
        "pages": validation.page_count,
        "has_text_layer": validation.has_text_layer,
        "parser": "pypdf",
        "parser_version": _pypdf_version(),
        "extraction_status": validation.status,
    }
    atomic_write_json(doc_path / "metadata.json", metadata, overwrite=False)

    return document_id


def read_metadata(document_id: str) -> dict:
    """Load ``metadata.json`` for a stored document."""
    doc_path = get_document_path(document_id)
    meta_path = doc_path / "metadata.json"
    if not meta_path.is_file():
        raise AssuranceError(f"metadata not found for {document_id}")
    return load_json(meta_path)


def read_pages_jsonl(document_id: str) -> list[dict]:
    """Load ``pages.jsonl`` as a list of per-page dicts."""
    doc_path = get_document_path(document_id)
    pages_path = doc_path / "pages.jsonl"
    if not pages_path.is_file():
        raise AssuranceError(f"pages.jsonl not found for {document_id}")
    text = pages_path.read_text(encoding="utf-8")
    pages: list[dict] = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped:
            pages.append(json.loads(stripped))
    return pages


def get_original_pdf_path(document_id: str) -> Path:
    """Return the path to ``original.pdf`` for *document_id*."""
    doc_path = get_document_path(document_id)
    pdf_path = doc_path / "original.pdf"
    if not pdf_path.is_file():
        raise AssuranceError(f"original.pdf not found for {document_id}")
    return pdf_path


def store_source_record(document_id: str, record: dict) -> Path:
    """Write (or overwrite) ``source_record.json`` for a stored document.

    Returns the path to the written file.
    """
    doc_path = get_document_path(document_id)
    if not doc_path.is_dir():
        raise AssuranceError(f"document not in store: {document_id}")
    out = doc_path / "source_record.json"
    atomic_write_json(out, record, overwrite=True)
    return out


def list_documents() -> list[str]:
    """List all stored ``document_id`` values."""
    _ensure_dirs()
    ids: list[str] = []
    if not PAPERS_DIR.is_dir():
        return ids
    for prefix_dir in sorted(PAPERS_DIR.iterdir()):
        if not prefix_dir.is_dir() or len(prefix_dir.name) != 2:
            continue
        for doc_dir in sorted(prefix_dir.iterdir()):
            if doc_dir.is_dir() and (doc_dir / "original.pdf").is_file():
                # doc_dir.name is the full 64-char hex; prefix_dir.name is the first 2 chars
                hex_part = doc_dir.name
                if len(hex_part) == 64:
                    ids.append(_document_id_from_sha256(hex_part))
    return ids


# ── helpers ──────────────────────────────────────────────────────────────────


def _pypdf_version() -> str:
    try:
        from pypdf import __version__
        return str(__version__)
    except ImportError:
        return "unknown"
