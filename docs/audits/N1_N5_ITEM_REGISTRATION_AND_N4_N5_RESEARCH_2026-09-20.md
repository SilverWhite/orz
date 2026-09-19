# N1–N5 立项与修复方案 ＋ N4／N5 业界调研（2026-09-20）

> 用户令：「除了 N6 以外，其他全部立项进行处理，N6 是日常用的设计，这不是什么需要处理的问题」；
> 并追问三事——N1/2/3 修复是否好处理、N5 无 fetch 目标是 grep 被拦还是 HTTP 形态本身的问题（若是后者要专门给 HTTP 形态做优化、
> 并问 GitHub 上有无相关项目）、N4 是否借鉴业界成熟的沙箱资源分配方式。
> 本批为**立项＋方案＋外部调研**：未闭合 **35 → 40**（新增 5 项）；**零代码、零子仓改动**；实施以实施令为准。
> 依据＝[`N1–N6 深挖`](0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md)（代码级根因＋一手读数）。

## §0 摘要

| 项 | 新条目 | 优先级 | 修复难度（主会话评估） | 一句话方案 |
|---|---|---|---|---|
| N1 逐 query 欠归因 | **0at** `GAP-RETRIEVAL-PER-QUERY-ATTRIBUTION` | P2 | **小→中**（B 面小、A 面中） | 先落「未归因计数」把失真显式化（小），再做归因谱系（中） |
| N2 run 尾部派发 | **0au** `GAP-RETRIEVAL-TAIL-DISPATCH-BUDGET` | P1 | **小** | 派发前加 run 余量判定＋尾部保留，复用 D3 未派发模板 |
| N3 倒数行无落盘面 | **0av** `GAP-RETRIEVAL-COUNTDOWN-AUDIT-FACE` | P2 | **小** | 机械审查面落一行 `retrieval_batch` 读数（journal 可核） |
| N4 资源门常态拒绝 | **0aw** `GAP-HOST-RESOURCE-ADMISSION-CALIBRATION` | P1 | **小→中**（判定参数化小、准入模型改造中） | 绝对余量＋基线校准＋分级降速；heavy 判定参数感知 |
| N5 检索形态无 URL | **0ax** `GAP-RETRIEVAL-HTTP-FORM-URL-INTEGRITY` | P1 | **小→中** | 官方口径下启用本地 SERP 车道／合成答案不计入可用＋fetch 侧指纹兜底 |

**N6 不立项**（用户裁决）：浏览器车道恒死属日常用法下的既有边界（官方 minimal 环境无浏览器、探针按"工具在位"语义记注），
子代理快速失败成本≈0，**不做处理**；本条只保留观察记录。

## §1 立项条目与方案

### 1.1 0at（N1）逐 query 归因：先"显式缺口"，再"归因谱系"

**根因**（深挖 §2）：`batch_close::per_query_usable_counts` 按**逐字 query 串**匹配 `EvidenceRecord::search_query`，
子代理自查串与之不同、派生证据无 query ⇒ 多 query 批 4/4 失真；且 `source_ledger` 不落归因依据。

**方案（两件可分笔）**：

1. **B 面（小，先做）**：`query_summary` 增 `unattributed_usable_count = 批级可用 − Σ 逐 query 可用`，
   并在检索子代理的车道披露里同步（与批级总数同源 helper）。收益＝主代理不再把"未归因"误读成"未覆盖"。
   契约面＝`runtime/retrieval-result-event-payload-v0.2.schema.json` 的 `query_entry` 增可选字段（按 0ar S1 "新增字段先立契约变更"先例），
   Python 冻结参照与 fixture 同批。
2. **A 面（中，随后）**：证据落**派发谱系**——`EvidenceRecord`／ledger 条目增 `origin_query_id`：
   子代理自查产生的证据归其所属派发 query（leader 谱系），派生证据（web_fetch/read_file）归"发起它的那次检索"。
   需要把 `query_id` 从派发层透传到证据装配层（`retrieval/dispatch.rs` → `evidence.rs`）。

**判据**：① 多 query 批 `Σ 逐 query 可用 + unattributed = 批级可用`（恒等式机械可核）；
② 归一化后逐 query 归因覆盖率 ≥ 90 %（同一 S3 语料回放）；③ 单 query 批 payload 逐字节不变。

**边界**：不动 FP-2；不改批级阈值语义（5／10 不变）。

### 1.2 0au（N2）run 尾部派发：派发前余量判定＋尾部保留

**根因**（深挖 §3）：派发预扫描只判合并上限 3，无 run 级墙钟余量项 ⇒ 四轮 5 次批被 run 墙钟截断（26–191 s）。

**方案**（落点 `agent_loop.rs` 派发前预扫描，与 D3 同一处）：

1. 计算 `remaining_run_wallclock`（run 级预算 − 已耗时）；
2. 若 `remaining < batch_wallclock + close_round_margin`（extended：300 s ＋ 一次收尾回合）⇒ 本批**不派发**，
   复用 D3 的"未派发"拒绝形态（无 `ToolStarted`、`stamp_failure(Refused)`、下一轮一次性重述），cause 另立
   `retrieval_dispatch_wallclock_reserved`；
3. **尾部保留**：距 run 墙钟 < 保留额（建议 90–120 s）时一律不派发新批，把尾部留给落盘；
4. 参数与档位表绑定（180／300／450），避免两套尺；`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS` 优先级不变。

**判据**：① 回放 S3／r1／r2 语料，`remaining < batch_wallclock` 的派发数 = 0；
② 新增拒绝形态在 journal 可见（无 `ToolStarted` 配对）且模型面文案只报事实；③ 不误伤"剩余充足但模型主动续派"的正常路径。

### 1.3 0av（N3）倒数行／β 注入块落盘面

**根因**（深挖 §4）：倒数行只入模型面消息；headless 路径侧车受 500 K 归档里程碑门（`ARCHIVE_INCREMENT_TOKENS`，ADR-0010 §14.68）
⇒ 未达里程碑时**结构性无落盘面**，判据 7 后段不可核。

**方案**（取深挖 §4.4 推荐项 1）：在既有 `mechanical_audit_update` 家族增 `kind="retrieval_batch"`，
payload 落 `activation_id／usable／cap／retrieval_calls／terminal_reason`（与 `batch_close` 单源 helper 同值）。
模型面零改动（不加说明性内容），journal 侧一格可核。

**可选项**（同一批或后续）：headless run 尾**无条件落侧车**（或 `ORZ_SESSION_SIDECAR_ALWAYS=1` 供评测轮），
使整条模型面消息（含倒数行、β 注入块）成为可审计面；代价是每 run 一份对话 JSON。

**判据**：① 任一真机 run 的 journal 可机械重算「倒数行读数 = 该批 `usable_source_count`」（判据 7 后段转为可核）；
② 不新增事件类型（复用 `mechanical_audit_update`）；③ 模型面字节不变。

### 1.4 0aw（N4）资源门校准

见 §3（含业界对照与四条改造方案）。

### 1.5 0ax（N5）检索形态 URL 完整性

见 §2（含根因归属、GitHub 项目清单与落法）。

## §2 N5：这是 **HTTP 形态本身**的问题（grep 被拦是次要加重项）

### 2.1 归属判定（三处一手证据）

| # | 证据 | 说明 |
|---|---|---|
| 1 | `web_search/client.rs::extract_citations` 内注释：**"xAI path: url_citation annotations (DeepSeek: empty)"** | 走 `responses + tools[web_search]` 时，引用只从 `url_citation` 注解取；**DeepSeek 侧恒为空** ⇒ 即便正文有内容也没有 URL 清单 |
| 2 | 官方跑批口径 `web_search local_segmented=off`（三 run 探针逐字） | 能产出 URL 的**本地 SERP 车道默认关**（`ORZ_WEB_SEARCH_LOCAL` 默认关闭，`local_segmented.rs` 文件头登记"默认关闭直至复验"） |
| 3 | S3 extract-elf 的 ledger：9/9 条 `web_search_result` **无 URL**；子代理自述"native search lane is returning synthesized answers rather than results with URLs" | 结果是**模型自答式合成文本**（provider 忽略 hosted 工具后的退化形态），不是"有 URL 但被反爬拦" |

⇒ **主因＝HTTP 形态**（provider 侧合成、无引用注解、本地 SERP 车道关闭）；
**次因＝fetch 侧反爬**（模型自行猜 URL 去取，`grep.app` 被 Vercel Security Checkpoint 拦、DuckDuckGo HTML 无正文）。

附：**同一台宿主**上本仓库的检索技能实测——原始 HTTP Bing 结果相关性 0/5、真实浏览器 4/4，
链路口径与官方跑批不同（技能侧默认带浏览器引擎与本地 SERP 车道），进一步说明"形态"是可控变量。

### 2.2 GitHub／开源生态可借鉴的项目（按层分类）

| 层 | 项目 | 作用（对 orz 的适配点） |
|---|---|---|
| **SERP 元搜索（自托管）** | [`searxng/searxng`](https://github.com/searxng/searxng)（官方文档 [docs.searxng.org](https://docs.searxng.org/)） | 聚合 200+ 引擎、返回**真 URL 列表**＋JSON API；可直接作为 orz 的 "provider 侧"（替代合成文本），或作为本地车道的一级引擎 |
| **SERP 直取（库级）** | [`deedy5/ddgs`](https://github.com/deedy5/ddgs)（原 `duckduckgo-search`） | 纯 HTTP 取 DDG 结果并结构化（title/url/snippet）；可作 `local_segmented` 的补充引擎实现参考 |
| **指纹层（反爬）** | [`lwthiker/curl-impersonate`](https://github.com/lwthiker/curl-impersonate)、`curl_cffi`（Python 绑定）、[`penumbra-x/rquest`](https://github.com/penumbra-x/rquest)／[`RockyZ/reqwest-impersonate`](https://github.com/RockyZ/reqwest-impersonate)（Rust 侧同族，另见 `wreq`） | 复刻浏览器 TLS/HTTP2 指纹（JA3/JA4）；对"原始 HTTP 0/5、浏览器 4/4"这类退化是**对症层**；orz 是 Rust 栈 ⇒ 优先评估 rquest／wreq 路线 |
| **正文提取（fetch 侧）** | [`unclecode/crawl4ai`](https://github.com/unclecode/crawl4ai)、[Firecrawl](https://github.com/firecrawl/)（原 `mendableai/firecrawl`）、`trafilatura`、`readability` | 把抓到的 HTML 转成 LLM 友好正文（分段＋URL 溯源），与 orz `web_fetch` 的"分段＋每段 URL"契约同形 |
| **Reader 服务** | Jina Reader（`r.jina.ai`） | 服务端渲染＋转 Markdown，绕开部分反爬；代价是外部依赖与隐私边界 |
| **浏览器自动化（兜底）** | `nodriver`／`patchright`／`browserless`、gVisor 生态外的 stealth 栈 | 官方跑批口径**不启用**（既有裁决：不在 orz 做容器内环境提前补强），仅在"另一参考系"里可选 |

### 2.3 orz 侧建议落法（0ax 方案骨架）

1. **形态修正（主）**：官方口径下评估**启用本地 SERP 车道**（`ORZ_WEB_SEARCH_LOCAL` 或等价路径），
   使检索结果天然带 URL（分段带 URL 是既有契约）——这是"HTTP 形态优化"的正面解法；若因可比性不能改口径，
   则退到第 2 条；
2. **口径修正（最小）**：机械层识别"**无 URL 合成答案**"并单列（不计入 5 条阈值、单列 `synthetic_answer` 计数），
   把"可用＝可引用"写进披露，避免阈值被不可引用的文本凑满（与 0at 的"可引用性"维度同源）；
3. **fetch 侧兜底**：对"已知需要抓取"的站点启用指纹伪装（Rust 侧 `rquest`／`wreq`）或 reader 服务，
   先覆盖"被判据要求的公共源"（如 raw.githubusercontent、文档站）。

**判据**：① 官方口径回放中"无 URL 结果占比"与"可引用来源数"可机械读数；
② 检索批的 `query_summary` 至少一条来源带 URL（或如实标注全为合成）；
③ 不新增工具面、不改 FP-2。

## §3 N4：借鉴业界沙箱资源分配方式

### 3.1 业界成熟做法（对照面）

| 体系 | 机制 | 对本案的启示 |
|---|---|---|
| **Linux cgroup v2**（[kernel doc](https://docs.kernel.org/admin-guide/cgroup-v2.html)） | `memory.max`＝硬上限（超则 OOM）、`memory.high`＝**软上限：超限即限流＋回收，不杀进程**；`memory.reclaim` 主动回收；`memory.pressure`／PSI 提供**真实压力**读数 | "先限流／回收，最后才拒绝"；判据用**压力读数**而不是静态百分比 |
| **systemd**（[systemd.resource-control](https://www.freedesktop.org/software/systemd/man/systemd.resource-control.html)） | `MemoryLow`（保护）／`MemoryHigh`（软限）／`MemoryMax`（硬限）三档 | 三档语义可直接映射 orz 的 watch／soft／hard 梯，但**动作应是降速而非拒绝** |
| **Kubernetes**（[node-pressure eviction](https://kubernetes.io/docs/concepts/scheduling-eviction/node-pressure-eviction/)、[QoS](https://kubernetes.io/docs/concepts/workloads/pods/pod-qos/)） | 三类 QoS（Guaranteed／Burstable／BestEffort）＋软阈值（`eviction-soft` 带宽限）＋硬阈值；按 QoS 决定驱逐顺序 | "**分级执行**＋软阈值给宽限窗口"；不同优先级工作负载不同待遇，而不是一刀切拒绝 |
| **Docker**（[resource constraints](https://docs.docker.com/engine/containers/resource_constraints/)） | `--memory` 硬限＋`--memory-reservation` **软限**（仅在内存紧张时生效） | 容器应被**限额约束**（limit-based），而不是靠宿主准入拒绝 |
| **Borg**（[paper](https://research.google/pubs/large-scale-cluster-management-at-google-with-borg/)） | 资源**回收（reclamation）＋超卖（overcommit）**、best-effort 任务、准入控制＋资源估算 | 高优先级任务保底、低优先级可回收；准入看的是"估算需求 vs 可用余量"，不是全局百分比 |

### 3.2 与 orz 现状的差距

| 维度 | 现状（`resource_gate.rs`） | 业界 |
|---|---|---|
| 判据 | **静态百分比**：commit headroom ≥ 25 % 才放行 heavy | 软／硬上限＋**压力读数**（PSI）＋绝对余量 |
| 动作 | **二值**：放行 or 拒绝（`Nothing was started.`） | 限流／回收／降速／宽限 → 最后才拒绝/驱逐 |
| 粒度 | 按**程序名**判 heavy（`tar --version` 也重活） | 按实际资源行为（cgroup 计量） |
| 环境耦合 | 百分比绑定宿主 commit 限额 ⇒ 本机（86–87 % 基线）**结构性不可满足** | 限额在容器/VM 层定义，准入看余量 |
| 回收 | 无 | `memory.reclaim`／驱逐／降级 |

### 3.3 改造方案（按落地成本排序，全部在 orz 侧、不动 task.toml／verifier／数据集）

1. **基线校准（小）**：run 起始测一次基线余量 `baseline_headroom`，放行门改为
   `headroom ≥ min(绝对下限, max(25 % × baseline_headroom, ε))`——消除"起跑即不可满足"的病态；
   绝对下限建议 256–512 MiB（够一次编译/解包）。
2. **分级执行替代拒绝（中）**：soft 档不再直接拒，改为**降速执行**（`nice`/`ionice`、限制并发、串行化重活）
   或短退避重试一次；hard 档才拒绝。对应 k8s 的 soft/hard eviction 与 cgroup `memory.high` 语义。
3. **heavy 判定参数感知＋引号感知分段（小）**：`--version`／`-t`／`-l`／`--dry-run` 等只读形态降级；
   `split_segments` 尊重引号状态（消掉 `grep -E "python|pip|conda"` 这类伪触发）。
4. **压力优先＋回收（中）**：能读到 PSI／`memory.pressure`（宿主 Windows 侧对应 commit 压力指标）时，
   判据改为"压力超阈才拦"，拦之前先做一次轻量回收（清缓存／等回收窗口）。

**判据**：① 同一 S3 语料回放：heavy 类 shell 的拒绝率从 **28/29** 降到"仅真实重活且真无余量"（目标 ≤ 1/3 且全部带可核压力读数）；
② 不出现"容器被 OOM 杀"或"任务墙钟因等待而恶化"（用 run 墙钟与 `host_resource_snapshot` 复核）；
③ 参数与官方环境口径解耦（只改 orz 侧判定，不改官方镜像/超时/数据集）。

### 3.4 追问评估：能否"接入资源管理器让它自己调度分配"（2026-09-20 实测）

用户追问：**能不能引入资源管理器、由它做调度分配（保活＋分配）**？实测三问三答：

| 问 | 实测（官方任务镜像 `alexgshaw/gpt2-codegolf:20251031`，按 harness 同参数 `--memory 8192m --cpus 1`） | 结论 |
|---|---|---|
| 容器有没有自己的资源管理？ | **有**：`/sys/fs/cgroup/memory.max = 8589934592`（8 GiB 硬限）、`cpus=1`；harness `docker.py` 按 `task.toml` 的 `memory_mb` 下 `--memory` | **运行时（Docker/harness）已经是资源管理器**——保活＝硬限＋OOM 保护，分配＝CPU/内存配额 |
| orz 能不能自己读写 cgroup 做分配？ | **不能**：`mount` 显示 `cgroup2 on /sys/fs/cgroup type cgroup2 (ro,…)`，`mkdir /sys/fs/cgroup/x` ⇒ `Read-only file system`；`cgroup.subtree_control` 为空（无委托） | 容器内**无法**自建子 cgroup、无法设 `memory.high`；要自管必须 privileged／显式委托 ⇒ 评测口径下**应否决** |
| orz 能不能读到压力信号做背压？ | **能**：`memory.current`、`memory.max`、`memory.high`（现为 `max`）、`memory.pressure`（PSI `some/full avg10/60/300`）与 `/proc/pressure/memory` 均可读 | **可用**：判据可从"宿主 commit 百分比"换成"容器限额内余量＋真实压力" |

**因此 0aw 的定案方向**（替代"orz 自管 cgroup"）：

1. **运行时当资源管理器**：容器硬限（`memory.max`）＋CPU 配额即"保活底座"，由 harness 按 `task.toml` 施加；
   若要软限（`memory.high`／`--memory-reservation`）也只能由 **harness 起容器时**设——**装置侧改动，需用户裁决**（会改变与官方 minimal 口径的可比性，默认不做）；
2. **orz 当限额内的调度器**：读 `memory.current|max` ＋ `memory.pressure` 判压力；可自主执行的动作只有
   进程级手段（作业队列并发=1、`nice`／`ionice`、`setrlimit`、进程树回收、退避重试）——即 §3.3 的第 ② 条；
3. **判据替换**：§3.3 的第 ① 条改为「容器限额内余量（`memory.max − memory.current`）＋ PSI 压力」，
   宿主 commit 百分比**降为观测读数**，不再单独作为拒绝依据。

**边界**：不引入 privileged 容器、不申请 cgroup 委托、不改 `task.toml`；0aw 的判据与实现落点以此为准。

## §4 N6 处置（用户裁决登记）

**不立项、不处理**：浏览器车道在官方 minimal 环境恒不可用属**日常用法下的既有边界**（工具在位≠可执行件在位；
快速失败零预算烧蚀，7 批约 4 个额外子代理回合）；既有裁决（不启用 `eval_browser`、不改任务镜像）维持。
探针"可执行件在位"位**不做**（用户裁决：不是需要处理的问题）。

## §5 落账

- 未闭合 **35 → 40**（新增 0at／0au／0av／0aw／0ax 五项；索引 §8 归 `pending` 桶，实施以实施令为准）。
- 三方同步：`CLI_PROJECT_INDEX.md`（v3.92 头行＋§6 条目＋§8 桶）／`docs/BACKLOG_AND_PRIORITIES.md`（P1／P2 开放项＋优先级总览＋新条目节）／`TODO.md`（P1／P2 路由＋勾选节）。
- 关联：深挖档 [`0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20`](0AR_S3_FRICTION_DEEP_DIVE_N1_N6_2026-09-20.md)、S3 复验档 [`0AR_S3_THREE_TASK_VERIFY_2026-09-20`](0AR_S3_THREE_TASK_VERIFY_2026-09-20.md)。

## §6 证据物

- 代码：`orz/crates/codegen/orz-tools/src/implementations/web_search/{client,local_segmented}.rs`（合成／引用／本地车道开关）、
  `orz/crates/orz-host/src/{resource_gate,acp_server}.rs`（25 % 门／侧车门槛）
- 一手读数：S3 三 run journal＋`retrieval-results` 归档件（见深挖档 §10）
- 外部参考：文内超链接（SearXNG／ddgs／curl-impersonate／crawl4ai／Firecrawl；kernel cgroup-v2／systemd／k8s／Docker／Borg）
