# 261 批 0db S4 banking 轮起跑——r1-banking 容器轮在役＋启跑故障三件处置（2026-10-13）

> **批次**：261（前档＝[`260 批 S3 冒烟`](260_0DB_S3_SMOKE_2026-10-13.md)）。
> **日期**：2026-10-13。**用户令**（三条裁决）：①「对照臂不影响上榜吧？看 orz 臂的具体分值，如果需要归因模型和框架的话再说」＝**对照臂退役**，258 §4 五元组的对照臂条目按此更正；②「现在先只跑banking，其他的再加上成本太高」＝**S4 先只跑 banking 单域**；③百炼 key 明文提供（「只存了10块余额，应该够用，只要不泄露即可」）＋「调用名称应该是qwen3.7-text-embedding-flash」＝**embedder 模型名更正**（取代 256/258 计划的 text-embedding-v4）。
> **性质**：真机起跑批——banking 轮 `r1-banking` 在容器内 detached 运行中（97 题×4 trials＝388 sims）；零 orz 源码、零冻结树写入。
>
> **结论先行**：官方提交要求逐项核对全绿后起跑；启跑途中连续排掉三件基建故障（全部零源码：warm 硬编码表置空／knowledge extra 补装／ACAF fail-closed 关闭）＋embedder 保序分批（dashscope 批上限 20 实证）；**首题 task_001 收官 4/4 reward 1.0**（2189.5s，tau2 侧 $0.0077＝user sim、banking 无 judge）；语料索引一次落盘（698 docs）宿主共享。轮节奏实测 ≈36.5 min/题 ⇒ 全轮 ≈59h，checkpoint＋余额地板护栏在役。

---

## §1 用户裁决落账

1. **对照臂退役**：官方文档明文「You may submit results for a single domain; the leaderboard ranks submissions per domain」＋榜面收单臂——对照臂不影响上榜；verified 徽章只看轨迹完整性与公开校验（全轨迹落盘在案）。归因诉求等 orz 臂分值出来再议；摩擦发现照旧走错题解剖＋journal 取证（260 批讨论口径）。
2. **banking 单域先行**：其余三域（airline/retail/telecom，Windows 宿主 CLI 形态即可跑）成本再裁后另轮。
3. **embedder 模型名更正**：`qwen3.7-text-embedding-flash`（用户指定调用名）取代 256 §12.10/258 §4 的 `text-embedding-v4` 计划；活体验证＝dashscope compatible-mode 1024 维（28 tokens/句计费粒度）；`retrieval_config` 徽章申报值随 S4 报告定稿（自述串带真实模型名）。

## §2 官方要求逐项核对（起跑门，全绿）

| 官方要求（docs/leaderboard-submission.md） | 本轮形态 | 判 |
|---|---|---|
| 单域可提交（per-domain ranking） | banking only | ✓ |
| 一致模型配置（agent/user 同串同参跨域） | 五元组钉死＋results `info.agent_info/user_info` 实录 `deepseek/deepseek-flash` temp 0 | ✓ |
| 每域一结果、全题无过滤 | runner 顺序迭代全 97 题（--task-ids 仅冒烟用、主轮不带） | ✓ |
| 4+ trials、base split | `--num-trials 4 --task-split-name base` | ✓ |
| banking 必带 retrieval_config 徽章 | `--retrieval-config alltools`（官方默认形态）；results info 含 retrieval_config 字段 | ✓ |
| custom 型申报 | submission_type=custom＋methodology.notes（orz scaffold／自任 user sim／judge 内部化／embedder 后端更换）＋references；`modified_prompts=false`（标准 alltools 模板原样、仅换后端 env） | ✓（S4 报告时落 submission 面） |
| 成本记录（optional） | tau2 侧逐 sim cost＋chunk 台账 | ✓ |

## §3 容器轮架构（banking 专配）

- **容器**：`uv:python3.12-trixie-slim` 重建，旗标沿 259 §4 定版（`--cap-add SYS_ADMIN --cap-add NET_ADMIN --security-opt seccomp=unconfined`），挂载四处＝tau2-bench:ro／tau2-0db／.env:ro／orz-linux:ro；依赖重装（apt rg/bwrap/socat/nodejs＋npm srt@0.0.23＋`-e '.[knowledge]'` editable）；srt 冒烟复跑 PASS。
- **代际门**：`/orz-linux/orz` sha256＝`1ce40857…`（r3 真值逐位命中，0.9.0 Linux 载体）。
- **数据路径**：Windows junction 在容器内呈死链（译为 `/mnt/host/...` 不存在）——容器用独立 `TAU2_DATA_DIR=/tau2-0db/data-c`（容器内 symlink→冻结树 data/tau2 只读），runner 的收卷路径改认 `TAU2_DATA_DIR`（`sim_root()`）；宿主 junction 不动（Windows 侧三域他日复用）。embeddings cache 落 `<cwd>/data/.embeddings_cache`＝宿主可见，97 子进程共享。
- **orz 姿态**：原生工具**不授能**（tau2 agent 动作面应只有 tau2 工具；SCB 的全开姿态不适用于此）＋`ORZ_ACAF_FAIL_CLOSED=0`（见 §4-③）。

## §4 启跑故障三件（发现即修，全部零源码）＋一处垫片补强

1. **warm_kb_cache 硬编码表**：`batch.py` 起跑即以**函数内硬编码表**（`"alltools"→text-embedding-3-large`）调 `warm_kb_cache` 预热——非官方 embedder 打到 dashscope 必 404（首挂首题实证）。表在函数体内不可补丁，但 `warm_kb_cache` 为函数级 from-import（每次 run_tasks 重新绑定）——垫片置空源模块属性即整体跳过预热；语料嵌入走惰性路径（真管线改写后模型＋磁盘 per_doc 缓存），首次 task_001 现场建 698 docs 索引一次、落盘共享。
2. **rank_bm25 缺失**：BM25 管线需要 `rank-bm25`，在官方 `[project.optional-dependencies] knowledge` extra（`pip install -e '.[knowledge]'` 正式装法）——容器补装即愈（非 pin 树缺陷，是 extras 未装）。
3. **ACAF fail-closed 起跑强制**：Linux 载体在 run 起点默认强制 ACAF signer（无 ORZ_ALLOW_* 授能也强制；session/prompt 处 assurance invariant 拒起，首题 4×infrastructure_error 实证）。tau2 chat-only 姿态下原生工具未授能、ACAF 无物可执——`ORZ_ACAF_FAIL_CLOSED=0` 显式关闭（python 探针 init→session/new→prompt→end_turn 全通实证）；`TAU2_0DB_ORZ_ACAF=1` 可恢复载体默认（须自备 provision 三件）。
4. **embedder 保序分批（垫片补强）**：`OpenAIEmbedder.embed` 整批一把梭，dashscope compatible-mode embeddings 批量上限 **20**（400 报文实证）——垫片改写为保序分批（批 10，`TAU2_0DB_EMBED_BATCH` 可调）。

其余小补强：shim `ENV_FILE` 容器可覆写（`TAU2_0DB_ENV_FILE`）；runner 余额地板护栏（`TAU2_0DB_BALANCE_FLOOR`，默认 100 CNY，任务边界轮询触线即停）；`--extra-args`/`--task-ids`（260 批）沿用。

## §5 首题读数与轮现实

- **task_001**：4 trials 全 **reward 1.0**；2189.5s（≈36.5 min/题）；tau2 侧 $0.0077（user sim 消耗；banking 判分确定性为主无 judge；orz agent 侧在 DeepSeek 余额、chunk 收口记账）。
- **语料索引**：698 docs 一次落盘（`62eca725….pkl`，宿主 `D:\tb-eval\tau2-0db\data\.embeddings_cache\`），百炼消耗量级 ¥1 内（¥10 余额充裕）。
- **全轮排程现实**：≈36.5 min/题 × 97 ≈ **59h**（24h 连续跑口径内；detached＋任务级断点＋余额地板护栏）。50-sim chunk 首次收口预计在第 13 题前后，届时首份 chunk 实账（tokens/USD/RMB）入 `chunks.jsonl`。

## §6 监控与恢复交接面

- 宿主可见：`D:\tb-eval\tau2-0db\rounds\r1-banking\{round.log, task_ledger.jsonl, sims.jsonl, chunks.jsonl, logs\}`（容器内同构 `/tau2-0db/rounds/...`）＋收卷 `D:\tb-eval\tau2-0db\data-c\simulations\r1-banking\`。
- 恢复：`docker start tau2-0db-srt`＋重挂 runner（同一命令）——台账跳过已完成题，续跑断点在首未完成题。
- 停止权：容器内 `pkill -f 'run_[r]ound'`（括号防自匹配）。

## §7 边界

- 零 orz 源码、零冻结树写入；评测侧本地件改动清单（`D:\tb-eval\tau2-0db\`，不入仓）：shim（ENV_FILE 覆写／embedder 更名／warm 置空／分批 embed）、runner（sim_root 认 TAU2_DATA_DIR／余额地板／`import os`）、adapter（原生工具条件化／ACAF fail-closed 关闭）。
- 百炼 key 只落 `.env`（ORZ_BAILIAN_API_KEY），零落盘其余面；260 批隔离复证成立（chat 臂 `deepseek/` 前缀显式路由不吃 `OPENAI_BASE_URL`）。
- 计数不变 61；门禁 valid true；索引 v4.233 → v4.234；BACKLOG_2 §1.207。
