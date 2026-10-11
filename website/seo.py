"""Generate crawlable GitHub Pages documents without requiring JavaScript."""
import html
import json
import re
from urllib.parse import quote
from xml.etree import ElementTree as ET

BASE = 'https://d-osc.github.io/DevLang/'


def build_seo(dist, content, catalog):
    shell = (dist / 'index.html').read_text(encoding='utf-8')
    pages = {}
    def links(items):
        return '<ul>' + ''.join(f'<li><a href="{html.escape(path)}">{html.escape(title)}</a></li>' for path, title in items) + '</ul>'
    doc_links = [('docs/' + p['id'] + '/', p['title']) for p in content['documents']]
    example_links = [('examples/' + p['id'] + '/', p['title']) for p in content['examples']]
    intro = next(p for p in content['documents'] if p['id'] == 'intro')
    pages[''] = ('DevLang — Programming language, runtime and native compiler',
                 'DevLang language documentation, syntax, executable examples, standard libraries, package tools and AI coding skill.',
                 intro['html'] + '<h2>Documentation</h2>' + links(doc_links) + '<h2>Examples</h2>' + links(example_links), 'th')
    for page in content['documents']:
        pages['docs/' + page['id'] + '/'] = (page['title'] + ' — DevLang Docs', page['description'], page['html'], page['language'].lower())
    for page in content['examples']:
        body = f'<h1>{html.escape(page["title"])}</h1><p>{html.escape(page["description"])}</p><pre translate="no"><code>{html.escape(page["code"])}</code></pre>'
        body += '<h2>Commands</h2><pre translate="no">' + html.escape('\n'.join(page.get('commands', []))) + '</pre>'
        body += '<h2>Output</h2><pre translate="no">' + html.escape(page.get('output', '')) + '</pre>'
        body += links([('docs/' + value + '/', value) for value in page.get('docs', [])])
        pages['examples/' + page['id'] + '/'] = (page['title'] + ' — DevLang Examples', page['description'], body, 'th')
    pages['examples/'] = ('DevLang Examples', 'Executable DevLang examples with source code, commands and validated output.', '<h1>DevLang Examples</h1>' + links(example_links), 'en')
    pages['ai/'] = ('DevLang AI coding skill', 'Download the DevLang skill for AI coding agents and read language usage rules.', '<h1>DevLang AI coding skill</h1><p>Use the downloadable skill to write, explain and validate DevLang programs.</p>' + links([('SKILL.md', 'AI skill instructions'), ('downloads/devlang-skill.zip', 'Download AI skill'), ('docs/ai/', 'AI usage guide')]), 'en')
    pages['downloads/'] = ('Download DevLang', 'Install DevLang for Windows and Linux, including runtime, compiler and standard libraries.', '<h1>Download DevLang</h1><p>Windows and Linux installers, runtime, compiler and standard libraries.</p><a href="https://github.com/d-osc/DevLang/releases/latest">Download the latest release</a>', 'en')
    packages = [(f'#/packages/{p["id"]}', p.get('name', p['id'])) for p in catalog['packages']]
    pages['packages/'] = ('DevLang Package Store', 'Discover DevLang modules and command line packages hosted on GitHub.', '<h1>DevLang Package Store</h1><p>Discover modules and CLI packages hosted on GitHub.</p>' + links(packages) + links([('packages/publish/', 'Publish a package')]), 'en')
    pages['packages/publish/'] = ('Publish a DevLang package', 'Publish a DevLang module on GitHub with a root package.don manifest and version tags.', '<h1>Publish a DevLang package</h1><p>Place package.don at the repository root, validate the package, configure GitHub topics and create version tags.</p>' + links([('docs/package-store/', 'Package publishing documentation'), ('docs/package-features/', 'Package tools and security')]), 'en')

    def crawl_links(markup):
        def replace(match):
            route = match.group(1)
            path, _, query = route.partition('?')
            destination = path.strip('/') + '/'
            if destination == '/': destination = ''
            if destination not in pages: return match.group(0)
            return 'href="' + html.escape(BASE + destination + ('?' + query if query else ''), quote=True) + '"'
        return re.sub(r'href="#(/[^"\s]*)"', replace, markup)

    for path, (title, description, body, language) in pages.items():
        canonical = BASE + path
        route = '/' + path.rstrip('/')
        base = '../' * path.count('/') if path else './'
        page = shell.replace('<head>', '<head>\n  <base href="' + base + '">', 1)
        page = re.sub(r'<title>.*?</title>', '<title>' + html.escape(title) + '</title>', page)
        description = re.sub(r'\s+', ' ', description)[:300]
        page = re.sub(r'<meta name="description"[^>]*>', '<meta name="description" content="' + html.escape(description, quote=True) + '">', page)
        metadata = f'<link rel="canonical" href="{canonical}">\n<meta property="og:type" content="website">\n<meta property="og:title" content="{html.escape(title, quote=True)}">\n<meta property="og:description" content="{html.escape(description, quote=True)}">\n<meta property="og:url" content="{canonical}">\n<meta name="twitter:card" content="summary">'
        metadata += '\n<script type="application/ld+json">' + json.dumps({'@context': 'https://schema.org', '@type': 'WebPage', 'name': title, 'description': description, 'url': canonical, 'inLanguage': language, 'isPartOf': {'@type': 'WebSite', 'name': 'DevLang', 'url': BASE}}, ensure_ascii=False).replace('<', '\\u003c') + '</script>'
        page = page.replace('</head>', metadata + '\n</head>')
        page = page.replace('<html lang="th">', '<html lang="' + language + '">')
        page = page.replace('<body>', '<body data-route="' + route + '">')
        article = '<main id="content" tabindex="-1"><article class="container doc-body">' + body + '</article></main>'
        page = re.sub(r'<main id="content".*?</main>', lambda _: article, page, flags=re.S)
        target = dist / path / 'index.html'
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(crawl_links(page), encoding='utf-8')
    namespace = 'http://www.sitemaps.org/schemas/sitemap/0.9'
    ET.register_namespace('', namespace)
    root = ET.Element('{' + namespace + '}urlset')
    for path in pages:
        item = ET.SubElement(root, '{' + namespace + '}url')
        ET.SubElement(item, '{' + namespace + '}loc').text = BASE + quote(path, safe='/')
    ET.ElementTree(root).write(dist / 'sitemap.xml', encoding='utf-8', xml_declaration=True)
    (dist / 'seo-routes.json').write_text(json.dumps(list(pages)), encoding='utf-8')
    print(f'Generated {len(pages)} crawlable pages and sitemap.xml')
