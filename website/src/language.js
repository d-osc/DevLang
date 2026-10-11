'use strict';

// Google translates public prose. Executable source and command output stay intact.
(() => {
  const supported = new Set(['en', 'th']);
  const url = new URL(location.href);
  let stored;
  try { stored = localStorage.getItem('devlang-language'); } catch (_) {}
  const requested = url.searchParams.get('lang');
  const language = supported.has(requested) ? requested : supported.has(stored) ? stored : 'th';
  if (supported.has(requested)) {
    try { localStorage.setItem('devlang-language', language); } catch (_) {}
  }
  document.documentElement.lang = language;

  const switcher = document.createElement('div');
  switcher.className = 'language-switch notranslate';
  switcher.setAttribute('translate', 'no');
  switcher.setAttribute('role', 'group');
  switcher.setAttribute('aria-label', 'Language / ภาษา');
  for (const [value, label, name] of [['en', 'EN', 'English'], ['th', 'TH', 'ไทย']]) {
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = label;
    button.title = name;
    button.setAttribute('aria-label', name);
    button.setAttribute('aria-pressed', String(value === language));
    button.addEventListener('click', () => {
      try { localStorage.setItem('devlang-language', value); } catch (_) {}
      const next = new URL(location.href);
      next.searchParams.set('lang', value);
      // A fresh document prevents translated DOM from becoming application state.
      location.assign(next.href);
    });
    switcher.append(button);
  }
  document.querySelector('.header-actions').prepend(switcher);

  function protectCode(root = document) {
    root.querySelectorAll('pre, code, kbd, .code-block, .code-tabs, .terminal, .output, #hero-terminal, [data-code-tab]').forEach(node => {
      node.classList.add('notranslate');
      node.setAttribute('translate', 'no');
    });
  }
  protectCode();
  new MutationObserver(records => {
    for (const record of records) for (const node of record.addedNodes) {
      if (node.nodeType === Node.ELEMENT_NODE) {
        if (node.matches('pre, code, kbd, .code-block, .output, #hero-terminal')) {
          node.classList.add('notranslate'); node.setAttribute('translate', 'no');
        }
        protectCode(node);
      }
    }
  }).observe(document.body, {childList: true, subtree: true});

  window.devlangInitializeLanguage = data => {
  if (language === 'en') {
    const labels = new Map([
      ['คู่มือ', 'Documentation'], ['ตัวอย่าง', 'Examples'], ['ดาวน์โหลด', 'Downloads'],
      ['ค้นหาคู่มือ', 'Search documentation'], ['ข้ามไปเนื้อหา', 'Skip to content'],
      ['เมนูหลัก', 'Main navigation'], ['DevLang หน้าแรก', 'DevLang home'],
      ['สลับธีมมืด/สว่าง', 'Toggle dark / light theme'], ['ปิดการค้นหา', 'Close search'],
      ['ค้นหาในคู่มือ', 'Search documentation'], ['ผลการค้นหา', 'Search results'],
      ['ค้นหา syntax, Vec, unsafe, main…', 'Search syntax, Vec, unsafe, main…'],
      ['Operators และ if / else', 'Operators and if / else'], ['ฟังก์ชันและ generics', 'Functions and generics'],
      ['Structs และ methods', 'Structs and methods'], ['Enums และ match', 'Enums and match'],
      ['Arrays, Vec, Slice และ Map', 'Arrays, Vec, Slice and Map'], ['Ref และ recursive data', 'Ref and recursive data'],
      ['Function values และ closures', 'Function values and closures'], ['Modules และ export', 'Modules and export']
    ]);
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode;
      if (node.parentElement.closest('.notranslate, pre, code, script, style')) continue;
      const translated = labels.get(node.textContent.trim());
      if (translated) node.textContent = node.textContent.replace(node.textContent.trim(), translated);
    }
    document.querySelectorAll('[aria-label], [placeholder]').forEach(node => {
      for (const attribute of ['aria-label', 'placeholder']) {
        const value = node.getAttribute(attribute);
        if (labels.has(value)) node.setAttribute(attribute, labels.get(value));
      }
    });
  }
  const docId = location.hash.match(/^#\/docs\/([^?]+)/)?.[1];
  const source = language === 'en' ? 'th' : data.documents.find(doc => doc.id === docId)?.language === 'EN' ? 'en' : 'th';
  // Reload on navigation so the translator never becomes application state, and
  // each reference article uses its actual source language.
  let previousRoute = location.hash.split('?')[0];
  window.addEventListener('hashchange', () => {
    const nextRoute = location.hash.split('?')[0];
    if (nextRoute !== previousRoute && (language === 'en' || source === 'en' || nextRoute.startsWith('#/docs/'))) location.reload();
    previousRoute = nextRoute;
  });
  if (source === language) return;
  const status = document.getElementById('language-status');
  const message = language === 'en'
    ? 'Automatic English translation · Code and commands keep their original text.'
    : 'แปลภาษาไทยอัตโนมัติ · โค้ดและคำสั่งคงข้อความต้นฉบับ';
  status.textContent = message;
  status.hidden = false;
  function unavailable() {
    status.textContent = language === 'en'
      ? 'Translation unavailable. Original content is shown. Reload to try again.'
      : 'บริการแปลภาษาไม่พร้อม แสดงเนื้อหาต้นฉบับ กรุณารีเฟรชเพื่อลองอีกครั้ง';
    status.dataset.state = 'error';
  }
  const timer = setTimeout(unavailable, 20000);
  window.devlangTranslateReady = () => {
    try {
      new google.translate.TranslateElement({pageLanguage: source, includedLanguages: 'en,th', autoDisplay: false}, 'language-service');
      let attempts = 0;
      const selectLanguage = () => {
        const combo = document.querySelector('#language-service select.goog-te-combo');
        if (!combo || !Array.from(combo.options).some(option => option.value === language)) {
          if (++attempts < 80) setTimeout(selectLanguage, 200);
          return;
        }
        clearTimeout(timer);
        combo.value = language;
        combo.dispatchEvent(new Event('change', {bubbles: true}));
      };
      selectLanguage();
    } catch (_) { clearTimeout(timer); unavailable(); }
  };
  const script = document.createElement('script');
  script.src = 'https://translate.google.com/translate_a/element.js?cb=devlangTranslateReady';
  script.async = true;
  script.onerror = () => { clearTimeout(timer); unavailable(); };
  document.head.append(script);
  };
})();
