# scripts/ 生命周期登记（LIFECYCLE）

> 设立：2026-09-13（全项目深审 S-14 处置：一次性探针与核心门禁同目录、无生命周期标记）。
> 本登记是 `scripts/` 的唯一生命周期权威；按**家族**归类，不逐文件罗列。
> 状态机：`active`（常设门禁/生成器/复现入口，随门禁演化）→ `retained`（批次未闭合的
> 常设工具件，批次闭合时转 consumed 或归档）→ `consumed`（一次性探针/诊断，随批闭合，
> **保留在位**供案例与审计引用追溯，不再演化；新批次不得复用改名，须另立新件并在本登记落行）。
> 彻底退出追溯价值的 consumed 家族整批移入 `存档/root-artifacts-*/`，同一变更中修正引用路径。

## active（常设件）

| 家族 / 文件 | 用途 | 备注 |
|---|---|---|
| `check_repository.py` | 仓库门禁（链接/结构/manifest 核验） | 核心门禁 |
| `generate_orz_source_manifest.py` | orz 源 manifest 重算 | 门禁配套 |
| `check_credential_isolation.py` | 凭据隔离检查 | 门禁配套 |
| `check_tool_availability.py` | 工具可用性检查 | 门禁配套 |
| `build_global_progress_checkpoint.py` | 全局进度 checkpoint 构建 | 门禁配套 |
| `generate_run_event_fixtures.py` | run-event fixture 生成器 | schema 配套 |
| `run_r3_unsolved20_per_task.ps1` / `run_r4_unsolved15_per_task.py` / `run_r4b_supplement4_per_task.py` | 官方复跑执行器（0s / 0u / r4b） | 结果复现入口 |
| `run_tb40_ctr_probe.py` | TB 4.0 单题摩擦探针执行器（0w） | 结果复现入口 |
| `rerun_failures_k1.ps1` / `run_failures_chunked_10.*` / `run_official_chunked_10.sh` | 官方批次补跑/分块执行器 | 结果复现入口 |
| `orz_acaf_run.ps1` | ACAF harbor 透传跑批入口 | `GAP-ACAF-HARNESS-PASSTHROUGH` 登记件 |
| `run_0x_0v_s4_live_verify.py` / `analyze_0x_0v_s4_journal.py` / `run_0vb_probe.py` | 0x/0v S4 实机复验执行器与分析器 | S4 记录引用 |
| `setup_harbor_proxy.ps1` / `setup_chrome_cdp.py` | 评测装置供给 | 装置复用件 |
| `build_orz_aarch64_musl_cross.sh` | aarch64 musl 三件套交叉编译入口（x86 容器 + zig cc + rust-lld） | 0y 安卓载体复现入口（2026-09-13 设立） |
| `s4_vm_checkpoint_slim.ps1` | win-s4 测试 VM 的 checkpoint 合并 + VHDX 压实（管理员执行；数据不保留的收敛清理） | 0l 环境维护一次性件（2026-09-13 设立） |
| `ops_bridge.ps1` / `ops_bridge.sh` / `ops_executor.ps1` + `ops-bridges.json` | OPS-PROTOCOL 参考执行器 | `reference` 面（不生产接线） |

## retained（批次开放中的常设工具件）

| 家族 | 用途 | 转出条件 |
|---|---|---|
| `s4_vm_create/start/shutdown/cleanup/inspect/check_setup/sync_orz/sync_assurance/batch_right/prep*.ps1` 等 VM 生命周期件 | 0l 加固 VM 常设操作件 | 0l ⑥⑦ 闭合时随批裁决转 consumed 或归档 |

## consumed（一次性探针 / 诊断，保留在位）

| 家族 | 批次 | 数量 | 备注 |
|---|---|---|---|
| `s4_vm_repro_*` / `s4_vm_probe_*` / `s4_vm_diag_*` / `s4_vm_fix_*` / `s4_vm_cred_*` / `s4_vm_harden*` / `s4_vm_elev*` / `s4_vm_wf12_probe` / `s4_vm_uac/icacls_test/reg_acl_test/applocker_reset` | 0l S1–S4 加固与 enforcement-probe | ~70 | 案例 `ORZ-WIN-SBX-001/002/003`、`ORZ-WIN-CTYPES-001` 等在位引用，勿移动 |
| `s4_admin_bridge*` / `s4_bridge_request` / `s4_check_tpm` / `s4_enable_tpm` / `s4_copy_to_vm` / `s4_host_download` / `s4_hv_diag` / `s4_pack_task_inputs` / `s4_run_elevated` / `s4_elev_*` / `s4_assurance_stub_init` | 0l S1–S4 装置与桥接 | ~13 | 同上 |
| `_s4br_probe5.ps1` / `__wtest.ps1` / `__wtest2.ps1` | 脚手架试跑 | 3 | 无案例引用；退出追溯价值时随批归档 |
| `invoke_grok_*` / `verify_grok_*` / `run_grok_*` / `capture_grok_*` / `record_grok_*` / `new_grok_*` / `manage_grok_probe_firewall` / `inspect_grok_install` / `child_tree_fixture` | 冻结期 Grok harness 血统探针（0.1.x–0.2.x 证据面） | ~25 | 部分被 `verify_grok_windows_child_tree_probe` 等审计引用；保留在位 |
| `capture_codex_*` / `verify_codex_*` / `normalize_codex_*` / `probe_codex_*` | Codex app-server 对照探针 | ~5 | FUS-UI-BOUNDARY 证据面 |
| `build_/append_/verify_/reduce_/run_ global_progress*`（checkpoint 除外） | 全局进度机制历史探针 | ~9 | 机制已收敛，门禁只走 checkpoint 件 |
| `verify_cli_session_lifecycle_receipt` / `append_cli_session_lifecycle_event` | 会话生命周期事件探针 | 2 | GAP-CONVERSATION-RESTORE 证据面 |
| `recover_torn_journal` / `recover_archive_journal` | journal 修复工具 | 2 | 0v-C / 撕裂修复批使用的恢复件；保留复用可能，批次闭合后随批裁决 |
| `test_web_retrieval.py` / `test_browser_retrieval_e2e.py` / `test_real_api_call.py` / `probe_cache_hit_template.py` / `deepseek_thinking_proxy.py` / `fake_deepseek_provider.py` / `run_grok_timeout_triage_matrix.ps1` / `gate_google_observe.ps1` / `verify_grok_timeout_gate_split.py` | 检索/缓存/传输期诊断 | 9 | 0i/0k/0v 期证据面 |
| `build_orz_aliyun_trixie.sh` / `test_orz_aliyun_trixie.sh` / `s3_smoke_20260829_s5-2.sh` / `run_windows_native_sandbox_{command,probe}.py` / `run_grok_0_2_112_live_gates.ps1` / `run_grok_acp_fake_tool_client.py` / `run_grok_compaction_provenance_probe.py` / `run_grok_windows_child_tree_probe.py` | 环境/沙箱/ACP 期一次性件 | 10 | P2-SANDBOX / GAP-ACAF 证据面 |

## 规则

1. 新增一次性脚本**必须**在落地同一变更中于本登记落行（家族 + 批次 + 状态），否则按 `unclassified` 对待、不得进门禁执行面。
2. `consumed` 家族默认保留在位（案例/审计引用追溯）；整批归档进 `存档/root-artifacts-*/` 时，同一变更修正全部行文引用。
3. 本登记由人工维护；2026-09-13 后续批可按深审 §3 建议把「未登记文件检查」并入 `check_repository.py`（见 BACKLOG 0ab）。
