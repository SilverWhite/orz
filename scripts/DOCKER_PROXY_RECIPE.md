# Docker 代理切换配方（Linux musl 载体重建；FR-D03 处置，2026-09-17）

> 来源先例：[`052_CARRIER_REBUILD_AND_0AI_RELEASE` §4](../docs/audits/052_CARRIER_REBUILD_AND_0AI_RELEASE_2026-09-16.md)（0.5.2 五连败→处置通过）、[`054_CARRIER_REBUILD` §4](../docs/audits/054_CARRIER_REBUILD_2026-09-16.md)（0.5.4 同形复现）。本报告只固化「靠先例处置」的口径，未立项、未改任何装置。

## 症状与根因（本机形态）

- 症状：容器内 `apt` 失败（`Unable to locate package …`；经代理节点到 `deb.debian.org` **间歇 502**），首轮五连败级偶发。
- 根因链：Docker Desktop `ProxyHTTPMode: manual` ＋ `OverrideProxyHTTP(S): http://127.0.0.1:7890` 指向**当前未运行/时通时断的 Clash**——容器流量经 `http.docker.internal:3128` 中继强制走死上游（清容器 env 无效，拦截在 Desktop 层）。
- 陷阱：apt `update` 半失败**仍 exit 0**——单跑 update 的预检会**假绿**，必须以 install 实包为准。

## 配方（字节级还原口径；逐条照做）

0. **预检优先（2026-09-18 补充，0.6.2 重建实证）**：先跑**实包预检**——`docker run --rm
   rust:1.97-slim bash -c "apt-get update -qq && apt-get install -y -qq musl-tools"`。
   **预检通过即跳过 3–4（不切换、不重启）**，直接走 6；失败才进入 3–4。理由：切换 +
   两段重启是**处置**而非**仪式**——上游（Clash）可达时切换无收益，反而是两次 Docker
   Desktop 重启的额外风险面。0.6.2 重建即预检一次通过（`APT_OK`），全流程零切换零还原。
1. **备份**：复制 Docker Desktop 设置文件（`settings-store.json`，位置随 Docker Desktop 版本）到位旁备份 `settings-store.json.<批>-bak`（先例：`settings-store.json.052-bak`）。
2. **记账**：记录原值（`ProxyHTTPMode`、`OverrideProxyHTTP(S)`）——还原判据。
3. **临时切换**：`ProxyHTTPMode` 改 `disabled`（不动 Override 行）。
4. **重启 Docker Desktop**；就绪判定 **≈5–10 s**（F-022 冷启动同形）。
5. **实包验证**（勿信 update）：`apt-get install musl-tools`（或本次所需实包）通过才算网络通。
6. **正式构建**：跑 `build_orz_aliyun_trixie.sh`（保持 `MSYS_NO_PATHCONV=1` 与 ORZ-BUILD-MOUNT-001 契约、`/target` 缓存沿用）；失败首轮日志留档（先例：`build-20260916-0.5.4.proxy-fail.log`）。
7. **字节级还原**：恢复备份文件原内容 → **再次重启** Docker Desktop → 复读 `ProxyHTTPMode / OverrideProxyHTTP(S)` 与记账原值一致（先例还原读数 `manual / http://127.0.0.1:7890`）。
8. **收尾核对**：与备份 diff 为空；本次读数（切换/还原/两段日志）登记到当批重建报告。

## 附注

- Harbor/WSL portproxy 面是**另一条通道**（非本配方）：`scripts/setup_harbor_proxy.ps1`（80/443 → WSL IP 转发）。
- 若未来脚本化：switch/restore 两段须连同 **Docker Desktop 重启面与还原校验**一并设计（本轮不做，留后续批）。
- 处理批报告：[`FRICTION_INVENTORY_TREATMENT_2026-09-17`](../docs/audits/FRICTION_INVENTORY_TREATMENT_2026-09-17.md)。
