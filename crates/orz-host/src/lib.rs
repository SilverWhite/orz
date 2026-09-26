//! orz-host — Thin core that bridges Grok providers to the self-built agent loop.
//!
//! Phase 0: Skeleton only — compiles against kept Grok providers.
//! Phase 1+: ACP server, LoopHost implementation, session lifecycle, journal writer.
//!
//! Dependency direction (Codex discipline):
//!   orz-host → {orz-loop, orz-assurance, Grok providers}
//!   No Grok crate depends on orz-host.

pub mod acp_server;
pub mod approval;
pub mod codex_app;
pub mod codex_permission;
pub mod credentials;
mod env_snapshot;
pub mod grok_home;
pub mod keystore;
pub mod local_browser;
pub mod pdf_evidence;
pub mod permission;
pub mod process_tree;
pub mod project_doc_index;
pub mod reclaim;
pub mod resource_hint;

/// Total order over tiers for the change detector (`host_resource_snapshot`
/// trigger). Order follows the design ladder: normal < watch < soft <
/// reclaim_direct < hard < unknown (unknown is kept distinct — never folded).
fn tier_rank(tier: &crate::resource_hint::ResourceTier) -> u8 {
    match tier {
        crate::resource_hint::ResourceTier::Normal => 1,
        crate::resource_hint::ResourceTier::Watch => 2,
        crate::resource_hint::ResourceTier::Soft => 3,
        crate::resource_hint::ResourceTier::ReclaimDirect => 4,
        crate::resource_hint::ResourceTier::Hard => 5,
        crate::resource_hint::ResourceTier::Unknown => 6,
    }
}

/// One in-flight call's kill handle (0z S2R F-BE-3(a)).
///
/// 0aw（2026-09-20 用户裁决 ④）：登记表与 `DispatchGuard::drop` 的句柄关闭
/// **保留**（`KILL_ON_JOB_CLOSE` 的调用级拆树仍靠它）；原挂在条目上的
/// `action_class` 字段随分类器一并退役——它只服务 hard 档树杀的「重档
/// call_id 集」枚举，该面已不存在。
#[derive(Clone, Debug)]
struct LiveCallJob {
    token: u64,
    call_id: String,
    /// Duplicated call-job handle (0 = none / platform without job handles).
    job_handle: isize,
}

/// The dispatch registration guard: closes this call's live call-job entries
/// (dropping the duplicated handles) on drop. `closed` lets a late spawn
/// observation (racy cross-thread attach) unregister itself immediately
/// instead of leaking a kill handle.
pub struct DispatchGuard<'a> {
    host: &'a OrzHost,
    token: u64,
    closed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for DispatchGuard<'_> {
    fn drop(&mut self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let jobs: Vec<LiveCallJob> = {
            let mut guard = self
                .host
                .live_call_jobs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            std::mem::take(&mut *guard)
        };
        let (mine, rest): (Vec<_>, Vec<_>) = jobs
            .into_iter()
            .partition(|entry| entry.token == self.token);
        for entry in mine {
            tracing::debug!(
                call_id = %entry.call_id,
                "dispatch closed: closing per-call job handle (KILL_ON_JOB_CLOSE)"
            );
            OrzHost::close_job_handle(entry.job_handle);
        }
        *self
            .host
            .live_call_jobs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = rest;
    }
}
pub mod retention;
pub mod session;
pub mod stdio;
pub mod tools;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol as acp;
use async_trait::async_trait;
use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::journal::JournalRecorder;
use orz_tools::computer::local::LocalTerminalBackend;
use orz_tools::computer::types::TerminalBackend;
use orz_tools::implementations::web_search::WebSearchConfig;
// NOTE: `PermitError` (orz_loop::host) is the LoopHost contract error; the
// assurance permit error is aliased to keep the two distinct.
use orz_assurance::permit::{
    PermitEnvelope, PermitError as PermitSigningError, PermitSigner, PermitStore,
    SensitiveActionPermit,
};
use orz_loop::host::{
    LoopHost, PermitDecision, PermitError, PermitSource, RiskClass, ToolError, ToolRegistry,
    ToolResult,
};

use crate::keystore::MemoryInstallationKeyStore;
use crate::permission::{PermissionBridge, PermissionPolicy};
use crate::tools::ToolsetRegistry;
use orz_workspace::permission::PermissionHookTransport;

/// P0-1 (2026-08-08 stall guards): per-tool-call wall-clock budget. A tool
/// that exceeds it is terminated (process tree killed) and the call fails
/// with `ToolError::Timeout` — the run continues with the model notified.
/// Default 5 minutes; configurable via [`OrzHost::with_tool_timeout`].
pub const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(300);

/// Full LoopHost implementation over the Grok providers.
///
/// Wires the journal, the finalized toolset, the workspace-trust observation,
/// and the IP6 permission bridge into the orz-loop contract. Persistence,
/// hooks, MCP, and credentials remain default stubs (chat-state/hooks wiring
/// in Phase 3).
pub struct OrzHost {
    journal: JournalRecorder,
    registry: ToolsetRegistry,
    workspace_trust: WorkspaceTrust,
    permission: Option<PermissionBridge>,
    /// Keystore-backed P1 permit signer (DPAPI install key; test-only
    /// memory store when not injected). Consumed by `issue_permit`.
    permit_signer: Arc<dyn PermitSigner>,
    /// Root for permit artifacts — `{cwd}/.gsa/` (mirrors the Python
    /// namespace layout `<root>/one_shot_permit/`).
    permit_store_root: PathBuf,
    /// Live-client session id (ACP `session_notification` target for
    /// streamed text deltas). `None` = headless — deltas are no-ops.
    session_id: Option<String>,
    /// Live-client outbound gateway (clone; the original goes to the
    /// permission bridge). `None` = headless — deltas are no-ops.
    gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
    /// D-9 (FIX_PLAN 2026-08-06): fixed test-runner command (Aider-model
    /// feedback loop) — `Some` declares the `run_tests` tool to the model;
    /// the command is host-owned and the test files stay hidden.
    test_runner: Option<orz_loop::host::TestRunner>,
    /// P0-1 (2026-08-08 stall guards): per-call wall-clock budget for tool
    /// execution (default `TOOL_CALL_TIMEOUT`). On expiry the tool's
    /// process tree is killed and the call fails with `ToolError::Timeout`.
    tool_timeout: Duration,
    /// Session working directory — the run_tests command's cwd.
    cwd: PathBuf,
    /// TER T1.6 (2026-09-04): 终端 backend 句柄——黑板 `section=processes`
    /// live 分区事实源（与工具集共用同一实例，读取时现算）。
    terminal: Arc<dyn TerminalBackend>,
    /// TER 全面审查 P1-1 (2026-09-04)：已向 loop 上报过的 idle-kill 任务 id
    /// （drain 去重——同一任务只上报一次，防止重复 `tool_running` 事件）。
    idle_kill_reported: std::sync::Mutex<std::collections::HashSet<String>>,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10) + GAP-PROJECT-DOC-INDEX-CACHE
    /// (2026-08-11): the project-doc index — the internal retrieval lane's
    /// real discovery/query tool (ADR-0010 §3.7.4/§3.7.5; host-owned,
    /// run_tests precedent). Holds an in-process diff snapshot, persisted
    /// across runs at `{cwd}/.gsa/project-doc-index/cache.json`.
    project_doc_index: crate::project_doc_index::ProjectDocIndex,
    /// ADR-0006 (2026-08-11): the web_search client config — single source
    /// of truth built once from the credential reader (was: env-only bool).
    /// Drives the framework_fallback capability probe (ADR-0010 §3.7.1;
    /// never a silent fallback). The secret may only leave via
    /// [`OrzHost::web_search_config_redacted`].
    web_search_config: WebSearchConfig,
    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): the global `web_search`
    /// concurrency gate (ADR-0010 §3.7.7/§11.3 — global web_search
    /// concurrency is fixed at 1; the main agent and the external-retrieval
    /// subagent share this single semaphore, internal retrieval does not
    /// touch it). `call_tool` acquires one permit per `web_search*` call and
    /// drops it when the call future ends (timeout included) — the permit is
    /// owned by the tokio future, so the P0-1 timeout drop releases it
    /// automatically.
    web_search_semaphore: Arc<tokio::sync::Semaphore>,
    /// local_browser (2026-08-10) + 0t (2026-09-09): the session's browser
    /// lane handle — `Mutex` 使 browser_read 调用期（`&self`）可按需懒启动
    /// 并换入真实 manager（启动/探活尝试落 `browser_launch_result` 事实
    /// 事件）。Defaults to a fail-closed [`UnavailableBrowserSession`]。
    browser: Arc<std::sync::Mutex<crate::local_browser::SharedBrowser>>,
    /// S2-R P3 / P2-1（2026-09-09）：懒启动 check+launch 串行锁——首次
    /// 并发 browser 调用的双重检查锁定：持锁期间二次检查 readiness，避免
    /// 双调用同见 not-ready、同 profile 拉第二个 Chrome（撞 SingletonLock
    /// 误报 `BrowserLaunchFailed`）。只串行「检查+启动」窗口；启动完成后
    /// 的页面动作仍可并发（browser_read tab 池语义不变）。
    browser_launch_lock: tokio::sync::Mutex<()>,
    /// 0aw（2026-09-20，HOST_RESOURCE_OS_DELEGATION_DESIGN §6）：派发前
    /// 目标卷读数 ＋ 卷软提示面（原「派发前资源预检门」——准入拒绝已退役，
    /// 见 [`crate::resource_hint`]）。`None` = 未注入（测试与嵌入式宿主
    /// 维持零行为变化）；生产装配（`-p` 与 ACP 两路）经
    /// [`OrzHost::with_host_resource_safety`] 注入真实探针并同时安装 run
    /// 硬上限。
    resource_hint: Option<crate::resource_hint::ResourceHint>,
    /// 0z S2 §4.2：进程树登记表（`.gsa/process_trees/`）——装配期扫除
    /// 上轮孤儿 + 工具调用期登记 + facts drain（loop 侧 journal 面）。
    process_trees: Option<crate::process_tree::ProcessTreeRegistry>,
    /// 0z S2 journal facts 暂存（`host_resource_snapshot`；原
    /// reclaim_performed / resource_exhausted / host_resource_denied 生产端
    /// 已随 0aw 裁决 ④ 退役——schema/verifier 保留供历史 journal 校验）。
    resource_facts: std::sync::Mutex<Vec<serde_json::Value>>,
    /// 0z S2R F-BE-3(a)（2026-09-13 用户裁决）：在跑调用的 call job
    /// 句柄登记——`DispatchGuard::drop` 据此关闭本次调用的 per-call job
    /// 句柄（调用级拆树）。条目随派发结束由 DispatchGuard 摘除并关闭
    /// 复制句柄。
    live_call_jobs: std::sync::Arc<std::sync::Mutex<Vec<LiveCallJob>>>,
    /// 派发序号源——`DispatchGuard` 的 token（作用域：本次调用的 live
    /// call job 条目）。
    dispatch_token: std::sync::atomic::AtomicU64,
    /// 上一次读到的档位（u8 编码，见 `tier_rank`）——跨档才落
    /// `host_resource_snapshot`（§4.5 低频，跨档才落；review F-EV-7）。
    last_resource_tier: std::sync::atomic::AtomicU8,
    /// 0bc 裁决 ②（2026-09-20）：commit 临限软提示**每 run 至多一条**的
    /// 去重位（内核通知可取多次，模型面提示只首件挂一次）。
    commit_hint_emitted: std::sync::atomic::AtomicBool,
}

impl OrzHost {
    /// Build a host for a session working directory.
    ///
    /// `workspace_trust` comes from the session bootstrap (see
    /// `session::check_workspace_trust`). No permission bridge — every
    /// `request_permission` fails closed to `Deny` (LoopHost default,
    /// review P1-1: a host that forgets to wire the bridge must not
    /// silently auto-allow `bash`).
    pub fn new(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
    ) -> Result<Self, String> {
        Self::with_permission(journal, cwd, workspace_trust, None)
    }

    /// Build a host with an optional IP6 permission bridge.
    ///
    /// `Some(bridge)` delegates `request_permission` to the Grok permission
    /// manager (默认自动审批——初始 yolo，用户令 2026-09-26；policy 短路与
    /// 无桥 fail-closed 默认不变); `None` keeps the
    /// fail-closed default.
    pub fn with_permission(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        permission: Option<PermissionBridge>,
    ) -> Result<Self, String> {
        Self::with_credential_reader(
            journal,
            cwd,
            workspace_trust,
            permission,
            crate::credentials::web_search_reader(),
        )
    }

    /// ADR-0006 (2026-08-11): build a host with an injected credential
    /// reader — the test seam. A failing reader fixes `web_search` to
    /// Disabled regardless of the platform (so semaphore tests fast-fail
    /// even on a machine that has registered `orz-grok/search`).
    pub(crate) fn with_credential_reader(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        permission: Option<PermissionBridge>,
        reader: Arc<dyn crate::credentials::CredentialReader>,
    ) -> Result<Self, String> {
        // 0p S2 复审（B5 已知 key 字面替换，2026-09-07，ADR-0010 §14.61）：
        // 装配期一次性扫描环境变量，把 KEY/TOKEN/SECRET/PASSWORD 命名的
        // 候选密钥值登记进 orz-secrets 已知密钥注册表（幂等；注册表空时
        // 脱敏行为与纯 shape 检测一致）。模型 API key 另由
        // read_agent_api_key 成功路径显式登记（覆盖 Windows 凭据管理器
        // 通道）。
        orz_secrets::register_known_secrets_from_env();
        let web_search_config = tools::web_search_config(reader.as_ref());
        // ORZ-LARGE-FILE-READ-CONTRACT (ADR-0010 §14.22): the `[toolset.read_file]`
        // coarse gate is resolved once at host build from the effective config
        // (system-managed > managed > user merge). Absent → None → the tool
        // falls back to the `ORZ_READ_FILE_COARSE_GATE_BYTES` env var at call
        // time; present → the injected tool param wins over the env var.
        let read_file_coarse_gate_bytes = orz_config::load_effective_config_disk_only()
            .ok()
            .as_ref()
            .and_then(tools::read_file_coarse_gate_from_config);
        if let Some(gate) = read_file_coarse_gate_bytes {
            tracing::info!(
                read_file_coarse_gate_bytes = gate,
                "toolset.read_file coarse gate applied"
            );
        }
        let terminal: Arc<dyn TerminalBackend> = Arc::new(LocalTerminalBackend::new());
        let toolset = tools::build_toolset(
            cwd,
            &web_search_config,
            read_file_coarse_gate_bytes,
            terminal.clone(),
        )?;
        // ADR-0006 (2026-08-11): the only sanctioned serialization exit for
        // the config — never log the raw `WebSearchConfig` (its `Debug`
        // contains the api_key; `redacted()` is the production surface).
        tracing::info!(
            "web_search client configured: {:?}",
            web_search_config.redacted()
        );
        Ok(Self {
            journal,
            registry: ToolsetRegistry::new(toolset),
            workspace_trust,
            permission,
            // Default: test-only memory signer (no permit is issued unless
            // `issue_permit` is called) — the session bootstrap injects the
            // DPAPI keystore-backed signer via `with_permit_signer`.
            permit_signer: Arc::new(MemoryInstallationKeyStore::new()),
            permit_store_root: cwd.join(".gsa"),
            session_id: None,
            gateway: None,
            test_runner: None,
            tool_timeout: TOOL_CALL_TIMEOUT,
            cwd: cwd.to_path_buf(),
            terminal,
            idle_kill_reported: std::sync::Mutex::new(std::collections::HashSet::new()),
            project_doc_index: crate::project_doc_index::ProjectDocIndex::new(cwd.to_path_buf()),
            web_search_config,
            web_search_semaphore: Arc::new(tokio::sync::Semaphore::new(1)),
            browser: Arc::new(std::sync::Mutex::new(Arc::new(
                crate::local_browser::UnavailableBrowserSession::new(
                    "no browser handle injected".to_string(),
                ),
            ))),
            browser_launch_lock: tokio::sync::Mutex::new(()),
            resource_hint: None,
            process_trees: None,
            resource_facts: std::sync::Mutex::new(Vec::new()),
            live_call_jobs: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            dispatch_token: std::sync::atomic::AtomicU64::new(0),
            last_resource_tier: std::sync::atomic::AtomicU8::new(0),
            commit_hint_emitted: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// 0aw（2026-09-20，HOST_RESOURCE_OS_DELEGATION_DESIGN §6；原 0z S1
    /// 装配语义由「门 ＋ 上限」改为「上限 ＋ 卷软提示」）：装配期注入资源面
    /// ——①安装 run 级硬上限（`JOB_OBJECT_LIMIT_JOB_MEMORY` commit /
    /// `ACTIVE_PROCESS` 并发 / CPU hard cap，内核强制，覆盖此后每个工具调用
    /// 的进程树）；②挂上派发前目标卷读数 ＋ 卷软提示面（**不做准入**——
    /// 内存/CPU 的调度与限额交操作系统，磁盘余量降为软提示，拒绝臂/树杀
    /// 臂/回收触发随裁决 ④ 一并退役）。读数不可得时上限照装（commit 上限
    /// 缺席记录在案），软提示静默。
    ///
    /// 生产装配点是 `orz-bin` 的 `-p` 路径与 `acp_server` 的 ACP 路径；测试
    /// 与嵌入式宿主不调用本方法 → 行为与本批之前逐字一致。
    pub fn with_host_resource_safety(mut self) -> Self {
        self.install_resource_safety(None);
        self
    }

    /// Test seam for the end-to-end "the tool's process tree really is inside
    /// the bounded run job" check: same wiring as
    /// [`OrzHost::with_host_resource_safety`], but the run ceilings are given
    /// explicitly (a deliberately small commit ceiling is the only way to
    /// observe kernel enforcement without a full-size machine).
    pub fn with_host_resource_safety_limits(mut self, limits: xai_tty_utils::JobLimits) -> Self {
        self.install_resource_safety(Some(limits));
        self
    }

    /// Shared assembly: probe once, install the run job, wire the hint face.
    fn install_resource_safety(&mut self, limits_override: Option<xai_tty_utils::JobLimits>) {
        let probe = Arc::new(crate::resource_hint::SystemCapacityProbe);
        let snapshot = crate::resource_hint::CapacityProbe::probe(probe.as_ref(), &self.cwd);
        let limits =
            limits_override.unwrap_or_else(|| crate::resource_hint::default_job_limits(&snapshot));
        let installed = if limits_override.is_some() {
            xai_tty_utils::replace_global_run_job_for_tests(limits)
        } else {
            xai_tty_utils::install_global_run_job(limits)
        };
        match installed {
            Ok(job) => tracing::info!(
                ceilings = %job.describe(),
                "host resource ceilings installed (kernel-enforced run job; 0aw)"
            ),
            Err(e) => tracing::warn!(
                error = %e,
                "host resource ceilings unavailable — the OS answer surface \
                 (allocation failure / OOM) is the remaining backstop"
            ),
        }
        tracing::info!(
            headroom = %snapshot.describe(),
            tier = %crate::resource_hint::tier_for(&snapshot).as_str(),
            "host resource observation installed (0aw; no admission gate)"
        );
        self.resource_hint = Some(crate::resource_hint::ResourceHint::new(probe));

        // 0z S2 §4.2 item 4 (start sweep): reap orphans left by crashed
        // previous runs BEFORE this run spawns anything. Records newer than
        // "now" cannot exist yet, so the window boundary is the assembly
        // instant; facts are stashed for the loop's journal face.
        let registry = crate::process_tree::ProcessTreeRegistry::new(&self.cwd);
        let records = registry.load_all();
        if !records.is_empty() {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);
            let decisions = registry.plan_sweep(
                &records,
                crate::process_tree::SweepMode::Start {
                    run_started_at: now_ms,
                },
                &crate::process_tree::pid_alive,
                &crate::process_tree::image_hash_of_pid,
                &crate::process_tree::creation_time_of_pid,
            );
            registry.execute_sweep(decisions, "parent_abort", &crate::process_tree::kill_pid);
        }
        self.process_trees = Some(registry);

        // §4.5 (review F-EV-7): the run-start reading row — the snapshot
        // family's second trigger, emitted once at assembly.
        let snapshot = crate::resource_hint::CapacityProbe::probe(
            &crate::resource_hint::SystemCapacityProbe,
            &self.cwd,
        );
        self.resource_facts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(serde_json::json!({
                "event": "host_resource_snapshot",
                "tier": crate::resource_hint::tier_for(&snapshot).as_str(),
                "trigger": "run_start",
                "readings": snapshot.to_json(),
            }));
        self.last_resource_tier.store(
            tier_rank(&crate::resource_hint::tier_for(&snapshot)),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// The run id this host journals under (derived from the journal
    /// directory name — `.gsa/runs/{run_id}`), used to scope registry rows
    /// and the finalize sweep (review F-BE-1).
    fn own_run_id(&self) -> String {
        self.journal
            .journal_dir()
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Open this call's dispatch guard: the token scopes its live call-job
    /// entries; the guard closes the duplicated per-call job handles on drop
    /// (`KILL_ON_JOB_CLOSE` 的调用级拆树，0aw 裁决 ④ 保留面).
    fn register_dispatch(&self) -> DispatchGuard<'_> {
        let token = self
            .dispatch_token
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        DispatchGuard {
            host: self,
            token,
            closed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn close_job_handle(handle: isize) {
        #[cfg(windows)]
        if handle != 0 {
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(
                    windows::Win32::Foundation::HANDLE(handle as _),
                );
            }
        }
        #[cfg(not(windows))]
        let _ = handle;
    }

    /// 0bl 审查修复（2026-09-24）：超时杀树的定向原语——对本次调用登记的
    /// duplicated job 句柄做 `TerminateJobObject`（调用级拆树，含孙进程）。
    /// 与 `close_job_handle` 同款平台门：非 Windows（句柄恒 0）为 no-op。
    /// 只终止、不关闭句柄——关闭仍由 `DispatchGuard::drop` 统一做
    /// （KILL_ON_JOB_CLOSE 语义不受影响）。
    fn terminate_job_handle(handle: isize) {
        #[cfg(windows)]
        if handle != 0 {
            unsafe {
                let _ = windows::Win32::System::JobObjects::TerminateJobObject(
                    windows::Win32::Foundation::HANDLE(handle as _),
                    1,
                );
            }
        }
        #[cfg(not(windows))]
        let _ = handle;
    }

    /// Drain the accumulated resource facts (journal face, loop side).
    pub fn drain_resource_facts(&self) -> Vec<serde_json::Value> {
        std::mem::take(
            &mut *self
                .resource_facts
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    /// The process-tree registry, when one is wired (0z S2 §4.2).
    pub fn process_tree_registry(&self) -> Option<&crate::process_tree::ProcessTreeRegistry> {
        self.process_trees.as_ref()
    }

    /// 0z S2 §4.2 (finalize sweep): reap this run's own leaked children at
    /// run end — records whose parent chain is this very process (still
    /// alive during the run, refused by the start sweep) are swept at
    /// shutdown when the run's tree should be gone but a breakaway/attach
    /// failure escaped both jobs.
    pub fn finalize_process_trees(&self) {
        let Some(registry) = self.process_trees.as_ref() else {
            return;
        };
        let records = registry.load_all();
        if records.is_empty() {
            return;
        }
        // Review F-BE-1 (2026-09-13): ONLY this run's rows are candidates;
        // foreign rows (parallel instances, older runs) are skipped — their
        // owner reaps them via its own finalize or the next assembly sweep.
        let decisions = registry.plan_sweep(
            &records,
            crate::process_tree::SweepMode::Finalize {
                own_run_id: &self.own_run_id(),
                own_pid: std::process::id(),
            },
            &crate::process_tree::pid_alive,
            &crate::process_tree::image_hash_of_pid,
            &crate::process_tree::creation_time_of_pid,
        );
        registry.execute_sweep(decisions, "run_shutdown", &crate::process_tree::kill_pid);
    }

    /// The injected resource hint face, when one is wired (0aw: observation
    /// + soft hint; no admission).
    pub fn resource_hint(&self) -> Option<&crate::resource_hint::ResourceHint> {
        self.resource_hint.as_ref()
    }

    /// Inject a specific hint face (test seam + hosts that bring their own
    /// probe).
    pub fn with_resource_hint(mut self, hint: crate::resource_hint::ResourceHint) -> Self {
        self.resource_hint = Some(hint);
        self
    }

    /// local_browser (2026-08-10) + 0t (2026-09-09, S2-R P3 / P2-3): inject
    /// the session's browser lane handle (the capability probe calls this
    /// with a launched manager; tests and conformance captures inject
    /// fakes)。注入**不影响**声明——0t 静态双族语义下 `browser_read` 声明
    /// 由检索启用门（`declare_browser_declared`）决定，句柄 readiness 只
    /// 驱动调用期懒启动。
    pub fn with_browser_session(mut self, browser: crate::local_browser::SharedBrowser) -> Self {
        self.set_browser_session(browser);
        self
    }

    /// local_browser (2026-08-10): in-place variant for the async probe
    /// (which holds `&mut self`).
    pub fn set_browser_session(&mut self, browser: crate::local_browser::SharedBrowser) {
        *self
            .browser
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = browser;
    }

    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1/§3.3): 检索启用会话把
    /// `browser_read` 声明为常驻（静态双族工具面——浏览器缺席时调用按普通
    /// 失败回传，不再以探活结果裁剪声明）。调用期懒启动成功后由
    /// [`OrzHost::swap_browser_session`] 换入真实句柄。
    pub fn declare_browser_declared(&mut self) {
        self.registry.set_browser_declared(true);
    }

    /// 0t: 调用期懒启动成功后换入真实 manager（`&self` 安全——句柄在
    /// `Mutex` 内）。
    pub fn swap_browser_session(&self, browser: crate::local_browser::SharedBrowser) {
        *self
            .browser
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = browser;
    }

    /// S2-R P3 / P2-1 + P1-2b（2026-09-09）：浏览器车道懒启动——check +
    /// launch 在 `browser_launch_lock` 内二次检查（双重检查锁定）：并发首调
    /// 只有一个执行 probe_launch；先完成者换入真实句柄后，等待者二次检查
    /// 见 ready、直接跳过启动（S4 无 fact）。锁在启动完成即释放，不串行
    /// 页面动作（browser_read tab 池并发不变）。返回 `true` = 本次调用完成
    /// 启动（调用方据此落 launch fact）。
    async fn ensure_browser_launched(&self) -> Result<bool, ToolError> {
        self.ensure_browser_launched_with(|cwd, session_key| {
            let cwd = cwd.to_path_buf();
            let session_key = session_key.to_string();
            async move {
                let manager = crate::local_browser::probe_launch(&cwd, &session_key).await?;
                let shared: crate::local_browser::SharedBrowser = Arc::new(manager);
                Ok::<_, String>(shared)
            }
        })
        .await
    }

    /// S2-R P3 / P2-1 复审处理 X3（2026-09-09）：启动 seam——生产路径委托
    /// [`crate::local_browser::probe_launch`]；测试注入计数启动器，可观测
    /// 「并发首调同一 profile 只拉起一次」（结构正确性由串行锁 + 二次检查
    /// 保证，此处把该语义变成可断言行为）。
    async fn ensure_browser_launched_with<F, Fut>(&self, launch: F) -> Result<bool, ToolError>
    where
        F: Fn(&std::path::Path, &str) -> Fut,
        Fut: std::future::Future<Output = Result<crate::local_browser::SharedBrowser, String>>,
    {
        let mut launch_attempted = false;
        {
            let _launch_guard = self.browser_launch_lock.lock().await;
            if !self
                .browser
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .ready()
            {
                launch_attempted = true;
                // profile 目录键沿用既有约定：ACP 会话 id 前 8 位；CLI
                // 一次性运行（无 live session id）用稳定 "cli"。
                let session_key = self.session_id.clone().unwrap_or_else(|| "cli".to_string());
                match launch(&self.cwd, &session_key).await {
                    Ok(browser) => self.swap_browser_session(browser),
                    Err(cause) => return Err(ToolError::BrowserLaunchFailed(cause)),
                }
            }
        }
        Ok(launch_attempted)
    }

    /// Whether the browser lane is ready——调用期懒启动的判定源（就绪则
    /// 跳过 probe_launch，否则按需启动；不再驱动工具声明）。
    pub fn browser_ready(&self) -> bool {
        self.browser
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .ready()
    }

    /// The browser lane handle (for the probe and shutdown paths).
    pub fn browser_session(&self) -> crate::local_browser::SharedBrowser {
        self.browser
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): whether the web_search client is
    /// configured (the framework_fallback capability probe).
    pub fn web_search_configured(&self) -> bool {
        self.web_search_config.is_enabled()
    }

    /// ADR-0006 (2026-08-11): the redacted web_search config — the only
    /// sanctioned serialization exit for the client configuration (the
    /// api_key is replaced with `***REDACTED***`; the raw config's `Debug`
    /// contains the key and must never be logged or journaled).
    pub fn web_search_config_redacted(&self) -> Option<WebSearchConfig> {
        self.web_search_config
            .is_enabled()
            .then(|| self.web_search_config.redacted())
    }

    /// P0-1 (2026-08-08 stall guards): set the per-tool-call wall-clock
    /// budget. Default `TOOL_CALL_TIMEOUT` (5 min) — a tool that exceeds it
    /// is terminated and the call fails with `ToolError::Timeout`; the run
    /// continues and the model is notified (never silently dropped).
    pub fn with_tool_timeout(mut self, timeout: Duration) -> Self {
        self.tool_timeout = timeout;
        self
    }

    /// D-9 (FIX_PLAN 2026-08-06): inject a fixed test-runner command —
    /// declares the `run_tests` tool (Aider-model feedback loop). The
    /// command is host-owned argv; the test files stay hidden from the model.
    pub fn with_test_runner(mut self, runner: Option<orz_loop::host::TestRunner>) -> Self {
        self.test_runner = runner;
        self
    }

    /// Inject the session's keystore-backed permit signer (see
    /// `session::bootstrap_session`). Without injection the host signs with
    /// a fresh in-memory key per run.
    pub fn with_permit_signer(mut self, signer: Arc<dyn PermitSigner>) -> Self {
        self.permit_signer = signer;
        self
    }

    /// The active permit signer (for inspection / verification).
    pub fn permit_signer(&self) -> &Arc<dyn PermitSigner> {
        &self.permit_signer
    }

    /// Issue a P1 one-shot sensitive-action permit signed by the
    /// keystore-backed signer, persisted under `{cwd}/.gsa/one_shot_permit/`.
    ///
    /// `envelope` carries the session's conversation/envelope identity and
    /// expiry clamp (the session security envelope in production).
    ///
    /// KNOWN GAP (2026-08-05 review P2-1): the Python authority verifies the
    /// envelope before issuance (`verify_security_envelope`: schema +
    /// `installation_key_id` match + signature). orz has no security-envelope
    /// module yet, so the envelope is caller-supplied plain data — the
    /// approval path (still a stub) must verify envelope authenticity before
    /// calling this when it lands. Until then, issuing is only sound when the
    /// caller owns the envelope construction.
    #[allow(clippy::too_many_arguments)]
    pub fn issue_permit(
        &self,
        envelope: &PermitEnvelope,
        confirmation_sha256: &str,
        action_sha256: &str,
        target_sha256: &str,
        impact_scope_sha256: &str,
        attempt: u32,
        ttl_seconds: u64,
    ) -> Result<SensitiveActionPermit, PermitSigningError> {
        PermitStore::new(self.permit_store_root.clone()).issue(
            self.permit_signer.as_ref(),
            envelope,
            confirmation_sha256,
            action_sha256,
            target_sha256,
            impact_scope_sha256,
            attempt,
            ttl_seconds,
            None,
        )
    }

    /// Build a host with an IP6 permission bridge in one step.
    ///
    /// `gateway` is the outbound ACP sender when interactive (`--stdio`);
    /// `None` (headless `-p`/`--plan`) substitutes a dead gateway — the
    /// manager 默认自动审批（初始 yolo，用户令 2026-09-26），非 yolo 的
    /// `Ask` 仍 fail closed to `Deny`.
    ///
    /// Note: `PermissionBridge::spawn` runs the manager actor via
    /// `spawn_local`, so callers must be inside a `tokio::task::LocalSet`.
    pub fn with_bridge(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
    ) -> Result<Self, String> {
        Self::with_bridge_and_hub(session_id, journal, cwd, workspace_trust, gateway, None)
    }

    /// `with_bridge` variant that additionally threads an interactive
    /// permission transport (`hub` — Phase 3 slice #12: the codex app-server
    /// approval surface). `None` keeps the ACP-gateway behavior; the rest is
    /// identical.
    pub fn with_bridge_and_hub(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
        hub: Option<Arc<dyn PermissionHookTransport>>,
    ) -> Result<Self, String> {
        Self::with_bridge_and_hub_policy(
            session_id,
            journal,
            cwd,
            workspace_trust,
            gateway,
            hub,
            PermissionPolicy::Interactive,
        )
    }

    /// `with_bridge_and_hub` variant that additionally fixes the per-session
    /// permission policy (slice #16): a codex read-only thread passes
    /// `PermissionPolicy::ReadOnly` — mutations/network are denied without
    /// prompting; `Interactive` keeps the historical behavior.
    pub fn with_bridge_and_hub_policy(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
        hub: Option<Arc<dyn PermissionHookTransport>>,
        policy: PermissionPolicy,
    ) -> Result<Self, String> {
        // Clone the live-client sender before handing the original to the
        // bridge — the streamed text-delta path uses its own sender clone
        // (the permission manager gets the original).
        let live_gateway = gateway.clone();
        let bridge =
            PermissionBridge::spawn_with_hub_and_policy(session_id, gateway, hub, cwd, policy)?;
        let mut host = Self::with_permission(journal, cwd, workspace_trust, Some(bridge))?;
        host.session_id = Some(session_id.to_string());
        host.gateway = live_gateway;
        Ok(host)
    }

    /// The underlying finalized toolset (for direct dispatch).
    pub fn toolset(&self) -> &Arc<orz_tools::registry::types::FinalizedToolset> {
        self.registry.toolset()
    }
}

/// Stability fix (2026-08-07): terminate the child AND its process tree.
/// `tokio::process::Child::kill` (= TerminateProcess) only kills the direct
/// child — grandchildren that inherited our capture pipes survive as
/// orphans and keep `read_capped` blocked on EOF forever (the 52-minute
/// forth hang: harness killed orz, orz's hung pytest held the pipes).
/// Windows: TaskKill `/T /F` terminates the whole tree (and closes the
/// pipe handles, releasing the readers). ORDER MATTERS: TaskKill `/T`
/// walks ParentProcessId links, so it must run while the tree is intact —
/// killing the parent first reparents the grandchildren (Windows orphans
/// get reparented to the system process) and `/T` then finds nothing.
/// Non-Windows: plain kill remains (recorded limitation — the project
/// runtime is Windows-first).
async fn kill_process_tree(child: &mut tokio::process::Child) {
    #[cfg(windows)]
    {
        if let Some(id) = child.id() {
            let tk = tokio::process::Command::new("taskkill")
                .args(["/PID", &id.to_string(), "/T", "/F"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if let Ok(mut tk) = tk {
                let _ = tk.wait().await;
            }
        }
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

/// F-09 (2026-08-07 review): collect a streamed pipe with a hard cap — the
/// TAIL is kept (test summaries live at the end); leading bytes are dropped
/// on overflow so pathological output cannot blow session memory. A closed
/// or failed pipe simply ends the collection.
async fn read_capped<R: tokio::io::AsyncRead + Unpin>(mut reader: R, buf: &mut Vec<u8>) {
    use tokio::io::AsyncReadExt;
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > orz_loop::host::RUN_TESTS_OUTPUT_CAP {
                    let excess = buf.len() - orz_loop::host::RUN_TESTS_OUTPUT_CAP;
                    buf.drain(..excess);
                }
            }
        }
    }
}

/// RT-002 (2026-08-11): the fixed test command's minimal environment
/// allowlist — the ONLY host environment the test process inherits (plus the
/// harness's explicit `TestRunner::env` entries). Everything else is cleared:
/// host secrets, `ORZ_*` session variables, arbitrary paths.
#[cfg(windows)]
const TEST_ENV_ALLOWLIST: &[&str] = &[
    // PATH: command lookup (the runner command often names an interpreter
    // by bare name). SystemRoot/PATHEXT/COMSPEC: Windows process bootstrap
    // (DLL search, batch invocation). TEMP/TMP: temp files. USERPROFILE:
    // many tools want a writable home for caches.
    "PATH",
    "SystemRoot",
    "PATHEXT",
    "COMSPEC",
    "TEMP",
    "TMP",
    "USERPROFILE",
];
#[cfg(not(windows))]
const TEST_ENV_ALLOWLIST: &[&str] = &[
    // PATH: command lookup. HOME: cache/config dirs. TMPDIR: temp files.
    // LANG: locale output stability.
    "PATH", "HOME", "TMPDIR", "LANG",
];

/// RT-003 (2026-08-11): directories excluded from the run_tests delta walk —
// RT-003 / AGENT-DELIVERY-FLOW (2026-08-23): the workspace-delta walk and
// diff are single-sourced in orz-loop (`host.rs`); orz-host imports them.
use orz_loop::host::{
    RUN_TESTS_DELTA_MAX_ENTRIES, TOOL_DELTA_MAX_ENTRIES, workspace_delta_diff, workspace_delta_walk,
};

/// 0t P1-2a（2026-09-09, S2-R P2 / 设计 §3.2）：浏览器工具调用收尾装配——
/// 启动尝试事实独立于动作结果。`launch_attempted` 由调用点如实上报：
///
/// - 启动成功 + 动作成功 → Ok 附带 success fact（S2，loop 在 ToolCompleted
///   前落 `browser_launch_result`）；
/// - 启动成功 + 动作失败 → `BrowserStepFailed` 携带 success fact（S3，新增
///   Err 接缝，页面/导航真实错误原样进 reason，FP-2 正常回传）；
/// - 未启动的普通失败 → 原样返回（S4，不包装）。
///
/// 启动失败不经本函数（调用点直接 `BrowserLaunchFailed` 返回，S1）。
fn finish_browser_call(
    launch_attempted: bool,
    result: Result<ToolResult, ToolError>,
) -> Result<ToolResult, ToolError> {
    match (launch_attempted, result) {
        (true, Ok(mut res)) => {
            res.browser_launch_fact = Some(orz_loop::host::BrowserLaunchFact::success());
            Ok(res)
        }
        (true, Err(e)) => Err(ToolError::BrowserStepFailed {
            reason: e.to_string(),
            launch_fact: orz_loop::host::BrowserLaunchFact::success(),
        }),
        (false, res) => res,
    }
}

impl OrzHost {
    /// P0-C S4 (2026-08-16): shared tool-execution core with an optional
    /// per-call timeout override (script step deadlines). `None` = the
    /// configured host budget; an override is capped at the configured
    /// budget (`min`) so the host ceiling can never be raised by callers.
    async fn call_tool_inner(
        &self,
        name: &str,
        args: serde_json::Value,
        call_id: &str,
        timeout_override: Option<std::time::Duration>,
    ) -> Result<ToolResult, ToolError> {
        // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.3): host tools that can
        // mutate the worktree in ways the model cannot see from the output
        // (terminal commands) carry a per-call workspace delta on the
        // result — the console receipt attaches it. Read-only tools skip
        // the full-tree walk (cost); run_tests has its own dedicated delta
        // path (TestRunResult), and search_replace is covered by its edit
        // diff + EditRecord.
        let delta_tracked = matches!(name, "run_terminal_cmd");
        let delta_before = if delta_tracked {
            Some(workspace_delta_walk(&self.cwd))
        } else {
            None
        };
        // P0-1 (2026-08-08 stall guards): bounded tool execution. Every
        // tool call runs under a wall-clock cap (default 5 min,
        // configurable via `with_tool_timeout`) — a tool whose
        // implementation awaits forever (a hung bash child, an
        // `ask_user_question` with no user attached, a stuck fs read, …)
        // used to hang the whole run with no bound anywhere (the
        // dna-assembly 16:02 hang attribution). On expiry the tool's
        // process tree is killed via the global process scope (Windows:
        // per-child Job Object `TerminateJobObject` — kills grandchildren
        // too, the 2026-08-07 orphan-holds-pipes mechanism) and the call
        // fails with `ToolError::Timeout`; the controller journals
        // `tool_completed{status:error}` and the model continues the loop.
        //
        // Recorded trade-off: the kill is process-global — any concurrently
        // running tool child (e.g. an earlier background command) is
        // terminated too. A hung tool poisons the session; leaving
        // orphaned processes behind is worse. The bash tool itself carries
        // a foreground timeout (120s default), so this wrapper is the
        // coarse backstop for every tool, bash included.
        // GAP-RETRIEVAL-TOOLS (2026-08-10): the project-doc index is a
        // host-owned tool (run_tests precedent) — routed before the
        // finalize toolset; synchronous and workspace-local.
        if name == "project_doc_index" {
            return self.project_doc_index.query(&args);
        }
        // local_browser (2026-08-10) + 0t (2026-09-09, ADR-0010 §14.65 /
        // 设计 §3.3/§3.5) + S2-R P3 / P1-2b：`browser_read` / `browser_control`
        // 是 host-owned（浏览器车道 = 会话状态）。启用门在 controller
        // （`is_retrieval_mode_gated_host_tool`）；此处只执行——无句柄时
        // 经 [`OrzHost::ensure_browser_launched`] 按需懒启动（P2-1 双重
        // 检查锁定）。启动失败按普通 host 错误回传（`BrowserLaunchFailed`，
        // loop 落 failure fact）；页面级失败走各 handler 显式错误（FP-2 正常
        // 回传）。P1-2a：启动尝试事实独立于动作结果——启动成功但动作失败
        // 也落 success fact（`BrowserStepFailed` 携带，设计 §3.2 场景 S3）。
        if matches!(name, "browser_read" | "browser_control") {
            let launch_attempted = self.ensure_browser_launched().await?;
            let browser = self
                .browser
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            let result = match name {
                "browser_read" => {
                    crate::local_browser::handle_browser_read(browser.as_ref(), &args).await
                }
                _ => crate::local_browser::handle_browser_control(browser.as_ref(), &args).await,
            };
            return finish_browser_call(launch_attempted, result);
        }
        // PDF evidence (2026-08-11): `pdf_read` reads the local evidence
        // store — synchronous, workspace-local (project_doc_index pattern).
        if name == "pdf_read" {
            return crate::pdf_evidence::handle_pdf_read(&self.cwd, &args).await;
        }
        // PDF evidence routing (2026-08-11): a `web_fetch` whose URL matches
        // ORZ_PDF_BROWSER_DOMAINS is intercepted BEFORE the toolset — the
        // whitelisted paper-library fetch happens through the browser
        // (operator login). Everything else falls through to the toolset
        // (direct channel, where PDFs are ingested inline). A whitelist hit
        // with an unavailable browser is an explicit failure — never an
        // automatic direct fallback (user ruling; §3.7.2).
        if name == "web_fetch"
            && crate::pdf_evidence::route_for_url(
                args.get("url").and_then(|u| u.as_str()).unwrap_or(""),
            )
        {
            let url = args.get("url").and_then(|u| u.as_str()).unwrap_or_default();
            let browser = self
                .browser
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            return crate::pdf_evidence::handle_browser_pdf(
                &self.cwd,
                self.session_id.as_deref(),
                browser.as_ref(),
                url,
            )
            .await;
        }
        // GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): global `web_search`
        // concurrency is fixed at 1 (ADR-0010 §3.7.7/§11.3 — the main agent
        // and the external-retrieval subagent share this semaphore; internal
        // retrieval does not touch it, so internal + external sessions still
        // run in parallel). Every `web_search*` call acquires the shared
        // permit BEFORE execution. The P0-1 timeout wraps the whole
        // acquire+call future, so a WAITING call is bounded by the same
        // 300s budget, and the timeout drop releases the permit with the
        // future — a holder can never leak the gate. `web_fetch` is not
        // gated (the contract limits only web_search).
        //
        // P3-1 (review 2026-08-10, upgraded to a fix — the new timeout test
        // reproduced the mis-kill): `started_exec` distinguishes a WAITING
        // timeout from an EXECUTION timeout. A call that times out while
        // waiting for the permit holds no process tree of its own, so the
        // global `kill_active` would only destroy unrelated concurrent
        // processes (reproduced: the parallel `call_tool_timeout_kills_
        // process_tree` test lost its python child to the semaphore test's
        // 100ms waiting timeout). Only an execution timeout kills.
        // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): 分层默认超时——
        // 模型未传 `timeout` 时，宿主按命令形态注入两档默认（普通 300s /
        // 程序脚本 600s，毫秒）；显式传入以模型为准（工具层再按
        // max_timeout_secs=900 封顶）。注入只发生在执行侧，模型面不变。
        // 0aw（2026-09-20，HOST_RESOURCE_OS_DELEGATION_DESIGN §5/§6；原
        // 0z S1「派发前资源预检门」退役）：派发前取一次**目标卷**读数——
        // ①观测面：tier 跨档照旧落 `host_resource_snapshot`（tier 只是读数
        // 标签）；②软提示面：目标卷 free < 4 GiB 时**附一条机械软提示、
        // 不阻断**（每 run 每卷至多一次）。没有任何拒绝臂——重活/轻活照常
        // 派发，内存/磁盘的真实失败由 OS 以分配失败/写失败形式到达，宿主
        // 如实转达。
        let targets = crate::resource_hint::write_targets(name, &args, &self.cwd);
        // 0z S2R F-BE-3(a)：派发登记（live call job 上下文）先于执行创建
        // ——spawn sink 闭包（下方）据 token/closed 挂钩。
        let dispatch_guard = self.register_dispatch();
        let dispatch_token = dispatch_guard.token;
        let dispatch_closed = dispatch_guard.closed.clone();
        let mut volume_hint: Option<String> = None;
        if let Some(hint) = &self.resource_hint {
            let readings = hint.read_for_volumes(&targets);
            // §4.5（review F-EV-7）：跨档才落 host_resource_snapshot。
            if let Some(binding) = hint.last_snapshot() {
                let tier = crate::resource_hint::tier_for(&binding);
                let rank = tier_rank(&tier);
                let last = self
                    .last_resource_tier
                    .swap(rank, std::sync::atomic::Ordering::Relaxed);
                if last != rank {
                    self.resource_facts
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .push(serde_json::json!({
                            "event": "host_resource_snapshot",
                            "tier": tier.as_str(),
                            "trigger": "tier_change",
                            "readings": binding.to_json(),
                        }));
                }
            }
            // 软提示（不阻断；每 run 每卷一次的机械去重在 hint 面内）。
            volume_hint = hint.soft_hint(&readings);
            if volume_hint.is_some() {
                tracing::info!(
                    tool = name,
                    write_targets = targets.len(),
                    "pre-dispatch volume soft hint attached (admission retired, 0aw)"
                );
            }
        }
        // 0bc 裁决 ②（2026-09-20）：commit 临限通知——非阻塞取件（无 run job
        // 或未装通知 ⇒ 恒空）。到件＝观测事实：①落 host_resource_snapshot
        // (trigger=commit_notification，照实落内核读数；tier 轴不属本面，
        // 如实登记机器键 "unknown")；②首件时把机械软提示挂到本次派发结果头
        // （每 run 至多一条——通知不叫停任何动作，模型照常决策）。
        let mut commit_hint: Option<String> = None;
        for notification in xai_tty_utils::drain_global_run_notifications() {
            self.resource_facts
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .push(serde_json::json!({
                    "event": "host_resource_snapshot",
                    "tier": "unknown",
                    "trigger": "commit_notification",
                    "readings": {
                        "commit_notification_bytes": notification.limit_bytes,
                        "commit_used_bytes": notification.used_bytes,
                        "limit_flags": notification.limit_flags,
                        "violation_flags": notification.violation_flags,
                    },
                }));
            if !self
                .commit_hint_emitted
                .swap(true, std::sync::atomic::Ordering::Relaxed)
            {
                commit_hint = Some(crate::resource_hint::commit_notification_hint(
                    &notification,
                ));
            }
        }
        let args = if name == "run_terminal_cmd" {
            crate::tools::inject_terminal_default_timeout(args)
        } else {
            args
        };
        let started_exec = std::sync::atomic::AtomicBool::new(false);
        // 0z S2 §4.2 item 3: the ambient spawn-sink lives exactly for this
        // dispatch — every ProcessGroup attach inside the tool writes a
        // `.gsa/process_trees/` record (pid + image fingerprint + started-at).
        // Best-effort and racy by contract: concurrent tool calls on other
        // threads may attribute under this call's id; the sweep never trusts
        // the label alone (fingerprint + window guards decide the kill).
        let process_trees_dir = self
            .process_trees
            .as_ref()
            .map(|registry| registry.dir().to_path_buf());
        let spawn_sink_guard = process_trees_dir.map(|dir| {
            let call_id = call_id.to_string();
            // Review F-BE-1: rows carry the owning run id — the finalize
            // sweep's scope guard keys on it.
            let run_id = self.own_run_id();
            let live_call_jobs = self.live_call_jobs.clone();
            xai_tty_utils::set_spawn_sink(std::sync::Arc::new(
                move |observation: xai_tty_utils::SpawnObservation| {
                    let record = crate::process_tree::ProcessTreeRecord {
                        call_id: call_id.clone(),
                        pid: observation.pid,
                        parent_chain: vec![std::process::id()],
                        image_sha256: observation.image_sha256,
                        started_at: observation.started_at,
                        run_id: run_id.clone(),
                        job_name: "call".to_string(),
                    };
                    if let Err(e) = crate::process_tree::write_record(&dir, &record) {
                        tracing::debug!("process tree registration failed: {e}");
                    }
                    // 0z S2R F-BE-3(a)：本调用的 live-call 登记——
                    // `DispatchGuard::drop` 据它关闭本次调用的 per-call job
                    // 句柄（KILL_ON_JOB_CLOSE 的调用级拆树）。0aw 裁决 ④：
                    // 原 hard 档树杀消费面已退役，登记表只为句柄关闭存续。
                    if !dispatch_closed.load(std::sync::atomic::Ordering::Relaxed) {
                        live_call_jobs
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .push(LiveCallJob {
                                token: dispatch_token,
                                call_id: call_id.clone(),
                                // Non-Windows has no job handle: register with 0 so
                                // the drop path stays uniform (close is a no-op).
                                #[cfg(windows)]
                                job_handle: observation.job_handle_dup,
                                #[cfg(not(windows))]
                                job_handle: 0,
                            });
                    } else {
                        #[cfg(windows)]
                        OrzHost::close_job_handle(observation.job_handle_dup);
                    }
                },
            ))
        });
        let fut = async {
            if crate::tools::is_web_search_tool(name) {
                tracing::debug!(
                    tool = name,
                    "web_search: acquiring the global semaphore (concurrency=1)"
                );
                let acquire = self.web_search_semaphore.clone().acquire_owned();
                // 0ac S3① (2026-09-13, design §10.3 item 1)：acquire 独立截止
                // —— 检索车道（concurrency=1，主代理与检索子代理共用）排队
                // 等待不得吃掉整个工具墙钟，也不得静默等成「零观测」。超时
                // 以自描述 cause `retrieval_lane_busy` 立即返回（工具错误
                // details.cause → `tool_completed.cause`，F-003 验收样本）。
                let _permit = match crate::tools::retrieval_lane_wait_budget() {
                    Some(budget) => match tokio::time::timeout(budget, acquire).await {
                        Ok(permit) => permit.map_err(|e| {
                            // P3-4 (review 2026-08-10): the map_tool_error
                            // bridge surfaces only the Display text — inline the
                            // code so a model never sees a bare "semaphore
                            // closed" that reads like a tool being switched off.
                            xai_tool_runtime::ToolError::custom(
                                "web_search_semaphore",
                                format!("web_search_semaphore: {e}"),
                            )
                        })?,
                        Err(_) => {
                            tracing::warn!(
                                tool = name,
                                waited_ms = budget.as_millis() as u64,
                                "web_search: retrieval lane (semaphore) acquire deadline reached"
                            );
                            return Err(xai_tool_runtime::ToolError::custom(
                                "retrieval_lane_busy",
                                format!(
                                    "retrieval_lane_busy: web_search waited {}ms for the single \
                                     retrieval lane (concurrency=1) and gave up — another \
                                     retrieval call holds the lane; this call never started",
                                    budget.as_millis()
                                ),
                            )
                            .with_details(serde_json::json!({
                                "tool_id": "web_search",
                                "cause": "retrieval_lane_busy",
                                "lane": "web_search_semaphore",
                                "concurrency": 1,
                                "waited_ms": budget.as_millis() as u64,
                            })));
                        }
                    },
                    None => acquire.await.map_err(|e| {
                        // P3-4 (review 2026-08-10): the map_tool_error
                        // bridge surfaces only the Display text — inline the
                        // code so a model never sees a bare "semaphore
                        // closed" that reads like a tool being switched off.
                        xai_tool_runtime::ToolError::custom(
                            "web_search_semaphore",
                            format!("web_search_semaphore: {e}"),
                        )
                    })?,
                };
                started_exec.store(true, std::sync::atomic::Ordering::SeqCst);
                self.registry
                    .toolset()
                    .call(name, args, call_id, None)
                    .await
            } else {
                started_exec.store(true, std::sync::atomic::Ordering::SeqCst);
                self.registry
                    .toolset()
                    .call(name, args, call_id, None)
                    .await
            }
        };
        let effective_timeout = timeout_override
            .map(|t| t.min(self.tool_timeout))
            .unwrap_or(self.tool_timeout);
        let result = match tokio::time::timeout(effective_timeout, fut).await {
            Ok(result) => result.map_err(|e| crate::tools::map_tool_error(&e))?,
            Err(_) if started_exec.load(std::sync::atomic::Ordering::SeqCst) => {
                tracing::warn!(
                    tool = name,
                    timeout = ?effective_timeout,
                    "tool call TIMED OUT — killing the tool process tree"
                );
                // 2026-08-08 review F1 (P1-1/D1-1): `kill_active` — NOT
                // `kill_all`. The latter latches the global scope closed
                // (its contract is "call only when the process is genuinely
                // exiting"); a mid-session latch would kill every LATER
                // spawn on the spot (terminal.rs ignores `register`'s
                // return), so one tool timeout would poison all subsequent
                // bash calls for the whole session.
                // 0bl 审查修复（2026-09-24）：杀树**定向化**——优先用本次
                // 派发登记的 per-call job 句柄（spawn sink 挂到
                // `live_call_jobs[token=dispatch_token]`）做
                // `TerminateJobObject`，只拆本调用的进程树（含孙进程），
                // 不再波及同进程内并发派发的无关工具进程（全局
                // `kill_active` 会误杀并发工具——见上方 P3-1 注释的自认）。
                // 兜底：句柄不可得（spawn sink 未及登记 / 未接
                // process_trees / 非 Windows 句柄恒 0）时保留全局
                // `kill_active`——杀不到比误杀更糟（孤儿进程握管道）。
                // 残余取舍（0bm 复审补记，双向如实）：① 漏杀方向——定向命中
                // 依赖 sink 登记，存在「已 spawn 未登记」的竞窗（进程树清扫
                // finalize sweep 另行兜底）；② **误杀方向**——定向性以 sink
                // 归因正确为前提，而 spawn sink 现为**全局单槽**（每次
                // call_tool_inner 覆盖安装），并发下 B 的 sink 可能以 B 的
                // token 登记 A 的子进程 job ⇒ B 超时定向杀到 A 的树（杀伤
                // 半径不大于旧全局杀，KILL_ON_JOB_CLOSE 幂等兜底）；根治随
                // 0bm ③（spawn sink per-dispatch 归因）——届时须保留全局
                // 可见杀伤面或把 per-dispatch scope 接入本杀路径（依赖声明
                // 已入 0bm 实施约束）。
                let per_call_handles: Vec<isize> = {
                    let jobs = self
                        .live_call_jobs
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    jobs.iter()
                        .filter(|entry| entry.token == dispatch_token)
                        .map(|entry| entry.job_handle)
                        .collect()
                };
                if per_call_handles.iter().any(|handle| *handle != 0) {
                    for handle in &per_call_handles {
                        Self::terminate_job_handle(*handle);
                    }
                } else {
                    orz_tools::util::global_process_scope().kill_active();
                }
                return Err(ToolError::Timeout(format!(
                    "tool '{name}' TIMED OUT after {timeout:?} wall-clock budget — \
                     process tree killed; the tool did not complete",
                    timeout = effective_timeout,
                )));
            }
            Err(_) => {
                // Waiting timeout: bounded by the same 300s budget (D-3),
                // but nothing was killed — the call never reached a tool.
                tracing::warn!(
                    tool = name,
                    timeout = ?effective_timeout,
                    "web_search call TIMED OUT while waiting for the global permit"
                );
                return Err(ToolError::Timeout(format!(
                    "tool '{name}' TIMED OUT after {timeout:?} wall-clock budget — \
                     still waiting for the global web_search permit (concurrency=1); \
                     nothing was killed, retry when the holder finishes",
                    timeout = effective_timeout,
                )));
            }
        };
        drop(spawn_sink_guard);
        let mut tool_result = ToolResult {
            // 0aw §5 模型面：软提示附在结果头部（动作照跑；每 run 每卷一次
            // ——去重在 hint 面内完成），中文短句 ＋ 英文机械读数（0af 混排）。
            // 0bc 裁决 ②：commit 临限软提示（每 run 一条）走同一头部槽位。
            output: {
                let mut head: Vec<&str> = Vec::new();
                if let Some(line) = &volume_hint {
                    head.push(line.as_str());
                }
                if let Some(line) = &commit_hint {
                    head.push(line.as_str());
                }
                if head.is_empty() {
                    result.prompt_text
                } else {
                    let mut text = head.join("\n");
                    text.push('\n');
                    text.push_str(&result.prompt_text);
                    text
                }
            },
            // 2026-08-08 blackboard-partition review closure (conformance
            // agent D1-1): the controller's edit-action gate keys on
            // `exit_code == Some(0)` ("实际变动" 才记). Previously this was
            // hardcoded `None` — the production shape never reached the
            // controller and A1/A2 (edit records + incremental push) were
            // dead in real runs; test hosts fabricating `Some(0)` masked it.
            // Map the structured output: bash carries its real exit code;
            // search_replace reports "applied" only via the EditsApplied
            // variant (NoMatchesFound etc. are Ok outputs that changed
            // nothing → non-zero); every other successful output is 0.
            exit_code: crate::tools::exit_code_from_output(&result.output),
            // GAP-ENCODING-GATE: forward the decode stage observed by the
            // tool implementation (run_terminal_cmd / read_file) to the
            // journal's `tool_completed.output_encoding`.
            output_encoding: result.output_encoding,
            // FUS-RETRIEVAL-MECH B-1 (2026-08-13): web_search citation URLs
            // ride the structured seam into the loop (candidate pool for
            // the mechanical prefilter); every other tool is `None`.
            structured: crate::tools::structured_from_output(&result.output),
            ..Default::default()
        };
        // 0p S2 两段门 / W2 D-3（2026-09-07，ADR-0010 §14.61 设计 B/C）：
        // 会话卷访问状态的 per-call 瞬态旗标转译——内部区首读通知 →
        // 结构化 `policy_denial{source=permission, code=session_volume_
        // notice}` + exit_code=1；普通读沙箱拒绝 → `policy_denial`
        // （code=outside_workspace 等）+ exit_code=1；二读放行 →
        // `session_volume_opened`（journal 记 open_after_notice）。结构化
        // seam 在资源旗标上，绝不做输出文本前缀判定
        // （FUS-CONSOLE-POLICY-DENIAL 纪律）。
        {
            let access = self
                .registry
                .toolset()
                .resources
                .lock()
                .await
                .get::<orz_tools::types::resources::SessionVolumeAccess>()
                .cloned();
            if let Some(access) = access {
                // 0p S2 复审 P2 修复（2026-09-07）：旗标按 call-id 键控——
                // 并发工具批下 take 只消费本调用的信封，绝不错配到同批
                // 其他调用。finish_call 清理条目（Err 路径漏调由旗标表
                // 容量上限兜底）。
                if access.take_notice_this_call(call_id) {
                    tool_result.exit_code = Some(1);
                    tool_result.policy_denial = Some(orz_loop::host::PolicyDenial {
                        source: orz_loop::host::PolicyDenialSource::Permission,
                        code: "session_volume_notice".to_string(),
                        reason: "session volume first access: duties and structure preview \
                                 provided; read again to open (open_after_notice)"
                            .to_string(),
                    });
                } else if let Some(denial) = access.take_denial_this_call(call_id) {
                    tool_result.exit_code = Some(1);
                    tool_result.policy_denial = Some(orz_loop::host::PolicyDenial {
                        source: orz_loop::host::PolicyDenialSource::Permission,
                        code: denial.code.to_string(),
                        reason: denial.reason,
                    });
                }
                if access.take_opened_this_call(call_id) {
                    tool_result.session_volume_opened = true;
                }
                access.finish_call(call_id);
            }
        }
        // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): 中间回报结构化
        // 透传——run_terminal_cmd 自动后台化返回 `BackgroundTaskStarted`
        // （命令仍在运行）：填 `mid_run` 供控制器记 `tool_running` 事件；
        // exit_code 保持 None（命令未结束，避免读作已成功退出）。
        if let Some(mid_run) = crate::tools::terminal_mid_run_from_output(name, &result.output) {
            tool_result.mid_run = Some(mid_run);
            tool_result.exit_code = None;
        }
        // TER T1.11 (W-F13b)：截断输出的结构化事实（output_truncated +
        // 持久化对象指针）——控制器据此在 tool_completed 落
        // output_truncated/total_bytes/output_object_id（schema T0.2）。
        if let Some(output_object) =
            crate::tools::terminal_output_object_from_output(name, &result.output)
        {
            tool_result.output_truncated = true;
            tool_result.output_object = Some(output_object);
        }
        if let Some(before) = delta_before {
            let (workspace_delta, workspace_delta_truncated) = workspace_delta_diff(
                &before,
                &workspace_delta_walk(&self.cwd),
                TOOL_DELTA_MAX_ENTRIES,
            );
            tool_result.workspace_delta = workspace_delta;
            tool_result.workspace_delta_truncated = workspace_delta_truncated;
        }
        // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
        // project_doc_index v2 写后失效——工作区写类工具完成即置 dirty，
        // 下一次索引 query 增量刷新（模型刚写的内容立即可搜，v1 空转
        // 场景闭环）。run_terminal_cmd 输出不可解析 → 整索引 dirty；
        // 后台任务（mid_run）也置位（命令仍在写盘）。
        if matches!(name, "search_replace" | "run_terminal_cmd") {
            self.project_doc_index.mark_dirty();
        }
        Ok(tool_result)
    }
}

/// TER 全面审查 P1-1 (2026-09-04)：把「live 快照 + 解析到的输出路径」映射
/// 为 loop 侧 idle-kill 事实（纯函数，drain 去重语义——`reported` 已含的
/// 任务跳过）。reason 按任务实际生效阈值渲染（S3：非默认阈值不失真）。
fn terminal_idle_kill_facts_from(
    resolved: Vec<(
        orz_tools::computer::types::TaskLiveSnapshot,
        Option<std::path::PathBuf>,
    )>,
    reported: &mut std::collections::HashSet<String>,
) -> Vec<orz_loop::host::TerminalIdleKillFact> {
    let mut facts = Vec::new();
    for (s, output_file) in resolved {
        if s.signal.as_deref() != Some(orz_tools::computer::local::terminal::IDLE_KILL_SIGNAL)
            || s.status != "killed"
        {
            continue;
        }
        if reported.contains(&s.task_id) {
            continue;
        }
        let Some(output_file) = output_file else {
            continue;
        };
        reported.insert(s.task_id.clone());
        let idle_timeout_ms = s.idle_timeout_ms.unwrap_or(300_000);
        facts.push(orz_loop::host::TerminalIdleKillFact {
            task_id: s.task_id,
            pid: s.pid,
            total_bytes: s.total_bytes,
            output_file: output_file.display().to_string(),
            wall_ms: s.elapsed_ms,
            reason: orz_tools::computer::local::terminal::idle_kill_reason(
                std::time::Duration::from_millis(idle_timeout_ms),
            ),
        });
    }
    facts
}

#[async_trait]
impl LoopHost for OrzHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }

    fn tools_registry(&self) -> &dyn ToolRegistry {
        &self.registry
    }

    fn workspace_trust(&self) -> WorkspaceTrust {
        self.workspace_trust
    }

    /// IP2a (FIX_PLAN 2026-08-06 D-3): map the bridge's session permission
    /// policy onto the loop's `ToolPolicy` — the loop filters policy-refused
    /// tools out of the model-visible declarations. A missing bridge (no
    /// policy known) maps to `Interactive` for the DECLARATION projection
    /// only; execution still fails closed through `request_permission`
    /// (LoopHost default → Deny), so nothing is auto-allowed by this.
    fn tool_policy(&self) -> orz_loop::host::ToolPolicy {
        use orz_loop::host::ToolPolicy;
        match self.permission.as_ref().map(|b| b.policy()) {
            Some(PermissionPolicy::ReadOnly) => ToolPolicy::ReadOnly,
            // FUS-BENCHMARK-FULL-EXEC (2026-08-18): a Benchmark policy with
            // the shell axis open maps to `BenchmarkFull` (read + write +
            // terminal projection); the default false/false axes keep the
            // original `Benchmark` semantics.
            Some(PermissionPolicy::Benchmark {
                allow_shell: true, ..
            }) => ToolPolicy::BenchmarkFull,
            Some(PermissionPolicy::Benchmark { .. }) => ToolPolicy::Benchmark,
            _ => ToolPolicy::Interactive,
        }
    }

    /// D-9 (FIX_PLAN 2026-08-06): the injected fixed test-runner command.
    fn test_runner(&self) -> Option<orz_loop::host::TestRunner> {
        self.test_runner.clone()
    }

    fn session_cwd(&self) -> std::path::PathBuf {
        self.cwd.clone()
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the mechanical delivery
    /// status baseline — the worktree metadata snapshot. `Some` = the real
    /// walk; the controller diffs it against a fresh snapshot at `submit`.
    fn workspace_snapshot(&self) -> Option<std::collections::HashMap<String, (u64, u64, u32)>> {
        Some(workspace_delta_walk(&self.cwd))
    }

    /// The ACP outbound gateway is the only live interactive-user channel
    /// (`ask_user_question` delivers through it); headless sessions
    /// (`gateway == None`, including the codex hub-only approval surface)
    /// fail closed.
    fn interactive_user(&self) -> bool {
        self.gateway.is_some()
    }

    /// FUS-TOOL-PROBE P0-A-2 (v0.2 single probe face): the ORZ host always
    /// wires a local terminal backend (`tools::build_toolset` constructs
    /// `LocalTerminalBackend`), so `run_terminal_cmd`'s mechanical chain is
    /// complete at the host level (policy still gates exec per session).
    fn terminal_available(&self) -> bool {
        true
    }

    /// TER 全面审查 F7 (2026-09-04)：orz 主线固定接线 LocalTerminalBackend，
    /// 支持 live 进程读取（空列表 = 当前无进程，与「不支持」区分）。
    fn terminal_live_capable(&self) -> bool {
        true
    }

    /// TER T1.6 (2026-09-04): 黑板 `section=processes` live 事实——把终端
    /// 读取时现算快照映射为 loop 侧结构化事实。
    async fn terminal_live_processes(&self) -> Vec<orz_loop::host::LiveProcessFact> {
        use orz_tools::computer::types::TaskLiveSnapshot;
        self.terminal
            .list_live_tasks()
            .await
            .into_iter()
            .map(|s: TaskLiveSnapshot| orz_loop::host::LiveProcessFact {
                task_id: s.task_id,
                command: s.command,
                display_command: s.display_command,
                pid: s.pid,
                elapsed_ms: s.elapsed_ms,
                status: s.status,
                total_bytes: s.total_bytes,
                cpu_micros: s.cpu_micros,
                killable: s.killable,
                owner_session_id: s.owner_session_id,
                description: s.description,
            })
            .collect()
    }

    /// TER 全面审查 P1-1 (2026-09-04)：idle-kill 生命周期事实源（drain
    /// 语义）——自上次调用以来新 idle-kill 的后台任务（signal=idle_killed
    /// 且 status=killed），去重后映射为 loop 侧事实。输出路径经
    /// `get_task` 解析（live 快照不带落盘路径，事件 schema 需要）。
    async fn drain_terminal_idle_kills(&self) -> Vec<orz_loop::host::TerminalIdleKillFact> {
        let live = self.terminal.list_live_tasks().await;
        let mut resolved = Vec::new();
        for s in live {
            if s.signal.as_deref() != Some(orz_tools::computer::local::terminal::IDLE_KILL_SIGNAL)
                || s.status != "killed"
            {
                continue;
            }
            let output_file = self
                .terminal
                .get_task(&s.task_id)
                .await
                .map(|t| t.output_file);
            resolved.push((s, output_file));
        }
        let mut reported = self
            .idle_kill_reported
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::terminal_idle_kill_facts_from(resolved, &mut reported)
    }

    /// 0z S2 §4.2（2026-09-12）：进程树扫除事实源——登记表的 drain 面
    /// （planned/executed 行；审计先行由扫除器保证）。
    /// 0z S2 §5（review F-EV-11 注释归位）：宿主资源事实源——
    /// `host_resource_snapshot` 暂存行的 drain 面（0aw 后该族只剩快照；
    /// `reclaim_performed`／`resource_exhausted`／`host_resource_denied`
    /// 生产端已随裁决 ④ 退役，schema/verifier 保留供历史 journal 校验）。
    async fn drain_host_resource_facts(&self) -> Vec<serde_json::Value> {
        self.drain_resource_facts()
    }

    /// 0ac S3①-b M2（2026-09-15，IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_
    /// RETRIEVAL_DESIGN §2.1/§4.1「合法边界投递」）：后台任务完成的投递
    /// 事实源——与 per-call `TaskCompletionReminder` 共用同一
    /// `ReportedTaskCompletions` 记账（`drain_between_turn_bash_completions`
    /// 内部 mark_reported），同一任务只经一个通道投给模型一次；owner
    /// 会话过滤同源（子代理不泄漏父/兄弟会话的任务）。文本由来源侧
    /// 单一源 `format_bash_completion` 格式化——loop 只注入不重写。
    async fn drain_completed_tasks(&self) -> Vec<orz_loop::host::CompletedTaskFact> {
        let bridge = orz_tools::bridge::ToolBridge::from_parts(
            self.registry.toolset().clone(),
            Some(self.terminal.clone()),
        );
        let tasks = bridge.drain_between_turn_bash_completions(&[]).await;
        if tasks.is_empty() {
            return Vec::new();
        }
        let task_output_name =
            orz_tools::reminders::task_completion::resolve_task_output_tool_name(&bridge).await;
        let read_tool_name =
            orz_tools::reminders::task_completion::resolve_read_tool_name(&bridge).await;
        tasks
            .into_iter()
            .map(|t| orz_loop::host::CompletedTaskFact {
                report: orz_tools::reminders::task_completion::format_bash_completion(
                    &t,
                    task_output_name.as_deref(),
                    read_tool_name.as_deref(),
                ),
                task_id: t.task_id,
                exit_code: t.exit_code,
            })
            .collect()
    }

    async fn finalize_process_trees(&self) {
        OrzHost::finalize_process_trees(self)
    }

    async fn drain_process_tree_reap_facts(&self) -> Vec<orz_loop::host::ProcessTreeReapFact> {
        let Some(registry) = self.process_trees.as_ref() else {
            return Vec::new();
        };
        registry
            .drain_facts()
            .into_iter()
            .map(|fact| orz_loop::host::ProcessTreeReapFact {
                reason: fact.reason.to_string(),
                phase: fact.phase.to_string(),
                pids: fact.pids,
                call_ids: fact.call_ids,
            })
            .collect()
    }

    /// TER T1.12 (W-F11)：黑板 `section=env` 的机械层环境快照事实。
    async fn env_snapshot_facts(&self) -> Vec<orz_loop::host::EnvSnapshotFact> {
        crate::env_snapshot::snapshot_env(&self.cwd).await
    }

    /// P2-3（2026-09-10）：会话浏览器 SERP 物理事实——loop 层据此为检索
    /// 车道保留底线额度。事实源是会话共享的浏览器句柄（跨 run 存活、
    /// 跨车道共享同一计数器）；无浏览器会话时返回 `None`（loop 不施加
    /// 底线规则，调用按普通失败回传）。
    async fn serp_session_facts(&self) -> Option<orz_loop::host::SerpSessionFacts> {
        let browser = self
            .browser
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let (navigations, ceiling) = browser.serp_session_navigations().await?;
        Some(orz_loop::host::SerpSessionFacts {
            navigations,
            ceiling,
        })
    }

    /// FUS-TOOL-PROBE P0-A-2: the ORZ host currently builds its toolset
    /// with every optional backend disabled (`build_toolset`:
    /// `memory_backend: None`, `lsp: None`, image/video configs
    /// `Default::default()` = Disabled, no MCP registration). The probes
    /// therefore fail closed — these tools are declared in the GrokBuild
    /// registry but cannot complete a call, so the single probe face
    /// removes them from the model-visible list. When a backend is wired
    /// in `tools::build_toolset`, flip the matching accessor here.
    fn lsp_configured(&self) -> bool {
        false
    }

    fn memory_enabled(&self) -> bool {
        false
    }

    fn image_backend_configured(&self) -> bool {
        false
    }

    fn video_backend_configured(&self) -> bool {
        false
    }

    fn mcp_registry_available(&self) -> bool {
        false
    }

    /// D-9: run the FIXED test command in the session cwd. The model never
    /// supplies argv — the command is host-owned; stdout/stderr/exit code
    /// feed back to the model (Aider-model loop).
    async fn run_tests(&self) -> Result<orz_loop::host::TestRunResult, ToolError> {
        let Some(runner) = self.test_runner.clone() else {
            return Err(ToolError::NotFound("no test runner configured".into()));
        };
        if runner.command.is_empty() {
            return Err(ToolError::ExecutionFailed("empty test command".into()));
        }
        let timeout = runner.timeout.unwrap_or(orz_loop::host::RUN_TESTS_TIMEOUT);
        // RT-003 (2026-08-11): workspace delta — snapshot the worktree
        // metadata before the run; after the run the diff (added/modified/
        // deleted, capped) is the audit trace of the test's file side
        // effects (ADR §3.8.2 requires workspace-delta/journal recording).
        let before = workspace_delta_walk(&self.cwd);
        // F-09 (2026-08-07 review): bounded execution + context gating. The
        // command runs under a wall-clock cap (default 30min — the cap only
        // catches true hangs; context size, not wall time, is the priority);
        // a hung child is killed. Output is collected with a TAIL cap (1MB)
        // so pathological output cannot blow session memory; the full
        // (capped) output is written to a file the model can read
        // (read_file), and the controller injects only the final 32KB into
        // the conversation.
        let mut cmd = tokio::process::Command::new(&runner.command[0]);
        cmd.args(&runner.command[1..])
            .current_dir(&self.cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        // RT-002 (2026-08-11): the test process must NOT inherit the host's
        // environment wholesale — it would see host secrets and paths
        // (ADV §3.8.2: secret/host-path leakage through test output). Clear
        // everything, then restore a fixed minimal platform allowlist plus
        // the harness's explicit `TestRunner::env` entries.
        cmd.env_clear();
        for var in TEST_ENV_ALLOWLIST {
            if let Ok(value) = std::env::var(var) {
                cmd.env(var, value);
            }
        }
        for (k, v) in &runner.env {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("test runner spawn: {e}")))?;
        // Stability fix (2026-08-07, design review D2 #3/#4) + FUS-HOST-
        // RESOURCE-SAFETY §4.7（复核 F-2）：bind the child into the **run's**
        // job hierarchy through the shared `ProcessGroup` — root job (run-wide
        // commit / concurrency / CPU ceilings) first, then the per-call job
        // (`KILL_ON_JOB_CLOSE`). This closes the original hang mechanism
        // end-to-end: when orz ITSELF is killed (the harness's timeout path),
        // the job handle closes with the process and the kernel terminates the
        // whole contained tree — the orphaned pytest that held our capture pipes
        // and blocked the harness forever (the 52-minute forth hang) cannot
        // survive orz. Assigning after spawn (running) is a millisecond window
        // vs CREATE_SUSPENDED, accepted and recorded; descendants spawned after
        // assignment inherit both jobs.
        //
        // Before 2026-09-12 this used a separate `JobObjectSupervisor` that
        // carried `KILL_ON_JOB_CLOSE` only, so the test runner was the one heavy
        // path outside the §4.7 ceilings. Non-Windows (or job creation failure):
        // `None` falls back to the TaskKill tree-kill path below.
        let mut test_group = xai_tty_utils::ProcessGroup::new().ok();
        if let Some(group) = test_group.as_mut()
            && let Err(e) = group.attach(&child)
        {
            tracing::warn!(
                error = %e,
                "run_tests: child NOT associated with the run job — run-wide ceilings do \
                 not cover this test run; falling back to TaskKill on timeout"
            );
        }
        let stdout = child.stdout.take().ok_or_else(|| {
            ToolError::ExecutionFailed("test runner stdout pipe unreadable".into())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ToolError::ExecutionFailed("test runner stderr pipe unreadable".into())
        })?;
        let mut out_buf = Vec::<u8>::new();
        let mut err_buf = Vec::<u8>::new();
        let collect = async {
            tokio::join!(
                read_capped(stdout, &mut out_buf),
                read_capped(stderr, &mut err_buf),
            );
            child.wait().await
        };
        let status = match tokio::time::timeout(timeout, collect).await {
            Ok(status) => {
                status.map_err(|e| ToolError::ExecutionFailed(format!("test runner wait: {e}")))?
            }
            Err(_) => {
                // Stability fix (2026-08-07): `Child::kill` terminates only
                // the direct child — grandchildren that inherited our capture
                // pipes (e.g. a pytest spawned by the runner) survive as
                // orphans and keep `read_capped` blocked on EOF forever (the
                // 52-minute forth hang). Kill the whole tree on Windows via
                // TaskKill; unix keeps the plain kill (recorded limitation).
                // The process-group kill runs first when the child is in the
                // run hierarchy (terminates the whole job tree).
                if let Some(group) = test_group.as_ref() {
                    let _ = group.kill();
                }
                kill_process_tree(&mut child).await;
                // RT-003: the timed-out run may still have written files —
                // record what it changed before returning.
                let (workspace_delta, workspace_delta_truncated) = workspace_delta_diff(
                    &before,
                    &workspace_delta_walk(&self.cwd),
                    RUN_TESTS_DELTA_MAX_ENTRIES,
                );
                let (partial_output, output_encoding) =
                    orz_tools::util::encoding::decode_text(&out_buf);
                return Ok(orz_loop::host::TestRunResult {
                    output: format!(
                        "[test runner TIMED OUT after {timeout:?} — process tree killed; \
                         partial output follows]\n{}",
                        partial_output,
                    ),
                    exit_code: None,
                    // THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1 审查
                    // 处理 P2-1): 结构化超时标记（F-09 墙钟掐杀）——控制器
                    // 据此落 tool_completed.timed_out 与明确模型文案，不靠
                    // 文本前缀判定。
                    timed_out: true,
                    full_output_path: None,
                    output_encoding: Some(output_encoding),
                    workspace_delta,
                    workspace_delta_truncated,
                });
            }
        };
        // GAP-ENCODING-GATE (OPS-PROTOCOL §8): stdout/stderr each decode
        // through the fixed chain; both labels are recorded (comma-joined).
        let (mut text, stdout_encoding) = orz_tools::util::encoding::decode_text(&out_buf);
        let mut encodings = vec![stdout_encoding];
        if !err_buf.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            let (err_text, err_encoding) = orz_tools::util::encoding::decode_text(&err_buf);
            encodings.push(err_encoding);
            text.push_str(&err_text);
        }
        let output_encoding =
            orz_tools::util::encoding::merge_encoding_labels(encodings.iter().map(String::as_str));
        // 0p S2 复审 P1-2 修复（B5 第 5 漏斗，2026-09-07，ADR-0010 §14.61）：
        // run_tests 全量输出落 `.gsa/run_tests_output.txt`（恒直读窗口，
        // B1 直读类）——env_clear+allowlist 只隔离宿主 env，测试进程仍可能
        // 打印工作区自带密钥。落盘与对话尾窗统一在此接 orz-secrets 机械
        // 脱敏（key 不落卷是两段门放开的先决不变量；「全卷零 sk-」判据）。
        let text = orz_secrets::redact_secrets(&text).into_owned();
        // F-09: write the full (capped) output to a file the model can read
        // (read_file) — the conversation only carries the final 32KB.
        let gsa_dir = self.cwd.join(".gsa");
        let full_output_path = match std::fs::create_dir_all(&gsa_dir)
            .and_then(|()| std::fs::write(gsa_dir.join("run_tests_output.txt"), &text))
        {
            Ok(()) => Some(
                gsa_dir
                    .join("run_tests_output.txt")
                    .to_string_lossy()
                    .into_owned(),
            ),
            Err(e) => {
                tracing::warn!("run_tests: full output file write failed: {e}");
                None
            }
        };
        // RT-003: workspace delta — diff the post-run walk against the
        // pre-run baseline (`.gsa` is excluded, so the output file written
        // above does not pollute the trace).
        let (workspace_delta, workspace_delta_truncated) = workspace_delta_diff(
            &before,
            &workspace_delta_walk(&self.cwd),
            RUN_TESTS_DELTA_MAX_ENTRIES,
        );
        Ok(orz_loop::host::TestRunResult {
            output: text,
            exit_code: status.code(),
            timed_out: false,
            full_output_path,
            output_encoding,
            workspace_delta,
            workspace_delta_truncated,
        })
    }

    /// Forward a streamed model text chunk to the live ACP client as an
    /// `agent_message_chunk` notification — the TUI renders it incrementally
    /// into the current model card (streaming slice). Deltas are live-only,
    /// never journaled (Python `text_delta` precedent); headless hosts
    /// (no gateway) drop the chunk silently.
    fn on_text_delta(&self, text: &str) {
        let (Some(gateway), Some(session_id)) = (&self.gateway, &self.session_id) else {
            return;
        };
        let notification = acp::SessionNotification::new(
            acp::SessionId::new(session_id.clone()),
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new(text.to_string()),
            ))),
        );
        // Fire-and-forget: the `acp::Agent::session_notification` trait method
        // is `#[async_trait(?Send)]` (its future is not Send), so it cannot be
        // awaited inside this Send-bound LoopHost impl — `forward_fire_and_forget`
        // is the documented bypass. A dropped receiver discards the chunk.
        if !gateway.forward_fire_and_forget(notification) {
            tracing::debug!("on_text_delta: gateway receiver dropped, chunk discarded");
        }
    }

    async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
        call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        self.call_tool_inner(name, args, call_id, None).await
    }

    /// P0-C S4 (2026-08-16): script step deadlines — a per-call override
    /// bounded by the configured host budget (`min`). The host still owns
    /// process-tree reclamation on expiry (0bl 审查修复 2026-09-24：per-call
    /// job 句柄定向 `TerminateJobObject`，句柄不可得时回退全局
    /// `kill_active`——见 `call_tool_inner` 超时臂），exactly like the
    /// default path.
    async fn call_tool_with_timeout(
        &self,
        name: &str,
        args: serde_json::Value,
        call_id: &str,
        timeout: Option<std::time::Duration>,
    ) -> Result<ToolResult, ToolError> {
        self.call_tool_inner(name, args, call_id, timeout).await
    }

    async fn request_permission(
        &self,
        risk: RiskClass,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<PermitDecision, PermitError> {
        self.request_permission_with_source(risk, tool, args)
            .await
            .map(|(decision, _source)| decision)
    }

    /// 0bt④（2026-09-26）：桥自报判定来源（封闭集，观测面）——判定语义与
    /// 拒绝序不变；无桥 fail-closed 亦带来源。
    async fn request_permission_with_source(
        &self,
        risk: RiskClass,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<(PermitDecision, Option<PermitSource>), PermitError> {
        match &self.permission {
            Some(bridge) => bridge
                .request_with_source(risk, tool, args)
                .await
                .map(|(decision, source)| (decision, Some(source))),
            // No bridge wired → fail closed, never auto-allow (review P1-1).
            None => Ok((PermitDecision::Deny, Some(PermitSource::FailClosed))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_assurance::journal::{EventType, RunEvent};
    use orz_loop::AgentLoopController;
    use orz_loop::gateway::fake::FakeProvider;
    use orz_loop::gateway::model::{ModelGateway, ToolCall};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// P2-1 (review 2026-08-10): every test that mutates the web_search env
    /// vars takes this lock — `web_search_config_follows_credential_source`
    /// (tools.rs) and the semaphore tests here share one test binary and
    /// would otherwise race on the `ORZ_WEB_SEARCH_BASE_URL`/`_MODEL`
    /// overrides (a cross-test flake). (2026-08-11: `ORZ_WEB_SEARCH_API_KEY`
    /// was removed with the direction correction — the key now comes from
    /// the credential reader; the lock guards the remaining non-secret env
    /// overrides.)
    /// tokio mutex: the async tests hold it across awaits (clippy
    /// await_holding_lock would flag a std guard); OnceLock because tokio's
    /// `Mutex::new` is not const.
    pub(crate) static TESTS_ENV_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
        std::sync::OnceLock::new();

    /// P2-1: accessor for the shared env-test lock.
    pub(crate) fn tests_env_lock() -> &'static tokio::sync::Mutex<()> {
        TESTS_ENV_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    /// P2-1 (review 2026-08-10): restores an env var on drop — tests that
    /// mutate shared env state must not leak into later tests.
    pub(crate) struct EnvVarGuard {
        key: &'static str,
        old: Option<String>,
    }

    impl EnvVarGuard {
        pub(crate) fn new(key: &'static str) -> Self {
            let old = std::env::var(key).ok();
            Self { key, old }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.old {
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-host-lib-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 2026-08-11: a credential reader that always fails — fixes
    /// `web_search` to Disabled regardless of the platform (semaphore and
    /// routing tests must fast-fail even on a machine whose DeepSeek
    /// credential is registered; without this seam they would attempt real
    /// API calls).
    fn failing_reader() -> Arc<dyn crate::credentials::CredentialReader> {
        struct Fail;
        impl crate::credentials::CredentialReader for Fail {
            fn read(&self) -> Result<String, crate::credentials::CredentialError> {
                Err(crate::credentials::CredentialError {
                    message: "test: no credential registered".into(),
                })
            }
        }
        Arc::new(Fail)
    }

    /// Host with the failing reader injected (see [`failing_reader`]).
    fn host_with_failing_reader(journal: JournalRecorder, dir: &std::path::Path) -> OrzHost {
        OrzHost::with_credential_reader(
            journal,
            dir,
            WorkspaceTrust::ObservedTrusted,
            None,
            failing_reader(),
        )
        .expect("host build")
    }

    /// Toolset construction is expensive (builder finalize) — share one.
    static SHARED_TOOLSET: OnceLock<Arc<orz_tools::registry::types::FinalizedToolset>> =
        OnceLock::new();

    fn shared_toolset() -> &'static Arc<orz_tools::registry::types::FinalizedToolset> {
        SHARED_TOOLSET.get_or_init(|| {
            tools::build_toolset(
                &std::env::temp_dir(),
                &WebSearchConfig::Disabled,
                None,
                Arc::new(LocalTerminalBackend::new()),
            )
            .expect("shared toolset")
        })
    }

    #[tokio::test]
    async fn registry_lists_builtin_tools() {
        let toolset = shared_toolset();
        let names: Vec<String> = toolset
            .tool_definitions()
            .iter()
            .map(|d| d.function.name.clone())
            .collect();
        assert!(
            names.iter().any(|n| n == "read_file"),
            "expected read_file in builtin tools, got {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "run_terminal_cmd"),
            "expected run_terminal_cmd (GrokBuild bash), got {names:?}"
        );
    }
    /// GAP-ENCODING-GATE e2e: `read_file` through the host decodes a
    /// GB18030 file with the fixed chain and forwards the observed stage to
    /// the loop's `ToolResult.output_encoding` (journaled upstream as
    /// `tool_completed.output_encoding`).
    #[tokio::test]
    async fn read_file_gb18030_forwards_output_encoding() {
        let dir = test_dir();
        let mut bytes = vec![0xd6, 0xd0, 0xce, 0xc4];
        bytes.push(b'\n');
        std::fs::write(dir.join("gb.txt"), &bytes).unwrap();
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let result = host
            .call_tool(
                "read_file",
                serde_json::json!({"target_file": "gb.txt"}),
                "call-gb",
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains('中'), "output: {}", result.output);
        assert_eq!(result.output_encoding.as_deref(), Some("gb18030"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// local_browser (2026-08-10) + 0t (2026-09-09, S2-R P3 / P2-3): the
    /// registry declares `browser_read` by the 0t enable-gate declaration
    /// bit（静态双族），NOT by lane readiness——未置位 fail-closed 默认不
    /// 声明；置位后恒声明（浏览器缺席由调用期懒启动/普通失败回传兜底）。
    #[tokio::test]
    async fn browser_read_declaration_follows_enable_gate_bit() {
        let mut registry = ToolsetRegistry::new(shared_toolset().clone());
        // Default: 启用门未置位 → 不声明（fail-closed）。
        assert!(registry.get("browser_read").is_none());
        assert!(registry.get("browser_control").is_none());
        assert!(!registry.list().iter().any(|d| d.name == "browser_read"));
        assert!(!registry.list().iter().any(|d| d.name == "browser_control"));
        // 置位 → 恒声明。
        registry.set_browser_declared(true);
        assert!(registry.get("browser_read").is_some());
        assert!(registry.get("browser_control").is_some());
        assert!(registry.list().iter().any(|d| d.name == "browser_read"));
        assert!(registry.list().iter().any(|d| d.name == "browser_control"));
        // 复位 → 移除。
        registry.set_browser_declared(false);
        assert!(registry.get("browser_read").is_none());
        assert!(registry.get("browser_control").is_none());
    }

    /// 0t (2026-09-09, S2-R P3 / P2-3)：句柄注入不翻转声明位——未启用
    /// 会话注入 ready 句柄仍不声明 browser_read（fail-closed 面由启用门
    /// 决定）；启用会话的声明由 `declare_browser_declared` 置位、与注入
    /// 次序无关。
    #[tokio::test]
    async fn browser_session_injection_does_not_flip_declaration() {
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let stub = crate::local_browser::tests::ready_stub_browser();
        let mut host = host.with_browser_session(stub);
        // 注入 ready 句柄本身不得翻转声明（未置位仍不声明）。
        assert!(
            host.registry.get("browser_read").is_none(),
            "handle injection must not flip the declaration bit"
        );
        // 启用门置位后经 registry 声明。
        host.declare_browser_declared();
        assert!(
            host.registry.get("browser_read").is_some(),
            "enable-gate declaration must expose browser_read via the registry"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// local_browser (2026-08-10) + 0t (2026-09-09, ADR-0010 §14.65 /
    /// 设计 §3.3/§3.5): `call_tool("browser_read")` routes to the injected
    /// browser session; 无句柄时调用期懒启动——启动失败按普通 host 错误
    /// 回传（`BrowserLaunchFailed`，真实原因），成功则完成读取。
    #[tokio::test]
    async fn call_browser_read_routes_to_session_and_fails_closed() {
        let dir = test_dir();

        // 无句柄 + 浏览器发现失败（ORZ_BROWSER_PATH 指向不存在文件）→
        // BrowserLaunchFailed 携带真实原因（never a silent stub success）。
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let missing = dir.join("no-such-browser.exe");
        // SAFETY: test-only env mutation; the real-browser tests read it
        // through find_browser which tolerates concurrent set/remove.
        unsafe { std::env::set_var(crate::local_browser::ORZ_BROWSER_PATH_ENV, &missing) };
        let err = host
            .call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c1",
            )
            .await
            .unwrap_err();
        unsafe { std::env::remove_var(crate::local_browser::ORZ_BROWSER_PATH_ENV) };
        assert!(
            err.to_string().contains("browser launch failed")
                && err.to_string().contains("browser_not_found"),
            "{err}"
        );

        // Ready session → success path with the real wrapper.
        let stub = crate::local_browser::tests::ready_stub_browser();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_browser_session(stub);
        assert!(host.browser_ready());
        let result = host
            .call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c2",
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains("hello page"), "{}", result.output);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// S2-R P3 / P2-1（2026-09-09）：懒启动 check+launch 串行锁烟雾——
    /// 并发首调 browser_read 不得死锁、不得误报；浏览器不可用时双双按
    /// `BrowserLaunchFailed` 普通失败（真实原因），Ready 场景并发成功。
    /// 「同一 profile 只拉起一次」的可观测断言由
    /// `concurrent_first_launch_invokes_probe_once`（launch seam 计数）
    /// 覆盖（X3 补强，2026-09-09 复审处理）。
    #[tokio::test]
    async fn concurrent_browser_read_lazy_launch_does_not_deadlock() {
        let _env_lock = crate::tests::tests_env_lock().lock().await;
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let missing = dir.join("no-such-browser.exe");
        // SAFETY: test-only env mutation (find_browser tolerates concurrent
        // set/remove; the env lock above keeps cross-test races out).
        unsafe { std::env::set_var(crate::local_browser::ORZ_BROWSER_PATH_ENV, &missing) };
        let (a, b) = tokio::join!(
            host.call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c-race-a",
            ),
            host.call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c-race-b",
            ),
        );
        unsafe { std::env::remove_var(crate::local_browser::ORZ_BROWSER_PATH_ENV) };
        for err in [a, b] {
            let err = err.expect_err("both calls must fail explicitly");
            assert!(err.to_string().contains("browser launch failed"), "{err}");
        }

        // Ready 场景：注入 stub 后并发调用都成功（锁不串行页面动作）。
        let stub = crate::local_browser::tests::ready_stub_browser();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_browser_session(stub);
        let (a, b) = tokio::join!(
            host.call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c-race-ok-a",
            ),
            host.call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c-race-ok-b",
            ),
        );
        assert_eq!(a.expect("ok a").exit_code, Some(0));
        assert_eq!(b.expect("ok b").exit_code, Some(0));

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── FUS-HOST-RESOURCE-SAFETY §4.1（0z S1）汇点接线 ─────────────────

    /// Serializes the tests that install/replace the process-wide run job (the
    /// slot is process-wide by design).
    static RESOURCE_SAFETY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn resource_safety_lock() -> std::sync::MutexGuard<'static, ()> {
        RESOURCE_SAFETY_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// 0aw 判据 2 的汇点面：低余量读数下派发**照常执行**，结果头部带一条
    /// `[资源软提示]`（中文短句＋英文读数）；同一卷第二次派发不再重复提示
    /// （每 run 每卷一次的机械去重）。没有任何拒绝臂。
    #[tokio::test]
    async fn volume_soft_hint_attaches_at_the_call_tool_seam() {
        use crate::resource_hint::{
            CapacityProbe, GIB, HostCapacitySnapshot, ResourceHint, SourceQuality,
        };

        struct Fixed(HostCapacitySnapshot);
        impl CapacityProbe for Fixed {
            fn probe(&self, _path: &std::path::Path) -> HostCapacitySnapshot {
                self.0
            }
        }
        let low = HostCapacitySnapshot {
            collected_at_ms: 7,
            volume_free_bytes: GIB,
            volume_total_bytes: 100 * GIB,
            commit_limit_bytes: 32 * GIB,
            commit_used_bytes: 30 * GIB,
            source_quality: SourceQuality::Available,
        };
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_resource_hint(ResourceHint::new(Arc::new(Fixed(low))));

        // First dispatch: the action RUNS (exit 0 on its own merits), and the
        // result head carries exactly one soft-hint line with readings.
        let first = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo hint-once",
                    "description": "proxy: hint attaches, dispatch proceeds"
                }),
                "c-hint-1",
            )
            .await
            .expect("hint is not a refusal");
        assert_eq!(first.exit_code, Some(0));
        let line_count = first.output.matches("[资源软提示]").count();
        assert_eq!(line_count, 1, "exactly one hint line: {}", first.output);
        assert!(first.output.contains("动作照常执行"), "{}", first.output);
        assert!(
            first
                .output
                .contains("free 1.00 GiB of 100.00 GiB < 4.00 GiB"),
            "mechanical readings ride the line: {}",
            first.output
        );
        assert!(
            first.output.contains("hint-once"),
            "the tool output itself is still there: {}",
            first.output
        );

        // Second dispatch on the same volume: deduped — no second hint.
        let second = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo hint-never",
                    "description": "proxy: per-run per-volume dedup"
                }),
                "c-hint-2",
            )
            .await
            .expect("second call");
        assert_eq!(second.exit_code, Some(0));
        assert!(
            !second.output.contains("[资源软提示]"),
            "no repeated hint: {}",
            second.output
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 观测面纪律钉子（沿 0af F-BE-12 形态）：资源读数面对每次工具调用
    /// **只读一次**——单写入目标 ⇒ 恰好一次探针（软提示与档位观测共用同
    /// 一次读数，不存在第二次探针分歧面）。
    #[tokio::test]
    async fn resource_hint_reads_once_per_call_tool() {
        use crate::resource_hint::{
            CapacityProbe, GIB, HostCapacitySnapshot, ResourceHint, SourceQuality,
        };
        use std::sync::Mutex;

        struct Counting(Arc<Mutex<u32>>);
        impl CapacityProbe for Counting {
            fn probe(&self, _path: &std::path::Path) -> HostCapacitySnapshot {
                *self
                    .0
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
                HostCapacitySnapshot {
                    collected_at_ms: 7,
                    volume_free_bytes: GIB,
                    volume_total_bytes: 100 * GIB,
                    commit_limit_bytes: 32 * GIB,
                    commit_used_bytes: 30 * GIB,
                    source_quality: SourceQuality::Available,
                }
            }
        }
        let reads = Arc::new(Mutex::new(0u32));
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_resource_hint(ResourceHint::new(Arc::new(Counting(Arc::clone(&reads)))));

        let done = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo read-once",
                    "description": "proxy: one call, one probe read"
                }),
                "c-once",
            )
            .await
            .expect("call completes");
        assert_eq!(done.exit_code, Some(0));
        assert_eq!(
            *reads
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            1,
            "one call ⇒ exactly one dispatch reading (one probe read)"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0aw 边界钉子：读数不可得 ⇒ **无提示、照常派发**（观测标签落
    /// `unknown`；准入时代的 fail-closed 拒绝语义已退役——不再有「因读数
    /// 缺席而拒绝」的面）。
    #[tokio::test]
    async fn unavailable_readings_hint_nothing_and_never_block() {
        use crate::resource_hint::{CapacityProbe, HostCapacitySnapshot, ResourceHint};

        struct Blind;
        impl CapacityProbe for Blind {
            fn probe(&self, _path: &std::path::Path) -> HostCapacitySnapshot {
                HostCapacitySnapshot::unavailable()
            }
        }
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_resource_hint(ResourceHint::new(Arc::new(Blind)));

        let done = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo blind-still-runs",
                    "description": "proxy: unavailable readings never block"
                }),
                "c-blind",
            )
            .await
            .expect("call completes");
        assert_eq!(done.exit_code, Some(0));
        assert!(
            !done.output.contains("[资源软提示]"),
            "no hint without readings: {}",
            done.output
        );
        assert!(
            done.structured.is_none(),
            "no refusal envelope exists any more"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F 的装配面（真机）：`with_host_resource_safety` 用真实探针装上限——本机
    /// 读数可得、且内核读回至少一条限项（判据 12 的 S1 半段）。
    #[tokio::test]
    async fn host_resource_safety_installs_real_probe_and_ceilings() {
        let _guard = resource_safety_lock();
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_host_resource_safety();
        let hint = host.resource_hint().expect("hint face wired at assembly");
        let snapshot = hint.last_snapshot();
        // The face probes lazily; read once through the public seam.
        let readings = hint.read_for_volumes(&[dir.clone()]);
        assert!(!readings.is_empty(), "the target volume is read");
        let job = xai_tty_utils::global_run_job().expect("run ceilings installed");
        assert!(
            job.is_kernel_enforced(),
            "at least one ceiling must be kernel-visible on Windows: {}",
            job.describe()
        );
        // `snapshot` is what the *assembly* probe read (may predate the call
        // above); both paths must agree that the reading source is real.
        assert!(
            snapshot.is_none()
                || snapshot.unwrap().source_quality
                    == crate::resource_hint::SourceQuality::Available
        );
        // Review F-1: the ceilings live on the **run** job (aggregate), and the
        // readback the assembly face publishes is that same job.
        assert!(
            job.readback().is_enforced(),
            "the run job must be the enforcement point: {}",
            job.describe()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 判据 12 的端到端面（独立复核 F-1/F-9）：一次**轻档**工具调用（不受预检门
    /// 限制）经终端 spawn 拉起子进程，子进程的超额提交必须被 **run 级** job 拒绝
    /// —— 证明这条生产 spawn 链真的把工具进程树挂进了带限 job，而不是只有单测里
    /// 手工 attach 的那条路径。
    #[cfg(windows)]
    #[tokio::test]
    async fn tool_spawn_is_really_bounded_by_the_run_job() {
        let _guard = resource_safety_lock();
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        // A ceiling small enough to observe: the per-call job carries none, so a
        // refusal can only come from the run job.
        .with_host_resource_safety_limits(xai_tty_utils::JobLimits {
            commit_limit_bytes: Some(300 * 1024 * 1024),
            commit_notification_bytes: None,
            active_process: None,
            cpu_rate_percent: None,
        });
        let result = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    // `python -c` is a conditional program without a build word,
                    // so the call classifies Light and the gate never refuses it
                    // — exactly the shape that has to be bounded by the kernel.
                    // (Single quotes on purpose: the tool runs this through
                    // PowerShell, which would expand a `$name` before the inner
                    // interpreter ever sees it.)
                    "command": "python -c 'b = bytearray(1073741824)'",
                    "description": "proxy: end-to-end job ceiling check"
                }),
                "c-e2e-job",
            )
            .await
            .expect("light call returns a tool result");
        assert!(
            result.output.contains("MemoryError"),
            "the run job must bound the tool's child; output was: {}",
            result.output
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// S2-R P3 / P2-1 复审处理 X3（2026-09-09）：并发首调成功启动只发生
    /// 一次——launch seam 计数 + 双重检查语义：先完成者换入 ready 句柄，
    /// 等待者二次检查见 ready 直接跳过（返回 false）；随后 ready 会话的
    /// 并发调用零启动。
    #[tokio::test]
    async fn concurrent_first_launch_invokes_probe_once() {
        let _env_lock = crate::tests::tests_env_lock().lock().await;
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let launches = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let stub = crate::local_browser::tests::ready_stub_browser();
        let (a, b) = tokio::join!(
            host.ensure_browser_launched_with(|_cwd, _session| {
                let launches = launches.clone();
                let stub = stub.clone();
                async move {
                    launches.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<_, String>(stub)
                }
            }),
            host.ensure_browser_launched_with(|_cwd, _session| {
                let launches = launches.clone();
                let stub = stub.clone();
                async move {
                    launches.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<_, String>(stub)
                }
            }),
        );
        assert!(
            a.expect("first caller must launch"),
            "first caller must attempt the launch"
        );
        assert!(
            !b.expect("second caller must not fail"),
            "second caller must see the ready handle and skip the launch"
        );
        assert_eq!(
            launches.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the same profile must only be launched once under concurrent first calls"
        );

        // Ready 会话并发调用：零启动（既有调用直接走 ready 分支，S4 无
        // launch fact 的 host 侧同源语义）。
        let (c, d) = tokio::join!(
            host.ensure_browser_launched_with(|_cwd, _session| {
                let launches = launches.clone();
                let stub = stub.clone();
                async move {
                    launches.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<_, String>(stub)
                }
            }),
            host.ensure_browser_launched_with(|_cwd, _session| {
                let launches = launches.clone();
                let stub = stub.clone();
                async move {
                    launches.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<_, String>(stub)
                }
            }),
        );
        assert!(!c.expect("ready call c"), "ready calls must not launch");
        assert!(!d.expect("ready call d"), "ready calls must not launch");
        assert_eq!(
            launches.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "ready-lane concurrent calls must not launch the browser"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// S2-R P3 / P1-2b：`browser_control` 经 host `call_tool` 路由到会话层
    /// 控制 stub——ready 场景正常返回动作信封（懒启动跳过，无 launch
    /// fact）；未注入句柄 + 浏览器发现失败按 `BrowserLaunchFailed` 普通
    /// 失败回传（与 browser_read 同懒启动路径）。
    #[tokio::test]
    async fn call_browser_control_routes_to_session_and_fails_closed() {
        let _env_lock = crate::tests::tests_env_lock().lock().await;
        let dir = test_dir();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let missing = dir.join("no-such-browser.exe");
        // SAFETY: test-only env mutation (find_browser tolerates concurrent
        // set/remove; the env lock above keeps cross-test races out).
        unsafe { std::env::set_var(crate::local_browser::ORZ_BROWSER_PATH_ENV, &missing) };
        let err = host
            .call_tool(
                "browser_control",
                serde_json::json!({"action": "navigate", "url": "https://example.com"}),
                "call-ctrl-1",
            )
            .await
            .unwrap_err();
        unsafe { std::env::remove_var(crate::local_browser::ORZ_BROWSER_PATH_ENV) };
        assert!(err.to_string().contains("browser launch failed"), "{err}");

        // Ready stub → 正常信封回传（stub 默认 idle outcome）。
        let stub = crate::local_browser::tests::ready_stub_browser();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_browser_session(stub);
        let result = host
            .call_tool(
                "browser_control",
                serde_json::json!({"action": "snapshot"}),
                "call-ctrl-2",
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(
            result.browser_launch_fact.is_none(),
            "ready lane must not emit a launch fact (S4)"
        );
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert_eq!(parsed["action"], "snapshot");
        assert_eq!(parsed["action_status"], "ok");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0t P1-2a（2026-09-09, S2-R P2 / 设计 §3.2）：浏览器调用收尾装配——
    /// 启动尝试事实独立于动作结果。确定性覆盖三臂：
    /// S2 启动成功 + 动作成功 → Ok 附 success fact；
    /// S3 启动成功 + 动作失败 → `BrowserStepFailed` 携带 success fact（真实
    /// 原因进 reason，不吞错）；
    /// S4 未启动 + 普通失败 → 原样返回（不包装）。
    #[test]
    fn finish_browser_call_attaches_launch_fact_on_success_or_step_failure() {
        use orz_loop::host::BrowserLaunchFact;

        // S2: launch attempted + Ok → success fact attached。
        let res = finish_browser_call(
            true,
            Ok(ToolResult {
                output: "page text".to_string(),
                exit_code: Some(0),
                ..Default::default()
            }),
        )
        .expect("S2 ok");
        assert_eq!(res.browser_launch_fact, Some(BrowserLaunchFact::success()));

        // S3: launch attempted + Err → BrowserStepFailed 携带 success fact，
        // reason 保留页面层真实错误文本。
        let err = finish_browser_call(
            true,
            Err(ToolError::ExecutionFailed(
                "browser_read failed [browser_read_empty_content]: page produced \
                 no readable text"
                    .to_string(),
            )),
        )
        .expect_err("S3 err");
        match &err {
            ToolError::BrowserStepFailed {
                reason,
                launch_fact,
            } => {
                assert!(
                    reason.contains("browser_read_empty_content"),
                    "real page-level cause preserved: {reason}"
                );
                assert_eq!(*launch_fact, BrowserLaunchFact::success());
            }
            other => panic!("expected BrowserStepFailed, got {other:?}"),
        }

        // S4: no launch attempted + Err → unchanged（不包装）。
        let err = finish_browser_call(
            false,
            Err(ToolError::ExecutionFailed(
                "browser_read failed [browser_read_missing_url]: ...".to_string(),
            )),
        )
        .expect_err("S4 err");
        assert!(
            matches!(err, ToolError::ExecutionFailed(_)),
            "unlaunched failures must not be wrapped: {err:?}"
        );

        // S4: no launch attempted + Ok → unchanged, no fact。
        let res = finish_browser_call(
            false,
            Ok(ToolResult {
                output: "page text".to_string(),
                exit_code: Some(0),
                ..Default::default()
            }),
        )
        .expect("S4 ok");
        assert!(res.browser_launch_fact.is_none());
    }

    /// PDF evidence (2026-08-11): `call_tool("web_fetch", whitelisted url)`
    /// is intercepted at the chokepoint and routed through the browser lane;
    /// without a ready lane it fails explicitly with the whitelist failure
    /// code (user ruling — no automatic fallback to direct fetch).
    #[tokio::test]
    async fn call_web_fetch_whitelist_routes_through_browser() {
        let _env_lock = crate::tests::tests_env_lock().lock().await;
        let dir = test_dir();
        crate::pdf_evidence::set_domains_override(Some("*.cnki.net".to_string()));

        // No browser handle → explicit whitelist failure, no fallback.
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let err = host
            .call_tool(
                "web_fetch",
                serde_json::json!({"url": "https://kns.cnki.net/paper.pdf"}),
                "p1",
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("web_fetch_pdf_browser_unavailable"),
            "{err}"
        );

        // Ready lane → intercepted: the stub's download path runs (returns
        // a Page outcome — shaped like browser_read, no document_id).
        let stub = crate::local_browser::tests::ready_stub_browser();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_browser_session(stub);
        let result = host
            .call_tool(
                "web_fetch",
                serde_json::json!({"url": "https://kns.cnki.net/kcms/detail"}),
                "p2",
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains("hello page"), "{}", result.output);
        assert!(!result.output.contains("document_id="), "{}", result.output);

        crate::pdf_evidence::set_domains_override(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): a `web_search` call waits on
    /// the global permit (concurrency=1, ADR-0010 §3.7.7/§11.3). The test
    /// holds the permit, spawns the call, asserts it is still pending while
    /// held, then releases — the call completes (fast-fails: the tool is
    /// unregistered without an env key, but the gate ordering is what is
    /// under test).
    #[tokio::test]
    async fn web_search_call_waits_for_global_permit() {
        let dir = test_dir();
        // ADR-0006 (2026-08-11): the injected failing reader fixes
        // web_search to Disabled on every platform (was: env removal — a
        // machine with a registered `orz-grok/search` credential would
        // otherwise enable the client and hit the real API).
        let host = Arc::new(host_with_failing_reader(
            JournalRecorder::new(dir.clone()),
            &dir,
        ));

        // Hold the single permit.
        let permit = host
            .web_search_semaphore
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let host2 = host.clone();
        let handle = tokio::spawn(async move {
            host2
                .call_tool(
                    "web_search",
                    serde_json::json!({"query": "test"}),
                    "ws-gate-1",
                )
                .await
        });

        // While the permit is held the call must still be pending.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(
            !handle.is_finished(),
            "web_search call must wait on the global permit"
        );

        // Release → the call proceeds (and fast-fails: no key = unregistered).
        drop(permit);
        let result = handle.await.unwrap();
        assert!(
            result.is_err(),
            "unregistered web_search must fail: {result:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10, review F7): a P0-1 timeout on a
    /// WAITING call drops the future → the permit releases with it. A holder
    /// can never leak the gate: after the timeout the next call acquires
    /// immediately (fast-fails again — unregistered without an env key).
    #[tokio::test]
    async fn web_search_timeout_releases_the_permit() {
        let dir = test_dir();
        // ADR-0006 (2026-08-11): injected failing reader (see the waiting
        // test above) — platform-independent Disabled, no real API risk.
        let host = Arc::new(
            host_with_failing_reader(JournalRecorder::new(dir.clone()), &dir)
                .with_tool_timeout(std::time::Duration::from_millis(100)),
        );

        // Hold the single permit so the call can only wait.
        let permit = host
            .web_search_semaphore
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let host2 = host.clone();
        let handle = tokio::spawn(async move {
            host2
                .call_tool("web_search", serde_json::json!({"query": "x"}), "ws-tout-1")
                .await
        });
        let err = handle.await.unwrap().unwrap_err();
        assert!(
            err.to_string().contains("TIMED OUT"),
            "waiting call must hit the P0-1 budget: {err}"
        );

        // The timeout dropped the future → the permit is back.
        drop(permit);
        let host3 = host.clone();
        let again = tokio::spawn(async move {
            host3
                .call_tool("web_search", serde_json::json!({"query": "y"}), "ws-tout-2")
                .await
        });
        let res = again.await.unwrap();
        assert!(
            res.is_err(),
            "second call must acquire the released permit and fast-fail: {res:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): the gate covers ONLY the
    /// `web_search` family — a non-search tool (read_file) completes while
    /// the permit is held.
    #[tokio::test]
    async fn non_web_search_tools_ignore_the_gate() {
        let dir = test_dir();
        let path = dir.join("gate.txt");
        std::fs::write(&path, "gate").unwrap();
        let host = Arc::new(host_with_failing_reader(
            JournalRecorder::new(dir.clone()),
            &dir,
        ));

        let permit = host
            .web_search_semaphore
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let host2 = host.clone();
        let handle = tokio::spawn(async move {
            host2
                .call_tool(
                    "read_file",
                    serde_json::json!({"target_file": path.to_string_lossy()}),
                    "ws-gate-2",
                )
                .await
        });

        // read_file is not gated — it completes while the permit is held.
        let result = handle.await.unwrap().unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains("gate"), "{}", result.output);
        drop(permit);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-11 (direction correction): live DeepSeek backend check —
    /// the web_search executor against the real `/v1/responses` endpoint
    /// with the real key. Evidence that the request shape (async-openai
    /// `CreateResponseArgs`) is accepted and the raw-JSON parse extracts
    /// content + citations (the typed Responses parse does not match the
    /// DeepSeek backend). Double-gated: `--ignored` + `ORZ_TEST_LIVE=1`.
    #[tokio::test]
    #[ignore = "live: requires a real DeepSeek key (ORZ_TEST_LIVE=1)"]
    async fn live_deepseek_web_search_roundtrip() {
        if std::env::var("ORZ_TEST_LIVE").as_deref() != Ok("1") {
            eprintln!("skipping live web search test (ORZ_TEST_LIVE != 1)");
            return;
        }
        let key = orz_loop::gateway::credentials::read_agent_api_key()
            .expect("DeepSeek key (orz-deepseek/agent)");
        let config = WebSearchConfig::Enabled {
            api_key: key,
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-v4-flash".to_string(),
            extra_headers: Default::default(),
            alpha_test_key: None,
        };
        let client =
            orz_tools::implementations::web_search::client::WebSearchClient::new(&config, None)
                .expect("client build");
        let (content, citations) = client
            .search("2026 年 xAI 最新发布的技术", None)
            .await
            .expect("live search");
        assert!(!content.is_empty(), "search content must not be empty");
        assert!(
            !citations.is_empty(),
            "DeepSeek open_page citations expected; content head: {}",
            &content[..content.len().min(80)]
        );
    }

    /// ADR-0006 (2026-08-11): the host's redacted-config accessor — the
    /// only sanctioned serialization exit (the raw config's Debug contains
    /// the api_key and must never be logged or journaled).
    #[tokio::test]
    async fn host_web_search_config_redacted_never_leaks_key() {
        struct OkReader;
        impl crate::credentials::CredentialReader for OkReader {
            fn read(&self) -> Result<String, crate::credentials::CredentialError> {
                Ok("sk-test-9f8e7d6c5b4a".to_string())
            }
        }
        let dir = test_dir();
        let host = OrzHost::with_credential_reader(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
            None,
            Arc::new(OkReader),
        )
        .expect("host build");
        assert!(host.web_search_configured());
        let redacted = host
            .web_search_config_redacted()
            .expect("enabled config redacted");
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(json.contains("***REDACTED***"), "{json}");
        assert!(!json.contains("sk-test-9f8e7d6c5b4a"), "{json}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// C2-1 (2026-08-11, ADR-0006 web-search slice): end-to-end through the
    /// shared agent loop with a REAL host — the main lane delegates
    /// web_search to the external retrieval subagent, and the subagent
    /// SELF-EXECUTES the web tool through the host (no
    /// `nested_subagent_dispatch_refused`). With the failing credential
    /// reader the client is not injected, so the call fast-fails at the
    /// toolset sink ("missing required resource") — proving the routing
    /// change without any real API call.
    #[tokio::test]
    async fn retrieval_lane_web_search_routes_to_host_and_fast_fails() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = host_with_failing_reader(journal, &dir);

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            orz_loop::gateway::fake::ScriptedResponse::tool_calls(vec![ToolCall {
                name: "web_search".to_string(),
                arguments: serde_json::json!({"query": "t"}),
                call_id: "call-1".to_string(),
            }]),
            orz_loop::gateway::fake::ScriptedResponse::tool_calls(vec![ToolCall {
                name: "web_search".to_string(),
                arguments: serde_json::json!({"query": "t"}),
                call_id: "call-2".to_string(),
            }]),
            orz_loop::gateway::fake::ScriptedResponse::text("检索完成"),
            orz_loop::gateway::fake::ScriptedResponse::text("完成"),
            orz_loop::gateway::fake::ScriptedResponse::text("完成"),
        ]));
        // ACAF shadow 默认（signer 存量失败族修复，2026-09-07）：逻辑测试
        // 默认 shadow；生产默认 fail-closed 在下游 crate 测试编译时生效。
        let controller = orz_loop::AgentLoopController::with_gateway(gateway)
            .with_acaf_fail_closed(false)
            .with_retrieval_enabled(true);
        controller
            .run_turn(
                &host,
                "查一下",
                "RUN-R2H",
                "manifest-sha",
                0,
                None,
                None,
                None,
            )
            .await
            .expect("run turn");

        let content = std::fs::read_to_string(dir.join("j").join("events.jsonl")).unwrap();
        let events: Vec<RunEvent> = content
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        // The web tool was never refused as a nested dispatch.
        assert!(
            !events.iter().any(|e| {
                e.payload.get("error").and_then(|v| v.as_str())
                    == Some("nested_subagent_dispatch_refused")
            }),
            "the lane-internal web call must self-execute, not be nested-refused"
        );
        // The lane-internal call (call-2) reached the REAL toolset and
        // fast-failed: client not injected (failing reader) →
        // "missing required resource" at the call_tool sink.
        let failure = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-2")
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
            })
            .expect("web_search host failure journaled");
        let err = failure
            .payload
            .get("error")
            .and_then(|v| v.as_str())
            .expect("error field");
        assert!(
            err.contains("missing required resource"),
            "web_search must reach the toolset sink: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn call_read_file_returns_content() {
        let dir = test_dir();
        let path = dir.join("test.txt");
        std::fs::write(&path, "hello orz tools").unwrap();

        let toolset = shared_toolset();
        let result = toolset
            .call(
                "read_file",
                serde_json::json!({"target_file": path}),
                "call-1",
                None,
            )
            .await
            .expect("read_file call");
        assert!(result.prompt_text.contains("hello orz tools"), "{result:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ORZ-LARGE-FILE-READ-CONTRACT (ADR-0010 §14.22): a file above the
    /// coarse gate returns the bounded read-handle envelope through the host
    /// toolset — never full content.
    #[tokio::test]
    async fn call_read_file_large_file_returns_handle_envelope() {
        let dir = test_dir();
        let path = dir.join("big.txt");
        // 尺寸对齐 TER T1.10（2026-09-04）粗门 16K→64K 新口径——旧 39 KiB
        // 夹具在 64K 门下返回全文、句柄信封不再触发（signer 失败族修复中
        // 暴露的测试漂移，2026-09-07 修正）。
        let content = format!("{}\n", "x".repeat(200)).repeat(400);
        assert!(content.len() > 64 * 1024);
        std::fs::write(&path, &content).unwrap();

        let toolset = shared_toolset();
        let result = toolset
            .call(
                "read_file",
                serde_json::json!({"target_file": path}),
                "call-big",
                None,
            )
            .await
            .expect("read_file call");
        assert!(
            result.prompt_text.contains("[read handle]"),
            "large file must return the read-handle envelope, got: {result:?}"
        );
        assert!(
            result.prompt_text.len() <= 8 * 1024,
            "envelope must stay bounded, got {} bytes",
            result.prompt_text.len()
        );
        assert!(
            result.prompt_text.contains("truncated=true"),
            "large multi-line file must report truncated, got: {result:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ORZ-LARGE-FILE-READ-CONTRACT (P3-1): the `[toolset.read_file]` coarse
    /// gate lands in the finalized toolset — a ~10 KiB file returns the
    /// envelope under an 8 KiB gate and full content under the default.
    #[tokio::test]
    async fn read_file_coarse_gate_changes_envelope_threshold() {
        let dir = test_dir();
        let path = dir.join("mid.txt");
        let content = format!("{}\n", "y".repeat(100)).repeat(100); // ~10.2 KiB
        std::fs::write(&path, &content).unwrap();

        let gated = tools::build_toolset(
            &dir,
            &WebSearchConfig::Disabled,
            Some(8 * 1024),
            Arc::new(LocalTerminalBackend::new()),
        )
        .expect("gated toolset");
        let result = gated
            .call(
                "read_file",
                serde_json::json!({"target_file": path}),
                "call-gated",
                None,
            )
            .await
            .expect("read_file call");
        assert!(
            result.prompt_text.contains("[read handle]"),
            "8 KiB gate must envelope a ~10 KiB file, got: {result:?}"
        );

        let default = tools::build_toolset(
            &dir,
            &WebSearchConfig::Disabled,
            None,
            Arc::new(LocalTerminalBackend::new()),
        )
        .expect("default toolset");
        let result = default
            .call(
                "read_file",
                serde_json::json!({"target_file": path}),
                "call-default",
                None,
            )
            .await
            .expect("read_file call");
        assert!(
            !result.prompt_text.contains("[read handle]"),
            "default 64 KiB gate must return full content for ~10 KiB, got: {result:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-08 blackboard-partition review closure (conformance D1-1):
    /// the host derives the ToolResult exit_code from the structured output
    /// — search_replace reports "applied" only via EditsApplied (Ok outputs
    /// like FileNotFound/NoMatchesFound changed nothing → non-zero), every
    /// other successful output is 0. Previously hardcoded `None` made the
    /// controller's edit-action gate dead in production.
    #[tokio::test]
    async fn call_tool_maps_real_exit_codes() {
        let dir = test_dir();
        let toolset = shared_toolset();

        // read_file (generic success) → Some(0).
        let read = dir.join("readme.txt");
        std::fs::write(&read, "x").unwrap();
        let ok = toolset
            .call(
                "read_file",
                serde_json::json!({"target_file": read}),
                "c-r",
                None,
            )
            .await
            .expect("read_file");
        assert_eq!(crate::tools::exit_code_from_output(&ok.output), Some(0));

        // search_replace on a missing file — Ok(FileNotFound), changed
        // nothing → non-zero ("实际变动" gate stays closed).
        let missing = toolset
            .call(
                "search_replace",
                serde_json::json!({
                    "file_path": dir.join("nope.txt"),
                    "old_string": "a",
                    "new_string": "b",
                }),
                "c-m",
                None,
            )
            .await
            .expect("search_replace on missing file");
        assert_eq!(
            crate::tools::exit_code_from_output(&missing.output),
            Some(1),
            "a search_replace that applied nothing must be non-zero"
        );

        // A real applied edit → EditsApplied → Some(0).
        let target = dir.join("edit.txt");
        std::fs::write(&target, "before").unwrap();
        let applied = toolset
            .call(
                "search_replace",
                serde_json::json!({
                    "file_path": target,
                    "old_string": "before",
                    "new_string": "after",
                }),
                "c-a",
                None,
            )
            .await
            .expect("search_replace applied");
        assert_eq!(
            crate::tools::exit_code_from_output(&applied.output),
            Some(0),
            "an applied edit must be zero"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn orz_host_full_loophost_chain() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join(".gsa").join("runs").join("RUN-T"));
        let host =
            OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted).expect("host build");

        // Two texts — the counterexample gate (§4.6) intercepts the first.
        let gateway: Arc<dyn ModelGateway> =
            Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let controller = AgentLoopController::with_gateway(gateway).with_acaf_fail_closed(false);
        let (response, _, _) = controller
            .run_turn(&host, "hi", "RUN-T", "manifest-sha", 0, None, None, None)
            .await
            .expect("run turn");
        assert_eq!(response, "完成");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3 wiring: a host WITHOUT a permission bridge must fail closed —
    /// never auto-allow (review P1-1).
    #[tokio::test]
    async fn host_without_bridge_fails_closed_on_permission() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted).expect("host");

        for (risk, tool) in [
            (RiskClass::ReadOnly, "read_file"),
            (RiskClass::SandboxEscape, "bash"),
        ] {
            let decision = host
                .request_permission(risk, tool, &serde_json::json!({}))
                .await
                .expect("request_permission");
            assert_eq!(
                decision,
                PermitDecision::Deny,
                "no bridge → fail closed for {tool}"
            );
            // 0bt④：无桥 fail-closed 也须自报来源（封闭集观测面）。
            let (decision, source) = host
                .request_permission_with_source(risk, tool, &serde_json::json!({}))
                .await
                .expect("request_permission_with_source");
            assert_eq!(decision, PermitDecision::Deny);
            assert_eq!(
                source,
                Some(PermitSource::FailClosed),
                "no-bridge fail-closed must report source=fail_closed for {tool}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P1 permit keystore injection: a permit issued through the host signs
    /// with the injected keystore signer and self-verifies; the artifact
    /// lands under `{cwd}/.gsa/one_shot_permit/` (Python namespace layout).
    #[tokio::test]
    async fn issue_permit_uses_injected_keystore_signer() {
        use orz_assurance::permit::verify_sensitive_action_permit;

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted)
            .expect("host build")
            .with_permit_signer(Arc::new(
                MemoryInstallationKeyStore::from_secret(&[0x42u8; 32]).unwrap(),
            ));

        let envelope = PermitEnvelope {
            conversation_id: "conv-1".into(),
            envelope_id: "env-1".into(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        };
        let digest = |s: &str| orz_assurance::sha256_hex(s.as_bytes());
        let permit = host
            .issue_permit(
                &envelope,
                &digest("confirmation"),
                &digest("action"),
                &digest("target"),
                &digest("impact scope"),
                1,
                300,
            )
            .expect("issue permit");

        assert!(
            permit.integrity.key_id.starts_with("KEY-"),
            "key id: {}",
            permit.integrity.key_id
        );
        let verification =
            verify_sensitive_action_permit(host.permit_signer().as_ref(), &envelope, &permit, None);
        assert!(verification.valid, "{:?}", verification.errors);
        let artifact = dir
            .join(".gsa")
            .join("one_shot_permit")
            .join(format!("{}.issued.json", permit.permit_id));
        assert!(artifact.is_file(), "missing permit artifact");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3 wiring: host WITH the bridge — Read auto-allows through the
    /// permission manager; Bash 亦自动放行（默认自动审批＝初始 yolo，
    /// 用户令 2026-09-26「直接开auto mode就行」）——即使网关为 dead
    /// gateway 也不再等待/拒绝。
    #[tokio::test]
    async fn host_with_bridge_default_yolo_auto_allows_read_and_bash() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let journal = JournalRecorder::new(dir.join("j"));
                let host = OrzHost::with_bridge(
                    "sess-1",
                    journal,
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                    None, // headless dead gateway
                )
                .expect("host with bridge");

                let read = host
                    .request_permission(
                        RiskClass::ReadOnly,
                        "read_file",
                        &serde_json::json!({"target_file": "a.txt"}),
                    )
                    .await
                    .expect("read request");
                assert_eq!(read, PermitDecision::AllowOnce);

                let bash = host
                    .request_permission(
                        RiskClass::SandboxEscape,
                        "run_terminal_cmd",
                        &serde_json::json!({"command": "dir"}),
                    )
                    .await
                    .expect("bash request");
                assert_eq!(
                    bash,
                    PermitDecision::AllowOnce,
                    "default yolo must auto-approve Ask-class requests"
                );

                let _ = std::fs::remove_dir_all(&dir);
            })
            .await
    }

    /// Streaming slice: `on_text_delta` forwards the chunk to the live
    /// client as an `agent_message_chunk` session notification, in order.
    #[tokio::test]
    async fn on_text_delta_forwards_session_notification_to_gateway() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let (tx, mut rx) =
                    tokio::sync::mpsc::unbounded_channel::<xai_acp_lib::AcpClientMessage>();
                let sender = xai_acp_lib::AcpAgentGatewaySender::new(tx);
                let host = OrzHost::with_bridge(
                    "sess-1",
                    JournalRecorder::new(dir.join("j")),
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                    Some(sender),
                )
                .expect("host with bridge");

                host.on_text_delta("你好");
                host.on_text_delta("世界");

                for expected in ["你好", "世界"] {
                    let msg = rx.recv().await.expect("notification");
                    match msg {
                        xai_acp_lib::AcpClientMessage::SessionNotification(args) => {
                            assert_eq!(args.request.session_id.0.as_ref(), "sess-1");
                            match args.request.update {
                                acp::SessionUpdate::AgentMessageChunk(chunk) => {
                                    match chunk.content {
                                        acp::ContentBlock::Text(t) => {
                                            assert_eq!(t.text, expected);
                                        }
                                        other => panic!("unexpected content block: {other:?}"),
                                    }
                                }
                                other => panic!("unexpected update: {other:?}"),
                            }
                        }
                        other => panic!("unexpected message: {other:?}"),
                    }
                }

                // Headless host (no gateway): on_text_delta is a silent no-op.
                let headless = OrzHost::new(
                    JournalRecorder::new(dir.join("j2")),
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                )
                .expect("headless host");
                headless.on_text_delta("丢弃");

                let _ = std::fs::remove_dir_all(&dir);
            })
            .await
    }

    /// Stability fix (2026-08-07): a hung test child plus its grandchildren
    /// must ALL be terminated on timeout — killing only the direct child
    /// left an orphan holding the capture pipes (the 52-minute forth hang:
    /// harness killed orz, orz's hung pytest kept the pipes open, the
    /// harness's communicate() blocked on EOF forever).
    #[cfg(windows)]
    #[tokio::test]
    async fn run_tests_timeout_kills_process_tree() {
        let dir = test_dir();
        let pidfile = dir.join("gc.pid");
        let script = format!(
            // Child spawns a grandchild that inherits our stdout/stderr
            // pipes and writes its pid to the pidfile; both then sleep
            // forever (deadlock-style hang).
            "import subprocess, sys, time, pathlib, os; \
             g = subprocess.Popen([sys.executable, '-c', \
             'import time, pathlib, os; pathlib.Path(r\"{pf}\").write_text(str(os.getpid())); \
             [time.sleep(1) for _ in range(999999)]'], \
             stdout=sys.stdout, stderr=sys.stderr); \
             [time.sleep(1) for _ in range(999999)]",
            pf = pidfile.display().to_string().replace('\\', "/"),
        );
        let runner = orz_loop::host::TestRunner {
            command: vec!["python".to_string(), "-c".to_string(), script.clone()],
            timeout: Some(std::time::Duration::from_secs(2)),
            env: Vec::new(),
        };
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_test_runner(Some(runner));
        let result = host.run_tests().await.expect("run_tests returns a result");
        assert!(
            result.output.contains("TIMED OUT"),
            "expected TIMED OUT marker: {}",
            result.output
        );
        assert_eq!(result.exit_code, None);
        assert!(
            result.timed_out,
            "F-09 wall-clock kill must carry the structured timed_out flag"
        );
        // Give TaskKill a moment to reap the tree, then verify the
        // grandchild (the pipe holder) is gone.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let gcid: i64 = std::fs::read_to_string(&pidfile)
            .expect("grandchild pid written")
            .trim()
            .parse()
            .expect("pid parses");
        let listing = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {gcid}"), "/NH"])
            .output()
            .expect("tasklist runs");
        let text = String::from_utf8_lossy(&listing.stdout);
        assert!(
            !text.contains(&gcid.to_string()),
            "grandchild {gcid} survived the tree kill: {text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RT-002 (2026-08-11): the fixed test command runs env_clear()ed — the
    /// test process sees ONLY the platform allowlist (TEST_ENV_ALLOWLIST)
    /// plus the harness's explicit `TestRunner::env` entries. Host
    /// environment (session variables, arbitrary paths, secrets) is never
    /// inherited.
    #[tokio::test]
    async fn run_tests_env_is_isolated_from_host() {
        let dir = test_dir();
        let runner = orz_loop::host::TestRunner {
            command: vec![
                "python".to_string(),
                "-c".to_string(),
                "import os; print('KEYS:' + ','.join(sorted(os.environ.keys()))); \
                 print('INJECTED:' + os.environ.get('ORZ_TEST_RUNNER_INJECTED', '<absent>'))"
                    .to_string(),
            ],
            timeout: Some(std::time::Duration::from_secs(30)),
            env: vec![(
                "ORZ_TEST_RUNNER_INJECTED".to_string(),
                "visible".to_string(),
            )],
        };
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_test_runner(Some(runner));
        let result = host.run_tests().await.expect("run_tests returns a result");
        let keys_line = result
            .output
            .lines()
            .find(|l| l.starts_with("KEYS:"))
            .expect("KEYS line in output");
        let keys: Vec<&str> = keys_line["KEYS:".len()..].split(',').collect();
        // Windows env vars are case-insensitive — compare uppercase.
        let mut allowed: std::collections::HashSet<String> = TEST_ENV_ALLOWLIST
            .iter()
            .map(|k| k.to_uppercase())
            .collect();
        allowed.insert("ORZ_TEST_RUNNER_INJECTED".to_string());
        // CPython 3.13+ 初始化时把 UTF-8 模式的 LC_CTYPE 写入自身
        // os.environ（即使进程环境无该变量）——不是宿主泄漏，显式放行。
        allowed.insert("LC_CTYPE".to_string());
        for key in &keys {
            assert!(
                allowed.contains(&key.to_uppercase()),
                "test process saw host env var '{key}' that is not in the allowlist"
            );
        }
        assert!(
            keys.iter().any(|k| k.eq_ignore_ascii_case("PATH")),
            "PATH (command lookup) is always in the allowlist"
        );
        assert!(
            result.output.contains("INJECTED:visible"),
            "harness-injected env entry must be visible: {}",
            result.output
        );
        assert!(
            !result.output.contains("ORZ_TEST_RUNNER="),
            "the runner's own env var must not leak into the test process"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RT-003 (2026-08-11): the workspace delta records added/modified/
    /// deleted files across a run_tests call (ADR-0010 §3.8.2
    /// workspace-delta recording — tests may write files, populate caches,
    /// etc.; the change list is the audit trace).
    #[tokio::test]
    async fn run_tests_records_workspace_delta() {
        let dir = test_dir();
        std::fs::write(dir.join("existing.txt"), "v1").unwrap();
        std::fs::write(dir.join("todelete.txt"), "x").unwrap();
        let runner = orz_loop::host::TestRunner {
            command: vec![
                "python".to_string(),
                "-c".to_string(),
                "from pathlib import Path; \
                 Path('new.txt').write_text('n'); \
                 Path('existing.txt').write_text('version two - longer'); \
                 Path('todelete.txt').unlink()"
                    .to_string(),
            ],
            timeout: Some(std::time::Duration::from_secs(30)),
            env: Vec::new(),
        };
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_test_runner(Some(runner));
        let result = host.run_tests().await.expect("run_tests returns a result");
        use orz_loop::host::WorkspaceDeltaKind;
        let kinds: Vec<(String, WorkspaceDeltaKind)> = result
            .workspace_delta
            .iter()
            .map(|e| (e.path.clone(), e.kind.clone()))
            .collect();
        assert!(
            kinds.contains(&("new.txt".to_string(), WorkspaceDeltaKind::Added)),
            "added file in delta: {kinds:?}"
        );
        assert!(
            kinds.contains(&("existing.txt".to_string(), WorkspaceDeltaKind::Modified)),
            "modified file in delta: {kinds:?}"
        );
        assert!(
            kinds.contains(&("todelete.txt".to_string(), WorkspaceDeltaKind::Deleted)),
            "deleted file in delta: {kinds:?}"
        );
        assert!(
            !result.workspace_delta_truncated,
            "small delta must not be flagged truncated"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RT-003: the delta diff caps the entry list (deterministic order,
    /// truncation flag) so a test that churns thousands of files cannot
    /// drown the journal event.
    #[test]
    fn workspace_delta_diff_caps_and_sorts() {
        let mut before = std::collections::HashMap::new();
        let mut after = std::collections::HashMap::new();
        // Seed one untouched file (not in the diff).
        before.insert("stable.txt".to_string(), (1, 1, 0));
        after.insert("stable.txt".to_string(), (1, 1, 0));
        // More changes than the cap — all "added".
        for i in 0..(RUN_TESTS_DELTA_MAX_ENTRIES + 50) {
            after.insert(format!("gen/{i}.txt"), (1, 2, 0));
        }
        let (entries, truncated) =
            workspace_delta_diff(&before, &after, RUN_TESTS_DELTA_MAX_ENTRIES);
        assert!(truncated, "over-cap diff must be flagged truncated");
        assert_eq!(entries.len(), RUN_TESTS_DELTA_MAX_ENTRIES);
        // Deterministic order.
        let mut sorted = entries.clone();
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(entries, sorted);
        // Deleted detection — a separate small diff (an over-cap diff
        // truncates away later-sorted entries, which is the cap working).
        let mut before2 = std::collections::HashMap::new();
        before2.insert("gone.txt".to_string(), (1, 1, 0));
        before2.insert("stable.txt".to_string(), (1, 1, 0));
        let after2 = std::collections::HashMap::from([("stable.txt".to_string(), (1, 1, 0))]);
        let (entries, truncated) =
            workspace_delta_diff(&before2, &after2, RUN_TESTS_DELTA_MAX_ENTRIES);
        assert!(!truncated);
        assert!(
            entries
                .iter()
                .any(|e| e.path == "gone.txt"
                    && e.kind == orz_loop::host::WorkspaceDeltaKind::Deleted),
            "deleted file detected"
        );
    }

    /// P0-1 (2026-08-08 stall guards): a tool call that exceeds the host's
    /// per-call wall-clock budget must fail with `ToolError::Timeout` (the
    /// reason carried for the journal / model), kill the tool's process
    /// tree INCLUDING grandchildren (the orphan-holds-pipes hang
    /// mechanism), and leave the session usable for the next call.
    #[cfg(windows)]
    #[tokio::test]
    async fn call_tool_timeout_kills_process_tree() {
        let dir = test_dir();
        // A python script that spawns a grandchild (which writes its pid
        // and sleeps forever), then sleeps forever itself — a deadlock-style
        // hang inside run_terminal_cmd.
        let pidfile = dir.join("gc.pid");
        let script = dir.join("hang.py");
        let script_body = format!(
            "import subprocess, sys, time, pathlib, os; \
             g = subprocess.Popen([sys.executable, '-c', \
             'import time, pathlib, os; pathlib.Path(r\"{pf}\").write_text(str(os.getpid())); \
             [time.sleep(1) for _ in range(999999)]'], \
             stdout=sys.stdout, stderr=sys.stderr); \
             [time.sleep(1) for _ in range(999999)]",
            pf = pidfile.display().to_string().replace('\\', "/"),
        );
        std::fs::write(&script, &script_body).unwrap();

        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        // 2s (not 500ms — 2026-08-08 review P3-6b): under parallel-test CPU
        // contention the 500ms budget could expire before python even
        // spawned the grandchild, making the pidfile assertion below panic
        // spuriously. 2s is comfortably past python's cold start while far
        // below any real tool window.
        .expect("host")
        .with_tool_timeout(std::time::Duration::from_millis(2000));
        let result = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": format!("python {}", script.display()),
                    "description": "stall-guard timeout test",
                }),
                "call-t1",
            )
            .await;
        let err = result.expect_err("hung tool must time out");
        assert!(
            err.to_string().contains("TIMED OUT"),
            "expected TIMED OUT marker: {err}"
        );
        // The session survives — including for PROCESS-type tools (2026-08-08
        // review F1: `kill_active` must not latch the scope, or every later
        // spawn would die on arrival). The follow-up is a real
        // `run_terminal_cmd` — the original review flagged that a `read_file`
        // follow-up (no process) could not catch the latch.
        // 0bg ①（2026-09-22）：续跑活性调用改**显式更长覆盖**——宿主 2s 默认
        // 预算在并行负载（宿主套件并行档）下可能被冷启动壳层吃掉，让本测试
        // 假红（0bd 轮定位：串行 322/0/5 全绿、并行恒现 1 条）。语义不变：
        // 断言的是「超时后进程类工具仍可用」，不是 2s 这个界；显式 30s 覆盖
        // 只去负载敏感，不放宽断言强度。
        let follow_up = host
            .call_tool_with_timeout(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo orz-alive",
                    "description": "post-timeout liveness",
                }),
                "call-t2",
                Some(std::time::Duration::from_secs(30)),
            )
            .await
            .expect("process-type tool works after the timeout");
        assert!(
            follow_up.output.contains("orz-alive"),
            "follow-up bash returned the expected output: {follow_up:?}"
        );
        // Give the kill a moment to reap the tree, then verify the
        // grandchild (the pipe holder) is gone.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let gcid: i64 = std::fs::read_to_string(&pidfile)
            .expect("grandchild pid written")
            .trim()
            .parse()
            .expect("pid parses");
        let listing = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {gcid}"), "/NH"])
            .output()
            .expect("tasklist runs");
        let text = String::from_utf8_lossy(&listing.stdout);
        assert!(
            !text.contains(&gcid.to_string()),
            "grandchild {gcid} survived the tool-timeout tree kill: {text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-C S4 (2026-08-16): a per-call timeout override is honored — the
    /// host is configured with a LONG budget (10s), but
    /// `call_tool_with_timeout` with a 1.2s override must kill the process
    /// tree at the override bound, fail with `ToolError::Timeout` carrying
    /// the override budget, and leave the session usable.
    #[cfg(windows)]
    #[tokio::test]
    async fn call_tool_with_timeout_override_is_honored() {
        let dir = test_dir();
        let script = dir.join("hang.py");
        std::fs::write(
            &script,
            "import time\n[time.sleep(1) for _ in range(999999)]\n",
        )
        .unwrap();
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_tool_timeout(std::time::Duration::from_secs(10));
        let started = std::time::Instant::now();
        let result = host
            .call_tool_with_timeout(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": format!("python {}", script.display()),
                    "description": "per-call override timeout test",
                }),
                "call-t3",
                Some(std::time::Duration::from_millis(1200)),
            )
            .await;
        let err = result.expect_err("override bound must time out");
        assert!(
            err.to_string().contains("TIMED OUT after 1.2s"),
            "error carries the override budget: {err}"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "override bound respected: {:?}",
            started.elapsed()
        );
        // The session survives the override kill (kill_active not latched).
        let follow_up = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo orz-alive",
                    "description": "post-override-timeout liveness",
                }),
                "call-t4",
            )
            .await
            .expect("process-type tool works after the override timeout");
        assert!(follow_up.output.contains("orz-alive"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TER T1.6 (2026-09-04): orz-host 把终端读取时现算的 live 快照映射
    /// 为 LoopHost 事实（黑板 `section=processes` 分区的事实源）。
    #[tokio::test]
    async fn terminal_live_processes_maps_terminal_snapshot() {
        let dir = std::env::temp_dir().join(format!("orz-host-live-proc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let journal = JournalRecorder::new(dir.clone());
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted).expect("host");
        let request = orz_tools::computer::types::TerminalRunRequest {
            command: "sleep 60".to_string(),
            working_directory: dir.clone(),
            env: std::collections::HashMap::new(),
            timeout: std::time::Duration::from_secs(3600),
            output_byte_limit: 10000,
            output_file: dir.join("live-test.out"),
            notification_handle: orz_tools::notification::types::ToolNotificationHandle::noop(),
            tool_call_id: "live-host-test".to_string(),
            display_command: Some("sleep 60".to_string()),
            auto_background_on_timeout: false,
            foreground_block_budget: None,
            kind: orz_tools::computer::types::TaskKind::Bash,
            owner_session_id: Some("sess-main".to_string()),
            description: Some("live probe".to_string()),
        };
        let handle = host
            .terminal
            .run_background(request)
            .await
            .expect("spawn bg task");
        let facts = host.terminal_live_processes().await;
        let row = facts
            .iter()
            .find(|f| f.task_id == handle.task_id)
            .unwrap_or_else(|| panic!("live facts must include task {}", handle.task_id));
        assert_eq!(row.display_command.as_deref(), Some("sleep 60"));
        assert!(row.pid.is_some(), "row must carry pid: {row:?}");
        assert!(row.killable, "running task must be killable: {row:?}");
        assert_eq!(row.owner_session_id.as_deref(), Some("sess-main"));

        let outcome = host.terminal.kill_task(&handle.task_id).await;
        assert!(
            matches!(
                outcome,
                orz_tools::computer::types::KillOutcome::Killed
                    | orz_tools::computer::types::KillOutcome::AlreadyExited
            ),
            "kill cleanup: {outcome:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// TER 全面审查 P1-1 (2026-09-04)：idle-kill 事实映射——只收
    /// signal=idle_killed + status=killed 的任务；reason 按实际阈值渲染；
    /// 无输出路径不标记已报（可下次补报）；drain 去重。
    #[test]
    fn terminal_idle_kill_facts_from_dedupes_and_formats_reason() {
        use orz_tools::computer::types::TaskLiveSnapshot;
        let snap = |id: &str,
                    signal: Option<&str>,
                    timeout_ms: Option<u64>,
                    status: &str|
         -> TaskLiveSnapshot {
            TaskLiveSnapshot {
                task_id: id.to_string(),
                command: "sleep".to_string(),
                display_command: Some("sleep".to_string()),
                pid: Some(42),
                elapsed_ms: 5005,
                status: status.to_string(),
                signal: signal.map(str::to_string),
                idle_timeout_ms: timeout_ms,
                total_bytes: 1024,
                cpu_micros: 0,
                killable: false,
                owner_session_id: Some("sess".to_string()),
                description: None,
            }
        };
        let mut reported = std::collections::HashSet::new();
        let facts = super::terminal_idle_kill_facts_from(
            vec![
                (
                    snap("t1", Some("idle_killed"), Some(5000), "killed"),
                    Some(std::path::PathBuf::from("/tmp/t1.log")),
                ),
                (
                    snap("t2", Some("idle_killed"), None, "killed"),
                    Some(std::path::PathBuf::from("/tmp/t2.log")),
                ),
                (
                    snap("t3", Some("timeout"), Some(5000), "killed"),
                    Some(std::path::PathBuf::from("/tmp/t3.log")),
                ),
                (snap("t4", Some("idle_killed"), Some(5000), "killed"), None),
            ],
            &mut reported,
        );
        assert_eq!(facts.len(), 2, "{facts:?}");
        assert_eq!(facts[0].task_id, "t1");
        assert_eq!(facts[0].reason, "no output growth or CPU activity for 5s");
        assert_eq!(facts[0].output_file, "/tmp/t1.log");
        assert_eq!(facts[0].wall_ms, 5005);
        assert_eq!(facts[0].total_bytes, 1024);
        assert_eq!(facts[1].reason, "no output growth or CPU activity for 300s");
        assert!(reported.contains("t1"));
        assert!(
            !reported.contains("t3"),
            "non-idle kill must not be reported"
        );
        assert!(
            !reported.contains("t4"),
            "missing output path must not mark the task reported"
        );
        let again = super::terminal_idle_kill_facts_from(
            vec![(
                snap("t1", Some("idle_killed"), Some(5000), "killed"),
                Some(std::path::PathBuf::from("/tmp/t1.log")),
            )],
            &mut reported,
        );
        assert!(again.is_empty(), "drain must dedupe: {again:?}");
    }

    /// TER T1.11 (W-F13b)：run_terminal_cmd 长输出被截断时，host 映射
    /// `output_truncated` + 持久化输出检索对象（完整输出落盘路径 + 截断前
    /// 字节）——模型无需 .gsa 即可按对象补读。
    #[tokio::test]
    async fn run_terminal_cmd_truncation_carries_output_object() {
        let dir = std::env::temp_dir().join(format!("orz-host-f13-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host");
        // 30K 字符输出远超默认 8K 工具输出档 → 必截断（Windows PowerShell /
        // Linux bash 双语法）。
        let command = if cfg!(windows) {
            "'x' * 30000".to_string()
        } else {
            "python3 -c \"print('x' * 30000)\"".to_string()
        };
        let result = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": command,
                    "description": "output object truncation test",
                }),
                "call-f13-host",
            )
            .await
            .expect("run_terminal_cmd succeeds");
        assert!(
            result.output_truncated,
            "30K output must truncate under the default 8K budget"
        );
        let object = result
            .output_object
            .expect("truncated output must carry the retrieval object");
        assert!(object.total_bytes >= 30_000, "{object:?}");
        assert!(
            std::path::Path::new(&object.output_object_id).exists(),
            "output object must persist on disk: {}",
            object.output_object_id
        );
        assert!(
            result.output.contains("read_file"),
            "model-facing pointer text must survive: {}",
            result.output
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-0m S3（ADR-0010 §14.56 D3 装配接线端到端）：`.gsa` symlink 会话
    /// 卷实机构造——host 装配期经链接 canonical 解析卷根；终端截断补读链
    /// 把完整输出落进卷内并经 `session/terminal/*.log` 窗口 read_file 可达；
    /// run_tests 全量输出 `run_tests_output.txt` 同样落卷且窗口可读；卷内
    /// 非窗口面保持 agent-invisible。
    #[tokio::test]
    async fn session_volume_symlink_windows_end_to_end() {
        let base = std::env::temp_dir().join(format!("orz-host-0m-s3-{}", std::process::id()));
        let volume = base.join("volume");
        let workspace = base.join("workspace");
        std::fs::create_dir_all(&volume).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();
        // 会话卷在卷根之外、`.gsa` 为指向卷根的链接（评测容器挂载形态）。
        #[cfg(windows)]
        let link_ok = std::os::windows::fs::symlink_dir(&volume, workspace.join(".gsa")).is_ok();
        #[cfg(not(windows))]
        let link_ok = std::os::unix::fs::symlink(&volume, workspace.join(".gsa")).is_ok();
        if !link_ok {
            eprintln!("symlink creation unsupported, skipping");
            return;
        }
        // 卷内非窗口面负测样本（装配前就位）。
        std::fs::write(volume.join("secret.txt"), "interior").unwrap();

        let host = OrzHost::new(
            JournalRecorder::new(base.join("j")),
            &workspace,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host");

        // 1) 终端截断补读链：>8K 输出 → 截断 + 检索对象词法 id 在
        //    `.gsa/session/terminal/*.log`，内容经链接落进卷内。
        let command = if cfg!(windows) {
            "'x' * 30000".to_string()
        } else {
            "python3 -c \"print('x' * 30000)\"".to_string()
        };
        let result = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": command,
                    "description": "session volume truncation window test",
                }),
                "call-0m-s3-term",
            )
            .await
            .expect("run_terminal_cmd succeeds");
        assert!(result.output_truncated, "30K output must truncate");
        let object = result
            .output_object
            .expect("truncated output must carry the retrieval object");
        let object_id = std::path::PathBuf::from(&object.output_object_id);
        let rel = object_id
            .strip_prefix(&workspace)
            .expect("object id stays under the workspace cwd");
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        assert!(
            rel_str.starts_with(".gsa/session/terminal/") && rel_str.ends_with(".log"),
            "object id must match the terminal window shape: {rel_str}"
        );
        // 链接本身即卷根：卷内落点 = rel 去掉 `.gsa` 段。
        let in_volume = volume.join(rel.strip_prefix(".gsa").unwrap_or(rel));
        assert!(
            in_volume.exists(),
            "full output must land inside the volume through the link: {}",
            in_volume.display()
        );

        // 2) 窗口 1 正测：read_file 经窗口读回截断全文。
        let read_back = host
            .call_tool(
                "read_file",
                serde_json::json!({ "target_file": rel_str }),
                "call-0m-s3-read",
            )
            .await
            .expect("terminal window read must succeed");
        assert!(
            read_back.output.contains("xxxx"),
            "read-back must return the logged content"
        );

        // 3) 卷内非窗口面：0p S2 两段门（ADR-0010 §14.61 设计 B，取代
        //    0m 的 agent-invisible 恒拒）——内部区首读返回通知信封
        //    （非内容 + 职责图/结构预览/黑板指针/询问句），access_state
        //    落盘；二读放行（open_after_notice）。
        let denied = host
            .call_tool(
                "read_file",
                serde_json::json!({ "target_file": ".gsa/secret.txt" }),
                "call-0m-s3-deny",
            )
            .await
            .expect("deny path surfaces as a result envelope");
        assert!(
            denied.output.contains("[session_volume_notice]"),
            "non-window volume interior first read must return the notice envelope: {}",
            denied.output
        );
        // D-3 闭合（设计 C）：拒绝信封结构化——policy_denial{source=
        // permission, code=session_volume_notice} + exit_code=1（journal
        // 面由 host_exec 落 status=error + policy_denial，见 orz-loop 测试）。
        let denial = denied
            .policy_denial
            .as_ref()
            .expect("structured denial envelope");
        assert_eq!(denial.source.as_str(), "permission");
        assert_eq!(denial.code, "session_volume_notice");
        assert_eq!(denied.exit_code, Some(1));
        let opened = host
            .call_tool(
                "read_file",
                serde_json::json!({ "target_file": ".gsa/secret.txt" }),
                "call-0m-s3-open",
            )
            .await
            .expect("second read must open");
        assert!(
            !opened.output.contains("[session_volume_notice]"),
            "second read must be opened (open_after_notice): {}",
            opened.output
        );
        assert!(
            opened.session_volume_opened,
            "post-notice open must set the audit marker"
        );

        // 4) run_tests 输出窗口：全量输出 `run_tests_output.txt` 落卷且
        //    窗口可读（TestRunner 注入，避免依赖真实测试框架）。
        let runner = orz_loop::host::TestRunner {
            command: if cfg!(windows) {
                vec![
                    "cmd".to_string(),
                    "/c".to_string(),
                    "echo rt-window".to_string(),
                ]
            } else {
                vec!["echo".to_string(), "rt-window".to_string()]
            },
            timeout: Some(std::time::Duration::from_secs(30)),
            env: Vec::new(),
        };
        let host = host.with_test_runner(Some(runner));
        let test_run = host.run_tests().await.expect("run_tests returns");
        let full_path = test_run
            .full_output_path
            .expect("run_tests full output must persist");
        let full = std::path::PathBuf::from(&full_path);
        let rel_tests = full
            .strip_prefix(&workspace)
            .expect("run_tests output stays under cwd");
        assert_eq!(
            rel_tests.to_string_lossy().replace('\\', "/"),
            ".gsa/run_tests_output.txt",
            "window 2 fixed file name"
        );
        assert!(
            volume.join("run_tests_output.txt").exists(),
            "run_tests output must land inside the volume through the link"
        );
        let read_tests = host
            .call_tool(
                "read_file",
                serde_json::json!({ "target_file": ".gsa/run_tests_output.txt" }),
                "call-0m-s3-rt",
            )
            .await
            .expect("run_tests window read must succeed");
        assert!(
            read_tests.output.contains("rt-window"),
            "run_tests window read-back must return the logged content: {}",
            read_tests.output
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 0p S2 复审 P1-1 修复回归（2026-09-07，ADR-0010 §14.61 设计 B）：
    /// 带桥 host（生产装配形态）下，`.gsa` 内部区读由桥放行、落入工具层
    /// 两段门——通知信封（policy_denial code=session_volume_notice）与
    /// 二读放行（session_volume_opened）在**穿透桥的全链**上可达。修复前
    /// 桥镜像先拒，两段门在带桥路径不可达（W2 D-3 原形）。
    #[tokio::test]
    async fn bridge_yields_internal_reads_and_envelope_lands() {
        let dir = test_dir();
        let gsa = dir.join(".gsa");
        std::fs::create_dir_all(gsa.join("ledger")).unwrap();
        std::fs::write(gsa.join("ledger").join("current.md"), "[1] row\n").unwrap();
        let bridge = crate::permission::bridge_allow_all_for_test(&dir);
        let host = OrzHost::with_permission(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
            Some(bridge),
        )
        .expect("host with bridge");

        // ① 桥面：内部区读不再被镜像预拒（AllowOnce）。
        let decision = host
            .request_permission(
                RiskClass::ReadOnly,
                "read_file",
                &serde_json::json!({"target_file": ".gsa/ledger/current.md"}),
            )
            .await
            .expect("permission request resolves");
        assert_eq!(
            decision,
            PermitDecision::AllowOnce,
            "bridge must yield internal-region reads to the tool-layer gate"
        );

        // ② 工具层两段门：首读通知信封 + 结构化 policy_denial（exit 1）。
        let first = host
            .call_tool(
                "read_file",
                serde_json::json!({"target_file": ".gsa/ledger/current.md"}),
                "call-bridge-1",
            )
            .await
            .expect("first read returns");
        assert!(
            first.output.contains("[session_volume_notice]"),
            "notice envelope expected: {}",
            first.output
        );
        let denial = first
            .policy_denial
            .as_ref()
            .expect("structured denial envelope on the bridged path");
        assert_eq!(denial.code, "session_volume_notice");
        assert_eq!(first.exit_code, Some(1));

        // ③ 二读放行 + session_volume_opened 审计标记。
        let second = host
            .call_tool(
                "read_file",
                serde_json::json!({"target_file": ".gsa/ledger/current.md"}),
                "call-bridge-2",
            )
            .await
            .expect("second read must open");
        assert!(
            second.output.contains("[1] row"),
            "content served after notice: {}",
            second.output
        );
        assert!(
            second.session_volume_opened,
            "post-notice open must set the audit marker"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0p S2 复审 P2 修复回归（旗标按 call-id 键控）：并发批下两个调用
    /// 各自取走各自的 `session_volume_opened` 旗标——共享单旗标形态下后
    /// take 的一方会拿到 false（信封丢失）。
    #[tokio::test]
    async fn session_volume_flags_attribution_per_call_under_concurrency() {
        let dir = test_dir();
        let gsa = dir.join(".gsa");
        std::fs::create_dir_all(&gsa).unwrap();
        // 预置已通知态：两读都走「通知后放行」臂。
        std::fs::write(
            gsa.join("access_state.json"),
            "{\"schema_version\":\"0.1.0-draft\",\"notice_shown\":true,\
             \"notice_shown_at_unix\":0}",
        )
        .unwrap();
        std::fs::write(gsa.join("a.md"), "aaa\n").unwrap();
        std::fs::write(gsa.join("b.md"), "bbb\n").unwrap();
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host");
        let (ra, rb) = tokio::join!(
            host.call_tool(
                "read_file",
                serde_json::json!({"target_file": ".gsa/a.md"}),
                "call-conc-a"
            ),
            host.call_tool(
                "read_file",
                serde_json::json!({"target_file": ".gsa/b.md"}),
                "call-conc-b"
            )
        );
        let ra = ra.expect("call a");
        let rb = rb.expect("call b");
        assert!(
            ra.session_volume_opened,
            "call A must carry its own opened flag: {}",
            ra.output
        );
        assert!(
            rb.session_volume_opened,
            "call B must carry its own opened flag: {}",
            rb.output
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0p S2 复审 P1-2 修复回归（B5 第 5 漏斗）：run_tests 全量输出落
    /// `run_tests_output.txt`（恒直读窗口）前接 orz-secrets 脱敏——
    /// 测试进程打印的 key 形态串以占位符落卷（判据「全卷零 sk-」）。
    #[tokio::test]
    async fn run_tests_output_scrubbed_of_secrets() {
        let dir = test_dir();
        let secret = "sk-abcdefghijklmnopqrstuvwxyz012345";
        let runner = orz_loop::host::TestRunner {
            command: if cfg!(windows) {
                vec![
                    "python".to_string(),
                    "-c".to_string(),
                    format!("print('{secret}')"),
                ]
            } else {
                vec![
                    "python3".to_string(),
                    "-c".to_string(),
                    format!("print('{secret}')"),
                ]
            },
            timeout: Some(std::time::Duration::from_secs(30)),
            env: Vec::new(),
        };
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_test_runner(Some(runner));
        let result = host.run_tests().await.expect("run_tests returns");
        assert!(
            result.output.contains("[REDACTED_SECRET]"),
            "conversation tail must be scrubbed too: {}",
            result.output
        );
        let full = std::fs::read_to_string(dir.join(".gsa").join("run_tests_output.txt"))
            .expect("full output persisted");
        assert!(
            !full.contains(secret),
            "raw key must never land in the volume: {full}"
        );
        assert!(
            full.contains("[REDACTED_SECRET]"),
            "placeholder expected in the volume artifact: {full}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
