# 405 批：0.9.17 双平台载体重建出件＋Windows 换装进体（2026-11-20）

> 用户令：「请进行双端重建并换装吧」。性质＝载体重建＋换装批：源冻结 orz `51cea4ac`（404 处置 `2bc7075c`＋版本 bump `51cea4ac` 恰两行——403 落码 `2b534ffb`＋404 审查处置全数进体，**0dv S2 缺省翻转自此进载体**）。未闭合计数 73 不变。未推送、未发行。**换装后 0dv S4 九条判据＋0dw S3＋0dy/0dz/0dx 活体/0dt 判据 4 的合并首启窗自此开启**（核证随用户手动首启，不在本批）。

## §1 版本 bump

`0.9.16` → `0.9.17`：orz `51cea4ac` 恰两行（`crates/orz-bin/Cargo.toml`＋`Cargo.lock`；`env!("CARGO_PKG_VERSION")` 单源扇出同 400 口径）。

## §2 Windows 构建（exit 0）

`vcvars64`（`B:\VS\BuildTools`）＋`cargo build -p orz-bin --release -j 2`：**5m56s exit 0**（403/404 触碰 loop/tools/host ⇒ 级联重编 11 crate；较 400 8m36s 为短——402/403 两批零重建留足暖缓存）。日志 `D:\tb-eval\buildlogs\win_0.9.17_build.log`（构建跑法：批文件包装 vcvars64＋日志重定向——内联 `cmd //c` 多语句链引号解析失败一例，批文件形态实证可行，随批注记）。

## §3 Windows 换装（394/400 程序全步）

1. **停栈双证**（换装前后各一）：`tasklist`＋`Get-Process` 均 **0 个 orz 进程**。
2. **备份**：装机位三件 `*.0.9.16-405-bak`＋ACAF `signer-manifest.json.bak-20261120-405`（备份 cmp 逐位一致；回滚＝停栈→还原→重 provision）。
3. **复制＋MATCH 3/3**：暂存 `D:\tb-eval\orz-windows-0.9.17\` → 装机位 `%LOCALAPPDATA%\orz\bin`：
   - orz.exe 68,898,304 B `1b4cb128eac73db9a11736253309c123b2ebacf2ff94f5e797062fbc10cce5a0`（较 400 ＋274,944＝403/404 唤醒链＋状态机＋写回登记在体）
   - orz-signer.exe 6,740,480 B `f45c5a8429aaed671d490e436eb2dc03e900460349af2dbf1c6dc70d949f4930`
   - orz-acaf-provision.exe 6,644,736 B `b671dbc5bd53d2ce314c33bb6bc227a17cd9ac5fac6af0f6be98bf4bdd4932d1`
   - 暂存载体清单 `carrier-manifest.json` 3 entries 0.9.17（装机位本不放清单口径，394/400 同）。
4. **ACAF 重 provision** exit 0（装机位目录内实跑，邻接解析绑新 signer）：`binary_sha256=f45c5a84…` 逐位一致；新旧 manifest 逐字段 diff **恰 `binary_sha256` 一处**（`56aaa228…`→`f45c5a84…`）；keystore 两件（`installation-key.dpapi` `33c70b60…`／`.json` `5a57a2cb…`）前后 sha256 逐位未动。
5. **doctor** 装机位实跑（`doc_0.9.17_install.txt`）：exit 0，**ok=10 warn=2 fail=0**（两 warn＝runs 空＋装机位无清单，与 394/400 同构环境性）。
6. **版本串三面**：`--version`／`--build-info`＝`0.9.17 (windows/x86_64/release)`。

## §4 0dv/0dw 进体字面量核证（活体判据留 S4/S3 真机首启）

- 0dv 面：`idle_wake` ×10、`ORZ_IDLE_WAKE` ×3、`ORZ_IDLE_WAKE_DEBOUNCE_MS` ×1、`[idle_wake] ` ×2（TRIGGER＋PROMPT）、`ORZ_IMMEDIATE_RESULT_DELIVERY_M1` 尾段 ×1（retired 告知面）、CJK `[结果投递]` ×10＋`后台任务已完成` ×2（唤醒投递消息头在体）——**缺省翻转行为自此在役**（B1/B2 投递激活＋唤醒缺省 on）。
- 0dw 面：`model_call_usage` ×2（同 400 读数）；`carrier_version`／`binary_sha256`／`usage_complete` 零连续明文＝MSVC 拆立数构造（400 §4 同口径，据实注记非缺码）。
- 活体判据＝**用户手动首启窗**：①0dv S4 九条（唤醒 run 自动创建/在途不打断/合并投递/不换代/usage 落账/web 单卡；负三臂）②0dw S3 preflight 五键（`carrier_version`=0.9.17＋`binary_sha256`=`1b4cb128…` 清单同口径）＋逐调用 usage 落账＋CJK 偏差读数③0dx 活体面④0dt 判据 4。

## §5 Linux 构建（exit 0；代理 052 处置）

- Docker Desktop 未运行（拉起）；**代理透传死路复现**＝`ProxyHTTPMode=manual` 指宿主 Clash 7890（未运行，netstat 零监听）——沿 **052 先例**：settings-store.json 备份 `.bak-20261120-405`（sha256 `2e3ac271…`）→ `manual→disabled` → 重启 → 构建 → **字节级还原**（sha256 逐位一致复证）→ 再重启。
- 构建 exit 0（rust:1.97-slim 容器，镜像本地在位；rustup musl target 直连拉取正常）。日志 `D:\tb-eval\buildlogs\lin_0.9.17_build.log`。
- 出件（`D:\tb-eval\orz-linux\`，清单 3 entries 0.9.17）：
  - orz 98,900,080 B `528a3d38dba02520dd0a0621f89dae22767ada3a66b95108638062f9d584f245`
  - orz-signer 1,401,296 B `7f2537d6bd1cec40ce073c01e17dd5e92260bd3a3bdc64d25ab779c670d4507b`
  - orz-acaf-provision 1,217,736 B `85a0fbdefc5e26052a395f3914b8ddd701db5fc961e9e8d813391d75ae841e63`
- 核证：ELF 三件 `e_type=3`＋`PT_INTERP` 无＝static-PIE（392/400 口径）；冒烟＝alpine:3.20（musl）＋rust:1.97-slim（glibc）双容器 `orz 0.9.17 (linux/x86_64/release)`；附加 alpine 容器内 doctor ok=10 warn=2 fail=0（两 warn＝runs 空＋容器无浏览器，环境性）。**Linux 未换装**（部署面不在本机；出件暂存随发行/部署取用）。

## §6 父仓随批件

- `orz_source_manifest.sha256` 重算 **1507 条**（恰 2 行变更＝bump 两件，`51cea4ac` 先例同形）。
- `check_repository.py`：error_count 40 全数 ledger-slimming 既有类（超长＋行龄自然越界）；本批新行零命中（405 批档入库后断链清零核证）。

## §7 边界与不做清单（本批）

- **不推送、不发行**（随用户令范围＝重建＋换装）；GitHub Release 与包构建未动。
- **Linux 不换装**（部署面不在本机）；Linux 件暂存随发行批取用。
- **S4/S3 活体判据不在本批**：全部随用户手动首启（§4 列表）。
- Docker Desktop 代理已字节级还原（manual/7890 形态照旧）——下次容器构建若 Clash 仍未运行需同款处置。
- 换装判据中「web 桥 skew＝false／0dx 活体面」属真机窗，不在本批机械面。

## §8 入口

- 前批：[404 处置档](404_0DV_S2_REVIEW_DISPOSITION_2026-11-20.md)／[403 批档](403_0DV_S2_IDLE_WAKE_IMPL_2026-11-19.md)／[400 先例档](400_REBUILD_V0916_DUAL_PLATFORM_SWAP_WIN_0DW_S2_2026-11-18.md)。
- 子仓：`51cea4ac`（bump）← `2bc7075c`（404）← `2b534ffb`（403）。
- 台账：TODO 405 行／BACKLOG 本批记录指针／索引头 v4.365。
