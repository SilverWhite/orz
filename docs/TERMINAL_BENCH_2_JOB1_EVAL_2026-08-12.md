# Terminal-Bench 2 job1 评测记录（2026-08-12）——v1.5 工具可用性重构验证

> 日期：2026-08-12
> 范围：TB 2.0 job1（gpt2-codegolf + make-doom-for-mips，900s×2 并发），两次运行共 4 trial
> 目的：验证 ADR-0010 v1.5 工具可用性机制重构（registry 全量目录 + 调用时逐次判定，
> orz `28b67fc`）在真实评测中的行为；对照 B 组 9/9 全灭（2026-08-11）的"声明可用却
> 调用被拒"病灶
> 结论摘要：**v1.5 核心行为验证达成**（拒绝→转向、无盲改、熔断按设计）；4 trial 中
> 3 个死于同一架构缺口 GAP-STREAM-RETRY（流式中断不可重试），1 个为任务难度超时；
> 无独立于 GAP-STREAM-RETRY 的新架构缺口

## 1. 运行配置

| 项 | 值 |
|---|---|
| 二进制 | `D:\tb-eval\orz-linux\orz`（2026-08-12 01:31 重建，含 28b67fc，67,821,328 B） |
| 模型 | `deepseek-v4-flash`（`run_pro_gpt2.sh` 已从 pro 对照回退；orz.py 默认） |
| 任务 | gpt2-codegolf + make-doom-for-mips（`relaunch_4.sh job1`，2 并发） |
| 卷 | `gsa-volumes/b4-900s`；`max_wallclock=1740`（900s×2 超时倍率） |
| 运行 1 | 01:53:51~01:56:14（2m17s，两题 agent 异常退出） |
| 运行 2 | 01:58:02~02:38:36（40m34s，一题异常退出、一题 wallclock 超时） |

## 2. 结果总览

| Trial | 结果 | 失败方式 | 归因 |
|---|---|---|---|
| gpt2-codegolf__ySayq4F（运行 1） | 0.0 | 首轮模型请求 transport error，agent exit 1 | **架构不足：GAP-STREAM-RETRY** |
| make-doom-for-mips__uvrZze4（运行 1） | 0.0 | 第 8 轮模型请求 transport error，agent exit 1 | **架构不足：GAP-STREAM-RETRY** |
| gpt2-codegolf__Ad9CbFB（运行 2） | 0.0 | 第 8 轮模型请求 transport error，agent exit 1 | **架构不足：GAP-STREAM-RETRY** |
| make-doom-for-mips__qzanLHK（运行 2） | 0.0 | wallclock 1740s 到点，run_invalidated 终止 | **超时**（有效开发循环，任务难度 vs 预算） |

归因框架（用户裁决 2026-08-12）：除**解题方向错误**与**超时**外，其余失败均为**架构设计不足**。

## 3. 逐 Trial 行为重建

### 3.1 gpt2-codegolf__ySayq4F（首轮失败，无行为样本）

trajectory 仅 1 步 `run_failed: model error: transport error: error decoding response body`。
启动（session 初始化、web_search client 配置）正常后首个模型请求即流式中断。

### 3.2 make-doom-for-mips__uvrZze4（7 轮正常探索后失败）

7 步序列（全部真实执行）：

1. `list_dir(/app)` + `run_terminal_cmd`（并行批）——terminal 被拒
2. message "Terminal was denied — let me explore with the available tools first" +
   `read_file(vm.js)` + `list_dir(doomgeneric)` —— **拒绝后正确转向只读工具**
3. `grep("doomgeneric_img")` + `read_file(vm.js, offset=1000)`
4. `run_terminal_cmd` 再次尝试（被拒）+ `grep("DG_")`
5. `grep("DG_", glob=*)` + `read_file(doomgeneric_img.c)`
6. `list_dir(/)` + `list_dir(/tmp)` —— 环境全面探索
7. **transport error → run_failed**

**v1.5 正面证据**：terminal 被拒后模型 message 明确表达理解并转向只读工具；无盲目重试、
无盲改声称完成（B 组 9/9 全灭病灶未复现）。第 4 步的第二次 terminal 尝试属模型行为
（熔断轮级计数 2 轮 < 3 轮阈值，未触发为设计预期）。

### 3.3 gpt2-codegolf__Ad9CbFB（4 次 terminal 尝试后转向，第 8 轮失败）

7 步序列：

1. `list_dir(/app)` + `memory_search` + `search_tool`（首轮探索含无关检索，模型行为）
2. **并行 3 个 `run_terminal_cmd`**（ckpt 文件头探测/xxd/vocab.bpe 检查）——全拒
3. 再 1 个 `run_terminal_cmd`（`ls -la /app`）——拒
4. message "Terminal access is denied, so I'll inspect the checkpoint files directly
   with file reads..." + `read_file(.index)` + `read_file(vocab.bpe)` —— **自行转向**
5. `list_dir(gpt2-124M.ckpt)` + `read_file(ckpt)`（单文件 checkpoint 探测）
6. `grep("model/wte")` + `grep("wte")`
7. **transport error → run_failed**

**熔断查证**：`DENIAL_BREAKER_CONSECUTIVE=3`（orz-loop controller.rs）为**轮级**计数——
一轮内并行多次拒绝算一次。4 次工具尝试 = 连续 2 轮 < 3 阈值 → 熔断未注入
TOOL_POLICY_BREAKER 是**设计预期**；且模型在第 2 轮后自行转向（早于熔断兜底）。
deny 措辞（"denied by the permission gate for this call"）按 v1.5 用户裁决语义工作，
不承诺策略级不可用——**非架构缺陷**。

### 3.4 make-doom-for-mips__qzanLHK（73 轮有效推进后 wallclock 超时）

73 步 / 102 次工具调用：

| 工具 | 次数 | 工具 | 次数 |
|---|---|---|---|
| search_replace | 36 | run_terminal_cmd | 6 |
| read_file | 20 | list_dir | 4 |
| run_tests | 19 | search_tool / memory_search / blackboard_read / todo_write / ask_user_question | 各 1-2 |

- **有效开发循环**：step 18 起严格 search_replace↔run_tests 交替（改代码→测试→再改），
  36 次编辑 + 19 次测试为真实推进，非无效循环
- `ask_user_question` 走 fire-and-forget 返回（"UserQuestionSender not available; falling
  back to fire-and-forget QuestionsSent. This is expected during migration (TS-03 not
  yet wired)"）——**不阻塞** ✓
- 18:27:25 UTC `max-wallclock reached — recording run_invalidated terminal`，agent 正常
  结束（exit 0 语义，无 exception）
- **归因：超时**（MIPS 移植属 B 组最难档；73 轮 × ~24s/轮的平均节奏在 29 分钟内未完成
  属任务难度 vs 时间预算，不算架构不足）

## 4. GAP-STREAM-RETRY（本评测暴露的唯一架构缺口）

- **现象**：3 trial 死于 `transport error: error decoding response body`（async-openai fork
  流式响应解析错误）。运行 1 中两个并发容器首轮/第 4 轮各断一次；运行 2 中一个在第 8 轮断。
  make-doom 超时题全程 29 分钟零中断——中断随机分布，指向服务侧偶发窗口
- **历史基线**：8月11 全部 job（含 pro 对照 9/9）零此错误；8月8 出现 1 次（pre-existing 偶发）
- **排除项**：宿主 curl 非流式/流式单请求均正常（HTTP 200、SSE 格式正确）；orz 源码无此
  错误文本（第三方依赖）；与 v1.5 重构代码面（tool.rs/controller.rs/agent_loop.rs/prompt.rs）
  无关
- **机制**：ADR-0007 §2.1「流式路径不重试」——流中断即失败（防已产出 chunk 后重发造成
  重复工具调用）。本次暴露：**零 chunk 产出的中断同样不重试**，偶发中断即 agent 全灭
- **用户裁决（2026-08-12）**：流式中断需要能够重试
- **登记**：ADR-0007 §4「已登记缺口」+ 索引 GAP-STREAM-RETRY（`pending`）
- **边界**：仅**零 chunk 产出**中断可重试（握手/首字节前失败，重发同一请求体幂等）；
  **已产出 chunk** 中断保持不重试（重发会重复工具调用）
- **排期（用户裁决 2026-08-12）**：**ACAF Slice 1 之前**；暂不行动（登记待排期）

## 5. 附加观察（不阻塞，候选登记）

1. **可观测性**：trajectory（ATIF-v1.7）只记工具调用不记工具结果；run_tests/search_replace
   无 INFO 级日志——事后还原 run_tests 19 次成败不可行（本次靠行为模式推断"有效"）
2. **模型轮浪费**：gpt2 首轮调用 `search_tool("web search search engine http")` 等无关
   检索（离线任务）——模型行为，非架构

## 6. 结论

1. **v1.5 机制验证达成**：registry 全量目录 + 调用时逐次判定在 Benchmark 下按设计工作；
   工具调用链真实执行（read_file 正确拒绝二进制、grep 正常返回）；拒绝→转向行为健康；
   熔断机制按轮级语义工作
2. **唯一架构缺口 = GAP-STREAM-RETRY**：3/4 trial 死于同一缺口，修复（ACAF 前排期）
   将直接提升评测可靠性
3. **超时题**：任务难度边界，非架构问题；等待 GAP-STREAM-RETRY 落地后重跑补充样本

## 7. 证据文件

- `D:\tb-eval\jobs\2026-08-12__01-53-51\`（运行 1：gpt2-codegolf__ySayq4F / make-doom-for-mips__uvrZze4）
- `D:\tb-eval\jobs\2026-08-12__01-58-02\`（运行 2：gpt2-codegolf__Ad9CbFB / make-doom-for-mips__qzanLHK）
- 每 trial：`agent/orz.txt`（orz stderr）、`agent/trajectory.json`（ATIF-v1.7）、`exception.txt`、`result.json`
- 构建日志：`D:\tb-eval\build_log_20260812.txt`（orz-linux 重建，5m07s 增量）
