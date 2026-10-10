"""Validate checked-in package examples in isolated copies, preserving source files."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
args = parser.parse_args()
d = Path(args.d).resolve()
repo = Path(__file__).resolve().parents[1]
count = 0
with tempfile.TemporaryDirectory(prefix='dev-package-examples-') as temporary:
    root = Path(temporary)
    for name, package, expected, workspace in [
        ('tooling', 'app', '42', False),
        ('workspace', 'app', '42', True),
        ('package-don', 'demo_app', 'Hello from package.don\n42', True),
        ('don', None, 'DevLang demo\n3000\n8080\nfalse\n{"name":"Dev"}', False),
    ]:
        project = root / name
        shutil.copytree(repo / 'examples' / name, project,
                        ignore=shutil.ignore_patterns('.dev', '.dev-cache', 'out', 'dev.lock', 'package-lock.don'))
        cwd = project / 'app' if name == 'tooling' else project
        def command(*options):
            result = subprocess.run([str(d), '-C', str(cwd), *options], capture_output=True, text=True, encoding='utf-8', timeout=60)
            assert result.returncode == 0, (name, options, result.stdout, result.stderr)
            return result.stdout
        group = ['--workspace'] if workspace else []
        command('pkg', 'install', *group)
        selection = ['--package', package] if workspace else []
        for engine in ('auto', 'ast'):
            assert command('run', *selection, '--engine', engine).strip() == expected
        command('pkg', 'install', *group, '--locked')
        if name == 'tooling':
            command('fmt', '--check')
        lock = cwd / 'package-lock.don'
        parsed = json.loads(subprocess.check_output([str(d), 'don', 'to-json', str(lock)], text=True, encoding='utf-8'))
        assert parsed['version'] == 1
        assert not (cwd / 'dev.lock').exists()
        if name == 'tooling':
            command('build', '--release', '-o', 'out/app.exe')
            run = subprocess.run([str(cwd / 'out/app.exe')], capture_output=True, text=True, timeout=30)
            assert run.returncode == 0 and run.stdout.strip() == '42', run
        count += 1
        print(f'PASS {name}: install, both engines, DON lock and locked install')
for name in ['github-tag', 'archive-url']:
    subprocess.run([sys.executable, str(repo / 'examples' / name / 'verify.py'), str(d)], check=True, timeout=120)
    count += 1
subprocess.run([sys.executable, str(repo / 'scripts/smoke_dev_dependencies.py'), '--d', str(d)], check=True, timeout=120)
count += 1
subprocess.run([sys.executable, str(repo / 'scripts/smoke_package_bins.py'), '--d', str(d)], check=True, timeout=120)
count += 1
print(f'PASS: {count} package example groups')
subprocess.run([sys.executable, str(repo / 'scripts/smoke_global_bins.py'), '--d', str(d)], check=True, timeout=120)

subprocess.run([sys.executable, str(repo / 'scripts/smoke_peers.py'), '--d', str(d)], check=True, timeout=120)
