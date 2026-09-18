# 0.6.2 双平台载体重建、换装与字面量核证（2026-09-18）

> 用户 2026-09-18 指示：「请进行重建吧，双平台」——本批＝**0.6.2 双平台重建 ＋ 换装 ＋
> 字面量核证**，即 0ap 回执 §7「待放行 S2」的执行。
> **源冻结面**：orz `07405e61`（已提交基线）＋工作树在场两批——0ap／0ao 未提交批
> （本批对象）与 0am 影子 RLI 批（未提交，随 O2 裁决）——＋版本 bump **0.6.1 → 0.6.2**。
> **上一载体**：Windows **0.6.1**（`1b047158` 冻结）／Linux **0.6.0**（`5041c3dc` 冻结）——
> 0.6.1 批按用户指示未重建 Linux，本批一并补平线。
> 性质：**重建＋换装＋核证**；不提交、不推送、不发 Release、不改计数（用户本批仅指示重建）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.1 → **0.6.2**（工作树内） |
| Windows 三件套 | clean 全量重建 `CARGO_EXIT=0`（**10m36s**）；冒烟双 exit 1 形态；换装逐件 MATCH；ACAF 重 provision（manifest↔signer 一致） |
| Linux musl 三件套 | 首轮 apt 经代理 5 连败（FR-D03 复现）⇒ 按先例切代理后 `LINUX_BUILD_EXIT=0`（**18m51s**）；PT_INTERP=0；bookworm/alpine 双向冒烟全绿；构建脚本直写 `/out` 完成换装 |
| 字面量核证 | 双平台同表：0ap 三处新面 0→非零、0ao 收敛面 94→18（Win）／45→1（Lin）、v8／0af／0ak／退役面**逐项保持** |
| 代理处置 | 切换前备份＋切换后**字节级还原**（还原件与备份件哈希相同、读数 `manual / http://127.0.0.1:7890` 复原） |
| 未做 | 提交／推送／Release／计数变更／测试面复跑（读数沿用 0ap 回执 §5／§8） |

## 1. 前置基线核证（只读）

重建前先取两平台在役载体的尺寸与 SHA256，与账本记录逐位对照——**备份链完整性**的前提：

| 平台 | 在役件 | 尺寸 (B) | SHA256（前 8） | 对照 |
|---|---|---|---|---|
| Windows（0.6.1） | orz.exe | 53,937,152 | `5C991621…` | 索引 v3.62 记录一致 ✓ |
| Windows（0.6.1） | orz-signer.exe | 6,742,528 | `759A1DEF…` | 索引 v3.62 记录一致 ✓ |
| Windows（0.6.1） | orz-acaf-provision.exe | 6,642,176 | `EEE03BE9…` | 索引 v3.62 记录一致 ✓ |
| Linux（0.6.0） | orz | 111,256,888 | `F9B4B2ED…` | 060 审计 §3 记录一致 ✓ |
| Linux（0.6.0） | orz-signer | 1,397,808 | `71082AC1…` | 060 审计 §3 记录一致 ✓ |
| Linux（0.6.0） | orz-acaf-provision | 1,216,608 | `439F6550…` | 060 审计 §3 记录一致 ✓ |

## 2. Windows 三件套（宿主 release，clean 全量）

- 构建：`cargo clean`（移除 54,409 文件／33.6 GiB）后全量重建
  `cargo build --release -p orz-bin`（`PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`，
  清 `ORZ_*`／`GROK_*` env 口径）；`CARGO_EXIT=0`，**10m36s**，日志收尾
  `Compiling orz-bin v0.6.2` / `Finished release profile in 10m 36s`。
- 告警：2 项**均为既有 dead_code**（`xai-tty-utils::resource_job::process_alive`、
  `orz-host::register_live_call_job`——0.6.0 Linux 构建日志同样存在 xai-tty-utils 该条），
  **无 error**。构建日志留档 `.tmp-062-windows-build.log`。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.1 差异 |
|---|---|---|---|
| orz.exe | 53,989,888 | `D33D904B6F20C163232FACE99E4969FDFB5BF1FAEC85DAC6C66C87F0EF400CA2` | +52,736 B |
| orz-signer.exe | 6,742,528 | `0BF87AB02FCDC16671129E708E22D5136D04F0E030CAA1B460846EB845C65D8C` | 尺寸同（哈希随重建变更） |
| orz-acaf-provision.exe | 6,642,176 | `30CC538A9D828AB91E247B9BF9E0A4589B246D4DEA0EC83B642043A0561E889A` | 尺寸同（哈希随重建变更） |

- **冒烟（构建位）**：`orz-acaf-provision` 无参 usage **exit 1**；`orz-signer` 无
  manifest fatal **exit 1**。
- **换装**（`D:\tb-eval\orz-windows`）：先备份（三件 `.0.6.1-bak`）后覆盖；换装后逐件
  SHA256 与构建产物 **MATCH**（三件全对）。
- **ACAF 重 provision**（signer 哈希随 clean 重建变更，按 052/061 先例）：keystore 保留
  （`create_or_load` 只读既有件）、旧 manifest 留 `signer-manifest.json.bak-20260918-062`；
  重 provision 后 manifest `binary_sha256=0bf87ab0…` ↔ 换装位 `orz-signer.exe` 实哈希
  **逐位一致**；工具回显 launch env（`ORZ_ACAF_KEYSTORE`／`ORZ_ACAF_MANIFEST`／
  `ORZ_ACAF_BINARY`）。
- **冒烟（换装位）**：provision usage **exit 1**；signer fatal **exit 1**（同 052–061 形态）。

## 3. Linux musl 三件套（Docker）

### 3.1 首轮失败与先例处置

- 首轮构建**失败**（既有 **FR-D03** 摩擦复现）：容器内 `apt` 经 Docker 代理取索引时
  `SSL routines::unexpected eof while reading`（`deb.debian.org` InRelease 半失败），
  `musl-tools`／`protobuf-compiler`／`ripgrep`／`make` 全不可用，5 次重试全败 ⇒
  `LINUX_BUILD_EXIT=1`；失败日志留档 `.tmp-062-linux-build.proxy-fail.log`（4,603 B）。
  **复现要点**：`apt-get update -qq` 半失败仍 exit 0 ⇒ 单跑 update 会假绿（配方 §症状已记）。
- 按 **052／054／060 先例**处置：`settings-store.json` 备份 `.062-bak`（原值
  `ProxyHTTPMode=manual`／`OverrideProxyHTTP(S)=http://127.0.0.1:7890`）→ 切
  `disabled`（Override 行未动）→ `docker desktop restart` → **实包预检**
  （`musl-tools`＋`protobuf-compiler`＋`ripgrep`＋`make` 全装、回 `APT_OK`）→ 重跑构建。
- 构建后**字节级还原**：还原件与备份件 **SHA256 相同**，读数复原
  `manual / http://127.0.0.1:7890`，并再次重启 Docker Desktop（引擎就绪）。

### 3.2 构建与产物

- 构建：`rust:1.97-slim` ＋ `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约、`MSYS_NO_PATHCONV=1`、`/target` 缓存沿用）；
  `LINUX_BUILD_EXIT=0`（**18m51s**：13:34:21 → 13:53:12）；日志 `.tmp-062-linux-build.log`。
- 告警：4 项与 0.6.0 基线**逐项同源**（`xai-tty-utils` 1／`orz-config` 1／
  `orz-assurance` 4／`orz-host` 3）——Linux-only cfg 面既有告警，非本批新增。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.0 差异 |
|---|---|---|---|
| orz | 111,394,672 | `D14D6D9C2E6AD14FC48601AAD94C0499012BFA1D40491C1F8922A36DE1441A44` | +137,784 B |
| orz-signer | 1,398,880 | `73F5CE71077B07A5073EFF73B8015426AAF4AB0AD11B692D8E26BE8E164FD028` | +1,072 B |
| orz-acaf-provision | 1,217,640 | `EB186E4F677EF4E992164AADC0E9A02770FFCAE2DD087FD3345A000168FDB0F2` | +1,032 B |

- **ELF 静态核验**（程序头解析）：三件均 `e_type=3`（ET_DYN static-pie）＋
  `e_machine=62`（x86-64）＋ **PT_INTERP=0**。
- **双向加载冒烟**：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件**均 exit 1**
  （预期形态，同 052–060）。
- **换装**：构建脚本直写 `/out`（＝ `D:\tb-eval\orz-linux`）；构建前已把在役 0.6.0 三件
  预留 `.0.6.0-bak`（哈希与 §1 基线对照一致），历史备份链（0.5.1／0.5.3／0.5.4／0.4.2）保留。
  Linux 侧 ACAF 由评测容器运行期 provision（`FileInstallationKeyStore`），与 Windows 不同，
  本批不涉 manifest 换装。

## 4. 字面量核证（双平台同表，`-a` 字节级计数）

计数口径：逐文件 UTF-8 解码后按**出现次数**统计（含跨 crate 常量池副本）。基线列＝
Windows 0.6.1 备份件／Linux 0.6.0 备份件；新件列＝Windows 构建产物／Linux `D:\tb-eval\orz-linux`。

| 字面量 | Win 0.6.1 | **Win 0.6.2** | Lin 0.6.0 | **Lin 0.6.2** | 判读 |
|---|---|---|---|---|---|
| `session archive: ` | 3 | 3 | 1 | 1 | 0ak 面保持 |
| `blackboard_write` | 94 | **18** | 45 | **1** | **0ao 收敛生效**（字面折叠至单源；余量＝跨 crate 常量池副本，见 §4 注） |
| `context_compress` | 4 | **14** | 4 | **5** | **0ap 工具名进件** |
| `context_compressed`（事件名，对照） | 3 | 3 | 3 | 3 | 事件族零改动（用于剔除子串误命中） |
| `Open a compression window` | 0 | **5** | 0 | **5** | 0ap 工具描述进件 |
| `滑块外可压缩` | 0 | **1** | 0 | **1** | 0ap 滑块读数行进件 |
| `压缩窗口已在程中（in_progress）` | 0 | **1** | 0 | **1** | 0ap 三态文案进件 |
| `ORZ_MODEL_FACE_SLIDER_TOKENS` | 1 | 1 | 1 | 1 | v8 主滑块 env 保持 |
| `context_scale:hard_truncate` | 7 | 7 | 7 | 7 | v8 T1 键保持 |
| `ORZ_SLIDER_WINDOW_TOKENS` | 0 | 0 | 0 | 0 | v7 退役面保持 |
| `hard_950k_intercepted` | 0 | 0 | 0 | 0 | 950K 退役面保持 |
| `宿主机内存/储存资源即将耗尽` | 1 | 1 | 1 | 1 | 0af 定案句保持 |
| `读数不可得` | 1 | 1 | 1 | 1 | 0af Unknown 变体句保持 |
| `（前略）` | 0 | 0 | 0 | 0 | 收口清理面保持 |
| `WALLCLOCK_REMAINING_ROUNDS` | 1 | 1 | **0** | **1** | **0am 影子批（未提交）进件**——见 §5 边界 |
| `rli_shadow` | 2 | 2 | **0** | **4** | 同上（0am 面） |

**注（0ao 收敛读数的跨平台差异）**：`blackboard_write` 收敛后 Windows 记 18、Linux 记 1。
差异来源是**链接期常量合并策略**（ELF 侧合并同名只读常量、PE 侧保留各 crate 副本），
不是源码面第二字面——权威判据仍是 0ao 的源码级机械扫描钉子（«单一源＋豁免表为空»），
二进制计数只作**面变化指示**（0.6.0/0.6.1 基线的 94/45 同源差异亦如此）。

## 5. 边界（如实登记）

1. **0am 未提交批进入二进制**：本批二进制由含 0am 影子 RLI 批的工作树构建
   （`WALLCLOCK_REMAINING_ROUNDS`／`rli_shadow` 读数坐实）；该批仍未提交、随 O2 裁决。
   Windows 0.6.1 已有同形边界，Linux 0.6.2 为**首次**含 0am 面。
2. **无源冻结提交**：本批 bump 与全部批内容仍在工作树（用户未指示提交）；「提交线」与
   「实际二进制源」之间的差异按 0.6.1 先例登记，不作隐性等价声明。合回后如需可重现二进制
   须再走一次重建。
3. **未发 Release／未推送**：用户本批仅指示重建（0.5.1 先例：单项指示不沿用）。
4. **未复跑测试面**：本批相对 0ap／0ao 回执仅增 bump 两行（`Cargo.toml`／`Cargo.lock`），
   读数沿用回执 §5／§8（orz-loop 796/0/3、orz-assurance 246/0、orz-host 332/0/5、
   clippy 逐位持平、fmt 仅 0am 件）。承载判据的载体面读数见 §2–§4。
5. **054 遗留观察项**未触碰（「未知旗标进 TUI 挂住」；`--version` 仍不作冒烟项）。
6. **0an 载体面**：其唯一未勾动作（Linux musl 三件套重建）随本批**达成**（0.6.2）；
   闭合入账随下一提交批（0ai 先例：产出未提交则项保持开放）。

## 6. 账本同步面

- **索引**：头行新增 **v3.64**（本批复述＋入口）；`GAP-BYTE-BOUNDARY-PANIC` 条目**句中措辞同步**
  （载体句补「；**双平台补平**＝Linux musl 三件套随 **0.6.2** 重建换装」＋入口）。
- **BACKLOG**：0an 节标题与边界行改写（Linux 载体已随 0.6.2 重建，闭合入账随提交批）
  ＋「载体重建」小节新增；**优先级总览 P1 行**句中同步（`载体 0.6.1；Linux 0.6.2 已补平，闭合入账随提交批`）。
- **TODO**：0an 唯一未勾动作改写为「待提交批入账（Linux 载体重建已完成）」＋读数与入口。
- **配方**：`scripts/DOCKER_PROXY_RECIPE.md` 新增第 0 步「**预检优先**」——先跑实包预检，通过即
  跳过切换与两次重启（本批实证：首轮失败后按 3–4 切换即一次通过）。
- 计数：**不变**（0ap／0ao／0an 的闭合入账仍随提交批）。
- **核验读数**：`git diff --numstat`＝索引 2/1、BACKLOG 9/4、TODO 6/4（与上述逐处同步一一对应）；
  门禁 `python scripts/check_repository.py` → `error_count=1`（唯一＝`orz submodule working tree is dirty`，
  未提交态预期）。
- **证据留档**（仓根，`.tmp-*` 门禁豁免面）：`.tmp-062-windows-build.log`／
  `.tmp-062-linux-build.log`／`.tmp-062-linux-build.proxy-fail.log`（首轮失败原样）。

## 7. 摩擦登记与配方改进

| 序 | 摩擦 | 证据 | 处置 |
|---|---|---|---|
| **F-062-1（环境，既有 FR-D03 复现）** | Linux 容器 apt 经 Docker 代理取索引 SSL 中断（`error:0A000126`），首轮 5 连败 | `.tmp-062-linux-build.proxy-fail.log` | 按先例切 `ProxyHTTPMode=disabled` 后一次通过；**配方改进**：把「实包预检」提到切换之前（`DOCKER_PROXY_RECIPE.md` 第 0 步），使「上游可达」时免去两次 Docker 重启 |
| **F-062-2（观察）** | `blackboard_write` 二进制字面计数跨平台差异（Win 18／Lin 1） | §4 表 | 判为链接期常量合并策略差异，非源码面双字面；权威判据＝源码级机械扫描钉子（已在 §4 注登记，避免误读为收敛不彻底） |

## 8. 入口与关键词

入口：[`0ap／0ao 回执`](0AP_0AO_COMPRESSION_INTERACTION_AND_TOOLNAME_SINGLE_SOURCE_2026-09-18.md) ／
[`060 载体重建先例`](060_CARRIER_REBUILD_2026-09-16.md) ／
[`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) ／
[`BACKLOG 0an`](../BACKLOG_AND_PRIORITIES.md)。
关键词：0.6.2、双平台、载体重建、换装、字面量核证、ACAF 重 provision、Linux musl static-pie、
Docker 代理切换与还原、0ap S2、0ao 收敛面、0an Linux 载体。
