# 347 批：0dp 立项——审批缺省回归全自动与用户命令接口（2026-10-25）

> **用户令**（照录）：「是我的错，我没仔细看审查报告」「我需要自动审批，之前专门改的默认自动审批」「现在允许面大多都需要加参数，不行，日常和跑分要完全一致，全部默认开启，该开的全都开，让模型用个残废的orz完全没意义」「命令自动审批这点要给用户一个命令接口」「先立项不直接动手」。
>
> **范围**：立项登记＋ADR 转录（§14.91/v1.96）＋台账面。**零产品码**——落码随 0dp S2，载体重建随 S3。

## §0 结论

0dp 立项（64 → 65，`pending`，P1）。背景＝§14.90 条 3（Windows 交互缺省 confirm-first）随
0.9.12 进体后，用户实测确认与 2026-09-26「直接开 auto mode」原令意图冲突：日常使用被迫逐次
审批，或被迫以 Benchmark 轴旗子（`--allow-write --allow-shell --allow-network`）作日常通路——
后者使日常与跑分姿态分叉，不可接受。用户令三裁决＋ADR §14.91 即日转录（344 审查 P0 教训
「治理不得滞后于裁决」的就地应用，转录先于落码）。

## §1 裁决内容（ADR §14.91 转录同步）

1. **全缺省自动审批恢复**：`initial_yolo` 缺省 `true` 于全部车道；§14.90 条 3 的
   「Windows 交互缺省 confirm-first」缺省面被取代（该条 2026-10-23～10-25 记为过渡现行）。
   依据＝日常与跑分姿态完全一致、全部能力缺省开启。
2. **用户命令接口**：审批自动化给显式命令接口——形态 S1 勘定（候选＝`--yolo`／`--no-yolo`
   启动旗＋`ORZ_YOLO` env 单源（`default_initial_yolo` 单一源改造）＋
   `permission_mode_banner` 姿态反映；Web 工作台开关为可选增项随 S1 裁决）。既有
   `set_yolo_mode`／TUI「always allow」／Benchmark policy 面不变。
3. **不受影响**：§14.90 条 1（写面包含性）与条 2（`encoded-exec` warn）照旧现行——皆不弹审批。

## §2 批序与涉及面（S1→S3）

- **S1 设计定稿**：旗名／env 名／语义矩阵（缺省、`--yolo`、`--no-yolo` 三态）／banner 文案／
  Web 工作台开关是否入 S2。
- **S2 落码**：`default_initial_yolo` 单一源改造（env 覆盖）＋ orz-bin 旗解析（无值精确形态
  家规，`--allow-*` 先例）＋ banner 勘正（现文案「mode=interactive-yolo (default auto-approve)」
  在 `--no-yolo` 下失真）＋ **344 五契约测试按本条第三轮改写**（`*_follows_1490` →
  `*_follows_1491`：Windows 期望回归 0 审批、dead gateway 回归 yolo 自动放行）＋姿态钉改写＋
  orz-host 五处／README 双语姿态注释三更＋`lib.rs:685` banner doc 勘正。
- **S3 Windows 载体重建＋换装**：bump 0.9.13（343 先例：Windows-only 重建换装，Linux 载体与
  发行留下一自然节点）；备份/ACAF 重 provision/清单重生成/doctor/活体全门。用户自行重启
  （346 已登记偏好）。

## §3 过渡期注记（S3 前现状）

在役与已发行 0.9.12 载体仍为 §14.90 条 3 缺省（Windows 交互 confirm-first）。急用自动化者
过渡通路＝Benchmark 轴（`orz.exe web --real --allow-write --allow-shell --allow-network` 或
`ORZ_ALLOW_*` env）——确定性行、双平台一致，但与跑分缺省面分叉，仅作过桥不作日常终态；
S3 进体后回归零旗缺省，本注记随之失效。

## §4 台账

- 计数：64 → 65（`GAP-APPROVAL-AUTO-DEFAULT`，`pending`，P1）。
- 本批触点：ADR vol-14 §14.91（v1.96）＋入口页 v1.96 bullet＋BACKLOG（计数行/P1 行/开放项叙事/
  本批指针）＋TODO（计数行/P1 行/台账）＋索引（v4.315 头部＋条目行）＋本档。零源码、零跑批。
- 后续批序：0dp S1（设计定稿，待用户放行）→ S2 → S3。
