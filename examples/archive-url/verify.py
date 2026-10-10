"""Build archives, serve real HTTP URLs and exercise package installs."""
import functools
import hashlib
import http.server
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
import zipfile

d = Path(sys.argv[1]).resolve()
base = Path(__file__).resolve().parent
class Quiet(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args): pass
with tempfile.TemporaryDirectory(prefix='dev-archive-url-') as temporary:
    root = Path(temporary)
    app = root / 'app'
    shutil.copytree(base / 'app', app)
    files = [(p.relative_to(base / 'library').as_posix(), p.read_bytes()) for p in (base / 'library').rglob('*') if p.is_file()]
    for extension in ('zip', 'tar', 'tar.gz', 'tgz'):
        path = root / ('math.' + extension)
        if extension == 'zip':
            with zipfile.ZipFile(path, 'w', zipfile.ZIP_DEFLATED) as archive:
                for name, data in files: archive.writestr(name, data)
        else:
            with tarfile.open(path, 'w:gz' if extension in ('tar.gz', 'tgz') else 'w') as archive:
                for name, data in files:
                    entry = tarfile.TarInfo('math-1.2.0/' + name)
                    entry.size = len(data)
                    archive.addfile(entry, io.BytesIO(data))
    with zipfile.ZipFile(root / 'bad.zip', 'w') as archive:
        archive.writestr('../escape.dev', 'bad')
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), functools.partial(Quiet, directory=str(root)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    def command(*args, error=False):
        result = subprocess.run([str(d), '-C', str(app), *args], capture_output=True, text=True, timeout=45)
        assert (result.returncode != 0) == error, (args, result.stdout, result.stderr)
        return result.stdout
    def source(filename, digest=None):
        return {'url': f'http://127.0.0.1:{server.server_port}/{filename}', 'sha256': digest or hashlib.sha256((root / filename).read_bytes()).hexdigest(), 'version': '=1.2.0'}
    def manifest(dep):
        (app / 'package.don').write_text(json.dumps({'package': {'name': 'archive_app'}, 'dependencies': {'math': dep}}), encoding='utf-8')
    try:
        for extension in ('zip', 'tar', 'tar.gz', 'tgz'):
            dep = source('math.' + extension)
            manifest(dep)
            command('pkg', 'install')
            assert command('run').strip() == '42'
            assert command('run', '--engine', 'ast').strip() == '42'
            command('pkg', 'install', '--locked')
            lock = json.loads(subprocess.check_output([str(d), "don", "to-json", str(app / 'package-lock.don')], text=True, encoding="utf-8"))
            cached = app / lock['packages']['math']['root']
            cached.rename(root / ('saved-' + extension))
            command('pkg', 'install', '--locked')
            command('pkg', 'update')
        command('pkg', 'add', 'math', '--url', dep['url'], '--sha256', dep['sha256'], '--version', '=1.2.0')
        lock = json.loads(subprocess.check_output([str(d), "don", "to-json", str(app / 'package-lock.don')], text=True, encoding="utf-8"))
        cached = app / lock['packages']['math']['root']
        source_file = cached / 'src/lib.dev'
        original = source_file.read_bytes()
        source_file.write_bytes(b'changed')
        command('pkg', 'install', '--locked', error=True)
        source_file.write_bytes(original)
        for dep in [source('bad.zip'), source('math.zip', '0' * 64), {'url': source('math.zip')['url']}, {**source('math.zip'), 'branch': 'main'}]:
            manifest(dep)
            command('pkg', 'install', error=True)
        assert not (root / 'escape.dev').exists()
        print('PASS: HTTP zip/tar/tar.gz, both engines, locked restoration, update, CLI add, bad hashes and unsafe paths')
    finally:
        server.shutdown()
        server.server_close()
        thread.join()
