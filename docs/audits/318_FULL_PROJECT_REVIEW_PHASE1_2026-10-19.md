# 318 批：全项目阶段性深度审查（Phase 1）——会话生命周期／0di 现场修复＋发布门禁与安全真值（2026-10-19）

> **性质**：只读审查批。**零源码改动、零账本计数改动、零载体重建**。
> **被审对象**：父仓 main `215245d9`（本地）；orz 子树 HEAD `5fd4c42d`（0.9.3 源冻结）；
> **7 个未提交文件**（`acp_server.rs`／`stdio.rs`／`orz-web` 四件／`runs.rs`／`frontend_smoke.mjs`，
> +251/−27）——即用户真机遇到 0di 问题后的现场修复，下文称「现场补丁」。
> **方法**：索引路由回查 → ADR-0010／设计稿／304／305／306／308／0cy／0dd／0dh 文档 →
> 源码逐条亲验 → `git`／`gh` 远端与 CI 读数 → 本地定向测试 → 六路子代理并行只读深查
> （host／loop·journal／web·TUI／设计符合性／安全／测试·CI·仓库健康）。
> **边界**：未覆盖 ADR-0010 全部章节与全部 50 个 workspace member；未跑全量构建；
> 未执行 S4 真机判据；Phase 2 清单见 §8。
> **一句话**：设计体系仍然强，305 的 0dj 修复也真实；但**发布门禁已失效、宿主灾难保底可被常见命令绕过、
> 会话恢复现场补丁引入 0cy 收卷副作用与跨进程写冲突**——当前不能闭合 0di，也不能对 0.9.3 直接开 S4。

---

## §1 总体裁决

| 面 | 裁决 | 要点 |
|---|---|---|
| 0di 设计本体 | **基本成立** | 会话级队列、loop-top flush、SUCCESS-ONLY、ordinal 截断、不回滚面、idle 门、单进程互斥与 D1–D8 基本一致；305 R1–R4/R6/R8/R17 等修复经代码与测试复核真实 |
| 0di 现场补丁 | **不可原样合入** | 修对了 `state.running` 被历史回放污染的主因，但夹带未设计的 `session/load`，引入/暴露 1 个 P0＋多个 P1；没有设计稿、审查档、ADR、索引、账本记录 |
| 项目级发布门禁 | **P0 失效** | 已推送 main 的 CI 5/5 job 全红；v0.9.3 在红管线上发行；本地门禁只有 dirty 单错、干净检出 51 条断链；Python 合约轨 69≠67 实际红；远端/本地已分叉 |
| 安全声明 | **P0/P1 不实** | 宿主灾难保底对 `rm -rf .`、`rm -rf /usr/*`、`find -delete`、Windows 通配删除基本不设防；`orz delete-session` 等维护 CLI 可被模型经 `run_terminal_cmd` 直接调用；`orz/SECURITY.md` 仍宣称系统核心硬拒 |
| 测试与账本 | **P1 结构性问题** | CI 实际只执行 orz-loop 单包且 2 例并行红；web/tui/bin/Linux/集成测试不在 CI；17 个 crate 从发行二进制不可达（1,830 个测试永不跑）；5 本账本、12 处状态矛盾、112/458 篇 audit 不可达 |
| S4 | **不具备条件** | 0.9.3 载体冻结 `5fd4c42d`，**不包含现场补丁**；0dh/0di 的 S4 判据未执行；对 0.9.3 跑 S4 只会验证旧行为 |

**结论**：0di 保持 `pending`，0dj 维持 306 的闭合结论（不开新回退）；现场补丁应拆成「0di S4 前置修复」与
新的「会话恢复」缺口（建议 `0dk`）分别裁决；安全绕过与发布门禁应各自单独立项（建议 `0dl`／0aq 追加）。
**在 P0 修复、CI 恢复全绿、含补丁载体重建之前，不应再切任何发行，也不应开 S4。**

---

## §2 P0（阻断）

### P0-1 已推送 main 的 CI 5/5 全红，v0.9.3 在红管线上发行

**证据**
- `gh run list -R SilverWhite/orz`：最近 20 次 CI 中仅 1 次成功（`5fd4c42d`，疑似子分支运行）；
  main 自 `54a4fcb9`（2026-10-18 15:21Z，307 推送）起连续 failure；最新 `39242209300`（`004f8b72`）
  与 v0.9.3 发行提交 `39197221568`（`9b9d5b98`）均 **5/5 job failure**。
- 失败面：
  1. **Rust 轨**：`orz-loop` `884 passed; 2 failed; 3 ignored`——
     `cancel_during_permission_await_resolves_then_terminates`
     (`orz/crates/orz-loop/src/controller.rs:6691`) 与
     `cancel_mid_multi_tool_round_skips_unstarted_tools` (`:6415`)；CI 的 loop 步没有
     `--test-threads=1`，而账本读数一律「loop 串行 886/0」——本地串行与 CI 并行不可复现。
  2. **validate 轨**：Linux 两个 job 各 `error_count: 51`，均为 docs 指向
     **gitignored `.tmp-*`** 或仓外 `tb-eval`／`存档/evidence/*.pdb` 的断链；本地
     `check_repository.py` 只有 `"orz submodule working tree is dirty"` 一条，因为 740 个
     `.tmp-*` 在本机真实存在（全部命中 `git check-ignore`）。
  3. **Python 合约轨**：`runtime/tests/test_run_event_conformance.py:359` 硬编码
     `len(enum_events)==67`，实际 **69**（0df `run_usage_metered`＋0di
     `supplement_delivered` 两次新增未校准）；实跑 `Ran 16 tests ... FAILED (failures=1)`。
- **有效覆盖**：本次 CI 实际只执行 orz-loop 单包（889 例，约全仓 `#[test]` 的 6%–9%）；
  orz-assurance／orz-host／Python 四 job 的测试步被 fail-fast 跳过；web/tui/bin/Linux 不在 CI。
- **发布后果**：v0.9.3 三资产已在 GitHub Release 发布，但没有任何 CI 绿证据；RS-01 的修复只恢复了
  workflow 存在，没有恢复「绿才可发」。

**影响**：发布门禁事实不存在；后续回归会被同一片红噪声吞掉；用户拿到的 0.9.3 未经过完整测试轨。

**建议**
1. 修 CI 红：loop 串行或把两例改成确定性同步；`check_repository` 按 `git ls-files` 解析链接
   （不在 tracked 集合即 broken）；修 51 条断链；合约钉值从 schema/registry 派生，不硬编码。
2. release job 加 `needs: [validate, rust-tests]` 与 main 分支保护；测试步 `if: always()`／
   `--no-fail-fast`，避免一步失败吞掉全部测试。
3. 补 `orz-web`（含 node smoke）、`orz-tui`、`orz-bin`、Linux 构建 job；先恢复「全绿才发行」。

### P0-2 宿主灾难保底可被常见命令绕过

**证据**（`orz/crates/codegen/orz-tools/src/types/exec_policy.rs`／`write_control.rs`）
- 规则 1 只在「删除动词＋递归旗＋目标**恰等于**卷根／根本树根」时 block
  （`exec_policy.rs:646-691`；`write_control.rs:322-324`）。因此：
  - `rm -rf /usr/*`、`rm -rf /*`、`Remove-Item C:\Windows\* -Recurse` 只落
    `broad-destructive` **warn**，然后继续执行；
  - `rm -rf .`：目标 `.` 不是树根，`is_rootish(".")` 为假，`path_forms` 落在 cwd 内，
    **连 warn 都不触发**，会直接删掉整个工作区（含 `.git` 与 `.gsa`）；
  - `cd .gsa && rm -rf *`、`rm -rf .*`：目标为 `*`／`.*`，只 warn，执行后清空 `.gsa`；
  - `find /usr -delete`／`find . -delete`：`find` 不在 `DELETE_VERBS`，规则 1 不扫，
    不 warn、不 block。
- 规则 5 的 `.gsa` 祖先链臂只覆盖 keystore/manifest
  （`write_control.rs:179-201`）；`.gsa` 本体的祖先扫荡没有实现，与设计稿／英文文档的表述不符。

**影响**：README 把写控定位为「宿主机灾难保底／防扬盘级不可逆毁灭」；上述命令是模型日常最可能
误触的灾难形态，却基本不设防。`rm -rf .` 可直接删除工作区与全部会话证据。

**建议**：在规则 1/5 增加「目标为受护根的祖先，或含 `*`／`?`／`.`／`..` 的破坏形态」block；
覆盖 `find -delete`／`xargs`／`sed -i` 等写向旁路；补负向钉子（`rm -rf .`、`rm -rf /usr/*`、
`cd .gsa && rm -rf *`、`find /usr -delete`、`Remove-Item C:\Windows\* -Recurse` 必须 block）。
若产品坚持「精确子目录删除放行」，必须把 `.`／通配／工作区根列入显式例外并如实改口径。

### P0-3 模型可经 `run_terminal_cmd` 调用未鉴权的维护 CLI

**证据**
- `orz/crates/orz-bin/src/main.rs:306-316` 直接分派 `orz file archive|unarchive|delete-session`；
  `:702-705` 注释明写「此处不做二次确认（桥是唯一调用方）」——该假设不成立。
- `delete_session_on_demand` (`orz/crates/orz-host/src/acp_server.rs:2083`) 只做 8-hex 校验，
  删除侧车、`RUN-*`／`RST-*`／`ARC-*` journal、orientation/activation/grill/compaction/blocks；
  无 ACAF 票据、无 TTY 确认、无桥令牌。
- `review_command` 只按删除动词/路径判定（`exec_policy.rs:805-844`）；`orz` 不在
  `DELETE_VERBS`，`delete-session` 只是参数，故静默 `Allow`；模型可先 `ls .gsa/conversations`
  （shell 读面不设 `.gsa` 窗口门）取得 s8，再执行删除。
- 同族可达：`orz trust`（改信任基线）、`orz rollback restore`（文件回退）、
  `orz archive`／`unarchive`（会话归档面）。

**影响**：模型可绕开 UI 的两段确认，删除任意会话的侧车与证据链；`orz trust` 可绕开工作区信任门；
`rollback restore` 可在无审批下回退用户文件。这直接否定「桥是唯一调用方」与「删除有前端确认」的安全声明。

**建议**：维护子命令要求桥注入一次性能力令牌（env/fd，模型不可读）或 TTY 确认；命令面把
`orz` 维护动词列入 block；`.gsa` 的 shell 读面纳入会话卷窗口门。补「模型路径不可达」负向钉。

### P0-4 `session/load` 复用 `session/new`，普通 UI 点击触发 0cy 全工作区收卷

**证据**
- `handle_session_load_with_options` (`orz/crates/orz-host/src/acp_server.rs:3270-3304`) 在确认侧车存在后
  直接调用 `handle_session_new_with_options`（`:3292`）；后者 `:3177-3201` 无条件
  `sweep_session_leftovers_with(keep=本次 s8, live=本进程 sessions)`。
- web `openConversation` 在普通点击时自动 `acp.sessionLoad`（`orz-web/assets/app/main.js:326-334`）；
  `widgets.js:279-283` 单击会话即打开。
- sweep 对 keep/live 之外的每个 footprint 调 `rollup_session`，打包成功后
  `delete_session_sources` (`:1597-1651`) 删除侧车与 run 目录；`hygiene_enabled()` 缺省 true。
- 0cy 设计只把「新会话启动（ACP `session/new`／CLI `-p`）」列为唯一删改路径
  (`docs/GSA_SESSION_HYGIENE_DESIGN_2026-10-11.md:30`)，`session/load` 不是新会话。

**影响**：新开 web（本进程 sessions 为空）→ 单击任意旧会话 B → 同工作区 A、C、D 全部被收卷、
侧车与 run 目录被删，5 秒后从「活跃会话」消失。若 A 是另一进程（TUI）正在跑的会话，
web 的 live 集看不到它，可能 unlink 其正在写的 journal（Windows 可能因占用失败，Linux 直接删）；
若 A 是 CLI 侧车会话，归档三键 journal 清单还漏 `RUN-CLI-{s8}`。数据可经 archives 回档，
但「只读打开」触发破坏性维护，且跨进程活 journal 可能不可恢复。

**建议**：拆出无 sweep 的 `attach_session`；sweep 只留在真正的 `session/new`；补两会话回归钉
（load B 后 A 的 sidecar/runs 原样）。若产品坚持 load 也收卷，必须改成显式「恢复并收卷」确认。

---

## §3 P1（按主题分组）

### A. 现场补丁的会话/恢复面

| ID | 发现 | 证据 | 影响／修法 |
|---|---|---|---|
| P1-A1 | `session/load` 不按 ACP 规范重放历史 | `stdio.rs:102-104` 广告 `load_session(true)`；`:124-139` 只返回空 `LoadSessionResponse`，零 `session/update`。ACP v1 规范要求响应前把整段会话经 `session/update` 回放（agentclientprotocol.com/protocol/v1/session-setup.md） | 外部 ACP 客户端恢复后窗口为空；web 靠 journal 自回放所以第一方看不出。实现回放，或在实现前不广告 capability |
| P1-A2 | `build_host` 失败泄漏 Prompt marker，被新 `contains_key` 永久化 | 注册 `acp_server.rs:3383-3393`；`build_host` 失败臂 `:3429-3436` 未 `runs.remove`；成功路径 `:3746-3749` 才清；`ORZ_TOOL_TIMEOUT_SECS` 非法 (`:4492-4494`) 可确定性触发 | 该会话之后所有 prompt 都报 `already in flight`；TUI 需重启、web 需重开 pump。加 Prompt RAII guard 或所有早退路径 remove；补 build_host 失败后重试钉 |
| P1-A3 | `next_prompt_count` 以 sidecar 存在为门 | `:3221-3226` 仅 `persisted_continuation.is_some()` 才推导；`:657-670` 只扫 `RUN-<s8>-<n>`；`persist_conversation_sidecar` 调用点 `:3732` `let _ =` 吞写失败 | sidecar 缺失/损坏/身份不匹配但 run 目录仍在时，`prompt_count=0` → 复用 `RUN-<s8>-0` → 向已有 events.jsonl 追加第二条 preflight/终态，verifier 判 invalid。无条件按 runs 扫描；补 sidecar-missing 钉 |
| P1-A4 | `restore_count` 未随 load 恢复 | `:3242` 无条件 `restore_count:0`；`:4036-4052` 生成 `RST-<s8>-<n>`；`bootstrap_session` 以 append 打开既有 RST 目录 | 载入已有 `RST-<s8>-0` 的会话后 restore → 追加第二条 preflight/终态，RST 链损坏。按 `RST-<s8>-*` max+1 恢复，或持久化计数器 |
| P1-A5 | codex `thr-{uuid}` 的 8 字符域过短＋`run_dir_session` 解析错误 | `codex_app.rs:238-240` 生成 `thr-{uuid}`；`acp_server.rs:3360-3361`/`:643-649` 取前 8（`"thr-"`＋4 hex，仅 65,536 种）；`:1246-1258` 对 `RUN-thr-1a2b-0` 取 `"thr"`；`:1888-1903` sweep 候选出现 `"thr"`；`run_attributable("thr")` 前缀匹配全部 `RUN-thr-*` | 多 codex 线程时 sidecar/run 前缀碰撞；sweep 把全部 codex run 归到伪造会话 `"thr"` 并删源/错归档。session 段改用 full UUID/更长 hash；`run_dir_session` 改 `rsplit_once('-')`；补 codex sweep 钉 |
| P1-A6 | `conversation_session_id` 用 64KiB 严格 UTF-8 解码 | `orz-web/src/runs.rs:78-88` 读 64KiB 后 `std::str::from_utf8(&data).ok()?`；`read_probe` 固定读 65536 字节 | 第 65536 字节落在多字节字符中间时返回 `None` → 长中文会话在 explorer 中随机失去续接能力（只读）。用 `from_utf8_lossy`／读到 `"session_id"` 结束／截到 char boundary；补 UTF-8 边界钉 |
| P1-A7 | 跨进程同会话互斥仍是单进程 `runs` 表 | 设计稿 `:151`／ADR `:5180-5182` 的「跨进程不可达」前提被 `session/load` 推翻；`acp_server.rs:3383-3391` 只查本进程；`next_prompt_count` 无租约；`persist_conversation_sidecar` 最后写者覆盖 | TUI＋web／两个 orz 进程可同时 load 同一 session，取同一 max+1 写同一 journal/sidecar。加会话租约（`.gsa/sessions/<s8>.lease`，PID/主机/启动时间，原子创建、stale 回收）或撤掉 load 能力；同步改设计 v1.2／ADR |

### B. Web/TUI 状态机与交互

| ID | 发现 | 证据 | 影响／修法 |
|---|---|---|---|
| P1-B1 | `/rewind` 在只读回看/归档视图下截断「绑定会话」而非当前视图 | `main.js:466-473` 的 `rewindPreview/Recall` 只用 `ensureSession()`（当前 `acpSessionId`）；`:751-759` 只查 `state.running`；`submitPrompt` 的只读门 `:76-88` 不覆盖 `/rewind` | 屏幕显示 A，实际截断 B；无 UI 撤销。让 `/rewind` 与 submit 共用只读/跨会话门；显式传 viewed session id 并由服务端校验 |
| P1-B2 | `openConversation` 自动 load 把「只读回看」变成「可续接」，文案仍称只读 | `main.js:326-341` 自动 `sessionLoad` 并切绑定；`:337` 说「下一条输入将续接」；`:353-358` 又说「只读」；`widgets.js:279-283` 单击即打开（归档/删除组操作也要求先单击选中） | 浏览动作改变活跃会话；随后输入写进旧会话；与 P0-4 叠加。默认保持只读，加显式「恢复并继续」动作；恢复后再改标记 |
| P1-B3 | `session/load` 完成期无代次/运行态复检 | `main.js:326-341` 在 `await initialize/sessionLoad` 后无条件写 `acpSessionId/sessionId/sessionRoot`；await 前只检查一次 `!state.running`；等待期间可 `returnToLive()` 并给旧绑定发 prompt | load 返回后绑定漂移；update 过滤器 `:846-854` 丢弃旧会话输出；`syncLiveTail` 用新 s8；`stopRun` 取消错误会话。加 `navEpoch/sessionLoadToken`，await 后校验；load 期间禁发/禁切 |
| P1-B4 | 状态栏「运行中/空闲」仍由 journal 回放驱动，与 `state.running` 分叉 | `projection.js:52-57` 仍 `setRunState('运行中')`；`:275-319` 仍写终态；`state.js:296-301` 停止按钮只看 `state.running`；`main.js:173-181` 在 `!state.running` 时提示「当前无运行中」 | 刷新/回放中断 run 时状态栏永久「运行中」、停止键灰、Ctrl+Z 说无运行；反向则发 prompt 被服务端拒。status 与 running 同源；回放无终态 run 标「未完成/历史」；服务端 busy 时触发状态重同步 |
| P1-B5 | `RUN-CLI` 实时尾回归，新 smoke 把 bug 钉成绿 | `main.js:240` 改为只认 `RUN-${s8}-`；HEAD 版 `:218-220` 正则含 `RUN-CLI-`；加载 CLI 侧车后 `returnToLive` 清空内容，`syncLiveTail` 找不到 `RUN-CLI-{s8}` → 空白实时窗；`frontend_smoke.mjs:754` 只断言新实现 | 点真实 CLI 会话后实时窗空白；应匹配 `RUN-<s8>-` 或精确 `RUN-CLI-<s8>`，只排除 ARC/RST；smoke 补 RUN-CLI 行为断言 |
| P1-B6 | web 实时 journal tail 的 64KiB 分块 bug | `orz-web/src/journal_tail.rs:28` 64KiB buf；`:68-90` 只在块内含 newline 时 `offset += upto`，不累计无 newline 块；合法 `supplement_delivered` text 上限 256KiB | 单行 >64KiB 时事件被丢/后续重复；web 补充卡不渲染、`deliveredSupplements` 不增，失败警告不出现（R6/S4⑥⑨失败）。移植 TUI 的 pending-buffer＋`offset += n`；补 >64KiB 行测试 |
| P1-B7 | 刷页/断线杀子进程，队列/后台任务无提示 | `acp_pump.rs:118-162` socket 关闭即 `start_kill`/`wait`；`main.js:837-842` beforeunload 只弹确认；`StoredSession.supplement_queue` 与 `terminal_backend` 在内核 | 刷新页会丢已排队补充、杀 30h 后台任务；session/load 只恢复文本/黑板/LIF，不恢复任务。beforeunload 统计并警告；UI 明说「仅文本上下文恢复，后台任务不恢复」；队列持久化另裁 |

### C. 0di 契约与实现完整性

| ID | 发现 | 证据 | 影响／修法 |
|---|---|---|---|
| P1-C1 | 侧车写非原子，recall 的「durable」承诺只在内存回滚 | `acp_server.rs:718-761` 直接 `std::fs::write`；`:677-710` 半写 JSON 视为损坏→None；`:4352-4363` 失败只回滚 continuation；305 R3 测试只造 `create_dir_all` 失败 | ENOSPC/断电/进程被杀时旧侧车被截断，重启整段会话丢失。改 temp+fsync+原子 replace，失败保留旧文件；补半写注入钉 |
| P1-C2 | 机械注入块登记不完整，污染侧车并进入 `/rewind` | `recallable_user_message` (`:3004-3008`) 只排除 `is_injected_block_text`＋`[任务状态`；`prompt.rs:63-119` 未登记 `[工作台]`（`agent_loop.rs:5403`）、`[上轮检索未派发]`、console 询问/重填、`[结果投递]`；成功写回过滤 `controller.rs:4504-4519` 保留它们 | `/rewind` 菜单出现框架文本、原文回填框架文本；305 R5「ordinal 只覆盖真实用户消息」不成立，S4⑧直接失败。登记这些前缀或改结构化 `injected` 标志 |
| P1-C3 | 满队补充绕过水位/硬截断/请求预算 | 估算/阶梯在 `agent_loop.rs:2523`、T1 在 `:2762-2803`；drain 在 `:3223-3245`；`build_model_face` 在 `:3261`，drain 后无二次估算；队列 16×256KiB | 480K 时投一条 256KiB 可推过 500K/700K → 提供方 400 → run 失败、补充蒸发。drain 后重算；加最低守卫/限量投并留队；S4 用满队样本 |
| P1-C4 | `recall_preview` 持全局 `runs` 锁读/解析整侧车 | `:4225-4248` | 大会话 `/rewind` 阻塞所有会话 prompt/queue 注册。锁内只做 idle 判定，clone/读盘放锁外 |
| P1-C5 | 前缀碰撞：真实用户文本以已登记前缀开头时被成功写回静默丢弃 | `controller.rs:4504-4519` 按前缀过滤 User；`queue_append` 不校验内容；`recallable_user_message` 同样隐藏 | 用户引用这些块讨论时，run 内看得到、下一轮/侧车没有，无警告。改结构化 `injected` 标志或对 prompt/supplement 来源豁免 |

### D. 文档／账本／契约真值

| ID | 发现 | 证据 | 影响／修法 |
|---|---|---|---|
| P1-D1 | README 与代码/ADR 多处矛盾 | 根 `README.md:39/41` 仍写检索默认关闭、需显式 `--retrieval-enabled`；实际 286 起缺省开；`README:62/89` 写「冻结 10 工具面」并列出已退役的 `context_compress`；实际 8 保留＋`blackboard_read`＋`blackboard_write`＋`context_manage`＋`submit`＝12；`README:64` 称 TUI/ACP 可跨进程恢复，TUI `input.rs:185` 明写 never resumes；`README:81` 称 fires 只内部留痕，RLI notices 实际模型面注入 | 用户看到错误的网络默认、错误的工具面、错误的恢复能力；英文 `docs/en/README.en.md:51/53/73/75/83` 同形。以代码单源生成/校验 README 关键数字 |
| P1-D2 | `orz/SECURITY.md` 安全声明过强且过期 | `SECURITY.md:24` 宣称系统核心（systemroot/Program Files/ProgramData；/boot /etc /usr /lib* /bin /sbin /dev /proc /sys）与载体自保护集硬拒；实际 v3 只拒 `.gsa`＋keystore/manifest，`/etc`、`/usr` 写放行；`:28` 称 Linux Landlock「待落地」，实际已实施；`:5` 安全报告仍指向 `hackerone.com/x`（xAI 上游），不是本仓渠道 | 外部安全研究与用户会误信防护范围；与 README 的「两窄目标」自相矛盾。按 0cc v3 重写 SECURITY；HackerOne 改本仓渠道；把 `.gsa` 祖先链等未实现项如实登记 |
| P1-D3 | 工具面冻结口径三方不一致 | ADR v1.74 写「10 工具」；§14.87 写「8 工具面第二显式例外 +2」；README 写 10；代码实际 12 | 模型面冻结是用户主导契约，必须单源。以 `tool_names`/生产装配为单源，加精确面测试，同步 ADR/README/索引 |
| P1-D4 | 索引 `FUS-BUDGET` 仍为 `current-design` 的 120 轮 | `CLI_PROJECT_INDEX.md:139`；ADR-0008 已注「默认 unlimited」；README:73 也写无限制 | 索引与 ADR/README 冲突。改 `withdrawn`/`reference` 或改写为「默认 unlimited，显式 >0 逃生阀」 |
| P1-D5 | 0.9.3 载体不含现场补丁，S4 计划无效 | 308 源冻结 `5fd4c42d`；补丁 7 文件未提交；`check_repository` 当前 `valid:false`（dirty）；`orz_source_manifest.sha256` 1,488 entries 未吸收补丁 | 对 0.9.3 跑 S4 验证旧行为；发布/账本/补丁三方不一致。先落设计/审查、提交补丁、重算 manifest、重建 0.9.4/S3′，再 S4 |
| P1-D6 | 现场补丁无设计/审查/账本记录 | 最新 audit 到 317；索引 v4.289；0di 仍 pending、0dj implemented；设计稿 v1.1；ADR §14.88 v1.92；无 318 档 | traceability 断裂；后续无法区分「0di 原设计」与「现场新增 session/load」。新增 318 档、设计 v1.2、ADR §14.88 v1.93（或新 §14.89）、index/BACKLOG/TODO 同步 |
| P1-D7 | Python 合约测试与生成器漂移 | 见 P0-1；`scripts/generate_run_event_fixtures.py:3504-3506` 会 rmtree 后漏生成 supplement/usage；envelope fixture `character_count:12` 而 text 13；README/registry 计数 57/41 陈旧 | 契约门红；按文档重生成会删 fixture；305 R15 只修 payload minimal。67→69、生成器补两事件、envelope 改 13、计数同步 |
| P1-D8 | 5 本账本、12 处状态矛盾、112/458 篇 audit 不可达 | `CLI_PROJECT_INDEX.md` §8 桶 vs 条目状态 12 处冲突（如 `DESIGN-WRITE-CONTROL-BACKSTOP` entry pending/§8 implemented）；BACKLOG/TODO/INDEX 之外还有 `_2`/`TODO2`/`BACKLOG2`；112 篇 audit 未被三本账本逐字引用 | 状态查询会读到互相矛盾的结论；审计不可达。砍成三本、`_2` 归档、index 由 BACKLOG 生成、状态交叉校验入 gate |

---

## §4 P2／P3（摘要）

**P2**
- `session/load` 的 cwd 未要求绝对路径、未 canonicalize/信任校验；`conversation_sidecar_path` 直接拼前 8，未用 `require_session8`（路径分隔符/`..` 加固面）。
- `session/load` 对已在内存会话直接返回 loaded，不校验 cwd/policy/retrieval；`PermissionPolicy::default()`＝Interactive（yodl 自动放行），read-only 会话可被 load 成可写。
- `prompt_count` 在 `:3350-3351` 先自增、`:3388` 才检查并发拒绝，拒绝也烧号；`next_prompt_count` `read_dir` 失败静默 0（fail-open）。
- `openConversation` 每次 load 在同一 agent 进程新增 `StoredSession`（terminal backend/browser），web 无 `session/close`；`sessions` map 无界；`newSession` 也不 close 旧会话（0dd「新会话收上一轮」在单进程内不成立）。
- 删除已加载绑定会话后仍可继续：`deleteSession` 只 spawn `orz delete-session`，不清 `acpSessionId`；下一条 prompt 把 sidecar 写回，「彻底删除」失效。
- 失败/取消 run 的 assistant/tool 输出不写回 continuation，屏幕有、模型没有；提示只覆盖补充（`projection.js:285-319`）。
- `/rewind` 成功只有 4 秒 flash、无持久系统行（TUI 有）；`/rewind` 不在 web help/命令表；只读门在清空输入之后（`main.js:796-807`），拒绝时草稿丢失；`conversation://` 的 s8 未转义进 RegExp（`main.js:317`）。
- `recall_preview` 返回 `queue_depth` 但双端丢弃；用户确认撤回前不知道会丢几条补充。
- `Prompt` 无 RAII（见 P1-A2）；`RestoreInflightGuard::drop` 无所有权校验，配 close×session/load 可误删新 Prompt marker/写回旧 continuation。
- 归档三键 journal 清单漏 `RUN-CLI-{s8}`（`acp_server.rs:2605-2614`／`:961-994`）。
- `runtime/fixtures/.../README.md` 57 events vs 实 69；`assurance/run_event_journal_validation.py` v02=41 vs 实 42。
- 0di S4 判据①要求「事件先于当轮 `model_request`」，但 `EventType::ModelRequest` 无生产者；测试实际用 `FaceFingerprint`；S4 字面不可执行。
- 305 R20 称增补 Unicode 判据，但设计 D8 九条无 Unicode；302 §6 S4 仍 4 条。
- `orz-web` 47 单测＋648 行 smoke 不在 CI；`orz-tui`、`orz-bin`、Linux 不在 CI；17 个 crate 从发行二进制不可达（1,830 个 `#[test]` 永不跑）。
- 仓库卫生：根 `.tmp-*` 740 个/9.44MB、根 `tmp*` 目录 10 个、`.gsa` 137MB；D 盘仅 0.55–6GB 空闲，`target` 85GB。

**P3**
- TUI 同文 prompt 第二次不出卡（`projection.rs:35-42/91-107`）与 web 不一致；双端 `/rewind` 默认端相反；web rewind 行不可键盘选择。
- `frontend_smoke.mjs` §14 全是源码 regex，不执行 UI；`doesNotMatch` 漏 `run_terminated`。
- `session/load` 的 `LoadSessionResponse` 返回额外字段；stdio 错误全坍 `-32603`，busy 属正常条件宜映射 `-32600`。
- `next_prompt_count` max+1 会跳失败 attempt 烧掉的 ordinal；`ARC-<s8>-<prompt_count>` 在 close/load 无 prompt 循环下可能重复。
- `run_started` 仍把状态栏设「运行中」但 `state.running=false`；回放旧失败 run 会触发 disclose 假警报。
- `recall_preview` 持锁读盘（P1-C4）；`list_conversations` 每 5s 对每个 `.json` 读 64KiB；`LIST_CAP=200` 使旧会话 run 列表缺失。
- 17 个不可达 crate、18,695 处 `unwrap/expect`（排除测试仍 584）、78 处 `#[ignore]`、22 处 `unimplemented!`；`acp_server.rs` 9,101 行、`agent_loop.rs` 9,680 行、`controller.rs` 8,126 行。
- 远端/本地分叉：远端 main `004f8b72` vs 本地 `215245d9`；本地 10 个 docs 提交未推，远端 4 个 README 直改提交不在本地；下次 push 会被拒。
- `orz_source_manifest.sha256` 1,488 条与 HEAD tracked 精确对齐（健康面）；但未提交补丁不纳入，commit 后必须重算。

---

## §5 设计合理性（项目级）

1. **会话生命周期缺少单一状态机**。0cy 的启动侧 sweep、0dd 的「新会话收上一轮」、0dh 的会话级
   terminal backend、0di 的会话级补充队列、现场补丁的 `session/load`，五个机制各自合理，
   但没有共享的「active-attached／active-detached／archived／closed」状态图与唯一 owner。
   直接后果：`newSession` 不 close 旧会话却宣称归档；`session/load` 复用 new 触发 sweep；
   跨进程租约缺失；web 刷新杀任务/队列；`delete-session` 后会话复活。
   **建议**：在 ADR §14.88/§14.89 增「会话附着/恢复/关闭」状态机，明确 sidecar/journal/
   terminal/queue/browser 的归属与跨进程租约；所有入口（TUI/web/ACP/codex/headless）共用。
2. **UI 运行态有两个 writer**。0di 真机 bug 的根因是 `state.running` 同时被「本客户端 prompt
   生命周期」和「journal 投影」写；补丁只删后者，status 仍由 journal 写，于是出现
   「状态栏运行中、停止键灰、Ctrl+Z 说无运行」。需要显式的 live/replay/loading 视图模式与
   单一运行态 owner，journal 只提供历史事实。
3. **安全边界的口径与实现脱节**。README 已把写控诚实地收窄为「宿主状态两窄目标＋五条命令规则」，
   但 SECURITY.md 仍按 v1 宣称系统核心硬拒；同时规则本身对 `.`／通配／`find -delete` 与
   维护 CLI 不设防。安全设计应按「保证／阻力／审计」逐条对代码，而不是按旧设计稿宣传。
4. **工具面冻结契约漂移**。10→12 的变化有用户裁决，但 ADR、README、索引、代码没有单源；
   对一个「固定工具面」为核心卖点的产品，这是设计权威问题，不只是文档问题。
5. **门禁非封闭**。本地绿/CI 红、dirty 单错掩盖 51 条内容错、硬编码 contract 钉值、
   5 本账本与 112 篇不可达 audit——治理体系的数量已经超过机械校验能覆盖的范围。
   需要把「门禁」从单布尔提升为可复现的 clean-tree 检查与分类报告。

---

## §6 实现合理性（项目级）

- **强项**：0di 主路径（队列、drain、事件先于消息、recall 三方互斥、R1–R4/R6/R8 修复）
  经代码与测试复核成立；305 的处置没有虚报；host 串行 385/0/6、web 47/0、frontend smoke 全过；
  manifest 与 tracked 精确对齐；许可/第三方声明完整；`Cargo.lock`/toolchain 锁定。
- **弱项**：`acp_server.rs` 9,101 行同时承载 session/prompt/recall/supplement/archive/hygiene/load，
  P0/P1 全部集中于此；`agent_loop.rs` 9,680 行、`controller.rs` 8,126 行；
  Prompt 无 RAII、sidecar 非原子写、`read_dir` fail-open、`let _ = persist` 吞错；
  web tail 64KiB 分块 bug 与 source-regex smoke 使 0di 的可见面无法被测试保护。
- **测试**：数量大但覆盖偏；CI 实际只跑 loop 单包且并行红；`--lib` 排除 478 个集成测试；
  17 个 crate 不可达；`#[ignore]` 78 处；`unwrap/expect` 生产面 584 处。
- **运行环境**：D 盘 0.55–6GB 空闲、target 85GB；任何构建/测试都可能因空间失败；
  protoc 未入库、CI 下载无 sha256 校验、README 构建前置不全。

---

## §7 设计与实现符合性（关键偏差）

| 设计/声明 | 实现 | 判定 |
|---|---|---|
| 0di D1「prompt-vs-prompt 服务端 overwrite」 | 现场补丁改为 any-inflight reject（`acp_server.rs:3388-3392`） | 偏差，设计/304 未同步 |
| 0di §4.7／ADR「跨进程同会话不可达」 | `session/load` 已实现并广告 | 前提失效 |
| 0cy「负载只在新会话启动扫描」 | `session/load` 复用 new 触发扫描 | 偏差（P0-4） |
| 0dd「web 新开对话→扫描收上一轮」 | `newSession` 不 close 旧会话，本进程 live 集保护旧会话 | 不成立 |
| 0dh「跨 run/会话域存活」 | web 页面刷新杀子进程 | 产品面边界未登记 |
| README「检索默认关闭」 | 286 起缺省开 | 不实 |
| README「10 工具面/context_compress」 | 实际 12/context_manage | 不实 |
| README「TUI/ACP 可跨进程恢复」 | TUI never resumes | 半不实 |
| README「fires 只内部留痕」 | RLI notices 直投模型面 | 不实 |
| SECURITY「系统核心硬拒」 | 只拒 `.gsa`＋keystore/manifest | 不实（过强） |
| SECURITY「Landlock 待落地」 | 已实施 | 过期 |
| INDEX `FUS-BUDGET` 120 current | ADR/README 默认 unlimited | 不实 |
| 305 R5「ordinal 只覆盖真实用户消息」 | `[工作台]` 等未登记 | 不成立 |
| 305 R15「fixture 字符数已勘正」 | envelope 仍 12，text 13 | 不完整 |
| 0di S4①「事件先于 model_request」 | 无 `model_request` 生产者 | 不可执行 |
| 306「0di 实现＋审查面已完结」 | 真机 bug＋补丁暴露新面 | 历史事实，需 318 追记 |
| 308「工作树干净」 | 当前 7 文件 dirty | 历史事实，需追记 |

---

## §8 Phase 2（本轮未覆盖，建议下一轮）

1. **权限/ACAF/session volume 深审**：安全子代理仍在补 ACAF、凭据、`.gsa` 读面、permission 判定面。
2. **上下文/压缩/T1/滑块**：v8 投影、`context_manage`、压缩 marker、500K/900K 窗口、RLI 注入面。
3. **检索默认开与浏览器车道**：默认开后的网络/成本/隐私面，SERP/DDG/代理链，子代理并发。
4. **评测与计量**：SCB/TB 读数、`run_usage_metered`、成本代理、S4 判据的可执行性。
5. **UI/UX**：三份 UI 设计稿 vs web/TUI 实际，键盘/可发现性/回执/无障碍。
6. **代码健康**：17 个不可达 crate、超大文件、unwrap/ignore、D 盘与 target。
7. **账本收敛**：5 本账本→3 本，112 篇 orphan audit 归档/索引，状态交叉校验。

---

## §9 建议动作（顺序）

1. **停止对 0.9.3 跑 S4／停止再发行**；先把 0.9.3 标为「功能预览、S4 未验、CI 红」。
2. **修发布门禁**（P0-1）：loop 串行/确定性、clean-tree 链接检查、合约 67→69、补 web/Linux/集成 job、
   release 加 `needs` 与分支保护；重算 manifest。
3. **修安全边界**（P0-2/P0-3）：通配/`.`/`find -delete`/受护祖先 block；维护 CLI 桥令牌或命令面 block；
   按 0cc v3 重写 SECURITY；补负向钉。
4. **修会话恢复**（P0-4/P1-A）：load 不 sweep；Prompt RAII；`next_prompt_count` 无条件扫描；
   `restore_count` 恢复；codex 前缀/解析；UTF-8；跨进程租约；ACP 历史回放。
5. **修 web 状态机**（P1-B）：running/status 同源；`/rewind` 只读门；显式恢复 vs 浏览；
   load 代次；RUN-CLI tail；64KiB tail。
6. **修 0di 契约**（P1-C）：原子 sidecar；注入块登记；满队守卫；recall preview 锁；前缀碰撞。
7. **落账**：318 审查档（本档）＋设计 v1.2／ADR §14.88 v1.93（或新 §14.89）＋建议 `0dk`
   （会话恢复）／`0dl`（安全边界）＋索引/ BACKLOG/TODO；0di 保持 pending，0dj 不重开。
8. **重建 0.9.4／S3′**：提交补丁→重算 manifest→双平台重建＋新字节判据→`check_repository` 绿→S4。

**建议计数**：当前 60 不变；若用户裁决立项，`0dk`（会话恢复）60→61，`0dl`（安全边界）61→62。
本档未动计数。

---

## §10 证据附录（本档亲验）

```text
git -C D:\CLI status --short         →  M orz（父仓）；orz 7 个 M 文件
git -C D:\CLI\orz diff --stat        → 7 files, +251/−27
git ls-remote origin refs/heads/main → 004f8b72（本地 main 215245d9；本地 origin/main 8dfa30c1）
gh run list -R SilverWhite/orz -L 20  → main 自 54a4fcb9 起 failure；5fd4c42d 一次 success
gh run view 39242209300 --json jobs   → 5/5 failure
gh run view 39197221568 --json jobs   → 5/5 failure（v0.9.3 发行提交 9b9d5b98）
cargo test -p orz-host --lib -- --test-threads=1 → 385 passed; 0 failed; 6 ignored
cargo test -p orz-loop --lib          → 884 passed; 2 failed; 3 ignored（并行；CI 同形）
cargo test -p orz-web --lib           → 47 passed; 0 failed
node crates/orz-web/tests/frontend_smoke.mjs → 全部通过（含 §14）
python -m unittest ...test_run_event_conformance → Ran 16; FAILED (69 != 67)
python scripts/check_repository.py    → valid:false, error_count:1（dirty；本地 .tmp 掩盖 51 条）
```

- `.tmp-*` 断链：`docs/audits/074/075/076/...` 共 37 处（CI 报 51 条，含 TODO/BACKLOG 与仓外
  `tb-eval`／`存档/evidence/*.pdb`）；`git check-ignore` 逐条命中。
- 本档为 Phase 1；安全子代理的 ACAF/凭据/`.gsa` 读面最终结论未并入（仅并入其已确认的
  session/load sweep、维护 CLI、写控绕过三项）。
