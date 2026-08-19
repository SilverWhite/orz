//! Live probe — FIX_PLAN 2026-08-06 item ① (max 档实测, independent first).
//!
//! Measures the real DeepSeek V4 surface under the production thinking
//! config: `thinking: {type: "enabled"}` + `reasoning_effort: "max"` +
//! `max_tokens: 32_000` (OUTPUT-DEGENERATION-GUARD 2026-08-19, ADR-0010
//! §14.33: single-round budget 160K → 32K; previously 160_000) — latency,
//! convergence, content behavior (empty content on tool rounds is legal),
//! tool-round protocol (reasoning_content replay), and
//! `usage.reasoning_tokens` (raw JSON — the fork's typed `CompletionUsage`
//! drops unknown fields).
//!
//! TWO probes:
//!   1. `probe_thinking_max_tool_task`  — file-based multi-round tool task
//!      (read ×3 → write → final answer) mirroring the polyglot benchmark
//!      shape. Exercises the full tool-round protocol with replay.
//!   2. `probe_thinking_max_streaming_ttft` — streaming TTFT: time to first
//!      reasoning delta vs first content delta. Calibrates the D-7 stream
//!      idle watchdog (5s warn / 50s hard abort, STREAM-RETRY-RHYTHM
//!      2026-08-20).
//!
//! Both are `#[ignore]` + double-gated on `ORZ_TEST_LIVE=1` and read the
//! ADR-0006 Windows Credential Manager key — same convention as the existing
//! `live_chat_completion_roundtrip`. Raw reqwest (dev-dep) for full JSON
//! visibility; this is a probe, not production code. Rounds are capped and
//! each HTTP call is timeout-guarded (thinking max may legitimately take
//! minutes on a hard task; the task here is deliberately small).
//!
//! Run: `ORZ_TEST_LIVE=1 cargo test -p orz-loop --lib -- --ignored --nocapture probe_thinking_max`

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::gateway::credentials::read_agent_api_key;
use crate::gateway::transport::DEFAULT_DEEPSEEK_API_BASE;

const MODEL: &str = "deepseek-v4-flash";
const ROUND_CAP: usize = 6;
/// Per-request wall-clock guard (thinking max on the 32K single-round
/// budget is still slow by design — this only catches true hangs).
const ROUND_TIMEOUT: Duration = Duration::from_secs(20 * 60);

fn gate() -> Option<String> {
    if std::env::var("ORZ_TEST_LIVE").as_deref() != Ok("1") {
        return None;
    }
    match read_agent_api_key() {
        Ok(key) => Some(key),
        Err(e) => {
            eprintln!("live probe skipped: {e}");
            None
        }
    }
}

fn request_body(messages: &[Value], tools: bool, max_tokens: u32, stream: bool) -> Value {
    let mut body = json!({
        "model": MODEL,
        "messages": messages,
        "max_tokens": max_tokens,
        "thinking": {"type": "enabled"},
        "reasoning_effort": "max",
        "stream": stream,
    });
    if tools {
        body["tools"] = json!([
            {"type": "function", "function": {
                "name": "read_file",
                "description": "Read a UTF-8 text file inside the probe data directory.",
                "parameters": {"type": "object", "properties": {
                    "path": {"type": "string", "description": "File name, e.g. a.txt"}
                }, "required": ["path"]}
            }},
            {"type": "function", "function": {
                "name": "write_file",
                "description": "Write a UTF-8 text file inside the probe data directory.",
                "parameters": {"type": "object", "properties": {
                    "path": {"type": "string", "description": "File name, e.g. total.txt"},
                    "content": {"type": "string", "description": "File content"}
                }, "required": ["path", "content"]}
            }}
        ]);
    }
    body
}

/// Serialize one response into the replay-ready assistant message.
/// `reasoning_content` is kept RAW (including `""` — empty string is normal
/// on tool rounds; empty-object/missing would be a 400 replay, D-6).
fn to_replay_message(msg: &Value) -> Value {
    let mut m = json!({"role": "assistant"});
    let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
    if !content.is_empty() {
        m["content"] = json!(content);
    }
    if let Some(rc) = msg.get("reasoning_content")
        && rc.is_string()
    {
        m["reasoning_content"] = rc.clone();
    }
    if let Some(tc) = msg.get("tool_calls")
        && tc.is_array()
    {
        m["tool_calls"] = tc.clone();
    }
    m
}

fn summarize(msg: &Value) -> String {
    let content = msg.get("content").and_then(|c| c.as_str()).unwrap_or("");
    let reasoning = msg
        .get("reasoning_content")
        .and_then(|c| c.as_str())
        .unwrap_or("<absent>");
    format!(
        "content={} chars reasoning={} chars",
        content.chars().count(),
        reasoning.chars().count()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::timeout;

    #[tokio::test]
    #[ignore = "live probe — set ORZ_TEST_LIVE=1 (API key from Windows Credential Manager, ADR-0006)"]
    async fn probe_thinking_max_tool_task() {
        let Some(key) = gate() else { return };

        // ── fixture: three small files with numbers; expected total 37 ─────
        let dir =
            std::env::temp_dir().join(format!("orz-probe-thinking-max-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "3\n5\n").unwrap();
        std::fs::write(dir.join("b.txt"), "7\n2\n10\n").unwrap();
        std::fs::write(dir.join("c.txt"), "4\n6\n").unwrap();

        let client = reqwest::Client::new();
        let url = format!("{DEFAULT_DEEPSEEK_API_BASE}/chat/completions");

        let system = "You are a precise file-processing assistant. Use the read_file and \
             write_file tools (files live in a directory on your side; paths are \
             plain file names like a.txt). After writing the answer file, reply \
             with the final total as plain text.";
        let user = "Read the three files a.txt, b.txt, c.txt. Add up ALL numbers found \
             in them. Write the total to total.txt. Then reply with the final \
             total as plain text.";
        let mut messages = vec![
            json!({"role": "system", "content": system}),
            json!({"role": "user", "content": user}),
        ];

        println!("── probe_thinking_max_tool_task ──");
        println!(
            "model={MODEL} thinking=enabled effort=max max_tokens=32000 task=read×3+sum+write"
        );
        let t0 = Instant::now();
        let mut rounds = 0usize;
        loop {
            let body = request_body(&messages, true, 32_000, false);
            let round_start = Instant::now();
            let resp = match timeout(
                ROUND_TIMEOUT,
                client.post(&url).bearer_auth(&key).json(&body).send(),
            )
            .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    println!("round={rounds} NETWORK ERROR: {e}");
                    break;
                }
                Err(_) => {
                    println!("round={rounds} TIMEOUT after {ROUND_TIMEOUT:?}");
                    break;
                }
            };
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                println!("round={rounds} HTTP {status}: {}", truncate(&text, 400));
                break;
            }
            let v: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
                println!("round={rounds} JSON PARSE ERROR: {e}");
                json!({})
            });
            let choice = v["choices"][0].clone();
            let msg = choice["message"].clone();
            let usage = &v["usage"];
            println!(
                "round={rounds} elapsed={:?} finish={} {} usage={} raw_usage={}",
                round_start.elapsed(),
                choice["finish_reason"].as_str().unwrap_or("?"),
                summarize(&msg),
                usage,
                v["usage"],
            );
            rounds += 1;

            let tool_calls = msg.get("tool_calls").and_then(|t| t.as_array()).cloned();
            let Some(calls) = tool_calls else {
                println!(
                    "FINAL: {}",
                    msg.get("content").and_then(|c| c.as_str()).unwrap_or("")
                );
                break;
            };
            if calls.is_empty() {
                println!("FINAL (no tool_calls): {}", summarize(&msg));
                break;
            }
            if rounds >= ROUND_CAP {
                println!("ROUND CAP {ROUND_CAP} reached — no final content");
                break;
            }

            messages.push(to_replay_message(&msg));
            for call in &calls {
                let name = call["function"]["name"].as_str().unwrap_or("?");
                let args: Value =
                    serde_json::from_str(call["function"]["arguments"].as_str().unwrap_or("{}"))
                        .unwrap_or_else(|_| json!({}));
                let call_id = call["id"].as_str().unwrap_or("").to_string();
                let result = match (name, args["path"].as_str()) {
                    ("read_file", Some(path)) => {
                        let p = dir.join(path);
                        match std::fs::read_to_string(&p) {
                            Ok(s) => format!("OK: {s}"),
                            Err(e) => format!("ERROR: {e}"),
                        }
                    }
                    ("write_file", Some(path)) => {
                        let p = dir.join(path);
                        let content = args["content"].as_str().unwrap_or("");
                        match std::fs::write(&p, content) {
                            Ok(()) => format!("OK: wrote {} bytes", content.len()),
                            Err(e) => format!("ERROR: {e}"),
                        }
                    }
                    _ => "ERROR: unknown tool call".to_string(),
                };
                println!("  tool={name} args={args} -> {result}");
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call_id,
                    "content": result,
                }));
            }
        }
        let total = t0.elapsed();
        let total_txt = dir.join("total.txt");
        let written =
            std::fs::read_to_string(&total_txt).unwrap_or_else(|_| "<not written>".into());
        println!(
            "TOTAL wall={total:?} rounds={rounds} fixture_total_file={:?}",
            written.trim()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    #[ignore = "live probe — set ORZ_TEST_LIVE=1 (API key from Windows Credential Manager, ADR-0006)"]
    async fn probe_thinking_max_streaming_ttft() {
        let Some(key) = gate() else { return };

        let client = reqwest::Client::new();
        let url = format!("{DEFAULT_DEEPSEEK_API_BASE}/chat/completions");
        let messages = vec![
            json!({"role": "system", "content": "You are a careful arithmetic assistant."}),
            json!({"role": "user", "content":
                "Think carefully, then answer: what is 123456789 * 987654321? Give only the number."}),
        ];
        let body = request_body(&messages, false, 32_000, true);

        println!("── probe_thinking_max_streaming_ttft ──");
        let t0 = Instant::now();
        let resp = match timeout(
            ROUND_TIMEOUT,
            client.post(&url).bearer_auth(&key).json(&body).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                println!("NETWORK ERROR: {e}");
                return;
            }
            Err(_) => {
                println!("TIMEOUT after {ROUND_TIMEOUT:?}");
                return;
            }
        };
        if !resp.status().is_success() {
            println!(
                "HTTP {}: {}",
                resp.status(),
                truncate(&resp.text().await.unwrap_or_default(), 400)
            );
            return;
        }
        let mut stream = resp.bytes_stream();
        let mut first_delta: Option<Duration> = None;
        let mut first_content_delta: Option<Duration> = None;
        let mut reason_chars = 0usize;
        let mut content_chars = 0usize;
        let mut chunk_count = 0usize;
        let mut usage: Option<Value> = None;
        let mut sse_buf = String::new();
        use futures::StreamExt;
        while let Some(Ok(chunk)) = stream.next().await {
            let text = String::from_utf8_lossy(&chunk);
            sse_buf.push_str(&text);
            // split complete lines
            while let Some(pos) = sse_buf.find('\n') {
                let line: String = sse_buf.drain(..=pos).collect();
                let line = line.trim_end();
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }
                let now = t0.elapsed();
                let delta: Value = serde_json::from_str(data).unwrap_or_default();
                chunk_count += 1;
                if let Some(d) = delta["choices"][0]["delta"].as_object() {
                    if let Some(r) = d.get("reasoning_content").and_then(|v| v.as_str()) {
                        reason_chars += r.chars().count();
                        if first_delta.is_none() {
                            first_delta = Some(now);
                            println!("first reasoning delta after {now:?} (reasoning_char={r:?})");
                        }
                    }
                    if let Some(c) = d.get("content").and_then(|v| v.as_str()) {
                        content_chars += c.chars().count();
                        if first_content_delta.is_none() {
                            first_content_delta = Some(now);
                            println!("first content delta after {now:?} (content={c:?})");
                        }
                    }
                }
                if delta.get("usage").is_some() {
                    usage = delta.get("usage").cloned();
                }
            }
        }
        println!(
            "STREAM END total={:?} chunks={chunk_count} reason_chars={reason_chars} content_chars={content_chars}",
            t0.elapsed(),
        );
        println!(
            "TTFT first_delta={:?} first_content_delta={:?} usage={}",
            first_delta,
            first_content_delta,
            usage.unwrap_or(json!("<none>")),
        );
    }

    fn truncate(s: &str, n: usize) -> String {
        if s.chars().count() <= n {
            s.to_string()
        } else {
            let mut out: String = s.chars().take(n).collect();
            out.push('…');
            out
        }
    }
}
