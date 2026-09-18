# ORZ-CI-BLINDOUT-001 — 父仓 CI 断流 16 天（2026-09-02 … 2026-09-18）事故台账

- **登记**：2026-09-18（RS-01/RS-02 处置收官批补记——事故发生期账本零记录，本文件即盲区补登）。
- **事故**：父仓 CI 自 2026-09-02 起连续红灯 16 天，4 个 Python matrix job 全部死在第一步「Check repository contracts」；5 个测试步骤断流；**v0.6.0／0.6.1／0.6.2 三次发布均在红灯下打出**。
- **根因**：[`_windows_high_nist/S4_PROGRESS_2026-09-02.md:1128`](../audits/FULL_PROJECT_STRICT_REVIEW_2026-09-18.md) 链接写为 Windows 绝对路径 `D:/CLI/docs/...`；`scripts/check_repository.py::_check_markdown_links` 的路径解析在 Windows 能解析盘符、在 Linux runner 沦为含 `D:` 目录的相对路径必断——守门者自身平台不对称（详见案例 [`ORZ-GATE-ASYM-001`](../cases/harness_environment/ORZ-GATE-ASYM-001-gate-platform-asym-fake-green.md)）。
- **掩盖期积压（恢复时一次性结算，共六层）**：①runtime 事件枚举契约钉漂移 55→65（0ac/0z 两批落事件面时无 CI 拦截，漂移 9 天）；②`ProjectDocIndex.search` 非确定 top-K（本机过、CI 挂）；③rust 对拍缺 `jsonschema`（原 CI 无 Rust 轨，新增 job 首跑暴露）；④`orz-host` session 测试 8.3 短路径断言（CI runner TEMP 形态）；⑤`codex_app` 测试 5s/8s 开发机调参上界超时；⑥门禁 manifest↔pin 一致性（处置批自身漏算，非断流期缺陷，见案例 [`ORZ-CI-BLINDOUT-001`](../cases/harness_environment/ORZ-CI-BLINDOUT-001-ci-red-blindout-cost.md) 教训条）。
- **修复**：处置六支——父仓 `667d7d8d`／`5cda9957`／`db864130`／`cae0041f`／`08307b08`＋子仓 `546f9ec5`／`1ff6bb3d`；五轮 CI 迭代；账本闭合批 `6f4fb381`。
- **验收**：run 35338101577（33m49s）五 job 全绿——契约门禁/compileall/PS 语法/grok/runtime/assurance 五步恢复常绿＋rust-tests 三连（orz-loop 791/0/3／orz-assurance／orz-host 串行）全绿。
- **全程记录**：[`FULL_PROJECT_STRICT_REVIEW_2026-09-18`](../audits/FULL_PROJECT_STRICT_REVIEW_2026-09-18.md) §2 RS-01/RS-02 批注；BACKLOG 0aq（RS-01/RS-02 已勾选）。
- **晋级的五案例**：`ORZ-CI-BLINDOUT-001`（红灯掩盖经济学）／`ORZ-GATE-ASYM-001`（守门者平台不对称）／`ORZ-DEV-TUNED-BOUND-001`（开发机调参上界与缓存掩盖）／`ORZ-GLOB-TIEBREAK-001`（非确定 top-K）／`ORZ-WIN-TEMP83-001`（TEMP 8.3 短路径断言，windows 分类）。
