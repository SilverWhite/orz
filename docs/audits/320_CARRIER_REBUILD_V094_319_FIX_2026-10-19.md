# 320 批：0.9.4 双平台载体重建——319 修复进体（2026-10-19）

> **用户令**：「请进行重建吧，不过目前D盘又红了，估计还是要先清理一波」。**本批**＝0.9.3→0.9.4
> 代窗口双平台载体重建（沿 247/280/289/308 批形态：清理保空间→版本 bump→源冻结→双平台重建换装→
> 进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**源冻结**＝orz **`5ee818d3`**
> （0.9.3→0.9.4 bump；进件＝319 审查修复 `41534aee`＋Web `/rewind` 命令表 `995a7345`＋
> smoke queue_depth `c1d3bd53`＋Windows 8.3 测试 `7516e160`；`cargo metadata --locked` exit 0）。
> **两线 S4 真机核证待续**；**未推送未发行**。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 磁盘清理 | D: 清理前 **6.3 GB free（2.3%）**；删 `target/debug/incremental`（15.81 GB，纯增量缓存）＋`target/release-dist`（7.57 GB，release-dist profile 旧产物）＋根目录 `.tmp-*` 残件（743 个 / 10.8 MB）；**释放 20.72 GB → 26.99 GB free**（构建结束 26.4 GB） |
| 版本 bump | `crates/orz-bin/Cargo.toml`＋`Cargo.lock` 恰两行 0.9.3→0.9.4；`cargo metadata --locked` exit 0；orz 提交 `5ee818d3`；父仓 pin＋manifest 提交 `c244495c` |
| Windows 重建 | `cargo build -p orz-bin --release -j 2` **exit 0（4m56s 暖缓存）**；换装 **MATCH 3/3**；`.0.9.3-bak` 链预建（前件与 308 账面 `dabaaf7e…`/`b597cbb0…`/`fd7f92d5…` 逐位吻合）；`--build-info`＝`0.9.4 os=windows`；ACAF 重 provision **exit 0**（`binary_sha256=c7f5aa2f…`↔换装位 signer 逐位一致；旧 manifest 留 `bak-20261019-320`）；清单刷新 v0.9.4（3 entries） |
| Linux musl | docker `rust:1.97-slim`＋trixie 脚本 **exit 0（cargo `Finished … 20m28s`；容器墙钟≈21 分）**；`.0.9.3-bak` 宿主预建（前件与 308 账面 `4d7917c3…`/`00a16266…`/`8fc0b65b…` 逐位吻合）；**直写换装位**；**static-pie×3**（type=DYN、无 PT_INTERP、`.symtab` 在＝未 strip）；alpine 3.20／bookworm-slim 双冒烟 `0.9.4 os=linux` rc0；Linux 不重 provision（沿同口径）；清单刷新 v0.9.4（3 entries） |
| 进体字节判据 | 319 新面 **10/10 双侧进体**（维护令牌键/维护子命令文案/会话卷祖先规则/xargs 递归删除文案/维护 CLI 拦截文案/租约冲突文案/`session/load` 回放锚/cwd 门/补充延迟文案/run 尾延迟文案）；保留面 **9/9 逐项持平**（0di 三面、0cy schema、0df 计量、ACAF 规则、检索探针、写控、灾难删除）；版本面 `0.9.4` 双侧 0→1、`0.9.3` 残留 WIN 7→6／LIN 8→7（载体版本字面替换，非回归） |
| 冒烟 | WIN scratch `D:/tb-eval/smoke-320` 三轮 `--fake-provider -p` rc0（`RUN-CLI-6ad62f5b`／`6ad62f60`／`6ad62ffe`）；**活体**：seq2 `tool_availability_check{retrieval_enabled:true}`＋seq3 `run_started` 双锚（`failure_pipeline:"funnel-v1"`＋`usage_metering:"run-v1"`）＋seq9 `run_usage_metered` 九字段恰终态前一条＋哈希链连续（11 events）＋0cy 收卷链（`ARC-6ad62f5b-1` 回执＋`archives/6ad62f5b.json.gz`＋源目录删除）；**0dl 活体门**：无令牌 `delete-session` → exit 2＋明确拒绝文案；16 字符令牌 → 越过门后报「无数据可删除」exit 1 |
| Linux 冒烟 | `D:/tb-eval/smoke-320-lin`：alpine `6ad63513`→`6ad6351d`（swept `6ad63513`）→`6ad63525`（swept `6ad6351d`）→bookworm `6ad63529`（swept `6ad63525`）；四轮 rc0＋三份归档包（3299/3308/3306 B）＋ARC 回执三份；journal 11 events 与 WIN 同构；清单刷新后**无 carrier-integrity 警告** |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `4d7917c3…`→**`aaa68d4b…`**；py_compile 过＋`--help` rc0 |
| 构建告警 | WIN 1＝orz-host permission.rs `unused_imports`（287 在案预存）；LIN 8＝orz-config 1＋orz-assurance 4＋orz-host 3（musl cfg 门控预存 lint；308 同口径 8）——非本批引入 |
| 门禁 | `check_repository.py` **valid:true / error_count:0**（warning 4＝既知 allowlist 证据类）；`orz_source_manifest.sha256` 1488 条 `--check` valid |

## §1 磁盘清理（构建前置）

1. **读数**：D: 276.6 GB 中仅 **6.3 GB free（2.3%）**；`orz/target` 独占 86.35 GB
   （debug 68.67＝deps 48.79＋incremental 15.81；release 10.06；release-dist 7.57）。
2. **删除**（先解析绝对路径并确认前缀 `D:\CLI\orz\target\*`）：
   - `target/debug/incremental`：**15.81 GB**——纯增量编译缓存，cargo 自动再生。
   - `target/release-dist`：**7.57 GB**——`release-dist` profile 的旧产物（10-19 20:04 的
     实验构建，非载体口径；载体走 `--release`→`target/release`），仓内无脚本引用。
   - `D:\CLI\.tmp-*`：743 个 / **10.8 MB**——历批 gitignored 临时残件。
   - **合计释放 20.72 GB**，free 6.3 → **26.99 GB**；未动 `target/release`（0.9.3 暖缓存）、
     `D:/tb-eval/orz-target`（Linux 构建缓存 8.6 GB）、`D:/tb-eval` 评测数据与 bak 链。
3. **保留**：pagefile（14 GB）、WSL/Docker vhdx、Android/steam 等用户数据；回收站仅 1 文件 0 GB。

## §2 双平台三件套（终态）

| 平台 | 文件 | 尺寸 (B)（0.9.3→0.9.4） | SHA256 |
|---|---|---|---|
| WIN | `orz.exe` | 58,703,360（+169,472） | `f0bb5d92dd5b74d2…` |
| WIN | `orz-signer.exe` | 6,740,480（Δ0） | `c7f5aa2fb34c9f03…`（provision 新值） |
| WIN | `orz-acaf-provision.exe` | 6,640,128（Δ0） | `478fcce72ff40a05…` |
| LIN | `orz` | 117,597,120（+192,888） | **`aaa68d4b529bee84…`（身份门新值）** |
| LIN | `orz-signer` | 1,397,984（−16） | `fdc3273e77f4f13d…` |
| LIN | `orz-acaf-provision` | 1,216,400（−48） | `95523f60cf5939f2…` |

（换装基线＝0.9.3 三件 `dabaaf7e…`/`b597cbb0…`/`fd7f92d5…`（WIN）与
`4d7917c3…`/`00a16266…`/`8fc0b65b…`（LIN），与 308 批账面逐位吻合；
`.0.9.3-bak` 双侧六件预建在案。）

## §3 进体字节判据（方法与读数）

**方法**：python 字节直数（`bytes.count`；243/247/280/289/308 同法），基线＝换装位
`.0.9.3-bak` 预建件；脚本 `D:/tb-eval/byte_criteria_320.py`（评测区本地面，不入仓）。

| 判据 | WIN（0.9.3→0.9.4） | LIN（0.9.3→0.9.4） |
|---|---|---|
| 319：`ORZ_MAINTENANCE_TICKET` | 0 → **3** | 0 → **3** |
| 319：`是维护子命令，需要桥注入的` | 0 → **1** | 0 → **1** |
| 319：`carrier:session-volume-ancestor` | 0 → **1** | 0 → **1** |
| 319：`pipes unbounded paths into a recursive delete` | 0 → **1** | 0 → **1** |
| 319：`invokes the bridge-only maintenance CLI` | 0 → **3** | 0 → **3** |
| 319：`is attached to another orz process (lease` | 0 → **1** | 0 → **1** |
| 319：`session/load history replayed` | 0 → **1** | 0 → **1** |
| 319：`session/load requires an absolute cwd` | 0 → **2** | 0 → **1** |
| 319：`supplement delivery deferred` | 0 → **14** | 0 → **14** |
| 319：`run-scoped carry queue`（run 尾延迟） | 0 → **7** | 0 → **7** |
| 保留：`supplement_delivered`（serde 名） | 4 → 4 | 4 → 4 |
| 保留：`_orz/queue_append`／`recall_log` | 1／6 → 1／6 | 1／6 → 1／6 |
| 保留：`session-archive-package-v0.3` | 1 → 1 | 1 → 1 |
| 保留：`run_usage_metered` | 2 → 2 | 2 → 2 |
| 保留：`carrier:keystore-root`／`carrier-write` | 1／1 → 1／1 | 1／1 → 1／1 |
| 保留：`catastrophic-recursive-delete` | 1 → 1 | 1 → 1 |
| 保留：`tool_availability_check` | 4 → 4 | 4 → 4 |
| 版本面：`0.9.4` | 0 → **1** | 0 → **1** |
| 版本残留：`0.9.3`（载体版本字面替换） | 7 → 6 | 8 → 7 |

**判读**：319 的十项新面**双侧全数进体**；九项 0.9.3 保留面**逐项持平零回归**；
`0.9.4` 进体、`0.9.3` 残留各减 1（载体版本字面替换，280/289/308 同形非回归），
机械旗 `ALL-OK`（无 `MISSING`/`DROPPED`）。

**已知盲区（如实记）**：`defer_run_end` 在 WIN 侧连续字节计数为 0——反汇编为
imm64 立即数拆分（`mov rcx,"_run_end"`＋`mov rcx,"defer_ru"`），即
`check_carrier_literals.py` 头注所记「短字面可被 LLVM 内联拆开」；判据改用同块
长文案 `run-scoped carry queue`（7 次）承载，语义面在体由长文案＋`supplement_delivery_budget`
联合核证。`supplement_delivery_budget` 等键名在两侧均为连续字节可见。

## §4 冒烟与活体核证

**Windows（`D:/tb-eval/smoke-320`，`--fake-provider -p`）**

- round1 `RUN-CLI-6ad62f5b` rc=0（启动 sweep 走掉此前 `--help` 探针遗留的 `6ad62f58`）。
- round2 `RUN-CLI-6ad62f60` rc=0：`gsa hygiene sweep: swept ["6ad62f5b"]`→
  `archives/6ad62f5b.json.gz`（3322 B）＋`ARC-6ad62f5b-1` 回执（0cy 收卷链在体）。
- round3 `RUN-CLI-6ad62ffe` rc=0：清单刷新后运行，**无 carrier-integrity 警告**。
- **活体 journal**（11 events，sequence 0–10 连续）：`run_preflight`／
  `tool_availability_check`×2（`retrieval_enabled:true`，286 缺省开）／`run_started`
  双锚（`failure_pipeline:"funnel-v1"`＋`usage_metering:"run-v1"`）／`prompt_submitted`／
  `request_header_change`／`face_fingerprint`／`model_output`／`host_resource_snapshot`／
  `run_usage_metered`（九字段：`cache_hit_tokens`/`cache_miss_tokens`/`completion_tokens`/
  `effort_tier`/`model`/`model_calls`/`prompt_tokens`/`reasoning_tokens`/`usage_incomplete`，
  index 9）／`run_finished`（index 10）；`previous_event_sha256` 链逐条连续。
- **0dl 活体门**（新面）：

  ```text
  orz delete-session deadbeef
  → error: `orz delete-session` 是维护子命令，需要桥注入的 ORZ_MAINTENANCE_TICKET
    能力令牌（长度 ≥ 16）；未授权调用被拒绝        [exit 2]
  ORZ_MAINTENANCE_TICKET=0123456789abcdef orz delete-session deadbeef
  → error: 会话 deadbeef 在本工作区无数据可删除      [exit 1]
  ```

  即**鉴权门先于存在性检查**，未授权调用绝不触达删除路径；令牌长度下限 16 生效。

**Linux（`D:/tb-eval/smoke-320-lin`，两镜像）**

- alpine 3.20：`6ad63513`（首轮，清单尚未刷新→仅此轮出现 carrier-integrity 长度差异警告，
  属预期中间态）→ `6ad6351d`（swept `6ad63513`）→ `6ad63525`（swept `6ad6351d`）；
  bookworm-slim：`6ad63529`（swept `6ad63525`）；四轮 rc0，归档 3299/3308/3306 B，
  `ARC-*-1` 回执三份；清单刷新后**无 carrier-integrity 警告**。
- `6ad63529` journal 11 events 与 WIN 同构（双锚＋`run_usage_metered` index 9 恰在
  `run_finished` index 10 前）。
- `--build-info` 两镜像均 `version=0.9.4 os=linux arch=x86_64 profile=release`。
- Linux 侧 ACAF 采用**容器内 /tmp 临时 provision**（plain-file keystore）供冒烟，
  **不改载体位 `acaf` 面**——沿 308「Linux 不重 provision」口径。

## §5 台账

- 本档：`docs/audits/320_CARRIER_REBUILD_V094_319_FIX_2026-10-19.md`。
- orz 子仓：bump 提交 `5ee818d3`（`Cargo.toml`＋`Cargo.lock` 两行）。
- 父仓：`c244495c`（pin orz `5ee818d3`＋`orz_source_manifest.sha256` 1488 条重算）。
- 身份门：`scripts/run_r0_heavy_official.py` 载体锁定值换装（py_compile＋`--help` rc0）。
- 换装位清单：`D:/tb-eval/orz-windows/carrier-manifest.json`／`D:/tb-eval/orz-linux/carrier-manifest.json`
  刷新 0.9.4（`generate_carrier_manifest.py`，各 3 entries）。
- 字节判据脚本：`D:/tb-eval/byte_criteria_320.py`（评测区本地面，不入仓）。
- 构建日志：`D:/tb-eval/buildlogs/win_094_build2.log`／`lin_094_build.log`；
  冒烟落点 `D:/tb-eval/smoke-320`、`D:/tb-eval/smoke-320-lin`。

## §6 边界与如实记

1. **未推送未发行**：本批 orz bump 与父仓落账提交均在本地；不建 Release、不做 rel-stage
   打包；发布面停 v0.9.3（README 安装示例与 Release 链接保持 v0.9.3 不变）。
   装机位（`C:\Users\1\AppData\Local\orz\bin`）未换装——沿 281/290/309 式另批口径。
2. **S4 待续**：0di 仍 `pending`；0dk/0dl 仍 `pending`。本批只证「319 修复进体＋双平台可运行」，
   不替代 0dh/0di 真机判据。
3. **Windows 构建首跑误报**：首次以 `& build_orz.ps1 *>&1 | Tee-Object` 启动，脚本内
   `$ErrorActionPreference='Stop'` 把 cargo 的 stderr 进度行转成 `NativeCommandError` 提前终止
   （exit 1，非源码失败）；改用 `cmd /c "cargo build … > log 2>&1"` 复跑 **exit 0（4m56s）**。
   后续重建建议直接走 cmd 重定向或 `Start-Process` 分离 stderr。
4. **墙钟非同口径**：WIN 4m56s（`target/release` 暖缓存，仅 `incremental` 被清）；
   LIN 容器墙钟≈21 分（apt＋rg 静态树＋cargo 20m28s，`/target` 暖缓存）。
5. **告警面**：WIN 1（permission.rs 预存）；LIN 8（orz-config 1／orz-assurance 4／orz-host 3，
   musl cfg 门控预存 lint）——非本批引入，不入处置。
6. **磁盘**：清理后 26.99 GB free，构建结束 26.4 GB；`target/debug` 仍保留 48.8 GB deps
   （后续 debug 测试可复用，只缺 incremental 缓存）。若再告急，下一档可清
   `target/debug/deps`（48.8 GB，代价是测试全量重编）。

## §7 关键词

320 批、0.9.4 重建、源冻结 `5ee818d3`、0.9.3→0.9.4 bump、319 修复进体、D 盘清理 20.72 GB、
`incremental` 15.81 GB、`release-dist` 7.57 GB、MATCH 3/3、ACAF 重 provision `c7f5aa2f…`、
`f0bb5d92…`、`aaa68d4b…` 身份门、static-pie×3、alpine/bookworm 双冒烟、`6ad63513→1d→25→29`
收卷链、0dl 维护令牌活体门、`ORZ_MAINTENANCE_TICKET` 进体、`session/load history replayed`
进体、`run-scoped carry queue` 进体、保留面零回归、未推送未发行、计数 62 不变、v4.291。
