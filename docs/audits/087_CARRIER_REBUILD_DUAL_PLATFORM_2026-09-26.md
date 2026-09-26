# 087 载体重建与换装（双平台 · 0.7.0 → 0.7.1）

> **日期**：2026-09-26；**用户令**：「请先重建吧」（重建先行，落账待令）。
> **进体内容**＝0bw v1（`write_control.rs`／`exec_policy.rs`＋工具面/命令面/载体自保护接线＋SECURITY.md）＋086 F7-B 方案收口（bash 提示词兜底臂／索引预览／标题抽取三处梯）＋版本 bump 两文件两行。**源冻结＝worktree 态**（orz 子模块 HEAD 未动 ⇒ 源清单按设计不动）；**未提交／未推送／未发行**（打包随发行令循 077 §8 配方）。

## 1. 版本 bump（两文件两行）

- `orz/crates/orz-bin/Cargo.toml:3` 0.7.0 → **0.7.1**；`orz/Cargo.lock` orz-bin 条目同步。`cargo metadata --locked` exit 0。（锁内另三处 `0.7.1`＝petgraph／serde_urlencoded／untrusted 第三方巧合同号，未动。）

## 2. Windows 增量重建与换装

- 构建：`cargo build --release` exit 0，**3m57s**（`Compiling orz-bin v0.7.1`）；重建前按 080 档惯例 `cargo clean --profile dev` 腾挪 16.0 GiB（D: 余 12G → 27G）。
- 换装：构建产物三件 → `D:\tb-eval\orz-windows\`，**MATCH 3/3**（旧三件留 `.0.7.0-bak` 链）；`--build-info` 换装位读数 **`version=0.7.1 os=windows arch=x86_64 profile=release`**。

| 文件 | 尺寸 (B) | 与 0.7.0 差异 | SHA256（前 16） |
|---|---:|---:|---|
| `orz.exe` | 56,888,320 | **+59,392 B** | `ec4bd1e4a37f21bb` |
| `orz-signer.exe` | 6,740,480 | 0 | `2f76b0ce7e15fc49` |
| `orz-acaf-provision.exe` | 6,640,128 | 0 | `4beed214f260fce8` |

## 3. ACAF 重 provision

- 旧 manifest 留 `signer-manifest.json.bak-20260926-087`；provision **exit 0**；keystore 两件哈希前后逐位未动（`f37556ab653c`／`2aa80cb8d618`）；回显 `binary_sha256=2f76b0ce7e15fc49…` ↔ 换装位 signer 逐位一致。

## 4. 字面量核证（Windows 二进制；对照＝0.7.0）

| 字节模式 | 0.7.0 | 0.7.1 | 判定 |
|---|---:|---:|---|
| `[写入管控·提示]` | 0 | **1** | 0bw warn 头行进体 ✅ |
| `protected by the write control` | 0 | **2** | 0bw deny 文案进体 ✅ |
| `carrier:install-file` ／ `carrier:session-volume` | 0／0 | **1／1** | deny 规则 id 进体 ✅ |
| `orz-build-info: version=` | 1 | 1 | 保持面 ✅ |
| `No, and tell the AI what to do differently` | 1 | 1 | ⑮ 保持面 ✅ |
| `No, and tell Grok what to do differently` | 0 | 0 | 零回退 ✅ |
| `GrokBuild:` ／ `GROK_HOME` | 26／7 | 26／7 | 边界保留 ✅ |
| `360search` ／ `ORZ_RETRIEVAL_FINGERPRINT` | 3／1 | 3／1 | 保持面 ✅ |

## 5. 载体级 Web 探针（`.tmp-b087-win-web-probe.ps1`，端口 21560）

- 全过：`/` 200／无令牌 **401**／伪 Host **401**／带令牌 **200**；`boot_keys = cwd,agent,trusted_workspaces`；归档 **28 件**（0.7.0 读数 27 → +1＝0bw 轮会话归档 `6ab7b7eb.json.gz`）；`leftover_orz_procs=0`。启动横幅 GBK 呈现为既有已知观察面（探针捕获路径，0bs ② 同族）。

## 6. Linux musl 三件套（Docker，直写换装位）

- `rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约＋嵌套 runtime 绑定＋`MSYS_NO_PATHCONV=1`）**exit 0**，**10m42s**；换装位旧三件预置 `.0.7.0-bak` 链后直写。

| 文件 | 尺寸 (B) | 与 0.7.0 差异 |
|---|---:|---:|
| `orz` | 115,228,640 | **+61,240 B** |
| `orz-signer` | 1,401,688 | −56 B |
| `orz-acaf-provision` | 1,220,440 | −56 B |

- ELF 静态核验（alpine 3.20 ＋ binutils/file）：三件 `static-pie linked`＋**`PT_INTERP=0`**（BuildID `6d5b30ce`／`6a51297e`／`bb12ef61`）。
- 双向加载冒烟：bookworm-slim／alpine 3.20 **6/6 exit 1**（预期形态）。
- 警告面：代码警告 **7 条与基线同数**（`WindowsDpapiInstallationKeyStore`／`Mutex` unused import、`cmd`／`path`／`pid`×3 unused——非 Windows 分支同族）＋2 条工具链噪音（rustup override 注记／rg PATH 提示）。

## 7. 未做项

1. **未提交／未推送**：0bw v1＋086 B 方案＋本批 bump 同停树面（用户令「请先重建吧」＝重建先行，落账待令）；源冻结＝worktree 态。
2. **本地打包与发行未做**（随发行令）；**S4 真机复验未跑**（0bw 锁死面拦截读数／L2 留痕读数／CFA audit 轮 1124 读数，随 0bv 轮）。
3. **预检未做**（默认口径）。

## 8. 本批摩擦（仅记录，不另立项）

1. **D: 余量回弹**（狗粮轮＋本会话测试产物 → 12G）⇒ `cargo clean --profile dev` 腾挪 16.0 GiB 后 27G（连续第四批同族操作，已惯例化）。
2. 无新增（`MSYS_NO_PATHCONV=1`／alpine binutils 现装等按 080 配方注记预先规避）。
