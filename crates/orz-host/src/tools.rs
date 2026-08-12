//! Tool wiring — Grok `ToolRegistryBuilder` → `FinalizedToolset`.
//!
//! The toolset is the Grok provider side of the LoopHost contract:
//! `tools_registry()` serves its definitions; `call_tool()` dispatches
//! through `FinalizedToolset::call`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use orz_tools::computer::local::file_system::LocalFs;
use orz_tools::computer::local::terminal::LocalTerminalBackend;
use orz_tools::computer::types::{AsyncFileSystem, TerminalBackend};
use orz_tools::implementations::web_search::WebSearchConfig;
use orz_tools::registry::types::{
    FinalizedToolset, SessionContext, ToolRegistryBuilder, ToolServerConfig,
};

use crate::credentials::CredentialReader;

/// 2026-08-11 (direction correction): the web_search client config — the
/// reader supplies the DeepSeek API key (the SAME key as the main
/// transport; the earlier xAI Grok search backend was withdrawn by user
/// adjudication — retrieval must come from the current provider).
/// `ORZ_WEB_SEARCH_BASE_URL`/`ORZ_WEB_SEARCH_MODEL` override the DeepSeek
/// defaults (non-secret configuration). Absent key = Disabled = the client
/// is not injected and the capability probe records it (never a silent
/// fallback).
pub fn web_search_config(reader: &dyn CredentialReader) -> WebSearchConfig {
    match reader.read() {
        Ok(key) => WebSearchConfig::Enabled {
            api_key: key,
            base_url: std::env::var("ORZ_WEB_SEARCH_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com".to_string()),
            model: std::env::var("ORZ_WEB_SEARCH_MODEL")
                .unwrap_or_else(|_| "deepseek-v4-flash".to_string()),
            extra_headers: Default::default(),
            alpha_test_key: None,
        },
        Err(e) => {
            tracing::warn!("web_search credential read failed: {}", e.message);
            WebSearchConfig::Disabled
        }
    }
}

/// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): the global `web_search` tool
/// family — the exact `web_search` name plus any `web_search_*` variant
/// (ADR-0010 §3.7.7/§11.3: global web_search concurrency is 1; the main
/// agent and the external-retrieval subagent share the same semaphore,
/// while internal and external retrieval sessions still run in parallel).
/// `web_fetch` is deliberately NOT here — the contract limits only
/// `web_search`; the variant prefix mirrors `relay::is_web_retrieval_tool`
/// (the subagent-dispatch match) so a name cannot dodge the semaphore by
/// switching spellings.
pub fn is_web_search_tool(name: &str) -> bool {
    name == "web_search" || name.starts_with("web_search_")
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): the web_fetch client config — always
/// enabled (direct HTTP fetch, no key dependency); bounded by the
/// fail-closed defaults (SSRF guard, size caps). PDF evidence pipeline
/// (2026-08-11): the cwd-level evidence store is wired so direct fetches of
/// PDFs are ingested inline (ADR-0010 §3.7.6). Other orz-tools hosts that
/// don't configure a root keep the legacy save-to-downloads behavior.
pub fn web_fetch_config_default(
    cwd: &Path,
) -> orz_tools::implementations::grok_build::web_fetch::WebFetchConfig {
    use orz_tools::implementations::grok_build::web_fetch::{WebFetchConfig, WebFetchParams};
    WebFetchConfig::Enabled {
        params: WebFetchParams {
            cache_ttl_secs: None,
            max_cache_entries: None,
            timeout_secs: None,
            max_content_length: None,
            max_markdown_length: None,
            context_window_tokens: None,
            allowed_domains: None,
            proxy_endpoint: None,
            allow_local: None,
            pdf_evidence_root: Some(crate::pdf_evidence::evidence_root(cwd)),
        },
    }
}

/// Build a finalized toolset for a session working directory.
///
/// SessionContext is constructed with minimal-but-functional defaults:
/// local terminal backend, local fs rooted at `cwd`, noop notification
/// handle, all optional backends (memory/MCP/LSP/image) disabled. Web
/// retrieval is config-driven (GAP-RETRIEVAL-TOOLS 2026-08-10 / ADR-0006
/// 2026-08-11): web_fetch always enabled, web_search gated on the
/// credential-read config (the caller constructs it once and stores it —
/// single source of truth, no double read).
pub fn build_toolset(
    cwd: &Path,
    web_search_config: &WebSearchConfig,
) -> Result<Arc<FinalizedToolset>, String> {
    let backend: Arc<dyn TerminalBackend> = Arc::new(LocalTerminalBackend::new());
    let fs: Arc<dyn AsyncFileSystem> = Arc::new(LocalFs);

    let ctx = SessionContext {
        backend,
        fs,
        cwd: cwd.to_path_buf(),
        session_folder: cwd.join(".gsa").join("session"),
        session_env: Arc::new(HashMap::new()),
        notification_handle: Default::default(),
        owner_session_id: None,
        subagent: None,
        parent_scheduler_handle: None,
        skills: Vec::new(),
        state_path: cwd.join(".gsa").join("state.json"),
        memory_backend: None,
        web_search_config: web_search_config.clone(),
        web_fetch_config: web_fetch_config_default(cwd),
        lsp: None,
        image_gen_config: Default::default(),
        video_gen_config: Default::default(),
        app_builder_deployer_config: Default::default(),
        api_key_provider: None,
        auth_provider: None,
        attribution_callback: None,
        system_reminder_tag: "system-reminder",
    };

    let builder = ToolRegistryBuilder::new();
    // finalize only enables the tools listed in the config. The builder
    // pre-registers multiple tool packs (GrokBuild / Codex / OpenCode / …)
    // whose default client names collide — enable the GrokBuild namespace only.
    //
    // Scheduler-family ban (2026-08-09, TB hard B 组复盘裁决): the system
    // holds exactly TWO designed subagents (project-doc retrieval + external
    // retrieval — CN §7.2 / design §4.5); GrokBuild's local multi-agent
    // scheduler ecosystem is a third subagent class and is banned. The whole
    // async ecosystem is removed, not just `task`: `monitor` runs on the same
    // task system (bg_handle.task_id / terminal.get_task), so with
    // `get_task_output` banned its results would be unrecoverable.
    // TB failure 缺口 1 (results silently dropped at run end — gpt2-codegolf
    // and train-fasttext) is eliminated at the tool-surface: the model is
    // left with synchronous tools only (bash/read/edit/…).
    // `kill_terminal_command` is the async kill switch for the same
    // ecosystem; banning it with the rest leaves no dangling-command hazard.
    // Complete scheduler-ecosystem surface (2026-08-09 census): task /
    // task_output (get_task_output, wait_tasks, get_terminal_command_output) /
    // kill_task / kill_terminal_command / monitor / scheduler (create, delete,
    // list) / workflow. `todo_write` and `update_goal` are task-list/goal
    // bookkeeping, NOT subagent scheduling — they stay.
    //
    // NOTE (residual, 2026-08-09): the inherited-crate implementations are
    // NOT deleted (Slice #13 inherited-crate discipline + orz-agent tests
    // lock TaskTool presence + terminal/session coupling). This filter is the
    // only surface — KEEP this list in sync whenever the GrokBuild tool
    // packs change. Future long-task needs go through the run_tests pattern
    // (host-owned synchronous tools) or an explicit CN §5.1 review — never
    // resurrect the async scheduler ecosystem.
    const BANNED_GROK_BUILD_TOOLS: &[&str] = &[
        "task",                        // subagent/task scheduler (third subagent class)
        "get_task_output",             // polls task results (dead without task)
        "wait_tasks",                  // waits on tasks (dead without task)
        "get_terminal_command_output", // async terminal output collector
        "kill_task",                   // kills tasks (dead without task)
        "kill_terminal_command",       // kills monitor/terminal commands (async ecosystem)
        "monitor",                     // async terminal watch (task-system based)
        "scheduler_create",            // recurring task scheduler
        "scheduler_delete",            // scheduler bookkeeping
        "scheduler_list",              // scheduler bookkeeping
        "workflow",                    // multi-agent workflow orchestrator
    ];
    let tools: Vec<_> = builder
        .known_tool_ids()
        .into_iter()
        .filter(|id| {
            id.starts_with("GrokBuild:")
                && !BANNED_GROK_BUILD_TOOLS
                    .iter()
                    .any(|t| id.ends_with(&format!(":{t}")))
        })
        .map(|id| {
            // bash background mode (`enabled_background`) requires the banned
            // `kill_task` (background tasks must be observable/cancellable),
            // so it is disabled too — synchronous-only tool surface, per the
            // scheduler-family ban above. Long-running commands rely on the
            // P0-1 tool timeout + P1-1 stall watchdogs instead.
            let params = if id.ends_with(":run_terminal_cmd") {
                Some(serde_json::Map::from_iter([(
                    "enabled_background".to_string(),
                    serde_json::Value::Bool(false),
                )]))
            } else {
                None
            };
            orz_tools::registry::types::ToolConfig {
                id,
                params,
                name_override: None,
                params_name_overrides: None,
                description_override: None,
                behavior_version: None,
                kind: None,
            }
        })
        .collect();
    let config = ToolServerConfig {
        tools,
        behavior_preset: None,
    };
    builder
        .finalize(config, ctx)
        .map(Arc::new)
        .map_err(|e| format!("toolset finalize failed: {e:?}"))
}

/// Adapt `FinalizedToolset` definitions to the orz-loop `ToolRegistry` view.
pub struct ToolsetRegistry {
    toolset: Arc<FinalizedToolset>,
    /// local_browser (2026-08-10): whether `browser_read` is declared.
    /// Set by the session bootstrap via the capability probe — declaration
    /// and probe are the same source of truth (fail-closed default: false).
    browser_ready: bool,
}

impl ToolsetRegistry {
    pub fn new(toolset: Arc<FinalizedToolset>) -> Self {
        Self {
            toolset,
            browser_ready: false,
        }
    }

    pub fn toolset(&self) -> &Arc<FinalizedToolset> {
        &self.toolset
    }

    /// local_browser (2026-08-10): flip `browser_read` declaration on/off
    /// (caller is the host's `with_browser_session`).
    pub fn set_browser_ready(&mut self, ready: bool) {
        self.browser_ready = ready;
    }
}

impl orz_loop::host::ToolRegistry for ToolsetRegistry {
    fn get(&self, name: &str) -> Option<orz_loop::host::ToolDef> {
        // GAP-RETRIEVAL-TOOLS (2026-08-10): the host-owned project-doc
        // index is declared alongside the GrokBuild registry (executed via
        // the call_tool special case).
        if name == "project_doc_index" {
            return Some(crate::project_doc_index::ProjectDocIndex::tool_def());
        }
        // local_browser (2026-08-10): `browser_read` is only declared when
        // the session's browser lane is actually ready — a model must never
        // see a tool that will fail on every call.
        if name == "browser_read" && self.browser_ready {
            return Some(crate::local_browser::browser_read_tool_def());
        }
        // PDF evidence (2026-08-11): `pdf_read` reads the local evidence
        // store — no browser dependency, always declared (project_doc_index
        // pattern). The mode gate lives in the relay.
        if name == "pdf_read" {
            return Some(crate::pdf_evidence::pdf_read_tool_def());
        }
        self.toolset
            .tool_definitions()
            .into_iter()
            .find(|d| d.function.name == name)
            .map(|d| orz_loop::host::ToolDef {
                name: d.function.name,
                description: d.function.description.unwrap_or_default(),
                parameters: d.function.parameters,
            })
    }

    fn list(&self) -> Vec<orz_loop::host::ToolDef> {
        let mut defs: Vec<orz_loop::host::ToolDef> = self
            .toolset
            .tool_definitions()
            .into_iter()
            .map(|d| orz_loop::host::ToolDef {
                name: d.function.name,
                description: d.function.description.unwrap_or_default(),
                parameters: d.function.parameters,
            })
            .collect();
        // GAP-RETRIEVAL-TOOLS (2026-08-10): the host-owned project-doc
        // index rides the registry list (declaration + availability).
        if !defs.iter().any(|d| d.name == "project_doc_index") {
            defs.push(crate::project_doc_index::ProjectDocIndex::tool_def());
        }
        // local_browser (2026-08-10): declared only when the lane is ready.
        if self.browser_ready && !defs.iter().any(|d| d.name == "browser_read") {
            defs.push(crate::local_browser::browser_read_tool_def());
        }
        // PDF evidence (2026-08-11): always declared (local store, no
        // browser dependency).
        if !defs.iter().any(|d| d.name == "pdf_read") {
            defs.push(crate::pdf_evidence::pdf_read_tool_def());
        }
        defs
    }
}

/// Map a `xai_tool_runtime::ToolError` to the orz-loop contract error.
pub fn map_tool_error(err: &xai_tool_runtime::ToolError) -> orz_loop::host::ToolError {
    // The runtime error carries a name/message; surface it as execution failure.
    orz_loop::host::ToolError::ExecutionFailed(err.to_string())
}

/// 2026-08-08 blackboard-partition review closure: derive the exit-code
/// semantics of a structured tool output for the orz-loop `ToolResult`
/// contract (the controller's edit-action gate keys on `Some(0)` = the edit
/// actually happened).
///
/// - `bash` carries its real exit code (the terminal-backend captures it).
/// - `search_replace` reports "applied" only via the `EditsApplied` variant;
///   every other variant (`NoMatchesFound`, `MultipleMatchesFound`,
///   `FileNotFound`, `InvalidInput`, `FileAlreadyExists`, `FilenameTooLong`)
///   is an Ok output that changed NOTHING — non-zero, so the "实际变动" gate
///   stays closed for them.
/// - every other successful output is `0`.
pub fn exit_code_from_output(output: &orz_tools::types::output::ToolOutput) -> Option<i32> {
    use orz_tools::types::output::SearchReplaceOutput;
    match output {
        orz_tools::types::output::ToolOutput::Bash(bash) => Some(bash.exit_code),
        orz_tools::types::output::ToolOutput::SearchReplace(sr) => Some(match sr {
            SearchReplaceOutput::EditsApplied(_) => 0,
            _ => 1,
        }),
        _ => Some(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_tools::implementations::web_search::WebSearchConfig;

    /// A fake credential reader with a scripted outcome.
    struct FakeReader {
        result: Result<String, crate::credentials::CredentialError>,
    }
    impl crate::credentials::CredentialReader for FakeReader {
        fn read(&self) -> Result<String, crate::credentials::CredentialError> {
            self.result.clone()
        }
    }

    /// 2026-08-11: the web_search config follows the credential reader —
    /// Err = Disabled (tool client not injected, capability probe reports
    /// degraded); Ok = Enabled with the DeepSeek defaults; the non-secret
    /// `ORZ_WEB_SEARCH_BASE_URL`/`ORZ_WEB_SEARCH_MODEL` env overrides still
    /// apply. P2-1 (review 2026-08-10): the env overrides are serialized
    /// via the shared `TESTS_ENV_LOCK` and restored on drop.
    #[tokio::test]
    async fn web_search_config_follows_credential_source() {
        let _env_lock = crate::tests::tests_env_lock().lock().await;
        let _base = crate::tests::EnvVarGuard::new("ORZ_WEB_SEARCH_BASE_URL");
        let _model = crate::tests::EnvVarGuard::new("ORZ_WEB_SEARCH_MODEL");

        // Err → Disabled.
        let err = crate::credentials::CredentialError {
            message: "test: no credential".into(),
        };
        assert!(matches!(
            web_search_config(&FakeReader {
                result: Err(err.clone())
            }),
            WebSearchConfig::Disabled
        ));
        assert!(!web_search_config(&FakeReader { result: Err(err) }).is_enabled());

        // Ok → Enabled with DeepSeek defaults.
        unsafe {
            std::env::remove_var("ORZ_WEB_SEARCH_BASE_URL");
            std::env::remove_var("ORZ_WEB_SEARCH_MODEL");
        }
        match web_search_config(&FakeReader {
            result: Ok("sk-test-key".into()),
        }) {
            WebSearchConfig::Enabled {
                api_key,
                base_url,
                model,
                ..
            } => {
                assert_eq!(api_key, "sk-test-key");
                assert_eq!(base_url, "https://api.deepseek.com");
                assert_eq!(model, "deepseek-v4-flash");
            }
            other => panic!("expected Enabled, got {other:?}"),
        }

        // Overrides respected.
        unsafe {
            std::env::set_var("ORZ_WEB_SEARCH_BASE_URL", "https://example.invalid/v1");
        }
        match web_search_config(&FakeReader {
            result: Ok("sk-test-key".into()),
        }) {
            WebSearchConfig::Enabled { base_url, .. } => {
                assert_eq!(base_url, "https://example.invalid/v1");
            }
            other => panic!("expected Enabled, got {other:?}"),
        }
    }

    /// 2026-08-11: `redacted()` is the only sanctioned exit — serializing
    /// it must never leak the api_key (the DeepSeek key is the main
    /// credential — the discipline matters MORE after the direction
    /// correction). Production wiring regression: both the config level and
    /// the host accessor assert `***REDACTED***` in place of the key.
    #[test]
    fn web_search_config_redacted_never_leaks_key() {
        let config = web_search_config(&FakeReader {
            result: Ok("sk-test-9f8e7d6c5b4a".into()),
        });
        let redacted = config.redacted();
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(json.contains("***REDACTED***"), "{json}");
        assert!(!json.contains("sk-test-9f8e7d6c5b4a"), "{json}");
    }

    /// web_fetch is always enabled (no key dependency).
    #[test]
    fn web_fetch_config_is_always_enabled() {
        assert!(web_fetch_config_default(std::path::Path::new(".")).is_enabled());
    }

    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): the semaphore matches the
    /// `web_search` family — the exact name plus `_*` variants (mirror of
    /// `relay::is_web_retrieval_tool`'s search half, so a variant spelling
    /// cannot dodge the gate) — and nothing else. `web_fetch` is NOT gated
    /// (ADR-0010 §3.7.7 limits only web_search).
    #[test]
    fn is_web_search_tool_matches_search_family_only() {
        assert!(is_web_search_tool("web_search"));
        assert!(is_web_search_tool("web_search_arxiv_paper"));
        assert!(is_web_search_tool("web_search_custom"));
        // web_fetch family stays ungated; host/retrieval tools never acquire.
        assert!(!is_web_search_tool("web_fetch"));
        assert!(!is_web_search_tool("web_fetch_page"));
        assert!(!is_web_search_tool("project_doc_index"));
        assert!(!is_web_search_tool("browser_read"));
        assert!(!is_web_search_tool("read_file"));
        assert!(!is_web_search_tool("bash"));
        // Prefix boundary: the underscore separates the family.
        assert!(!is_web_search_tool("web_searchX"));
        assert!(!is_web_search_tool("web_searchx"));
    }
}
