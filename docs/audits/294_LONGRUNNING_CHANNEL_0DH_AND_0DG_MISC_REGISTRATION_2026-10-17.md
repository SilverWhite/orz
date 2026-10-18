# 294 批：0dg 立项（293 修复杂项）＋0dh 立项（长程进程显式通道）——长程收割真机证据、Grok 机制盘点与业界调研（2026-10-17）

> 性质：**立项＋只读取证批**（用户令「293 批中需要修的内容直接落成一个待办杂项」＋长程通道两条设计裁决＋业界调研令）。
> 零源码、零 orz 改动、零跑批；计数 62 → 64（0dg、0dh 各一件）。

## §1 由来（用户五项指令）

1. 293 批中需要修的内容直接落成一个待办杂项，方便后续修；
2. 长进程跨 run 存活，**不做收割**；且 10h 不够，**放宽至 30h**；
3. 查目前 orz 最新会话存档（新建文件夹路径）——昨晚模型告知无法启动长程；
4. Grok 的后台进程管理机制我们留没留，能不能拿来用？
5. 教模型怎么用会是大问题：极常用工具，用备注（长描述）会频繁注入干扰内容，
   机制得往简化做，不能叠加复杂内容——查一下业界的实现。

## §2 真机证据（会话 b41b096d，runs 3–7，2026-10-17 晚）

昨晚训练轮（R225 冻结 decoder 重跑）完整事故链，journal 与模型自述逐段核对：

- **第一次（`Start-Process` 脱离挂后台）**：调用结束即被回收——模型自述
  「子进程随调用域一起被 job object 杀掉，只跑到 ep1」。机理＝Windows 侧
  每调用 Job Object kill-on-close 收容（0bl ⑦ per-call JobObject），脱离
  出去的子进程仍在 job 内，句柄关闭整树回收。
- **第二次（受管后台路径，PID 16164，21:33:13 起）**：跑至 **ep96（21:36:44
  末次写入）** 后在跨 run 边界消失——模型自述「走受管后台路径也只在调用
  存活期内有效，跨轮不再续命」「进程列表里已无 python」。机理＝run 收尾
  **`run_shutdown` sweep**（`orz-host/src/lib.rs:601`，
  `SweepMode::Finalize{own_run_id}` → `kill_pid`；0bq/0z 线的 run 尾扫净）
  把登记在本 run 名下的进程树整棵收割。
- **模型结论（RUN-6）**：「这个会话里起不了持续数小时的训练」，改向用户
  交接（`schtasks` 操作系统级计划任务方案——不属 shell 子进程、不受 job
  object 约束）；用户在自己 PowerShell 窗口手动起 P0 进程后存活（RUN-7
  核查：orz 三进程与 P0 训练进程并存，模型判 trampoline 经
  ParentProcessId 验证）。
- **用户判语（RUN-7 prompt 原文）**：「看来是目前版本的orz存在bug，待我
  修一下」。

**判定**：非偶发 bug，是两层收割语义的必然结果——①per-call Job Object
kill-on-close（前台域）；②`run_shutdown` sweep（run 域）。TER 后台化管线
本身存活判据（T2.3）在**单 run 内**成立，跨 run 存活从未被验证或承诺。

## §3 现役机制盘点（含 Grok 机制问）

**TER 后台化管线（已落码在役）**：前台阻塞预算 180s（TER T1.2 首报后台化）
→ `tool_running` 中报＋`tool_completed{running:true, task_id, output_file}`
→ 回查面 get_task_output／ListTasks／WaitForCompletion／KillTasksByOwner
→ 完成后 system-reminder（`surface_bg_completion_reminders` 默认 true）
→ idle-kill 兜底 300s「无输出增长且 CPU 时间不增」（CPU 忙不杀、CPU 记账
未知不判）＋ 10h 绝对兜底（TER T1.5）。Windows 后台存活 T2.3 已验
（fake 场景、单 run 内）。

**模型面封闭（TER T1.3）**：`hide_background_input` 默认 `true`——
`is_background` 参数不进模型 schema、显式后台输入被拒；后台化只能由框架
180s auto-bg 触发。**这就是「起长程进程」体感摩擦的根源**：模型不能声明
长程意图，只能前台死等 180s。

**Grok 机制＝留了，且可整链复用**：上游显式后台面完整在树——`is_background`
参数＋描述模板（「`timeout: 0` in background mode disables the wrapper
timeout entirely; the child's lifetime is owned by the model via
kill_task」）＋`requires_expr`（显式后台可见时强制 get_task_output/kill_task
同台）。TER T1.3 只关了默认门，`hide_background_input=false` 逃生阀一根
开关重开（现仅测试在用）。**但**：重开显式面只解决「模型声明」，解决不了
§2 两层收割——跨 run 存活要在收割语义层动刀（用户已裁决：不收割）。

## §4 业界实现调研（用户令「查一下业界的实现」）

- **Claude Code**（最成熟形态）：Bash 工具布尔参数 `run_in_background`＋
  `BashOutput`/`TaskOutput`（按 shell id 读增量输出）＋`KillShell`/
  `TaskStop`；教学＝工具描述内一句话＋参数自解释，**无额外注入机制**；
  后台 shell 会话内跨轮存活、不跨会话。
- **Gemini CLI**：`run_shell_command` 支持 `is_background`（`&` 亦立即
  返回），但后台进程管理面（列表/日志/杀）仍在演进（issue #14845、
  #21999）——「只有起、没有管」的反面样本。
- **OpenAI Codex CLI**：长期缺位，2026 年才上 experimental background
  terminals（issue #3968，明引 Claude Code 先例）；仍有粗糙面——后台完成
  不唤醒模型（turn 结束即闲置）、跨轮存活近期出现回归（issue #10767，
  与 orz 昨晚实锤同形）。
- **对 orz 的映射**：orz 现役管线（task_id/回查/完成提醒/双兜底）已接近
  Claude Code 完备度、高于 Gemini；真缺口恰是业界也踩过的同一块石头
  （Codex #10767＝跨轮收割）。教学面业界共识＝**schema 参数自解释＋结果
  文案即时教学（just-in-time）＋一次完成提醒**，忌长备注常驻注入——与
  用户简化诉求一致。

## §5 设计基调（供 0dh S1 设计稿；用户两条裁决已定案）

- **通道形态**：沿上批结论——同工具可选参数（重开 `is_background` 或等价
  枚举），声明即**立即后台化**（不烧 180s 前台预算），复用现役管线
  （task_id／output_file／回查／完成提醒）。
- **裁决一（跨 run 存活）**：后台任务**不做收割**——`run_shutdown` sweep
  对显式/受管后台任务开豁除口（owner 归会话不归 run；豁除面如何与 0bq
  「run 尾扫净绕出 Job 的游离进程」兜底初衷并存，随 S1 定）。
- **裁决二（兜底放宽）**：绝对兜底 10h → **30h**；idle-kill 300s
  CPU 感知判据原样保留（对静默计算型训练友好）。
- **per-call Job Object**：长程任务的宿主进程不得落在前台调用域 job 内
  （否则调用结束即回收——昨晚 ep1 事故机理），归属层级随 S1 定。
- **教学面（用户简化令）**：不写长备注——参数进 schema（枚举/布尔自解释）
  ＋描述一句话增量＋起任务时结果文案即时带 task_id 与回查指引（只在用时
  花 token）＋完成 reminder（现役）。机制往简化做、不叠加复杂内容。
- **S4 真机判据（预登记）**：一次跨 run 过夜训练在 orz 会话内存活至完成
  （对照昨晚 ep96 事故）；run 尾 sweep 后仍存活；兜底 30h 生效路径可回放。

## §6 处置

- **0dg 立项（62 → 63）**＝293 真机反馈修复杂项：①web 对话卡用户/系统卡
  `white-space: pre-wrap`（一行 CSS；293 N1）；②Ctrl+Z 输入框内豁免还原生
  撤销＋闲置期按下给反馈（293 N2）。OBS-WEB-DOGFOOD-2026-10-17 candidate
  三件中两件随本立项升级（N3 撤回/中途补充维持 candidate 待设计裁决）。
- **0dh 立项（63 → 64）**＝长程进程显式通道设计线（TER §14.55 T1.3 封闭面
  的受控重开＋两层收割语义修订＋30h 兜底）；S0 调研与机制盘点本批已收
  （§2–§4），S1 设计稿待启动；批序 S1 设计→S2 落码→S3 载体重建→S4 真机。
- 零源码、零跑批；本批未发起任何 orz 会话。
