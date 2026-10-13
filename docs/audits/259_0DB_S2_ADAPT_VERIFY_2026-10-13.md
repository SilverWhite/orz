# 259 批 0db S2 适配核证——四件核证全过＋runner 三件套落码＋透传核对 12/12（2026-10-13）

> **批次**：259（前档＝[`258 批 S1 冻结`](258_0DB_S1_FREEZE_2026-10-13.md)）。
> **日期**：2026-10-13。**用户令**：「请开始进行S2」（排期 ①落账→②S1→③S2→④S3→⑤S4 之 ③）。
> **性质**：核证＋落码批——零 orz 源码、零冻结树写入（fc0055dc 三次 `git status --porcelain` 空）；API 消耗＝3 次微型核证调用（余额 328.17 CNY 两位小数不变）；评测侧本地件全落 `D:\tb-eval\tau2-0db\`（不入任何仓）。
>
> **结论先行**：S2 前置核证四件全过——①模型串 `deepseek/deepseek-flash` 活体 pong（推理先行模型实锤）；②judge CLI 覆盖旗标**不存在**（258 存疑实证落定）＋垫片覆写法真判通过；③`--retrieval-config-kwargs` 注入 embedder **被静默吞掉**（`**_extra`）＋垫片改写注册表法生效（零源码）；④srt 容器冒烟 7/7 PASS（docker 旗标 `--cap-add SYS_ADMIN --cap-add NET_ADMIN --security-opt seccomp=unconfined` 新发现）。runner 三件套（`tau2_shim.py`／`run_round.py`／`orz_agent.py`）落码并自测 4/4；透传核对 12/12 全过。**S3 前置清单三项**（百炼 key／orz 线缆活体校准／banking 容器旗标）随批登记。

---

## §1 核证① LiteLLM 模型串与 base_url 配置法（过）

- **静态**：pin 范围内 litellm（1.80.x）内置 `deepseek/deepseek-flash` 映射（provider=deepseek、max_input 1,000,000）；`openai/deepseek-flash` 未映射（get_model_info 异常）；裸名 `deepseek-flash` 亦解析但带 provider 前缀为提交纪律正形。endpoint 默认 `https://api.deepseek.com/beta`（litellm deepseek transformation），可由 env `DEEPSEEK_API_BASE` 或 `--agent-llm-args {"api_base":...}` 覆盖；key 读 env `DEEPSEEK_API_KEY`——垫片从 `D:\tb-eval\.env` 的 `ORZ_DEEPSEEK_API_KEY` 注入，与 orz 臂同源同 key。
- **活体**：经 `tau2.utils.llm_utils.generate` 全链路（tau2 Message→litellm completion→api.deepseek.com）temp 0 ping——回复 `pong`、finish=stop、**wire model 字段＝`deepseek-flash`**、成本由 litellm 正常计出。核证脚本 `verify/s2_verify_1_2.py`，结果 `verify/s2_verify_results.json`。
- **发现（如实登记）**：`deepseek-flash` 为**推理先行模型**——`reasoning_content` 在场、一词回复耗 49 completion tokens（48 推理＋1 正文）；首轮 ping 以 `max_tokens=8` 截断时 content 为空（8 tokens 全落推理）。tau2 运行不设 max_tokens 上限（默认 None）⇒ 生产面无截断风险；成本记账自然含推理 tokens（双侧臂同口径）；⑤元组读数解读时 latency/成本高于非推理模型属预期。

## §2 核证② judge 覆盖旗标实证（过——结论为否定＋垫片法成立）

- **静态**：`tau2/cli.py add_run_args` 全参数清单**无任何 NL judge 旗标**（有 --agent-llm/--user-llm/--retrieval-config/--review-model，无 --nl-assertions-*）；`config.py` 头注「overridable via CLI」对该字段不成立。judge 模型经 `evaluator_nl_assertions.py` **from-import 绑定**（import 时钉死，运行期必须改写 evaluator 模块属性而非 config 模块属性）。
- **活体**：垫片覆写 `tau2.evaluator.evaluator_nl_assertions.DEFAULT_LLM_NL_ASSERTIONS`＝`deepseek/deepseek-flash` 后，`NLAssertionsEvaluator.evaluate_nl_assertions` 对微型 mock 轨迹（2 消息×1 断言，故意不含 agent 回复）真判一次——判 `met=false` 且理由正确（「conversation only contains the user…no agent response」）＝**调用确实路由到改写后模型且判分行为正常**。judge＝DeepSeek 内部化以零源码改动成立；258 §6-2 的「须实证」闭合。

## §3 核证③ banking embedder 覆盖机制（过——CLI 注入不成立＋垫片法成立）

- **静态**：`resolve_variant()` 签名只收 `top_k/grep_top_k/case_sensitive/reranker_min_score` 四个具名覆盖，其余落 `**_extra` **静默吞掉**——`--retrieval-config-kwargs {"embedder_model": "text-embedding-v4"}` 注入**不成立**（活体调 `resolve_variant("alltools", embedder_model=...)` 复证，变体仍 `text-embedding-3-large`）。
- **垫片法**：`resolve_variant` 对注册表条目 **deepcopy 后改副本**⇒在 resolve 发生前原地改写 `RETRIEVAL_VARIANTS["alltools"].kb_search_dense.embedder_model`＝`text-embedding-v4` 即生效；实测改写生效／复原无损／resolve 不回污注册表三件全过。
- **徽章面**：`format_all_tools_dense_instructions` 的工具自述串随改写走——「uses **OpenAI API** with embedding model `text-embedding-v4`」＝256 §12.9「标准 variant 模板原样、只换 embedder 后端」口径成立，`modified_prompts=false` 可保。
- **端点面**：`OpenAIEmbedder`＝openai SDK（`OPENAI_API_KEY` env＋SDK 默认读 `OPENAI_BASE_URL`）⇒百炼改向＝env 两件（`OPENAI_BASE_URL`→dashscope compatible 端点＋百炼 key），零源码。**S3 前置**：百炼 key 尚未落 `.env`（256 §12.10 已定案两 key 清单；banking 冒烟前补位）；活体 embed 调用留 S3。

## §4 核证④ srt 容器面（过——冒烟 7/7 PASS＋容器旗标新发现）

- **静态**：`sandbox_manager._required_binaries()` 仅支持 linux/darwin——**win32 在 SandboxManager 构造即 raise**（256 批「源码级拒 win32→容器」实锤复核）；Linux 依赖四件＝`srt`（npm `@anthropic-ai/sandbox-runtime@0.0.23` pin，0.0.24+ 有 bwrap stub 回归）＋`rg`＋`bwrap`＋`socat`；缺件在构造时 loudly fail（不静默降级）。
- **容器冒烟**（`uv:python3.12-trixie-slim`＋apt 四件＋npm srt@0.0.23＋editable tau2；冻结树 ro 挂载）：`verify/s2_verify_4_sandbox.py` 七步全过＝构造依赖检查／KB 文档导出（INDEX.md）／`ls`／`grep` 命中／绝对路径逃逸词法拦截／**写拒绝＝srt 真强制**（`Read-only file system`，非 bwrap 崩溃假阳性）／清理。
- **容器旗标（本轮新发现，S4 banking 段起跑配置）**：docker 默认 seccomp 挡 bwrap 的 namespace 创建（`Creating new namespace failed: Operation not permitted`→需 `--cap-add SYS_ADMIN`）；加 SYS_ADMIN 后 bwrap 卡 loopback（`Failed RTM_NEWADDR`→需 `--cap-add NET_ADMIN`）。**定版旗标＝`docker run --cap-add SYS_ADMIN --cap-add NET_ADMIN --security-opt seccomp=unconfined …`**。另 apt 镜像瞬断一次（`--fix-missing` 重试过），长跑前宜预拉镜像预装依赖。

## §5 runner 三件套落码（§5 规格逐条落地，自测 4/4）

| 件 | 落点 | 对应规格 |
|---|---|---|
| **垫片启动器** | `tau2_shim.py` | judge 覆写＋banking embedder 改写＋orz 臂注册三件运行期覆写（§2/§3 法的工程化）；key 注入；`TAU2_0DB_SHIM_DEBUG=1` 实证三覆写在真实 argv 全中 |
| **轮次 runner** | `run_round.py` | §5-1..5-5 全条：manifest 驱动域序（airline→retail→telecom→banking 50/114/114/97）／题级台账去重断点（`task_ledger.jsonl` done 跳过）／逐 sim 提取入 `sims.jsonl`（reward/usage/cost 机械抽取）／限流退避（429/5xx 正则×指数退避＋抖动，≤3 次/题）／50-sim chunk 收口（`chunks.jsonl`：tokens/USD/RMB 余额/限流计数）／双臂（`--arm cli|orz`） |
| **orz 臂适配器** | `orz_agent.py` | τ² 工具面→orz（256 §12.4 裁决形态）：每仿真全新 `orz --stdio --real`＋全新 ACP 会话（get_init_state 起／stop 收——orchestrator 正常 finalize 与异常清理两路均调 stop）；线规沿 slop_code `_AcpProcess` 先例（newline JSON-RPC、permission allow 服务、session/cancel＋宽限） |

- **机制自测 4/4**：chunk 边界（49 不收口／50 收口，记账含余额轮询 328.17 实通）／断点跳过（done 留、failed 重跑）／manifest 域序计数（50/114/114/97＝375）／逐 sim 提取（reward/usage/cost 全对）。
- **数据面纪律（新识别）**：tau2 收卷落 `DATA_DIR/simulations/`，`DATA_DIR` 默认指向**源码树内 data/**——不设 `TAU2_DATA_DIR` 会写穿冻结树。落位＝`TAU2_DATA_DIR=D:\tb-eval\tau2-0db\data`＋junction `data\tau2`→冻结树 data\tau2（题源单一真值）＋sims 落 0db 工作区；四域装载实测 50/114/114/97 与 manifest 恰等。
- **dry-run**：375 tasks × 4 trials＝**1,500 sims** 与 258 §3 恰等。

## §6 透传核对（12/12 PASS，mock ACP 零二进制零 API）

`check_pass_through.py` 五组核对：
- **T1** 系统面前缀：orz 臂提示基底与 CLI 臂 `LLMAgent.system_prompt`（AGENT_INSTRUCTION＋policy）**逐字节相等**；转译层＝纯后缀（工具协议），256 §12.4「适配器层 vs orz 核心面」摩擦归因的分层面由此固定。
- **T2** 工具面：orz 臂提示内嵌 tools JSON 与 CLI 臂 litellm `tools` 参数（openai_schema 清单）规范化后逐项相等。
- **T3** 轮输入渲染：UserMessage／ToolMessage／MultiToolMessage 文本渲染保原文与 call_id。
- **T4** 回复映射：单 JSON／围栏包裹／前后杂音三类终文→`AssistantMessage`（message 或 tool_calls）全对；tool_calls 交 tau2 orchestrator 经环境执行＝**评测路径与 CLI 臂同一条**（无旁路桥，env 状态/动作检查不被绕开）。
- **T5** 注册装配：`orz_agent` 入 registry、`build_agent` 可实例化、首提示携带完整系统面、stop 收进程。
- **申报差异点（报表面）**：D1 tool_call id 合成（`orz_####`，判分无关）；D2 orz 载体模型串自持（S3 活体首验项，CLI 侧 wire model=deepseek-flash 已证）；D3 推理 tokens 双臂同记。

## §7 S3 前置清单（三项，S3 批首组动作）

1. **百炼 key 落 `.env`**（banking 冒烟前置；256 §12.10 两 key 定案的第二把）＋活体 embed 调用一次（`text-embedding-v4`@dashscope）。
2. **orz 线缆活体校准**：适配器终文收集以 `session/update → agent_message_chunk` 为主通道（形状按 ACP 标准假设，零活体样本）——S3 冒烟首验，若形状有差则修 `OrzTurnClient._service` 单点；journal 兜底沿 slop_code 先例备而未接。
3. **banking 容器形态**：§4 定版旗标；镜像预拉＋依赖预装（apt 瞬断教训）；`OPENAI_BASE_URL`/`OPENAI_API_KEY` env 双件与 chat 臂 `DEEPSEEK_API_KEY` 隔离共存（chat 侧模型串已带 `deepseek/` 前缀显式路由，不吃 `OPENAI_BASE_URL`——结构上已隔离，S3 复证一次）。

## §8 边界与台账

- 零 orz 源码、零冻结树写入（fc0055dc；`git status --porcelain` 三次空）；评测侧本地件 `D:\tb-eval\tau2-0db\`（venv/shim/runner/adapter/manifest/verify，不入仓）；API 消耗 3 次微型调用（余额 328.17 CNY 两位小数不变）；计数不变 61。
- **随批修缮（258 批遗留病灶）**：258 批落账 heredoc 把路径 `D:\tb-eval…` 的 `\t`/`\v` 转义成真 TAB/VT 落盘（BACKLOG_2 §1.204 两处路径＋TODO P1-0db S1 行落点＋System32\tar.exe 一处）；本批机械还原为字面反斜杠（七处，四卷一索引终检 TAB/VT 全零），语义零变更。
- 门禁 valid true（`scripts/check_repository.py`）；索引 v4.231 → v4.232；BACKLOG_2 §1.205。
