"""End-to-end test: browser retrieval → PDF download → evidence store.

Usage::

    python scripts/test_browser_retrieval_e2e.py

Requires Chrome installed.  Launches headless Chrome, navigates to an
arXiv SNN survey paper, downloads the PDF, and stores it in the evidence
store.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Ensure the project root is on sys.path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from assurance.retrieval_workflow import RetrievalProgress, run_retrieval


def _on_progress(evt: RetrievalProgress) -> None:
    print(f"  [{evt.stage}] {evt.message}")
    if evt.detail:
        print(f"           {evt.detail}")


def main() -> int:
    # arXiv SNN survey 2026 — open access, always available
    paper_url = "https://arxiv.org/abs/2605.15058"

    print("=" * 60)
    print("LBR-001 End-to-End Test: Browser → PDF → Evidence Store")
    print("=" * 60)
    print(f"\nPaper: {paper_url}")
    print("Launching headless Chrome...\n")

    result = run_retrieval(
        paper_url,
        port=9225,
        launch_browser=True,
        browser="chrome",
        headless=True,
        on_progress=_on_progress,
    )

    print()
    if result.error:
        print(f"FAILED: {result.error}")
        return 1

    print("=" * 60)
    print("SUCCESS")
    print("=" * 60)
    print(f"  Document ID:  {result.document_id}")
    print(f"  Title:        {result.title}")
    print(f"  DOI:          {result.doi or 'n/a'}")
    print(f"  Authors:      {', '.join(result.authors) if result.authors else 'n/a'}")
    print(f"  Year:         {result.year or 'n/a'}")
    print(f"  Pages:        {result.page_count}")
    print(f"  Total chars:  {result.total_chars}")
    print(f"  PDF candidates found: {result.pdf_candidates_found}")
    print(f"  Evidence level: {result.evidence_level}")
    print(f"  Source record: {result.source_record_path}")

    # Verify: can we read the stored PDF?
    print("\n--- Stored PDF pages (first 3) ---")
    from assurance.evidence_store import read_pages_jsonl
    pages = read_pages_jsonl(result.document_id)
    for p in pages[:3]:
        preview = p["text"][:120].replace("\n", " ")
        print(f"  Page {p['page']} ({p['char_count']} chars): {preview}…")

    return 0


if __name__ == "__main__":
    sys.exit(main())
