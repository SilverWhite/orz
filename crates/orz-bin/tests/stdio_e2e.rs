//! E2E: `orz --stdio` ACP server over real JSON-RPC frames.
//!
//! Drives the orz binary with a bare pipe client (the same wire shape a real
//! ACP client uses): newline-delimited JSON-RPC 2.0. Asserts the response
//! frames and the resulting hash-chained events.jsonl.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

fn spawn_stdio(cwd: &std::path::Path) -> Child {
    Command::new(env!("CARGO_BIN_EXE_orz"))
        .arg("--stdio")
        .current_dir(cwd)
        // ACAF production flip (2026-08-16): fail-closed is the default;
        // this wire-shape E2E does not exercise the fabric, so it runs in
        // explicit shadow mode.
        .env("ORZ_ACAF_FAIL_CLOSED", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn orz --stdio")
}

/// Send one request frame, read the matching response frame.
///
/// Interleaved server notifications (e.g. `session/update` streamed text
/// deltas — streaming slice) are collected into `notifications` and skipped,
/// the way a real ACP client consumes them.
fn roundtrip(
    writer: &mut impl Write,
    reader: &mut impl BufRead,
    id: u64,
    method: &str,
    params: serde_json::Value,
    notifications: &mut Vec<serde_json::Value>,
) -> serde_json::Value {
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    });
    writeln!(writer, "{frame}").unwrap();
    writer.flush().unwrap();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("read response frame");
        let response: serde_json::Value =
            serde_json::from_str(&line).expect("parse response frame");
        if response.get("id") == Some(&serde_json::Value::from(id)) {
            return response;
        }
        // A frame without our id is a server notification — collect and skip.
        notifications.push(response);
    }
}

#[test]
fn session_new_and_prompt_over_real_frames() {
    let dir = std::env::temp_dir().join(format!(
        "orz-stdio-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    // A clean directory (no repo-controlled trust-sensitive config) passes
    // the folder-trust gate; config-carrying dirs are the fail-closed case.

    let mut child = spawn_stdio(&dir);
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut writer = stdin;
    let mut reader = BufReader::new(stdout);

    // 1. initialize
    let mut notifications = Vec::new();
    let init = roundtrip(
        &mut writer,
        &mut reader,
        1,
        "initialize",
        serde_json::json!({"protocolVersion": 1, "clientCapabilities": {}}),
        &mut notifications,
    );
    assert_eq!(init["result"]["protocolVersion"], 1);

    // 2. session/new — the agent generates the session id.
    let created = roundtrip(
        &mut writer,
        &mut reader,
        2,
        "session/new",
        serde_json::json!({"cwd": dir, "mcpServers": []}),
        &mut notifications,
    );
    let session_id = created["result"]["sessionId"]
        .as_str()
        .expect("sessionId in result")
        .to_string();
    assert!(!session_id.is_empty());

    // 3. session/prompt
    let prompted = roundtrip(
        &mut writer,
        &mut reader,
        3,
        "session/prompt",
        serde_json::json!({
            "sessionId": session_id,
            "prompt": [{"type": "text", "text": "hello world"}],
        }),
        &mut notifications,
    );
    assert_eq!(prompted["result"]["stopReason"], "end_turn");

    // Streaming slice: the host must have streamed text-delta notifications
    // over the wire before the prompt response.
    assert!(
        notifications.iter().any(|n| n["method"] == "session/update"
            && n["params"]["update"]["sessionUpdate"] == "agent_message_chunk"),
        "expected streamed text-delta notifications, got: {notifications:?}"
    );

    // Close stdin → agent sees EOF → io future completes → process exits.
    drop(writer);
    let status = child.wait().expect("wait for orz");
    assert!(status.success(), "orz exited with {status}");

    // The ACP path must have produced exactly one run journal with a valid chain.
    let runs_dir = dir.join(".gsa").join("runs");
    let run_dirs: Vec<_> = std::fs::read_dir(&runs_dir)
        .expect("runs dir")
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(run_dirs.len(), 1, "expected exactly one run journal");
    let events_path = run_dirs[0].join("events.jsonl");

    let replay = orz_assurance::replay_journal(&events_path, None, None, true);
    assert!(
        replay.valid,
        "stdio path journal invalid: {:?}",
        replay.errors
    );
    // preflight + started + prompt_submitted + tool_availability
    // + request_header_change (ORZ-CACHE-CONTEXT-COST 2026-08-15)
    // + 3 plan-round model_output (no plan_write from the canned gateway —
    //     PLAN-FIRST 阶段 A degrade cap) + plan_write (plan_not_submitted)
    // + model_output + counterexample_gate + model_output + stagnation
    // + finished (GAP-INQUIRY-SPLIT: no per-turn orientation event — the
    // orientation producer fires only on the session-level 7-round trigger)
    assert_eq!(replay.event_count, 14);
    assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

    let _ = std::fs::remove_dir_all(&dir);
}
