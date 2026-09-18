"""Project document index — filesystem scanner for GSA project documentation.

Provides :class:`ProjectDocIndex`, a lightweight in-memory index of
project source code, architecture documents, audit documents, schemas,
protocols, and configuration files.  Designed for the retrieval subagent
to search project-internal documentation without network access.

No persistent index file — the index is rebuilt on each :meth:`scan`
call since project documentation changes frequently during development.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from .utils import sha256_file


# ── category definitions ──

CATEGORY_GLOBS: dict[str, list[str]] = {
    "source_code": [
        "assurance/**/*.py",
        "scripts/**/*.py",
        "runtime/**/*.py",
        "integration/**/*.py",
        "evaluation/**/*.py",
        "regression/**/*.py",
    ],
    "architecture": [
        "architecture/**/*.md",
    ],
    "audit_docs": [
        "docs/**/*.md",
    ],
    "adr": [
        "adr/**/*.md",
    ],
    "schemas": [
        "assurance/**/*.schema.json",
        "runtime/**/*.schema.json",
        "integration/**/*.schema.json",
        "evaluation/**/*.schema.json",
        "regression/**/*.schema.json",
        "upstream/**/*.schema.json",
    ],
    "protocol": [
        "protocol/**/*.yaml",
        "protocol/**/*.md",
        "protocol/**/*.schema.json",
    ],
    "regression": [
        "regression/**/*.yaml",
        "regression/**/*.md",
    ],
    "project_config": [
        "pyproject.toml",
        "pytest.ini",
        "*.md",
        ".github/**/*.yml",
    ],
}

EXCERPT_MAX_CHARS = 2000


@dataclass
class DocMatch:
    """A single document match from the project index."""

    path: str
    category: str
    title: str
    excerpt: str = ""
    sha256: str = ""
    size_bytes: int = 0


@dataclass
class _IndexEntry:
    path: str
    category: str
    title: str
    sha256: str
    size_bytes: int
    content_lower: str = field(repr=False)


class ProjectDocIndex:
    """In-memory index of project documentation files.

    Usage::

        index = ProjectDocIndex(Path("D:/CLI"))
        index.scan()
        matches = index.search("instruction provenance gate")
        for m in matches:
            content = index.read_doc(m.path)
    """

    def __init__(self, project_root: Path) -> None:
        self.project_root = project_root.resolve()
        self._entries: list[_IndexEntry] = []
        self._scanned = False

    # ── scan ──

    def scan(self) -> None:
        """Walk the project root and index all matching documentation files."""
        self._entries.clear()
        for category, globs in CATEGORY_GLOBS.items():
            seen: set[str] = set()
            for pattern in globs:
                for matched in self.project_root.glob(pattern):
                    if not matched.is_file():
                        continue
                    rel = matched.relative_to(self.project_root).as_posix()
                    if rel in seen:
                        continue
                    seen.add(rel)
                    try:
                        sha = sha256_file(matched)
                        size = matched.stat().st_size
                    except OSError:
                        sha = ""
                        size = 0
                    self._entries.append(_IndexEntry(
                        path=rel,
                        category=category,
                        title=self._derive_title(matched, rel, category),
                        sha256=sha,
                        size_bytes=size,
                        content_lower="",  # lazy-loaded on first search
                    ))
        self._scanned = True

    @staticmethod
    def _derive_title(file_path: Path, rel_path: str, category: str) -> str:
        """Derive a human-readable title from a file path."""
        stem = file_path.stem
        # Convert kebab/snake case to title
        title = stem.replace("-", " ").replace("_", " ")
        if category == "audit_docs":
            # Try to extract a more meaningful title from markdown headings
            try:
                first_lines = file_path.read_text(encoding="utf-8")[:500]
                for line in first_lines.split("\n"):
                    if line.startswith("# "):
                        return line[2:].strip()
            except Exception:
                pass
        return title

    # ── search ──

    def search(
        self,
        query: str,
        *,
        max_results: int = 20,
        categories: list[str] | None = None,
    ) -> list[DocMatch]:
        """Search the index for documents matching *query*.

        Matches against filename, derived title, and file content (first
        2000 chars).  Results are ranked by relevance heuristics.
        """
        if not self._scanned:
            self.scan()

        terms = query.lower().split()
        results: list[tuple[DocMatch, int]] = []

        allowed = set(categories) if categories else None

        for entry in self._entries:
            if allowed is not None and entry.category not in allowed:
                continue

            score = 0
            path_lower = entry.path.lower()
            title_lower = entry.title.lower()

            for term in terms:
                if term in path_lower:
                    score += 3  # path match is strong
                if term in title_lower:
                    score += 2  # title match is moderate

            # Lazy-load content for deeper search
            if score > 0 and not entry.content_lower:
                entry.content_lower = self._load_content_lower(entry.path)

            for term in terms:
                if term in entry.content_lower:
                    score += 1  # content match is supplementary

            if score > 0:
                if not entry.content_lower:
                    entry.content_lower = self._load_content_lower(entry.path)
                excerpt = entry.content_lower[:EXCERPT_MAX_CHARS]
                results.append((
                    DocMatch(
                        path=entry.path,
                        category=entry.category,
                        title=entry.title,
                        excerpt=excerpt,
                        sha256=entry.sha256,
                        size_bytes=entry.size_bytes,
                    ),
                    score,
                ))

        # Deterministic ordering: score descending, then path ascending.
        # A stable sort alone keeps filesystem-glob insertion order among
        # tied scores, so the top-K window differed between machines
        # (RS-01 2026-09-18: test_search_p3_action_authorization green on
        # the dev worktree, red on a clean CI checkout).
        results.sort(key=lambda item: (-item[1], item[0].path))
        return [doc for doc, _ in results[:max_results]]

    def _load_content_lower(self, rel_path: str) -> str:
        """Load and lowercase file content for search."""
        try:
            raw = (self.project_root / rel_path).read_text(
                encoding="utf-8", errors="replace"
            )
            return raw[:EXCERPT_MAX_CHARS].lower()
        except Exception:
            return ""

    # ── read ──

    def read_doc(self, rel_path: str) -> bytes:
        """Read the full content of a document by its relative path."""
        abs_path = self.project_root / rel_path
        if not abs_path.is_file():
            raise FileNotFoundError(f"document not found: {rel_path}")
        return abs_path.read_bytes()

    # ── stats ──

    def stats(self) -> dict[str, Any]:
        """Return index statistics: file counts by category."""
        if not self._scanned:
            self.scan()
        counts: dict[str, int] = {}
        total_size = 0
        for entry in self._entries:
            counts[entry.category] = counts.get(entry.category, 0) + 1
            total_size += entry.size_bytes
        return {
            "total_files": len(self._entries),
            "total_size_bytes": total_size,
            "by_category": dict(sorted(counts.items())),
            "categories": sorted(counts.keys()),
        }
