'use strict';
const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => Array.from(root.querySelectorAll(selector));
const escapeHTML = text => String(text).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const repo = 'https://github.com/d-osc/DevLang';
const release = repo + '/releases/download/v0.4.0/';
let site, previousFocus, toastTimer;
const hello = 'fn main() {\n    let answer = 19 + 23\n    print("Hello Dev")\n    print(answer)\n}\n\nmain()';
try { document.body.dataset.theme = localStorage.getItem('devlang-theme') || 'light'; } catch (_) {}

function highlight(code, numbers = false) {
  const tokens = /("(?:\\.|[^"\\])*"|\/\/[^\n]*|#[^\n]*|\b(?:fn|let|return|struct|enum|trait|const|if|else|while|for|in|break|continue|match|use|as|export|extern|unsafe|async|true|false|null|and|or|not)\b|\b(?:i8|i16|i32|i64|u8|u16|u32|u64|usize|isize|f32|f64|bool|str|void|Ref|Vec|Map|Slice|Task|Self|Number|Integer|Equatable|ContextCallback)\b|\b(?:0x[\da-fA-F_]+|\d+(?:\.\d+)?)(?:e[+-]?\d+)?\b)/g;
  let result = '', last = 0;
  for (const match of code.matchAll(tokens)) {
    result += escapeHTML(code.slice(last, match.index));
    const token = match[0];
    const kind = token.startsWith('"') ? 'string' : /^(?:\/\/|#)/.test(token) ? 'comment' : /^\d/.test(token) ? 'number' : /^(?:[iu](?:\d+|size)|f\d+|bool|str|void|Ref|Vec|Map|Slice|Task|Self|Number|Integer|Equatable|ContextCallback)$/.test(token) ? 'type' : 'key';
    result += `<span class="syntax-${kind}">${escapeHTML(token)}</span>`;
    last = match.index + token.length;
  }
  result += escapeHTML(code.slice(last));
  return numbers ? result.split('\n').map((line, i) => `<span class="code-line" data-line="${i+1}">${line}</span>`).join('\n') : result;
}
function codeBlock(code, language = 'dev', filename = '') {
  return `<div class="code-block"><div class="code-toolbar"><span>${escapeHTML(filename || language.toUpperCase())}</span><button class="copy-code" aria-label="คัดลอกโค้ด">Copy</button></div><pre><code class="language-${escapeHTML(language)}">${language.startsWith('dev') ? highlight(code) : escapeHTML(code)}</code></pre></div>`;
}
function prepareCode() {
  $$('.doc-body pre').forEach(pre => {
    const code = $('code', pre), text = code.textContent;
    const language = code.className.replace('language-', '') || 'text';
    const block = document.createElement('div');
    block.innerHTML = codeBlock(text, language);
    pre.replaceWith(block.firstElementChild);
  });
  $$('.doc-body table').forEach(table => {
    const wrapper = document.createElement('div'); wrapper.className = 'table-wrap';
    table.replaceWith(wrapper); wrapper.append(table);
  });
}
function home() {
  return `<div class="container">
    <section class="hero"><div><span class="pill"><i></i> DevLang v0.4.0 · อยู่ในช่วงพัฒนา</span><h1>เขียนให้น้อย<br><span class="accent">ทำให้ได้มาก</span></h1><p>ภาษาเล็กที่ให้คุณเริ่มจากไอเดีย<br>รัน source ได้ทันที สร้าง native code ได้เมื่อพร้อม<br>และเชื่อมต่อกับ C ด้วย syntax ที่ตรงไปตรงมา</p><div class="hero-actions"><a class="button primary" href="#/docs/intro">เริ่มเขียน DevLang <span>→</span></a><a class="button secondary" href="${repo}" target="_blank" rel="noreferrer">ดูบน GitHub <span>↗</span></a></div><div class="hero-note">Source runtime & native compiler · Windows / Linux</div></div>
    <div class="code-window"><div class="window-top"><span class="dot"></span><span class="dot"></span><span class="dot"></span><span class="filename">hello.dev</span><span class="language">DEVLANG</span></div><div class="hero-code"><pre><code>${highlight(hello, true)}</code></pre></div><div class="code-tabs" role="tablist" aria-label="วิธีรัน"><button role="tab" aria-selected="true" class="active" data-code-tab="runtime">Source runtime</button><button role="tab" aria-selected="false" data-code-tab="native">Native build</button></div><div class="terminal" id="hero-terminal"><span class="prompt">$</span> d hello.dev<div class="output">Hello Dev\n42</div><small>ผลลัพธ์ตัวอย่าง · รันจริงด้วย d ในเครื่อง</small></div></div></section>
    <section class="feature-strip" aria-label="ความสามารถหลัก"><div><span class="feature-number">01 / SIMPLE</span><strong>Syntax สั้น ความหมายชัด</strong><p>ไม่ต้องมี semicolon · อนุมานชนิดตัวแปรได้</p></div><div><span class="feature-number">02 / FLEXIBLE</span><strong>รันได้ก่อน แล้วค่อย build</strong><p>Pure Dev ใช้ runtime ได้โดยไม่ต้องมี compiler</p></div><div><span class="feature-number">03 / CONNECTED</span><strong>ไปต่อกับระบบจริง</strong><p>Modules · Managed collections · C interoperability</p></div></section>
    <section class="section"><div class="section-heading"><div><span class="eyebrow">LEARN BY BUILDING</span><h2>จากโปรแกรมแรก ถึงงานที่ซับซ้อน</h2><p>อธิบายทีละส่วน พร้อม syntax และตัวอย่างที่ตรวจด้วย runtime จริง</p></div><a class="text-link" href="#/docs/intro">เปิดคู่มือทั้งหมด →</a></div><div class="learn-grid">
    ${[
      ['{ }','พื้นฐานที่ใช้ทุกวัน','ตัวแปร ชนิดข้อมูล เงื่อนไข loops และ functions เริ่มเขียนได้จากโปรแกรมแรก','layout','01 — LANGUAGE BASICS'],
      ['<T>','จัดข้อมูลให้เป็นระบบ','Structs, payload enums, generics และ collections ที่ใช้ value semantics','structs','02 — DATA & TYPES'],
      ['↗','เชื่อมต่อ และทำงานพร้อมกัน','Modules, C FFI, callbacks และ thread-backed tasks พร้อมขอบเขตการใช้งาน','tasks','03 — SYSTEMS']
    ].map(([icon,title,text,id,label]) => `<a class="learn-card" href="#/docs/${id}"><div class="card-icon">${icon}</div><h3>${title}</h3><p>${text}</p><div class="card-bottom"><span>${label}</span><span>→</span></div></a>`).join('')}</div></section>
    <section class="banner"><div><span class="eyebrow">BUILT FOR YOUR WORKFLOW</span><h3>ให้ AI เข้าใจภาษา DevLang ของคุณ</h3><p>ดาวน์โหลด skill พร้อมกฎภาษา reference และเครื่องมือตรวจโค้ด</p></div><a class="button" href="#/ai">ดู AI skill <span>→</span></a></section></div>`;
}
function docs(id, section) {
  const page = site.documents.find(page => page.id === id);
  if (!page) return notFound();
  document.title = page.title + ' — DevLang Docs';
  const index = site.documents.indexOf(page), previous = site.documents[index-1], next = site.documents[index+1];
  const sidebar = site.groups.map(group => `<div class="side-title">${escapeHTML(group)}</div>${site.documents.filter(doc => doc.group === group).map(doc => `<a href="#/docs/${doc.id}" class="${id === doc.id ? 'active' : ''}" ${id === doc.id ? 'aria-current="page"' : ''}>${escapeHTML(doc.title)}</a>`).join('')}`).join('');
  return `<div class="docs-layout"><aside class="docs-sidebar" aria-label="หัวข้อคู่มือ"><button class="doc-menu-toggle" data-close-menu>ปิดเมนู ×</button>${sidebar}</aside><div><button class="doc-menu-toggle" data-doc-menu>☰ เลือกหัวข้อคู่มือ</button><article class="doc-article"><div class="breadcrumbs"><a href="#/">DevLang</a><span>/</span><span>${escapeHTML(page.group)}</span></div><div class="doc-body">${page.html}</div><div class="doc-bottom"><span>อ้างอิง implementation v0.4 · ${page.language}</span><a href="markdown/${page.id}.md" download>ดาวน์โหลด Markdown ↓</a><a href="${repo}/blob/main/${page.source}" target="_blank" rel="noreferrer">ดู source ↗</a></div><div class="doc-pagination">${previous ? `<a href="#/docs/${previous.id}">← ${escapeHTML(previous.title)}</a>` : '<span></span>'}${next ? `<a href="#/docs/${next.id}">${escapeHTML(next.title)} →</a>` : ''}</div></article></div><aside class="docs-toc" aria-label="สารบัญในหน้านี้"><p>ในหน้านี้</p>${page.toc.map(item => `<a href="#/docs/${id}?section=${item.id}" data-scroll="${item.id}">${escapeHTML(item.title)}</a>`).join('')}<p style="margin-top:27px">DEVLANG v0.4.0</p><a href="#/docs/limitations">ความสามารถและข้อจำกัด ↗</a></aside></div>`;
}
function examples(category = 'ทั้งหมด') {
  const categories = ['ทั้งหมด', ...new Set(site.examples.map(example => example.category))];
  const cards = site.examples.filter(example => category === 'ทั้งหมด' || example.category === category);
  return `<div class="container"><div class="page-heading"><span class="eyebrow">EXAMPLE COLLECTION</span><h1>เริ่มจากตัวอย่างที่รันได้จริง</h1><p>อ่านโค้ด ดูผลลัพธ์ แล้วดาวน์โหลดไปรันด้วย d ทุกตัวอย่างมาจาก repository จริง</p></div><div class="filter-bar" aria-label="หมวดตัวอย่าง">${categories.map(item => `<button data-filter="${escapeHTML(item)}" class="${item === category ? 'active' : ''}" aria-pressed="${item === category}">${escapeHTML(item)}</button>`).join('')}</div><div class="example-grid">${cards.map(example => `<article class="example-card"><div class="mini-code" aria-hidden="true">${highlight(example.code.split('\n').slice(0,5).join('\n'))}</div><div class="card-body"><span class="eyebrow">${escapeHTML(example.category)}</span><h2><a href="#/examples/${example.id}">${escapeHTML(example.title)}</a></h2><p>${escapeHTML(example.description)}</p></div><div class="card-bottom"><a class="text-link" href="#/examples/${example.id}">เปิดตัวอย่าง →</a><a href="${example.download}" download aria-label="ดาวน์โหลด ${escapeHTML(example.title)}">ZIP ↓</a></div></article>`).join('')}</div></div>`;
}
function example(id) {
  const item = site.examples.find(item => item.id === id);
  if (!item) return notFound();
  document.title = item.title + ' — DevLang Examples';
  return `<div class="container"><div class="page-heading"><a class="text-link" href="#/examples">← ตัวอย่างทั้งหมด</a><h1>${escapeHTML(item.title)}</h1><p>${escapeHTML(item.description)}</p></div><div class="example-detail"><div>${codeBlock(item.code, 'dev', item.path)}<div class="notice">ZIP รวม module dependencies เพื่อรักษา relative imports รันไฟล์ที่แสดงจากโฟลเดอร์ที่แตกไฟล์แล้ว</div></div><aside class="example-aside"><h3>ผลลัพธ์ที่ตรวจไว้</h3><p>บันทึกจาก DevLang source runtime ขณะสร้างคู่มือ ไม่ใช่การรันใน browser</p><pre>${escapeHTML(item.output)}</pre><h3>รันในเครื่อง</h3>${codeBlock('d ' + item.path.split('/').pop(), 'sh')}<a class="button primary" href="${item.download}" download>ดาวน์โหลดตัวอย่าง ↓</a><a class="text-link" href="${repo}/blob/main/${item.path}" target="_blank" rel="noreferrer">ดูใน repository ↗</a></aside></div></div>`;
}
function ai() {
  return `<div class="container"><div class="page-heading"><span class="eyebrow">AI-READY DOCUMENTATION</span><h1>ให้ AI เขียน DevLang ได้ถูกทาง</h1><p>Skill แบบเปิดที่รวมกฎสำคัญของภาษา ข้อจำกัดที่ควรรู้ และวิธีตรวจโค้ดจาก runtime จริง</p></div><div class="ai-layout"><div><span class="pill">devlang / SKILL.md</span><h2>Context ที่ตรงกับภาษา</h2><p>ลดการเดา syntax จากภาษาอื่น ให้ AI เข้าใจ explicit main, value semantics, COW collections และความต่างระหว่าง source runtime กับ native compiler</p><ol><li>ดาวน์โหลดและแตก skill ZIP</li><li>วางโฟลเดอร์ <code>devlang</code> ใน <code>~/.codex/skills/</code></li><li>เริ่ม session ใหม่ แล้วเรียก <code>$devlang</code></li></ol><a class="button primary" href="downloads/devlang-skill.zip" download>ดาวน์โหลด AI skill <span>↓</span></a><p>AI tools ที่อ่าน Markdown ได้สามารถใช้ SKILL.md และ references เป็น context ได้ Skill ไม่ได้เป็นตัวรันภาษา</p></div><div>${codeBlock('~/.codex/skills/devlang/\n├── SKILL.md\n├── agents/openai.yaml\n├── references/\n│   ├── core.md\n│   └── advanced.md\n└── scripts/verify.py', 'text', 'Skill bundle')}<div class="ai-prompt">${codeBlock('ใช้ $devlang เขียนโปรแกรมคำนวณผลรวมด้วย Vec\nให้รันด้วย source runtime และตรวจ native build\nอธิบายผลและข้อจำกัดที่พบ', 'text', 'ตัวอย่าง prompt')}</div><a class="text-link" href="SKILL.md">อ่าน SKILL.md ↗</a> &nbsp; <a class="text-link" href="llms.txt">เปิด llms.txt ↗</a></div></div><div class="banner"><div><h3>คู่มือสำหรับคน และ context สำหรับ AI</h3><p>ดาวน์โหลด Markdown ฉบับเต็ม หรือเปิด reference ในเว็บไซต์</p></div><a class="button" href="downloads/devlang-guide.md" download>ดาวน์โหลดคู่มือ ↓</a></div></div>`;
}
function downloads() {
  return `<div class="container"><div class="page-heading"><span class="eyebrow">DEVLANG v0.4.0</span><h1>พร้อมเริ่มเขียนแล้วหรือยัง?</h1><p>ติดตั้ง compiler, source runtime, stdlib และ examples ในชุดเดียว</p></div><div class="download-grid"><section class="download-card featured"><span class="os">⊞ Windows</span><h2>ไฟล์เดียว กดติดตั้งได้เลย</h2><span class="tag">Windows 10 / 11 · x86_64 · Offline EXE</span><p>เปิดไฟล์แล้วกด Install เพิ่ม user PATH ให้อัตโนมัติ และถอนการติดตั้งผ่าน Installed apps ได้</p><a class="button primary" href="${release}devlang-setup-v0.4.0-windows-x86_64.exe">ดาวน์โหลด Setup.exe ↓</a><ul><li>รวม d, devc, devrun, stdlib และ examples</li><li>ไม่ต้องแตก ZIP หรือใช้ administrator</li><li>เปิด terminal ใหม่หลังติดตั้ง</li></ul></section><section class="download-card"><span class="os">&gt;_ Linux</span><h2>Offline installer สำหรับผู้ใช้</h2><span class="tag">x86_64 · Python 3.9+ · glibc 2.39+</span><p>ติดตั้งใน ~/.local/share/devlang และเชื่อมคำสั่งเข้า ~/.local/bin</p><a class="button secondary" href="${release}devlang-setup-v0.4.0-linux-x86_64.py">ดาวน์โหลด Linux installer ↓</a>${codeBlock('python3 devlang-setup-v0.4.0-linux-x86_64.py\nexport PATH="$HOME/.local/bin:$PATH"', 'sh')}</section></div><div class="notice">ตัวติดตั้งยังไม่ได้ลงลายเซ็นดิจิทัล · Pure Dev runtime ไม่ต้องมี C compiler ส่วน native builds และ first-use C source FFI ต้องมี C backend ที่เหมาะสม อ่านรายละเอียดในคู่มือติดตั้ง</div><div class="download-extra"><a href="#/docs/install">วิธีติดตั้งและถอนการติดตั้ง →</a><a href="${repo}/releases/tag/v0.4.0" target="_blank" rel="noreferrer">ทุกแพ็กเกจบน GitHub ↗</a><a href="${release}SHA256SUMS">SHA256 checksums ↗</a><a href="downloads/devlang-examples.zip" download>ตัวอย่างทั้งหมด ↓</a></div></div>`;
}
function notFound() { return '<div class="container"><div class="page-heading"><h1>ไม่พบหน้านี้</h1><p>เลือกหัวข้อจากคู่มือ หรือกลับไปหน้าแรก</p><a class="button primary" href="#/">กลับหน้าแรก →</a></div></div>'; }
function render() {
  const route = location.hash.slice(1) || '/';
  const [path, query] = route.split('?'), parts = path.split('/').filter(Boolean);
  document.title = 'DevLang — เขียนให้น้อย ทำให้ได้มาก';
  const page = parts[0];
  $('#content').innerHTML = page === 'docs' ? docs(parts[1] || 'intro') : page === 'examples' ? (parts[1] ? example(parts[1]) : examples(new URLSearchParams(query).get('category') || 'ทั้งหมด')) : page === 'ai' ? ai() : page === 'downloads' ? downloads() : !page ? home() : notFound();
  $$('[data-nav]').forEach(link => link.classList.toggle('active', link.dataset.nav === page));
  prepareCode();
  window.scrollTo(0, 0);
  const section = new URLSearchParams(query).get('section');
  if (section && /^[\w-]+$/.test(section)) requestAnimationFrame(() => document.getElementById(section)?.scrollIntoView());
}
function toast(message) { $('#toast').textContent = message; $('#toast').classList.add('visible'); clearTimeout(toastTimer); toastTimer = setTimeout(() => $('#toast').classList.remove('visible'), 2200); }
async function copy(text) {
  try {
    if (navigator.clipboard && window.isSecureContext) await navigator.clipboard.writeText(text);
    else { const area = document.createElement('textarea'); area.value = text; area.style.position = 'fixed'; area.style.opacity = '0'; document.body.append(area); area.select(); const ok = document.execCommand('copy'); area.remove(); if (!ok) throw new Error('Copy unavailable'); }
    toast('คัดลอกแล้ว');
  } catch (_) { toast('คัดลอกไม่ได้ กรุณาเลือกข้อความและกด Ctrl+C'); }
}
function search() {
  const query = $('#search-input').value.trim().toLocaleLowerCase();
  const matches = site.documents.map(doc => {
    const title = doc.title.toLocaleLowerCase(), text = doc.text.toLocaleLowerCase(), index = text.indexOf(query);
    const score = !query ? (doc.language === 'TH' ? 1 : 0) : title.includes(query) ? 10 : index >= 0 ? (doc.language === 'TH' ? 3 : 2) : 0;
    return {doc, score, index};
  }).filter(item => item.score).sort((a,b) => b.score - a.score).slice(0, 12);
  $('#search-results').innerHTML = matches.length ? matches.map(({doc,index}) => `<a href="#/docs/${doc.id}" data-search-result><strong>${escapeHTML(doc.title)}</strong><span>${escapeHTML(doc.group)}</span><span>${escapeHTML(query ? doc.text.slice(Math.max(0,index-30),Math.max(0,index-30)+120).replace(/[#`*\n]/g,' ') : doc.description)}${query ? '…' : ''}</span></a>`).join('') : '<p>ไม่พบหัวข้อ ลองคำว่า main, generics, Vec หรือ unsafe</p>';
}
function openSearch() { if (!site) return; previousFocus = document.activeElement; $('#search-dialog').showModal(); $('#search-input').value = ''; search(); $('#search-input').focus(); }
$('#search-open').addEventListener('click', openSearch);
$('#search-close').addEventListener('click', () => $('#search-dialog').close());
$('#search-dialog').addEventListener('close', () => previousFocus?.focus());
$('#search-dialog').addEventListener('click', e => { if (e.target === $('#search-dialog')) { const r = e.target.getBoundingClientRect(); if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) e.target.close(); } });
$('#search-input').addEventListener('input', search);
$('#search-input').addEventListener('keydown', e => { if (e.key === 'ArrowDown') { $('#search-results a')?.focus(); e.preventDefault(); } if (e.key === 'Enter') $('#search-results a')?.click(); });
$('#search-results').addEventListener('keydown', e => { if (!['ArrowDown','ArrowUp'].includes(e.key)) return; const links = $$('#search-results a'), index = links.indexOf(document.activeElement); links[Math.max(0,Math.min(links.length-1,index+(e.key==='ArrowDown'?1:-1)))]?.focus(); e.preventDefault(); });
document.addEventListener('keydown', e => { if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); openSearch(); } if (e.key === 'Escape') $('.docs-sidebar')?.classList.remove('open'); });
$('#theme-toggle').addEventListener('click', () => { document.body.dataset.theme = document.body.dataset.theme === 'dark' ? 'light' : 'dark'; try { localStorage.setItem('devlang-theme', document.body.dataset.theme); } catch (_) {} });
document.addEventListener('click', e => {
  const target = e.target.closest('button,a'); if (!target) return;
  if (target.matches('.skip-link')) { e.preventDefault(); $('#content').focus(); return; }
  if (target.matches('.copy-code')) copy($('code', target.closest('.code-block')).textContent);
  if (target.dataset.filter) location.hash = '/examples?category=' + encodeURIComponent(target.dataset.filter);
  if (target.hasAttribute('data-doc-menu')) $('.docs-sidebar').classList.add('open');
  if (target.hasAttribute('data-close-menu')) $('.docs-sidebar').classList.remove('open');
  if (target.hasAttribute('data-search-result')) $('#search-dialog').close();
  if (target.dataset.codeTab) {
    $$('.code-tabs button').forEach(button => { const active = button === target; button.classList.toggle('active', active); button.setAttribute('aria-selected', active); });
    $('#hero-terminal').innerHTML = target.dataset.codeTab === 'runtime' ? '<span class="prompt">$</span> d hello.dev<div class="output">Hello Dev\n42</div><small>ผลลัพธ์ตัวอย่าง · รันจริงด้วย d ในเครื่อง</small>' : '<span class="prompt">$</span> d build hello.dev --release --output out/hello<div class="output">$ ./out/hello\nHello Dev\n42</div><small>ตัวอย่างคำสั่ง Linux · Windows ใช้ executable .exe · ต้องมี C backend</small>';
  }
});
fetch('content.json').then(response => { if (!response.ok) throw new Error('Content unavailable'); return response.json(); }).then(data => { site = data; render(); window.addEventListener('hashchange', render); }).catch(() => { $('#content').innerHTML = '<div class="loading">เปิดเนื้อหาไม่สำเร็จ กรุณารีเฟรชหน้าเว็บ หรืออ่าน <a href="downloads/devlang-guide.md">คู่มือ Markdown</a></div>'; });
