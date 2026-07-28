"""PDF evidence layer — Phase 1 of Local Browser Retrieval & PDF Evidence.

Provides PDF validation, text-layer extraction, page indexing, and
full-text search using :mod:`pypdf`.  All operations are deterministic
and operate on local files only — no network access, no browser
automation.

Design constraints (from :file:`architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`):
    - No OCR for scanned PDFs.
    - PDFs with no text layer are reported as ``no_text_layer``.
    - Text is extracted per page and indexed for deterministic reads.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

from .errors import AssuranceError
from .utils import sha256_file


# ── validation ───────────────────────────────────────────────────────────────


@dataclass
class PdfValidation:
    """Result of :func:`validate_pdf`.

    Attributes
    ----------
    valid_pdf:
        ``True`` when the file starts with a valid PDF signature and can
        be opened by :mod:`pypdf`.
    has_text_layer:
        ``True`` when at least one page contains extractable text.
    page_count:
        Number of pages (0 if *valid_pdf* is ``False``).
    warnings:
        Non-fatal observations (e.g. suspicious file size).
    status:
        Discriminant: ``"ok"`` | ``"invalid_pdf"`` | ``"no_text_layer"``.
    """

    valid_pdf: bool = False
    has_text_layer: bool = False
    page_count: int = 0
    warnings: list[str] = field(default_factory=list)
    status: str = ""


def validate_pdf(path: Path) -> PdfValidation:
    """Validate a PDF file and detect its text layer.

    Checks the PDF signature, counts pages, and determines whether
    extractable text exists.  Does **not** perform OCR.

    Returns
    -------
    :
        A :class:`PdfValidation` with ``status`` set to one of:

        * ``"ok"`` — valid PDF with extractable text
        * ``"no_text_layer"`` — valid PDF but no extractable text
        * ``"invalid_pdf"`` — not a PDF or unreadable
    """
    from pypdf import PdfReader
    from pypdf.errors import PdfReadError

    result = PdfValidation()

    # Check file exists and is non-empty
    if not path.is_file():
        result.status = "invalid_pdf"
        result.warnings.append(f"not a file: {path}")
        return result

    file_size = path.stat().st_size
    if file_size == 0:
        result.status = "invalid_pdf"
        result.warnings.append("file is empty")
        return result

    # Suspiciously tiny file (likely HTML login page saved as .pdf)
    if file_size < 256:
        result.warnings.append(f"suspiciously small file ({file_size} bytes)")

    # Validate PDF signature
    try:
        reader = PdfReader(str(path))
    except (PdfReadError, ValueError, OSError) as exc:
        result.status = "invalid_pdf"
        result.warnings.append(f"cannot open as PDF: {exc}")
        return result

    result.valid_pdf = True
    result.page_count = len(reader.pages)

    if result.page_count == 0:
        result.warnings.append("PDF has zero pages")

    # Check for extractable text
    text_chars = 0
    for page in reader.pages:
        try:
            page_text = page.extract_text() or ""
        except Exception:
            page_text = ""
        text_chars += len(page_text.strip())

    result.has_text_layer = text_chars > 0

    if not result.has_text_layer:
        result.status = "no_text_layer"
    else:
        result.status = "ok"

    return result


# ── text extraction ──────────────────────────────────────────────────────────


def extract_text(path: Path) -> list[str]:
    """Extract text from each page of a valid PDF.

    Returns a list where ``result[i]`` is the text of page *i+1*.
    Raises :class:`AssuranceError` if the file cannot be read as a PDF.

    Empty pages produce empty strings — the list length always matches
    the page count.
    """
    validation = validate_pdf(path)
    if not validation.valid_pdf:
        raise AssuranceError(f"Cannot extract text: {validation.status} — {'; '.join(validation.warnings)}")

    from pypdf import PdfReader

    reader = PdfReader(str(path))
    pages: list[str] = []
    for page in reader.pages:
        try:
            pages.append((page.extract_text() or ""))
        except Exception:
            pages.append("")
    return pages


# ── page index ───────────────────────────────────────────────────────────────


@dataclass
class PageEntry:
    """One page of extracted text."""

    page: int         # 1-indexed
    text: str
    char_count: int


@dataclass
class PageIndex:
    """Complete page-level text index for a stored PDF document.

    Attributes
    ----------
    document_id:
        Content-addressed identifier, e.g. ``"sha256:abcd1234..."``.
    pages:
        Ordered list of :class:`PageEntry` items, one per page.
    total_chars:
        Sum of ``char_count`` across all pages.
    """

    document_id: str
    pages: list[PageEntry] = field(default_factory=list)
    total_chars: int = 0


def build_page_index(path: Path, document_id: str) -> PageIndex:
    """Validate, extract, and build a :class:`PageIndex` for *path*.

    Parameters
    ----------
    path:
        Path to a PDF file on disk.
    document_id:
        Content-addressed identifier (typically ``"sha256:<hex>"``).

    Returns
    -------
    :
        A complete page index suitable for serialisation to ``pages.jsonl``.
    """
    validation = validate_pdf(path)
    if not validation.valid_pdf:
        raise AssuranceError(
            f"PDF validation failed: {validation.status} — "
            f"{'; '.join(validation.warnings)}"
        )

    pages_text = extract_text(path)
    entries: list[PageEntry] = []
    total = 0
    for i, text in enumerate(pages_text, start=1):
        stripped = text.strip()
        char_count = len(stripped)
        total += char_count
        entries.append(PageEntry(page=i, text=stripped, char_count=char_count))

    return PageIndex(
        document_id=document_id,
        pages=entries,
        total_chars=total,
    )


# ── page-level read & search ─────────────────────────────────────────────────


def read_pages(index: PageIndex, start: int, end: int) -> str:
    """Return concatenated text for pages [*start*, *end*] (1-indexed, inclusive).

    Raises :class:`AssuranceError` if the range is invalid.
    """
    page_count = len(index.pages)
    if start < 1:
        raise AssuranceError(f"start page must be >= 1, got {start}")
    if end > page_count:
        raise AssuranceError(
            f"end page {end} exceeds page count {page_count}"
        )
    if start > end:
        raise AssuranceError(f"start page {start} > end page {end}")

    selected = index.pages[start - 1 : end]
    return "\n\n".join(p.text for p in selected)


def find_in_pages(index: PageIndex, query: str) -> list[dict]:
    """Case-insensitive search across all pages.

    Returns
    -------
    :
        A list of hits, each a dict with keys ``page`` (int), ``context``
        (str — surrounding text), and ``position`` (int — character offset
        within the page).
    """
    q = query.lower()
    results: list[dict] = []
    context_radius = 60  # characters on each side

    for entry in index.pages:
        text_lower = entry.text.lower()
        pos = 0
        while True:
            idx = text_lower.find(q, pos)
            if idx == -1:
                break
            # Extract surrounding context
            ctx_start = max(0, idx - context_radius)
            ctx_end = min(len(entry.text), idx + len(query) + context_radius)
            context = entry.text[ctx_start:ctx_end]
            if ctx_start > 0:
                context = "…" + context
            if ctx_end < len(entry.text):
                context = context + "…"

            results.append({
                "page": entry.page,
                "context": context,
                "position": idx,
            })
            pos = idx + 1
    return results


# ── metadata helpers ─────────────────────────────────────────────────────────


def guess_version(
    title: str,
    doi: str = "",
    metadata_text: str = "",
) -> tuple[str, float]:
    """Return a ``(version_guess, confidence)`` pair.

    Uses simple heuristics based on title and metadata text substrings.
    Confidence is a float in [0.0, 1.0].

    Currently recognises: ``"publisher"``, ``"preprint"``, ``"accepted_manuscript"``,
    ``"supplement"``, ``"correction"``, ``"unknown"``.
    """
    combined = f"{title} {metadata_text} {doi}".lower()

    # Strong signals
    if any(kw in combined for kw in ("correction to", "corrigendum", "erratum")):
        return ("correction", 0.90)
    if any(kw in combined for kw in ("supplementary", "supplemental", "supporting information")):
        return ("supplement", 0.85)

    # Medium signals
    preprint_signals = sum(1 for kw in ("arxiv", "preprint", "biorxiv", "medrxiv", "chemrxiv") if kw in combined)
    publisher_signals = sum(1 for kw in ("published by", "volume", "issue", "pages:", "doi.org/10.") if kw in combined)
    accepted_signals = sum(1 for kw in ("accepted manuscript", "accepted for publication", "postprint") if kw in combined)

    if preprint_signals >= 2 and publisher_signals == 0:
        return ("preprint", 0.80)
    if accepted_signals >= 1:
        return ("accepted_manuscript", 0.75)
    if publisher_signals >= 3:
        return ("publisher", 0.70)
    if publisher_signals >= 1:
        return ("publisher", 0.60)
    if preprint_signals >= 1:
        return ("preprint", 0.55)

    return ("unknown", 0.30)
