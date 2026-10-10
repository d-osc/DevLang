"""Build the static docs site and validate runnable guide examples against DevLang."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
sys.path.insert(0, str(HERE / '.build-deps'))
from markdown_it import MarkdownIt

GROUPS = {
    'เริ่มต้น': ['intro', 'install', 'layout', 'cli', 'tooling'],
    'พื้นฐานภาษา': ['variables', 'types', 'operators', 'loops', 'functions', 'strings-arrays', 'modules'],
    'ข้อมูลและ abstraction': ['structs', 'enums-match', 'references', 'collections', 'generics', 'closures'],
    'ระบบและ interoperability': ['tasks', 'safety', 'ffi', 'callbacks', 'hardware', 'stdlib', 'basic-libs', 'json', 'don', 'fs-http', 'node-core', 'limitations', 'ai'],
}
REFERENCES = {
    'basic-libs-reference': ('Foundational libraries', 'docs/basic-libs.md'),
    'don-reference': ('Dev Object Notation', 'docs/don.md'),
    'node-core-reference': ('Core runtime modules', 'docs/node-core.md'),
    'fs-http-reference': ('Filesystem and HTTP API', 'docs/fs-http.md'),
    'tooling-reference': ('Developer tools', 'docs/tooling.md'),
    'json-reference': ('JSON API', 'docs/json.md'),
    'language-reference': ('Language specification', 'docs/language.md'),
    'advanced-reference': ('Advanced contracts', 'docs/advanced.md'),
    'runtime-reference': ('Runtime API', 'docs/runtime.md'),
    'stdlib-reference': ('Stdlib API', 'docs/stdlib.md'),
    'cli-reference': ('CLI reference', 'cli/README.md'),
    'architecture-reference': ('Architecture', 'docs/architecture.md'),
    'map-performance': ('Map benchmarks', 'docs/map-performance.md'),
    'runtime-performance': ('Runtime benchmarks', 'docs/runtime-compute.md'),
}
EXAMPLES = json.loads((HERE / 'content/examples.json').read_text(encoding='utf-8'))


def bundle(source, output, root):
    with zipfile.ZipFile(output, 'w', zipfile.ZIP_DEFLATED) as archive:
        for file in sorted(source.rglob('*')):
            if file.is_file() and '__pycache__' not in file.parts and '.dev-cache' not in file.parts and file.suffix not in ('.exe', '.dll', '.so'):
                archive.write(file, root + '/' + file.relative_to(source).as_posix())


def main():
    global REPO
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--d', default=str(REPO / 'dist/d/windows-x86_64/d.exe'))
    parser.add_argument('--skip-validation', action='store_true', help='Reuse captured example output from the existing build')
    args = parser.parse_args()
    materials = HERE / 'content/materials'
    if (REPO / 'docs/language.md').is_file():
        for _, path in REFERENCES.values():
            target = materials / path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(REPO / path, target)
        for path in ('examples', 'skills/devlang', 'stdlib'):
            ignored = ['.dev-cache', '__pycache__', '*.exe', '*.dll', '*.so', '*.a', '*.o']
            if path == 'stdlib':
                ignored.append('lib')
            shutil.copytree(REPO / path, materials / path, dirs_exist_ok=True,
                            ignore=shutil.ignore_patterns(*ignored))
    else:
        REPO = materials
    dist = HERE / 'dist'
    dist.mkdir(exist_ok=True)
    for name in ('index.html', 'style.css', 'app.js', 'logo.svg'):
        shutil.copy2(HERE / 'src' / name, dist / name)
    guide = (HERE / 'content/guide.md').read_text(encoding='utf-8')
    chunks = re.split(r'^# \[([^\]]+)\] (.+)\n', guide, flags=re.M)
    raw = []
    for i in range(1, len(chunks), 3):
        slug, title, text = chunks[i:i+3]
        group = next(name for name, ids in GROUPS.items() if slug in ids)
        raw.append(dict(id=slug, title=title, text='# ' + title + '\n' + text, group=group, source='website/content/guide.md', language='TH'))
    for slug, (title, path) in REFERENCES.items():
        raw.append(dict(id=slug, title=title, text=(REPO / path).read_text(encoding='utf-8'), group='Reference · EN', source=path, language='EN'))
    markdown = MarkdownIt('commonmark', {'html': False}).enable('table')
    link_map = {str((REPO / path).resolve()): slug for slug, (_, path) in REFERENCES.items()}
    documents = []
    for page in raw:
        tokens = markdown.parse(page['text'])
        toc = []
        for i, token in enumerate(tokens):
            if token.type == 'heading_open':
                anchor = 'sec-' + str(len(toc))
                token.attrSet('id', anchor)
                if token.tag != 'h1':
                    toc.append(dict(id=anchor, title=tokens[i+1].content, level=int(token.tag[1])))
                else:
                    token.attrSet('id', 'page-title')
            if token.type == 'inline':
                for child in token.children or []:
                    if child.type == 'link_open':
                        href = child.attrGet('href') or ''
                        if href and not re.match(r'^(?:https?:|#|mailto:)', href):
                            destination = (REPO / page['source']).parent / href.split('#')[0]
                            slug = link_map.get(str(destination.resolve()))
                            if slug:
                                child.attrSet('href', '#/docs/' + slug)
                            else:
                                try:
                                    path = destination.resolve().relative_to(REPO).as_posix()
                                    child.attrSet('href', 'https://github.com/d-osc/DevLang/blob/main/' + path)
                                except ValueError:
                                    child.attrSet('href', 'https://github.com/d-osc/DevLang')
        rendered = markdown.renderer.render(tokens, markdown.options, {})
        documents.append(dict(page, html=rendered, toc=toc, description=re.sub(r'[`*#]', '', page['text']).split('\n\n')[1].strip()[:140]))
    if not args.skip_validation:
        with tempfile.TemporaryDirectory(prefix='devlang-guide-') as temporary:
            validated = 0
            for token in markdown.parse(guide):
                if token.type == 'fence' and token.info.strip() in ('dev', 'dev-runtime'):
                    file = Path(temporary) / f'guide-{validated}.dev'
                    file.write_text(token.content, encoding='utf-8')
                    if token.info.strip() == 'dev':
                        subprocess.run([args.d, 'check', str(file)], capture_output=True, check=True, timeout=20)
                    results = [subprocess.run([args.d, str(file), '--engine', engine], capture_output=True, check=True, timeout=20) for engine in ('auto', 'ast')]
                    if results[0].stdout != results[1].stdout:
                        raise ValueError(f'Guide source engines differ: {file}')
                    validated += 1
            print(f'Validated {validated} guide programs in both source engines; native frontend for dev fences')
    examples = []
    captured_file = HERE / 'content/example-outputs.json'
    captured = json.loads(captured_file.read_text(encoding='utf-8')) if args.skip_validation and captured_file.is_file() else {}
    for item in EXAMPLES:
        slug, path = item['id'], item['path']
        file = REPO / path
        folder = file.parent
        # Include imported sources and C dependencies in snapshot freshness checks.
        package = [file] if folder.name == 'howto' or folder == REPO / 'examples' else sorted(
            p for p in folder.rglob('*') if p.is_file() and p.suffix in ('.dev', '.c', '.h'))
        for root in item.get('package_roots', []):
            package.extend(sorted(p for p in (REPO / root).rglob('*') if p.is_file()
                                  and p.suffix in ('.dev', '.c', '.h', '.py')))
        source_hash = hashlib.sha256(file.read_text(encoding='utf-8').encode('utf-8')).hexdigest()
        package_hash = hashlib.sha256()
        for p in package:
            package_hash.update(p.relative_to(REPO).as_posix().encode())
            package_hash.update(p.read_text(encoding='utf-8').encode())
        package_hash.update(json.dumps(item, sort_keys=True, ensure_ascii=False).encode())
        validation_hash = package_hash.hexdigest()
        if args.skip_validation:
            if captured.get(slug, {}).get('validation_sha256') != validation_hash:
                raise ValueError(f'Missing or stale captured output for {slug}; run build with --d to validate')
            output = captured[slug]['output']
        else:
            launcher = str(Path(args.d).resolve())
            mode = item.get('mode', 'both')
            check_args = item.get('native_args', [])
            if mode != 'runtime':
                subprocess.run([launcher, 'check', str(file), *check_args], cwd=REPO,
                               capture_output=True, check=True, timeout=30)
            with tempfile.TemporaryDirectory(prefix='devlang-example-') as work:
                if mode == 'native':
                    binary = str(Path(work) / 'example.exe')
                    subprocess.run([launcher, 'build', str(file), '--release', '-o', binary, *check_args],
                                   cwd=REPO, capture_output=True, check=True, timeout=90)
                    result = subprocess.run([binary], cwd=work, capture_output=True, check=True, timeout=30)
                    output = result.stdout.decode('utf-8').replace('\r\n', '\n')
                else:
                    results = [subprocess.run([launcher, str(file), '--engine', engine, *item.get('runtime_args', [])],
                                              input=item.get('stdin', '').encode(), cwd=work,
                                              capture_output=True, check=True, timeout=30)
                               for engine in ('auto', 'ast')]
                    if results[0].stdout != results[1].stdout:
                        raise ValueError(f'Example source engines differ: {path}')
                    output = results[0].stdout.decode('utf-8').replace('\r\n', '\n')
            captured[slug] = dict(source_sha256=source_hash, validation_sha256=validation_hash, output=output)
            print(f'Validated {slug} ({mode})')
        validation = 'native frontend + compiled executable' if item.get('mode') == 'native' else (
            'runtime auto + AST' if item.get('mode') == 'runtime' else 'native frontend + runtime auto + AST')
        dependencies = [dict(path=p.relative_to(REPO).as_posix(), code=p.read_text(encoding='utf-8'))
                        for p in package if p != file and p.suffix in ('.dev', '.c', '.h')
                        and not item.get('package_roots')]
        examples.append(dict(item, code=file.read_text(encoding='utf-8'), output=output,
                             validation=validation, files=dependencies, download='downloads/' + slug + '.zip'))
    if not args.skip_validation:
        captured_file.write_text(json.dumps(captured, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    downloads = dist / 'downloads'
    downloads.mkdir(exist_ok=True)
    bundle(REPO / 'skills/devlang', downloads / 'devlang-skill.zip', 'devlang')
    bundle(REPO / 'examples', downloads / 'devlang-examples.zip', 'examples')
    for example in examples:
        folder = (REPO / example['path']).parent
        with zipfile.ZipFile(downloads / (example['id'] + '.zip'), 'w', zipfile.ZIP_DEFLATED) as archive:
            files = [REPO / example['path']] if folder.name == 'howto' or folder == REPO / 'examples' else [
                p for p in folder.rglob('*') if p.is_file() and p.suffix in ('.dev', '.c', '.h', '.md')]
            for root in example.get('package_roots', []):
                files.extend(p for p in (REPO / root).rglob('*') if p.is_file()
                             and p.suffix in ('.dev', '.c', '.h', '.py', '.md'))
            for file in files:
                archive.write(file, file.relative_to(REPO).as_posix())
    docs_dir = dist / 'markdown'
    docs_dir.mkdir(exist_ok=True)
    for page in raw:
        (docs_dir / (page['id'] + '.md')).write_text(page['text'], encoding='utf-8')
    (downloads / 'devlang-guide.md').write_text(guide, encoding='utf-8')
    shutil.copy2(REPO / 'skills/devlang/SKILL.md', dist / 'SKILL.md')
    llms = '# DevLang\n\n> Source interpreter and native compiler with explicit main invocation.\n\n## AI skill\n\n- [Skill](SKILL.md): essential coding and validation rules.\n- [Skill bundle](downloads/devlang-skill.zip): self-contained references and verification helper.\n\n## Documentation\n\n' + '\n'.join(f'- [{page["title"]}](markdown/{page["id"]}.md): {page["group"]}' for page in raw)
    (dist / 'llms.txt').write_text(llms + '\n', encoding='utf-8')
    content = dict(version='v0.4.0', documents=documents, examples=examples, groups=list(GROUPS) + ['Reference · EN'])
    (dist / 'content.json').write_text(json.dumps(content, ensure_ascii=False), encoding='utf-8')
    print(f'Built {len(documents)} articles and {len(examples)} executable examples -> {dist}')


if __name__ == '__main__':
    main()
