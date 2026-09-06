# 任务 D S3/S4 翻转实施审计（2026-09-06）

> **性质**：任务 D S3（registry 翻转 + 门禁改接 Rust 法官）与 S4（Python
> 判官退役/归档登记、契约同步、收口）同一实施批落盘。用户裁决（2026-09-06，
> [退役边界深挖](TASK_D_S3_S4_PYTHON_RETIREMENT_BOUNDARY_2026-09-06.md)）：
> ① 接线形态 = 独立 CLI；② 门禁期刊校验 v0.1 纳入（双轨 18 期刊不减）；
> ③ D-1 = **方案 α**（Python 模块转冻结 reference，对拍对照面保留）——
> D-2（258 测试保留）、D-3（`_WORK_TOOLS` 单源消解）随 α 定案。
> **关联**：[S2d 收口审计](TASK_D_S2D_CLOSURE_2026-09-06.md) /
> [batch-1 治理审计](P0_GOV_TASK_D_DUAL_IMPL_GOVERNANCE_2026-09-04.md)。

## 1. 变更清单

### 1.1 orz 子模块（本批实施提交）

- 新增 `crates/orz-assurance/src/bin/journal-conformance.rs`：独立 CLI 薄封装
  `journal::conformance::validate_journal_file(journal, repo_root)`——零判
  定逻辑自增，仅参数接线；exit 0=valid / 1=invalid（每错误一行
  `error: …` 于 stderr）/ 2=usage 错误。
- 新增 `crates/orz-assurance/tests/journal_conformance_cli.rs`（4 测试）：
  双轨真 fixture 正测（v0.1 + v0.2 各一）、64-hex 摘要篡改副本负测
  （exit 1 + stderr error 行 + INVALID 摘要行 + stdout 空）、usage 三负测
  （缺 `--repo-root` / 未知旗标 / 双期刊路径 → exit 2）、缺席期刊文件
  fail-closed（exit 1 + `journal file not found`，非崩溃）。
- 生产判官、schema、事件面、registry 读取逻辑**零改动**。

### 1.2 父仓库

| 文件 | 变更 |
|---|---|
| `runtime/run-event-payload-registry-v0.1.json` | `source` 字段改自指权威声明（**唯一权威**；Python 视图派生 + Rust 法官直读）；显式 LF 复写 |
| `assurance/run_event_journal_validation.py` | ① 头部 RETIREMENT STATUS 标注（frozen reference / 执法权在 Rust 法官 / 仅保留对拍对照面与 `_WORK_TOOLS` 单源两个合法消费角色）；② 两个硬编码 dict（被替换区间共 **218 行**——原文误记 225，2026-09-06 复审处理批更正）整体替换为 `_derive_payload_registry_views()`——模块导入时从 registry JSON 派生 `PAYLOAD_SCHEMA_BY_EVENT_TYPE(_V02)`，registry 缺失/轨不全/条目畸形一律 RuntimeError fail-loud |
| `scripts/check_repository.py` | ① 新增 `_rust_journal_conformance_errors()`：二进制解析序 = env `ORZ_JOURNAL_CONFORMANCE_BIN` → `orz/target/debug/journal-conformance[.exe]` → 兜底 `cargo build -p orz-assurance --bin journal-conformance`（1200s 超时）后重试；逐刊调用，exit 1 映射为门禁错误、非 0/1 退出码报 CLI 失败；② v0.1（6 期刊）与 v0.2（12 期刊）两个期刊循环改接该助手（**Python `validate_journal_file` 门禁调用摘除**）；③ registry 同步检查语义反转登记（JSON 为权威、dict 为派生视图，漂移=派生回归或手改回硬编码）；④ required 文件清单摘除导出脚本；⑤ 摘除 `validate_journal_file` 导入（保留 dict 导入供夹具映射） |
| `scripts/export_run_event_payload_registry.py` | **删除（退役）**——权威反转后失去存在意义；batch-1 治理审计 §3.1 死链同步加勘误注 |
| `architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md` | §9 变更流程整体改写为翻转后口径（registry JSON 第一动作 / Rust 法官 0 差硬约束 / Python reference 同步限于对拍镜像场景）+ §10 参考行状态更新 |

## 2. 关键验证证据（全部实测）

| 项 | 结果 |
|---|---|
| **dict 派生等值**：翻转前基线（slug+相对路径，v01=34/v02=27）vs 派生视图 | **IDENTICAL**（逐条目等值） |
| Python 判官三套件（judge 258 + conformance 15 + probe_audit 8） | **281 passed / 0 failed**（0.89s） |
| orz-assurance 全量（lib 204 + fixture 9 + **CLI 4** + fake_provider 9 + doctest 1） | 全绿 |
| `cargo check --workspace` | 零警告零错误；`cargo fmt --all --check` 净 |
| **门禁改接负测**：64-hex 篡改副本经 `_rust_journal_conformance_errors` | 篡改 = **9 错误**（event digest / manifest digest 链式暴露）；原刊 = 0 错误。*（2026-09-06 复审处理批勘误：9 为审计时点特定篡改——v0.2 `plain-run.jsonl` 首 64-hex 摘要翻转——的错误计数，非跨 fixture 常数；18 期刊同法定向篡改实测 3–70 错不等。负测的定性结论——篡改必拒（exit 1 + 链式 error 行 + INVALID）且原刊必过（exit 0）——与具体计数无关，已由 CLI 集成测试 `cli_rejects_tampered_copy_with_error_lines` 永久钉住。）* |
| 门禁（提交后复跑） | **Exit 0**（唯一前序失败 = orz 工作树未提交，提交后消解） |

## 3. 登记事项（不改代码）

1. **门禁新增 Rust 工具链依赖面**：改接后门禁期刊校验依赖 `journal-conformance`
   二进制（解析序见 §1.2，含 cargo 构建兜底与 `ORZ_JOURNAL_CONFORMANCE_BIN`
   覆盖通道）；门禁其余部分仍纯 Python。
2. **已知语义差异转移**：畸形期刊处理 Python 侧更严（parse 错误即早退、
   不报次级链错误）vs Rust 侧对已解析前缀继续链校验——裁决等值（两侧均
   invalid）、消息面不同；门禁语义自此以 Rust 侧为准（模块 docstring 原有
   注记保留作 provenance）。
3. **Python 侧退役边界（方案 α 落地形态）**：`run_event_journal_validation.py`
   保留在仓（reference 冻结、可导入）——合法消费角色仅两项（对拍对照面 +
   `_WORK_TOOLS` 单源）；258 个判官测试保留（Python 参照物健康度证据）；
   新增执法消费者被头部标注禁止。
4. **任务 D 闭合**：S2a/S2b/S2c/S2d/S3/S4 全部完成，任务 D（BACKLOG 00
   主体）按「登记不动计数」口径收口；双实现终局治理的终态 =
   **Rust 单一执法 + Python 冻结参照 + 对拍长期回归**。
5. *（2026-09-06 复审处理批补登）***registry JSON 信任边界（复审 B3）**：
   派生代码 `_derive_payload_registry_views` 对 `schema` 字段不拒绝对路径/
   `..` 越界（`ROOT / 绝对路径` 被绝对路径吞）——registry 为仓内受门禁
   同步守卫的受控文件、非模型可写面，风险接受、登记不改；如未来 registry
   变为外部可注入面，须先加 repo-root 内 containment 校验。
6. *（2026-09-06 复审处理批补登）***CLI 正测覆盖面（复审 B4）**：
   `journal_conformance_cli` 正测仅 2 fixture（双轨各一）——CLI 壳薄、
   18 期刊库级全量覆盖由 `fixture_journal_conformance` 承担，两层互补、
   登记不改。
7. *（2026-09-06 复审处理批补登）***usage 口径入文档（复审 B2）**：
   `--repo-root` 可重复后值生效、其余旗标/第二位置参数即 usage 错——行为
   已写入 `journal-conformance.rs` doc（orz `05f29fdc`）。
8. *（2026-09-06 复审处理批修复）***门禁 subprocess 编码（复审 B1，P1）**：
   两处 `subprocess.run` 的 `text=True`（默认本地 ANSI 编码）改显式
   `encoding="utf-8", errors="replace"`——期刊 payload 非 ASCII 内容进入
   判官错误消息时，非 UTF-8 机器上原实现抛 `UnicodeDecodeError`
   （ValueError，未被 `except (OSError, TimeoutExpired)` 捕获）致门禁崩溃；
   修复后中文错误消息经门禁助手无损返回（实测）。

## 4. 遗留

- 无任务 D 范围内遗留。批 2 复审 R6/R7/R9②③ 登记项不随本批变动。
- 权威收口表述见 BACKLOG 00 / TODO P0-GOV / 索引 `IMPL-PYTHON-REFERENCE`
  状态行更新（本批同步）。
