"""Verify development/production graphs, non-transitive tools and CLI edits."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
d = Path(parser.parse_args().d).resolve()
repo = Path(__file__).resolve().parents[1]
count = 0
with tempfile.TemporaryDirectory(prefix='dev-devdeps-') as temporary:
    root = Path(temporary) / 'project'
    shutil.copytree(repo / 'examples/dev-dependencies', root,
                    ignore=shutil.ignore_patterns('.dev', 'dev.lock', 'package-lock.don'))
    app = root / 'app'
    def command(*args, error=None):
        global count
        result = subprocess.run([str(d), '-C', str(app), *args], capture_output=True, text=True, encoding='utf-8', timeout=45)
        if error:
            assert result.returncode != 0 and error in result.stderr, result.stderr
        else: assert result.returncode == 0, (args, result.stdout, result.stderr)
        count += 1
        return result.stdout.strip()
    def lock():
        return json.loads(subprocess.check_output([str(d), 'don', 'to-json', str(app / 'package-lock.don')], text=True))
    command('pkg', 'install')
    assert set(lock()['packages']) == {'math', 'testkit'}
    assert command('run') == '42'
    for engine in ('auto', 'ast'):
        assert command('run', 'tests/main.dev', '--engine', engine) == 'PASS'
    command('pkg', 'install', '--locked')
    command('pkg', 'install', '--production', '--locked', error='mode differs')
    command('pkg', 'install', '--production')
    assert lock()['production'] is True and set(lock()['packages']) == {'math'}
    assert command('run') == '42'
    command('run', 'tests/main.dev', error="testkit")
    command('pkg', 'install', '--production', '--locked')
    command('pkg', 'install', '--locked', error='mode differs')
    command('pkg', 'install')
    command('pkg', 'remove', 'testkit', '--dev')
    assert set(lock()['packages']) == {'math'}
    command('pkg', 'add', 'testkit', '--path', '../testkit', '--dev')
    assert command('run', 'tests/main.dev') == 'PASS'
    before = (app / 'package.don').read_bytes()
    command('pkg', 'add', 'math', '--path', '../math', '--dev', error='both dependencies')
    assert (app / 'package.don').read_bytes() == before
    (root / 'package.don').write_text("package: { name: 'devdeps_workspace' }\nworkspace: { members: ['app'] }\n", encoding='utf-8')
    command('pkg', 'install', '--workspace', '--production')
    command('pkg', 'install', '--workspace', '--production', '--locked')
    assert set(lock()['packages']) == {'math'}
    command('pkg', 'install', '--workspace')
    assert command('run', 'tests/main.dev') == 'PASS'
print(f'PASS: {count} devDependencies commands')
