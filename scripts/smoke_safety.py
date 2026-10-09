"""Safety boundaries and checked arithmetic in source and native execution."""
import argparse,os,subprocess,tempfile
from pathlib import Path

def main():
    ap=argparse.ArgumentParser();ap.add_argument('--bin-dir',required=True);folder=Path(ap.parse_args().bin_dir).resolve();ext='.exe' if os.name=='nt' else '';count=0
    with tempfile.TemporaryDirectory(prefix='dev-safety-') as directory:
        root=Path(directory);source=root/'main.dev'
        def check(code,expected=None,error=None):
            nonlocal count
            source.write_text(code,encoding='utf-8')
            for mode in ['auto','ast','native']:
                command=[str(folder/('devc'+ext)),'run',str(source),'-o',str(root/('program'+ext))] if mode=='native' else [str(folder/('devrun'+ext)),str(source),'--engine',mode]
                if mode=='native' and (root/'native.c').exists():command += ['--link',str(root/'native.c')]
                r=subprocess.run(command,capture_output=True,text=True,timeout=45)
                if error:assert r.returncode!=0 and error in r.stderr and 'main.dev:' in r.stderr,(mode,code,r.stdout,r.stderr)
                else:assert r.returncode==0 and r.stdout==expected,(mode,code,r.stdout,r.stderr)
            count+=1
        for code,message in [
            ('let x=1; let p=&x','requires an unsafe block'),
            ('let p=1 as *i64','requires an unsafe block'),
            ('extern fn puts(s str) i32; puts("bad")','requires an unsafe block'),
            ('fn bad(p *i64) i64 { return *p }','requires an unsafe block'),
            ('let a=[1,2]; let i=2; print(a[i])','array index out of bounds'),
            ('let a=[1,2]; let i=-1; print(a[i])','index'),
            ('let n=0; print(42/n)','division by zero'),
            ('let n=0; let a=42; a%=n','division by zero'),
            ('let n=64; print(1<<n)','shift count out of range'),
            ('let n=-1; print(1>>n)','shift count out of range'),
            ('let s="hi"; let i=2; print(s[i])','string index out of bounds'),
            ('unsafe { let p=null as *i64; print(*p) }','null pointer dereference'),
        ]:
            check(code,error=message)
        check('let n i64=-9223372036854775808; print(n/-1); print(n%-1)','-9223372036854775808\n0\n')
        check('let n i64=-1; print(n<<1)','-2\n')
        (root/'native.c').write_text('#include <stdint.h>\n#include <stdlib.h>\ntypedef struct { int32_t x; double y; } Point;\nPoint native_point(Point p) { p.x+=1; p.y+=0.5; return p; }\nvoid *native_alloc(void) { return calloc(4,sizeof(int64_t)); }\nvoid native_free(void *p) { free(p); }',encoding='utf-8')
        check('extern fn printf(format str, ...) i32; unsafe { printf("%d %.1f %s\\n",7 as i8,1.5 as f32,"ok"); printf("%d\\n",true) }','7 1.5 ok\n1\n')
        check('fn add(x i64) i64 { return x+1 }; unsafe { let cb=callback(add); print(cb(41)) }','42\n')
        with (root/'native.c').open('a',encoding='utf-8') as fixture:fixture.write('\nint64_t native_callback(int64_t (*cb)(int64_t),int64_t x) { return cb(x); }\nint64_t native_context(int64_t (*cb)(int64_t,void*),int64_t x,void *data){return cb(x,data);}\n')
        check('extern fn native_callback(cb callback(i64) i64,x i64) i64; fn make(n i64) fn(i64) i64 { let r=ref(n); return fn(x i64) i64 { return deref(r)+x } }; unsafe { let a=callback(make(19)); let b=callback(make(23)); print(native_callback(a,23)); print(native_callback(b,19)); print(a(23)) }','42\n42\n42\n')
        check('extern fn native_callback(cb callback(i64) i64,x i64) i64; unsafe { let callbacks=Vec<callback(i64) i64>(); for i in 0..32 { callbacks.push(callback(fn(x i64) i64 { return i+x })) }; print(native_callback(callbacks[0],42)); print(native_callback(callbacks[31],11)) }','42\n42\n')
        check('fn make(n i64) fn(i64) i64 { return fn(x i64) i64 { return n+x } }; let f=make(19); unsafe { let sum=0; for i in 0..200 { let cb=callback(f); sum+=cb(23) }; print(sum) }','8400\n')
        check('extern fn native_context(cb callback(i64,*void) i64,x i64,data *void) i64; let contexts=Vec<ContextCallback<fn(i64) i64>>(); unsafe { for i in 0..200 { let r=ref(i); contexts.push(callback_context(fn(x i64) i64 { return deref(r)+x })) }; let keep=contexts[42]; contexts.clear(); print(native_context(keep.call,0,keep.data)); print(keep.call(0,keep.data)) }','42\n42\n')
        check('callback_context(fn(n i64) i64 { return n })',error='requires an unsafe block')
        check('let t=then(spawn(fn() i64 { let zero=0; return 42/zero }),fn(n i64) i64 { print(99); return n }); print(await(t))',error='division by zero')
        check('fn make_callback(n i64) callback(i64) i64 { unsafe { return callback(fn(x i64) i64 { return n+x }) } }; unsafe { let factory=callback(make_callback); let a=factory(19); let b=factory(23); print(a(23)); print(b(19)) }','42\n42\n')
        check('async fn make_callback(n i64) callback(i64) i64 { unsafe { return callback(fn(x i64) i64 { return n+x }) } }; let a=await(make_callback(19)); unsafe { print(a(23)) }','42\n')
        with (root/'native.c').open('a',encoding='utf-8') as fixture:fixture.write('\n#include <stdatomic.h>\nstatic atomic_int gate_open, gate_done;\nvoid native_gate_open(void){atomic_store(&gate_open,1);}\nvoid native_gate_wait(void){while(!atomic_load(&gate_open)){} atomic_store(&gate_done,1);}\nint native_gate_done(void){return atomic_load(&gate_done); }\n')
        check('extern fn native_gate_open(); extern fn native_gate_wait(); async fn work() i64 { unsafe { native_gate_wait() }; return 19 }; let t=then(work(),fn(n i64) i64 { return n+23 }); unsafe { native_gate_open() }; print(await(t))','42\n')
        check('extern fn native_gate_open(); extern fn native_gate_wait(); extern fn native_gate_done() i32; async fn work() Ref<i64> { unsafe { native_gate_wait() }; return ref(42) }; if true { let detached=work() }; unsafe { native_gate_open(); while native_gate_done()==0 {} }; print(42)','42\n')
        check('fn bad(x i64) i64 { let zero=0; return x/zero }; extern fn native_callback(cb callback(i64) i64,x i64) i64; unsafe { print(native_callback(callback(bad),41)) }',error='division by zero')
        check('fn add(x i64) i64 { return x+1 }; extern fn native_callback(cb callback(i64) i64,x i64) i64; unsafe { print(native_callback(callback(add),41)) }','42\n')
        with (root/'native.c').open('a',encoding='utf-8') as fixture:fixture.write('\nfloat native_callback_float(float (*cb)(float),float x){return cb(x); }\nint8_t native_callback_byte(int8_t (*cb)(int8_t),int8_t x){return cb(x); }\nvoid native_callback_void(void (*cb)(void)){cb();}\n')
        check('fn inc(x f32) f32 { return x+1 }; extern fn native_callback_float(cb callback(f32) f32,x f32) f32; unsafe { print(native_callback_float(callback(inc),41)) }','42\n')
        check('fn neg(x i8) i8 { return -x }; extern fn native_callback_byte(cb callback(i8) i8,x i8) i8; unsafe { print(native_callback_byte(callback(neg),1)) }','-1\n')
        check('extern fn native_callback_float(cb callback(f32) f32,x f32) f32; let offset f32=19; unsafe { let cb=callback(fn(x f32) f32 { return offset+x }); print(native_callback_float(cb,23)) }','42\n')
        check('extern fn native_callback_byte(cb callback(i8) i8,x i8) i8; let offset i8=-2; unsafe { let cb=callback(fn(x i8) i8 { return offset+x }); print(native_callback_byte(cb,1)) }','-1\n')
        check('extern fn native_callback_void(cb callback() void); let r=ref(42); unsafe { native_callback_void(callback(fn() { print(deref(r)) })) }','42\n')
        check('fn say() { print(42) }; extern fn native_callback_void(cb callback() void); unsafe { native_callback_void(callback(say)) }','42\n')
        # Native trampolines must fail at capacity instead of aliasing an old environment.
        code='unsafe { let callbacks=Vec<callback(i64) i64>(); for i in 0..65 { callbacks.push(callback(fn(x i64) i64 { return i+x })) }; print(callbacks[64](1)) }'
        source.write_text(code,encoding='utf-8')
        for mode in ['auto','ast','native']:
            command=[str(folder/('devc'+ext)),'run',str(source),'-o',str(root/('program'+ext))] if mode=='native' else [str(folder/('devrun'+ext)),str(source),'--engine',mode]
            r=subprocess.run(command,capture_output=True,text=True,timeout=45)
            if mode=='native': assert r.returncode!=0 and 'callback capacity exceeded' in r.stderr and 'main.dev:' in r.stderr,(r.stdout,r.stderr)
            else: assert r.returncode==0 and r.stdout=='65\n',(mode,r.stdout,r.stderr)
        count+=1
        source.write_text('let holder=Vec<ContextCallback<fn(i64) i64>>(); unsafe { holder.push(callback_context(fn(n i64) i64 { return n+1 })); let call=holder[0].call; let data=holder[0].data; holder.clear(); print(call(41,data)) }',encoding='utf-8')
        for mode in ['auto','ast']:
            r=subprocess.run([str(folder/('devrun'+ext)),str(source),'--engine',mode],capture_output=True,text=True,timeout=45)
            assert r.returncode!=0 and 'context callback owner has expired' in r.stderr,(mode,r.stdout,r.stderr)
        count+=1
        declarations='extern fn native_alloc() *void; extern fn native_free(p *void); '
        check(declarations+'unsafe { let p=native_alloc() as *i64; p[0]=19; p[1]=23; print(p[0]+p[1]); native_free(p as *void) }','42\n')
        check(declarations+'unsafe { let p=native_alloc() as *i64; volatile_store(p,42); print(volatile_load(p)); native_free(p as *void) }','42\n')
        check(declarations+'struct Point { x i32; y f64 }; unsafe { let p=native_alloc() as *Point; (*p).x=42; (*p).y=2; print((*p).x); print((*p).y); native_free(p as *void) }','42\n2\n')
        check('struct Point { x i32; y f64 }; extern fn native_point(p Point) Point; unsafe { let p=native_point(Point(41,1.5)); print(p.x); print(p.y) }','42\n2\n')
    print(f'PASS: {count} safety/aggregate-FFI scenarios')

if __name__=='__main__':main()
