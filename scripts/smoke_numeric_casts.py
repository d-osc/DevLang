"""Exercise float cast boundaries in both interpreters and optimized native code."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', required=True)
    folder = Path(parser.parse_args().bin_dir).resolve()
    ext = '.exe' if os.name == 'nt' else ''
    cases = 0
    with tempfile.TemporaryDirectory(prefix='dev-numeric-casts-') as directory:
        root = Path(directory)
        source = root / 'main.dev'

        def check(target, expression, expected=None, floating='f64'):
            nonlocal cases
            code = f'fn cast(n {floating}) {target} {{ return n as {target} }}\nlet zero {floating}=0.0\nprint(cast({expression}))\n'
            source.write_text(code, encoding='utf-8')
            for mode in ('auto', 'ast', 'native'):
                command = ([str(folder / ('devc' + ext)), 'run', str(source), '--release', '-o', str(root / ('program' + ext))]
                           if mode == 'native' else [str(folder / ('devrun' + ext)), str(source), '--engine', mode])
                result = subprocess.run(command, capture_output=True, text=True, timeout=45)
                if expected is None:
                    assert result.returncode != 0 and 'float-to-integer cast out of range' in result.stderr and 'main.dev:' in result.stderr, (mode, code, result.stdout, result.stderr)
                else:
                    assert result.returncode == 0 and result.stdout == expected + '\n', (mode, code, result.stdout, result.stderr)
            cases += 1

        for target in ('i8', 'u8', 'i16', 'u16', 'i32', 'u32', 'i64', 'u64', 'isize', 'usize'):
            for floating in ('f32', 'f64'):
                check(target, '42.9', '42', floating)
                for expression in ('zero/zero', '1.0/zero', '-1.0/zero'):
                    check(target, expression, floating=floating)
            check(target, '-0.9', '0')
        for target, expression, expected in (
            ('i8', '-128.9', '-128'), ('i8', '127.9', '127'),
            ('u8', '255.9', '255'), ('i16', '-32768.9', '-32768'),
            ('u16', '65535.9', '65535'), ('i32', '-2147483648.0', '-2147483648'),
            ('u32', '4294967295.9', '4294967295'),
            ('i64', '-9223372036854775808.0', '-9223372036854775808'),
            ('i64', '9223372036854774784.0', '9223372036854774784'),
            ('u64', '18446744073709549568.0', '18446744073709549568'),
        ):
            check(target, expression, expected)
        for target, expression in (
            ('i8', '128.0'), ('i8', '-129.0'), ('u8', '256.0'), ('u8', '-1.0'),
            ('i16', '32768.0'), ('i16', '-32769.0'), ('u16', '65536.0'),
            ('i32', '2147483648.0'), ('i32', '-2147483649.0'), ('u32', '4294967296.0'),
            ('i64', '9223372036854775808.0'), ('i64', '-9223372036854777856.0'),
            ('u64', '18446744073709551616.0'), ('u64', '-1.0'),
        ):
            check(target, expression)
    print(f'Numeric casts: {cases} cases passed in auto, AST and release native execution')


if __name__ == '__main__':
    main()
