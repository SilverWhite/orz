from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
from pathlib import Path
import queue
import subprocess
import sys
import threading
import time
from typing import Any


ALLOWED_EXTENSION_NOTIFICATIONS = {
    "_x.ai/mcp/servers_updated",
    "_x.ai/mcp_initialized",
    "_x.ai/queue/changed",
    "_x.ai/session/prompt_complete",
    "_x.ai/session_notification",
    "_x.ai/sessions/changed",
}
ALLOWED_SESSION_NOTIFICATION_UPDATES = {
    "interaction_resolved",
    "pending_interaction",
    "session_summary_generated",
    "tool_call_delta_chunk",
    "turn_completed",
}
TERMINAL_TOOL_STATUSES = {"completed", "failed"}


def _atomic_write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _write_jsonl(path: Path, values: list[dict[str, Any]]) -> None:
    temporary = path.with_name(f".{path.name}.{time.time_ns()}.tmp")
    with temporary.open("w", encoding="utf-8", newline="\n") as handle:
        for value in values:
            handle.write(json.dumps(value, ensure_ascii=False, sort_keys=True, allow_nan=False))
            handle.write("\n")
    temporary.replace(path)


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    values: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            value = json.loads(line)
            if isinstance(value, dict):
                values.append(value)
    return values


class AcpClient:
    def __init__(
        self,
        *,
        binary: Path,
        workspace: Path,
        model: str,
        scenario: str,
        timeout_seconds: float,
    ) -> None:
        self.binary = binary
        self.workspace = workspace
        self.model = model
        self.scenario = scenario
        self.deadline = time.monotonic() + timeout_seconds
        self.transcript: list[dict[str, Any]] = []
        self.sequence = 0
        self.parse_failure_count = 0
        self.unexpected_message_count = 0
        self.extension_notification_counts = {
            method: 0 for method in sorted(ALLOWED_EXTENSION_NOTIFICATIONS)
        }
        self.session_id = ""
        self.early_session_id = ""
        self.updates: list[dict[str, Any]] = []
        self.permission_requests: list[dict[str, Any]] = []
        self.permission_outcome = ""
        self.cancel_sent = False
        self.prompt_responses: list[dict[str, Any]] = []
        self.stderr_chunks: list[str] = []
        self.stdout_queue: queue.Queue[str | None] = queue.Queue()
        self.process = subprocess.Popen(
            [str(binary), "agent", "--model", model, "stdio"],
            cwd=workspace,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            bufsize=1,
        )
        self.inherited_job_at_start = False
        if sys.platform == "win32":
            in_job = ctypes.c_int(0)
            if ctypes.windll.kernel32.IsProcessInJob(
                ctypes.c_void_p(int(self.process._handle)),  # type: ignore[attr-defined]
                ctypes.c_void_p(),
                ctypes.byref(in_job),
            ):
                self.inherited_job_at_start = bool(in_job.value)
        assert self.process.stdout is not None
        assert self.process.stderr is not None
        self.stdout_thread = threading.Thread(
            target=self._read_stdout, name="grok-acp-stdout", daemon=True
        )
        self.stderr_thread = threading.Thread(
            target=self._read_stderr, name="grok-acp-stderr", daemon=True
        )
        self.stdout_thread.start()
        self.stderr_thread.start()

    def _read_stdout(self) -> None:
        assert self.process.stdout is not None
        try:
            for line in self.process.stdout:
                self.stdout_queue.put(line.rstrip("\r\n"))
        finally:
            self.stdout_queue.put(None)

    def _read_stderr(self) -> None:
        assert self.process.stderr is not None
        for chunk in iter(lambda: self.process.stderr.read(4096), ""):
            self.stderr_chunks.append(chunk)

    def _record(self, direction: str, message: dict[str, Any]) -> None:
        self.sequence += 1
        encoded = json.dumps(
            message, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False
        ).encode("utf-8")
        self.transcript.append(
            {
                "sequence": self.sequence,
                "direction": direction,
                "message_bytes": len(encoded),
                "message_sha256": hashlib.sha256(encoded).hexdigest(),
                "message": message,
            }
        )

    def send(self, message: dict[str, Any]) -> None:
        if self.process.stdin is None or self.process.stdin.closed:
            raise RuntimeError("Grok ACP stdin is closed")
        self._record("client_to_agent", message)
        self.process.stdin.write(
            json.dumps(message, ensure_ascii=False, separators=(",", ":"), allow_nan=False)
            + "\n"
        )
        self.process.stdin.flush()

    def receive(self) -> dict[str, Any]:
        remaining = self.deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("timed out waiting for Grok ACP output")
        try:
            line = self.stdout_queue.get(timeout=remaining)
        except queue.Empty as error:
            raise TimeoutError("timed out waiting for Grok ACP output") from error
        if line is None:
            raise EOFError("Grok ACP stdout closed before the expected response")
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            self.parse_failure_count += 1
            raise ValueError(f"Grok ACP emitted non-JSON stdout: {line[:200]!r}") from error
        if not isinstance(message, dict):
            self.parse_failure_count += 1
            raise ValueError("Grok ACP emitted a non-object JSON value")
        self._record("agent_to_client", message)
        return message

    def _is_allowed_extension_notification(
        self, message: dict[str, Any]
    ) -> bool:
        method = message.get("method")
        params = message.get("params")
        if (
            method not in ALLOWED_EXTENSION_NOTIFICATIONS
            or message.get("jsonrpc") != "2.0"
            or "id" in message
            or not isinstance(params, dict)
        ):
            return False
        if method == "_x.ai/mcp/servers_updated":
            return set(params) == {"mcpServers"} and params.get("mcpServers") == []
        if method == "_x.ai/mcp_initialized":
            session_id = params.get("sessionId")
            valid = (
                set(params) == {"elapsedMs", "mcpToolCount", "sessionId"}
                and type(params.get("elapsedMs")) is int
                and params["elapsedMs"] >= 0
                and params.get("mcpToolCount") == 0
                and isinstance(session_id, str)
                and bool(session_id)
            )
            if not valid:
                return False
            if self.session_id:
                return session_id == self.session_id
            if self.early_session_id and session_id != self.early_session_id:
                return False
            self.early_session_id = session_id
            return True
        if method == "_x.ai/queue/changed":
            entries = params.get("entries")
            valid_entries = isinstance(entries, list) and all(
                isinstance(entry, dict)
                and set(entry) == {"id", "kind", "position", "text", "version"}
                and isinstance(entry.get("id"), str)
                and entry.get("kind") == "prompt"
                and type(entry.get("position")) is int
                and isinstance(entry.get("text"), str)
                and type(entry.get("version")) is int
                for entry in entries
            )
            keys_valid = set(params).issubset(
                {
                    "entries",
                    "runningKind",
                    "runningPromptId",
                    "runningText",
                    "sessionId",
                }
            )
            running_metadata_valid = (
                "runningKind" not in params
                and "runningText" not in params
            ) or (
                params.get("runningKind") == "prompt"
                and isinstance(params.get("runningText"), str)
                and isinstance(params.get("runningPromptId"), str)
            )
            return (
                keys_valid
                and {"entries", "sessionId"}.issubset(params)
                and valid_entries
                and params.get("sessionId") == self.session_id
                and running_metadata_valid
                and (
                    "runningPromptId" not in params
                    or isinstance(params.get("runningPromptId"), str)
                )
            )
        if method == "_x.ai/session/prompt_complete":
            return (
                set(params)
                == {"agentResult", "promptId", "sessionId", "stopReason"}
                and params.get("agentResult") is None
                and isinstance(params.get("promptId"), str)
                and params.get("sessionId") == self.session_id
                and params.get("stopReason") in {"end_turn", "cancelled"}
            )
        if method == "_x.ai/session_notification":
            update = params.get("update")
            if not (
                set(params).issubset({"_meta", "sessionId", "update"})
                and {"sessionId", "update"}.issubset(params)
                and params.get("sessionId") == self.session_id
                and isinstance(update, dict)
                and update.get("sessionUpdate")
                in ALLOWED_SESSION_NOTIFICATION_UPDATES
                and (
                    "_meta" not in params
                    or isinstance(params.get("_meta"), dict)
                )
            ):
                return False
            update_type = update["sessionUpdate"]
            if update_type == "tool_call_delta_chunk":
                return (
                    set(update)
                    == {
                        "arguments_delta",
                        "name",
                        "sessionUpdate",
                        "tool_call_id",
                        "tool_index",
                    }
                    and isinstance(update.get("arguments_delta"), str)
                    and update.get("name") == "read_file"
                    and isinstance(update.get("tool_call_id"), str)
                    and type(update.get("tool_index")) is int
                )
            if update_type == "pending_interaction":
                return (
                    set(update)
                    == {"kind", "sessionUpdate", "tool_call_id"}
                    and update.get("kind") == "permission"
                    and isinstance(update.get("tool_call_id"), str)
                )
            if update_type == "interaction_resolved":
                return (
                    set(update) == {"sessionUpdate", "tool_call_id"}
                    and isinstance(update.get("tool_call_id"), str)
                )
            if update_type == "session_summary_generated":
                return (
                    set(update) == {"sessionUpdate", "session_summary"}
                    and isinstance(update.get("session_summary"), str)
                )
            return (
                set(update)
                == {"prompt_id", "sessionUpdate", "stop_reason", "usage"}
                and isinstance(update.get("prompt_id"), str)
                and update.get("stop_reason") in {"end_turn", "cancelled"}
                and isinstance(update.get("usage"), dict)
            )
        if method == "_x.ai/sessions/changed":
            upserted = params.get("upserted")
            return (
                set(params) == {"removed", "upserted"}
                and params.get("removed") == []
                and isinstance(upserted, list)
                and len(upserted) == 1
                and isinstance(upserted[0], dict)
                and set(upserted[0])
                == {
                    "activity",
                    "cwd",
                    "isWorktree",
                    "lastChangeUnixMs",
                    "modelId",
                    "origin",
                    "resident",
                    "sessionId",
                    "title",
                    "yolo",
                }
                and upserted[0].get("activity") in {"working", "idle"}
                and upserted[0].get("sessionId") == self.session_id
            )
        return False

    def _handle_agent_message(self, message: dict[str, Any]) -> None:
        method = message.get("method")
        if method == "session/update":
            params = message.get("params")
            update = params.get("update") if isinstance(params, dict) else None
            if isinstance(update, dict):
                self.updates.append(update)
                return
        elif method == "session/request_permission" and "id" in message:
            self.permission_requests.append(message)
            self._answer_permission(message)
            return
        elif self._is_allowed_extension_notification(message):
            self.extension_notification_counts[str(method)] += 1
            return
        elif method is None and "id" in message:
            return
        self.unexpected_message_count += 1

    def request(self, request_id: str, method: str, params: dict[str, Any]) -> dict[str, Any]:
        self.send(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "method": method,
                "params": params,
            }
        )
        while True:
            message = self.receive()
            self._handle_agent_message(message)
            if str(message.get("id")) == request_id and "method" not in message:
                if "error" in message:
                    raise RuntimeError(f"{method} returned JSON-RPC error: {message['error']}")
                result = message.get("result")
                if not isinstance(result, dict):
                    raise RuntimeError(f"{method} did not return an object result")
                return result

    def _answer_permission(self, request: dict[str, Any]) -> None:
        params = request.get("params")
        options = params.get("options", []) if isinstance(params, dict) else []
        request_id = request["id"]
        if self.scenario == "allow_once":
            allow = next(
                (
                    option
                    for option in options
                    if isinstance(option, dict) and option.get("kind") == "allow_once"
                ),
                None,
            )
            if allow is None or not isinstance(allow.get("optionId"), str):
                raise RuntimeError("permission request did not offer an allow_once option")
            self.permission_outcome = "allow_once"
            self.send(
                {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "outcome": {
                            "outcome": "selected",
                            "optionId": allow["optionId"],
                        }
                    },
                }
            )
            return
        session_id = params.get("sessionId") if isinstance(params, dict) else None
        if not isinstance(session_id, str) or not session_id:
            raise RuntimeError("permission request did not contain a sessionId")
        self.send(
            {
                "jsonrpc": "2.0",
                "method": "session/cancel",
                "params": {"sessionId": session_id},
            }
        )
        self.cancel_sent = True
        self.permission_outcome = "cancelled"
        self.send(
            {
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {"outcome": {"outcome": "cancelled"}},
            }
        )

    def prompt(self, session_id: str, text: str) -> dict[str, Any]:
        self.send(
            {
                "jsonrpc": "2.0",
                "id": "acp-prompt-1",
                "method": "session/prompt",
                "params": {
                    "sessionId": session_id,
                    "prompt": [{"type": "text", "text": text}],
                },
            }
        )
        while True:
            message = self.receive()
            self._handle_agent_message(message)
            if str(message.get("id")) == "acp-prompt-1" and "method" not in message:
                self.prompt_responses.append(message)
                if "error" in message:
                    raise RuntimeError(
                        f"session/prompt returned JSON-RPC error: {message['error']}"
                    )
                result = message.get("result")
                if not isinstance(result, dict):
                    raise RuntimeError("session/prompt did not return an object result")
                return result

    def close_input(self) -> None:
        if self.process.stdin is not None and not self.process.stdin.closed:
            self.process.stdin.close()

    def projection(self, session_id: str, prompt_result: dict[str, Any]) -> dict[str, Any]:
        tool_calls = [
            update for update in self.updates if update.get("sessionUpdate") == "tool_call"
        ]
        tool_updates = [
            update
            for update in self.updates
            if update.get("sessionUpdate") == "tool_call_update"
        ]
        tool_ids = list(
            dict.fromkeys(
                update["toolCallId"]
                for update in tool_calls + tool_updates
                if isinstance(update.get("toolCallId"), str)
            )
        )
        statuses = [
            update["status"].lower()
            for update in tool_updates
            if isinstance(update.get("status"), str)
        ]
        option_kinds = sorted(
            {
                option["kind"]
                for request in self.permission_requests
                for option in (
                    request.get("params", {}).get("options", [])
                    if isinstance(request.get("params"), dict)
                    else []
                )
                if isinstance(option, dict)
                and option.get("kind")
                in {"allow_once", "allow_always", "reject_once", "reject_always"}
            }
        )
        return {
            "protocol_version": 1,
            "session_id": session_id,
            "request_methods": ["initialize", "session/new", "session/prompt"],
            "update_types": [
                update["sessionUpdate"]
                for update in self.updates
                if isinstance(update.get("sessionUpdate"), str)
            ],
            "tool_call_id": tool_ids[0] if len(tool_ids) == 1 else "",
            "tool_call_count": len(tool_calls),
            "tool_update_count": len(tool_updates),
            "terminal_tool_update_count": sum(
                status in TERMINAL_TOOL_STATUSES for status in statuses
            ),
            "tool_statuses": statuses,
            "permission_request_count": len(self.permission_requests),
            "permission_option_kinds": option_kinds,
            "permission_outcome": self.permission_outcome,
            "cancel_notification_sent": self.cancel_sent,
            "prompt_response_count": len(self.prompt_responses),
            "prompt_stop_reason": prompt_result.get("stopReason", ""),
            "extension_notification_counts": self.extension_notification_counts,
            "unexpected_message_count": self.unexpected_message_count,
            "parse_failure_count": self.parse_failure_count,
        }


def _find_session_files(profile_root: Path, session_id: str) -> tuple[Path, Path]:
    sessions_root = profile_root / ".grok" / "sessions"
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline:
        candidates = [
            path
            for path in sessions_root.rglob(session_id)
            if path.is_dir() and path.name == session_id
        ] if sessions_root.is_dir() else []
        if len(candidates) == 1:
            events = candidates[0] / "events.jsonl"
            updates = candidates[0] / "updates.jsonl"
            if events.is_file() and updates.is_file():
                return events, updates
        time.sleep(0.1)
    raise RuntimeError(f"could not uniquely discover session files for {session_id}")


def _session_projection(events_path: Path, updates_path: Path) -> dict[str, Any]:
    events = _read_jsonl(events_path)
    update_rows = _read_jsonl(updates_path)
    event_types = [
        row["type"] for row in events if isinstance(row.get("type"), str)
    ]
    updates: list[dict[str, Any]] = []
    for row in update_rows:
        params = row.get("params")
        update = params.get("update") if isinstance(params, dict) else None
        if isinstance(update, dict):
            updates.append(update)
    return {
        "discovered": True,
        "event_types": event_types,
        "update_types": [
            update["sessionUpdate"]
            for update in updates
            if isinstance(update.get("sessionUpdate"), str)
        ],
        "tool_call_ids": sorted(
            {
                update["toolCallId"]
                for update in updates
                if isinstance(update.get("toolCallId"), str)
            }
        ),
        "permission_decisions": [
            row["decision"]
            for row in events
            if row.get("type") == "permission_resolved"
            and isinstance(row.get("decision"), str)
        ],
        "completed_tool_event_count": event_types.count("tool_completed"),
    }


def _wait_for_session_projection(
    events_path: Path,
    updates_path: Path,
    *,
    scenario: str,
    tool_call_id: str,
    timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    deadline = time.monotonic() + timeout_seconds
    last_fingerprint: tuple[int, int, int, int] | None = None
    stable_since: float | None = None
    latest: dict[str, Any] | None = None
    while time.monotonic() < deadline:
        latest = _session_projection(events_path, updates_path)
        expected_decision = "allow" if scenario == "allow_once" else "cancelled"
        sufficient = (
            tool_call_id in latest["tool_call_ids"]
            and latest["permission_decisions"] == [expected_decision]
            and (
                latest["completed_tool_event_count"] == 1
                if scenario == "allow_once"
                else latest["completed_tool_event_count"] == 0
            )
        )
        event_stat = events_path.stat()
        update_stat = updates_path.stat()
        fingerprint = (
            event_stat.st_size,
            event_stat.st_mtime_ns,
            update_stat.st_size,
            update_stat.st_mtime_ns,
        )
        now = time.monotonic()
        if sufficient and fingerprint == last_fingerprint:
            if stable_since is not None and now - stable_since >= 0.25:
                return latest
        else:
            stable_since = now if sufficient else None
        last_fingerprint = fingerprint
        time.sleep(0.05)
    raise RuntimeError(
        "session evidence did not settle with the expected tool and permission state"
    )


def run(args: argparse.Namespace) -> dict[str, Any]:
    output = args.output_directory.resolve()
    if output.exists():
        raise ValueError(f"refusing to overwrite client output: {output}")
    output.mkdir(parents=True)
    transcript_path = output / "transcript.private.jsonl"
    stderr_path = output / "grok.stderr.log"
    result_path = output / "client-result.json"
    prompt = args.prompt_file.resolve().read_text(encoding="utf-8").strip()
    if not prompt:
        raise ValueError("prompt fixture is empty")
    if args.ready_path or args.start_signal:
        if not args.ready_path or not args.start_signal:
            raise ValueError("--ready-path and --start-signal must be supplied together")
        ready_path = args.ready_path.resolve()
        start_signal = args.start_signal.resolve()
        if ready_path.exists() or start_signal.exists():
            raise ValueError("ready and start-signal paths must not already exist")
        _atomic_write_json(
            ready_path,
            {
                "schema_version": "0.1.0",
                "ready": True,
                "grok_started": False,
            },
        )
        deadline = time.monotonic() + args.timeout_seconds
        while not start_signal.is_file():
            if time.monotonic() >= deadline:
                raise TimeoutError("timed out waiting for the parent start signal")
            time.sleep(0.05)
    client = AcpClient(
        binary=args.binary.resolve(),
        workspace=args.workspace.resolve(),
        model=args.model,
        scenario=args.scenario,
        timeout_seconds=args.timeout_seconds,
    )
    try:
        initialize = client.request(
            "acp-init-1",
            "initialize",
            {
                "protocolVersion": 1,
                "clientCapabilities": {},
                "clientInfo": {
                    "name": "lif-acp-fake-tool-probe",
                    "title": "LIF ACP Fake Tool Probe",
                    "version": "0.1.0",
                },
            },
        )
        if initialize.get("protocolVersion") != 1:
            raise RuntimeError("Grok did not agree to ACP protocol version 1")
        session = client.request(
            "acp-session-1",
            "session/new",
            {"cwd": str(args.workspace.resolve()), "mcpServers": []},
        )
        session_id = session.get("sessionId")
        if not isinstance(session_id, str) or not session_id:
            raise RuntimeError("session/new did not return a non-empty sessionId")
        if client.early_session_id and client.early_session_id != session_id:
            raise RuntimeError(
                "early MCP initialization sessionId does not match session/new"
            )
        client.session_id = session_id
        prompt_result = client.prompt(session_id, prompt)
        client.close_input()
        events_path, updates_path = _find_session_files(
            args.profile_root.resolve(), session_id
        )
        acp = client.projection(session_id, prompt_result)
        session_evidence = _wait_for_session_projection(
            events_path,
            updates_path,
            scenario=args.scenario,
            tool_call_id=acp["tool_call_id"],
        )
        result = {
            "schema_version": "0.1.0",
            "scenario": args.scenario,
            "acp": acp,
            "session_evidence": session_evidence,
            "session_events_path": str(events_path.resolve()),
            "session_updates_path": str(updates_path.resolve()),
            "grok_process": {
                "pid": client.process.pid,
                "inherited_job_at_start": client.inherited_job_at_start,
                "still_running_after_prompt": client.process.poll() is None,
            },
        }
        _write_jsonl(transcript_path, client.transcript)
        stderr_path.write_text("".join(client.stderr_chunks), encoding="utf-8")
        _atomic_write_json(result_path, result)
        return result
    finally:
        client.close_input()


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Drive one fixed fake-only Grok ACP tool lifecycle"
    )
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--profile-root", type=Path, required=True)
    parser.add_argument("--prompt-file", type=Path, required=True)
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--model", default="lif-fake-deepseek")
    parser.add_argument(
        "--scenario", choices=("allow_once", "cancel_permission"), required=True
    )
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--ready-path", type=Path)
    parser.add_argument("--start-signal", type=Path)
    args = parser.parse_args()
    try:
        result = run(args)
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
