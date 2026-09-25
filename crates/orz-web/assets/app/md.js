/*
 * md.js —— Markdown 渲染（照搬 marked v18.0.14，MIT，vendor/marked.umd.js；
 * 代换登记 0br S1 §4.2 R-1：orz-markdown 是 ratatui 渲染器不产 DOM）。
 *
 * 安全（审查处理批 2026-09-25 收口，双层如实描述）：
 *   ① 源文本先做 HTML 逃逸——封死原始 HTML/事件属性注入；
 *   ② 逃逸封不住 Markdown 语法本身（`[x](javascript:…)` 经 parse 仍产出
 *      <a href>）——因此 parse 后再做净空扫描：链接 scheme 白名单
 *      （http/https 之外一律剥成纯文本）、图片整体剥成 alt 文本（同时
 *      兑现「运行时零 CDN、离线可用」）、补 rel=noopener。②是承重防线。
 * 代码块内容做一次实体还原（escape-first 会让围栏内文本双重转义，
 * 还原后「模型输出 HTML 按字面呈现」才成立）。
 */

function escapeHtml(s) {
  return s
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function decodeEntities(s) {
  const ta = document.createElement('textarea');
  ta.innerHTML = s; // textarea 内容是 RCDATA，不会按 HTML 解析——安全
  return ta.value;
}

function unwrapToText(node) {
  const parent = node.parentNode;
  if (!parent) return;
  while (node.firstChild) parent.insertBefore(node.firstChild, node);
  parent.removeChild(node);
}

function hrefAllowed(href) {
  try {
    const u = new URL(href, location.href);
    return u.protocol === 'http:' || u.protocol === 'https:';
  } catch {
    return false;
  }
}

/** Parse 后净空：承重防线（scheme 白名单＋图片剥离＋事件属性清除）。 */
function scrub(root) {
  for (const a of Array.from(root.querySelectorAll('a'))) {
    const href = a.getAttribute('href') || '';
    if (hrefAllowed(href)) {
      a.setAttribute('rel', 'noopener noreferrer');
      a.setAttribute('target', '_blank');
    } else {
      unwrapToText(a); // 保留链接文字，丢弃不可信 scheme
    }
  }
  for (const img of Array.from(root.querySelectorAll('img'))) {
    const alt = img.getAttribute('alt') || '';
    img.replaceWith(document.createTextNode(`[图片${alt ? `：${alt}` : ''}]`));
  }
  for (const bad of Array.from(root.querySelectorAll('script, style, iframe, object, embed'))) {
    bad.remove();
  }
  // 表格窗口适配（走查反馈二/三 2026-09-25）：表格包进 .table-wrap 容器；
  // 主适配靠 CSS 单元格内换行（表宽锁 100%），容器滚动仅作极端兜底。
  for (const table of Array.from(root.querySelectorAll('table'))) {
    const wrap = document.createElement('div');
    wrap.className = 'table-wrap';
    table.parentNode.insertBefore(wrap, table);
    wrap.appendChild(table);
  }
  for (const node of Array.from(root.querySelectorAll('*'))) {
    for (const attr of Array.from(node.attributes)) {
      if (/^on/i.test(attr.name)) node.removeAttribute(attr.name);
    }
  }
}

export function renderMarkdown(text) {
  const src = escapeHtml(String(text ?? ''));
  try {
    // marked 全局由 vendor/marked.umd.js 挂载（UMD）。
    const html = window.marked.parse(src, { breaks: true, gfm: true });
    const div = document.createElement('div');
    div.innerHTML = html;
    // 代码块/行内码：escape-first 的双重转义在这里还原一次，围栏内的
    // `<b>` 才按字面显示为 <b> 而不是 &lt;b&gt;。
    for (const code of div.querySelectorAll('code')) {
      code.textContent = decodeEntities(code.textContent);
    }
    scrub(div);
    return div.innerHTML;
  } catch {
    return '<p></p><pre>' + escapeHtml(src) + '</pre>';
  }
}
