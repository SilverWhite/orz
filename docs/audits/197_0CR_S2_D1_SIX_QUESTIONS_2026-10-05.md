# 197 批：0cr S2 D1 跑批——官方轮前六题（cfgpipe／circuit_eval／code_search／dag_execution／database_migration／datagate）（2026-10-05）

> **用户令**：「请开始进行起跑吧」（196 批 S1 勘定后的 S2 起跑放行）。
> **性质**：纯真机跑批＋读数收取，零源码、零子仓改动；未提交；计数不变 **60**。
> **结论先行**：**D1 六题全部收官（harness exit 0 ×6，零装置性失败、零重跑）**——checkpoint
> solved 合计 **24/34**；任务级 cfgpipe／circuit_eval／code_search／datagate／database_migration
> 五题有解、Hard dag_execution 0/3 全档坍缩；总墙钟 **≈5h00m**（D1 排期 34 ckpt 载荷如期）；
> 摩擦面＝写控 **block 1**（`//` 根祖先臂，模型改道）＋warn 64 全不阻断＋权限/infra 全净；
> **16 streak fire 全带预测段**（最大 333B ≤400B）；0cs S3 面＝**六题零 NotFound 样本**（如实顺延）。

---

## §1 官方记分卡（D1 六题，k=1；checkpoint_solved＝该档 Core 全绿）

| # | 题 | 难度 | ckpt solved | 末档 core | 末档 full | 墙钟 | 模型步/工具调用 |
|---|---|---|---|---|---|---|---|
| 1 | cfgpipe | Easy | **6/6** | 3/3 | 206/216 | 55.6m | 82/139 |
| 2 | circuit_eval | Medium | **7/8** | 15/17 | 564/566 | 80.7m | 176/227 |
| 3 | code_search | Easy | **4/5** | 13/13 | 102/104 | 33.1m | 86/149 |
| 4 | dag_execution | Hard | **0/3** | 0/3 | 33/51 | 34.9m | 126/208 |
| 5 | database_migration | Medium | **3/5** | 1/3 | 123/137 | 39.6m | 132/174 |
| 6 | datagate | Easy | **4/7** | 16/16 | 382/405 | 56.0m | 156/214 |
| — | **D1 合计** | — | **24/34** | — | 1410/1479 | **≈5h00m** | 758/1111 |

- 逐档明细（core）：cfgpipe 4/4·3/3·4/4·7/7·6/6·3/3 全绿＝**任务级完美**；circuit_eval
  9/9·5/5·18/18·29/29·7/7·3/3·4/4·**15/17**（末档 2 core 失败）；code_search 7/7·5/5·
  **7/8**·14/14·13/13（ckpt3 单测试未翻回）；dag_execution 8/12→2/5→**0/3 单调坍缩**
  ＝基准设计所测退化形态的首个 D1 实样本；database_migration 4/4·3/3·3/3·**5/6**·**1/3**
  （后段走低）；datagate **3/4·7/9**·5/5·**10/12**·6/6·12/12·16/16（前三档失分、后三档回稳
  ＝**非单调恢复**形态）。
- 结果面如实记（152 批评测哲学：题目是找 bug 与不足的工具，分数仅作评判参考）；
  与 recli 四跑方差注记同口径，k=1 不外推。

## §2 摩擦台账（181 §4b 口径，D1 聚合）

- **写控**：审查 314 次＝allow 246／warn 64／**block 1**。**block 全文**＝dag_execution
  RUN-77e882a6-0 seq652，规则 `carrier-write`，目标 `//`——632 字符命令中路径构造塌缩为
  `//`，根祖先臂按「sweeping it would destroy protected host state」拦截，模型改道后续跑
  （**真阳性保守臂，非误拦**；`//` 归一化是否收窄＝**登记不轮内修**，留观察项）。
  warn 64 全为留痕不阻断，且明细多为**散文匹配假阳性**（`rc=$?`／`*args`／markdown
  `*seconds*` 等），不触模型面，零改道——沿 0cb「warn 全集留痕」设计口径如实记。
- **权限**：1111/1111 自动放行零拒绝；**infra 全零**（transport_retry／resource_denied／
  reclaim 均 0）；counterexample_gate 34（每档 1，正常）。
- **压缩**：22 次 `context_compressed`，实得比 0.324–0.812；mode＝model_selected 19＋
  context_scale_window 3（量尺开窗），零机械兜底强压。
- **RLI**：notice 316 条（spike 进入／迁移确认／coverage_gap／streak）；**streak fire 16 次
  全带预测段**（329–333B ≤400B 新预算）＝0am 面在 0.8.14 官方轮全程在役；域机以
  normal↔low_progress 振荡为主（与 188 批形态同族）。
- **工具错误**：全数模型侧或框架如实引导——exit_1 语义失败为绝对多数；特征样本＝
  datagate `&` 后台符被框架指引改 `is_background=true`（框架引导面按设计工作）、code_search
  `grep outside_workspace`（工作区门按设计拒绝）、dag_execution `search_replace` 缺
  `old_string`（参数形状错误如实拒绝）。**框架侧零干扰摩擦**。
- **磁盘**：D 盘 13G→12G（六题输出树＋语料，量级符合预估）；持续监控，<10G 触发当日清理。

## §3 同轮并收

- **0cs S3（工具名近似提示）**：六题 **`Tool not found` 事件 0 次**——幻影工具名面未出现，
  提示面（0.8.14 进体件）无真机样本可核。**如实登记**：S3 真机核证不闭合、继续随跑顺带
  （任意后续题目出现 NotFound 即收）；先前「随 0cr 首题顺带」的排期假设（首题即有样本）
  未兑现，改挂「随轮顺带直至首样本」。
- **0bz S4（脸面对账）**：六题 journal 全部落盘含 `face_fingerprint` 事件（工件在
  outputs 树）——离线对账（192 批形态：第 2 针归零／空跑分叉／塌陷读数）**顺延至 S3 收口
  窗或 D2+ 空隙统一收取**，零额外跑批。

## §4 执行形态实录

- 发射＝196 批 §4 定形逐字执行（`uv run slop-code run --agent orz --model
  deepseek/deepseek-v4-flash --problem <题>`，key 经 .env 注入，日志落
  `0cr_official/logs/<题>_run1_0cr.log`）；六题顺序＝字母序 ①–⑥ 与冻结 manifest 一致。
- 容器形态＝每题一 agent 容器全 checkpoint 连续（题间全新），评估按档另起评估容器——
  两种容器在 `docker ps` 中可区分，非异常。
- 跑批前四项前置门（磁盘/Docker/身份门 `fc990a8a…`/key）逐题抽查全过；载体 0.8.14 全程
  不换（单一快照口径）。
- 读数收取＝`scan_0cr.py`（196 批部署件）逐题出 `readings_<题>.txt`，本档数字全部机械
  提取自 evaluation.json＋journal 去重（RUN 按最大快照去重）。

## §5 台账

- 本档：`docs/audits/197_0CR_S2_D1_SIX_QUESTIONS_2026-10-05.md`。
- TODO：`P1-0cr` S2 行进度注记（6/36）；头部计数行指针（计数不变 60）。
- BACKLOG：本批记录指针＋计数行（60 不变）＋`0cr` 专节 S2 进度；第二卷 §1.144。
- 索引：头行 v4.168 → **v4.169**。
- 门禁 `check_repository.py` ⇒ **`valid: true`／error_count 0**（两次回缠：索引头行首版 1367>1200 字符帽，压缩后 1255 仍超，旧链 193–183 逐档链接收敛为单指针后 883 全绿——近期批 196/195/194 链接保留）。
- 仓外工件：六份 readings＋logs（`D:/tb-eval/scbench/0cr_official/`）、六题输出树
  （`outputs/deepseek-v4-flash/orz_just-solve_none_2026100{5T0111,5T0212,5T0341,5T0421,5T0502,5T0549}/`）。

## §6 边界

1. **零源码**：orz 子仓与父仓源面零改动；轮中零修复顺延（发现全部登记：`//` 臂观察项、
   warn 散文假阳性注记）。
2. **未推送**；D1 落账后 D2（⑦–⑫，10-07 默认锚）待用户放行。
3. 装置性失败零发生——失败语义条款（装置重跑／模型不重跑）本轮未触发。
4. dag_execution 0/3 与 recli 四跑 8/8 并存＝结果面双峰的又一对照点（Hard 池内部方差），
   如实记不作能力面结论（k=1）。

## §7 关键词

197 批、0cr S2、D1 六题、24/34、cfgpipe 6/6、circuit_eval 7/8、code_search 4/5、
dag_execution 0/3 坍缩、database_migration 3/5、datagate 4/7 回稳、写控 block 1 例 `//`
根祖先臂、warn 散文假阳性、16 fire 全带预测段、333B、0cs S3 零样本顺延、0bz S4 顺延 S3 窗、
5h00m、计数 60 不变。
