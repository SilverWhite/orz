//! 半助理层实体登记（THIN-HARNESS-REDESIGN V2 §4.5，R2）。
//!
//! process / file / environment 三域实体 = 稳定状态视图。半助理层执行工具
//! 时登记/更新（文件锚点 hash/size、进程 pid/exit_code、环境可用性），
//! 模型经 `blackboard_read section=entities` 按需点读；**不新增只读工具**。
//!
//! 实体 id 形态为 R2 施工初定（`file:<path>` / `process:shell` /
//! `environment:host`）；W3-R3 再定最终形态与分区命名。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::diagnostics::DiagnosticRecord;

/// 实体总容量（有界，防黑板膨胀；超出按最旧 updated_at 淘汰）。
pub const ENTITIES_TOTAL_CAP: usize = 128;
/// 渲染摘要条目上限。
pub const ENTITIES_SUMMARY_MAX: usize = 24;
/// 渲染摘要总字节上限（§4.5 返回面纪律：条目上限**与总上限**）。
pub const ENTITIES_RENDER_MAX_BYTES: usize = 4096;
/// 文件锚点 sha256 计算大小上限（≤16MB；超限记 size/mtime 不记哈希）。
pub const FILE_ANCHOR_HASH_MAX: u64 = 16 * 1024 * 1024;

/// 实体域。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    File,
    Process,
    Environment,
}

/// 服务 target 策略（R2 服务调用形态收敛：`domain.service + target + data`）。
///
/// 用户裁决：target=**实体级**——动作作用对象显式、必填、不设默认；
/// 无作用对象的动作（terminal.run / retrieval.* / delivery.submit /
/// blackboard.* 等）**省略 target**，不硬造全局实体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TargetPolicy {
    /// 全局动作：无作用对象，订单必须省略 target。
    #[default]
    None,
    /// 作用于 file 实体。
    File,
    /// 文件域但作用对象可选（如 `workspace.grep` 无 path 时工作区级
    /// 搜索）：target 可省略，与 data 路径字段同样适用"二选一 + 双写
    /// 一致"规则。
    FileOptional,
    /// 作用于 process 实体。
    Process,
    /// 作用于 environment 实体。
    Environment,
    /// 作用于任意实体（如 `diagnostics.diagnose`：target=失败对象实体）。
    AnyEntity,
}

/// 实体状态（稳定视图 + 最近失败诊断）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    pub entity_id: String,
    pub kind: EntityKind,
    /// 有界同构摘要（字段按 kind 固定；见各 register_* 构造器）。
    pub summary: Value,
    /// 最近一次失败诊断（执行失败自动派发写；`diagnostics.diagnose`
    /// 按 target=实体点读）。单实体一份、覆盖写，有界。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_diagnostic: Option<DiagnosticRecord>,
    /// RFC 3339 最近更新时刻（淘汰与渲染排序用）。
    pub updated_at: String,
}

/// 实体注册表（BTreeMap 保证确定性顺序；单 writer=半助理层）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityRegistry {
    entities: BTreeMap<String, EntityState>,
    /// PULL 自描述分区版本计数（2026-08-31，P2-11 第 1 项）——每次可见
    /// 内容变化（登记/更新/诊断覆盖/淘汰）自增 1；仅内存、不序列化。
    #[serde(skip)]
    revision: u64,
}

/// 文件路径无状态归一（实体 id 与 target↔参数一致性校验共用）：反斜杠
/// 统一为正斜杠、去除 `./` 前缀段。相对/绝对与大小写归一属 W3-R3 实体
/// id 形态定稿，不在本批引入（避免跨平台误归一）。
pub fn normalize_entity_path(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    normalized
}

/// 文件实体 id：`file:<归一化路径>`。
pub fn file_entity_id(path: &str) -> String {
    format!("file:{}", normalize_entity_path(path))
}

/// 进程实体 id（R2 初定：单槽反映最近一次终端运行状态；W3 定最终形态）。
pub fn process_entity_id() -> String {
    "process:shell".to_string()
}

/// 环境实体 id。
pub fn environment_entity_id() -> String {
    "environment:host".to_string()
}

impl EntityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, entity_id: &str) -> Option<&EntityState> {
        self.entities.get(entity_id)
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// PULL 自描述分区版本计数（live-only）。
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// B1（2026-09-03）：恢复会话黑板快照时归零版本计数——分区版本只随
    /// run 存活；恢复入口在 reset 后经 [`Self::bump_revision`] 统一计
    /// 「1 次变化」（Blackboard::restore_conversation_snapshot 负责）。
    pub(crate) fn reset_revision(&mut self) {
        self.revision = 0;
    }

    /// B1 复审（2026-09-03，P2-13 全面审查处理）：恢复会话快照后由恢复
    /// 入口调用——实体分区与其他恢复分区一致地计「1 次变化」（restore =
    /// 可见内容整体替换，与 restore_epoch_snapshot 的 bump 语义对齐；
    /// 跨 prompt 不延续上 run 的累积计数）。
    pub(crate) fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    fn insert(&mut self, state: EntityState) {
        if !self.entities.contains_key(&state.entity_id)
            && self.entities.len() >= ENTITIES_TOTAL_CAP
        {
            // 淘汰最旧条目（BTreeMap 按 id 排序，扫描一次取 min updated_at）。
            if let Some(oldest_id) = self
                .entities
                .iter()
                .min_by_key(|(_, s)| s.updated_at.clone())
                .map(|(id, _)| id.clone())
            {
                self.entities.remove(&oldest_id);
            }
        }
        self.entities.insert(state.entity_id.clone(), state);
        self.revision = self.revision.saturating_add(1);
    }

    /// 登记/更新文件实体（锚点 size/mtime/sha256；sha256 由调用方决定
    /// 是否计算——半助理层对 ≤16MB 文件计算，超限留空）。
    #[allow(clippy::too_many_arguments)] // mirrors ToolResult 结构化字段面
    pub fn register_file(
        &mut self,
        path: &str,
        exists: bool,
        size: Option<u64>,
        mtime: Option<u64>,
        sha256: Option<String>,
        encoding: Option<String>,
        updated_at: &str,
    ) -> String {
        let entity_id = file_entity_id(path);
        let normalized_path = normalize_entity_path(path);
        let mut summary = serde_json::Map::new();
        summary.insert("path".to_string(), json!(normalized_path));
        summary.insert("exists".to_string(), json!(exists));
        if let Some(size) = size {
            summary.insert("size".to_string(), json!(size));
        }
        if let Some(mtime) = mtime {
            summary.insert("mtime".to_string(), json!(mtime));
        }
        // 16MB 不变式（§4.2）：锚点哈希只在 size ≤ FILE_ANCHOR_HASH_MAX
        // 时登记；调用方误传的超限哈希在此被丢弃（registry 层兜底，
        // 不依赖 controller 单侧执行）。
        let sha256 = sha256.filter(|_| size.is_none_or(|s| s <= FILE_ANCHOR_HASH_MAX));
        if let Some(sha256) = sha256 {
            summary.insert("sha256".to_string(), json!(sha256));
        }
        if let Some(encoding) = encoding {
            summary.insert("encoding".to_string(), json!(encoding));
        }
        let state = EntityState {
            entity_id: entity_id.clone(),
            kind: EntityKind::File,
            summary: Value::Object(summary),
            last_diagnostic: self
                .entities
                .get(&entity_id)
                .and_then(|e| e.last_diagnostic.clone()),
            updated_at: updated_at.to_string(),
        };
        self.insert(state);
        entity_id
    }

    /// 登记/更新进程实体（当前会话最近一次终端运行状态）。
    pub fn register_process(
        &mut self,
        command: &str,
        exit_code: Option<i32>,
        timed_out: bool,
        updated_at: &str,
    ) -> String {
        let entity_id = process_entity_id();
        let status = if timed_out {
            "timed_out"
        } else if exit_code == Some(0) {
            "exited_ok"
        } else if exit_code.is_some() {
            "exited_error"
        } else {
            "unknown"
        };
        let mut summary = serde_json::Map::new();
        summary.insert("command".to_string(), json!(truncate_chars(command, 200)));
        summary.insert("exit_code".to_string(), json!(exit_code));
        summary.insert("timed_out".to_string(), json!(timed_out));
        summary.insert("status".to_string(), json!(status));
        let state = EntityState {
            entity_id: entity_id.clone(),
            kind: EntityKind::Process,
            summary: Value::Object(summary),
            last_diagnostic: self
                .entities
                .get(&entity_id)
                .and_then(|e| e.last_diagnostic.clone()),
            updated_at: updated_at.to_string(),
        };
        self.insert(state);
        entity_id
    }

    /// 登记/更新环境实体（shell + 工具可用性；探针快照机械来源）。
    pub fn register_environment(
        &mut self,
        shell: &str,
        available_tools: Vec<String>,
        updated_at: &str,
    ) -> String {
        let entity_id = environment_entity_id();
        let mut tools = available_tools;
        tools.sort();
        tools.dedup();
        tools.truncate(48);
        let mut summary = serde_json::Map::new();
        summary.insert("shell".to_string(), json!(shell));
        summary.insert("available_tools".to_string(), json!(tools));
        summary.insert("tool_count".to_string(), json!(tools.len()));
        let state = EntityState {
            entity_id: entity_id.clone(),
            kind: EntityKind::Environment,
            summary: Value::Object(summary),
            last_diagnostic: self
                .entities
                .get(&entity_id)
                .and_then(|e| e.last_diagnostic.clone()),
            updated_at: updated_at.to_string(),
        };
        self.insert(state);
        entity_id
    }

    /// 附着失败诊断到实体（覆盖写；单实体一份，有界）。
    pub fn set_diagnostic(&mut self, entity_id: &str, diagnostic: DiagnosticRecord) {
        if let Some(state) = self.entities.get_mut(entity_id) {
            state.last_diagnostic = Some(diagnostic);
            self.revision = self.revision.saturating_add(1);
        }
    }

    /// 有界渲染：`{entries, total, truncated}`——摘要清单 + 总上限，
    /// 单实体详情（含诊断）按需点读（`diagnostics.diagnose`）。
    pub fn render(&self, limit: usize) -> Value {
        let limit = limit.min(ENTITIES_SUMMARY_MAX);
        let entries: Vec<Value> = self
            .entities
            .values()
            .take(limit)
            .map(|state| {
                json!({
                    "entity_id": state.entity_id,
                    "kind": state.kind,
                    "summary": state.summary,
                    "has_diagnostic": state.last_diagnostic.is_some(),
                    "updated_at": state.updated_at,
                })
            })
            .collect();
        json!({
            "entries": entries,
            "total": self.entities.len(),
            "truncated": self.entities.len() > limit,
        })
    }

    /// 模型面文本渲染（同构、有界）：摘要清单 + 总上限 + 截断标记。
    pub fn render_text(&self, limit: usize) -> String {
        let limit = limit.min(ENTITIES_SUMMARY_MAX);
        let mut lines: Vec<String> = Vec::new();
        for state in self.entities.values().take(limit) {
            let kind = match state.kind {
                EntityKind::File => "file",
                EntityKind::Process => "process",
                EntityKind::Environment => "environment",
            };
            let summary = match state.kind {
                EntityKind::File => {
                    let path = state
                        .summary
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let size = state
                        .summary
                        .get("size")
                        .and_then(Value::as_u64)
                        .map(|s| format!("size={s}"))
                        .unwrap_or_default();
                    let exists = state
                        .summary
                        .get("exists")
                        .and_then(Value::as_bool)
                        .map(|e| format!("exists={e}"))
                        .unwrap_or_default();
                    format!("{} {exists} {size}", truncate_chars(&path, 240))
                }
                EntityKind::Process => {
                    let status = state
                        .summary
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let exit = state
                        .summary
                        .get("exit_code")
                        .and_then(Value::as_i64)
                        .map(|c| format!("exit_code={c}"))
                        .unwrap_or_else(|| "exit_code=none".to_string());
                    format!("{status} {exit}")
                }
                EntityKind::Environment => {
                    let shell = state
                        .summary
                        .get("shell")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let count = state
                        .summary
                        .get("tool_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    format!("shell={shell} tool_count={count}")
                }
            };
            lines.push(format!(
                "- {} ({kind}, {}, has_diagnostic={})\n",
                truncate_chars(&state.entity_id, 320),
                summary,
                state.last_diagnostic.is_some()
            ));
        }
        // 总字节上限：超出后丢弃剩余条目（头部截断标记置真）。
        let mut kept = 0usize;
        let mut bytes = 0usize;
        for line in &lines {
            if bytes + line.len() > ENTITIES_RENDER_MAX_BYTES {
                break;
            }
            bytes += line.len();
            kept += 1;
        }
        let entries_capped = self.entities.len() > limit;
        let byte_capped = kept < lines.len();
        let mut out = format!(
            "[entities] total={} truncated={}\n",
            self.entities.len(),
            entries_capped || byte_capped
        );
        for line in lines.into_iter().take(kept) {
            out.push_str(&line);
        }
        out
    }
}

/// 本地有界截断（实体摘要用；不依赖 console 模块）。
fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let head: String = s.chars().take(max_chars).collect();
        format!("{head}…[truncated]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_entity_roundtrip_preserves_anchor() {
        let mut registry = EntityRegistry::new();
        let id = registry.register_file(
            "src\\main.rs",
            true,
            Some(42),
            Some(1700000000),
            Some("a".repeat(64)),
            Some("utf-8".to_string()),
            "2026-08-28T00:00:00Z",
        );
        assert_eq!(id, "file:src/main.rs");
        let state = registry.get(&id).expect("entity");
        assert_eq!(state.kind, EntityKind::File);
        assert_eq!(state.summary["size"], 42);
        assert_eq!(state.summary["sha256"], "a".repeat(64));
        assert!(!registry.is_empty());
    }

    /// PULL 自描述（2026-08-31，P2-11 第 1 项 / 设计 §2）：实体登记/更新
    /// 每次可见变化自增版本计数（live-only，不序列化）。
    #[test]
    fn revision_bumps_on_register_and_overwrite() {
        let mut registry = EntityRegistry::new();
        assert_eq!(registry.revision(), 0);
        registry.register_file(
            "a.txt",
            true,
            Some(1),
            Some(2),
            Some("a".repeat(64)),
            None,
            "2026-08-31T00:00:00Z",
        );
        assert_eq!(registry.revision(), 1);
        // 同一实体覆盖写（锚点变化）仍计 1 次。
        registry.register_file(
            "a.txt",
            true,
            Some(2),
            Some(3),
            Some("b".repeat(64)),
            None,
            "2026-08-31T00:00:01Z",
        );
        assert_eq!(registry.revision(), 2);
    }

    #[test]
    fn process_entity_overwrites_single_slot_and_carries_diagnostic() {
        let mut registry = EntityRegistry::new();
        let id = registry.register_process("cargo test", Some(1), false, "2026-08-28T00:00:01Z");
        assert_eq!(id, "process:shell");
        assert_eq!(registry.get(&id).unwrap().summary["status"], "exited_error");
        registry.set_diagnostic(
            &id,
            DiagnosticRecord {
                exit_code: Some(1),
                matched_signature: Some("exit_nonzero".to_string()),
                key_fields: Vec::new(),
                target_state: Vec::new(),
                tail: Vec::new(),
                tail_is_raw: false,
                log_pointer: "TRC-1".to_string(),
                truncated: false,
            },
        );
        assert!(registry.get(&id).unwrap().last_diagnostic.is_some());
        // 覆盖写：同一 id 单槽，last_diagnostic 保留（跨次调用状态延续）。
        registry.register_process("cargo test", Some(0), false, "2026-08-28T00:00:02Z");
        let state = registry.get(&id).unwrap();
        assert_eq!(state.summary["status"], "exited_ok");
        assert_eq!(
            state.last_diagnostic.as_ref().unwrap().matched_signature,
            Some("exit_nonzero".to_string())
        );
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn environment_entity_lists_sorted_bounded_tools() {
        let mut registry = EntityRegistry::new();
        let id = registry.register_environment(
            "powershell",
            vec![
                "grep".to_string(),
                "read_file".to_string(),
                "grep".to_string(),
            ],
            "2026-08-28T00:00:00Z",
        );
        assert_eq!(id, "environment:host");
        let state = registry.get(&id).unwrap();
        assert_eq!(state.summary["tool_count"], 2);
        assert_eq!(state.summary["available_tools"][0], "grep");
        assert_eq!(state.summary["available_tools"][1], "read_file");
    }

    #[test]
    fn render_is_bounded_with_cap_flag() {
        let mut registry = EntityRegistry::new();
        for i in 0..30 {
            registry.register_file(
                &format!("f{i}.txt"),
                true,
                Some(1),
                None,
                None,
                None,
                &format!("2026-08-28T00:{:02}:00Z", i % 60),
            );
        }
        let rendered = registry.render(10);
        assert_eq!(rendered["total"], 30);
        assert_eq!(rendered["entries"].as_array().unwrap().len(), 10);
        assert_eq!(rendered["truncated"], true);
    }

    #[test]
    fn total_cap_evicts_oldest() {
        let mut registry = EntityRegistry::new();
        for i in 0..ENTITIES_TOTAL_CAP + 10 {
            registry.register_file(
                &format!("f{i}.txt"),
                true,
                None,
                None,
                None,
                None,
                &format!("2026-08-28T00:{:02}:00Z", i % 60),
            );
        }
        assert!(registry.len() <= ENTITIES_TOTAL_CAP);
    }

    #[test]
    fn serde_roundtrip_preserves_state() {
        let mut registry = EntityRegistry::new();
        registry.register_file(
            "a.txt",
            true,
            Some(3),
            None,
            None,
            Some("utf-8".to_string()),
            "2026-08-28T00:00:00Z",
        );
        let json = serde_json::to_value(&registry).expect("serialize");
        let back: EntityRegistry = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back.len(), 1);
        assert_eq!(back.get("file:a.txt").unwrap().summary["size"], 3);
    }

    #[test]
    fn file_entity_id_normalizes_separators_and_dot_prefix() {
        assert_eq!(file_entity_id("src\\main.rs"), "file:src/main.rs");
        assert_eq!(file_entity_id("./a/b.txt"), "file:a/b.txt");
        assert_eq!(file_entity_id(".\\a\\b.txt"), "file:a/b.txt");
        // 已归一路径保持原样（幂等）。
        assert_eq!(file_entity_id("a/b.txt"), "file:a/b.txt");
    }

    #[test]
    fn register_file_drops_sha256_over_hash_max() {
        let mut registry = EntityRegistry::new();
        let id = registry.register_file(
            "big.bin",
            true,
            Some(FILE_ANCHOR_HASH_MAX + 1),
            None,
            Some("a".repeat(64)),
            None,
            "2026-08-28T00:00:00Z",
        );
        let state = registry.get(&id).unwrap();
        assert!(state.summary.get("sha256").is_none());
        // 边界内保留哈希。
        let id2 = registry.register_file(
            "small.txt",
            true,
            Some(FILE_ANCHOR_HASH_MAX),
            None,
            Some("b".repeat(64)),
            None,
            "2026-08-28T00:00:01Z",
        );
        assert_eq!(
            registry.get(&id2).unwrap().summary["sha256"],
            "b".repeat(64)
        );
    }

    #[test]
    fn render_text_enforces_total_byte_cap() {
        let mut registry = EntityRegistry::new();
        for i in 0..12 {
            registry.register_file(
                &format!("long-path-{}-{}", "x".repeat(2000), i),
                true,
                Some(1),
                None,
                None,
                None,
                &format!("2026-08-28T00:{i:02}:00Z"),
            );
        }
        let rendered = registry.render_text(24);
        assert!(rendered.len() <= ENTITIES_RENDER_MAX_BYTES + 128);
        assert!(
            rendered.contains("truncated=true"),
            "byte-capped render must be marked: {rendered}"
        );
        // 普通路径不受影响：仍完整列出且不误标截断。
        let mut small = EntityRegistry::new();
        small.register_file(
            "a.txt",
            true,
            Some(1),
            None,
            None,
            None,
            "2026-08-28T00:00:00Z",
        );
        assert!(!small.render_text(24).contains("truncated=true"));
    }
}
