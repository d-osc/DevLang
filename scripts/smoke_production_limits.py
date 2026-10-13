"""Resource-limit and repeated-allocation checks; not production certification."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', default='target/debug')
    parser.add_argument('--iterations', type=int, default=5000)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    if not 1 <= args.iterations <= 1000000:
        parser.error('--iterations must be in 1..1000000')
    folder = (ROOT / args.bin_dir).resolve()
    suffix = '.exe' if os.name == 'nt' else ''
    results = []
    def run(name, command, expected=None):
        start = time.monotonic()
        result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8', timeout=120)
        if expected is None:
            assert result.returncode == 1 and ('limit exceeded' in result.stderr or 'source exceeds 8 MiB' in result.stderr), (name, result)
        else:
            assert result.returncode == 0 and result.stdout.strip() == expected, (name, result)
        results.append({'name': name, 'seconds': round(time.monotonic()-start, 3), 'exit': result.returncode})
    with tempfile.TemporaryDirectory() as directory:
        source = Path(directory) / 'main.dev'
        hostile = {
            'parentheses': 'let x = ' + '(' * 5000 + '1' + ')' * 5000,
            'unary': 'let x = ' + '!' * 5000 + 'true',
            'binary': 'let x = ' + '1 + ' * 5000 + '1',
            'postfix': 'let x = f()' + '.x' * 5000,
            'types': 'fn f(x: ' + 'Ref<' * 5000 + 'i32' + '>' * 5000 + ') {}',
            'blocks': 'fn f(){' + 'if true {' * 5000 + 'print(1)' + '}' * 5001,
            'bytes': ' ' * (8 * 1024 * 1024 + 1),
        }
        for name, code in hostile.items():
            source.write_text(code, encoding='utf-8')
            for tool, prefix in [('devrun', []), ('devc', ['check']), ('d', ['fmt', '--stdout'])]:
                run(f'{tool}:{name}', [str(folder / (tool+suffix)), *prefix, str(source)])
        source.write_text('''fn garbage(){let a=new Object();a.self=a;let xs=new Array(a);a.xs=xs}
fn main(){
    let survivor=new Object({value:42})
    for i in 0..ITERATIONS {garbage()}
    GC.collect()
    print(survivor.value)
    print(GC.tracked()<20 as usize)
}
main()
'''.replace('ITERATIONS', str(args.iterations)), encoding='utf-8')
        for engine in ['auto', 'ast']:
            run(f'cycles:{engine}', [str(folder / ('devrun'+suffix)), str(source), '--engine', engine], '42\ntrue')
    report = {'scope': 'parser diagnostics and prototype allocation cycles; excludes native leak certification',
              'iterations': args.iterations, 'checks': results}
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')
    print(f'PASS {len(results)} production resource checks; {args.iterations} cycles per engine')

if __name__ == '__main__':
    main()
