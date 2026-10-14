"""Build the DevLang static library, shared adapter and Node-API addon."""
import argparse
import json
import os
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
parser.add_argument('--cc', default='clang')
parser.add_argument('--ar', default='llvm-ar')
parser.add_argument('--node-sdk', help='Directory containing sdk.json from prepare_node_sdk.py')
parser.add_argument('--output', default='out/managed-interop')
args = parser.parse_args()
output = Path(args.output).resolve()
output.mkdir(parents=True, exist_ok=True)
sdk = json.loads((Path(args.node_sdk) / 'sdk.json').read_text(encoding='utf-8')) if args.node_sdk else None

def run(command):
    result = subprocess.run(list(map(str, command)), capture_output=True, text=True, encoding='utf-8', timeout=180)
    if result.returncode:
        raise SystemExit(f'{command}:\n{result.stdout}\n{result.stderr}')

library = output / 'libdevmath.a'
command = [Path(args.d).resolve(), 'build', HERE.parent / 'api.dev', '--lib', '--release',
           '--cc', args.cc, '--ar', args.ar, '-o', library]
if os.name != 'nt': command += ['--cflag', '-fPIC']
run(command)
shared = output / ('devbridge.dll' if os.name == 'nt' else 'libdevbridge.so')
command = [args.cc, '-shared', HERE / 'bridge.c', library, '-o', shared]
if os.name != 'nt': command += ['-fPIC']
run(command)
addon = None
if sdk:
    addon = output / 'devmath.node'
    command = [args.cc, '-shared', HERE / 'node/addon.c', library,
               '-I' + sdk['include'], '-DNAPI_VERSION=8', '-o', addon]
    if os.name == 'nt': command += [sdk['library']]
    else: command += ['-fPIC']
    run(command)
(output / 'build.json').write_text(json.dumps({'shared': str(shared), 'addon': str(addon) if addon else None}, indent=2) + '\n', encoding='utf-8')
print(f'Built native interop artifacts: {output}')
