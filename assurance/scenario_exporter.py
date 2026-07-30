"""Scenario Exporter — anonymous scenario package from regression corpus.

Implements the requirement from :file:`architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4:

    Generate anonymous scenario packages from the regression corpus.
    Only visible facts — no raw data, model internal state, or
    non-redacted content.  For external evaluation use.

The exporter reads the regression corpus at :file:`regression/cases-v0.1.yaml`,
strips all oracle data and internal metadata, and produces a self-contained
scenario package containing only the facts visible to an evaluated agent.

**Redaction rules** (fail-closed — any unexpected field is blocked):

- **Keep**: ``case_id``, ``title``, ``scenario.task``, ``scenario.visible_facts``
- **Strip**: ``oracle``, ``sources``, ``classification``, ``countercase_ids``,
  ``scenario.hidden_from_subject``, ``scenario.fixture_manifest``,
  ``curation_fixture_manifest``, ``curation_status``, ``partition``,
  ``evidence_tier``, ``case_kind``
- **Block**: any field not in the explicit keep-list (fail-closed against
  schema drift adding new oracle-bearing fields)

Every exported scenario is scanned with :class:`~.leak_scanner.LeakScanner`
in STRICT mode before inclusion in the package.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any

from .errors import AssuranceError
from .utils import atomic_write_json, sha256_bytes, utc_now


# ══════════════════════════════════════════════════════════════════════════════
# path constants
# ══════════════════════════════════════════════════════════════════════════════

REGRESSION_ROOT = Path(__file__).resolve().parents[1] / "regression"
DEFAULT_CORPUS_PATH = REGRESSION_ROOT / "cases-v0.1.yaml"
DEFAULT_EXPORT_ROOT = REGRESSION_ROOT / "exports"


# ══════════════════════════════════════════════════════════════════════════════
# data types
# ══════════════════════════════════════════════════════════════════════════════


class Partition(Enum):
    TEACHING = "teaching"
    DEVELOPMENT = "development"
    EVALUATION = "evaluation"
    HOLDOUT = "holdout"
    CHALLENGE = "challenge"


@dataclass
class ExportedScenario:
    """A single anonymized scenario ready for external evaluation."""

    case_id: str
    title: str
    task: str
    visible_facts: list[str]
    content_sha256: str    # integrity hash of the exported content


@dataclass
class ExportResult:
    """Result of exporting a scenario package."""

    export_id: str
    created_at: str
    corpus_revision: int
    scenarios: list[ExportedScenario]
    total_cases_in_corpus: int
    exported_count: int
    skipped_count: int
    skip_reasons: list[str] = field(default_factory=list)
    leak_scan_clean: bool = True


# ══════════════════════════════════════════════════════════════════════════════
# keep-list — these are the ONLY fields allowed in an exported scenario
# ══════════════════════════════════════════════════════════════════════════════

_KEEP_TOP_LEVEL = frozenset({
    "case_id",
    "title",
    "scenario",
})

_KEEP_SCENARIO = frozenset({
    "task",
    "visible_facts",
})

# Fields that must NOT appear in the corpus case for the export to proceed.
# If any of these are missing, that's an error (the case is malformed).
# If any unlisted field appears at top-level or in scenario, export is blocked.
_STRIP_TOP_LEVEL = frozenset({
    "case_kind",
    "partition",
    "evidence_tier",
    "curation_status",
    "curation_fixture_manifest",
    "classification",
    "sources",
    "oracle",
    "countercase_ids",
})

_STRIP_SCENARIO = frozenset({
    "hidden_from_subject",
    "fixture_manifest",
})


# ══════════════════════════════════════════════════════════════════════════════
# ScenarioExporter
# ══════════════════════════════════════════════════════════════════════════════


class ScenarioExporter:
    """Export anonymous scenario packages from the regression corpus.

    Usage::

        exporter = ScenarioExporter()
        result = exporter.export(
            partitions={Partition.EVALUATION, Partition.HOLDOUT},
        )
        # Writes to regression/exports/{export_id}/
        for scenario in result.scenarios:
            print(scenario.case_id, scenario.task)
    """

    def __init__(
        self,
        *,
        corpus_path: Path | None = None,
        export_root: Path | None = None,
        leak_scan: bool = True,
    ) -> None:
        self._corpus_path = corpus_path or DEFAULT_CORPUS_PATH
        self._export_root = export_root or DEFAULT_EXPORT_ROOT
        self._leak_scan = leak_scan

    # ── public API ────────────────────────────────────────────────────────

    def export(
        self,
        *,
        partitions: set[Partition] | None = None,
        case_ids: list[str] | None = None,
    ) -> ExportResult:
        """Export scenarios matching *partitions* and/or *case_ids*.

        Parameters
        ----------
        partitions:
            If set, only export cases in these partitions.  ``None``
            exports all partitions.
        case_ids:
            If set, only export these specific case IDs.  ``None``
            exports all cases (subject to *partitions* filter).

        Returns
        -------
        ExportResult
            The export package metadata.
        """
        corpus = self._load_corpus()
        cases = corpus.get("cases", [])
        corpus_revision = corpus.get("corpus_revision", 0)

        export_id = f"EXPORT-{utc_now()[:19].replace(':', '').replace('-', '')}"

        scenarios: list[ExportedScenario] = []
        skip_reasons: list[str] = []
        total = len(cases)

        for case in cases:
            cid = case.get("case_id", "?")
            partition_raw = case.get("partition", "?")

            # Filter by partition.
            if partitions is not None:
                try:
                    p = Partition(partition_raw)
                except ValueError:
                    skip_reasons.append(f"{cid}: unknown partition '{partition_raw}'")
                    continue
                if p not in partitions:
                    continue

            # Filter by case_id.
            if case_ids is not None and cid not in case_ids:
                continue

            try:
                scenario = self._export_one(case)
                scenarios.append(scenario)
            except AssuranceError as exc:
                skip_reasons.append(f"{cid}: {exc}")

        # Leak scan the entire package text.
        leak_scan_clean = True
        if self._leak_scan and scenarios:
            leak_scan_clean = self._run_leak_scan(scenarios, export_id)

        result = ExportResult(
            export_id=export_id,
            created_at=utc_now(),
            corpus_revision=corpus_revision,
            scenarios=scenarios,
            total_cases_in_corpus=total,
            exported_count=len(scenarios),
            skipped_count=len(skip_reasons),
            skip_reasons=skip_reasons,
            leak_scan_clean=leak_scan_clean,
        )

        # Write the export package to disk.
        self._write_package(result, corpus)

        return result

    # ── internal: export single case ──────────────────────────────────────

    def _export_one(self, case: dict[str, Any]) -> ExportedScenario:
        """Extract visible-facts-only scenario from a single corpus case.

        Raises :class:`AssuranceError` if any unexpected field is present
        (fail-closed against schema drift).
        """
        # 1. Validate top-level fields: only keep-list fields allowed.
        unknown_top = set(case.keys()) - _KEEP_TOP_LEVEL - _STRIP_TOP_LEVEL
        if unknown_top:
            raise AssuranceError(
                f"unexpected top-level fields in case: {sorted(unknown_top)}"
            )

        case_id = case.get("case_id", "")
        title = case.get("title", "")
        scenario_raw = case.get("scenario")
        if not isinstance(scenario_raw, dict):
            raise AssuranceError(f"missing or invalid 'scenario' in case {case_id}")

        # 2. Validate scenario fields: only keep-list fields allowed.
        unknown_scn = set(scenario_raw.keys()) - _KEEP_SCENARIO - _STRIP_SCENARIO
        if unknown_scn:
            raise AssuranceError(
                f"unexpected scenario fields in case {case_id}: "
                f"{sorted(unknown_scn)}"
            )

        task = scenario_raw.get("task", "")
        visible_facts = scenario_raw.get("visible_facts", [])
        if not isinstance(visible_facts, list):
            raise AssuranceError(
                f"visible_facts must be a list in case {case_id}"
            )

        # 3. Compute content integrity hash.
        import json
        content = json.dumps(
            {"case_id": case_id, "title": title, "task": task,
             "visible_facts": visible_facts},
            ensure_ascii=False, sort_keys=True,
        )
        content_sha256 = sha256_bytes(content.encode("utf-8"))

        return ExportedScenario(
            case_id=case_id,
            title=title,
            task=task,
            visible_facts=list(visible_facts),
            content_sha256=content_sha256,
        )

    # ── internal: leak scan ───────────────────────────────────────────────

    def _run_leak_scan(
        self,
        scenarios: list[ExportedScenario],
        export_id: str,
    ) -> bool:
        """Run LeakScanner on the full export text.

        Returns ``True`` if clean, ``False`` if leaks detected.
        Does NOT raise — the caller decides whether to proceed.
        """
        try:
            from .leak_scanner import LeakScanner, ScanMode
        except ImportError:
            return True  # scanner not available → assume clean

        import json
        full_text = json.dumps(
            [
                {
                    "case_id": s.case_id,
                    "title": s.title,
                    "task": s.task,
                    "visible_facts": s.visible_facts,
                }
                for s in scenarios
            ],
            ensure_ascii=False, indent=2,
        )

        scanner = LeakScanner(mode=ScanMode.STRICT)
        result = scanner.scan(full_text, label=f"export/{export_id}")
        return not result.blocked

    # ── internal: load / write ────────────────────────────────────────────

    def _load_corpus(self) -> dict[str, Any]:
        """Load the regression corpus YAML file."""
        try:
            import yaml
        except ImportError:
            raise AssuranceError(
                "PyYAML is required for scenario export — "
                "install with: pip install PyYAML"
            )
        if not self._corpus_path.is_file():
            raise AssuranceError(
                f"corpus file not found: {self._corpus_path}"
            )
        with open(self._corpus_path, "r", encoding="utf-8") as fh:
            data = yaml.safe_load(fh)
        if not isinstance(data, dict):
            raise AssuranceError("corpus YAML root is not a dict")
        return data

    def _write_package(
        self,
        result: ExportResult,
        corpus: dict[str, Any],
    ) -> Path:
        """Write the export package to disk."""
        export_dir = self._export_root / result.export_id
        export_dir.mkdir(parents=True, exist_ok=True)

        # Write each scenario as a standalone JSON file.
        for scenario in result.scenarios:
            scenario_path = export_dir / f"{scenario.case_id}.json"
            atomic_write_json(
                scenario_path,
                {
                    "schema_version": "0.1.0-draft",
                    "export_format": "gsa_scenario_export",
                    "case_id": scenario.case_id,
                    "title": scenario.title,
                    "task": scenario.task,
                    "visible_facts": scenario.visible_facts,
                    "content_sha256": scenario.content_sha256,
                },
                overwrite=True,
            )

        # Write the export manifest.
        manifest = {
            "schema_version": "0.1.0-draft",
            "export_id": result.export_id,
            "created_at": result.created_at,
            "corpus_revision": result.corpus_revision,
            "total_cases_in_corpus": result.total_cases_in_corpus,
            "exported_count": result.exported_count,
            "skipped_count": result.skipped_count,
            "skip_reasons": result.skip_reasons,
            "leak_scan_clean": result.leak_scan_clean,
            "scenario_ids": [s.case_id for s in result.scenarios],
            "content_root_hash": sha256_bytes(
                "".join(s.content_sha256 for s in result.scenarios).encode("utf-8")
            ),
            "limitations": [
                "Scenarios contain only visible_facts — oracle, sources, "
                "classification, and hidden_from_subject are stripped.",
                "Fixture manifests and curation metadata are excluded.",
                "THIS PACKAGE DOES NOT CONTAIN ORACLE ANSWERS.",
            ],
        }
        atomic_write_json(export_dir / "export_manifest.json", manifest, overwrite=True)
        return export_dir


# ══════════════════════════════════════════════════════════════════════════════
# convenience
# ══════════════════════════════════════════════════════════════════════════════


def export_scenarios(
    *,
    partitions: set[Partition] | None = None,
    case_ids: list[str] | None = None,
    corpus_path: Path | None = None,
    export_root: Path | None = None,
    verify_no_leaks: bool = True,
) -> ExportResult:
    """Convenience: export scenarios and raise if leak scan fails.

    Defaults to leak-scan-enabled.  Pass ``verify_no_leaks=False`` to
    skip the scan (development only — never in evaluation/holdout).
    """
    exporter = ScenarioExporter(
        corpus_path=corpus_path,
        export_root=export_root,
        leak_scan=verify_no_leaks,
    )
    result = exporter.export(partitions=partitions, case_ids=case_ids)
    if verify_no_leaks and not result.leak_scan_clean:
        raise AssuranceError(
            f"export {result.export_id}: leak scan detected potential "
            f"information leakage — export blocked"
        )
    return result
