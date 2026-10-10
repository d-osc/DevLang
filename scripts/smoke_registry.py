"""Publish and consume a real static registry using localhost HTTP and file URLs."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from threading import Thread

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
d = Path(parser.parse_args().d).resolve()
repo = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='dev registry ') as temporary:
    root = Path(temporary)
    library = root / 'library'
    shutil.copytree(repo / 'examples/registry/library', library, ignore=shutil.ignore_patterns('.dev', 'package-lock.don'))
    registry = root / 'registry'
    app = root / 'app'
    def command(*args, cwd=app, error=None):
        result = subprocess.run([str(d), '-C', str(cwd), *map(str, args)], text=True, encoding='utf-8', capture_output=True, timeout=45)
        if error: assert result.returncode != 0 and error in result.stderr, (args, result.stdout, result.stderr)
        else: assert result.returncode == 0, (args, result.stdout, result.stderr)
        return result.stdout.strip()
    def version(value):
        (library / 'package.don').write_text(json.dumps({'version': value, 'package': {'name': 'math'}, 'bin': {'answer': 'src/cli.dev'}, 'devDependencies': {'testkit': {'path': '../nonexistent'}}}), encoding='utf-8')
    (library / 'src/lib.dev').write_text('fn add(a i64, b i64) i64 { return a + b }\n', encoding='utf-8')
    (library / 'src/cli.dev').write_text('fn main() { print(42) }\nmain()\n', encoding='utf-8')
    (library / '.dev').mkdir()
    (library / '.dev/cache.txt').write_text('excluded', encoding='utf-8')
    version('1.2.0')
    command('pkg', 'publish', '--registry', registry, cwd=library)
    first = (registry / 'index.don').read_bytes()
    command('pkg', 'publish', '--registry', registry, cwd=library, error='already published')
    assert (registry / 'index.don').read_bytes() == first
    assert not (registry / '.publish.lock').exists()
    for value in ['1.10.0', '2.0.0', '3.0.0-beta.1']:
        version(value)
        command('pkg', 'publish', '--registry', registry, cwd=library)
    shutil.copytree(repo / 'examples/registry/app', app, ignore=shutil.ignore_patterns('.dev', 'package-lock.don'))
    (app / 'src/main.dev').write_text('use "math/lib" as math\nfn main() { print(math.add(20, 22)) }\nmain()\n', encoding='utf-8')
    class QuietHandler(SimpleHTTPRequestHandler):
        def log_message(self, *args): pass
    server = ThreadingHTTPServer(('127.0.0.1', 0), partial(QuietHandler, directory=str(registry)))
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()
    index_url = f'http://127.0.0.1:{server.server_port}/index.don'
    try:
        listing = command('pkg', 'search', '--registry', index_url, 'math', cwd=root)
        assert 'math 1.10.0' in listing and 'math 3.0.0-beta.1' in listing
        command('pkg', 'add', 'math', '--registry', index_url, '--version', '^1')
        manifest = json.loads(command('don', 'to-json', app / 'package.don'))
        assert manifest['dependencies']['math']['version'] == '=1.10.0'
        assert command('run') == '42'
        assert command('run', '--engine', 'ast') == '42'
        assert command('exec', 'answer') == '42'
        lock = json.loads(command('don', 'to-json', app / 'package-lock.don'))
        cached = app / lock['packages']['math']['root']
        assert not (cached / '.dev').exists()
        assert 'devDependencies' not in json.loads(command('don', 'to-json', cached / 'package.don'))
        command('pkg', 'install', '--locked')
        before = (app / 'package.don').read_bytes()
        command('pkg', 'add', 'math', '--registry', index_url, '--version', '^4', error='no registry version')
        assert (app / 'package.don').read_bytes() == before
        # A pinned checksum rejects tampering and rolls back a changed manifest.
        archive = registry / 'packages/math/2.0.0.tar.gz'
        original = archive.read_bytes()
        archive.write_bytes(b'corrupt archive')
        command('pkg', 'add', 'math', '--registry', index_url, error='sha256 mismatch')
        assert (app / 'package.don').read_bytes() == before
        archive.write_bytes(original)
        command('pkg', 'add', 'math', '--registry', index_url)
        assert json.loads(command('don', 'to-json', app / 'package.don'))['dependencies']['math']['version'] == '=2.0.0'
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
    oversized = root / 'oversized.don'
    oversized.write_bytes(b' ' * (1024 * 1024 + 1))
    command('pkg', 'search', '--registry', oversized.as_uri(), cwd=root, error='download exceeds')
    (registry / '.publish.lock').write_text('another writer', encoding='utf-8')
    command('pkg', 'publish', '--registry', registry, cwd=library, error='cannot lock registry')
    assert (registry / '.publish.lock').read_text(encoding='utf-8') == 'another writer'
    (registry / '.publish.lock').unlink()
    command('pkg', 'publish', '--registry', library / 'inside', cwd=library, error='outside the source')
    # Locked cached installation does not need the registry online.
    command('pkg', 'install', '--locked')
    command('pkg', 'add', 'math', '--registry', (registry / 'index.don').as_uri(), '--version', '=1.2.0')
    assert command('run') == '42'
    command('pkg', 'add', 'math', '--registry', (registry / 'index.don').as_uri(), '--version', '=3.0.0-beta.1')
    assert command('run') == '42'
    command('pkg', 'remove', 'math')
    command('pkg', 'add', 'math', '--registry', (registry / 'index.don').as_uri(), '--version', '^1', '--dev')
    assert command('run') == '42'
    command('pkg', 'install', '--production')
    assert 'math' not in json.loads(command('don', 'to-json', app / 'package-lock.don'))['packages']
    command('pkg', 'install')
    assert command('run') == '42'
    current_lock = json.loads(command('don', 'to-json', app / 'package-lock.don'))
    current_cache = app / current_lock['packages']['math']['root']
    command('pkg', 'remove', 'math', '--dev')
    edited = current_cache / 'src/lib.dev'
    edited.write_text('fn add(a i64, b i64) i64 { return 99 }\n', encoding='utf-8')
    before_manifest = (app / 'package.don').read_bytes()
    command('pkg', 'add', 'math', '--registry', (registry / 'index.don').as_uri(), '--version', '^1', '--dev', error='cached archive dependency changed')
    assert (app / 'package.don').read_bytes() == before_manifest
    assert '99' in edited.read_text(encoding='utf-8')
    version('4.0.0')
    manifest = json.loads((library / 'package.don').read_text(encoding='utf-8'))
    manifest['dependencies'] = {'local': {'path': '../local'}}
    (library / 'package.don').write_text(json.dumps(manifest), encoding='utf-8')
    before = (registry / 'index.don').read_bytes()
    command('pkg', 'publish', '--registry', registry, cwd=library, error='portable')
    assert (registry / 'index.don').read_bytes() == before
print('PASS: static registry publishing, HTTP/file lookup, SemVer selection, bins, locked offline install, duplicate versions, checksum tampering and portable sources')
