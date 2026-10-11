'use strict';
let packageCatalog = null, packageLoading = false, packageError = false;
const packageState = {query: '', kind: 'all', sort: 'updated', page: 1};
const packageRoute = id => '#/packages/' + id.split('/').map(encodeURIComponent).join('/');
function packageIntro() {
  return `<div class="store-heading"><div><span class="eyebrow">DEVLANG ECOSYSTEM</span><h1>Package Store<span class="store-dot">.</span></h1><p>ค้นหา module และ CLI จาก GitHub<br>เลือก package แล้วนำไปใช้ในโปรเจกต์ของคุณ</p></div><a class="button primary" href="#/packages/publish">เผยแพร่ package ของคุณ ↗</a></div>`;
}
function packagesPage(parts) {
  if (parts[0] === 'status') return packageStatus();
  if (parts[0] === 'publish') return packagePublish();
  return `<div class="container store-page">${parts.length ? '' : packageIntro()}<div id="package-view" aria-live="polite">กำลังอ่านรายการ package…</div></div>`;
}
async function loadPackages(refresh = false) {
  if (location.hash.startsWith('#/packages/publish')) return;
  if (packageCatalog && !refresh) { paintPackages(); return; }
  if (packageLoading) return;
  packageLoading = true; packageError = false;
  try {
    const response = await fetch('packages.json', {cache: 'no-cache'});
    if (!response.ok) throw new Error('Catalog unavailable');
    const data = await response.json();
    if (data.schema_version !== 1 || !Array.isArray(data.packages)) throw new Error('Invalid catalog');
    packageCatalog = data;
  } catch (_) { packageError = true; }
  finally { packageLoading = false; paintPackages(); }
}
function paintPackages() {
  const view = document.getElementById('package-view'); if (!view) return;
  if (packageError) { view.innerHTML = '<div class="store-empty"><h2>อ่านรายการไม่สำเร็จ</h2><p>กรุณาลองโหลดรายการอีกครั้ง</p><button class="button primary" data-package-refresh>ลองใหม่</button></div>'; return; }
  if (!packageCatalog) return;
  let id;
  try { id = location.hash.slice(2).split('?')[0].split('/').slice(1).map(decodeURIComponent).join('/'); }
  catch (_) { id = '?'; }
  if (id === 'status') { paintPackageStatus(); return; }
  if (id) { view.innerHTML = packageDetail(id); prepareCode(); loadPackageDocuments(id); return; }
  view.innerHTML = `<div class="store-toolbar"><label class="store-search">ค้นหา package<input id="package-search" type="search" placeholder="ชื่อ package, owner หรือคำอธิบาย" value="${escapeHTML(packageState.query)}"></label><label>ประเภท<select id="package-kind"><option value="all">ทั้งหมด</option><option value="module">Module</option><option value="cli">CLI</option></select></label><label>เรียงตาม<select id="package-sort"><option value="updated">อัปเดตล่าสุด</option><option value="stars">ดาว GitHub</option><option value="name">ชื่อ package</option></select></label><button class="button small" data-package-refresh>โหลดรายการใหม่ ↻</button></div><div class="store-status"><span id="package-count"></span><span>ตรวจรายการ ${escapeHTML(new Date(packageCatalog.generated_at).toLocaleString('th-TH'))}</span></div><div id="package-results"></div><div class="store-note"><strong>รู้ที่มาของทุก package</strong><p>แต่ละรายการระบุ owner/repo และตรวจ manifest ที่ root กับไฟล์ source แล้ว การตรวจนี้ไม่ได้รับรองความปลอดภัยหรือผลทดสอบของโค้ด กรุณาอ่าน source ก่อนใช้งาน</p><a href="#/packages/publish">ดูเกณฑ์และวิธีเผยแพร่ →</a> · <a href="#/packages/status">ตรวจสถานะ repo →</a>${packageCatalog.limited ? '<p>แสดงบางส่วนตามขีดจำกัดการค้นหา GitHub</p>' : ''}</div>`;
  document.getElementById('package-kind').value = packageState.kind;
  document.getElementById('package-sort').value = packageState.sort;
  filterPackages();
}
function filterPackages() {
  const q = packageState.query.trim().toLowerCase();
  const items = packageCatalog.packages.filter(p => `${p.id} ${p.name} ${p.description} ${p.topics.join(' ')}`.toLowerCase().includes(q) && (packageState.kind === 'all' || (packageState.kind === 'cli' ? Object.keys(p.bin).length > 0 : Object.keys(p.bin).length === 0)));
  items.sort((a,b) => packageState.sort === 'stars' ? b.stars-a.stars || a.id.localeCompare(b.id) : packageState.sort === 'name' ? a.name.localeCompare(b.name) || a.id.localeCompare(b.id) : String(b.updated_at).localeCompare(String(a.updated_at)) || a.id.localeCompare(b.id));
  document.getElementById('package-count').textContent = `${items.length} / ${packageCatalog.packages.length} packages`;
  const size = 12, pages = Math.max(1, Math.ceil(items.length / size));
  packageState.page = Math.max(1, Math.min(packageState.page, pages));
  const visible = items.slice((packageState.page - 1) * size, packageState.page * size);
  document.getElementById('package-results').innerHTML = items.length ? `<div class="package-grid">${visible.map(packageCard).join('')}</div>` : `<div class="store-empty"><span class="store-empty-icon" aria-hidden="true">{ }</span><h2>${packageCatalog.packages.length ? 'ไม่พบ package ที่ค้นหา' : 'เริ่มสร้าง ecosystem ไปด้วยกัน'}</h2><p>${packageCatalog.packages.length ? 'ลองเปลี่ยนคำค้นหาหรือประเภท' : 'ยังไม่มี repo ที่ผ่านเกณฑ์ในรายการล่าสุด ดาวน์โหลด template แล้วเผยแพร่ package แรกของคุณได้เลย'}</p><a class="button primary" href="downloads/devlang-package-template.zip">ดาวน์โหลด package template ↓</a><a class="button" href="#/packages/publish">วิธีเผยแพร่ →</a></div>`;
  if (items.length) document.getElementById('package-results').innerHTML += `<div class="store-pagination"><button class="button small" data-package-page="${packageState.page-1}" ${packageState.page===1?'disabled':''}>← ก่อนหน้า</button><span>หน้า ${packageState.page} / ${pages}</span><button class="button small" data-package-page="${packageState.page+1}" ${packageState.page===pages?'disabled':''}>ถัดไป →</button></div>`;
}
function packageCard(p) {
  return `<a class="package-card" href="${packageRoute(p.id)}"><div class="package-card-top"><span class="package-glyph" aria-hidden="true">${Object.keys(p.bin).length ? '>_' : '{}'}</span><span class="package-version">${p.latest ? 'v' + escapeHTML(p.latest.version) : 'ยังไม่มี release tag'}</span></div><h2>${escapeHTML(p.name)}</h2><div class="package-owner">${escapeHTML(p.id)}</div><p>${escapeHTML(p.description || 'ไม่มีคำอธิบายจาก GitHub')}</p><div class="package-card-meta"><span>☆ ${escapeHTML(p.stars)}</span><span>${escapeHTML(p.license || 'ไม่ระบุ license')}</span><span>${Object.keys(p.bin).length ? 'CLI' : 'Module'}</span></div><span class="package-checked">✓ ตรวจ manifest แล้ว</span> ${packageQuality(p)}</a>`;
}
function packageDetail(id) {
  const p = packageCatalog.packages.find(item => item.id === id);
  if (!p) return '<div class="store-empty"><h1>ไม่พบ package นี้</h1><a class="button primary" href="#/packages">กลับ Package Store</a></div>';
  const links = `<a class="button" href="${escapeHTML(p.repository)}" target="_blank" rel="noopener">ดู GitHub ↗</a><a class="button" href="${escapeHTML(p.repository)}/blob/${encodeURIComponent(p.checked_commit)}/${p.subdir?p.subdir.split('/').map(encodeURIComponent).join('/')+'/':''}package.don" target="_blank" rel="noopener">ดู package.don ↗</a>`;
  return `<a class="store-back" href="#/packages">← Package Store</a><div class="store-heading"><div><span class="eyebrow">${escapeHTML(p.id)}</span><h1>${escapeHTML(p.name)}</h1><p>${escapeHTML(p.description || 'ไม่มีคำอธิบายจาก GitHub')}</p></div><span class="package-checked">✓ ตรวจ manifest แล้ว</span></div><div class="package-detail-layout"><section><h2>ติดตั้งในโปรเจกต์</h2>${p.latest ? `<label class="store-version-select">เลือก tag<select id="package-version" data-package-id="${escapeHTML(p.id)}">${p.versions.map(v => `<option value="${escapeHTML(v.tag)}" ${v.tag === p.latest.tag ? 'selected' : ''}>${escapeHTML(v.tag)}</option>`).join('')}</select></label><div id="package-install">${packageInstall(p,p.latest)}</div>` : '<div class="store-note"><h3>ยังไม่มี tag ที่ติดตั้งได้จาก Store</h3><p>ผู้พัฒนาใช้ manifest ที่ถูกต้องแล้ว แต่ต้องสร้าง SemVer tag ให้ตรงกับ version ใน package.don ก่อน</p></div>'}<div class="store-note"><h3>ก่อนใช้งาน</h3><p>อ่าน README, license และ source ของ repo ก่อนติดตั้ง รายการนี้ตรวจรูปแบบ manifest, ไฟล์ source และแสดงผล GitHub CI โดย crawler ไม่รันโค้ดจาก repo</p><p>Git tag อาจเปลี่ยนได้ ควรเก็บ package-lock.don เพื่อระบุ commit ที่ติดตั้งจริง</p></div></section><aside class="package-facts"><h2>ข้อมูล package</h2><dl><dt>Namespace</dt><dd>${escapeHTML(p.name)}</dd><dt>Owner / repo</dt><dd>${escapeHTML(p.id)}</dd><dt>Source directory</dt><dd>${escapeHTML(p.modules)}</dd><dt>Source version</dt><dd>${escapeHTML(p.source_version)}</dd><dt>License</dt><dd>${escapeHTML(p.license || 'ไม่ระบุ')}</dd><dt>ดาว GitHub</dt><dd>${escapeHTML(p.stars)}</dd><dt>Commands</dt><dd>${escapeHTML(Object.keys(p.bin).join(', ') || 'ไม่มี')}</dd><dt>Dependencies</dt><dd>${escapeHTML(p.dependencies.join(', ') || 'ไม่มี')}</dd><dt>Peer dependencies</dt><dd>${escapeHTML(Object.entries(p.peers).map(([name,req]) => name + ': ' + req).join(', ') || 'ไม่มี')}</dd></dl>${packageQuality(p)}<p class="store-commit">CI ของ default branch ณ commit ที่ตรวจ ไม่ได้ยืนยันผลทดสอบ release ทุก tag</p>${links}</aside></div><section id="package-documents" class="package-reader">กำลังอ่าน README และเอกสาร…</section>`;
}
function packageInstall(p,v) {
  return codeBlock(`d pkg add @${p.id} --as ${p.name} --version =${v.version}\nd pkg install --locked`, 'sh', 'Terminal') + `<p class="store-commit">ตรวจ tag ${escapeHTML(v.tag)} ที่ commit <a href="${escapeHTML(p.repository)}/tree/${encodeURIComponent(v.commit)}" target="_blank" rel="noopener">${escapeHTML(v.commit.slice(0,12))}</a></p>`;
}
function packagePublish() {
  return `<div class="container store-page"><a class="store-back" href="#/packages">← Package Store</a><div class="store-heading"><div><span class="eyebrow">BUILD SOMETHING USEFUL</span><h1>เผยแพร่ package<span class="store-dot">.</span></h1><p>ใช้ GitHub เป็นบ้านของ source และ version<br>ให้ DevLang Package Store ช่วยค้นพบผลงานของคุณ</p></div><a class="button primary" href="downloads/devlang-package-template.zip">ดาวน์โหลด template ↓</a></div><div class="publish-steps"><section><span class="step-number">01</span><h2>วาง manifest ที่ level 0</h2><p>หนึ่ง repo ต่อหนึ่ง package โดย package.don อยู่ที่ root เสมอ ชื่อ repo เช่น devlang-math ส่วน namespace ใช้ math ตามกฎ identifier ของภาษา</p>${codeBlock('devlang-math/\n├── package.don\n├── src/lib.dev\n├── examples/main.dev\n├── tests/main.dev\n├── README.md\n└── LICENSE', 'text', 'Repository')}${codeBlock("version: '1.0.0'\npackage: {\n  name: 'math'\n  modules: 'src'\n}", 'don', 'package.don')}<p>Monorepo ใช้ package.don ที่ root ประกาศ workspace.members ได้ Store จะตรวจและแสดงสมาชิกด้วยตัวตน @owner/repo/namespace ใช้ tag namespace/v1.0.0 สำหรับสมาชิก</p></section><section><span class="step-number">02</span><h2>ตั้งค่า GitHub ให้ค้นพบ</h2><p>ใช้ repo สาธารณะที่ไม่ใช่ fork และไม่ถูก archive ไปที่หน้า repo → About → ปุ่มเฟือง → Topics แล้วเพิ่ม</p><div class="store-topics"><code>devlang</code><code>devlang-package</code><code>devlang-module</code></div><p>ต้องมี devlang-package สำหรับการค้นหา ส่วน devlang-cli ใช้กับ command line package และ devlang-registry ใช้กับ repo ของ registry เพิ่ม About description และ license เพื่อให้ผู้อื่นเลือกใช้งานง่ายขึ้น</p></section><section><span class="step-number">03</span><h2>ตรวจ source แล้วออก tag</h2><p>modules ต้องมี source .dev จริง และ bin ต้องชี้ไฟล์ที่มีอยู่ version ใช้ SemVer และต้องตรงกับ tag เช่น version 1.0.0 กับ v1.0.0</p>${codeBlock('d pkg inspect --json\nd examples/main.dev\nd tests/main.dev\ngit add .\ngit commit -m "Release 1.0.0"\ngit tag v1.0.0\ngit push origin main --tags', 'sh', 'Terminal')}<p>คำสั่ง inspect ต้องใช้ CLI รุ่นที่มีคำสั่งนี้ ดาวน์โหลดจาก source ปัจจุบันได้ อย่าเปลี่ยน tag ของ release ที่เผยแพร่แล้ว</p></section><section><span class="step-number">04</span><h2>พร้อมให้ค้นพบและติดตั้ง</h2><p>Store ค้นหา GitHub ใหม่ทุก 6 ชั่วโมง และเมื่อเว็บ deploy โดย GitHub อาจใช้เวลา index topic ใหม่ รายการใช้ owner/repo แยกตัวตน จึงไม่ถือว่าชื่อ namespace เดียวกันเป็น package เดียวกัน</p><p>ตรวจ snapshot ของ default branch และสูงสุด 400 tag เรียง SemVer และเลือก stable version ก่อน prerelease Package ที่ยังไม่มี tag ตรง version จะแสดง source แต่ยังไม่มีคำสั่งติดตั้ง</p><a class="button" href="https://github.com/d-osc/DevLang/issues/new?template=package-submission.yml" target="_blank" rel="noopener">แจ้ง repo / ปัญหาการขึ้นรายการ ↗</a><a class="button" href="#/docs/package-store">อ่านรายละเอียดทั้งหมด →</a></section></div><div class="store-note"><strong>Topic ช่วยค้นพบ ไม่ใช่ตรารับรอง</strong><p>Store ไม่รัน source จาก repo ภายนอก ไม่รับรองความปลอดภัย และไม่รับประกันว่าชื่อ package จะไม่ซ้ำ ตรวจเจ้าของ repo และ source ก่อนติดตั้งทุกครั้ง</p></div></div>`;
}
document.addEventListener('input', e => { if (e.target.id === 'package-search') { packageState.query = e.target.value; packageState.page = 1; filterPackages(); } });
document.addEventListener('change', e => {
  if (e.target.id === 'package-doc-version') loadPackageDocuments(e.target.dataset.packageId,e.target.value||null);
  if (e.target.id === 'package-kind') { packageState.kind = e.target.value; packageState.page = 1; filterPackages(); }
  if (e.target.id === 'package-sort') { packageState.sort = e.target.value; packageState.page = 1; filterPackages(); }
  if (e.target.id === 'package-version') { const p = packageCatalog.packages.find(p => p.id === e.target.dataset.packageId), v = p?.versions.find(v => v.tag === e.target.value); if (v) { document.getElementById('package-install').innerHTML = packageInstall(p,v); prepareCode(); loadPackageDocuments(p.id,v.tag); } }
});
document.addEventListener('click', e => { if (e.target.closest('[data-package-refresh]')) { const view = document.getElementById('package-view'); if (view) view.innerHTML = 'กำลังอ่านรายการ package…'; loadPackages(true); } });

function packageQuality(p) {
  const labels = {passed:'CI ผ่าน',skipped:'CI ข้ามการทดสอบ',failed:'CI ไม่ผ่าน',pending:'CI กำลังรัน',unknown:'ยังไม่มีผล CI',unavailable:'อ่านผล CI ไม่ได้',partial:'ผล CI บางส่วน'};
  const state = p.quality?.state || 'unknown';
  return `<span class="package-ci">${escapeHTML(labels[state] || labels.unknown)}</span><p class="store-commit">DevLang CLI: ${escapeHTML(p.devlang || 'ไม่ระบุช่วงเวอร์ชัน')}${p.compatibility ? ' · ' + (p.compatibility.compatible ? 'ตรงกับ CLI ที่ตรวจ ' : 'ไม่ตรงกับ CLI ที่ตรวจ ') + escapeHTML(p.compatibility.cli_version) : ''}</p>`;
}
async function loadPackageDocuments(id,tag=null) {
  const container = document.getElementById('package-documents'); if (!container) return;
  const p = packageCatalog.packages.find(p => p.id === id);
  if (!p) return;
  const selector='<h2>README / API / Changelog</h2><label>เวอร์ชันเอกสาร<select id="package-doc-version" data-package-id="'+escapeHTML(id)+'"><option value="">Default branch</option>'+p.versions.map(v=>'<option value="'+escapeHTML(v.tag)+'" '+(v.tag===tag?'selected':'')+'>'+escapeHTML(v.tag)+'</option>').join('')+'</select></label>';
  container.dataset.requestedTag=tag||'';
  const version=tag?p?.versions.find(v=>v.tag===tag):null;
  const path=version?version.details:p?.details; const commit=version?version.commit:p?.checked_commit;
  if (!path || !/^package-details\/[a-f0-9]{64}\.json$/.test(path)) { container.innerHTML = selector+'<p>เอกสารเวอร์ชันนี้ยังไม่อยู่ใน snapshot · <a target="_blank" rel="noopener" href="'+escapeHTML(p.repository)+'/tree/'+encodeURIComponent(commit)+(p.subdir?'/'+p.subdir.split('/').map(encodeURIComponent).join('/'):'')+'">อ่านจาก GitHub ↗</a></p>'; return; }
  try {
    const response = await fetch(path); if (!response.ok) throw new Error('Documents unavailable');
    const details = await response.json();
    if (details.id !== id || details.commit !== commit) throw new Error('Document snapshot mismatch');
    if (document.getElementById('package-documents')!==container || container.dataset.requestedTag!==(tag||'') || !location.hash.endsWith('/' + id.split('/').map(encodeURIComponent).join('/'))) return;
    container.innerHTML = '<h2>README / API / Changelog</h2><label>เวอร์ชันเอกสาร<select id="package-doc-version" data-package-id="'+escapeHTML(id)+'"><option value="">Default branch</option>'+p.versions.map(v=>'<option value="'+escapeHTML(v.tag)+'" '+(v.tag===tag?'selected':'')+'>'+escapeHTML(v.tag)+'</option>').join('')+'</select></label><p class="store-commit">เอกสารที่ commit ' + escapeHTML(details.commit.slice(0,12)) + '</p>' + (details.documents.length ? details.documents.map(doc => `<details open><summary>${escapeHTML(doc.path)}</summary><article class="article-body">${doc.html}</article><a href="${escapeHTML(doc.source)}" target="_blank" rel="noopener">ดูต้นฉบับ ↗</a></details>`).join('') : '<p>repo นี้ยังไม่มี README.md, API.md, CHANGELOG.md หรือ docs/*.md</p>');
    prepareCode();
  } catch (_) { if(container.dataset.requestedTag!==(tag||'')) return; container.innerHTML = selector+'<p>อ่านเอกสารไม่สำเร็จ</p><button class="button" data-package-doc-retry="' + escapeHTML(id) + '" data-package-doc-tag="'+escapeHTML(tag||'')+'">ลองใหม่</button>'; }
}
function packageStatus() {
  return '<div class="container store-page"><a class="store-back" href="#/packages">← Package Store</a><div class="store-heading"><div><span class="eyebrow">INDEX STATUS</span><h1>สถานะการตรวจ repo</h1><p>ค้นหา owner/repo เพื่อดูว่าผ่านเกณฑ์ รอตรวจ หรือไม่ผ่านเพราะอะไร</p></div></div><div id="package-view">กำลังอ่านสถานะ…</div></div>';
}
function paintPackageStatus() {
  const rows = [...packageCatalog.packages.map(p => ({repository:p.id, state:'ผ่าน manifest',reason:p.latest?'มี release tag ที่ตรวจผ่าน':'ยังไม่มี tag ตรงกับ version',tags:p.ignored_tags||[]})), ...(packageCatalog.rejected||[]).map(p=>({...p,state:'ไม่ผ่าน'})), ...(packageCatalog.deferred||[]).map(p=>({...p,state:'รอตรวจรอบถัดไป'}))];
  document.getElementById('package-view').innerHTML = `<label class="store-search">ค้นหา repo<input id="package-status-search" type="search" placeholder="owner/repo"></label><p class="store-commit">ตรวจเมื่อ ${escapeHTML(new Date(packageCatalog.generated_at).toLocaleString('th-TH'))} · GitHub requests ${escapeHTML(packageCatalog.requests||0)} · cache hits ${escapeHTML(packageCatalog.cache_hits||0)}</p><div id="package-status-results"></div><p>แสดงสูงสุด 100 ผลลัพธ์ต่อคำค้น กรุณาค้นหา owner/repo หากรายการมีจำนวนมาก</p><p>ถ้า repo ไม่อยู่ในรายการ อาจยังไม่ติด topic devlang-package, เป็น private/fork/archive หรือ GitHub ยังไม่ index topic</p><a class="button" href="#/packages/publish">ดูเกณฑ์ทั้งหมด →</a>`;
  packageStatusRows = rows; filterPackageStatus('');
}
let packageStatusRows = [];
function filterPackageStatus(query) {
  const rows = packageStatusRows.filter(p => p.repository.toLowerCase().includes(query.toLowerCase())).slice(0,100);
  document.getElementById('package-status-results').innerHTML = rows.length ? rows.map(p => `<div class="store-note"><strong>${escapeHTML(p.repository)}</strong><span class="package-checked">${escapeHTML(p.state)}</span><p>${escapeHTML(p.reason)}</p>${(p.tags||[]).map(t=>`<p>Tag ${escapeHTML(t.tag)}: ${escapeHTML(t.reason)}</p>`).join('')}</div>`).join('') : '<div class="store-empty">ยังไม่มี repo ตรงกับคำค้นใน snapshot นี้</div>';
}
document.addEventListener('input',e=>{if(e.target.id==='package-status-search') filterPackageStatus(e.target.value);});
document.addEventListener('click',e=>{
  const page = e.target.closest('[data-package-page]'); if(page){packageState.page=Number(page.dataset.packagePage);filterPackages();}
  const retry = e.target.closest('[data-package-doc-retry]'); if(retry) loadPackageDocuments(retry.dataset.packageDocRetry,retry.dataset.packageDocTag||null);
});
