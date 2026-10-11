'use strict';
// Keep legacy hash bookmarks working, while exposing real crawlable page URLs.
(() => {
  const base = new URL('.', document.baseURI);
  let routes;
  function updateLinks(root = document) {
    if (!routes) return;
    root.querySelectorAll('a[href^="#/"]').forEach(link => {
      const route = link.getAttribute('href').slice(1);
      const [path, query = ''] = route.split('?');
      const page = path === '/' ? '' : path.slice(1).replace(/\/$/, '') + '/';
      if (!routes.has(page)) return;
      const destination = new URL(page, base);
      const parameters = new URLSearchParams(query);
      const language = new URL(location.href).searchParams.get('lang');
      if (language === 'en' || language === 'th') parameters.set('lang', language);
      destination.search = parameters.toString();
      link.href = destination.href;
    });
  }
  fetch(new URL('seo-routes.json', base)).then(response => {
    if (!response.ok) throw new Error('SEO route map unavailable');
    return response.json();
  }).then(data => {
    routes = new Set(data);
    updateLinks();
    new MutationObserver(records => {
      for (const record of records) for (const node of record.addedNodes) {
        if (node.nodeType === Node.ELEMENT_NODE) updateLinks(node.parentElement || document);
      }
    }).observe(document.body, {childList: true, subtree: true});
  }).catch(() => {}); // Original hash navigation remains usable if offline.
})();
