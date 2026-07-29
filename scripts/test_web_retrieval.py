"""Test general web retrieval — search + open multiple pages through local browser.

Usage::

    python scripts/test_web_retrieval.py
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from assurance.retrieval_workflow import (
    RetrievalProgress,
    retrieve_search,
    retrieve_urls,
)


def _on_progress(evt: RetrievalProgress) -> None:
    print(f"  [{evt.stage}] {evt.message}")
    if evt.detail:
        print(f"           {evt.detail}")


def main() -> int:
    print("=" * 60)
    print("LBR-001 General Web Retrieval Test")
    print("=" * 60)

    # Test 1: Search via user's default Google
    query = "spiking neural network neuromorphic chip 2026"
    print(f"\n--- Test: Google search '{query}' ---\n")

    result = retrieve_search(
        query,
        port=9222,
        launch_browser=True,     # auto-launch if needed
        browser="chrome",
        headless=False,
        on_progress=_on_progress,
        max_results=3,
        engine="google",
    )

    print(f"\nRetrieved {len(result.pages)} pages, {result.total_chars} chars")
    for p in result.pages:
        status = f"ERROR: {p.error}" if p.error else f"{p.char_count} chars, {p.link_count} links"
        print(f"  {p.title[:80]}")
        print(f"    {p.url[:80]}")
        print(f"    {status}")
        if p.pdf_candidates:
            print(f"    PDFs: {p.pdf_candidates}")
        if p.text_preview:
            preview = p.text_preview[:150].replace("\n", " ")
            print(f"    Preview: {preview}…")
        print()

    # Test 2: Direct URL retrieval (multiple pages)
    urls = [
        "https://arxiv.org/abs/2605.15058",        # SNN survey
        "https://en.wikipedia.org/wiki/Spiking_neural_network",
    ]
    print(f"\n--- Test: Direct URL retrieval ({len(urls)} URLs) ---\n")

    result2 = retrieve_urls(
        urls,
        port=9222,
        launch_browser=True,     # auto-launch if needed
        browser="chrome",
        headless=False,
        on_progress=_on_progress,
        max_pages=2,
    )

    print(f"\nRetrieved {len(result2.pages)} pages, {result2.total_chars} chars")
    for p in result2.pages:
        print(f"  {p.title[:80]} — {p.char_count} chars")

    return 0


if __name__ == "__main__":
    sys.exit(main())
