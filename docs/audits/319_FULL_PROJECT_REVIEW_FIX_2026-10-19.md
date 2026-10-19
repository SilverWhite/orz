# 319 批：318 全项目审查修复批（2026-10-19）

> **性质**：落码修复批。承接 [`318 审查`](318_FULL_PROJECT_REVIEW_PHASE1_2026-10-19.md) 的 P0／P1。
> **范围**：父仓 `D:\CLI` 与 orz 子仓；修复会话生命周期、宿主灾难保底、维护 CLI、0di 契约、
> Web 状态机、CI／合约门禁、README／SECURITY／索引／账本。
> **边界**：未推送远端；未重建 0.9.4 载体；未执行 0dh／0di S4；Linux 原生构建与
> `orz-bin` 运行时测试仍未进 CI；Phase 2 深审未开。
> **计数**：60 → 62（新增 `0dk`／`0dl` 两条 P1，均 `pending`）。

---

## §1 P0 闭环

| # | 318 发现 | 319 处置 | 证据 |
|---|---|---|---|
| P0-1 | 已推送 main CI 5/5 红；0.9.3 在红管线发行；Python 合约 69≠67；本地门禁非封闭 | ① `orz-loop` 两例固定 sleep 竞态改为**显式 host-entered 信号**（permission gate／首个 tool in-flight），并行 892/0，CI 恢复并行；② Python 事件集合/数量由 registry 派生（69），supplement `character_count` 13，生成器先去 rmtree、补两事件、表漂移零写入；③ `check_repository` 链接以 `git ls-files` tracked 集合为唯一真值，证据路径分类 warning；④ CI 增加 `if: always()`、`--no-fail-fast`、`strategy.fail-fast:false`、`orz-web`／`orz-aux` job、protoc sha256、release 前置门 | `cargo test -p orz-loop --lib` 892/0/3（并行）；`python runtime/tests/test_run_event_conformance.py` 17/17；`scripts/check_repository.py --self-test-links` 12/12；CI 5 jobs |
| P0-2 | `rm -rf .`／`/usr/*`／`find -delete`／Windows 通配递归删除基本不设防；`.gsa` 祖先链缺失 | 规则 1 重写为分段扫描：cwd 本体/祖先、卷根/系统根、受护根通配、`find -delete`／`-exec rm`、`xargs rm -rf`、`cd .gsa/.git && rm -rf *`、`cmd /c`／`bash -c` 内容位；规则 5 增 `carrier:session-volume-ancestor`。精确子目录与明确模式仍放行 | `exec_policy` 45/0；`write_control` 17/0；新增负向钉覆盖 `rm -rf .`、`rm -rf /usr/*`、`cd .gsa && rm -rf *`、`find /usr -delete`、`Remove-Item C:\Windows\* -Recurse` |
| P0-3 | 模型可经 `run_terminal_cmd` 调用 `orz delete-session`／`archive`／`unarchive`／`trust`／`rollback restore` | 双门：命令面 `carrier-write` block（含 `ORZ_MAINTENANCE_TICKET` 引用、内容位递归）；`orz-bin` 各维护入口要求桥注入 `ORZ_MAINTENANCE_TICKET`（≥16，缺失 exit 2），Web 桥每次 spawn 注入随机值 | `exec_policy` 维护 CLI 负向钉；`cargo test -p orz-bin --bin orz -- maintenance_ticket` 1/0 |
| P0-4 | `session/load` 复用 `session/new`，普通点击触发 0cy 全工作区收卷 | 拆出 `attach_session_with_options(..., sweep_enabled)`；`session/load` 走 `false`，并校验绝对/canonical cwd、同进程绑定；新增回归钉：load B 后 A 的 sidecar/run 目录原样 | `acp_server::tests::session_load_does_not_sweep_sibling_sessions` 通过 |

## §2 P1 闭环

### A. 会话/恢复面

| ID | 处置 |
|---|---|
| A1 | `session/load` 按 ACP v1 经 `session/update` 回放 user/assistant 历史（工具/系统消息不回放）；stdio 保留 `load_session(true)`，`LoadSessionResponse` 前完成回放 |
| A2 | 新增 `PromptInflightGuard`（RAII）：bootstrap／build_host／session 消失等所有早退路径释放 marker；正常路径显式 remove 后 `disarm()` |
| A3 | `prompt_count` 无条件从 `RUN-<s8>-<n>` 扫描；`.gsa/runs` 不可读 fail-closed，不再以 sidecar 存在为门 |
| A4 | `restore_count` 从 `RST-<s8>-<n>` 恢复，载入旧会话不再追加第二条 RST 链 |
| A5 | codex 线程 id 改为 8 位小写十六进制（不再 `thr-`＋4 hex）；`run_dir_session` 改 `rsplit_once('-')`，`RUN-thr-1a2b-0` 归 `thr-1a2b` 而非伪造 `thr` |
| A6 | `conversation_session_id` 改 `from_utf8_lossy`，64KiB 截断落在多字节字符中间仍可解析 |
| A7 | 新增 `{cwd}/.gsa/sessions/<session_id>.lease` OS 排他锁（`fs2`）：同会话第二进程 fail-closed，进程死亡自动释放；0cy sweep 将跨进程 lease 视为 live，不再 unlink 活会话 journal |

### B. Web/TUI 状态机

| ID | 处置 |
|---|---|
| B1 | `/rewind` 与 submit 共用只读/跨会话门；只读/归档/`run://` 视图直接拒绝；显式传绑定 session_id；拒绝时保留草稿；`conversation://` 的 s8 转义 |
| B2 | `openConversation` 默认只读、零 `sessionLoad`、零绑定漂移；新增显式「恢复并继续」唯一 load 入口；load 成功后先清空投影再回 live，避免历史回放重复渲染 |
| B3 | 新增 `sessionLoadToken`／`navEpoch`／`pendingLoadSessionId`：await 后复检 token、running、目标会话；加载期禁发 prompt/切会话/归档/删除；stopRun 用 `currentRunSessionId` |
| B4 | `projection.js` 不再写运行态；`vm.setRunning()` 单一 owner；停止键/Ctrl+Z 同源；回放无终态标「历史未完成」；服务端 busy 触发清单重同步 |
| B5 | 实时尾匹配 `RUN-<s8>-` 或精确 `RUN-CLI-<s8>`，排除 ARC/RST；smoke 改行为断言（含 RUN-CLI 非空） |
| B6 | `journal_tail` 累计 pending buffer、`offset += n`、只发完整行；>256KiB `supplement_delivered` 单行测试通过 |
| B7 | `beforeunload` 在 running/排队补充/后台任务存在时提示；UI 明示「仅文本上下文恢复，后台任务/队列不恢复」；删除已加载会话/newSession 解绑并停 tail |

### C. 0di 契约

| ID | 处置 |
|---|---|
| C1 | 会话侧车改 temp＋fsync＋rename 原子替换；旧侧车在失败前始终保留 |
| C2 | 新增 `INJECTED_BLOCK_PREFIXES` 单一表，补 `[工作台]`、`[上轮检索未派发]`、`[结果投递]`、console 询问/重填、`[任务状态`、检索收尾等；新增 `is_generated_injected_block` 结构化判据；host `/rewind` 与 retrieval activation 已迁到结构化判据 |
| C3 | drain 前按真实模型面估算余量；无余量不 drain；有余量 FIFO 只收不越线前缀，越线进 run 级延迟队列；接纳后复验回退；`GateDecision`＋run 尾 `defer_run_end` 不静默蒸发 |
| C4 | `recall_preview` 只在全局 runs 锁内做 idle 判定，clone/读盘移到锁外 |
| C5 | 写回按「真实用户来源精确同文豁免＋结构化注入形态」过滤；用户原文以 `[工作台]` 等前缀开头不再被丢弃；supplement 来源同等豁免 |

### D. 文档/账本/门禁

- README／英文 README：检索默认开、12 工具面（检索门下可见 11）、TUI 不恢复、
  ACP/Web `session/load` 的 lease/回放/队列边界、RLI 提醒直投；写控补 319 加固与残余绕过。
- `orz/SECURITY.md`：按 0cc v3＋319 真实边界重写；只硬拒 `.gsa`（含祖先）＋keystore/manifest；
  Landlock 已实施；报告渠道改本仓；TTY 双保险等余项如实登记。
- `CLI_PROJECT_INDEX.md`：`FUS-BUDGET` 改 `withdrawn`（默认 unlimited，显式 >0 逃生阀）；
  新增 318（reference/reviewed）与 319（pending）指针。
- 账本新增 `0dk`（会话附着/恢复/关闭生命周期）、`0dl`（安全保底与维护 CLI 加固），均 `pending`，
  计数 **60 → 62**。

## §3 亲验读数

```text
orz-host 串行            389 passed / 0 failed / 6 ignored
orz-loop 并行            892 passed / 0 failed / 3 ignored
orz-loop 串行            892 passed / 0 failed / 3 ignored
orz-web                  51 passed / 0 failed
frontend smoke          14.1–14.9 全部通过
exec_policy              45 passed / 0 failed
write_control            17 passed / 0 failed
orz-bin ticket           1 passed / 0 failed
Python conformance       17 tests OK
Python journal validation 273 tests OK
generator                二次运行逐字节幂等
check_repository         318 档 tracked 模拟后仅 1 条预期 dirty；链接硬错 0
CI YAML                  PyYAML 结构校验通过（5 jobs；release main-only/needs 四 job）
```

## §4 残余风险与未闭环

1. **远端仍未恢复绿**：本轮未 push；GitHub main 仍是旧红。必须提交并 push 后观察新 CI 五 job；
   新增的 `orz-web`／`orz-aux` 首次运行仍可能暴露环境问题。
2. **未重建 0.9.4 载体**：`orz_source_manifest.sha256` 需在 orz 提交后重算；0.9.3 仍不含本批补丁，
   不能以 0.9.3 开 0dh/0di S4。
3. **安全是阻力不是沙箱**：词法旁路仍存（未展开变量、`sudo -u`、`busybox`、python/perl 直调、
   symlink/junction/8.3、Linux 宿主上的 `C:\` 字面量、`.git` 直接删除、非删除动词写入等）；
   维护令牌可被同 UID 进程在 spawn 窗口读取 environ；TTY 双保险未落。已登记 0dl。
4. **0dk 余项**：补充队列未持久化、刷新/断线后后台任务不恢复、ACP 0.10.4 无 `session/close`、
   sessions map 无界；本轮只做 lease、历史回放、RAII、ordinal 恢复。
5. **C3 留队只在 run 内**：`LoopHost` 无 requeue/limited-drain，run 尾仍未投出的补充只能落事件，
   不能跨 run 保留；真正留队需 host 新接口。
6. **CI 覆盖仍偏**：orz-bin 无 lib target，其 bin/ACAF 运行时测试未进 CI；Linux Rust 构建/测试仍缺；
   17 个不可达 crate 未变。
7. **账本仍五本**：`BACKLOG_2`／`TODO2` 未归档，112 篇 audit 可达性治理未做；本轮只登记与同步三本主账。
8. **P2/P3 未清**：Web help/命令表未加 `/rewind`、撤回预览 `queue_depth` 未接 UI、会话 load 的
   sessions map 无界、`#` 级代码健康与不可达 crate 未动。

## §5 合并顺序（建议）

1. orz 子仓提交本批（24 文件＋`Cargo.lock`，commit `41534aee`，分支 `feat/fusion-architecture`）；
   父仓本批提交已含 submodule pin、318／319 两档、文档与 manifest 1488 条。
2. `python scripts/generate_orz_source_manifest.py` 已重算；`check_repository.py` 实测 `valid:true`／
   `error_count:0`。
3. **未推送**：父仓 main 相对 `origin/main` 为 11 ahead / 4 behind（远端 4 个 README 直改提交），
   orz `feat/fusion-architecture` 为 1 ahead；推送前需先合并／rebase 远端 4 提交，并先推 orz
   分支（否则父仓 pin 在 CI checkout 时不可达）。
4. push 后观察 CI 五 job；红则先修新 job 环境问题，不要先重建载体。
5. 全绿后重建 0.9.4（双平台）→ 更新载体哈希／适配器锁定值 → 再开 0dh 6 项＋0di 9 项 S4。
6. Phase 2 深审（权限/ACAF/session volume、上下文压缩、检索默认开、评测计量、UI/UX、代码健康、
   账本收敛）另批。

---

**结论**：318 的 P0 已全部处置，P1 的 A/B/C 主面与 D 文档门禁已落；0di 仍保持 `pending`
（S4 未跑、载体未重建），0dj 维持 306 闭合结论不重开。当前可提交、可推送、可开新 CI，
但**不可用 0.9.3 开 S4，也不可在 CI 未绿前发行 0.9.4**。
