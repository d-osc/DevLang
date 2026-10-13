"""Source runtime regression: prototype cycles and pooled filesystem tasks."""
import argparse, os, subprocess, tempfile
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent

def main():
 p=argparse.ArgumentParser();p.add_argument('--bin-dir',default='target/debug');a=p.parse_args();binary=(ROOT/a.bin_dir/('devrun.exe' if os.name=='nt' else 'devrun')).resolve()
 cases=[('gc', (ROOT/'examples/prototype-gc/main.dev').read_text(encoding='utf-8'),'14\ntrue\n'),('fs', '''use "std/fs/promises"
fn main(){
 await(promises.mkdir("files",true))
 let first=promises.writeFile("files/a.txt","Hello")
 let second=promises.writeFile("files/b.txt","Dev")
 print(await(first));print(await(second));print(await(first));print(ready(first))
 await(promises.appendFile("files/a.txt","!"))
 let a=promises.readFile("files/a.txt","utf8")
 let b=promises.readFile("files/b.txt","utf8")
 print(await(a));print(await(b));print(await(a))
 await(promises.rename("files/b.txt","files/c.txt"))
 print(await(promises.readFile("files/c.txt","utf8")))
 await(promises.unlink("files/a.txt"));await(promises.unlink("files/c.txt"))
}
main()
''','true\ntrue\ntrue\ntrue\nHello!\nDev\nHello!\nDev\n')]
 cases.append(('gc_closure','use "std/json"\nfn factory():fn():i64 {\n let n=7\n let f=fn():i64{return n}\n Object.defineProperty(f.prototype,"constructor",{value:null})\n Object.defineProperty(f,"marker",{value:12})\n return f\n}\nfn garbage(){let a=new Object();a.self=a;let xs=new Array(a);a.xs=xs}\nfn main(){\n let f=factory()\n GC.collect()\n print(f.marker)\n print(f())\n for i in 0..1000 {garbage()}\n GC.collect()\n print(GC.tracked()<20 as usize)\n print(f.marker)\n}\nmain()\n','12\n7\ntrue\n12\n'))
 for engine in ('auto','ast'):
  for name,code,expected in cases:
   with tempfile.TemporaryDirectory() as tmp:
    source=Path(tmp)/'main.dev';source.write_text(code,encoding='utf-8')
    r=subprocess.run([str(binary),'--engine',engine,str(source)],cwd=tmp,capture_output=True,text=True,timeout=60)
    assert r.returncode==0,(engine,name,r.stderr)
    assert r.stdout==expected,(engine,name,r.stdout,expected)
  with tempfile.TemporaryDirectory() as tmp:
   source=Path(tmp)/'main.dev';source.write_text('use "std/fs/promises"\nfn main(){await(promises.readFile("missing.txt","utf8"))}\nmain()\n')
   r=subprocess.run([str(binary),'--engine',engine,str(source)],cwd=tmp,capture_output=True,text=True,timeout=30)
   assert r.returncode!=0 and 'fs.read_text' in r.stderr,(engine,r.stderr)
 print('PASS prototype cycles, concurrent filesystem tasks, repeated await, ready and errors (auto/ast)')
if __name__=='__main__':main()
