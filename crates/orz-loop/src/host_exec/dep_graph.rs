//! 依赖图事实（P2-11）：`record_dep_graph_fact` 记录 read→write 锚点链 / 工具→实体变更边。
//! 0ai (2026-09-16) 拆分自 `host_exec.rs`（机械搬移，行为不变）。

use crate::controller::AgentLoopController;
#[cfg(test)]
use crate::gateway::model::ToolCall;
#[cfg(test)]
use crate::host::ToolResult;
#[cfg(test)]
use orz_assurance::EventType;

impl AgentLoopController {
    /// P2-11 第 4 项 / 依赖图主线设计 §3 (2026-09-01)：依赖图事实记录——
    /// `read_file` / `search_replace` **成功**（exit_code==0）时建图（文件
    /// 锚点链：read→write 锚点边 + 工具→实体变更边；D3 命令/检索副作用
    /// 不建图）。返回随 ToolCompleted 事件载荷写入的 `dep_graph` 事实
    /// （None = 不建图）。锚点计算与实体登记同口径（stat + sha256 ≤16MB）。
    pub(crate) async fn record_dep_graph_fact(
        &self,
        tool: &str,
        call_id: &str,
        arguments: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        if !matches!(tool, "read_file" | "search_replace") {
            return None;
        }
        let path = ["target_file", "file_path", "path"]
            .iter()
            .find_map(|key| arguments.get(*key).and_then(serde_json::Value::as_str))?;
        let anchor = crate::dep_graph::compute_anchor(path).await;
        let mut bb = self.blackboard.write();
        match tool {
            "read_file" => {
                let f = bb.dep_graph.record_read(call_id, path, anchor);
                Some(serde_json::json!({
                    "kind": "read",
                    "path": f.path,
                    "anchor": f.anchor,
                }))
            }
            "search_replace" => {
                let expected = arguments
                    .get("expected_anchor")
                    .and_then(crate::dep_graph::anchor_from_json);
                let f = bb.dep_graph.record_write(call_id, path, expected, anchor);
                Some(serde_json::json!({
                    "kind": "write",
                    "path": f.path,
                    "consumed_read": f.consumed_read,
                    "consumed_anchor": f.consumed_anchor,
                    "new_anchor": f.new_anchor,
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use orz_assurance::JournalRecorder;
    use std::sync::Arc;

    /// P2-11 第 4 项 / 依赖图主线设计 §3-§5 (2026-09-01)：真实工具链
    /// read_file → search_replace（携带 expected_anchor）→
    /// blackboard_read section=deps——依赖图记录 read→write 锚点边 +
    /// 工具→实体变更边，事实随 ToolCompleted 入事件面，模型可 PULL。
    #[tokio::test]
    async fn dep_graph_anchor_chain_recorded_and_pullable() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let target = dir.join("dep_target.py");
        std::fs::write(&target, b"alpha").unwrap();
        let target_str = target.to_string_lossy().to_string();
        let sha = orz_assurance::sha256_hex(b"alpha");
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // Round 1: read_file.
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({ "target_file": target_str }),
                call_id: "call-r1".to_string(),
            }]),
            // Round 2: search_replace consuming the read anchor.
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "search_replace".to_string(),
                arguments: serde_json::json!({
                    "file_path": target_str,
                    "old_string": "alpha",
                    "new_string": "beta",
                    "expected_anchor": { "sha256": sha, "size": 5 },
                }),
                call_id: "call-w1".to_string(),
            }]),
            // Round 3: PULL the dependency graph.
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "blackboard_read".to_string(),
                arguments: serde_json::json!({ "section": "deps" }),
                call_id: "call-d1".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "依赖图冒烟",
                "RUN-DEP1",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        // 图状态：一条 read + 一条 write，锚点边指向 call-r1。
        {
            let bb = controller.blackboard().read();
            assert_eq!(bb.dep_graph.read_count(), 1);
            assert_eq!(bb.dep_graph.write_count(), 1);
            let w = bb.dep_graph.writes().next().expect("write fact");
            assert_eq!(w.consumed_read.as_deref(), Some("call-r1"));
            assert!(bb.dep_graph.revision() >= 2);
        }
        // 模型 PULL 面：deps 渲染含锚点边与变更边。
        let received = fake.received_requests();
        let deps_reply = received
            .iter()
            .find_map(|r| {
                r.messages
                    .iter()
                    .find(|m| m.tool_call_id.as_deref() == Some("call-d1"))
            })
            .expect("deps reply");
        assert!(
            deps_reply.content.contains("read")
                && deps_reply.content.contains("write used=call-r1"),
            "deps render: {}",
            deps_reply.content
        );
        assert!(
            deps_reply
                .content
                .contains("call-w1 (search_replace) → file:"),
            "mutation edge: {}",
            deps_reply.content
        );
        // 工具定义增量扩展：blackboard_read 的 section 枚举含 deps。
        let last_request = received.last().expect("last request");
        let bb_def = last_request
            .tools
            .iter()
            .find(|t| t.name == "blackboard_read")
            .expect("blackboard_read declared");
        let sections = bb_def
            .parameters
            .get("properties")
            .and_then(|p| p.get("section"))
            .and_then(|s| s.get("enum"))
            .and_then(|e| e.as_array())
            .expect("section enum declared");
        assert!(
            sections.iter().any(|v| v.as_str() == Some("deps")),
            "deps must be declared in the section enum: {sections:?}"
        );
        // 事件面：read/write 事实随 ToolCompleted 入链（F11 同构核对面）。
        let payloads: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        let read_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-r1")
            .expect("read completion");
        assert_eq!(read_payload["dep_graph"]["kind"], "read");
        assert_eq!(
            read_payload["dep_graph"]["path"],
            crate::entities::normalize_entity_path(&target_str)
        );
        assert_eq!(read_payload["dep_graph"]["anchor"]["sha256"], sha);
        let write_payload = payloads
            .iter()
            .find(|p| p["call_id"] == "call-w1")
            .expect("write completion");
        assert_eq!(write_payload["dep_graph"]["kind"], "write");
        assert_eq!(write_payload["dep_graph"]["consumed_read"], "call-r1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P2-11 第 4 项 / 依赖图主线设计 §3 (2026-09-01)：失败/拒绝不建图
    /// （exit_code≠0 无 dep_graph 事实），且 D3 命令（run_terminal_cmd）
    /// 从不建图——依赖图只覆盖 read_file/search_replace 成功事实。
    #[tokio::test]
    async fn dep_graph_skips_failures_and_d3_commands() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "boom".to_string(),
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // 失败 read + D3 命令（即使成功也不建图）。
            ScriptedResponse::tool_calls(vec![
                ToolCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({ "target_file": "missing.py" }),
                    call_id: "call-f1".to_string(),
                },
                ToolCall {
                    name: "run_terminal_cmd".to_string(),
                    arguments: serde_json::json!({ "command": "echo hi" }),
                    call_id: "call-t1".to_string(),
                },
            ]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "不建图", "RUN-DEP2", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        {
            let bb = controller.blackboard().read();
            assert_eq!(bb.dep_graph.read_count(), 0, "failed read not graphed");
            assert_eq!(bb.dep_graph.write_count(), 0, "no writes");
        }
        let payloads: Vec<serde_json::Value> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::ToolCompleted)
            .map(|e| e.payload)
            .collect();
        assert_eq!(payloads.len(), 2, "{payloads:?}");
        for p in &payloads {
            assert!(p.get("dep_graph").is_none(), "{p:?}");
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 0v-A 引擎级取证面（0v 第二批 S2，2026-09-12；设计 §8.6/§8.7）─────
}
