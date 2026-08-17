# ORZ-LARGE-FILE-READ-CONTRACT 实施审计（2026-08-17）

- **范围**：GrokBuild `read_file` 工具文本路径有界返回——超过粗门的文件返回
  读取句柄信封（path/size/encoding/content_sha256/available_range/有界预览
  ≤4KB/truncated/offset 续读指针）而非全文；小文件保持全文一次返回；信封
  terminal-only 不流式。涉及 `orz-tools`（output.rs 信封类型 + grok_build
  read_file 信封构建 + hashline 测试同步 + cursor_rules 构造补字段）、
  `orz-loop`（console 注册表 `workspace.read_file` 描述 + BASE_SYSTEM_PROMPT
  读取纪律）、`orz-host`（read_file e2e 测试）。
- **依据**：ADR-0010 §14.22（v1.22）项 1–3 / FUS-LARGE-FILE-READ-CONTRACT /
  CLASSICAL-EXEC-ASSISTANT 设计 §11 / PLAN_FIRST_BLACKBOARD 设计 §4 /
  BACKLOG 6f / TODO P1。

## 实施内容

1. **信封类型**（`types/output.rs`）：`LineRange`（1-based 闭区间）+ 
   `ReadHandleEnvelope`（path/size/encoding/content_sha256/available_range/
   preview_range/preview/truncated/offset）；`ReadFileOutput` 新增
   `ReadHandle` 变体；`output_encoding()` 返回信封 encoding、`is_error()`
   判成功、`to_prompt_format()` 渲染 `[read handle]` 文本（含续读指令）。
2. **粗门配置**：默认 16KB，`ORZ_READ_FILE_COARSE_GATE_BYTES` env 口子 +
   `ReadFileParams.coarse_gate_bytes` TOML/config 口子（参数优先），均钳制
   8–32KB。
3. **信封构建**（`run_read_file` 文本路径、`!is_skill_markdown` 且
   `file_bytes.len() > 粗门`）：沿用 `N→` 行锚点格式构建有界预览（≤4KB，
   char 边界安全）；`truncated = 行内截断 || preview_end < total_lines`；
   `offset = Some(preview_end+1)` 供 `read_file(offset=…)` 续读；尾部幻影行
   仅在正常窗口耗尽时追加（预算中断不越界）；`content_sha256` = 原始字节
   SHA-256 hex（复用 pdf_evidence::hex_string）。
4. **边界**：SKILL.md / `skills` 路径 Markdown 保持全量读取豁免（技能文档不
   被静默截断）；PDF/PPTX/图片路径不变；`FileTooLarge`（25K token 事后拒绝）
   在文本路径被信封取代（≤32KB 文件不可能超过 25K 估计 token），保留为防御
   兜底；单行超长在预算内截断并报 truncated（offset 指向下一行，长行尾部经
   grep/execute 侧取——与既有单行长行提示一致）。
5. **模型面契约提示**：`BASE_SYSTEM_PROMPT` 读取纪律补信封语义（v1.9/v1.22）；
   `DESCRIPTION_FULL` 补粗门/信封说明；console 注册表 `workspace.read_file`
   描述补信封与 offset 续读。黑板/结果栏经信封只承载有界预览+指针，全文内容
   不上黑板（维持「不新增自由随记区」）。

## 测试证据

- orz-tools read_file 模块 199/199（新增：信封字段/预算边界/offset 起点续读/
  终点窗口 offset=None/单行截断/粗门参数覆盖/SKILL.md 豁免；改造：原
  FileTooLarge 用例改为信封断言）；output `read_handle_envelope_json_round_trips`
  1/1（序列化 round-trip + to_prompt_format 渲染）。
- orz-loop lib 440/440（console/prompt 改动无回归）。
- orz-host `call_read_file_large_file_returns_handle_envelope` + 
  `call_read_file_returns_content` 2/2（host 工具集 e2e：大文件信封、小文件
  全文）。
- clippy（orz-tools/orz-loop/orz-host all-targets）：无新增可归因告警（orz-loop
  lib 22 = HEAD 基线 22；hashline 未用导入已清理）。
- cargo fmt --all 收口（含此前 grep/planning/projection 遗留格式漂移）。

## 环境边界（本机验证范围）

- 本机 PATH 的 `rg` 链接损坏（WinGet 快捷方式启动失败），orz-tools
  grep/glob 43 项测试环境性失败（退出码 -1 = spawn 失败），清理前后计数一致，
  与本次改动无关。
- orz-host 全量套件在本沙箱环境存在网络类用例挂起（日志止于 codex_app 段，
  低 CPU 阻塞），未跑完；本次改动相关的 read_file 用例已单独通过。
- 构建依赖：`protoc` 本机缺失，已下载 protoc 25.3 至临时目录并仅用于本次
  构建验证（未改动仓库/环境）；磁盘清理：orz target 缓存 151.8GB 占用致 D 盘
  空间不足，已 `cargo clean` 后重建（构建产物可再生，无数据损失）。

## 登记

ADR-0010 §14.22（v1.22）项 3 / BACKLOG 6f / TODO P1 / CLI_PROJECT_INDEX /
操作台设计 §11 / 黑板设计 §4；orz 子模块 172b14e；manifest 重生成 1401 条目。

## 全面检查修复（2026-08-17 审查后处理）

按设计合理性 / 实现合理性 / 设计与实现符合性三面复查，处理全部审查问题：

1. **P2-1 空窗口/越界 offset 语义（代码修复 + 测试）**：旧实现空窗口
   （起始行超 EOF 或 `limit=0`）返回 `truncated=true` + `offset=Some(1)`，
   误导续读绕回首行；行内截断恰为文件最后一行时返回越界 `offset=Some(end+1)`。
   修复：past-EOF 窗口 `truncated=false`、`offset=None`、渲染报实际行数
   （对齐 FileContent 路径 past-EOF 提示）；范围内空窗口 `offset=Some(start_line)`
   续读；`preview_end >= total_lines` 时 `offset=None`（长行尾部经 grep/execute
   侧取）；`preview_range.start_line` 恒承载请求起始行（空窗口 `end_line=0`，
   `to_prompt_format` 按 start_line 与 available_range.end_line 的关系区分
   past-EOF / 空窗口）。新增测试：`envelope_past_eof_window_is_not_truncated`、
   `envelope_in_range_empty_window_resumes_at_start_line`、output 渲染两例；
   单行大文件测试断言更新（无尾换行 → offset=None；有尾换行 → Some(2)）。
2. **P3-1 TOML 口子接线明确**：生产 host 无独立 config.toml 工具参数管线；
   `ReadFileParams.coarse_gate_bytes` 为资源层工具参数口子，新增
   `AgentBuilder::with_read_file_params` 合并进
   `GrokBuild:read_file` / `GrokBuildConcise:read_file` 工具参数
   （与 bash/ask_user_question 同一 ToolConfig params 通路），新增测试
   `read_file_coarse_gate_params_reach_read_file_params` 锁定；文档口径收窄为
   "env 口子（生产可达）+ 工具参数口子（TOML/config 注入通路）"。
3. **P3-2 concise 变体描述同步**：`DESCRIPTION_CONCISE` 补粗门/信封/offset 续读
   说明（原审计只同步了 DESCRIPTION_FULL 与 console 注册表）。
4. **P3-3 cursor rules 边界登记**：envelope 路径早于
   `append_cursor_rules_for_read`，大文件信封不追加 cursor rules（有界预览保持
   纯文件内容）；登记为有意边界，`cursor_rules_on_read` 仅作用于全文路径。
5. **P3-4 提示词措辞精确化**：BASE_SYSTEM_PROMPT 读取纪律改为"只有证据关键的
   小文件才读全文，大文件一律经信封分段续读"（原"只有证据关键文件才读全文"与
   超粗门无全文并存，易误导模型）。

测试证据（本窗口复跑）：orz-tools read_file 201/201、types::output 84/84、
orz-agent 工具参数通路 1/1、orz-loop lib 440/440（3 ignored）、orz-host
read_file e2e 3/3；clippy 无新增可归因告警（orz-loop lib 22、orz-host lib 2
与基线一致）；cargo fmt 已收口；manifest 重生成 1401 条目、仓库门禁 valid。
orz 子模块 7c4a99e（feat/fusion-architecture）。
