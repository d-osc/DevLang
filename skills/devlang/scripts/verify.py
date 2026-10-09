"""Check DevLang source and compare the two source execution engines."""
import argparse
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('--d', default='d')
    parser.add_argument('--timeout', type=float, default=30)
    parser.add_argument('--skip-native-check', action='store_true')
    args = parser.parse_args()
    binary = shutil.which(args.d)
    if not binary:
        parser.error('d was not found; pass --d with the installed executable path')
    source = args.source.resolve()
    if not source.is_file():
        parser.error('Source file does not exist')
    if not args.skip_native_check:
        subprocess.run([binary, 'check', str(source)], check=True, timeout=args.timeout)
    results = [subprocess.run([binary, str(source), '--engine', engine], capture_output=True, timeout=args.timeout) for engine in ('auto', 'ast')]
    if any(result.returncode for result in results):
        for engine, result in zip(('auto', 'ast'), results):
            print(engine, result.returncode, result.stderr.decode(errors='replace'))
        raise SystemExit('Source execution failed')
    if results[0].stdout != results[1].stdout:
        raise SystemExit('Source engine stdout differs; inspect nondeterminism or an engine error')
    print(results[0].stdout.decode(errors='replace'), end='')
    print('Passed: frontend and source engines' if not args.skip_native_check else 'Passed: source engines')


if __name__ == '__main__':
    main()
