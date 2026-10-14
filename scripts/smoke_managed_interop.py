"""Test Python/.NET/JS/TS calls into DevLang and JSON workers called from DevLang."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / 'examples/export-api/managed'
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
parser.add_argument('--cc', default='clang')
parser.add_argument('--ar', default='llvm-ar')
parser.add_argument('--node-sdk')
parser.add_argument('--languages', default='python,dotnet,node')
parser.add_argument('--report')
args = parser.parse_args()
languages = args.languages.split(',')
assert set(languages) <= {'python', 'dotnet', 'node'} and languages
if 'node' in languages and not args.node_sdk: parser.error('--node-sdk is required when testing node')
binary = Path(args.d).resolve()
expected = '42\n12.5\ntrue\nfalse\nDevLang Math\n'
reverse = '42\nHello โลก\n'
report = {'platform': os.name, 'consumers': {}, 'workers': {}}
environment = dict(os.environ, PYTHONIOENCODING='utf-8', PYTHONUTF8='1', DOTNET_CLI_TELEMETRY_OPTOUT='1', DOTNET_NOLOGO='1')

def run(command, cwd=None, input=None, success=True):
    result = subprocess.run(list(map(str, command)), cwd=cwd, env=environment, input=input,
                            capture_output=True, text=True, encoding='utf-8', timeout=180)
    if success and result.returncode:
        raise AssertionError(f'{command}:\n{result.stdout}\n{result.stderr}')
    return result

with tempfile.TemporaryDirectory(prefix='devlang managed ') as temporary:
    root = Path(temporary)
    output = root / 'native'
    command = [sys.executable, SOURCE / 'build.py', '--d', binary, '--cc', args.cc, '--ar', args.ar, '--output', output]
    if args.node_sdk: command += ['--node-sdk', Path(args.node_sdk).resolve()]
    run(command)
    native = json.loads((output / 'build.json').read_text(encoding='utf-8'))
    consumers, workers = {}, {}
    if 'python' in languages:
        consumers['python'] = [sys.executable, SOURCE / 'python/client.py', native['shared']]
        workers['python'] = [sys.executable, SOURCE / 'python/worker.py']
        report['python'] = sys.version.split()[0]
    if 'dotnet' in languages:
        project = root / 'dotnet-source'
        shutil.copytree(SOURCE / 'dotnet', project, ignore=shutil.ignore_patterns('obj', 'bin'))
        target = root / 'dotnet'
        run(['dotnet', 'build', project / 'ManagedInterop.csproj', '--output', target, '--nologo'])
        assembly = target / 'ManagedInterop.dll'
        consumers['dotnet'] = ['dotnet', assembly, native['shared']]
        workers['dotnet'] = ['dotnet', assembly, 'worker']
        report['dotnet'] = run(['dotnet', '--version']).stdout.strip()
    if 'node' in languages:
        tsc = Path(shutil.which('tsc') or 'missing-typescript').resolve()
        sibling = tsc.parent / 'node_modules/typescript/bin/tsc'
        if sibling.is_file(): tsc = sibling
        assert tsc.is_file(), 'Install TypeScript before running the test'
        target = root / 'typescript'
        run(['node', tsc, '--strict', '--noEmitOnError', '--module', 'commonjs', '--target', 'ES2020',
             '--moduleDetection', 'force', '--outDir', target,
             SOURCE / 'node/client.ts', SOURCE / 'node/worker.ts'])
        consumers['javascript'] = ['node', SOURCE / 'node/client.js', native['addon']]
        consumers['typescript'] = ['node', target / 'client.js', native['addon']]
        workers['javascript'] = ['node', SOURCE / 'node/worker.js']
        workers['typescript'] = ['node', target / 'worker.js']
        report['node'] = run(['node', '--version']).stdout.strip()
        report['typescript'] = run(['node', tsc, '--version']).stdout.strip()
    for name, command in consumers.items():
        result = run(command)
        assert result.stdout == expected, (name, result.stdout)
        report['consumers'][name] = result.stdout
        print(f'PASS {name} -> DevLang native library')
    for name, command in workers.items():
        for engine in ['auto', 'ast']:
            result = run([binary, SOURCE / 'bridge.dev', '--engine', engine, '--', *command])
            assert result.stdout == reverse, (name, engine, result.stdout)
        for invalid in ['{', '{"a":true,"b":2,"name":"x"}', '{"a":2000000,"b":2,"name":"x"}']:
            failure = run(command, input=invalid, success=False)
            assert failure.returncode != 0 and failure.stderr, (name, invalid, failure)
        report['workers'][name] = {'engines': ['auto', 'ast'], 'output': reverse}
        print(f'PASS DevLang auto/ast -> {name} JSON worker; invalid input rejected')
    for engine in ['auto', 'ast']:
        failure = run([binary, SOURCE / 'bridge.dev', '--engine', engine, '--', sys.executable,
                       '-c', 'import sys; sys.stderr.write("worker failure"); sys.exit(7)'], success=False)
        assert failure.returncode != 0 and 'worker failure' in failure.stderr, failure
    print('PASS worker failure propagates to DevLang in both engines')
if args.report:
    destination = Path(args.report)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
