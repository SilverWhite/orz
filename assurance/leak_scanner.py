"""Leak Scanner — dual-channel information leakage detection.

Implements the requirement from :file:`architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4:

    Mechanical check + independent semantic review dual-channel.
    Evaluation/holdout scenarios fail-closed.
    Does NOT rely on model judgment for "is this a leak."

Extends beyond :mod:`~.credential_scrub` (credential-level) to cross-scenario
information leakage: internal paths, hostnames, code fingerprints, schema
fragments, and structural anomalies.

**Dual-channel design:**

1. **Mechanical channel** — regex/pattern/heuristic scans for known leak
   signatures.  Deterministic, fast, no model dependency.
2. **Semantic review channel** — structural analysis (entropy, density,
   fingerprint matching against known-sensitive corpora).  Also
   deterministic — no LLM call.

**Modes** (from :file:`protocol/reason-codes-v0.1.yaml`):

- ``discussion`` — all findings are warnings, never block.
- ``guarded`` — high-severity findings block; medium/low are warnings.
- ``strict`` — any finding blocks (evaluation/holdout default).
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable

from .errors import AssuranceError


# ══════════════════════════════════════════════════════════════════════════════
# data types
# ══════════════════════════════════════════════════════════════════════════════


class Severity(Enum):
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


class ScanCategory(Enum):
    CREDENTIAL = "credential"
    INTERNAL_PATH = "internal_path"
    INTERNAL_HOST = "internal_host"
    CODE_FINGERPRINT = "code_fingerprint"
    SCHEMA_FRAGMENT = "schema_fragment"
    STRUCTURAL_ANOMALY = "structural_anomaly"
    PROMPT_LEAKAGE = "prompt_leakage"


class ScanMode(Enum):
    DISCUSSION = "discussion"
    GUARDED = "guarded"
    STRICT = "strict"


@dataclass(frozen=True)
class LeakFinding:
    """A single leakage detection result."""

    category: ScanCategory
    severity: Severity
    location: str           # human-readable location (e.g. "line 42", "field $.answer")
    snippet_hash: str       # SHA-256 of the matched snippet (never the content itself)
    rule_id: str            # which rule triggered
    evidence: str           # why this was flagged (never contains the raw matched text)
    channel: str = "mechanical"  # "mechanical" | "semantic"


@dataclass
class ScanResult:
    """Aggregate scan result with findings and mode-based verdict."""

    findings: list[LeakFinding] = field(default_factory=list)
    channel_mechanical_count: int = 0
    channel_semantic_count: int = 0
    blocked: bool = False
    mode: ScanMode = ScanMode.GUARDED
    notes: list[str] = field(default_factory=list)


# ══════════════════════════════════════════════════════════════════════════════
# mechanical channel — rule-based scanners
# ══════════════════════════════════════════════════════════════════════════════


def _hash_snippet(text: str) -> str:
    """Return SHA-256 of *text* for evidence recording (never log raw text)."""
    import hashlib
    return hashlib.sha256(text.encode("utf-8", errors="replace")).hexdigest()


# ── credential patterns (extended from credential_scrub.py) ──────────────

# API key / token patterns
_CREDENTIAL_PATTERNS: list[tuple[str, str, Severity]] = [
    # OpenAI-style keys
    (r"sk-[A-Za-z0-9]{32,}", "openai_api_key_format", Severity.CRITICAL),
    # Anthropic-style keys
    (r"sk-ant-[A-Za-z0-9_-]{32,}", "anthropic_api_key_format", Severity.CRITICAL),
    # Generic bearer tokens (long base64-like strings after "Bearer")
    (r"Bearer\s+([A-Za-z0-9+/=_-]{40,})", "bearer_token", Severity.CRITICAL),
    # GitHub personal access tokens
    (r"gh[pousr]_[A-Za-z0-9]{36,}", "github_pat", Severity.CRITICAL),
    # AWS access keys
    (r"AKIA[0-9A-Z]{16}", "aws_access_key", Severity.CRITICAL),
    # Generic API key assignment (heuristic)
    (r'(?:api[_-]?key|apikey|secret[_-]?key)\s*[:=]\s*["\']?([A-Za-z0-9+/=_-]{20,})["\']?',
     "api_key_assignment", Severity.CRITICAL),
    # JWT tokens (three base64url segments separated by dots)
    (r"eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+", "jwt_token", Severity.HIGH),
]

# ── internal path patterns ───────────────────────────────────────────────

_INTERNAL_PATH_PATTERNS: list[tuple[str, str, Severity]] = [
    # Windows user profile paths
    (r"C:\\Users\\[^\\\s]{3,}", "windows_user_profile", Severity.MEDIUM),
    # Unix home directory paths
    (r"/home/[^/\s]{3,}", "unix_home_directory", Severity.MEDIUM),
    # Internal project paths (heuristic: long absolute paths with source directories)
    (r"(?:/[^\s]{2,}){3,}/(?:src|lib|include|internal|private)/[^\s]+",
     "internal_source_path", Severity.MEDIUM),
    # Temp directory paths with content
    (r"(?:/tmp|\\Temp|C:\\Windows\\Temp)/[^\s]{8,}", "temp_path_with_content", Severity.LOW),
]

# ── internal host / IP patterns ──────────────────────────────────────────

_INTERNAL_HOST_PATTERNS: list[tuple[str, str, Severity]] = [
    # Private IPv4 addresses
    (r"\b(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3})\b",
     "private_ipv4", Severity.MEDIUM),
    # Localhost with port
    (r"localhost:\d{2,5}", "localhost_with_port", Severity.LOW),
    # Internal hostnames (heuristic: .internal, .local, .corp TLDs)
    (r"\b[\w-]+\.(?:internal|local|corp|lan)\b", "internal_hostname", Severity.MEDIUM),
]

# ── code fingerprint patterns ────────────────────────────────────────────

_CODE_FINGERPRINT_PATTERNS: list[tuple[str, str, Severity]] = [
    # Python tracebacks with local paths
    (r'File\s+"[^"]*\.py",\s+line\s+\d+,\s+in\s+\w+', "python_traceback", Severity.MEDIUM),
    # SQL CREATE TABLE statements (schema leakage)
    (r"CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?`?\w+`?\s*\([^)]{20,}\)",
     "sql_create_table", Severity.HIGH),
    # Database connection strings
    (r"(?:mysql|postgres(?:ql)?|mongodb|sqlite|redis)://[^\s]{10,}",
     "db_connection_string", Severity.CRITICAL),
    # Private key headers (PEM format)
    (r"-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----", "private_key_pem", Severity.CRITICAL),
]

# ── prompt leakage patterns ──────────────────────────────────────────────

_PROMPT_LEAKAGE_PATTERNS: list[tuple[str, str, Severity]] = [
    # System prompt / instruction fragments commonly regurgitated
    (r"(?:You are|You're)\s+(?:an?\s+)?(?:AI|agent|assistant|helpful|Claude|GPT)",
     "system_persona_leak", Severity.MEDIUM),
    # Tool definition fragments
    (r'"name":\s*"[a-z_]+",\s*"description":\s*"[^"]{20,}"',
     "tool_definition_fragment", Severity.MEDIUM),
    # Markdown code fence with excessive internal detail
    (r"```(?:python|json|yaml|sql)\s*\n[^\n]{50,}\n```",
     "large_code_block", Severity.LOW),
]


# ══════════════════════════════════════════════════════════════════════════════
# semantic review channel — structural heuristics
# ══════════════════════════════════════════════════════════════════════════════


def _entropy(data: str) -> float:
    """Estimate Shannon entropy of *data* (bits per character).

    High entropy may indicate encoded/encrypted content or keys.
    Low entropy may indicate structured data leakage.
    """
    if not data:
        return 0.0
    from collections import Counter
    from math import log2
    counts = Counter(data)
    length = len(data)
    return -sum((c / length) * log2(c / length) for c in counts.values())


def _detect_structural_anomalies(text: str) -> list[LeakFinding]:
    """Detect structural anomalies without model judgment.

    Checks:
    - Entropy spikes (segments with unusually high/low entropy vs. surrounding)
    - Base64 density (many base64-like substrings in prose context)
    - Path density (unusual concentration of filesystem paths)
    """
    findings: list[LeakFinding] = []

    # Split into lines for per-line analysis
    lines = text.split("\n")

    # Entropy anomaly: lines with very high entropy (>5.5 bits/char) that
    # are long (>40 chars) may indicate embedded keys/tokens in prose.
    for i, line in enumerate(lines):
        if len(line) >= 40:
            ent = _entropy(line)
            if ent > 5.5:
                findings.append(LeakFinding(
                    category=ScanCategory.STRUCTURAL_ANOMALY,
                    severity=Severity.MEDIUM,
                    location=f"line {i + 1}",
                    snippet_hash=_hash_snippet(line[:60]),
                    rule_id="high_entropy_line",
                    evidence=f"entropy {ent:.2f} bits/char on {len(line)}-char line",
                    channel="semantic",
                ))

    # Base64 density: count base64-like substrings (>=24 chars of [A-Za-z0-9+/=_-])
    _b64_re = re.compile(r"[A-Za-z0-9+/=_-]{24,}")
    b64_matches = _b64_re.findall(text)
    if len(b64_matches) >= 5:
        findings.append(LeakFinding(
            category=ScanCategory.STRUCTURAL_ANOMALY,
            severity=Severity.LOW,
            location="global",
            snippet_hash=_hash_snippet("".join(m[:20] for m in b64_matches[:3])),
            rule_id="base64_density",
            evidence=f"{len(b64_matches)} base64-like substrings found",
            channel="semantic",
        ))

    # Path density: count filesystem paths (both Unix and Windows)
    _path_re = re.compile(
        r"(?:/(?:[A-Za-z0-9._-]+/){2,}[A-Za-z0-9._/-]+|"
        r"[A-Z]:\\(?:[A-Za-z0-9._-]+\\){2,}[A-Za-z0-9._\\-]+)"
    )
    path_matches = _path_re.findall(text)
    if len(path_matches) >= 10:
        findings.append(LeakFinding(
            category=ScanCategory.STRUCTURAL_ANOMALY,
            severity=Severity.MEDIUM,
            location="global",
            snippet_hash=_hash_snippet("".join(path_matches[:3])),
            rule_id="path_density",
            evidence=f"{len(path_matches)} filesystem paths found",
            channel="semantic",
        ))

    return findings


def _detect_fingerprint_match(
    text: str,
    *,
    known_fingerprints: list[str],
) -> list[LeakFinding]:
    """Check whether *text* contains substrings matching known-sensitive
    fingerprints.

    This is a content-blind check: *known_fingerprints* are SHA-256 hashes
    of sensitive n-grams, NOT the sensitive content itself.  We hash n-grams
    from *text* and check for collisions.
    """
    if not known_fingerprints:
        return []

    findings: list[LeakFinding] = []
    fp_set = set(known_fingerprints)

    # Use 5-gram sliding window for fingerprint matching
    words = text.split()
    for i in range(len(words) - 4):
        ngram = " ".join(words[i:i + 5])
        fp = _hash_snippet(ngram)
        if fp in fp_set:
            findings.append(LeakFinding(
                category=ScanCategory.CODE_FINGERPRINT,
                severity=Severity.HIGH,
                location=f"word range [{i}, {i + 4}]",
                snippet_hash=fp,
                rule_id="known_fingerprint_match",
                evidence=f"5-gram fingerprint matched at position {i}",
                channel="semantic",
            ))
            if len(findings) >= 10:
                break  # don't flood — 10 matches is enough

    return findings


# ══════════════════════════════════════════════════════════════════════════════
# LeakScanner
# ══════════════════════════════════════════════════════════════════════════════


class LeakScanner:
    """Dual-channel information leakage scanner.

    Parameters
    ----------
    mode:
        Scanning mode — controls whether findings block or warn.
    known_fingerprints:
        Optional list of SHA-256 hashes of known-sensitive 5-grams.
        Used by the semantic review channel for fingerprint matching.
        These are hashes of sensitive content, NOT the content itself.

    Usage::

        scanner = LeakScanner(mode=ScanMode.STRICT)
        result = scanner.scan(output_text)
        if result.blocked:
            raise AssuranceError("leak detected: " + result.notes[0])
    """

    def __init__(
        self,
        *,
        mode: ScanMode = ScanMode.GUARDED,
        known_fingerprints: list[str] | None = None,
    ) -> None:
        self._mode = mode
        self._known_fingerprints = known_fingerprints or []

    @property
    def mode(self) -> ScanMode:
        return self._mode

    # ── public API ────────────────────────────────────────────────────────

    def scan(self, text: str, *, label: str = "output") -> ScanResult:
        """Run both channels against *text* and return a :class:`ScanResult`.

        Parameters
        ----------
        text:
            The text to scan for information leakage.
        label:
            Human-readable label for the scanned artifact (used in notes).
        """
        result = ScanResult(mode=self._mode)

        # ── mechanical channel ──
        mech_findings = self._run_mechanical_scans(text)
        result.findings.extend(mech_findings)
        result.channel_mechanical_count = len(mech_findings)

        # ── semantic channel ──
        sem_findings = self._run_semantic_scans(text)
        result.findings.extend(sem_findings)
        result.channel_semantic_count = len(sem_findings)

        # ── mode-based verdict ──
        result.blocked = self._compute_blocked(result.findings)

        if result.findings:
            cats = {f.category.value for f in result.findings}
            sevs = {f.severity.value for f in result.findings}
            result.notes.append(
                f"[{label}] {len(result.findings)} finding(s) "
                f"(mechanical={result.channel_mechanical_count}, "
                f"semantic={result.channel_semantic_count}) | "
                f"categories={cats} | severities={sevs} | "
                f"blocked={result.blocked}"
            )
        else:
            result.notes.append(f"[{label}] clean — no leaks detected")

        return result

    def scan_dict(
        self,
        data: dict[str, Any],
        *,
        label: str = "artifact",
    ) -> ScanResult:
        """Scan a JSON-serializable dict for information leakage.

        Serializes *data* to JSON and scans the resulting text.
        Individual string values are also scanned separately for
        finer-grained location reporting.
        """
        import json

        # Full-text scan
        full_text = json.dumps(data, ensure_ascii=False, indent=2)
        result = self.scan(full_text, label=label)

        # Per-value scan for better locations
        self._scan_values(data, "$", result)

        return result

    # ── internal: mechanical channel ──────────────────────────────────────

    def _run_mechanical_scans(self, text: str) -> list[LeakFinding]:
        findings: list[LeakFinding] = []

        scanners: list[tuple[list[tuple[str, str, Severity]], ScanCategory]] = [
            (_CREDENTIAL_PATTERNS, ScanCategory.CREDENTIAL),
            (_INTERNAL_PATH_PATTERNS, ScanCategory.INTERNAL_PATH),
            (_INTERNAL_HOST_PATTERNS, ScanCategory.INTERNAL_HOST),
            (_CODE_FINGERPRINT_PATTERNS, ScanCategory.CODE_FINGERPRINT),
            (_PROMPT_LEAKAGE_PATTERNS, ScanCategory.PROMPT_LEAKAGE),
        ]

        for patterns, category in scanners:
            for pattern, rule_id, severity in patterns:
                for match in re.finditer(pattern, text, re.IGNORECASE | re.MULTILINE):
                    # Compute line number for location
                    line_no = text[:match.start()].count("\n") + 1
                    matched = match.group(0)
                    findings.append(LeakFinding(
                        category=category,
                        severity=severity,
                        location=f"line {line_no}",
                        snippet_hash=_hash_snippet(matched),
                        rule_id=rule_id,
                        evidence=f"pattern '{rule_id}' matched ({len(matched)} chars)",
                        channel="mechanical",
                    ))

        return findings

    # ── internal: semantic channel ────────────────────────────────────────

    def _run_semantic_scans(self, text: str) -> list[LeakFinding]:
        findings: list[LeakFinding] = []

        # Structural anomaly detection (entropy, density)
        findings.extend(_detect_structural_anomalies(text))

        # Fingerprint matching against known-sensitive corpora
        findings.extend(
            _detect_fingerprint_match(
                text, known_fingerprints=self._known_fingerprints,
            )
        )

        return findings

    # ── internal: per-value scanning ──────────────────────────────────────

    def _scan_values(
        self,
        obj: object,
        path: str,
        result: ScanResult,
    ) -> None:
        """Recursively scan individual string values for better locations."""
        if isinstance(obj, dict):
            for key, value in obj.items():  # type: ignore[attr-defined]
                child_path = f"{path}.{key}"
                if isinstance(value, str) and len(value) >= 20:
                    for pattern, rule_id, severity in _CREDENTIAL_PATTERNS:
                        if re.search(pattern, value, re.IGNORECASE):
                            result.findings.append(LeakFinding(
                                category=ScanCategory.CREDENTIAL,
                                severity=severity,
                                location=child_path,
                                snippet_hash=_hash_snippet(value[:60]),
                                rule_id=rule_id,
                                evidence=f"credential pattern '{rule_id}' in field",
                                channel="mechanical",
                            ))
                            break
                elif isinstance(value, (dict, list)):
                    self._scan_values(value, child_path, result)
        elif isinstance(obj, list):
            for idx, item in enumerate(obj):  # type: ignore[attr-defined]
                child_path = f"{path}[{idx}]"
                if isinstance(item, (dict, list)):
                    self._scan_values(item, child_path, result)

    # ── internal: verdict ─────────────────────────────────────────────────

    def _compute_blocked(self, findings: list[LeakFinding]) -> bool:
        """Determine whether findings should block based on mode."""
        if not findings:
            return False

        if self._mode == ScanMode.DISCUSSION:
            return False  # never block in discussion mode

        if self._mode == ScanMode.STRICT:
            return True   # any finding blocks in strict mode

        # GUARDED: block on HIGH or CRITICAL severity
        for f in findings:
            if f.severity in (Severity.HIGH, Severity.CRITICAL):
                return True
        return False


# ══════════════════════════════════════════════════════════════════════════════
# convenience functions
# ══════════════════════════════════════════════════════════════════════════════


def scan_for_leaks(
    text: str,
    *,
    mode: ScanMode = ScanMode.GUARDED,
    label: str = "output",
    known_fingerprints: list[str] | None = None,
) -> ScanResult:
    """Convenience function: scan *text* with a default :class:`LeakScanner`."""
    scanner = LeakScanner(mode=mode, known_fingerprints=known_fingerprints)
    return scanner.scan(text, label=label)


def assert_no_leaks(
    text: str,
    *,
    mode: ScanMode = ScanMode.STRICT,
    label: str = "output",
    known_fingerprints: list[str] | None = None,
) -> ScanResult:
    """Scan *text* and raise :class:`AssuranceError` if leaks are found.

    Defaults to ``STRICT`` mode (evaluation/holdout default: fail-closed).
    Returns the :class:`ScanResult` on success.
    """
    scanner = LeakScanner(mode=mode, known_fingerprints=known_fingerprints)
    result = scanner.scan(text, label=label)
    if result.blocked:
        cat_summary = {}
        for f in result.findings:
            cat_summary[f.category.value] = cat_summary.get(f.category.value, 0) + 1
        raise AssuranceError(
            f"leak scan blocked '{label}': "
            f"{len(result.findings)} finding(s) across "
            f"{len(cat_summary)} categories — {cat_summary}"
        )
    return result
