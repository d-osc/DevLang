"""Differential numeric-runtime checks, including errors and AST fallback."""
import argparse
import os
from pathlib import Path
import random
import subprocess
import tempfile

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runtime', required=True)
    args = parser.parse_args()
    binary = str(Path(args.runtime).resolve())
    env = dict(os.environ, PATH='', DEV_CC='missing-compiler', CC='missing-compiler')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-numeric-') as d:
        root = Path(d)
        source = root / 'main.dev'
        def check(code, expected=None, error=None, exit_code=0):
            nonlocal count
            source.write_text(code, encoding='utf-8')
            results = []
            for mode in ('ast','auto'):
                result = subprocess.run([binary,str(source),'--engine',mode],cwd=root,env=env,
                                        capture_output=True,text=True,encoding='utf-8',timeout=15)
                results.append((result.returncode,result.stdout,result.stderr))
            assert results[0] == results[1], (code,results)
            result = results[1]
            if error:
                assert result[0] != 0 and error in result[2] and 'main.dev:' in result[2], (code,result)
            else:
                assert result[0] == exit_code and result[1] == expected, (code,result)
            count += 1
        check('let i = 0; let n = 0; while i < 100000 { n += i & 255; i += 1 }; print(n)', '12742320\n')
        check('fn fact(n i64) i64 { if n <= 1 { return 1 }; return n * fact(n-1) }; print(fact(15))','1307674368000\n')
        check('fn fib(n i64) i64 { if n < 2 { return n }; return fib(n-1)+fib(n-2) }; print(fib(15))','610\n')
        check('let n = 0; let total = 0; while n < 8 { n += 1; if n == 2 { continue }; let j = 0; while j < 5 { j += 1; if j == 3 { break }; total += n }; if n == 4 { break } }; print(total)', '16\n')
        check('let x = 1; if true { let x = 9; x += 1; print(x) }; print(x)', '10\n1\n')
        check('let i = 0; let n = 0; while i < 4 { let x = i; n += x; i += 1 }; print(n)','6\n')
        check('let a [i32; 3] = [1,2,3]; let b = a; b[0] = 99; print(a[0]); print(b[0])','1\n99\n')
        check('fn change(a [i32; 3]) i32 { a[1] += 7; return a[1] }; let a [i32; 3] = [1,2,3]; print(change(a)); print(a[1]); print(change(a))','9\n2\n9\n')
        check('fn make(n i64) [i64; 2] { return [n,n+1] }; let a = make(40); print(a[1]); let b = a; b[1] += 1; print(a[1]); print(b[1])','41\n41\n42\n')
        check('let a = [true,false]; a[1] = !a[0]; print(a[1]); let b [f32; 2] = [1.5,2.5]; b[0] += 0.25; print(b[0])','false\n1.75\n')
        check('fn touch() bool { print(99); return true }; print(false and touch()); print(true or touch())','false\ntrue\n')
        check('fn touch() i64 { print(7); return 1 }; let a = [2,3]; a[touch()] += 4; print(a[1])','7\n7\n7\n')
        check('fn f(n i64) i64 { return n+1 }; let i = 0; while i < 200 { print(f(i) == i+1); i += 1 }','true\n'*200)
        check('fn unused() i64 { return missing }; print(42)','42\n')
        check('if false { print(1 / 0) }; print(42)','42\n')
        check('if false { let x bool = 1 }; print(42)','42\n')
        check('fn main() { let i = 0; while i < 3 { i += 1 }; print(i) }; main(); main()','3\n3\n')
        check('fn main() i32 { return 7 }; main()','')
        check('fn main() i32 { return 7 }; return main()','',exit_code=7)
        check('let n i64 = -9223372036854775808; print(n / -1); let m u64 = 18446744073709551615; print(m*2)','-9223372036854775808\n18446744073709551614\n')
        check('let x = 0.0 / 0.0; print(x == x); print(x != x); print(1.0 / 0.0)','false\ntrue\ninf\n')
        check('fn greet() { print("ไทย") }; fn sum(n i64) i64 { let i = 0; let x = 0; while i < n { x += i; i += 1 }; return x }; greet(); print(sum(1000))','ไทย\n499500\n')
        check('let a = [[1,2],[3,4]]; print(a[1][0])','3\n')
        check('let s = "abc"; print(s[1])','98\n')
        for code, error in [
            ('let n = 0; print(10/n)','division by zero'),
            ('let n = 64; print(1<<n)','shift count'),
            ('let n = -1; print(1>>n)','shift count'),
            ('let a = [1,2]; let i = 2; print(a[i])','out of bounds'),
            ('let a = [1,2]; let i = -1; print(a[i])','negative or excessive'),
            ('let a = [1,2]; let i = 2; a[i] = 3','out of bounds'),
            ('fn missing() i64 { let i = 0 }; print(missing())','missing return'),
            ('fn bad() i64 { return true }; print(bad())','expected i64'),
            ('let n = 1; let n = 2','duplicate local'),
            ('let a = [1,2]; a[0] = true','expected i64'),
            ('fn f() { f() }; f()','recursion limit'),
            ('while false { let x = 1 }; print(x)','unknown variable'),
            ('break','break/continue outside loop'),
            ('continue','break/continue outside loop'),
        ]:
            check(code,error=error)
        # Integer widths, unsigned overflow, casts and rounding-sensitive f32 conversion.
        for ty, bits, signed in [(f'{p}{b}',b,p=='i') for p in ('i','u') for b in (8,16,32,64)]:
            maximum=(1 << (bits-int(signed)))-1
            wrapped=-(1 << (bits-1)) if signed else 0
            check(f'let n {ty} = {maximum}; n += 1; print(n); print((n as u64) as {ty}); print(~n)',f'{wrapped}\n{wrapped}\n{-wrapped-1 if signed else maximum}\n')
        for n in [0,16777217,9223372586610589697,18446744073709551615]:
            # Compare exact outputs rather than computing host rounding independently.
            source.write_text(f'let n u64 = {n}; print(n as f32); print(n as f64); print((n as f32) as i64)',encoding='utf-8')
            result=subprocess.run([binary,str(source),'--engine','ast'],cwd=root,env=env,capture_output=True,text=True)
            check(source.read_text(encoding='utf-8'), result.stdout)
        rng=random.Random(417)
        for _ in range(50):
            n=rng.randrange(1,90); multiplier=rng.randrange(1,25); mask=rng.randrange(1,256)
            check(f'fn calc(n i64) i64 {{ let i = 0; let total = 0; while i < n {{ if i % 3 == 0 {{ total += (i*{multiplier}) & {mask} }} else {{ total -= i }}; i += 1 }}; return total }}; print(calc({n}))',
                  str(sum(((i*multiplier)&mask) if i%3==0 else -i for i in range(n)))+'\n')
        (root/'math.dev').write_text('fn calc(n i64) i64 { return n*2 }',encoding='utf-8')
        check('use math; print(math.calc(21))','42\n')
        assert all(p.suffix == '.dev' for p in root.rglob('*')), list(root.rglob('*'))
    print(f'PASS: {count} numeric scenarios in auto and ast modes; identical outputs, exit codes and source-located errors; no compiler or artifacts')

if __name__ == '__main__':
    main()
