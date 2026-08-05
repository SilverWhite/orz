//! Event sources — anything that produces `TuiEvent`s (Python
//! `event_source.py` shape): fake (in-process), replay (journal file) and
//! live journal tail.

use std::path::PathBuf;

use tokio::sync::mpsc;

use crate::bridge;
use crate::events::TuiEvent;
use crate::journal_tail::JournalTail;

/// A source of TUI events, drained periodically by the app.
pub trait EventSource {
    /// Drain any ready events (non-blocking).
    fn poll(&mut self) -> Vec<TuiEvent>;
    /// Whether this source still expects more events.
    fn is_active(&self) -> bool;
    /// Stop the source (best effort).
    fn close(&mut self);
}

/// In-process test/demo source (Python `FakeEventSource`).
#[derive(Debug, Default)]
pub struct FakeSource {
    events: Vec<TuiEvent>,
}

impl FakeSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: TuiEvent) {
        self.events.push(event);
    }
}

impl EventSource for FakeSource {
    fn poll(&mut self) -> Vec<TuiEvent> {
        std::mem::take(&mut self.events)
    }

    fn is_active(&self) -> bool {
        !self.events.is_empty()
    }

    fn close(&mut self) {
        self.events.clear();
    }
}

/// Replay a journal file once (Python `JsonlFileSource`): whole-file read,
/// canonical RunEvent → TuiEvent mapping, plus a chain-validity StatusUpdate.
pub struct ReplaySource {
    path: PathBuf,
    remaining: Vec<TuiEvent>,
    active: bool,
    /// Chain validation result (surfaced via the 验证 status card).
    pub valid: bool,
}

impl ReplaySource {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| std::io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
        let mut remaining: Vec<TuiEvent> = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<orz_assurance::journal::RunEvent>(line) {
                Ok(ev) => remaining.push(bridge::run_event_to_tui(&ev)),
                Err(_) => remaining.push(TuiEvent::Unknown {
                    event_type: "unparseable_line".into(),
                }),
            }
        }
        // Chain validity — status bar feedback (never blocks the stream).
        // require_terminal=true: a truncated journal (no terminal event) is
        // an integrity failure, not a valid replay (plan §4).
        let replay = orz_assurance::replay_journal(&path, None, None, true);
        let valid = replay.valid;
        remaining.push(TuiEvent::StatusUpdate {
            label: "验证".into(),
            ok: valid,
        });
        let active = !remaining.is_empty();
        Ok(Self {
            path,
            remaining,
            active,
            valid,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl EventSource for ReplaySource {
    fn poll(&mut self) -> Vec<TuiEvent> {
        if self.remaining.is_empty() {
            self.active = false;
            return Vec::new();
        }
        let mut out = std::mem::take(&mut self.remaining);
        out.push(TuiEvent::StatusUpdate {
            label: "空闲".into(),
            ok: true,
        });
        self.active = false;
        out
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn close(&mut self) {
        self.remaining.clear();
        self.active = false;
    }
}

/// Live source wrapping a journal tailer thread.
pub struct JournalTailSource {
    tail: JournalTail,
    buffer: Vec<TuiEvent>,
    ended: bool,
}

impl JournalTailSource {
    pub fn new(path: PathBuf) -> Self {
        Self {
            tail: JournalTail::spawn(path),
            buffer: Vec::new(),
            ended: false,
        }
    }
}

impl EventSource for JournalTailSource {
    fn poll(&mut self) -> Vec<TuiEvent> {
        for ev in self.tail.poll() {
            let tui = bridge::run_event_to_tui(&ev);
            if tui.is_terminal() {
                self.ended = true;
            }
            self.buffer.push(tui);
        }
        std::mem::take(&mut self.buffer)
    }

    fn is_active(&self) -> bool {
        !self.ended
    }

    fn close(&mut self) {
        self.tail.stop();
        self.ended = true;
        self.buffer.clear();
    }
}

/// A boxed source for the app/runner.
pub type BoxedSource = Box<dyn EventSource>;

/// Unbounded channel-backed source (runner → app handoff for ClientMsg).
pub struct ChannelSource {
    rx: mpsc::UnboundedReceiver<TuiEvent>,
}

impl ChannelSource {
    pub fn new(rx: mpsc::UnboundedReceiver<TuiEvent>) -> Self {
        Self { rx }
    }
}

impl EventSource for ChannelSource {
    fn poll(&mut self) -> Vec<TuiEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
        out
    }

    fn is_active(&self) -> bool {
        true
    }

    fn close(&mut self) {
        self.rx.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TuiEvent;

    #[test]
    fn fake_source_drains() {
        let mut src = FakeSource::new();
        src.push(TuiEvent::RunPreflight { timestamp: String::new() });
        src.push(TuiEvent::RunStarted { prompt: "q".into(), timestamp: String::new() });
        assert!(src.is_active());
        let drained = src.poll();
        assert_eq!(drained.len(), 2);
        assert!(!src.is_active());
        assert!(src.poll().is_empty());
    }

    #[test]
    fn replay_source_replays_and_validates() {
        let dir = std::env::temp_dir().join(format!("orz-tui-replay-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("events.jsonl");
        // A valid two-event chain (seal_event computes hashes).
        let mut ev0 = orz_assurance::journal::RunEvent::new(
            "RUN-REP01".into(),
            0,
            orz_assurance::journal::EventType::RunStarted,
            "m".into(),
            None,
            "run-event-v0.1.schema.json".into(),
            serde_json::json!({"prompt": "hi"}),
            orz_assurance::journal::Redaction::None,
            "2026-08-05T00:00:00Z".into(),
        );
        orz_assurance::seal_event(&mut ev0).unwrap();
        let mut ev1 = orz_assurance::journal::RunEvent::new(
            "RUN-REP01".into(),
            1,
            orz_assurance::journal::EventType::RunFinished,
            "m".into(),
            Some(ev0.event_sha256.clone()),
            "run-event-v0.1.schema.json".into(),
            serde_json::json!({"status": "completed"}),
            orz_assurance::journal::Redaction::None,
            "2026-08-05T00:00:01Z".into(),
        );
        orz_assurance::seal_event(&mut ev1).unwrap();
        let mut content = String::new();
        content.push_str(&serde_json::to_string(&ev0).unwrap());
        content.push('\n');
        content.push_str(&serde_json::to_string(&ev1).unwrap());
        content.push('\n');
        std::fs::write(&path, content).unwrap();

        let mut src = ReplaySource::new(path.clone()).unwrap();
        let events = src.poll();
        assert_eq!(events[0].kind(), "run_started");
        assert_eq!(events[1].kind(), "run_finished");
        // Chain validity status lands last.
        let last = events.last().unwrap();
        assert_eq!(last.kind(), "status_update");

        // Tampered journal → validation failure surfaced.
        let mut content = std::fs::read_to_string(&path).unwrap();
        content.push_str("{broken}\n");
        std::fs::write(&path, content).unwrap();
        let mut src = ReplaySource::new(path.clone()).unwrap();
        let events = src.poll();
        assert!(events.iter().any(|e| {
            matches!(e, TuiEvent::StatusUpdate { label, ok } if label == "验证" && !ok)
        }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn channel_source_drains() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut src = ChannelSource::new(rx);
        tx.send(TuiEvent::TextDelta { text: "流".into() }).unwrap();
        let drained = src.poll();
        assert_eq!(drained.len(), 1);
        assert!(src.poll().is_empty());
    }
}
