"""Exercise real bin execution, launchers, arguments and production pruning."""
import argparse
from pathlib import Path
import os
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
d = Path(parser.parse_args().d).resolve()
repo = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='dev package bins ') as temporary:
    root = Path(temporary) / 'project'
    shutil.copytree(repo / 'examples/package-bin', root, ignore=shutil.ignore_patterns('.dev', 'package-lock.don'))
    app = root / 'app'
    def command(*args, error=None):
        result = subprocess.run([str(d), '-C', str(app), *args], capture_output=True, text=True, encoding='utf-8', timeout=45)
        if error: assert result.returncode != 0 and error in result.stderr, result.stderr
        else: assert result.returncode == 0, (args, result.stdout, result.stderr)
        return result.stdout.strip()
    command('pkg', 'install')
    assert 'math-add' in command('pkg', 'bin')
    arguments = ['two words', '--help', '--version', '-C', 'literal']
    assert command('exec', 'hello', '--', *arguments).splitlines() == ['Hello from package bin', *arguments]
    assert command('exec', 'hello', '--help').splitlines()[-1] == '--help'
    assert command('exec', 'math-add') == '42'
    assert command('exec', 'dev-check') == 'PASS'
    launcher = app / '.dev/bin' / ('math-add.cmd' if os.name == 'nt' else 'math-add')
    invocation = ['cmd.exe', '/d', '/c', str(launcher)] if os.name == 'nt' else [str(launcher)]
    result = subprocess.run(invocation, capture_output=True, text=True, timeout=45)
    assert result.returncode == 0 and result.stdout.strip() == '42', result
    command('pkg', 'install', '--locked')
    custom = app / '.dev/bin/custom.txt'
    custom.write_text('user file')
    dev_launcher = app / '.dev/bin' / ('dev-check.cmd' if os.name == 'nt' else 'dev-check')
    command('pkg', 'install', '--production')
    assert not dev_launcher.exists() and custom.read_text() == 'user file'
    command('exec', 'dev-check', error='unknown bin')
    assert command('exec', 'math-add') == '42'
    command('pkg', 'install')
    cli = root / 'math/src/cli.dev'
    cli.write_text('fn main() { return 7 }\nreturn main()\n')
    command('pkg', 'install')
    result = subprocess.run([str(d), '-C', str(app), 'exec', 'math-add'], capture_output=True)
    assert result.returncode == 7
    manifest = root / 'testkit/package.don'
    before = manifest.read_text()
    manifest.write_text(before.replace('dev-check', 'math-add'))
    command('pkg', 'install', error='conflicting bin')
    manifest.write_text(before)
    manifest.write_text(before.replace('src/cli.dev', '../outside.dev'))
    command('pkg', 'install', error='relative paths')
    manifest.write_text(before)
    launcher.write_text('custom launcher')
    lock_before = (app / 'package-lock.don').read_bytes()
    command('pkg', 'install', error='refusing to overwrite')
    assert launcher.read_text() == 'custom launcher'
    assert (app / 'package-lock.don').read_bytes() == lock_before
print('PASS: package/local/dev bins, real launchers, literal arguments, exit codes, collisions and production pruning')
