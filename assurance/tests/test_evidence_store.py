"""Tests for PDF evidence store — Phase 1.

Covers :mod:`assurance.pdf_evidence` (validation, extraction, indexing,
search, version guessing) and :mod:`assurance.evidence_store` (storage,
retrieval, source records, schema validation).
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

from assurance.errors import AssuranceError
from assurance.utils import load_json, sha256_file

# ── path helpers ─────────────────────────────────────────────────────────────

ASSURANCE = Path(__file__).resolve().parents[1]
FIXTURES = ASSURANCE / "fixtures" / "evidence_store"
MINIMAL_PDF = FIXTURES / "minimal-text.pdf"
CORRUPT_PDF = FIXTURES / "not-a-pdf.pdf"
EMPTY_FILE = FIXTURES / "empty.dat"
SOURCE_RECORD_VALID = FIXTURES / "source_record.valid.json"
SOURCE_RECORD_INVALID = FIXTURES / "source_record.invalid.json"
SCHEMA = ASSURANCE / "evidence_store.schema.json"


# ══════════════════════════════════════════════════════════════════════════════
# pdf_evidence.py tests
# ══════════════════════════════════════════════════════════════════════════════


class PdfValidationTests(unittest.TestCase):
    """GAK-LBR-001: PDF validation — signature, text layer, page count."""

    def test_valid_pdf_with_text_passes(self) -> None:
        from assurance.pdf_evidence import validate_pdf
        v = validate_pdf(MINIMAL_PDF)
        self.assertTrue(v.valid_pdf)
        self.assertTrue(v.has_text_layer)
        self.assertEqual(v.page_count, 2)
        self.assertEqual(v.status, "ok")

    def test_non_pdf_rejected(self) -> None:
        from assurance.pdf_evidence import validate_pdf
        v = validate_pdf(CORRUPT_PDF)
        self.assertFalse(v.valid_pdf)
        self.assertEqual(v.status, "invalid_pdf")
        self.assertGreater(len(v.warnings), 0)

    def test_empty_file_rejected(self) -> None:
        from assurance.pdf_evidence import validate_pdf
        v = validate_pdf(EMPTY_FILE)
        self.assertFalse(v.valid_pdf)
        self.assertEqual(v.status, "invalid_pdf")

    def test_nonexistent_file_rejected(self) -> None:
        from assurance.pdf_evidence import validate_pdf
        v = validate_pdf(Path("/nonexistent/pdf_12345.pdf"))
        self.assertFalse(v.valid_pdf)
        self.assertEqual(v.status, "invalid_pdf")

    def test_small_file_warns(self) -> None:
        from assurance.pdf_evidence import validate_pdf
        v = validate_pdf(CORRUPT_PDF)
        self.assertIn("suspiciously small file", " ".join(v.warnings).lower())


class PdfExtractionTests(unittest.TestCase):
    """GAK-LBR-001: PDF text extraction and page indexing."""

    def test_extract_text_per_page(self) -> None:
        from assurance.pdf_evidence import extract_text
        pages = extract_text(MINIMAL_PDF)
        self.assertEqual(len(pages), 2)
        self.assertIn("page one", pages[0].lower())
        self.assertIn("page two", pages[1].lower())

    def test_extract_from_invalid_pdf_raises(self) -> None:
        from assurance.pdf_evidence import extract_text
        with self.assertRaises(AssuranceError):
            extract_text(CORRUPT_PDF)

    def test_build_page_index(self) -> None:
        from assurance.pdf_evidence import build_page_index, PageIndex
        index = build_page_index(MINIMAL_PDF, "sha256:test")
        self.assertIsInstance(index, PageIndex)
        self.assertEqual(len(index.pages), 2)
        self.assertEqual(index.document_id, "sha256:test")
        self.assertGreater(index.total_chars, 0)

    def test_read_pages_range(self) -> None:
        from assurance.pdf_evidence import build_page_index, read_pages
        index = build_page_index(MINIMAL_PDF, "sha256:test")
        text = read_pages(index, 1, 1)
        self.assertIn("page one", text.lower())
        self.assertNotIn("page two", text.lower())

    def test_read_pages_invalid_range_raises(self) -> None:
        from assurance.pdf_evidence import build_page_index, read_pages
        index = build_page_index(MINIMAL_PDF, "sha256:test")
        with self.assertRaises(AssuranceError):
            read_pages(index, 0, 1)
        with self.assertRaises(AssuranceError):
            read_pages(index, 1, 99)
        with self.assertRaises(AssuranceError):
            read_pages(index, 3, 1)

    def test_find_in_pages_case_insensitive(self) -> None:
        from assurance.pdf_evidence import build_page_index, find_in_pages
        index = build_page_index(MINIMAL_PDF, "sha256:test")
        hits = find_in_pages(index, "PAGE")
        # Should find hits in both pages
        pages_hit = {h["page"] for h in hits}
        self.assertIn(1, pages_hit)
        self.assertIn(2, pages_hit)

    def test_find_in_pages_no_match(self) -> None:
        from assurance.pdf_evidence import build_page_index, find_in_pages
        index = build_page_index(MINIMAL_PDF, "sha256:test")
        hits = find_in_pages(index, "xyznonexistent123")
        self.assertEqual(hits, [])

    def test_build_page_index_invalid_pdf_raises(self) -> None:
        from assurance.pdf_evidence import build_page_index
        with self.assertRaises(AssuranceError):
            build_page_index(CORRUPT_PDF, "sha256:test")

    def test_page_entry_fields(self) -> None:
        from assurance.pdf_evidence import build_page_index, PageEntry
        index = build_page_index(MINIMAL_PDF, "sha256:test")
        for entry in index.pages:
            self.assertIsInstance(entry, PageEntry)
            self.assertGreater(entry.page, 0)
            self.assertIsInstance(entry.text, str)
            self.assertGreaterEqual(entry.char_count, 0)


class VersionGuessingTests(unittest.TestCase):
    """GAK-LBR-001: version_guess / version_confidence heuristics."""

    def test_preprint_detected(self) -> None:
        from assurance.pdf_evidence import guess_version
        v, conf = guess_version(
            title="A Novel Method",
            metadata_text="arXiv:2501.12345 preprint",
        )
        self.assertEqual(v, "preprint")
        self.assertGreater(conf, 0.5)

    def test_publisher_detected(self) -> None:
        from assurance.pdf_evidence import guess_version
        v, conf = guess_version(
            title="Published Research",
            doi="10.1234/example",
            metadata_text="Published by Example Press, Volume 42, Issue 3, Pages: 100-120",
        )
        self.assertEqual(v, "publisher")
        self.assertGreater(conf, 0.5)

    def test_correction_detected(self) -> None:
        from assurance.pdf_evidence import guess_version
        v, conf = guess_version(
            title="Corrigendum: Original Title",
            metadata_text="correction to the original article",
        )
        self.assertEqual(v, "correction")
        self.assertGreater(conf, 0.8)

    def test_supplement_detected(self) -> None:
        from assurance.pdf_evidence import guess_version
        v, conf = guess_version(
            title="Supplementary Material for the Paper",
            metadata_text="supporting information",
        )
        self.assertEqual(v, "supplement")
        self.assertGreater(conf, 0.5)

    def test_accepted_manuscript_detected(self) -> None:
        from assurance.pdf_evidence import guess_version
        v, conf = guess_version(
            title="A Study",
            metadata_text="accepted manuscript, postprint version",
        )
        self.assertEqual(v, "accepted_manuscript")
        self.assertGreater(conf, 0.5)

    def test_unknown_when_no_signals(self) -> None:
        from assurance.pdf_evidence import guess_version
        v, conf = guess_version(title="A Study", metadata_text="Introduction Methods Results")
        self.assertEqual(v, "unknown")
        self.assertLess(conf, 0.5)


# ══════════════════════════════════════════════════════════════════════════════
# evidence_store.py tests
# ══════════════════════════════════════════════════════════════════════════════


class EvidenceStoreTests(unittest.TestCase):
    """GAK-LBR-001: content-addressed PDF storage and retrieval."""

    def setUp(self) -> None:
        from assurance.evidence_store import EVIDENCE_ROOT
        self._tmp = tempfile.TemporaryDirectory()
        self._saved_root = EVIDENCE_ROOT

    def tearDown(self) -> None:
        import assurance.evidence_store as es
        es.EVIDENCE_ROOT = self._saved_root
        es.PAPERS_DIR = self._saved_root / "papers"
        self._tmp.cleanup()

    def _use_temp_root(self) -> None:
        import assurance.evidence_store as es
        tmp_path = Path(self._tmp.name)
        es.EVIDENCE_ROOT = tmp_path
        es.PAPERS_DIR = tmp_path / "papers"

    def test_store_pdf_returns_document_id(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf
        doc_id = store_pdf(MINIMAL_PDF, source_url="https://example.com/test.pdf")
        self.assertTrue(doc_id.startswith("sha256:"))
        self.assertEqual(len(doc_id), 7 + 64)

    def test_document_id_is_deterministic(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf
        a = store_pdf(MINIMAL_PDF)
        b = store_pdf(MINIMAL_PDF)
        self.assertEqual(a, b)

    def test_store_pdf_creates_directory_layout(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf, get_document_path
        doc_id = store_pdf(MINIMAL_PDF)
        doc_path = get_document_path(doc_id)
        self.assertTrue(doc_path.is_dir())
        self.assertTrue((doc_path / "original.pdf").is_file())
        self.assertTrue((doc_path / "metadata.json").is_file())
        self.assertTrue((doc_path / "pages.jsonl").is_file())

    def test_document_exists(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import document_exists, store_pdf
        self.assertFalse(document_exists("sha256:" + "ab" * 32))
        doc_id = store_pdf(MINIMAL_PDF)
        self.assertTrue(document_exists(doc_id))

    def test_store_invalid_pdf_raises(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf
        with self.assertRaises(AssuranceError):
            store_pdf(CORRUPT_PDF)

    def test_read_metadata(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf, read_metadata
        doc_id = store_pdf(
            MINIMAL_PDF,
            source_url="https://example.com/p.pdf",
            doi="10.1234/test",
            title="Test Paper",
            authors=["A. Author"],
            year=2025,
        )
        meta = read_metadata(doc_id)
        self.assertEqual(meta["document_id"], doc_id)
        self.assertEqual(meta["title"], "Test Paper")
        self.assertEqual(meta["authors"], ["A. Author"])
        self.assertEqual(meta["year"], 2025)
        self.assertEqual(meta["source_url"], "https://example.com/p.pdf")
        self.assertEqual(meta["work_id"], "doi:10.1234/test")

    def test_read_pages_jsonl(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf, read_pages_jsonl
        doc_id = store_pdf(MINIMAL_PDF)
        pages = read_pages_jsonl(doc_id)
        self.assertEqual(len(pages), 2)
        self.assertIn("page", pages[0])
        self.assertIn("text", pages[0])
        self.assertIn("char_count", pages[0])

    def test_get_original_pdf_path(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf, get_original_pdf_path
        doc_id = store_pdf(MINIMAL_PDF)
        path = get_original_pdf_path(doc_id)
        self.assertTrue(path.is_file())
        self.assertEqual(path.name, "original.pdf")

    def test_store_source_record(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf, store_source_record
        doc_id = store_pdf(MINIMAL_PDF)
        record = load_json(SOURCE_RECORD_VALID)
        record["document_id"] = doc_id
        out = store_source_record(doc_id, record)
        self.assertTrue(out.is_file())
        self.assertEqual(out.name, "source_record.json")

    def test_list_documents(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import list_documents, store_pdf
        self.assertEqual(list_documents(), [])
        doc_id = store_pdf(MINIMAL_PDF)
        ids = list_documents()
        self.assertIn(doc_id, ids)

    def test_duplicate_store_is_idempotent(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf
        doc_id = store_pdf(MINIMAL_PDF)
        doc_id2 = store_pdf(MINIMAL_PDF)
        self.assertEqual(doc_id, doc_id2)

    def test_list_documents_finds_stored(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import list_documents, store_pdf
        from assurance.evidence_store import EVIDENCE_ROOT, PAPERS_DIR
        # Verify temp root is active
        self.assertEqual(str(EVIDENCE_ROOT), self._tmp.name)
        doc_id = store_pdf(MINIMAL_PDF)
        # Check files exist at expected path
        from assurance.evidence_store import get_document_path
        doc_path = get_document_path(doc_id)
        self.assertTrue(doc_path.is_dir(), f"doc_path missing: {doc_path}")
        self.assertTrue((doc_path / "original.pdf").is_file())
        # list_documents should find it
        ids = list_documents()
        self.assertIn(doc_id, ids, f"doc_id {doc_id} not in {ids}, PAPERS_DIR={PAPERS_DIR}")

    def test_store_pdf_without_optional_metadata(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import store_pdf, read_metadata
        doc_id = store_pdf(MINIMAL_PDF)
        meta = read_metadata(doc_id)
        self.assertEqual(meta["title"], "")
        self.assertEqual(meta["authors"], [])

    def test_read_metadata_nonexistent_raises(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import read_metadata
        with self.assertRaises(AssuranceError):
            read_metadata("sha256:" + "ab" * 32)

    def test_get_original_pdf_nonexistent_raises(self) -> None:
        self._use_temp_root()
        from assurance.evidence_store import get_original_pdf_path
        with self.assertRaises(AssuranceError):
            get_original_pdf_path("sha256:" + "ab" * 32)


# ══════════════════════════════════════════════════════════════════════════════
# Schema validation tests
# ══════════════════════════════════════════════════════════════════════════════


class SourceRecordSchemaTests(unittest.TestCase):
    """GAK-LBR-001: evidence_store.schema.json — source record validation."""

    @classmethod
    def setUpClass(cls) -> None:
        cls._schema = load_json(SCHEMA)
        cls._source_record_schema = cls._schema["$defs"]["source_record"]
        cls._metadata_schema = cls._schema["$defs"]["document_metadata"]
        cls._sr_validator = Draft202012Validator(
            cls._source_record_schema, format_checker=FormatChecker(),
        )

    def _validate_source_record(self, instance: dict) -> list[str]:
        errors = sorted(
            self._sr_validator.iter_errors(instance),
            key=lambda e: list(e.absolute_path),
        )
        return [
            f"#/{'/'.join(map(str, e.absolute_path))}: {e.message}"
            for e in errors
        ]

    def test_valid_source_record_passes(self) -> None:
        record = load_json(SOURCE_RECORD_VALID)
        errors = self._validate_source_record(record)
        self.assertEqual(errors, [], f"Unexpected errors: {errors}")

    def test_invalid_source_record_fails(self) -> None:
        record = load_json(SOURCE_RECORD_INVALID)
        errors = self._validate_source_record(record)
        self.assertGreater(len(errors), 0, "Expected validation errors")

    def test_metadata_schema_validates(self) -> None:
        """Generated metadata.json must pass schema validation."""
        saved_root = None
        saved_papers = None
        try:
            import assurance.evidence_store as es
            saved_root = es.EVIDENCE_ROOT
            saved_papers = es.PAPERS_DIR

            with tempfile.TemporaryDirectory() as td:
                es.EVIDENCE_ROOT = Path(td)
                es.PAPERS_DIR = Path(td) / "papers"

                from assurance.evidence_store import store_pdf
                doc_id = store_pdf(
                    MINIMAL_PDF,
                    doi="10.1234/test", title="Test", year=2025,
                )

                from assurance.evidence_store import read_metadata
                meta = read_metadata(doc_id)

                meta_errors = sorted(
                    Draft202012Validator(
                        self._metadata_schema, format_checker=FormatChecker(),
                    ).iter_errors(meta),
                    key=lambda e: list(e.absolute_path),
                )
                error_msgs = [
                    f"#/{'/'.join(map(str, e.absolute_path))}: {e.message}"
                    for e in meta_errors
                ]
                self.assertEqual(
                    error_msgs, [],
                    f"Metadata schema errors: {error_msgs}",
                )
        finally:
            if saved_root is not None:
                import assurance.evidence_store as es
                es.EVIDENCE_ROOT = saved_root
                es.PAPERS_DIR = saved_papers

    def test_source_record_schema_has_required_defs(self) -> None:
        schema = load_json(SCHEMA)
        self.assertIn("$defs", schema)
        self.assertIn("document_metadata", schema["$defs"])
        self.assertIn("source_record", schema["$defs"])
        self.assertIn("nonempty", schema["$defs"])

    def test_invalid_evidence_level_rejected(self) -> None:
        record = load_json(SOURCE_RECORD_VALID)
        record["evidence_level"] = "D"  # invalid
        errors = self._validate_source_record(record)
        self.assertGreater(len(errors), 0)

    def test_empty_source_id_rejected(self) -> None:
        record = load_json(SOURCE_RECORD_VALID)
        record["source_id"] = ""
        errors = self._validate_source_record(record)
        self.assertGreater(len(errors), 0)

    def test_invalid_document_id_pattern_rejected(self) -> None:
        record = load_json(SOURCE_RECORD_VALID)
        record["document_id"] = "not-a-sha256"
        errors = self._validate_source_record(record)
        self.assertGreater(len(errors), 0)


if __name__ == "__main__":
    unittest.main()
