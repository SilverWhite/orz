from __future__ import annotations

from copy import deepcopy
import json
import os
from pathlib import Path
import sys
import threading
from typing import Any, Callable, Protocol, TextIO

from .errors import PrototypeError
from .external_network import OneShotNetworkPermit
from .io_utils import canonical_bytes, sha256_bytes, utc_now
from .layout import RUNTIME_ROOT
from .network_broker import (
    confirmation_summary_sha256,
    render_network_confirmation,
)
from .schema import validate_instance


class ConfirmationIO(Protocol):
    def confirm(self, *, rendered_summary: str, expected_token: str) -> bool:
        ...


class ConsoleConfirmationIO:
    """Minimal terminal confirmation UI; summaries go to stderr, never JSON stdout."""

    def __init__(
        self,
        *,
        input_func: Callable[[], str] | None = None,
        output: TextIO | None = None,
    ) -> None:
        self._input = input_func or input
        self._output = output or sys.stderr

    def confirm(self, *, rendered_summary: str, expected_token: str) -> bool:
        print(rendered_summary, file=self._output)
        print(
            f"Type {expected_token} to allow this fake-provider attempt; any other input denies:",
            file=self._output,
        )
        self._output.flush()
        try:
            answer = self._input()
        except (EOFError, KeyboardInterrupt):
            return False
        return answer.strip() == expected_token


class ApprovalLedger:
    """Single-writer append-only approval decision ledger."""

    def __init__(self, path: Path) -> None:
        if path.exists():
            raise PrototypeError(f"refusing to reuse existing approval ledger: {path}")
        self.path = path
        self._sequence = 0
        self._previous_sha256: str | None = None
        self._failed = False
        self._lock = threading.Lock()

    def append(
        self,
        *,
        summary: dict[str, Any],
        decision: str,
    ) -> dict[str, Any]:
        if decision not in {"allow", "deny"}:
            raise PrototypeError("approval ledger decision must be allow or deny")
        summary_sha256 = confirmation_summary_sha256(summary)
        with self._lock:
            if self._failed:
                raise PrototypeError("approval ledger writer is poisoned after an earlier failure")
            sequence = self._sequence + 1
            core = {
                "schema_version": "0.1.0-prototype",
                "event_type": "network_approval_decision",
                "sequence": sequence,
                "recorded_at": utc_now(),
                "previous_event_sha256": self._previous_sha256,
                "provider": summary["provider"],
                "endpoint": summary["endpoint"],
                "turn": summary["turn"],
                "attempt": summary["attempt"],
                "request_body_sha256": summary["request_body_sha256"],
                "confirmation_summary_sha256": summary_sha256,
                "decision": decision,
                "authority": "local-interactive-user",
                "confirmation_method": "typed-summary-digest",
                "raw_message_content_recorded": False,
                "raw_reasoning_recorded": False,
                "authorization_recorded": False,
                "confirmation_token_recorded": False,
            }
            event = {
                **core,
                "event_sha256": sha256_bytes(canonical_bytes(core)),
            }
            validate_instance(
                event,
                RUNTIME_ROOT / "network-approval-event-v0.1.schema.json",
                label="network approval event",
            )
            encoded = json.dumps(
                event,
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
                allow_nan=False,
            ).encode("utf-8") + b"\n"
            self.path.parent.mkdir(parents=True, exist_ok=True)
            mode = "xb" if sequence == 1 else "ab"
            try:
                with self.path.open(mode) as handle:
                    handle.write(encoded)
                    handle.flush()
                    os.fsync(handle.fileno())
            except OSError as exc:
                self._failed = True
                raise PrototypeError(
                    f"cannot append network approval ledger ({type(exc).__name__})"
                ) from None
            self._sequence = sequence
            self._previous_sha256 = event["event_sha256"]
            return deepcopy(event)


class InteractivePermitBroker:
    """Digest-challenge broker restricted to the in-process fake provider."""

    fake_provider_only = True

    def __init__(self, *, ledger: ApprovalLedger, confirmation_io: ConfirmationIO) -> None:
        self._ledger = ledger
        self._confirmation_io = confirmation_io
        self._summaries: list[dict[str, Any]] = []

    def authorize(self, summary: dict[str, Any]) -> OneShotNetworkPermit:
        summary_sha256 = confirmation_summary_sha256(summary)
        expected_token = "ALLOW-" + summary_sha256[:12].upper()
        rendered = (
            "FAKE PROVIDER DRY RUN — no network, no billing\n"
            + render_network_confirmation(summary)
        )
        self._summaries.append(deepcopy(summary))
        allowed = self._confirmation_io.confirm(
            rendered_summary=rendered,
            expected_token=expected_token,
        )
        self._ledger.append(
            summary=summary,
            decision="allow" if allowed else "deny",
        )
        if not allowed:
            raise PrototypeError("interactive network confirmation denied the request")
        return OneShotNetworkPermit.issue_for_deepseek_chat(
            request_sha256=summary["request_body_sha256"]
        )

    @property
    def summaries(self) -> list[dict[str, Any]]:
        return deepcopy(self._summaries)


def verify_approval_ledger(path: Path) -> dict[str, Any]:
    errors: list[str] = []
    events: list[dict[str, Any]] = []
    previous_sha256: str | None = None
    try:
        with path.open("r", encoding="utf-8") as handle:
            for line_number, line in enumerate(handle, start=1):
                if not line.strip():
                    errors.append(f"blank approval ledger line {line_number}")
                    continue
                try:
                    event = json.loads(
                        line,
                        parse_constant=lambda value: (_ for _ in ()).throw(
                            ValueError(f"non-standard JSON constant {value}")
                        ),
                    )
                    validate_instance(
                        event,
                        RUNTIME_ROOT / "network-approval-event-v0.1.schema.json",
                        label=f"network approval event line {line_number}",
                    )
                except (json.JSONDecodeError, ValueError, PrototypeError) as exc:
                    errors.append(f"invalid approval ledger line {line_number}: {exc}")
                    continue
                if event["sequence"] != line_number:
                    errors.append(f"approval ledger sequence mismatch at line {line_number}")
                if event["previous_event_sha256"] != previous_sha256:
                    errors.append(f"approval ledger chain mismatch at line {line_number}")
                core = {key: value for key, value in event.items() if key != "event_sha256"}
                expected = sha256_bytes(canonical_bytes(core))
                if event["event_sha256"] != expected:
                    errors.append(f"approval ledger digest mismatch at line {line_number}")
                previous_sha256 = event["event_sha256"]
                events.append(event)
    except OSError as exc:
        errors.append(f"cannot read approval ledger: {type(exc).__name__}")
    return {
        "schema_version": "0.1.0-prototype",
        "valid": not errors and bool(events),
        "event_count": len(events),
        "allow_count": sum(event["decision"] == "allow" for event in events),
        "deny_count": sum(event["decision"] == "deny" for event in events),
        "terminal_event_sha256": previous_sha256,
        "events": events,
        "errors": sorted(set(errors)),
        "limitations": [
            "The ledger proves local hash-chain mechanics, not user identity or informed consent."
        ],
    }
