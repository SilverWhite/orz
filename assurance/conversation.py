from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import Any

from .contracts import ASSURANCE_ROOT, validate_contract
from .envelope import create_security_envelope, new_conversation_id
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_bytes,
    atomic_write_json,
    exclusive_create_bytes,
    is_link_or_reparse,
    load_json,
    require_no_linked_ancestors,
    require_within,
    safe_relative_path,
    sha256_bytes,
    utc_now,
)


MARKER_NAME = ".assurance-conversation.json"
STATE_NAME = "conversation-state.json"
ARTIFACTS_DIR = "artifacts"
RECEIPTS_DIR = "receipts"

PERSIST_CATEGORIES = {
    "redacted_conversation",
    "user_pinned_snapshot",
    "terminal_deletion_receipt",
}


def retention_delete_categories() -> tuple[str, ...]:
    policy = load_json(ASSURANCE_ROOT / "retention-policy-v0.1.json")
    validate_contract(
        policy,
        "retention-policy-v0.1.schema.json",
        label="conversation retention policy",
    )
    return tuple(policy["delete_on_archive"])


class ConversationNamespace:
    """One explicit filesystem namespace for one active conversation."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.marker_path = root / MARKER_NAME
        self.state_path = root / STATE_NAME
        self.artifacts_root = root / ARTIFACTS_DIR
        self.receipts_root = root / RECEIPTS_DIR
        if not root.is_dir() or is_link_or_reparse(root):
            raise AssuranceError(
                "conversation namespace must be a non-linked directory"
            )
        for control_path in (self.marker_path, self.state_path):
            require_within(control_path, root, must_exist=True)
            require_no_linked_ancestors(control_path, root)
            if not control_path.is_file():
                raise AssuranceError("conversation control files must be regular files")
        marker = load_json(self.marker_path)
        if set(marker) != {
            "schema_version",
            "marker_kind",
            "conversation_id",
            "created_at",
        }:
            raise AssuranceError("conversation marker shape is invalid")
        if (
            marker["schema_version"] != "0.1.0-draft"
            or marker["marker_kind"] != "assurance_conversation_namespace"
        ):
            raise AssuranceError("conversation marker identity is invalid")
        self.conversation_id = marker["conversation_id"]
        state = self.state()
        if state["conversation_id"] != self.conversation_id:
            raise AssuranceError("conversation state does not match namespace marker")

    @classmethod
    def create(
        cls,
        repository_root: Path,
        *,
        key_store: InstallationKeyStore,
        frozen_context: dict[str, Any],
        allowed_capabilities: list[str],
        denied_capabilities: list[str],
        parent_envelope_id: str | None = None,
        resumed_from_conversation_id: str | None = None,
        now: datetime | None = None,
    ) -> ConversationNamespace:
        repository_root.mkdir(parents=True, exist_ok=True)
        if is_link_or_reparse(repository_root):
            raise AssuranceError("conversation repository cannot be a link or reparse point")
        conversation_id = new_conversation_id()
        root = repository_root / conversation_id
        if root.exists():
            raise AssuranceError("refusing to reuse a conversation namespace")
        envelope = create_security_envelope(
            key_store=key_store,
            conversation_id=conversation_id,
            frozen_context=frozen_context,
            allowed_capabilities=allowed_capabilities,
            denied_capabilities=denied_capabilities,
            parent_envelope_id=parent_envelope_id,
            resumed_from_conversation_id=resumed_from_conversation_id,
            now=now,
        )
        root.mkdir()
        marker = {
            "schema_version": "0.1.0-draft",
            "marker_kind": "assurance_conversation_namespace",
            "conversation_id": conversation_id,
            "created_at": utc_now(),
        }
        atomic_write_json(root / MARKER_NAME, marker)
        namespace = cls.__new__(cls)
        namespace.root = root
        namespace.marker_path = root / MARKER_NAME
        namespace.state_path = root / STATE_NAME
        namespace.artifacts_root = root / ARTIFACTS_DIR
        namespace.receipts_root = root / RECEIPTS_DIR
        namespace.conversation_id = conversation_id

        envelope_path = (
            namespace.artifacts_root
            / "active_security_envelope"
            / f"{envelope['envelope_id']}.json"
        )
        atomic_write_json(envelope_path, envelope)
        state = {
            "schema_version": "0.1.0-draft",
            "state_kind": "conversation_namespace_state",
            "conversation_id": conversation_id,
            "envelope_id": envelope["envelope_id"],
            "state": "active",
            "revision": 0,
            "updated_at": utc_now(),
            "terminal_deletion_receipt_sha256": None,
        }
        namespace._write_state(state, overwrite=False)
        namespace.receipts_root.mkdir(parents=True, exist_ok=True)
        return cls(root)

    def state(self) -> dict[str, Any]:
        require_within(self.state_path, self.root, must_exist=True)
        require_no_linked_ancestors(self.state_path, self.root)
        if not self.state_path.is_file():
            raise AssuranceError("conversation state must be a regular file")
        state = load_json(self.state_path)
        validate_contract(
            state,
            "conversation-state-v0.1.schema.json",
            label="conversation state",
        )
        return state

    def _write_state(self, state: dict[str, Any], *, overwrite: bool) -> None:
        validate_contract(
            state,
            "conversation-state-v0.1.schema.json",
            label="conversation state",
        )
        require_no_linked_ancestors(self.state_path, self.root)
        atomic_write_json(self.state_path, state, overwrite=overwrite)

    def envelope_path(self) -> Path:
        state = self.state()
        return (
            self.artifacts_root
            / "active_security_envelope"
            / f"{state['envelope_id']}.json"
        )

    def load_active_envelope(self) -> dict[str, Any]:
        state = self.state()
        if state["state"] != "active":
            raise AssuranceError("conversation has no active security envelope")
        return load_json(self.envelope_path())

    def _artifact_path(self, category: str, relative_path: str) -> Path:
        allowed_categories = set(retention_delete_categories()) | PERSIST_CATEGORIES
        if category not in allowed_categories:
            raise AssuranceError(f"unknown retention category: {category}")
        relative = safe_relative_path(relative_path)
        path = self.artifacts_root / category / Path(*relative.parts)
        if self.root.exists():
            require_within(path, self.root, must_exist=False)
            require_no_linked_ancestors(path, self.root)
        return path

    def write_artifact(self, category: str, relative_path: str, value: bytes) -> Path:
        if self.state()["state"] != "active":
            raise AssuranceError("artifacts can only be written to an active conversation")
        if category == "active_security_envelope":
            raise AssuranceError("active security envelope is kernel-owned")
        path = self._artifact_path(category, relative_path)
        atomic_write_bytes(path, value)
        return path

    def claim_artifact(self, category: str, relative_path: str, value: bytes) -> Path:
        """Create a fail-closed, non-overwritable lifecycle claim."""
        if self.state()["state"] != "active":
            raise AssuranceError("artifacts can only be claimed by an active conversation")
        if category == "active_security_envelope":
            raise AssuranceError("active security envelope is kernel-owned")
        path = self._artifact_path(category, relative_path)
        exclusive_create_bytes(path, value)
        return path

    def read_artifact(
        self, requested_conversation_id: str, category: str, relative_path: str
    ) -> bytes:
        if requested_conversation_id != self.conversation_id:
            raise AssuranceError("cross-conversation read denied by default")
        path = self._artifact_path(category, relative_path)
        require_within(path, self.root, must_exist=True)
        if is_link_or_reparse(path) or not path.is_file():
            raise AssuranceError("artifact must be a regular non-linked file")
        return path.read_bytes()

    def import_untrusted_snapshot(
        self,
        *,
        source: ConversationNamespace,
        source_category: str,
        source_path: str,
        destination_name: str,
    ) -> dict[str, Any]:
        payload = source.read_artifact(
            source.conversation_id, source_category, source_path
        )
        destination = self.write_artifact(
            "unpinned_snapshot", destination_name, payload
        )
        receipt = {
            "schema_version": "0.1.0-draft",
            "import_status": "imported_untrusted",
            "source_conversation_id": source.conversation_id,
            "destination_conversation_id": self.conversation_id,
            "payload_sha256": sha256_bytes(payload),
            "destination": destination.relative_to(self.root).as_posix(),
            "created_at": utc_now(),
        }
        receipt_name = (
            f"imported-untrusted-{sha256_bytes(payload)[:16]}.json"
        )
        atomic_write_json(
            self.artifacts_root / "temporary_import_receipt" / receipt_name,
            receipt,
        )
        return receipt
