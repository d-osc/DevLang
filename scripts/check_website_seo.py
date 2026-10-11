"""Check crawler-visible HTML, canonical URLs and sitemap deployment paths."""
from pathlib import Path
from html.parser import HTMLParser
from xml.etree import ElementTree as ET
import json

root = Path(__file__).resolve().parents[1] / 'website/dist'
base = 'https://d-osc.github.io/DevLang/'
class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.canonical = []
        self.description = []
        self.headings = 0
        self.script_type = ''
        self.structured = []
    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'link' and attrs.get('rel') == 'canonical': self.canonical.append(attrs['href'])
        if tag == 'meta' and attrs.get('name') == 'description': self.description.append(attrs.get('content'))
        if tag == 'h1': self.headings += 1
        if tag == 'script': self.script_type = attrs.get('type', '')
    def handle_data(self, text):
        if self.script_type == 'application/ld+json': self.structured.append(json.loads(text))
    def handle_endtag(self, tag):
        if tag == 'script': self.script_type = ''

urls = [element.text for element in ET.parse(root / 'sitemap.xml').iter('{http://www.sitemaps.org/schemas/sitemap/0.9}loc')]
assert len(urls) == len(set(urls)), 'Duplicate sitemap URLs'
assert len(urls) == len(json.loads((root / 'seo-routes.json').read_text())), 'Route coverage mismatch'
for url in urls:
    assert url.startswith(base) and '#' not in url and '?' not in url, url
    route = url[len(base):]
    file = root / route / 'index.html'
    page = Page()
    page.feed(file.read_text(encoding='utf-8'))
    assert page.canonical == [url], f'Incorrect canonical: {url}'
    assert page.description and page.description[0], f'Missing description: {url}'
    assert page.headings > 0, f'No crawler-visible content: {url}'
    assert len(page.structured) == 1 and page.structured[0]['url'] == url, url
print(f'SEO checks passed: {len(urls)} static pages, canonical URLs, metadata, JSON-LD and sitemap')
