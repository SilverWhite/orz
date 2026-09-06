//! Tool wiring — Grok `ToolRegistryBuilder` → `FinalizedToolset`.
//!
//! The toolset is the Grok provider side of the LoopHost contract:
//! `tools_registry()` serves its definitions; `call_tool()` dispatches
//! through `FinalizedToolset::call`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use orz_tools::computer::local::file_system::LocalFs;
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

/// ORZ-LARGE-FILE-READ-CONTRACT (ADR-0010 §14.22) + TER T1.10 (W-F13a)：
/// the TOML 口子 for the read_file coarse gate — `[toolset.read_file]
/// coarse_gate_bytes` from the effective config (system-managed > managed >
/// user layer merge). Absent / non-integer → `None` (the tool falls back to
/// the env var); out-of-range values are clamped to 8–64 KiB exactly like
/// the env 口子 (64 KiB 档，2026-09-04)。
pub fn read_file_coarse_gate_from_config(config: &toml::Value) -> Option<usize> {
    const READ_COARSE_GATE_MIN: usize = 8 * 1024;
    const READ_COARSE_GATE_MAX: usize = 64 * 1024;
    config
        .get("toolset")
        .and_then(|t| t.get("read_file"))
        .and_then(|r| r.get("coarse_gate_bytes"))
        .and_then(|v| v.as_integer())
        .map(|v| (v.max(0) as usize).clamp(READ_COARSE_GATE_MIN, READ_COARSE_GATE_MAX))
}

/// THIN-HARNESS-REDESIGN §4.6 审查处理 (2026-08-27, 用户裁定) +
/// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：终端命令分层超时与
/// 中间回报——
/// - 分层默认超时：程序/脚本类 600s、普通命令 300s，由宿主逐调用按命令
///   形态注入（见 [`terminal_tier_default_timeout_ms`]）；模型可传
///   `timeout` 覆盖，上限 900s（`max_timeout_secs`）。TER T1.4
///   （2026-09-04）去硬杀后，这些值只作前台 auto-bg deadline / 上限
///   引用，不再是杀活跃命令的超时点。
/// - 中间回报：`auto_background_on_timeout` 自 TER T1.1（2026-09-03）起由
///   BashParams struct/serde 默认（true）单一提供，不再显式注入；
///   `foreground_block_budget_ms` 自 TER T1.2（2026-09-03）起同样由
///   BashParams struct/serde 默认（180_000）单一提供，本函数不再注入
///   （缺省经 serde 解析即 180_000）。TER T1.4（2026-09-04）：命令在
///   `min(解析超时, 180s 预算)` 先到者处自动后台化并返回一次「运行 +
///   工具自身情况」中间状态；后台化后原解析超时退役，仅 10h 绝对兜底
///   （T1.5 idle+CPU 兜底随后接管）——不再有「满 timeout 杀活跃命令」。
/// - 工具面封闭：`enabled_background` 自 T1.1 起由 struct/serde 默认 true
///   单一提供；`hide_background_input` 自 TER T1.3（2026-09-04）起同样由
///   BashParams struct/serde 默认（true）单一提供，本函数不再显式注入
///   （缺省经 serde 解析即 true，`is_background` 不出现在模型面 schema）；
///   另注入 `allow_background_operator=false` 封掉 `&`，保持模型侧
///   「一次调用 = 一个结果」，显式后台化不开放。
/// - TER 全面审查 S6（2026-09-04）边界登记：模型面封闭 = orz-host 装配
///   默认（此处显式注入 false 保证）；codegen BashParams 库层默认保留
///   兼容开放（`allow_background_operator` 默认 true，纯 struct 直用仍可
///   显式后台），该边界随 ADR §14.53 转正一并裁定。
/// 外层 ORZ_TOOL_TIMEOUT_SECS=900 维持全局兜底（终端命令实际到不了外层值）。
pub(crate) fn run_terminal_cmd_tool_params() -> Option<serde_json::Map<String, serde_json::Value>> {
    Some(serde_json::Map::from_iter([
        (
            "allow_background_operator".to_string(),
            serde_json::Value::Bool(false),
        ),
        ("timeout_secs".to_string(), serde_json::Value::from(600.0)),
        (
            "max_timeout_secs".to_string(),
            serde_json::Value::from(900.0),
        ),
    ]))
}

/// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：终端命令两档默认超时的
/// 机械分类——程序/脚本类 600_000ms，普通命令 300_000ms。规则（纯函数、
/// 逐 token 归一化，先统一启发式）：
///
/// - 命令首 token（去引号/`./` 前缀、小写）命中解释器/包管理/构建工具
///   集合（python/python3/py/node/npm/npx/yarn/pnpm/ruby/perl/php/go/
///   cargo/rustc/make/cmake/ninja/gradle/mvn/java/javac/gcc/g++/clang/
///   apt/apt-get/pip/pip3/docker/bash/sh/zsh/pwsh/powershell/cmd/pytest）；
/// - 或任一 token 以脚本扩展名结尾（.py/.sh/.js/.ts/.rb/.pl/.php/.ps1/
///   .bat/.cmd）；
/// - 或首 token 以 `./` 开头（工作区脚本/可执行文件）。
///
/// 其余判为普通命令。误判由模型显式 `timeout` 覆盖（可低可高）。
/// TER T1.4（2026-09-04）去硬杀：本函数的值经
/// [`inject_terminal_default_timeout`] 注入后只作前台 auto-bg deadline
/// 引用（主线 180s 预算先到时在 180s 后台化；预算关闭时才以注入值为
/// 后台化点），不再作为杀活跃命令的超时。
pub fn terminal_tier_default_timeout_ms(command: &str) -> u64 {
    const ORDINARY_MS: u64 = 300_000;
    const PROGRAM_MS: u64 = 600_000;

    const PROGRAM_LEADING: &[&str] = &[
        "python",
        "python3",
        "py",
        "node",
        "npm",
        "npx",
        "yarn",
        "pnpm",
        "ruby",
        "perl",
        "php",
        "go",
        "cargo",
        "rustc",
        "make",
        "cmake",
        "ninja",
        "gradle",
        "mvn",
        "java",
        "javac",
        "gcc",
        "g++",
        "clang",
        "apt",
        "apt-get",
        "pip",
        "pip3",
        "docker",
        "bash",
        "sh",
        "zsh",
        "pwsh",
        "powershell",
        "cmd",
        "pytest",
    ];
    const SCRIPT_EXTS: &[&str] = &[
        ".py", ".sh", ".js", ".ts", ".rb", ".pl", ".php", ".ps1", ".bat", ".cmd",
    ];

    let tokens: Vec<&str> = command.split_whitespace().collect();
    let Some(first) = tokens.first() else {
        return ORDINARY_MS;
    };
    let first_norm = first
        .trim_matches(|c| c == '"' || c == '\'')
        .trim_start_matches("./")
        .to_ascii_lowercase();
    if PROGRAM_LEADING.contains(&first_norm.as_str()) {
        return PROGRAM_MS;
    }
    if first.starts_with("./") {
        return PROGRAM_MS;
    }
    if tokens.iter().any(|t| {
        let t = t.trim_matches(|c| c == '"' || c == '\'');
        SCRIPT_EXTS
            .iter()
            .any(|ext| t.to_ascii_lowercase().ends_with(ext))
    }) {
        return PROGRAM_MS;
    }
    ORDINARY_MS
}

/// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：把两档默认超时注入
/// `run_terminal_cmd` 参数——仅当模型未显式传 `timeout` 时按命令形态
/// 注入（毫秒）；显式传入的任意非 null 值（含 0）原样保留，由工具层再
/// 按 max_timeout_secs 封顶；`null` 视为未传（审查处理 P3-1：语义与
/// BashParams serde 的 `Option` 一致——0 是显式值、null 是缺省）。
/// TER T1.4（2026-09-04）：注入的分层值只作 auto-bg deadline 引用，
/// 不再杀活跃命令。纯函数，供 `call_tool_inner` 执行侧调用。
pub fn inject_terminal_default_timeout(args: serde_json::Value) -> serde_json::Value {
    let timeout_present = args.get("timeout").map(|v| !v.is_null()).unwrap_or(false);
    if timeout_present {
        return args;
    }
    let Some(cmd) = args
        .get("command")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
    else {
        return args;
    };
    let mut args = args;
    args["timeout"] = serde_json::json!(terminal_tier_default_timeout_ms(&cmd));
    args
}

/// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：从工具结构化输出映射
/// `mid_run` 事实——`run_terminal_cmd` 自动后台化返回
/// `BackgroundTaskStarted`（命令仍在运行）时填充；其余工具/形态为 None。
/// 纯函数，供 `call_tool_inner` 在构建 `ToolResult` 时调用。
pub fn terminal_mid_run_from_output(
    name: &str,
    output: &orz_tools::types::output::ToolOutput,
) -> Option<orz_loop::host::ToolMidRunStatus> {
    if name != "run_terminal_cmd" {
        return None;
    }
    let orz_tools::types::output::ToolOutput::BackgroundTaskStarted(bg) = output else {
        return None;
    };
    Some(orz_loop::host::ToolMidRunStatus {
        task_id: bg.task_id.clone(),
        pid: bg.pid,
        output_file: bg.output_file.clone(),
        total_bytes: bg.total_bytes,
    })
}

/// TER T1.11 (W-F13b)：run_terminal_cmd **前台完成且输出被截断**时映射
/// 持久化输出检索对象（完整输出落盘路径 = `output_object_id` +
/// 截断前字节）；其余工具/形态为 None（未截断不需要对象指针）。
pub fn terminal_output_object_from_output(
    name: &str,
    output: &orz_tools::types::output::ToolOutput,
) -> Option<orz_loop::host::TerminalOutputObject> {
    if name != "run_terminal_cmd" {
        return None;
    }
    let orz_tools::types::output::ToolOutput::Bash(bash) = output else {
        return None;
    };
    if !bash.truncated {
        return None;
    }
    Some(orz_loop::host::TerminalOutputObject {
        total_bytes: bash.total_bytes as u64,
        output_object_id: bash.output_file.clone(),
    })
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
    read_file_coarse_gate_bytes: Option<usize>,
    backend: Arc<dyn TerminalBackend>,
) -> Result<Arc<FinalizedToolset>, String> {
    let fs: Arc<dyn AsyncFileSystem> = Arc::new(LocalFs);

    // P0-0m GSA-SESSION-VOLUME（ADR-0010 §14.56 D1，2026-09-06）：host 是
    // 唯一知道 `.gsa` 真实落点（含 fallback 链、评测容器把 `.gsa` 挂载为
    // 指向卷目录的符号链接）的角色——装配期做**一次** symlink-aware
    // canonical 解析（D1 单源规则 resources::session_volume_canonical_root，
    // permission.rs 等义镜像共用），解析结果即 SessionVolumeRoot.
    // canonical_root，下层工具沙箱不再各自 canonicalize 再猜。解析失败
    // （`.gsa` 尚不存在等）回退词法路径；窗口判定仍 fail-closed（canonical
    // 不可得时按词法落点判定）。
    let session_volume_root = Some(orz_tools::types::resources::session_volume_canonical_root(
        cwd,
    ));

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
        session_volume_root,
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
    // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.1): the main-agent GrokBuild
    // surface is converged to the hard-retained set. Everything else
    // (list_dir / run_tests / search_tool / todo_write / update_goal /
    // scheduler family / …) is either reachable through the retained tools
    // (bash/read/grep) or sealed at the boundary (todo_write / update_goal
    // — code retained, call interface rejected). The inherited-crate
    // implementations are NOT deleted (Slice #13 inherited-crate
    // discipline); this allowlist is the only surface. The scheduler-family
    // ban (2026-08-09) is subsumed: no async-scheduler name is retained.
    const RETAINED_GROK_BUILD_TOOLS: &[&str] = &[
        "run_terminal_cmd",
        "read_file",
        "grep",
        "search_replace",
        "web_search",
        "web_fetch",
    ];
    let tools: Vec<_> = builder
        .known_tool_ids()
        .into_iter()
        .filter(|id| {
            id.starts_with("GrokBuild:")
                && RETAINED_GROK_BUILD_TOOLS
                    .iter()
                    .any(|t| id.ends_with(&format!(":{t}")))
        })
        .map(|id| {
            // TER T1.3 (2026-09-04)：模型面封闭（`hide_background_input`
            // struct/serde 默认 true）+ `allow_background_operator=false`
            // ——显式 `&`/`is_background` 不开放，`requires_expr` 不再要求
            // 被禁的 get_task_output/kill_task 同台；内部 auto-background
            // 仍开启，可观察性/可取消性由 180s 中间回报（PID + 落盘输出）
            // 与完成提醒承担（scheduler 族工具不保留）。
            let params = if id.ends_with(":run_terminal_cmd") {
                run_terminal_cmd_tool_params()
            } else if id == "GrokBuild:read_file"
                && let Some(gate) = read_file_coarse_gate_bytes
            {
                Some(serde_json::Map::from_iter([(
                    "coarse_gate_bytes".to_string(),
                    serde_json::Value::from(gate),
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
        // local_browser (2026-08-10): `browser_read` is only declared when
        // the session's browser lane is actually ready — a model must never
        // see a tool that will fail on every call.
        if name == "browser_read" && self.browser_ready {
            return Some(crate::local_browser::browser_read_tool_def());
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
        // local_browser (2026-08-10): declared only when the lane is ready.
        if self.browser_ready && !defs.iter().any(|d| d.name == "browser_read") {
            defs.push(crate::local_browser::browser_read_tool_def());
        }
        defs
    }
}

/// Map a `xai_tool_runtime::ToolError` to the orz-loop contract error.
pub fn map_tool_error(err: &xai_tool_runtime::ToolError) -> orz_loop::host::ToolError {
    // THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1): 保留工具侧的
    // Timeout 类别——web_search 客户端超时（120s）返回
    // `ToolErrorKind::Timeout`，宿主不得降级为 ExecutionFailed（否则
    // 控制器 timed_out 标记丢失、模型看不到「TIMED OUT」语义）。
    match err.kind {
        xai_tool_runtime::ToolErrorKind::Timeout => {
            orz_loop::host::ToolError::Timeout(err.to_string())
        }
        _ => orz_loop::host::ToolError::ExecutionFailed(err.to_string()),
    }
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

/// FUS-RETRIEVAL-MECH B-1 (2026-08-13): structured tool metadata for the
/// orz-loop `ToolResult` seam. Only `web_search` carries a payload today —
/// its citation URLs (the candidate pool for the mechanical prefilter).
/// Everything else stays `None`, so no structured data leaves the
/// model-visible text contract except the designed web_search channel.
pub fn structured_from_output(
    output: &orz_tools::types::output::ToolOutput,
) -> Option<serde_json::Value> {
    match output {
        orz_tools::types::output::ToolOutput::WebSearch(ws) if !ws.citations.is_empty() => {
            Some(serde_json::json!({ "citations": ws.citations }))
        }
        _ => None,
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

    /// THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1): 工具侧 Timeout
    /// 类别经宿主映射后必须保持为 `ToolError::Timeout`（控制器据此落
    /// timed_out 标记与「TIMED OUT」模型语义）；其余类别保持
    /// ExecutionFailed。
    #[test]
    fn map_tool_error_preserves_timeout_kind() {
        let timeout = xai_tool_runtime::ToolError::timeout(
            xai_tool_runtime::ToolId::new("web_search").expect("valid"),
            "web_search timed out after 120s",
        );
        assert!(
            matches!(
                map_tool_error(&timeout),
                orz_loop::host::ToolError::Timeout(_)
            ),
            "tool-side Timeout must map to host Timeout"
        );
        let exec = xai_tool_runtime::ToolError::execution(
            xai_tool_runtime::ToolId::new("web_search").expect("valid"),
            "boom",
        );
        assert!(
            matches!(
                map_tool_error(&exec),
                orz_loop::host::ToolError::ExecutionFailed(_)
            ),
            "execution errors keep the ExecutionFailed mapping"
        );
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

    /// FUS-RETRIEVAL-MECH B-1 (2026-08-13): only `web_search` fills the
    /// structured seam — its citation URLs ride as `{"citations": [...]}`;
    /// empty citations and every other tool stay `None` (no empty pool, no
    /// accidental structured data outside the designed channel).
    #[test]
    fn structured_from_output_carries_web_search_citations_only() {
        use orz_tools::types::output::{TextOutput, ToolOutput, WebSearchOutput};

        let ws = ToolOutput::WebSearch(WebSearchOutput {
            query: "q".into(),
            content: "snippet".into(),
            citations: vec!["https://a.example".into(), "https://b.example".into()],
            allowed_domains: None,
            pre_formatted: None,
        });
        let structured = structured_from_output(&ws).expect("citations payload");
        assert_eq!(structured["citations"][0], "https://a.example");
        assert_eq!(structured["citations"][1], "https://b.example");

        let empty = ToolOutput::WebSearch(WebSearchOutput {
            query: "q".into(),
            content: "no results".into(),
            citations: vec![],
            allowed_domains: None,
            pre_formatted: None,
        });
        assert!(structured_from_output(&empty).is_none());

        let other = ToolOutput::Text(TextOutput {
            text: "x".into(),
            consumed_completion_task_id: None,
        });
        assert!(structured_from_output(&other).is_none());
    }
}

#[cfg(test)]
mod config_tests {
    use super::*;

    fn parse_gate(toml_src: &str) -> Option<usize> {
        // Document deserializer (`Value::from_str` only parses a single value
        // expression, not a full TOML document with table headers).
        let Ok(value) = toml::from_str::<toml::Value>(toml_src) else {
            return None;
        };
        read_file_coarse_gate_from_config(&value)
    }

    /// ORZ-LARGE-FILE-READ-CONTRACT (P3-1) + TER T1.10 (W-F13a):
    /// `[toolset.read_file] coarse_gate_bytes` parses from the effective
    /// config and is clamped to 8–64 KiB exactly like the env 口子.
    #[test]
    fn read_file_coarse_gate_from_config_parses_and_clamps() {
        assert_eq!(parse_gate(""), None, "absent section");
        assert_eq!(
            parse_gate("[toolset.read_file]\ncoarse_gate_bytes = 8192\n"),
            Some(8 * 1024)
        );
        assert_eq!(
            parse_gate("[toolset.read_file]\ncoarse_gate_bytes = 16384\n"),
            Some(16 * 1024)
        );
        assert_eq!(
            parse_gate("[toolset.read_file]\ncoarse_gate_bytes = 1\n"),
            Some(8 * 1024),
            "below minimum clamps up"
        );
        assert_eq!(
            parse_gate("[toolset.read_file]\ncoarse_gate_bytes = 65536\n"),
            Some(64 * 1024),
            "above maximum clamps down"
        );
        assert_eq!(
            parse_gate("[toolset.read_file]\ncoarse_gate_bytes = 200000\n"),
            Some(64 * 1024),
            "64 KiB is the T1.10 ceiling"
        );
        assert_eq!(
            parse_gate("[toolset.read_file]\ncoarse_gate_bytes = \"16k\"\n"),
            None,
            "non-integer is ignored"
        );
        assert_eq!(
            parse_gate("[toolset]\nread_file = {}\n"),
            None,
            "empty read_file table is ignored"
        );
    }

    /// THIN-HARNESS-REDESIGN §4.6 审查处理 (2026-08-27) +
    /// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：run_terminal_cmd
    /// 的分层超时与中间回报配置——`enabled_background` /
    /// `auto_background_on_timeout` 自 TER T1.1 起由 BashParams
    /// struct/serde 默认（true）单一提供（此处不注入，缺省即 true），
    /// `foreground_block_budget_ms` 自 TER T1.2 起由 BashParams
    /// struct/serde 默认（180_000，满 180s 后台化并返回一次中间状态）
    /// 单一提供（此处不注入，缺省经 serde 解析即 180_000）、
    /// `hide_background_input` 自 TER T1.3 起由 BashParams struct/serde
    /// 默认（true）单一提供（此处不注入，缺省经 serde 解析即 true——
    /// `is_background` 不出现在模型面 schema）、
    /// `allow_background_operator=false`（封掉 `&`）、
    /// `timeout_secs=600`（程序/脚本缺省；普通命令 300s 由宿主逐调用
    /// 注入；TER T1.4 起只作 auto-bg deadline 引用）、
    /// `max_timeout_secs=900`（模型可传上限；TER T1.4 起同样只作上限
    /// 引用）。配置键名与
    /// BashParams serde 字段一致（未知键静默 no-op，必须逐字匹配）。
    #[test]
    fn run_terminal_cmd_params_s5_2_layered_timeout_and_mid_run() {
        let params = run_terminal_cmd_tool_params().expect("params present");
        assert_eq!(
            params.get("foreground_block_budget_ms"),
            None,
            "TER T1.2: budget no longer injected — BashParams resident default is the single source"
        );
        // 单一生效源联检：无注入时把主线 params 交给 BashParams serde，
        // 缺省键必须收敛为 180_000。
        let parsed: orz_tools::implementations::grok_build::bash::BashParams =
            serde_json::from_value(serde_json::Value::Object(params.clone()))
                .expect("mainline params parse into BashParams");
        assert_eq!(
            parsed.foreground_block_budget_ms,
            Some(180_000),
            "TER T1.2: omitted budget must resolve to the 180s resident default"
        );
        assert!(
            parsed.hide_background_input,
            "TER T1.3: omitted hide_background_input must resolve to the resident true default"
        );
        assert_eq!(
            params.get("allow_background_operator"),
            Some(&serde_json::Value::Bool(false)),
            "explicit `&` backgrounding stays closed"
        );
        assert_eq!(
            params.get("hide_background_input"),
            None,
            "TER T1.3: hide flag no longer injected — BashParams resident default is the single source"
        );
        assert_eq!(
            params.get("timeout_secs"),
            Some(&serde_json::Value::from(600.0)),
            "program/script omit-default command timeout"
        );
        assert_eq!(
            params.get("max_timeout_secs"),
            Some(&serde_json::Value::from(900.0)),
            "model-passed timeout ceiling raised to 900s"
        );
        assert!(
            params.get("enabled_background").is_none()
                && params.get("auto_background_on_timeout").is_none(),
            "TER T1.1/T1.2/T1.3: flags + budget + hide now come from BashParams resident defaults, \
             no redundant mainline injection"
        );
        // 键名逐字匹配 BashParams 字段（未知键会被静默忽略，防拼写漂移）。
        for key in [
            "allow_background_operator",
            "timeout_secs",
            "max_timeout_secs",
        ] {
            assert!(params.contains_key(key), "param key {key} present");
        }
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：两档默认超时的
    /// 机械分类——程序/脚本 600s、普通 300s；模型显式 timeout 覆盖不受影响。
    #[test]
    fn terminal_tier_classifies_program_vs_ordinary() {
        // 程序/脚本类 → 600s
        for cmd in [
            "python train.py --epochs 100",
            "python3 -m pytest tests/",
            "node server.js",
            "npm run build",
            "cargo build --release",
            "make -j8",
            "pip install -r requirements.txt",
            "bash setup.sh",
            "sh ./install.sh",
            "powershell -File build.ps1",
            "cmd /c build.bat",
            "docker build -t app .",
            "apt-get install -y r-base",
            "./scripts/run_tests.sh",
            "java -jar app.jar",
            "go test ./...",
            "pytest -q tests/test_x.py",
        ] {
            assert_eq!(
                terminal_tier_default_timeout_ms(cmd),
                600_000,
                "program/script default for: {cmd}"
            );
        }
        // 普通命令 → 300s
        for cmd in [
            "dir",
            "echo hello",
            "ls -la",
            "git status",
            "curl -s https://example.com",
            "sleep 100",
            "wget -O out.bin https://example.com/x",
            "env",
            "",
            "   ",
        ] {
            assert_eq!(
                terminal_tier_default_timeout_ms(cmd),
                300_000,
                "ordinary default for: {cmd:?}"
            );
        }
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：默认超时注入——
    /// 模型未传 timeout 时按命令形态注入两档；显式 timeout 原样保留。
    #[test]
    fn inject_terminal_default_timeout_respects_class_and_override() {
        let ordinary = inject_terminal_default_timeout(serde_json::json!({
            "command": "echo hello",
            "description": "x",
        }));
        assert_eq!(ordinary["timeout"], serde_json::json!(300_000u64));

        let program = inject_terminal_default_timeout(serde_json::json!({
            "command": "python train.py",
            "description": "x",
        }));
        assert_eq!(program["timeout"], serde_json::json!(600_000u64));

        // 显式 timeout（含 0）不被覆盖。
        let explicit = inject_terminal_default_timeout(serde_json::json!({
            "command": "python train.py",
            "timeout": 120_000,
        }));
        assert_eq!(explicit["timeout"], serde_json::json!(120_000u64));
        let zero = inject_terminal_default_timeout(serde_json::json!({
            "command": "python train.py",
            "timeout": 0,
        }));
        assert_eq!(zero["timeout"], serde_json::json!(0u64));

        // 无 command 时保持原样。
        let no_cmd = inject_terminal_default_timeout(serde_json::json!({}));
        assert!(no_cmd.get("timeout").is_none());
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2)：mid_run 结构化映射
    /// ——run_terminal_cmd 自动后台化结果携带 pid/output_file/total_bytes/
    /// task_id；其余工具与形态为 None（无文本判定）。
    #[test]
    fn terminal_mid_run_from_output_maps_backgrounded_start() {
        use orz_tools::types::output::{BackgroundTaskStarted, ToolOutput};
        let bg = ToolOutput::BackgroundTaskStarted(BackgroundTaskStarted {
            task_id: "call-t1".into(),
            task_type: "bash".into(),
            output_file: "/tmp/terminal/call-t1.log".into(),
            status: "running".into(),
            command: "python train.py".into(),
            summary: "still running".into(),
            retrieval_hint: String::new(),
            pre_formatted: Some("report".into()),
            pid: Some(42),
            total_bytes: Some(8192),
        });
        let mid = terminal_mid_run_from_output("run_terminal_cmd", &bg).expect("mid_run");
        assert_eq!(mid.task_id, "call-t1");
        assert_eq!(mid.pid, Some(42));
        assert_eq!(mid.output_file, "/tmp/terminal/call-t1.log");
        assert_eq!(mid.total_bytes, Some(8192));

        assert!(terminal_mid_run_from_output("read_file", &bg).is_none());
        let text = ToolOutput::Text(orz_tools::types::output::TextOutput {
            text: "ok".into(),
            consumed_completion_task_id: None,
        });
        assert!(terminal_mid_run_from_output("run_terminal_cmd", &text).is_none());
    }
}
