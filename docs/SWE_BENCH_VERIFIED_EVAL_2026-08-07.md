# SWE-bench Verified 评测交接文档（2026-08-07）

**自包含交接文档**——新窗口按此继续，勿重开讨论。配套 memory `fusion-phase-tracking.md`（2026-08-07 SWE-bench 条目，含完整细节）。

## 1. 当前成绩

**全量进展 = 16/16 通过（agent-feedback，F2P 官方清单判定）**：
- **hard5 + 冒烟题 6/6**（严格单次协议 5/6——django-14011 第 2 次才过，第 1 次真实失败方向错误）
- **剩余最难 10 题 10/10**（django ×6 + astropy ×2 + sphinx + pytest，2026-08-07 晚场）
- 严格单次协议合计 **15/16**（唯一重跑：astropy-13398，模型前三次尝试全部因 harness/环境问题未获测试反馈，第 4 次有效尝试即过）
- 本轮成本 **~¥6**（余额 73.02→67.26；hard5 阶段 ¥10，83→73.02）

| 实例 | 难度 | 判定 | 耗时 | patch | 备注 |
|---|---|---|---|---|---|
| pydata__xarray-6992 | >4h | ✅ 12/12 F2P | 40min | 4.8KB | index refactor（gold 同向） |
| pylint-dev__pylint-4551 | 1-4h | ✅ 10/10 | 32min | 9.4KB | get_annotation 系 |
| sphinx-doc__sphinx-7590 | >4h | ✅ 1/1 | 14min | 4.9KB | C++ UDL |
| sympy__sympy-13878 | >4h | ✅ 1/1 | 19.5min | 5KB | CDF 预计算 |
| django__django-14011 | 1-4h | ✅ 9/9（第 2 次） | 21.7min | 1.3KB | connections_override 构造器 + 线程关连接（与 gold 同向） |
| sympy__sympy-16597 | 1-4h | ✅ fixes=3 F2P 精确命中 | 9.5min | 1.2KB | 冒烟题（F2P 判据接入前，用 fixes/regression 判据） |

### 1b. 剩余最难 10 题成绩（2026-08-07 晚场，all10.json）

| 实例 | 难度 | 判定 | 耗时 | patch | 备注 |
|---|---|---|---|---|---|
| django__django-11400 | 1-4h | ✅ 3/3 | 16.0min | 9.1KB | |
| django__django-13128 | 1-4h | ✅ 7/7 | 15.6min | 2.1KB | patch < gold 3.4KB |
| django__django-13212 | 1-4h | ✅ 5/5 | 3.9min | 9.8KB | |
| django__django-15957 | 1-4h | ✅ 4/4 | 21.1min | 8.2KB | |
| django__django-16560 | 1-4h | ✅ 8/8 | 6.4min | 16.1KB | 本轮 F2P 最多 |
| django__django-16263 | 1-4h | ✅ 3/3 | 23.1min | 13.4KB | |
| astropy__astropy-14369 | 1-4h | ✅ 3/3 | 18.9min | 13.7KB | P2P 732 无回归 |
| astropy__astropy-13398 | 1-4h | ✅ 4/4 | 12.2min | 7.0KB | 第 4 次尝试（环境兜底后有效尝试即过） |
| sphinx-doc__sphinx-9461 | 1-4h | ✅ 3/3 | 15.1min | 6.9KB | |
| pytest-dev__pytest-5787 | 1-4h | ✅ 2/2 | 26.5min | 16.9KB | |

### 1c. harness 修复（2026-08-07 晚场，run_swebench.py）

1. **venv 并发创建竞争**（首轮崩溃根因）：parallel 2 下相邻同 repo 实例（13398/14369）并发 `python -m venv` 同一目录——`_VENV_LOCK` 全局锁 + 锁内重查 + `_create_venv_retry` 失败重试一次；`_finish_venv`（pytest 钉版/distutils/sitecustomize）提取为幂等收尾。
2. **结果持久化**（9 个 PASS 丢失根因）：results.jsonl 原只在进程正常收尾时写入——`save()` 提取为 merge+落盘，**每题完成即写盘**；崩溃不再丢已定结果。9 条丢失记录已用 `recover_results.py` 从运行日志重建（`rebuilt_from_log: True` 诚实标记，journal 副本完好）。
3. **ENV_FIXES 加 astropy**：`setuptools<70`（新版移除 `setuptools.dep_util`，2023-era setup.py 依赖）+ `numpy<2`。
4. **editable install 失败重试**：attempt 1 失败 → `apply_env_fixes` → attempt 2。
5. **PYTHONPATH 源码导入兜底**（astropy-13398 最终解法，14369 先例）：install 失败不再置空测试命令——降级 `PYTHONPATH=<worktree>` 源码导入模式，模型保有测试反馈；缺失依赖以收集错误/baseline 无效呈现而非静默盲改。**通用修复，24 题全量任何安装失败的 repo 受益**。
6. **apply_env_fixes 显式 venv 解释器**：裸 `pip` 经 PATH 解析可能打到系统 Python——`pip install` 前缀改写为 `{venv_python} -m pip install`（specs 经 shlex 拆，路径走 argv 防反斜杠被吃）。

已知残余：astropy 真 editable install 仍不可行（build isolation seed 新 setuptools 崩 setup.py；`--no-build-isolation` 后 extension_helpers/cython 装上仍 metadata 失败）——PYTHONPATH 兜底是稳定路径，Docker 官方评测阶段可再攻。

## 2. 未提交改动（用户手动推送惯例）

- **orz** `D:\CLI\orz\crates\orz-loop\src\controller.rs`：新增 `max_tool_rounds_override()`（读 `ORZ_MAX_TOOL_ROUNDS` env，`with_gateway` 构造时 `unwrap_or(MAX_TOOL_ROUNDS)`）。默认 40/ADR-0008 不动；评测场景 999。**orz-bin 已重建（12:52）含此改动**。测试：orz-loop lib 98 passed 全绿。
- **主仓** `CLI_PROJECT_INDEX.md` 更新行（含本评测概要 + 下一步）。

## 3. 环境（全部在 D:，C:/B: 不动——用户裁决）

```
D:\swebench-eval\
  repos\           8 个 mirror clone（django/sympy/sphinx/astropy/xarray/pylint/pytest/scikit-learn，autocrlf=false）
  instances\       每实例 git worktree（detached base_commit；重跑自动 reset --hard + clean）
  venvs\           每仓库 1 个 venv（pytest<8.2 钉版 + sitecustomize collections shim + .installed 标记）
  .hidden\         隐藏测试树（模型不可见）
  data\            verified_hard30.jsonl（30 题全字段含 F2P/P2P/environment_setup_commit）+ swebench_verified.parquet（500 行全量）
  journals\        各题 journal 副本
  results.jsonl    结果（合并式，instance_id 键）
  run_swebench.py  主 harness
  validate_baselines.py / check_all_staging.py / pick_hardest.py / fetch_verified.py / snapshot.py（辅助）
  hard5.json       最难 5 题 id 清单
```
- 数据源：`SWE-bench/SWE-bench_Verified`（HF 公开，datasets-server parquet）。privaZer 曾清掉 %TEMP% 原始数据——**一切重建到 D:，勿再依赖 %TEMP%**。
- Docker Desktop Linux 容器可用（29.6.2）；vhdx 在 `D:\DockerData\DockerDesktopWSL\disk\docker_data.vhdx`（4.7GB）——**无需迁移**。
- D: 剩余 15.4GB（Phase 3 镜像可能挤爆——按 repo 分组评测 + 组间 `docker image prune`）。

## 4. harness 用法

```powershell
cd D:\swebench-eval
python run_swebench.py --ids-file hard5.json --parallel 2      # 指定清单
python run_swebench.py --instance <instance_id> --parallel 1   # 单题
python run_swebench.py --exclude-results                       # 断点续跑（跳过 results.jsonl 已有）
python run_swebench.py --min-balance 10                        # 余额下限（默认 10，低于则中止）
```
关键机制（勿改坏）：
- **ORZ_TEST_RUNNER 无引号**（orz 按空白切分 + CreateProcess 直启，引号=os error 123）
- **patch 经 stdin 必须 bytes**（text=True 在 Windows 翻译 \r\n → git apply 全灭）
- 隐藏测试目标 = test_patch 触碰的全部测试文件（django 框架文件 `django/test/testcases.py` 除外——应用到 worktree 作测试基建，patch 提取前还原）
- `--rootdir=<hidden>` 隔离 worktree 配置；sphinx 因 `pytest_plugins` 在 tests/conftest.py 需 rootdir=hidden/tests
- 判定：可验证 F2P 全过 + 基线有效时零回归（收集错误/超时 → 基线无效跳过回归检查）。F2P 名称规范化剥离 `[param]` 与 ` (class.path)` 后缀；描述性条目忽略
- 计费门禁：ORZ_MAX_TOOL_ROUNDS=999 + 2h 超时 + 逐题余额检查

## 5. 环境修复表（已固化在 run_swebench.py ENV_FIXES）

| 仓库 | 修复 |
|---|---|
| xarray | numpy<2（np.unicode_ 移除）+ pandas>=2.1,<2.2（'M' 频率别名） |
| pylint | wrapt>=1.16（formatargspec）+ setuptools<81（pkg_resources） |
| sphinx | setuptools<81 + roman/contrib 包 + docutils<0.18 + **jinja2<3.1 最后装**（contrib 会拉回新版）+ 可编辑重装（9.x 移除 pytest11 entry point，必须 3.1.0 可编辑） |
| 全部 | sitecustomize（collections.Mapping 等 abc 别名 shim）+ pytest<8.2 |
| django | conftest 注入（DJANGO_SETTINGS_MODULE=test_sqlite + INSTALLED_APPS=contrib 前置 + 隐藏路径 `tests.<app>` 注册，ORZ_SWE_APPS env）+ 合成 `tests/__init__.py` + 框架文件 worktree 补丁 |

## 6. 遗留 / 下一步

1. **剩余 14 题全量**（单次协议、无重跑）：已过 16 题（hard5 6 + 本轮 10），剩余 14 题（30-16）：
   `python run_swebench.py --ids-file all14.json --parallel 2`（用 `pick_hardest.py` 同款排序从 verified_hard30.jsonl 排除 results.jsonl 已有 16 题取 14；或直接无参跑 30 用 `--exclude-results`）。预计 ¥5-8、3-5h。**harness 已修复**（venv 锁 + 每题落盘 + astropy ENV_FIXES + install 重试 + PYTHONPATH 兜底），崩溃/环境风险已大幅收敛。
2. **Phase 3 官方 Docker 评测**（正式分）：
   - swebench 官方包装到 **D: 的 venv**（勿动 C: 系统 Python——用户裁决）
   - F2P/P2P 已在 data/verified_hard30.jsonl；django-14011 **两个 patch 都评**（第 1 次 close_old_connections 版 + 第 2 次 connections_override 版），如实报告
   - 模型污染文件留意：django 两次各建空文件（`conftest.py` / `tests/__init__.py`）已含在 patch 中
   - 镜像空间：D: 15.4GB，按 repo 分组 + 组间 prune
3. **提交推送**（用户手动惯例；**用户裁决 2026-08-07：先不提交，下一步审查完再推**）：orz controller.rs + 主仓索引/本文档。

## 7. 首轮 0/5 复盘（防重蹈）

8 个暴露问题：**6 个环境**（django 0 收集=numpy/框架文件、xarray numpy2、pylint wrapt、sphinx pkg_resources/jinja2/插件、sympy collections.Mapping、pytest9 参数化）+ **2 个判定逻辑**（基线收集错误误判为干净基线；F2P 清单未接入）。xarray/pylint/sphinx/sympy 首轮模型**其实已成功**（after-run 有 380/16/25/63 测试通过）被误判 FAIL。**真实模型失败仅 2 处**：django 首轮方向错误、sympy-13878 首轮 4 个 XFAIL 回归。教训：**基线必须先可运行再谈判定**；run_tests 输出会泄漏隐藏路径（模型据此建了空 conftest——无害，记录）。
