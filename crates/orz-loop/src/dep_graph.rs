//! 依赖图最小范围（P2-11 第 4 项 / MODEL_RESIDUAL_PRESSURE_DISCUSSION
//! 2026-08-31 §8 裁决 6 / 依赖图主线设计 2026-09-01）：
//!
//! 文件锚点链——read→write 锚点边 + 工具→实体变更边；D3 命令/检索副作用
//! **不建图**。图是重写系统内部状态（live-only，不进 epoch 快照、不持久化），
//! 模型经 `blackboard_read section=deps` 按需 PULL（零注入、无建议）。
//!
//! 事实模型（设计 §2）：
//! - ReadFact：read_file 成功（exit_code==0）登记——调用身份 + 归一化路径 +
//!   read 时刻内容锚点（sha256 权威 ≤16MB，超限/非文件 None）；
//! - WriteFact：search_replace 成功（exit_code==0）登记——调用身份 + 路径 +
//!   写参数期望锚点（expected_anchor，可能只 size/mtime）+ 写后内容锚点；
//!   锚点边匹配 = 同路径最晚 read 且锚点匹配（sha256 权威，缺失时
//!   size+mtime）。
//! - 派生边不重复存储：read→write 锚点边 = WriteFact.consumed_read；
//!   工具→实体变更边 = 每条 WriteFact。
//!
//! 容量：read/write 各 64，超出按最旧淘汰；revision 每次可见内容变化 +1
//! （PULL 自描述增量头徽章，`#[serde(skip)]` 不进序列化面）。

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::entities::{FILE_ANCHOR_HASH_MAX, normalize_entity_path};

/// read 事实容量（与 entities 128 同量级，防黑板膨胀）。
pub const DEPS_READS_CAP: usize = 64;
/// write 事实容量。
pub const DEPS_WRITES_CAP: usize = 64;
/// 渲染总字节上限（与其他分区同构的返回面纪律）。
pub const DEPS_RENDER_MAX_BYTES: usize = 8192;
/// 渲染 sha256 前缀长度（防膨胀；完整哈希随事件面/实体面留痕）。
pub const DEPS_HASH_PREFIX: usize = 8;

/// 文件内容锚点（与 F1 §2.1 / 实体登记同口径）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FileAnchor {
    /// 内容 sha256（≤16MB 计算；超限/非文件 = None）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mtime: Option<u64>,
}

impl FileAnchor {
    /// 锚点匹配：sha256 权威（双方都有时）；缺失时降级 size+mtime 快筛。
    /// 与写前核证 `verify_content_anchor` 的口径一致（R2：sha256 权威）。
    pub fn matches(&self, other: &FileAnchor) -> bool {
        match (&self.sha256, &other.sha256) {
            (Some(a), Some(b)) => a == b,
            _ => self.size == other.size && self.mtime == other.mtime,
        }
    }
}

/// read 事实（read_file 成功）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadFact {
    pub call_id: String,
    /// 归一化路径（`normalize_entity_path`）。
    pub path: String,
    pub anchor: Option<FileAnchor>,
    /// 全局单调递增序号（确定性排序；淘汰也按此）。
    pub seq: u64,
}

/// write 事实（search_replace 成功）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteFact {
    pub call_id: String,
    pub path: String,
    /// read→write 锚点边来源（read 事实 call_id；无匹配 = None）。
    pub consumed_read: Option<String>,
    /// 写参数携带的期望锚点（可能只 size/mtime，甚至全空）。
    pub consumed_anchor: Option<FileAnchor>,
    /// 写后内容锚点（与 read 锚点同口径计算）。
    pub new_anchor: Option<FileAnchor>,
    pub seq: u64,
}

/// 文件锚点链依赖图（live-only）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DepGraph {
    reads: VecDeque<ReadFact>,
    writes: VecDeque<WriteFact>,
    seq: u64,
    /// PULL 自描述分区版本计数（live-only，不进序列化面）。
    #[serde(skip)]
    revision: u64,
}

impl DepGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reads(&self) -> impl Iterator<Item = &ReadFact> {
        self.reads.iter()
    }

    pub fn writes(&self) -> impl Iterator<Item = &WriteFact> {
        self.writes.iter()
    }

    pub fn read_count(&self) -> usize {
        self.reads.len()
    }

    pub fn write_count(&self) -> usize {
        self.writes.len()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// 登记一次成功的 `read_file`：返回已登记事实（供事件面载荷复用）。
    pub fn record_read(
        &mut self,
        call_id: &str,
        path: &str,
        anchor: Option<FileAnchor>,
    ) -> ReadFact {
        self.seq = self.seq.saturating_add(1);
        let fact = ReadFact {
            call_id: call_id.to_string(),
            path: normalize_entity_path(path),
            anchor,
            seq: self.seq,
        };
        self.reads.push_back(fact.clone());
        while self.reads.len() > DEPS_READS_CAP {
            self.reads.pop_front();
        }
        self.revision = self.revision.saturating_add(1);
        fact
    }

    /// 登记一次成功的 `search_replace`：锚点边匹配 = 同路径最晚 read 且
    /// 锚点匹配（sha256 权威，缺失时 size+mtime）。返回已登记事实。
    pub fn record_write(
        &mut self,
        call_id: &str,
        path: &str,
        consumed_anchor: Option<FileAnchor>,
        new_anchor: Option<FileAnchor>,
    ) -> WriteFact {
        let path = normalize_entity_path(path);
        let consumed_read = consumed_anchor.as_ref().and_then(|expected| {
            self.reads
                .iter()
                .rev()
                .find(|r| r.path == path && r.anchor.as_ref().is_some_and(|a| a.matches(expected)))
                .map(|r| r.call_id.clone())
        });
        self.seq = self.seq.saturating_add(1);
        let fact = WriteFact {
            call_id: call_id.to_string(),
            path: path.clone(),
            consumed_read,
            consumed_anchor,
            new_anchor,
            seq: self.seq,
        };
        self.writes.push_back(fact.clone());
        while self.writes.len() > DEPS_WRITES_CAP {
            self.writes.pop_front();
        }
        self.revision = self.revision.saturating_add(1);
        fact
    }

    /// 模型面文本渲染（同 entities 纪律：确定性、有界、自描述）：
    /// 文件分组（路径序）展示 read/write 锚点链 + 工具→实体变更边
    /// 汇总 + 容量/截断标记。
    pub fn render_text(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut per_file: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for r in &self.reads {
            let anchor = render_anchor(r.anchor.as_ref());
            per_file
                .entry(r.path.clone())
                .or_default()
                .push(format!("  {} read {}", r.call_id, anchor));
        }
        for w in &self.writes {
            let used = w
                .consumed_read
                .as_ref()
                .map(|c| format!(" used={c}"))
                .unwrap_or_else(|| " (no anchor edge)".to_string());
            let new = render_anchor_value(w.new_anchor.as_ref());
            per_file
                .entry(w.path.clone())
                .or_default()
                .push(format!("  {} write{used} new={new}", w.call_id));
        }
        for (path, fact_lines) in &per_file {
            lines.push(format!("file:{path}"));
            lines.extend(fact_lines.iter().cloned());
        }
        // 工具→实体变更边（每条 WriteFact 一条）。
        lines.push("工具→实体变更:".to_string());
        for w in &self.writes {
            lines.push(format!(
                "  {} (search_replace) → file:{}",
                w.call_id, w.path
            ));
        }
        let truncated = self.reads.len() >= DEPS_READS_CAP || self.writes.len() >= DEPS_WRITES_CAP;
        lines.push(format!(
            "deps total=reads:{} writes:{} truncated={}",
            self.reads.len(),
            self.writes.len(),
            truncated
        ));

        let header = "== deps ==\n";
        let all_len: usize = lines.iter().map(|l| l.len() + 1).sum();
        let mut out = String::from(header);
        if header.len() + all_len <= DEPS_RENDER_MAX_BYTES {
            // 全部行放得下（含 total 行）：完整输出，无需 footer。
            for line in &lines {
                out.push_str(line);
                out.push('\n');
            }
            return out;
        }
        // 字节超限：正文预算先扣除 truncated footer 自身长度，保证 footer
        // 完整落盘后总长仍 ≤8 KiB（S1 审查处理 2026-09-01：原实现先按
        // 8192 塞正文、再无条件追加 footer，bytes==上限时会把返回值推超
        // 上限，且该路径无测试覆盖）。
        let footer = format!(
            "deps total=reads:{} writes:{} truncated=true\n",
            self.reads.len(),
            self.writes.len()
        );
        let mut bytes = header.len();
        let budget = DEPS_RENDER_MAX_BYTES - footer.len();
        for line in &lines {
            let line = format!("{line}\n");
            if bytes + line.len() > budget {
                break;
            }
            bytes += line.len();
            out.push_str(&line);
        }
        out.push_str(&footer);
        out
    }
}

fn render_anchor(a: Option<&FileAnchor>) -> String {
    format!("anchor={}", render_anchor_value(a))
}

fn render_anchor_value(a: Option<&FileAnchor>) -> String {
    match a {
        Some(a) => {
            let hash = a
                .sha256
                .as_ref()
                .map(|h| format!("sha256={}", &h[..h.len().min(DEPS_HASH_PREFIX)]))
                .unwrap_or_else(|| "sha256=none".to_string());
            let size = a.size.map(|s| format!(" size={s}")).unwrap_or_default();
            format!("({hash}{size})")
        }
        None => "none".to_string(),
    }
}

/// 计算目标文件的内容锚点（与实体登记同口径：stat size/mtime + 仅文件且
/// ≤16MB 计算 sha256；目录/超限/IO 错误 = None）。
pub async fn compute_anchor(path: &str) -> Option<FileAnchor> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    if !metadata.is_file() {
        return None;
    }
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let size = metadata.len();
    let sha256 = if size <= FILE_ANCHOR_HASH_MAX {
        tokio::fs::read(path)
            .await
            .ok()
            .map(|bytes| orz_assurance::sha256_hex(&bytes))
    } else {
        None
    };
    Some(FileAnchor {
        sha256,
        size: Some(size),
        mtime,
    })
}

/// 从 `expected_anchor` 参数值解析 FileAnchor（写参数形态，字段全可选）。
pub fn anchor_from_json(value: &Value) -> Option<FileAnchor> {
    let obj = value.as_object()?;
    Some(FileAnchor {
        sha256: obj
            .get("sha256")
            .and_then(Value::as_str)
            .map(str::to_string),
        size: obj.get("size").and_then(Value::as_u64),
        mtime: obj.get("mtime").and_then(Value::as_u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn anchor(sha: Option<&str>, size: u64, mtime: u64) -> FileAnchor {
        FileAnchor {
            sha256: sha.map(str::to_string),
            size: Some(size),
            mtime: Some(mtime),
        }
    }

    #[test]
    fn anchor_matching_prefers_sha256_authority() {
        let a = anchor(Some("aaa"), 100, 1);
        // 同 sha256 → 匹配（即使 size/mtime 不同）。
        let b = FileAnchor {
            sha256: Some("aaa".to_string()),
            size: Some(200),
            mtime: Some(2),
        };
        assert!(a.matches(&b));
        // 不同 sha256 → 不匹配（即使 size/mtime 相同）。
        let c = FileAnchor {
            sha256: Some("bbb".to_string()),
            size: Some(100),
            mtime: Some(1),
        };
        assert!(!a.matches(&c));
        // 缺 sha256 → size+mtime 快筛。
        let d = FileAnchor {
            sha256: None,
            size: Some(100),
            mtime: Some(1),
        };
        assert!(a.matches(&d));
        let e = FileAnchor {
            sha256: None,
            size: Some(101),
            mtime: Some(1),
        };
        assert!(!a.matches(&e));
    }

    #[test]
    fn write_links_to_most_recent_matching_read() {
        let mut g = DepGraph::new();
        g.record_read("r1", "src/a.rs", Some(anchor(Some("aaa"), 100, 1)));
        g.record_read("r2", "src/b.rs", Some(anchor(Some("bbb"), 50, 2)));
        g.record_read("r3", "src/a.rs", Some(anchor(Some("ccc"), 120, 3)));
        // 写消费 r1 的锚点（同路径最晚匹配 aaa → r1；r3 是 ccc 不匹配）。
        let w = g.record_write(
            "w1",
            "src/a.rs",
            Some(anchor(Some("aaa"), 100, 1)),
            Some(anchor(Some("ddd"), 130, 4)),
        );
        assert_eq!(w.consumed_read.as_deref(), Some("r1"));
        // 无匹配锚点 → 无边。
        let w2 = g.record_write("w2", "src/a.rs", Some(anchor(Some("zzz"), 1, 1)), None);
        assert_eq!(w2.consumed_read, None);
        // 未带 expected_anchor → 无边。
        let w3 = g.record_write("w3", "src/b.rs", None, None);
        assert_eq!(w3.consumed_read, None);
    }

    #[test]
    fn capacity_evicts_oldest_and_revision_is_monotonic() {
        let mut g = DepGraph::new();
        let mut rev = 0u64;
        for i in 0..(DEPS_READS_CAP + 5) {
            g.record_read(
                &format!("r{i}"),
                "src/a.rs",
                Some(anchor(Some("aaa"), i as u64, 1)),
            );
            rev += 1;
            assert_eq!(g.revision(), rev);
        }
        assert_eq!(g.read_count(), DEPS_READS_CAP);
        assert!(!g.reads().any(|r| r.call_id == "r0"), "oldest read evicted");
        assert!(
            g.reads()
                .any(|r| r.call_id == format!("r{}", DEPS_READS_CAP + 4))
        );
    }

    #[test]
    fn render_is_deterministic_bounded_and_self_describing() {
        let mut g = DepGraph::new();
        g.record_read("r1", "b.rs", Some(anchor(Some("bbb"), 50, 2)));
        g.record_read("r2", "a.rs", Some(anchor(Some("aaa"), 100, 1)));
        g.record_write(
            "w1",
            "a.rs",
            Some(anchor(Some("aaa"), 100, 1)),
            Some(anchor(Some("ccc"), 110, 3)),
        );
        let text = g.render_text();
        // 文件分组按路径序：a.rs 在 b.rs 前。
        let a_idx = text.find("file:a.rs").expect("a.rs section");
        let b_idx = text.find("file:b.rs").expect("b.rs section");
        assert!(a_idx < b_idx);
        assert!(text.contains("r2 read"));
        assert!(text.contains("w1 write used=r2"));
        assert!(text.contains("w1 (search_replace) → file:a.rs"));
        assert!(text.contains("deps total=reads:2 writes:1 truncated=false"));
        assert!(text.len() <= DEPS_RENDER_MAX_BYTES);
        // 确定性：两次渲染逐字节一致。
        assert_eq!(text, g.render_text());
    }

    /// S1 审查处理（2026-09-01）：字节超限截断路径——正文行塞满后追加的
    /// `truncated=true` footer 必须完整落盘且总长仍 ≤8 KiB（原实现
    /// bytes==上限时再追加 footer 会把返回值推超上限）。
    #[test]
    fn render_truncates_within_byte_cap_and_keeps_footer() {
        let mut g = DepGraph::new();
        for i in 0..DEPS_READS_CAP {
            g.record_read(
                &format!("r{i}"),
                &format!("src/long/path-{i}-{}", "x".repeat(160)),
                Some(anchor(Some("aaa"), 100, 1)),
            );
        }
        let text = g.render_text();
        assert!(
            text.len() <= DEPS_RENDER_MAX_BYTES,
            "render exceeds cap: {} > {}",
            text.len(),
            DEPS_RENDER_MAX_BYTES
        );
        assert!(text.starts_with("== deps ==\n"));
        assert!(
            text.ends_with("truncated=true\n"),
            "truncated footer must be complete: …{tail}",
            tail = &text[text.len().saturating_sub(64)..]
        );
        // 确定性：两次渲染逐字节一致。
        assert_eq!(text, g.render_text());
    }

    /// S1 审查处理（2026-09-01）：read 事实锚点为 None（非文件/IO 失败）时
    /// 永不成为锚点边来源——写即使携带 expected_anchor 也不链接。
    #[test]
    fn read_anchor_none_never_links() {
        let mut g = DepGraph::new();
        g.record_read("r1", "src/a.rs", None);
        let w = g.record_write("w1", "src/a.rs", Some(anchor(Some("aaa"), 100, 1)), None);
        assert_eq!(
            w.consumed_read, None,
            "read with no anchor cannot be consumed"
        );
    }

    #[test]
    fn anchor_from_json_parses_partial_anchor() {
        let v = json!({ "size": 4096, "mtime": 5 });
        let a = anchor_from_json(&v).expect("parses");
        assert_eq!(a.sha256, None);
        assert_eq!(a.size, Some(4096));
        assert_eq!(a.mtime, Some(5));
        assert_eq!(
            anchor_from_json(&json!({"sha256": "ab"}))
                .expect("parses")
                .sha256
                .as_deref(),
            Some("ab")
        );
        assert!(anchor_from_json(&json!("x")).is_none());
    }
}
