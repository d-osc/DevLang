"""Call C++, Rust and Go shared libraries from DevLang runtime and native code."""
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
parser.add_argument('--languages', default='cpp,rust,go')
parser.add_argument('--report')
args = parser.parse_args()
binary = Path(args.d).resolve()
languages = args.languages.split(',')
assert languages and set(languages) <= {'cpp', 'rust', 'go'}
expected = '42\n12.5\ntrue\nfalse\n-42\n'
report = {'platform': os.name, 'languages': {}}

def run(command, cwd, env=None):
    result = subprocess.run(list(map(str, command)), cwd=cwd, env=env,
                            capture_output=True, text=True, timeout=180)
    if result.returncode:
        raise AssertionError(f'{command}:\n{result.stdout}\n{result.stderr}')
    return result.stdout

with tempfile.TemporaryDirectory(prefix='devlang-import-') as temporary:
    root = Path(temporary)
    source = root / 'sources'
    shutil.copytree(ROOT / 'examples/export-api/imports', source)
    for language in languages:
        folder = root / language
        folder.mkdir()
        library = folder / ('foreign.dll' if os.name == 'nt' else 'libforeign.so')
        env = os.environ.copy()
        if language == 'cpp':
            compiler = args.cxx
            command = [compiler, '-shared', source / 'cpp/library.cpp', '-o', library]
            if os.name == 'nt':
                link = folder / 'foreign.lib'
                command += [f'-Wl,/implib:{link}']
            else:
                link = library
                command += ['-fPIC']
        elif language == 'rust':
            compiler = 'rustc'
            command = [compiler, '--edition', '2021', '--crate-type', 'cdylib',
                       source / 'rust/library.rs', '-o', library]
            link = library.with_suffix('.dll.lib') if os.name == 'nt' else library
        else:
            compiler = 'go'
            env['CGO_ENABLED'] = '1'
            env['CC'] = args.cc
            command = [compiler, 'build', '-buildmode=c-shared', '-o', library,
                       source / 'go/library.go']
            if os.name == 'nt':
                raise AssertionError('Windows Go requires a separately configured cgo/MinGW toolchain; not covered by this MSVC fixture')
            link = library
        version = run([compiler, 'version'] if language == 'go' else [compiler, '--version'], root).splitlines()[0]
        run(command, root, env)
        # Runtime uses a prebuilt shared library even with all C backends unavailable.
        runtime_env = dict(os.environ, PATH='', CC='missing-compiler', DEV_CC='missing-compiler')
        for engine in ['auto', 'ast']:
            output = run([binary, source / 'main.dev', '--ffi-lib', library,
                          '--engine', engine], root, runtime_env)
            assert output == expected, (language, engine, output)
        executable = folder / ('native.exe' if os.name == 'nt' else 'native')
        run([binary, 'build', source / 'main.dev', '--release', '--cc', args.cc,
             '--link', link, '-o', executable], root)
        output = run([executable], folder)
        assert output == expected, (language, 'native', output)
        report['languages'][language] = {'compiler': version, 'engines': ['auto', 'ast', 'native'], 'output': output}
        print(f'PASS {language} -> DevLang auto/ast/native: {version}')
if args.report:
    destination = Path(args.report)
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
