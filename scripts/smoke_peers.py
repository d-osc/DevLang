import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--d', required=True)
d = Path(parser.parse_args().d).resolve()
repo = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='dev-peers-') as temporary:
    root = Path(temporary) / 'project'
    shutil.copytree(repo / 'examples/peer-dependencies', root, ignore=shutil.ignore_patterns('.dev', 'package-lock.don'))
    app = root / 'app'
    def command(*args, error=None):
        result = subprocess.run([str(d), '-C', str(app), *args], capture_output=True, text=True, encoding='utf-8', timeout=45)
        if error: assert result.returncode != 0 and error in result.stderr, result.stderr
        else: assert result.returncode == 0, (args, result.stdout, result.stderr)
        return result.stdout.strip()
    command('pkg', 'install')
    assert command('run') == '42'
    assert command('run', '--engine', 'ast') == '42'
    command('pkg', 'install', '--locked')
    command('pkg', 'add', 'math', '--peer', '--version', '^1.2')
    before = (app / 'package.don').read_bytes()
    command('pkg', 'add', 'math', '--peer', '--version', '^2', error='does not satisfy')
    assert (app / 'package.don').read_bytes() == before
    command('pkg', 'remove', 'math', '--peer')
    math = root / 'math/package.don'
    source = math.read_text()
    math.write_text(source.replace('1.2.0', '2.0.0'))
    command('pkg', 'install', error='does not satisfy')
    math.write_text(source)
    manifest = {'package': {'name': 'peer_app'}, 'dependencies': {'plugin': {'path': '../plugin'}}, 'devDependencies': {'math': {'path': '../math'}}}
    (app / 'package.don').write_text(json.dumps(manifest))
    command('pkg', 'install')
    before = (app / 'package-lock.don').read_bytes()
    command('pkg', 'install', '--production', error="requires peer 'math'")
    assert (app / 'package-lock.don').read_bytes() == before
    manifest.pop('devDependencies')
    (app / 'package.don').write_text(json.dumps(manifest))
    command('pkg', 'install', error="requires peer 'math'")
    # The root package can provide an importable host peer.
    manifest['package']['name'] = 'math'
    manifest['version'] = '1.2.0'
    (app / 'src/lib.dev').write_text('fn add(a i64, b i64) i64 { return a + b }\n', encoding='utf-8')
    (app / 'package.don').write_text(json.dumps(manifest), encoding='utf-8')
    command('pkg', 'install')
    assert command('run') == '42'
    assert command('run', '--engine', 'ast') == '42'
    # A peer-only manifest must not bypass verification when no lock exists.
    (app / 'package-lock.don').unlink()
    manifest['dependencies'] = {}
    manifest['peerDependencies'] = {'absent': '^1'}
    (app / 'package.don').write_text(json.dumps(manifest), encoding='utf-8')
    command('run', error='run d pkg install')
    command('pkg', 'install', error="requires peer 'absent'")
print('PASS: peer provider resolution, versions, locked checks, development mode and CLI rollback')
