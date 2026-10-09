"""Test downloadable examples after extraction and validate skill/link artifacts."""
import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import subprocess
import tempfile
import zipfile


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.targets = []
    def handle_starttag(self, tag, attrs):
        for name, value in attrs:
            if name == 'href':
                self.targets.append(value)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--d', required=True)
    args = parser.parse_args()
    dist = Path(__file__).resolve().parent / 'dist'
    site = json.loads((dist / 'content.json').read_text(encoding='utf-8'))
    ids = {page['id'] for page in site['documents']}
    for page in site['documents']:
        links = Links()
        links.feed(page['html'])
        for target in links.targets:
            if target.startswith('#/docs/'):
                assert target.split('/')[2].split('?')[0] in ids, target
            elif not target.startswith(('https:', 'http:', 'mailto:', '#')):
                assert (dist / target).is_file(), target
    with zipfile.ZipFile(dist / 'downloads/devlang-skill.zip') as archive:
        for file in ('SKILL.md', 'agents/openai.yaml', 'references/core.md', 'references/advanced.md', 'scripts/verify.py'):
            assert 'devlang/' + file in archive.namelist()
    with tempfile.TemporaryDirectory(prefix='devlang-download-test-') as temporary:
        for example in site['examples']:
            folder = Path(temporary) / example['id']
            with zipfile.ZipFile(dist / example['download']) as archive:
                archive.extractall(folder)
            file = folder / Path(example['path']).name
            result = subprocess.run([args.d, str(file)], cwd=folder, capture_output=True, check=True, timeout=30)
            assert result.stdout.decode('utf-8') == example['output'], example['id']
    print('Passed: six extracted example downloads, relative imports, skill bundle and documentation links')


if __name__ == '__main__':
    main()
