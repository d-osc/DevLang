"""Build one DevLang static library and call its C ABI from real host languages."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
parser.add_argument('--cc', default='clang')
parser.add_argument('--cxx', default='clang++')
parser.add_argument('--ar', default='llvm-ar')
parser.add_argument('--languages', default='c,cpp,rust,go')
parser.add_argument('--report')
args = parser.parse_args()
binary = str(Path(args.d).resolve())
languages = args.languages.split(',')
assert languages and set(languages) <= {'c', 'cpp', 'rust', 'go'}
expected = '42\n12.5\ntrue\nDevLang Math\n'
report = {'platform': os.name, 'languages': {}, 'expected': expected}

def run(command, cwd, env=None):
    result = subprocess.run(list(map(str, command)), cwd=cwd, env=env,
                            capture_output=True, text=True, timeout=180)
    if result.returncode:
        raise AssertionError(f'{command}:\n{result.stdout}\n{result.stderr}')
    return result.stdout

with tempfile.TemporaryDirectory(prefix='devlang-export-') as temporary:
    root = Path(temporary)
    example = root / 'examples/export-api'
    shutil.copytree(ROOT / 'examples/export-api', example)
    output = root / 'out/export-api'
    output.mkdir(parents=True)
    library = output / 'libdevmath.a'
    run([binary, 'build', example / 'api.dev', '--lib', '--release',
         '--cc', args.cc, '--ar', args.ar, '-o', library], root)
    for language in languages:
        executable = output / (language + ('.exe' if os.name == 'nt' else ''))
        env = os.environ.copy()
        if language == 'c':
            compiler = args.cc
            command = [compiler, example / 'main.c', library, '-o', executable]
        elif language == 'cpp':
            compiler = args.cxx
            command = [compiler, example / 'main.cpp', library, '-o', executable]
        elif language == 'rust':
            compiler = 'rustc'
            command = [compiler, '--edition', '2021', example / 'main.rs',
                       '-L', f'native={output}', '-l', 'static=devmath', '-o', executable]
        else:
            compiler = 'go'
            env['CGO_ENABLED'] = '1'
            env['CC'] = args.cc
            command = [compiler, 'build', '-o', executable, example / 'go/main.go']
        version = run([compiler, 'version'] if language == 'go' else [compiler, '--version'], root).splitlines()[0]
        run(command, root, env)
        actual = run([executable], root)
        assert actual == expected, (language, actual, expected)
        report['languages'][language] = {'compiler': version, 'output': actual}
        print(f'PASS {language}: {version}')
if args.report:
    destination = Path(args.report)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
