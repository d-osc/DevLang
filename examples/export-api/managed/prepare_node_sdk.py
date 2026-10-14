"""Fetch matching Node headers/import library from nodejs.org and verify SHA-256."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
from urllib.request import urlopen

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', default='out/node-sdk')
args = parser.parse_args()
version = subprocess.check_output(['node', '-p', 'process.version'], text=True).strip()
arch = subprocess.check_output(['node', '-p', 'process.arch'], text=True).strip()
assert re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+', version), version
assert arch in ['x64', 'arm64'], arch
base = f'https://nodejs.org/download/release/{version}/'
checks = {}
for line in urlopen(base + 'SHASUMS256.txt', timeout=30).read().decode().splitlines():
    digest, name = line.split(maxsplit=1)
    checks[name.strip().lstrip('*')] = digest

def fetch(name):
    blob = urlopen(base + name, timeout=90).read()
    assert hashlib.sha256(blob).hexdigest() == checks[name], name
    return blob

output = Path(args.output).resolve()
output.mkdir(parents=True, exist_ok=True)
archive_name = f'node-{version}-headers.tar.gz'
with tarfile.open(fileobj=io.BytesIO(fetch(archive_name))) as archive:
    archive.extractall(output, filter='data')
configuration = {'version': version, 'include': str(output / f'node-{version}/include/node')}
if os.name == 'nt':
    library = output / 'node.lib'
    library.write_bytes(fetch(f'win-{arch}/node.lib'))
    configuration['library'] = str(library)
(output / 'sdk.json').write_text(json.dumps(configuration, indent=2) + '\n', encoding='utf-8')
print(f'Verified Node SDK {version} {arch}: {output}')
