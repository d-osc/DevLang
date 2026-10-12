"""Canonical colon annotations in formatter, both source engines and native code."""
import argparse, os, subprocess, tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', required=True)
    folder = Path(parser.parse_args().bin_dir).resolve()
    suffix = '.exe' if os.name == 'nt' else ''
    cases = [
        ('let a: i32 = 1; fn t(a: i32): i32 { return a }; print(t(a))', '1\n'),
        ('fn apply(f: fn(i32): i32, a: i32): i32 { return f(a) }; print(apply(fn(n: i32): i32 { return n+1 },41))', '42\n'),
        ('struct Point { x: i32; y: i32 }; fn Point.sum(self: Point): i32 { return self.x+self.y }; print(Point(19,23).sum())', '42\n'),
        ('fn identity<T>(n: T): T { return n }; print(identity<i32>(42))', '42\n'),
        ('fn add(n: i64): i64 { return n+1 }; unsafe { let cb: callback(i64): i64 = callback(add); print(cb(41)) }', '42\n'),
        ('use "math"; print(math.add(19,23))', '42\n'),
    ]
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory); source = root/'main.dev'
        (root/'math.dev').write_text('fn add(a: i32, b: i32): i32 { return a+b }', encoding='utf-8')
        for code, expected in cases:
            source.write_text(code, encoding='utf-8')
            for mode in ['auto','ast','native']:
                command = [str(folder/('devc'+suffix)), 'run',str(source),'--release','-o',str(root/('app'+suffix))] if mode=='native' else [str(folder/('devrun'+suffix)), str(source),'--engine',mode]
                result = subprocess.run(command,capture_output=True,text=True,encoding='utf-8',timeout=45)
                assert result.returncode==0 and result.stdout==expected,(mode,code,result.stdout,result.stderr)
        for code in ['let a: = 1','fn t(a: i32): {}','fn t(a:): i32 { return a }']:
            source.write_text(code,encoding='utf-8')
            for executable in ['devc','devrun']:
                command=[str(folder/(executable+suffix)), 'check',str(source)] if executable=='devc' else [str(folder/(executable+suffix)),str(source)]
                result=subprocess.run(command,capture_output=True,text=True,encoding='utf-8',timeout=45)
                assert result.returncode!=0 and 'expected a type' in result.stderr,(code,result.stderr)
    print('PASS: 6 colon-annotation scenarios in auto, AST and release native; 3 malformed annotations rejected')

if __name__=='__main__': main()
