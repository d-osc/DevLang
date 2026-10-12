"""Verify interpolation evaluation, escaping and diagnostics across backends."""
import argparse, os, subprocess, tempfile
from pathlib import Path

def main():
    parser=argparse.ArgumentParser(); parser.add_argument('--bin-dir',required=True)
    folder=Path(parser.parse_args().bin_dir).resolve(); suffix='.exe' if os.name=='nt' else ''
    cases=[
        ('let name: str="Dev"; print("Hello ${name}!")','Hello Dev!\n'),
        ('let n=41; print("${n+1} ${true} ${false} ${1.5}")','42 true false 1.5\n'),
        ('let name="世界"; print("ไทย ${name} 100%")','ไทย 世界 100%\n'),
        (r'let name="Dev"; print("literal \${name}; ${name}")','literal ${name}; Dev\n'),
        (r'print("${"nested"} ${"x ${1+2}"}")','nested x 3\n'),
        ('fn piece(n: i64): str { print(n); return "v${n}" }; print("${piece(1)}-${piece(2)}")','1\n2\nv1-v2\n'),
        ('fn piece(n: i64): str { return "v${n}" }; let a=piece(1); let b=piece(2); print("${a} ${b}")','v1 v2\n'),
        ('fn accept(s: str): bool { return true }; fn side(): str { print(99); return "x" }; let ok=false && accept("${side()}"); print(ok)','false\n'),
        ('fn check(s: str,n: i64): bool { print(s); return n<2 }; let n=0; while check("${n}",n) { n+=1 }','0\n1\n2\n'),
        ('fn identity<T>(x: T): T { return x }; print(identity("answer ${42}"))','answer 42\n'),
        ('fn make(name: str): fn(): str { return fn(): str { return "Hello ${name}" } }; let f=make("Dev"); print(f())','Hello Dev\n'),
        ('let n: u64=18446744073709551615; print("${n}")','18446744073709551615\n'),
        ('let n: i64=-9223372036854775808; print("${n}")','-9223372036854775808\n'),
        ('use "helper"; print(helper.greeting("Dev"))','Hello Dev\n'),
        ('async fn greeting(): str { return "Hello ${42}" }; print(await(greeting()))','Hello 42\n'),
    ]
    cases.extend([
        ('let a="Hello "; let b="Dev"; let c=a+b; c+="!"; print(c)','Hello Dev!\n'),
        ('fn text(n: i64): str { print(n); return "v${n}" }; print(text(1)+text(2))','1\n2\nv1v2\n'),
        ('struct S { text: str }; let s=S("a"); s.text+="b"; print(s.text)','ab\n'),
        ('fn join(a: str,b: str): str { return a+b }; let a=join("a","b"); let b=join("c","d"); print(a+b)','abcd\n'),
    ])
    failures=[('print("${}")','expected an expression'),('print("${missing}")','unknown variable'),('print("${1 2}")','expected end of interpolation'),('print("${name")','unterminated string interpolation'),('print("${[1,2]}")','interpolation supports')]
    with tempfile.TemporaryDirectory(prefix='dev-interpolation-') as directory:
        root=Path(directory);source=root/'main.dev'
        (root/'helper.dev').write_text('fn greeting(name: str): str { return "Hello ${name}" }',encoding='utf-8')
        for code,expected in cases+failures:
            source.write_text(code,encoding='utf-8')
            for mode in ['auto','ast','native']:
                command=[str(folder/('devc'+suffix)),'run',str(source),'--release','-o',str(root/('app'+suffix))] if mode=='native' else [str(folder/('devrun'+suffix)),str(source),'--engine',mode]
                result=subprocess.run(command,capture_output=True,text=True,encoding='utf-8',timeout=45)
                if (code,expected) in failures:
                    assert result.returncode!=0 and expected in result.stderr and 'main.dev:' in result.stderr,(mode,code,result.stdout,result.stderr)
                else:
                    assert result.returncode==0 and result.stdout==expected,(mode,code,result.stdout,result.stderr)
    print(f'PASS: {len(cases)+len(failures)} interpolation scenarios in auto, AST and release native')

if __name__=='__main__':main()
