# 323 批：0.9.5 双平台载体重建＋装机位换装（322 摩擦修复进体，2026-10-21）

> **用户令**：「请进行重建并换装吧，目前没有进程在跑了，实验的下半部分没有正常进行」。
> **本批**＝0.9.4→0.9.5 代窗口双平台重建（Windows release＋Linux musl）→ 载体位换装 →
> 装机位换装 → ACAF 重 provision → 载体清单 → 双平台冒烟 → 身份门 → 落账。
> **源冻结**＝orz **`974d0345`**（322 摩擦修复 10 文件＋版本 bump 两行，`+420/−56`；
> `cargo metadata --locked` exit 0）。**未推送、未发行、未打包**。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结 | orz `974d0345`（`批(323 重建): 0.9.4 → 0.9.5——322 摩擦修复进体`；12 文件 `+420/−56`） |
| Windows 重建 | `cargo build -p orz-bin --release -j 2` **exit 0（3m22s 暖缓存）**；唯一警告＝`orz-host/src/permission.rs` unused imports（287 在案预存） |
| Windows 三件 | `orz.exe` **58,674,176 B `10b54299…`**（较 0.9.4 −29,184）／`orz-signer.exe` 6,740,480 `036073ad…`（Δ0）／`orz-acaf-provision.exe` 6,640,128 `11cfffef…`（Δ0）；载体位与 `target/release` **MATCH 3/3** |
| 装机位换装 | `C:\Users\1\AppData\Local\orz\bin` 三件 MATCH 3/3；`.0.9.4-bak` 链预建；`--build-info`＝`0.9.5 os=windows arch=x86_64 profile=release` rc0 |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261021-323`；provision **exit 0**；`binary_sha256=036073ad…` ↔ 换装位 signer **逐位一致**；keystore 两件 `f37556ab…`/`2aa80cb8…`（Sep 12 原件）**逐位未动** |
| Linux musl | docker `rust:1.97-slim`＋`build_orz_aliyun_trixie.sh` **exit 0（cargo 7m05s，暖缓存）**；三件 `a409e873…`/`8fc105ef…`/`23d11c45…`；**static-pie×3、INTERP=0、not stripped** |
| Linux 冒烟 | alpine 3.20 `RUN-CLI-6ad85199`、bookworm-slim `RUN-CLI-6ad8519f` 均 rc0；journal 11 events（usage index 9、terminal 10）；双镜像 `0.9.5 os=linux` rc0 |
| 身份门 | `scripts/run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `aaa68d4b…`→**`a409e873…`**；py_compile 过＋`--help` rc0 |
| 字节判据 | 322 新面 WIN：`recent_workspaces` 0→**3**、`本次会话打开过` 0→**1**、`（未持久授信）` 0→**1** |
| Web 实机探针 | 0.9.5 桥：boot 出 `recent_workspaces`；切到 `D:\CLI` 200；**删掉 smoke 根的 TrustStore 授予后仍可 visited-only 切回** 200；旧根 `/api/conversations?root=` 200 |
| 门禁 | `orz_source_manifest.sha256` 重算 **1488 条**（12 行差异＝本批 12 文件）；`check_repository.py` 见 §6 |
| 计数 | 62 不变；索引 v4.292→**v4.293** |

## §1 源冻结

- 322 修复（未提交工作树）＋版本 bump 一并冻结为 orz **`974d0345`**：
  - `context_scale.rs`：H1 闩位随会话水位恢复；T1 仍按越线重发。
  - `agent_loop.rs`：候选终答自带「反例自查」时跳过 gate；gate 确需注入时下一轮不消费语义摘要（留 run 尾）。
  - `action_ledger.rs`：300 字符字段上限＋400 字符整行上限（marker 不再内嵌整段命令）。
  - `server.rs`/`lib.rs`/`runs.rs`/前端三件：visited 工作区、`recent_workspaces`、会话 `root`、跨根恢复。
  - `crates/orz-bin/Cargo.toml`＋`Cargo.lock` 恰两行 0.9.4→0.9.5。
- `cargo metadata --locked --format-version 1 --no-deps` exit 0。
- 322 批的 `cargo check`／定向单测读数沿用：`context_scale::tests::` 15/15、`action_ledger::tests::` 23/23、`counterexample_gate*` 3/3、run 尾语义摘要 1/1、`server::tests::` 19/19。

## §2 Windows 重建与换装

**基线**（换装前，与 320/321 账面逐位吻合）：

| 文件 | 0.9.4 SHA256 |
|---|---|
| `orz.exe` | `f0bb5d92dd5b74d2…`（58,703,360 B） |
| `orz-signer.exe` | `c7f5aa2f…`（6,740,480 B） |
| `orz-acaf-provision.exe` | `478fcce7…`（6,640,128 B） |

**0.9.5 产物**：

| 文件 | 尺寸 | SHA256 |
|---|---|---|
| `orz.exe` | 58,674,176（−29,184） | `10b542991db855d4…` |
| `orz-signer.exe` | 6,740,480（Δ0） | `036073ad67250bd3…` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `11cfffef3b579e19…` |

- 构建：`cargo build -p orz-bin --release -j 2`，`Finished release profile in 3m 22s`，exit 0。
- 载体位（`D:\tb-eval\orz-windows`）三件 `MATCH 3/3`（与 `target/release` 逐位一致）。
- `.0.9.4-bak` 链：载体位与装机位各三件预建。
- 装机位换装 MATCH 3/3；`--build-info`＝`version=0.9.5 os=windows arch=x86_64 profile=release` rc0。
- 装机位无 `acaf` 面（无 `signer-manifest.json`）⇒ **不重 provision**（沿 266/281/290 口径）；signer 二进制已随载体位 provision 核证为同一构建。
- 载体位清单 `D:/tb-eval/orz-windows/carrier-manifest.json` 刷新 0.9.5（3 entries）。

**ACAF（载体位 `D:\tb-eval\orz-windows\acaf`）**：

```text
orz-acaf-provision.exe D:/tb-eval/orz-windows/acaf/keystore D:/tb-eval/orz-windows/acaf/signer-manifest.json
→ binary_sha256: 036073ad67250bd303a66e51e871bbf006499ab04b42159568d3f105c3559b9a  [exit 0]
```

- manifest `binary_sha256` ↔ `orz-signer.exe` 实哈希逐位一致。
- keystore 两件 `installation-key.dpapi f37556ab…`／`installation-key.json 2aa80cb8…`（Sep 12 mtime）逐位未动。
- 旧 manifest 留 `signer-manifest.json.bak-20261021-323`。

## §3 Linux musl 重建

- 命令：`docker run --rm -v D:/CLI:/orz -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml -v D:/tb-eval/orz-target:/target -v D:/tb-eval/orz-linux:/out -v D:/tb-eval/build_orz_aliyun_trixie.sh:/build.sh -w /orz/orz rust:1.97-slim bash /build.sh`
- 结果：**exit 0**；`Finished release profile [optimized] target(s) in 7m 05s`（`/target` 暖缓存）。
- 产物与 `/target/x86_64-unknown-linux-musl/release` **MATCH 3/3**：

| 文件 | 尺寸 | SHA256 |
|---|---|---|
| `orz` | 117,602,608 | **`a409e8733f134e07…`**（身份门新值） |
| `orz-signer` | 1,397,944 | `8fc105ef5a052869…` |
| `orz-acaf-provision` | 1,216,408 | `23d11c455932cf18…` |

- ELF：三件均 `ELF 64-bit LSB pie executable, x86-64, static-pie linked, not stripped`；`readelf -l orz | grep -c INTERP`＝**0**。
- `--build-info`：alpine 3.20／bookworm-slim 均 `version=0.9.5 os=linux arch=x86_64 profile=release` rc0。
- 载体位清单 `D:/tb-eval/orz-linux/carrier-manifest.json` 刷新 0.9.5（3 entries）。
- Linux 不重 provision（沿 308/320 口径；容器内 `/tmp` 临时 provision 仅用于冒烟）。

## §4 冒烟与活体核证

**Windows（`D:\tb-eval\smoke-323`，`--fake-provider -p`）**

- `RUN-CLI-6ad84f35` **rc=0**；journal 11 events（seq 0–10）：`run_preflight`／`tool_availability_check`×2／`run_started`／`prompt_submitted`／`request_header_change`／`face_fingerprint`／`model_output`／`host_resource_snapshot`／`run_usage_metered`（九字段，index 9）／`run_finished`（index 10）。
- 维护 CLI 门：`orz delete-session deadbeef` 无令牌 **exit 2**（明确「需要桥注入的 ORZ_MAINTENANCE_TICKET」）；16 字符令牌 → **exit 1**「无数据可删除」——鉴权门先于存在性检查。
- 0.9.5 桥（`orz web --addr 127.0.0.1:21587`，fake）实机探针：
  - `/` 200；`/api/boot` 返回 `cwd`＋`trusted_workspaces`＋`recent_workspaces`；
  - `POST /api/workspace/switch {"path":"D:/CLI"}` **200**，响应 `recent_workspaces=[smoke, D:\CLI]`；
  - 从 TrustStore **删除 smoke 根授予之后**，`POST /api/workspace/switch {"path":"D:/tb-eval/smoke-323"}` 仍 **200**——visited-only 切回生效（322 用户报告的直接判据）；
  - 切换后 `/api/conversations?root=D:/tb-eval/smoke-323` **200**——旧根只读回看不因未信任而 403；
  - 切回后 boot `cwd`＝smoke，`recent_workspaces` 同时保留两个根。
- **装机位实环境复验**（`C:\Users\1\AppData\Local\orz\bin\orz.exe web --addr 127.0.0.1:21588`，启动 cwd＝桌面工作区）：
  - `boot.trusted_workspaces` 含桌面根（`decided_at=1792512647`）＋`D:\CLI`（`1790355043`）——322 补的信任记录在换装后仍在；
  - `POST /api/workspace/switch {"path":"D:/CLI"}` **200**，再切回桌面 **200**；`recent_workspaces` 两次均同时保留两个根。
- 字节判据（0.9.4-bak→0.9.5）：`recent_workspaces` 0→3、`本次会话打开过` 0→1、`（未持久授信）` 0→1；`反例自查` 23→23（同字面量被编译器合并，门逻辑由单测承载）。

**Linux（容器）**

- alpine 3.20 `RUN-CLI-6ad85199` **rc=0**；bookworm-slim `RUN-CLI-6ad8519f` **rc=0**。
- 两侧 journal 均 11 events、seq 0–10、`run_usage_metered` index 9、`run_finished` index 10（与 WIN 同构）。
- 容器内 provision（plain-file keystore）＋fake provider；ACAF 未绕过。

## §5 身份门与清单

- `scripts/run_r0_heavy_official.py`：`EXPECTED_CARRIER_SHA256` `aaa68d4b…`→**`a409e8733f134e07eadd0bed415fa2c5c6531ee8f95b5e33ffaad37de4911d41`**；注释补 0.9.5 源冻结 `974d0345`；`python -m py_compile` 过＋`--help` rc0。适配器哈希 `6d55c26e…` 未动。
- `orz_source_manifest.sha256`：重算 **1488 条**，diff 恰 12 行（10 个源码/前端文件＋`Cargo.toml`＋`Cargo.lock`）。
- 载体清单：`D:/tb-eval/orz-windows/carrier-manifest.json`／`D:/tb-eval/orz-linux/carrier-manifest.json` 均 `version=0.9.5`、3 entries。

## §6 门禁与台账

- `python scripts/check_repository.py`：**valid: true / error_count: 0**（warning 4＝既知 allowlist 证据类）。
- 索引头行 v4.292→**v4.293**（本批 323＋前批 322 摘要）。
- 本档；322 档（[`322_TONIGHT_FRICTION_FIX_2026-10-21.md`](322_TONIGHT_FRICTION_FIX_2026-10-21.md)）。
- 计数 **62 不变**（本批零闭合、零新增开放项）。

## §7 边界与如实记

1. **未推送、未发行、未打包**：不建 GitHub Release、不做 rel-stage、不更新 README 发布面；发布面仍停 v0.9.4。
2. **装机位只换 Windows 三件**（`C:\Users\1\AppData\Local\orz\bin`）；Linux 载体位为 `D:/tb-eval/orz-linux`，无 Windows 式「安装位」面。
3. **`orz.exe` 尺寸下降 29,184 B**：322 修复删除/收紧了部分代码路径（含重复门分支与未截断台账渲染），非功能缺失；进体判据以新面字符串与单测承载。
4. **signer/provision 哈希变化但尺寸 Δ0**：0.9.5 版本串链接传播（155/170/180 同形）；已重 provision 并逐位绑定。
5. **`反例自查` 字节数未变**：该字面量在 prompt 块与 helper 中重复，LLVM 合并；门行为由 `counterexample_gate_skipped_when_draft_already_has_selfcheck` 等 3 个单测证明。
6. **实验下半部分未正常进行**：本批开始前确认无 python/orz 进程；重建使用空闲 CPU，未触碰 `D:\AGI` 实验数据。
7. **D 盘余量**：构建前后约 24 GB free；未 `cargo clean`，未删除任何 bak 链或评测数据。
8. **Windows 构建告警 1 条**（`permission.rs` unused imports）与 Linux 告警（`orz-assurance` 4／`orz-host` 3 等 musl cfg 预存 lint）均非本批引入，不入处置。
