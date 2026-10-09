"""Check explicit entry calls against both source runtime and native execution."""
import argparse
from pathlib import Path
import subprocess
import tempfile

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--runtime',required=True)
    parser.add_argument('--compiler',required=True)
    args=parser.parse_args()
    runtime=str(Path(args.runtime).resolve())
    compiler=str(Path(args.compiler).resolve())
    cases=[
        ('fn main() { print("never") }','',0),
        ('fn main() { print("once") }\nmain()','once\n',0),
        ('print("before")\nfn main() { print("inside") }\nmain()\nprint("after")','before\ninside\nafter\n',0),
        ('fn main() { print("twice") }\nmain(); main()','twice\ntwice\n',0),
        ('print(42)','42\n',0),
        ('let x = 40; x += 2; print(x)','42\n',0),
        ('fn hello() { print("hello") }\nhello()','hello\n',0),
        ('fn main(n i64) { print(n) }\nmain(42)','42\n',0),
        ('fn main() { return 7 }\nmain()','',0),
        ('fn main() { return 7 }\nreturn main()','',7),
    ]
    with tempfile.TemporaryDirectory(prefix='dev-entry-') as d:
        root=Path(d)
        source=root/'main.dev'
        for text,output,code in cases:
            source.write_text(text,encoding='utf-8')
            for command in ([runtime,str(source)],[compiler,'run',str(source),'-o',str(root/'program')]):
                result=subprocess.run(command,cwd=root,capture_output=True,text=True,timeout=30)
                assert (result.returncode,result.stdout)==(code,output),(command,text,result.stdout,result.stderr)
        imported=root/'library.dev'
        imported.write_text('fn main() { print("import main never") }\nfn hello() { print("import hello") }')
        source.write_text('use library\nfn main() { library.hello() }\nmain()')
        for command in ([runtime,str(source)],[compiler,'run',str(source),'-o',str(root/'program')]):
            result=subprocess.run(command,cwd=root,capture_output=True,text=True,timeout=30)
            assert result.returncode==0 and result.stdout=='import hello\n',result.stderr
        imported.write_text('fn hello() {}\nprint("side effect")')
        for command in ([runtime,str(source)],[compiler,'check',str(source)]):
            result=subprocess.run(command,cwd=root,capture_output=True,text=True,timeout=30)
            assert result.returncode!=0 and 'declarations only' in result.stderr,result.stderr
    print('PASS: 24 explicit-entry scenarios across runtime and native compiler')

if __name__=='__main__':
    main()
