from source_text import unsafe_fixture
"""Validate recursive raw layouts and native immutable-reference cleanup."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', required=True)
    parser.add_argument('--sanitize', action='store_true')
    args = parser.parse_args()
    compiler = str(Path(args.compiler).resolve())
    cases = [
        ('struct Node { value i64; next *Node }; let first = Node(1,null); let second = Node(2,&first); (*second.next).value = 42; print(first.value)', '42\n'),
        ('struct A { value i64; next *B }; struct B { next *A }; let a = A(42,null); let b = B(&a); a.next = &b; print((*(*a.next).next).value)', '42\n'),
        ('enum E { A,B }; struct S { p *E }; let e = E.B; let s = S(&e); print(*s.p)', 'B\n'),
        ('enum List<T> { Nil,Cons(T,Ref<List<T>>) }; let list = List<i64>.Cons(42,ref(List<i64>.Nil())); match list { Nil => {}; Cons(n,next) => { print(n); print(deref(next)) } }', '42\nNil\n'),
        ('struct P { x i64 }; fn make() Ref<P> { return ref(P(42)) }; for i in 0..1000 { let r = make(); let p = deref(r); p.x = 99 }; print(deref(make()).x)', '42\n'),
        ('let a=ref(42); let b=a; a=ref(99); print(deref(b)); print(deref(a))','42\n99\n'),
        ('fn f(v Ref<i64>) Ref<i64> { return v }; let r=f(ref(42)); print(deref(r))','42\n'),
        ('struct P { r Ref<i64> }; let a=P(ref(42)); let b=a; a.r=ref(99); print(deref(b.r)); print(deref(a.r))','42\n99\n'),
        ('struct P { r Ref<i64> }; let a=[P(ref(19)),P(ref(23))]; print(deref(a[0].r)+deref(a[1].r))','42\n'),
        ('let a=[ref(19),ref(23)]; print(deref(a[0])+deref(a[1]))','42\n'),
        ('enum E { R(Ref<i64>) }; fn make() E { return E.R(ref(42)) }; match make() { R(r) => { print(deref(r)); for i in 0..3 { break }; print(deref(r)) } }','42\n42\n'),
        ('fn make(n i64) Ref<i64> { return ref(n) }; let i=0; while deref(make(i))<3 { print(i); i+=1 }','0\n1\n2\n'),
        ('fn make() Ref<bool> { print(99); return ref(true) }; print(false && deref(make())); print(true || deref(make()))','false\ntrue\n'),
        ('enum E { R(Ref<i64>) }; fn yes(v Ref<i64>) bool { return deref(v)==42 }; match E.R(ref(42)) { R(r) if yes(ref(deref(r))) => { print(deref(r)) }; _ => {} }','42\n'),
        ('fn f() Ref<i64> { for i in 0..3 { let r=ref(i); if i==2 { return r }; continue }; return ref(-1) }; print(deref(f()))','2\n'),
        ('struct Node { children Vec<Node> }; for i in 0..1000 { let n=Node(Vec<Node>()); n.children.push(n); let snapshot=n.children.slice(0,1); n.children.clear(); print(snapshot[0].children.len()) }','0\n'*1000),
        ('struct Node { children Map<i64,Node> }; for i in 0..1000 { let n=Node(Map<i64,Node>()); n.children.set(1,n); let copy=n; n.children.set(2,copy); n.children.remove(1) }; print(42)','42\n'),
        ('fn make() Vec<Ref<i64>> { return Vec<Ref<i64>>(ref(42)) }; let a=make(); let b=a.slice(0,1); a[0]=ref(99); a.clear(); print(deref(b[0])); let c=make(); print(deref(c.pop()))','42\n42\n'),
        ('let a=Map<str,Ref<i64>>(); a.set("x",ref(42)); let b=a; a.set("x",ref(99)); let r=b.get("x"); a.clear(); b.clear(); print(deref(r))','42\n'),
        ('fn make(n i64) fn(i64) i64 { let r=ref(n); return fn(x i64) i64 { return deref(r)+x } }; for i in 0..1000 { let a=make(19); let b=a; print(b(23)) }','42\n'*1000),
        ('let a=Vec<fn(i64) i64>(); for i in 0..100 { a.push(fn(x i64) i64 { return i+x }) }; let s=a.slice(0,100); a.clear(); print(s[99](1))','100\n'),
        ('async fn work(n i64) Ref<i64> { return ref(n) }; let tasks=Vec<Task<Ref<i64>>>(); for i in 0..20 { tasks.push(work(i)) }; let total=0; for i in 0..20 { total+=deref(await(tasks[i])) }; print(total)','190\n'),
        ('struct B { data [Ref<i64>;2] }; fn make() B { let a=[ref(19),ref(23)]; return B(a) }; let b=make(); print(deref(b.data[0])+deref(b.data[1]))','42\n'),
        ('let m=Map<i64,Ref<i64>>(); let seed=1; for i in 0..4096 { seed=(seed*25173+13849)%65536; let key=seed%512; if seed%3==0 { m.remove(key) } else { m.set(key,ref(i)) } }; let sum=0; for i in 0..512 { if m.contains(i) { sum+=deref(m.get(i)) } }; print(sum); print(m.len())','1262831\n329\n'),
        ('unsafe { let callbacks=Vec<callback(i64) i64>(); for i in 0..64 { let r=ref(i); callbacks.push(callback(fn(n i64) i64 { return deref(r)+n })) }; let total=0; for i in 0..64 { total+=callbacks[i](1) }; print(total) }','2080\n'),
        ('unsafe { for i in 0..2000 { let r=ref(i); let cb=callback_context(fn(x i64) i64 { return deref(r)+x }); let copy=cb; if copy.call(0,copy.data)!=i { print(-1) } }; print(42) }','42\n'),
        ('async fn work(n i64) Ref<i64> { return ref(n) }; let jobs=Vec<Task<Ref<i64>>>(); for i in 0..30 { jobs.push(then(work(i),fn(r Ref<i64>) Ref<i64> { return ref(deref(r)+1) })) }; let sum=0; for i in 0..30 { sum+=deref(await(jobs[i])) }; print(sum)','465\n'),
    ]
    with tempfile.TemporaryDirectory(prefix='dev-native-reference-') as directory:
        root = Path(directory)
        source = root/'main.dev'
        for code, expected in cases:
            source.write_text(unsafe_fixture(code),encoding='utf-8')
            output = root/('program.exe' if os.name=='nt' else 'program')
            if args.sanitize:
                generated = root/'generated'
                subprocess.run([compiler,'emit',str(source),'-o',str(generated)],check=True,capture_output=True)
                subprocess.run(['cc','-std=c11','-g','-fsanitize=address,undefined','-fno-omit-frame-pointer',*[str(p) for p in generated.glob('*.c')],'-o',str(output)],check=True,capture_output=True)
                result = subprocess.run([str(output)],capture_output=True,text=True,timeout=45)
            else:
                result = subprocess.run([compiler,'run',str(source),'-o',str(output)],capture_output=True,text=True,timeout=45)
            assert result.returncode==0 and result.stdout==expected,(code,result.stdout,result.stderr)
    print(f'PASS: {len(cases)} recursive pointer/reference native scenarios' + (' (ASan/UBSan)' if args.sanitize else ''))


if __name__=='__main__':
    main()
