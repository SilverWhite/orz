# W2 批次 v4-pro 请求归属判定：orz 主车道无 pro 路由（2026-09-07）

> **触发**：用户查 DeepSeek 平台用量见 `deepseek-v4-pro` 9/7 有 4 次请求
> （13,917 tokens：输入未命中 596 / 输出 13,321），疑 VM 内 orz 主车道
> 路由到 pro。本判定基于 chunk1 journal（w2-chunk1-031）+ orz 0.3.1 源码
> + runner 注入链三方证据；chunk2 收尾后补全量对账。
> **结论**：orz 主车道 = `deepseek-v4-flash`，无任何 pro 路由；v4-pro
> 请求全部来自**模型在任务内用自己的工具命令、持同一 API key 点名调用**
> （知识神谕行为），非 harness/orz 路由。

## 1. 证据链（三方互证）

1. **源码**（orz `d21b883e`，0.3.1）：主车道模型解析唯一入口
   `orz-loop/src/gateway/transport.rs:69-75`——`ORZ_MAIN_AGENT_MODEL`
   env 覆盖，缺省常量 `MAIN_AGENT_MODEL = "deepseek-v4-flash"`；全仓
   grep 无 `deepseek-v4-pro` 代码路径（仅 transport.rs 单测用 pro 字符串
   验证 env 覆盖机制）。web_search 客户端硬编码 v4-flash
   (`orz-tools/src/implementations/web_search/client.rs:973/1027/1055`)。
2. **runner 注入**：`run_agent_arm.ps1` env-file 三键
   `ORZ_DEEPSEEK_API_KEY` / `ORZ_MAIN_AGENT_MODEL=deepseek-v4-flash`
   （param 默认）/ `ORZ_F6_PUSH=on`；control 臂同名进程 env。三任务
   journal 共享同一 `request_header_change.header_sha256`
   （`7d85136f…`，配置同形）。
3. **流量形态**：平台 9/7 v4-flash 583 次/17.2M tokens（主车道数百轮
   量级）vs v4-pro 4 次/13.9K tokens（输出占 96%，问答形态）。

## 2. v4-pro 4 次请求的归属（chunk1 实证）

mteb-leaderboard 任务内（本地时间 16:14，对上平台 9/7 峰），模型以
terminal 工具持 `$env:ORZ_DEEPSEEK_API_KEY` 自调 API：

- 16:14:10 `Invoke-WebRequest api.deepseek.com/models`（连通探测，
  不计模型）
- 16:14:17 chat 请求显式 `model='deepseek-v4-pro'`（问嵌入模型选型，
  89.6s）
- 16:16:22 第二条排行榜知识提问（150.9s）

两条 chat 请求 + 可能的重试/拆分与平台计数 4 次吻合；输出 13.3K tokens
与两条长答案形态一致。make-doom/gcode 无 API 自调（探测全打 github/
ziglang 等被墙目标）。9/4 的 v4-pro 单峰早于本批（9/7 启动），不在本批
证据链内。

## 3. 残余问题定性与方法学影响

- **性质**：不是路由缺陷，是**评测方法学条件**——high-nist 墙 egress
  只放行 DeepSeek 端点（orz 自身需要），任务又是 allow-network 形态，
  故模型可把同 key 的更强模型（v4-pro）当知识神谕。mteb 答案
  （Qwen/Qwen3-Embedding-8B，≠0303 参数化路径的 Snowflake）即此通道
  产物。
- **影响**：对本批（摩擦/事件面诊断）无判分影响；对将来 TB2.1 官方
  计分跑批，属需显式声明的跑批条件（模型可用同厂商 API 自助）或需
  收紧的口径。
- **防除路径**（登记，批次源冻结期不实施）：key 对工具子进程不可见化
  ——orz 侧在 shell 子进程 env 剥离 `ORZ_DEEPSEEK_API_KEY`（key 收敛
  host 侧代理或启动期读取后从 env 摘除）；属 orz 设计改动，冻结解除后
  立项。harness 侧无可行拦截（key 必须进 orz 进程 env，子进程按 OS
  语义继承）。

## 4. chunk2 补充对账（闭环，2026-09-07 晚）

chunk2 三任务（path-tracing / train-fasttext /
adaptive-rejection-sampler）journal 的 tool_calls **零**
`api.deepseek.com` / `v4-pro` 引用——平台 9/7 的 4 次 v4-pro 请求
**全部归属 chunk1 mteb 任务内模型自调**（16:14 两条 chat 显式
`model='deepseek-v4-pro'`，含重试/拆分与平台计数吻合）。9/4 单峰早于
本批启动，不在本批证据链。对账闭环：orz 主车道 = v4-flash，无 pro
路由；pro 用量 = 模型任务内知识神谕行为。
