"""Exercise ranges, nominal types and explicit generics in both execution paths."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--bin-dir',required=True)
    folder=Path(parser.parse_args().bin_dir).resolve()
    suffix='.exe' if os.name=='nt' else ''
    count=0
    with tempfile.TemporaryDirectory(prefix='dev-features-') as d:
        root=Path(d); source=root/'main.dev'
        env=dict(os.environ,PATH='',DEV_CC='missing-compiler')
        def check(code, expected=None, error=None):
            nonlocal count
            source.write_text(code,encoding='utf-8')
            for mode in ('auto','ast','native'):
                if mode=='native':
                    command=[str(folder/('devc'+suffix)),'run',str(source),'-o',str(root/('program'+suffix))]
                    child_env=None
                else:
                    command=[str(folder/('devrun'+suffix)),str(source),'--engine',mode];child_env=env
                result=subprocess.run(command,cwd=root,env=child_env,capture_output=True,text=True,encoding='utf-8',timeout=45)
                if error:
                    assert result.returncode!=0 and error in result.stderr,(mode,code,result.stdout,result.stderr)
                    assert 'main.dev:' in result.stderr,(mode,result.stderr)
                else:
                    assert result.returncode==0 and result.stdout==expected,(mode,code,result.stdout,result.stderr)
            count+=1
        check('let sum = 0; for i in 0..10 { sum += i }; print(sum)','45\n')
        check('let sum = 0; for i in -3..3 { sum += i }; print(sum); for i in 5..2 { print(99) }','-3\n')
        check('let n = 0; for i in 0..10 { if i == 2 { continue }; if i == 5 { break }; n += i }; print(n)','8\n')
        check('let n = 0; for i in 0..4 { for j in 0..4 { if j == 1 { continue }; if j == 3 { break }; n += i+j } }; print(n)','20\n')
        check('let n = 0; for i in 0..3 { let j = 0; while j < 3 { j += 1; if j == 2 { continue }; n += i } }; print(n)','6\n')
        check('let i = 4; let n = 0; for i in 0..i { i = 99; n += 1 }; print(i); print(n)','4\n4\n')
        check('let n = 0; for i in 0..3 { if true { let i = 9; n += i; continue } }; print(n)','27\n')
        check('fn bound() i64 { print(7); return 3 }; let n = 0; for i in bound()..bound()+3 { n += i }; print(n)','7\n7\n12\n')
        check('fn find() i64 { for i in 0..5 { if i == 3 { return i } }; return -1 }; print(find())','3\n')
        check('struct Point { x i32; y i32 }; fn shift(p Point) Point { p.x += 1; return p }; let p = Point(19,23); let q = shift(p); print(p.x); print(q.x); print(q.y)','19\n20\n23\n')
        check('struct Person { name str; age i64 }; let p = Person("Dev",3); print(p.name); p.age += 1; print(p.age)','Dev\n4\n')
        check('struct Point { x i64; y i64 }; struct Outer { p Point; flag bool }; let a = Outer(Point(1,2),true); let b = a; b.p.x = 42; print(a.p.x); print(b.p.x); print(b.flag)','1\n42\ntrue\n')
        check('struct Point { x i64 }; let a = [Point(1),Point(2)]; a[1].x += 40; print(a[0].x); print(a[1].x)','1\n42\n')
        check('struct Buffer { values [i32; 3] }; let a = Buffer([1,2,3]); let b = a; b.values[1] += 40; print(a.values[1]); print(b.values[1])','2\n42\n')
        check('struct Empty {}; fn make() Empty { return Empty() }; let a = make(); print(42)','42\n')
        check('enum Color { Red, Green, Blue }; let color = Color.Green; print(color); print(color == Color.Green); print(color != Color.Red); print(color as i32)','Green\ntrue\ntrue\n1\n')
        check('enum Mode { A,B }; fn pick(m Mode) Mode { if m == Mode.A { return Mode.B }; return Mode.A }; let a = [Mode.A,Mode.B]; print(pick(a[0])); a[1] = Mode.A; print(a[1])','B\nA\n')
        check('fn identity<T>(value T) T { return value }; print(identity<i64>(42)); print(identity<f64>(1.5)); print(identity<bool>(true)); print(identity<str>("Dev"))','42\n1.5\ntrue\nDev\n')
        check('fn main<T>(value T) { print(value) }; main<i64>(42)','42\n')
        check('fn add<T>(a T,b T) T { return a+b }; print(add<i32>(19,23)); print(add<f64>(1.5,2.5)); let n u8 = 255; print(add<u8>(n,1))','42\n4\n0\n')
        check('fn second<A,B>(a A,b B) B { return b }; print(second<i64,bool>(1,true))','true\n')
        check('fn fact<T>(n T) T { if n <= 1 { return 1 }; return n * fact<T>(n-1) }; print(fact<i64>(10))','3628800\n')
        check('struct Box<T> { value T }; fn wrap<T>(v T) Box<T> { return Box<T>(v) }; let a = wrap<i64>(42); print(a.value); let b = Box<Box<i64>>(a); print(b.value.value)','42\n42\n')
        check('struct Point { x i64 }; fn identity<T>(v T) T { return v }; let p = identity<Point>(Point(42)); print(p.x)','42\n')
        check('fn sum<T>(n i64) T { let total T = 0; for i in 0..n { total += i as T }; return total }; print(sum<i64>(100)); print(sum<f64>(100))','4950\n4950\n')
        (root/'types.dev').write_text('struct Point { x i64 }; struct Box<T> { value T }; enum Mode { A,B }; fn wrap<T>(value T) Box<T> { return Box<T>(value) }',encoding='utf-8')
        check('use types; let p types.Point = types.Point(42); let b = types.wrap<types.Point>(p); print(b.value.x); print(types.Mode.B)','42\nB\n')
        (root/'a.dev').write_text('use b; struct A { x i64 }; fn identity<T>(v T) T { return b.identity<T>(v) }',encoding='utf-8')
        (root/'b.dev').write_text('use a; fn identity<T>(v T) T { return v }',encoding='utf-8')
        check('use a; print(a.identity<i64>(42))','42\n')
        check('struct Node { next *Node }; let n = Node(null); print(n.next == null)','true\n')
        check('struct A { b *B }; struct B { a *A }; let a = A(null); print(a.b == null)','true\n')
        check('enum Result<T> { Ok(T), Err(str) }; fn read(r Result<i64>) i64 { match r { Ok(n) => { return n }; Err(_) => { return 0 } } }; print(read(Result.Ok<i64>(42))); print(read(Result.Err<i64>("bad")))','42\n0\n')
        check('enum E { Empty, Pair(i64,bool) }; let e = E.Pair(42,true); match e { Empty => { print(0) }; Pair(n,b) => { print(n); print(b) } }','42\ntrue\n')
        check('enum E { A,B }; match E.B { A => { print(1) }; B => { print(2) } }','2\n')
        check('enum E { V(i64) }; for i in 0..5 { match E.V(i) { V(n) => { if n == 2 { continue }; if n == 4 { break }; print(n) } } }','0\n1\n3\n')
        check('enum E { V(i64) }; fn make() E { print(1); return E.V(42) }; match make() { V(n) => { print(n) } }','1\n42\n')
        check('struct Point { x i64 }; enum E { P(Point) }; match E.P(Point(42)) { P(p) => { p.x += 1; print(p.x) } }','43\n')
        check('fn make() Ref<i64> { let x = 42; return ref(x) }; let r = make(); print(deref(r))','42\n')
        check('struct P { x i64 }; let p = P(42); let r Ref<P> = ref(p); p.x = 99; print(deref(r).x)','42\n')
        check('enum List<T> { Nil, Cons(T,Ref<List<T>>) }; let tail = ref(List.Nil<i64>()); let list = List.Cons<i64>(42,tail); match list { Nil => { print(0) }; Cons(n,next) => { print(n); match deref(next) { Nil => { print(1) }; Cons(_,_) => { print(2) } } } }','42\n1\n')
        check('struct Node { next Ref<Link> }; enum Link { End, More(Node) }; let n = Node(ref(Link.End)); match deref(n.next) { End => { print(42) }; More(_) => { print(0) } }','42\n')
        (root/'payload.dev').write_text('enum Result<T> { Ok(T),Err(str) }; fn wrap<T>(v T) Result<T> { return Result.Ok<T>(v) }',encoding='utf-8')
        check('use payload; let r = payload.wrap<i64>(42); match r { Ok(n) => { print(n) }; Err(_) => { print(0) } }','42\n')
        check('enum Result<T> { Ok(T),Err(str) }; match Result<i64>.Ok(42) { Ok(n) => { print(n) }; Err(_) => {} }','42\n')
        check('enum E { A(i64),B }; print(E.A(42)); print(E.B)','A\nB\n')
        check('struct P { x i64 }; let r = ref(P(42)); let p = deref(r); p.x = 99; print(deref(r).x)','42\n')
        check('fn identity<T>(v T) T { return v }; print(deref(identity<Ref<i64>>(ref(42))))','42\n')
        check('enum E { A(i64) }; let original = E.A(42); let copy = original; match copy { A(n) => { n = 99 } }; match original { A(n) => { print(n) } }','42\n')
        (root/'references.dev').write_text('struct P { x i64 }; fn make() Ref<P> { return ref(P(42)) }',encoding='utf-8')
        check('use references; print(deref(references.make()).x)','42\n')
        check('enum E { A(i64),B }; match E.A(42) { A(n) if n<10 => { print(0) }; A(n) => { print(n) }; _ => { print(-1) } }','42\n')
        check('enum E { A(i64),B }; match E.B { A(_) => {}; _ => { print(42) } }','42\n')
        check('enum Inner { N,V(i64) }; enum Outer { Some(Inner),None }; match Outer.Some(Inner.V(42)) { Some(V(n)) if n>0 => { print(n) }; _ => {} }','42\n')
        check('enum Inner { N,V(i64) }; enum Outer { Some(Inner),None }; match Outer.Some(Inner.N) { Some(N) => { print(42) }; _ => {} }','42\n')
        check('enum E { V(i64) }; for i in 0..5 { match E.V(i) { V(n) if n==2 => { continue }; V(n) if n==4 => { break }; _ => { print(i) } } }','0\n1\n3\n')
        check('let v=Vec<i64>(); v.push(19); v.push(23); print(v.len()); print(v[0]+v[1]); print(v.pop()); print(v.len()); v.clear(); print(v.len())','2\n42\n23\n1\n0\n')
        check('let a=Vec<i64>(19,23); let b=a; a[0]=99; a.push(1); print(b[0]); print(b.len()); print(a.len())','19\n2\n3\n')
        check('let a=Vec<i64>(19,23,99); let s=a.slice(0,2); a[0]=100; a.clear(); print(s[0]+s[1]); print(s.len())','42\n2\n')
        check('struct P { x i64 }; let v=Vec<P>(P(42)); print(v[0].x); v[0].x=99; print(v[0].x)','42\n99\n')
        check('let v=Vec<Ref<i64>>(ref(19),ref(23)); let s=v.slice(0,2); v.clear(); print(deref(s[0])+deref(s[1]))','42\n')
        check('fn make() Vec<i64> { return Vec<i64>(42) }; print(make()[0]); let r=ref(make()); print(deref(r)[0])','42\n42\n')
        check('let a=Vec<i64>(1,19,23,4); let s=a.slice(1,4).slice(0,2); print(s[0]+s[1])','42\n')
        check('let m=Map<i64,i64>(); m.set(1,19); m.set(2,23); print(m.get(1)+m.get(2)); print(m.len()); print(m.contains(2)); print(m.remove(1)); print(m.contains(1)); m.clear(); print(m.len())','42\n2\ntrue\ntrue\nfalse\n0\n')
        check('struct Ops { call fn(i64) i64 }; let o=Ops(fn(n i64) i64 { return n+1 }); print(o.call(41))','42\n')
        check('async fn work() Ref<i64> { return ref(19) }; let a=then(work(),fn(r Ref<i64>) i64 { return deref(r)+23 }); print(await(a)); print(ready(a)); print(await(a))','42\ntrue\n42\n')
        check('async fn work() {}; let a=then(work(),fn() i64 { return 42 }); print(await(a))','42\n')
        check('async fn work() i64 { return 42 }; let a=then(work(),fn(n i64) { print(n) }); await(a); let b=then(spawn(fn() {}),fn() { print(42) }); await(b)','42\n42\n')
        check('let a=then(spawn(fn() i64 { return 19 }),fn(n i64) i64 { return n+1 }); let b=then(a,fn(n i64) Ref<i64> { return ref(n+22) }); print(deref(await(b)))','42\n')
        check('then(42,fn(n i64) i64 { return n })',error='then expects Task<T>')
        check('then(spawn(fn() i64 { return 42 }),fn(n bool) i64 { return 0 })',error='then continuation parameters')
        check('async fn answer() Ref<i64> { return ref(42) }; let t=answer(); while !ready(t) {}; print(deref(await(t))); print(ready(t)); print(deref(await(t)))','42\ntrue\n42\n')
        check('async fn work() { }; let t=work(); while !ready(t) {}; await(t); print(ready(t))','true\n')
        check('let m=Map<i64,Ref<i64>>(); let seed=1; for i in 0..4096 { seed=(seed*25173+13849)%65536; let key=seed%512; if seed%3==0 { m.remove(key) } else { m.set(key,ref(i)) } }; let sum=0; for i in 0..512 { if m.contains(i) { sum+=deref(m.get(i)) } }; print(sum); print(m.len())','1262831\n329\n')
        check('let m=Map<i64,i64>(); let v=Vec<i64>(); print(sizeof(m)==sizeof(v)*2)','true\n')
        check('let m=Map<i64,i64>(); for i in 0..4096 { m.set(i,i*3) }; let snapshot=m; for i in 0..4096 { if i%2==0 { m.remove(i) } }; let sum=0; for i in 0..4096 { if m.contains(i) { sum+=m.get(i) } }; print(sum); print(m.len()); print(snapshot.len()); print(snapshot.get(42)); for i in 0..4096 { m.set(i,i) }; print(m.len()); print(m.get(42))','12582912\n2048\n4096\n126\n4096\n42\n')
        check('let m=Map<f64,i64>(); m.set(0.0,19); m.set(-0.0,42); print(m.len()); print(m.get(0.0)); for i in 1..300 { m.set(i as f64,i) }; for i in 1..300 { if i%3==0 { m.remove(i as f64) } }; print(m.get(299.0)); print(m.get(-0.0))','1\n42\n299\n42\n')
        check('enum Key { A,B,C }; let m=Map<Key,i64>(); m.set(Key.A,19); m.set(Key.B,23); m.set(Key.C,99); m.remove(Key.A); print(m.get(Key.B)); print(m.get(Key.C)); let b=Map<bool,i64>(); b.set(false,19); b.set(true,23); print(b.get(false)+b.get(true))','23\n99\n42\n')
        check('let m=Map<f64,i64>(); let nan=0.0/0.0; m.set(1.0,19); m.set(nan,23); print(m.contains(nan)); m.remove(1.0); m.set(2.0,42); print(m.get(2.0)); print(m.len())','false\n42\n2\n')
        check('let a=Map<str,i64>(); a.set("answer",42); let b=a; a.set("answer",99); print(b.get("answer")); print(a.get("answer"))','42\n99\n')
        check('struct P { r Ref<i64> }; let m=Map<i64,P>(); m.set(1,P(ref(42))); let p=m.get(1); m.clear(); print(deref(p.r))','42\n')
        check('struct Node { children Vec<Node> }; let n=Node(Vec<Node>()); n.children.push(n); print(n.children.len()); print(n.children[0].children.len())','1\n0\n')
        check('struct Node { children Map<i64,Node> }; let n=Node(Map<i64,Node>()); n.children.set(1,n); print(n.children.len()); print(n.children.get(1).children.len())','1\n0\n')
        check('fn id<T>(v T) T { return v }; print(id(42)); print(id(id(42))); let x i32=id(42); print(x)','42\n42\n42\n')
        check('struct Box<T> { v T }; fn unwrap<T>(b Box<T>) T { return b.v }; let b=Box(42); print(unwrap(b))','42\n')
        check('enum Result<T> { Ok(T),Err(str) }; let r=Result.Ok(42); match r { Ok(n) => { print(n) }; _ => {} }; let e Result<i64>=Result.Err("bad"); match e { Err(s) => { print(s) }; _ => {} }','42\nbad\n')
        check('let v Vec<i64>=Vec(); v.push(42); print(v[0]); let m Map<str,i64>=Map(); m.set("x",42); print(m.get("x"))','42\n42\n')
        check('use payload; match payload.wrap(42) { Ok(n) => { print(n) }; _ => {} }','42\n')
        check('struct Point { x i64; y i64 }; fn Point.sum(self Point) i64 { return self.x+self.y }; let p=Point(19,23); print(p.sum()); print(Point(19,23).sum())','42\n42\n')
        check('struct Box<T> { v T }; fn Box.get<T>(self Box<T>) T { return self.v }; let b=Box(42); print(b.get())','42\n')
        check('fn inc(x i64) i64 { return x+1 }; let f=inc; print(f(41)); fn apply(f fn(i64) i64,x i64) i64 { return f(x) }; print(apply(inc,41))','42\n42\n')
        check('let offset=19; let f=fn(x i64) i64 { return offset+x }; offset=99; print(f(23))','42\n')
        check('fn make(n i64) fn(i64) i64 { return fn(x i64) i64 { return n+x } }; let f=make(19); print(f(23)); print(make(19)(23))','42\n42\n')
        check('let n=ref(19); let f=fn(x i64) i64 { return deref(n)+x }; let v=Vec<fn(i64) i64>(f); let s=v.slice(0,1); v.clear(); print(s[0](23))','42\n')
        check('fn sum<T:Number>(a T,b T) T { return a+b }; print(sum(19,23)); print(sum<f64>(19,23))','42\n42\n')
        check('trait HasValue { fn value(self Self) i64 }; struct P { x i64 }; fn P.value(self P) i64 { return self.x }; fn read<T:HasValue>(p T) i64 { return p.value() }; print(read(P(42)))','42\n')
        check('fn answer<const N>() i64 { return N }; print(answer<42>())','42\n')
        check('struct Buffer<T,const N> { data [T;N] }; let b=Buffer<i64,2>([19,23]); print(b.data[0]+b.data[1])','42\n')
        check('async fn answer(n i64) i64 { return n+1 }; let t=answer(41); print(await(t)); print(await(t))','42\n42\n')
        check('let n=19; let a=spawn(fn() i64 { return n+23 }); let b=spawn(fn() i64 { return n*2 }); print(await(a)); print(await(b))','42\n38\n')
        check('let v=Vec<Ref<i64>>(ref(42)); let t=spawn(fn() Ref<i64> { return v[0] }); v.clear(); print(deref(await(t)))','42\n')
        check('let v=Vec<Vec<i64>>(Vec<i64>(19)); v[0].push(23); print(v[0][0]+v[0][1]); let m=Vec<Map<i64,i64>>(Map<i64,i64>()); m[0].set(1,42); print(m[0].get(1))','42\n42\n')
        check('struct Buffer<T,const N> { data [T;N] }; let a=[19,23]; let b=Buffer<i64,2>(a); a[0]=99; print(b.data[0]+b.data[1])','42\n')
        check('fn side() i64 { print(99); return 1 }; print(sizeof(side())); let v=Vec<i64>(); print(sizeof(v)); let a=[1,2]; print(sizeof(a))','8\n24\n16\n')
        (root/'advanced_module.dev').write_text('struct Box<T> { v T }; fn Box.get<T>(self Box<T>) T { return self.v }; fn make(n i64) fn(i64) i64 { return fn(x i64) i64 { return n+x } }; async fn work(n i64) Ref<i64> { return ref(n) }',encoding='utf-8')
        check('use advanced_module as a; let b=a.Box(42); print(b.get()); print(a.make(19)(23)); let task=a.work(42); print(deref(await(task)))','42\n42\n42\n')
        check('struct Buffer<T,const N> { data [T;N] }; let a=[19,23]; let b=Buffer(a); print(b.data[0]+b.data[1])','42\n')
        check('let task=spawn(fn() { print(42) }); await(task)','42\n')
        check('let closure_env=19; let f=fn(x i64) i64 { return closure_env+x }; print(f(23))','42\n')
        for code, message in [
            ('fn f<T:Number>(x T) T { return x }; print(f(true))','does not satisfy constraint'),
            ('fn f<const N>() i64 { return N }; print(f<i64>())','parameter kind'),
            ('enum I { N,V(i64) }; enum E { A(I) }; match E.A(I.N) { A(N) => {} }','non-exhaustive match'),
            ('enum E { A }; match E.A { _ => {}; A => {} }','unreachable arm'),
            ('enum E { A }; match E.A { A if 1 => {}; _ => {} }','bool'),
            ('enum E { A(i64),B }; match E.B { B => {} }','non-exhaustive match'),
            ('enum E { A }; match E.A { A => {}; A => {} }','duplicate match variant'),
            ('enum E { A }; match E.A { B => {} }','unknown enum variant'),
            ('enum E { A(i64) }; match E.A(1) { A => {} }','wrong match payload binding count'),
            ('enum E { A(i64,i64) }; match E.A(1,2) { A(x,x) => {} }','duplicate match binding'),
            ('match 1 { A => {} }','match requires an enum'),
            ('enum E { A(i64) }; let e = E.A','requires payload arguments'),
            ('enum E { A(i64) }; let e = E.A()','wrong enum payload argument count'),
            ('enum E { A(void) }','payload cannot be void'),
            ('enum E { A(i64) }; let n = E.A(1) as i64','payload enums cannot be cast'),
            ('let r = ref(42) as *i64',''),
            ('let r = ref(42); *r = 1',''),
            ('struct P { x i64 }; let r = ref(P(42)); deref(r).x = 99',''),
            ('export fn f(v Ref<i64>) {}','managed references cannot cross'),
            ('enum E { R(Ref<i64>) }; export fn f(v E) {}','managed references cannot cross'),
            ('enum E { A(i64) }; let n = E.A(true)','expected i64'),
            ('let r = ref<i64>(42)','do not take explicit type'),
            ('let r = deref(42)','deref requires a managed Ref'),
            ('let r = ref([1,2])','ref requires a non-array value'),
            ('struct Node { n Ref<Node>; other Node }','recursive type layout'),
        ]:
            if message:
                check(code,error=message)
            else:
                source.write_text(code,encoding='utf-8')
                for mode in ('auto','ast','native'):
                    command=([str(folder/('devc'+suffix)),'check',str(source)] if mode=='native' else [str(folder/('devrun'+suffix)),str(source),'--engine',mode])
                    result=subprocess.run(command,capture_output=True,text=True,timeout=45)
                    assert result.returncode!=0 and 'main.dev:' in result.stderr,(mode,code,result.stderr)
                count+=1
        for code,error in [
            ('for i in 0..2 {}; print(i)','unknown variable'),
            ('for i in 0..true {}','expected i64'),
            ('struct Point { x i64; x i64 }','duplicate field'),
            ('struct Node { next Node }','recursive type layout'),
            ('struct Box<T> { v T }; fn grow<T>(v T) { grow<Box<T>>(Box<T>(v)) }; grow<i64>(1)','nesting limit'),
            ('struct i64 { value bool }','type name is reserved'),
            ('fn identity<i64>(v i64) i64 { return v }','type parameter name is reserved'),
            ('struct Bad { value void }','cannot be void'),
            ('enum Mode {}','needs a variant'),
            ('enum Mode { A,A }','duplicate field'),
            ('enum Mode { A }; print(Mode.B)','unknown enum variant'),
            ('enum A { X }; enum B { X }; let a = A.X; a = B.X',None),
            ('struct Point { x i64 }; let p = Point()','wrong struct field count'),
            ('struct Point { x i64 }; let p = Point(true)','expected i64'),
            ('struct Point { x i64 }; let p = Point(1); print(p.y)','unknown struct field'),
            ('struct Point { x i64 }; let p = Point(1); p.x = true','expected i64'),
            ('fn identity<T>(v T) T { return v }; print(identity<i64,bool>(42))','wrong type argument count'),
            ('fn identity<T>(v T) T { return v }; print(identity<void>(42))','void is not'),
            ('struct Box<T> { v T }; let b = Box<void>()','cannot be void'),
            ('fn f<T,T>(v T) T { return v }','duplicate field'),
            ('export fn f<T>(v T) T { return v }','cannot be extern'),
        ]:
            if error is None:
                # Nominal type mismatch text includes each mode's internal type identity.
                error='expected'  # both paths reject assignment across enum types
            check(code,error=error)
    print(f'PASS: {count} language-feature scenarios across auto, AST and native modes')

if __name__=='__main__':
    main()
