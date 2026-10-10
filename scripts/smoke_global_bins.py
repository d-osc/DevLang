"""Run commands by name on an isolated global PATH; preserve the real user PATH."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
d = Path(parser.parse_args().d).resolve()
repo = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix='dev global bins ') as temporary:
    root = Path(temporary)
    home = root / 'global home'
    project = root / 'project'
    shutil.copytree(repo / 'examples/package-bin', project, ignore=shutil.ignore_patterns('.dev', 'package-lock.don'))
    app = project / 'app'
    elsewhere = root / 'unrelated directory'
    elsewhere.mkdir()
    env = dict(os.environ, DEVLANG_HOME=str(home), DEVLANG_NO_PATH_UPDATE='1')
    env['PATH'] = str(home / 'bin') + os.pathsep + env.get('PATH', '')
    def command(*args, error=None, cwd=app):
        result = subprocess.run([str(d), '-C', str(cwd), *args], env=env, capture_output=True, text=True, encoding='utf-8', timeout=45)
        if error: assert result.returncode != 0 and error in result.stderr, result.stderr
        else: assert result.returncode == 0, (args, result.stdout, result.stderr)
        return result.stdout
    def named(name, expected):
        args = ['powershell.exe', '-NoProfile', '-NonInteractive', '-Command', name + '; exit $LASTEXITCODE'] if os.name == 'nt' else [name]
        result = subprocess.run(args, cwd=elsewhere, env=env, capture_output=True, text=True, timeout=45)
        assert result.returncode == 0 and result.stdout.strip() == expected, (name, result.stdout, result.stderr)
    command('pkg', 'install', '-g')
    named('math-add', '42')
    named('hello', 'Hello from package bin')
    named('dev-check', 'PASS')
    command('pkg', 'install', '--global', '--locked')
    unrelated = root / 'other'
    unrelated.mkdir()
    (unrelated / 'package.don').write_text("package: { name: 'other' }\nbin: { hello: 'cli.dev' }", encoding='utf-8')
    (unrelated / 'cli.dev').write_text('fn main() { print("other") }\nmain()\n')
    command('pkg', 'install', '-g', cwd=unrelated, error='owned by another')
    named('hello', 'Hello from package bin')
    command('pkg', 'install', '-g', '--production')
    filename = lambda name: name + '.cmd' if os.name == 'nt' else name
    assert not (home / 'bin' / filename('dev-check')).exists()
    named('math-add', '42')
    command('pkg', 'uninstall', '--global')
    assert not (home / 'bin' / filename('hello')).exists()
    # Uninstall leaves PATH/profile settings and unrelated custom files alone.
    custom = home / 'bin' / filename('hello')
    custom.write_text('custom command', encoding='utf-8')
    command('pkg', 'install', '-g', error='refusing to overwrite')
    assert custom.read_text() == 'custom command'
    custom.unlink()
    command('pkg', 'install', '-g')
    cli = project / 'math/src/cli.dev'
    cli.write_text('fn main() { return 7 }\nreturn main()\n')
    command('pkg', 'update', '--global')
    args = ['powershell.exe', '-NoProfile', '-NonInteractive', '-Command', 'math-add; exit $LASTEXITCODE'] if os.name == 'nt' else ['math-add']
    result = subprocess.run(args, cwd=elsewhere, env=env, capture_output=True, timeout=45)
    assert result.returncode == 7
print('PASS: -g/--global, command lookup from unrelated cwd, ownership, custom files, production, update and uninstall')
