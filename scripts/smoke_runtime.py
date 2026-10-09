"""Exercise the source interpreter with no compiler/toolchain in PATH."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runtime', required=True)
    binary = str(Path(parser.parse_args().runtime).resolve())
    count = 0
    with tempfile.TemporaryDirectory(prefix='devrun-') as d:
        root = Path(d)
        env = dict(os.environ, PATH='', CC='missing-compiler', DEV_CC='missing-compiler')
        def run(code, expected=None, error=None, stdin='', args=()):
            nonlocal count
            source = root / 'main.dev'
            source.write_text(code, encoding='utf-8')
            result = subprocess.run([binary, str(source), *args], cwd=root, env=env,
                                    input=stdin, capture_output=True, text=True, encoding='utf-8', timeout=10)
            if error:
                assert result.returncode != 0 and error in result.stderr, (code,result.stdout,result.stderr)
                assert 'main.dev:' in result.stderr, result.stderr
            else:
                assert result.returncode == 0 and result.stdout == expected, (code,result.stdout,result.stderr)
            count += 1
        run('fn main() { print(40 + 2) }', '42\n')
        run('fn main() { let x i64 = -9223372036854775808; print(x / -1); print(0xDEAD); print(0XFF) }','-9223372036854775808\n57005\n255\n')
        run('fn fact(n i64) i64 { if n <= 1 { return 1 }; return n * fact(n - 1) }\nfn main() { print(fact(10)) }', '3628800\n')
        run('fn test(n i64) bool { return n > 1 }\nfn main() { print(test(2)) }','true\n')
        run('fn main() { let a [i32; 3] = [1,2,3]; let i = 1; a[i] += 40; print(a[i]); let n = 0; while n < 5 { n += 1; if n == 2 { continue }; if n == 4 { break }; print(n) } }','42\n1\n3\n')
        run('fn main() { let n u8 = 255; n += 1; print(n); let x u64 = 18446744073709551615; print(x * 2); print(1.5 + 2.5) }','0\n18446744073709551614\n4\n')
        run('fn main() { print(false and (1 / 0 == 0)); print(true or (1 / 0 == 0)) }','false\ntrue\n')
        (root/'a.dev').write_text('use b\nfn answer() i64 { return b.answer() }\nfn base() i64 { return 42 }')
        (root/'b.dev').write_text('use a\nfn answer() i64 { return a.base() }')
        run('use a\nfn main() { print(a.answer()) }','42\n')
        run('use "std/strings" as s\nuse "std/io"\nfn main() { let text = s.concat("ภาษา", "ไทย"); print(s.len(text)); print(s.equal(text, "ภาษาไทย")); io.write_file("ข้อมูล.txt", text); io.writeln(io.read_file("ข้อมูล.txt")); io.writeln(io.read_line()) }','21\ntrue\nภาษาไทย\ninput\n', stdin='input\n')
        run('use "std/args"\nuse "std/time"\nfn main() { print(args.len()); print(args.get(0)); let before = time.now_ns(); time.sleep_ms(1); print(time.now_ns() >= before) }','1\nhello\ntrue\n',args=('--','hello'))
        for code, error in [
            ('print(42)', 'expected'),
            ('fn main() { print(1 / 0) }', 'division by zero'),
            ('fn main() { let a = [1]; print(a[2]) }', 'out of bounds'),
            ('fn main() { print(1 << 64) }','shift count'),
            ('fn main() { let x bool = 1 }','expected bool'),
            ('fn main() { let x i8 = 128 }','out of range'),
            ('fn main() { let x = [1, true] }','expected i64'),
            ('fn main() { let x = 1; x = true }','expected i64'),
            ('fn main() { if 1 { print(2) } }','condition must'),
            ('fn main() { print(nope) }','unknown variable'),
            ('extern fn puts(s str) i32\nfn main() { puts("x") }','no native FFI'),
            ('fn main() { let n = 1; print(&n) }','raw pointers'),
            ('fn loop() { loop() }\nfn main() { loop() }','recursion limit'),
            ('fn main() { let n = 256 as u8; print(n) }',None),
        ]:
            run(code, expected='0\n' if error is None else None, error=error)
        result = subprocess.run([binary,'-e','print(40 + 2)'],cwd=root,env=env,capture_output=True,text=True)
        assert result.returncode == 0 and result.stdout == '42\n', result.stderr
        count += 1
        assert all(p.suffix in ('.dev','.txt') for p in root.rglob('*')), list(root.rglob('*'))
    print(f'PASS: {count} interpreter scenarios; empty PATH, no compiler and no generated artifacts')

if __name__ == '__main__':
    main()
