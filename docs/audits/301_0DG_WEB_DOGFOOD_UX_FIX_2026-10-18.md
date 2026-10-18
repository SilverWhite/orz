# 301 批：0dg 两件落码——web 对话卡 pre-wrap＋Ctrl+Z 输入框豁免与闲置反馈（GAP-WEB-DOGFOOD-UX-FIX）（2026-10-18）

> **用户令**：「接下来请进行0dg吧」（294 批立项；293 批 N1/N2 取证落码；零模型面改动，
> orz-tui 不受影响不改——285 批「双实现同步纪律」本批两件不适用＝TUI 无此缺陷）。
> **落码 orz `ba77aacb`（5 文件 +68/−4，不推送）**。

## §1 ①对话卡换行渲染（293 N1）

- **根因**（293 批取证）：`widgets.js:388-389` 非「模型」卡 `body.textContent` 直填，
  `app.css:242` `.msg-card .msg-body` 无 `white-space: pre-wrap` ⇒ HTML 默认
  `white-space: normal` 把 `\n` 折叠为空格；存档/回放面 `\n` 本身逐字保留（缺陷 web 层独有）。
- **修法**：`widgets.js` 非模型卡 body 加 `.plain` 类（`msg-body plain`）；`app.css` 两条
  白名单规则——`.msg-card .msg-body.plain { white-space: pre-wrap; }`＋`.system-line`
  同面补 `pre-wrap`（系统行多行错误/透传不折叠）。pre-wrap 只作用纯文本直填面，
  **模型卡 markdown 面不动**（`\n` 由段落化消费）。
- **随批裁决（登记）**：① markdown 渲染替代案**维持不做**——零 md 管道注入面（用户输入
  不过 renderMarkdown），plain 保留源换行已满足 N1 判据；② 「取消运行」语义**不拆分**
  （输入框外 Ctrl+Z 语义原样）。

## §2 ②Ctrl+Z 输入框豁免＋闲置反馈（293 N2）

- **根因**：`keymap.js:79-83` Ctrl/Cmd+Z 全局分支在 `inInput` 豁免逻辑之外 ⇒ 输入框内
  原生文本撤销被抢占；`main.js:140` `stopRun` 闲置期静默 return（293「不好使」体感根因）。
- **修法**：Ctrl+Z 分支补 `!inInput`——输入框内放行原生撤销（不 `preventDefault`、不调
  `stopRun`），输入框外「取消运行」语义保留；`stopRun` 闲置臂
  `flash('当前无运行中')`（与运行中「已发送取消」同通道 4s 横幅）。
- **服务端链路不动**：`acp.sessionCancel` → 泵 → `cancel_current_run` 机制在位（293 §N2 取证）。

## §3 钉子（frontend_smoke.mjs §12 两枚）

- ① `.plain`/`.msg-body` 作用面断言（非模型卡带 `.plain`＋textContent 保 `\n`；模型卡
  不带）＋`app.css` 两规则字面断言（`.msg-body.plain` pre-wrap＋`.system-line` pre-wrap）。
- ② Ctrl+Z TEXTAREA：零 `preventDefault`＋横幅文案不变（不触发取消）；BODY：
  `preventDefault` 恰一次＋横幅＝「当前无运行中」（闲置反馈非静默）。

## §4 读数

- `node crates/orz-web/tests/frontend_smoke.mjs` ⇒ **全部通过**（含新增两节）。
- `cargo test -p orz-web` ⇒ **47/0**。
- clippy：orz-web 本体零告警（输出中 19 条全为依赖 orz-tools/orz-assurance 预存基线）；
  触碰件零 Rust 文件，fmt 不适用。

## §5 边界与不取项

1. 零模型面改动、零契约面、零 orz-web Rust 改动（assets 为静态文件）。
2. N3（撤回/中途补充）不在本项——已立项 0di（296 批）。
3. 真机观感复验随下一 web 狗粮轮（钉子面已覆盖作用域与语义；浏览器渲染层
   `white-space` 生效性＝CSS 声明直接成立，无 JS 分支）。
4. 载体：web 桥装机位为源码直服（assets 随仓），无独立重建步骤；0.9.x 载体字节
   判据不含本批面（orz-web assets 非重建判据域，沿 285 批先例）。

## §6 台账

- 本档；索引头行 v4.273 → **v4.274**；BACKLOG 指针行/计数行（65 不变）/P1 路由行/`0dg`
  专节两件达成注记；TODO 计数行/P1 路由行/`P1-0dg` 两件勾选；[`BACKLOG 第二卷 §1.241`](../BACKLOG_AND_PRIORITIES_2.md)；
  orz_source_manifest 重算 1,488 entries（恰 5 行变化＝触碰文件）。
- **计数 65 不变**（两件落码达成＝闭合待用户裁决）。**不推送**。
- **门禁执法随批如实记**：首次跑门禁报 `ledger consistency: TODO 全部勾选但 BACKLOG 仍记开放——token: 0dg`——沿 0cv 先例补 `[ ] 闭合` 待裁决框（TODO `P1-0dg` 尾）后 `valid: true`（error_count 0）。

## §7 关键词

301 批、0dg、GAP-WEB-DOGFOOD-UX-FIX、pre-wrap、msg-body plain、system-line、
Ctrl+Z、inInput 豁免、原生文本撤销、stopRun 闲置反馈、当前无运行中、
frontend_smoke §12、47/0、计数 65 不变、索引 v4.274。
