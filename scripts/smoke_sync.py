"""Exercise shared channels, mutex callbacks and cooperative cancellation on real workers."""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True)
    d = Path(parser.parse_args().bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-sync-') as temp:
        source = Path(temp) / 'main.dev'

        def run(body, expected=None, error=None):
            nonlocal count
            source.write_text('use "std/sync"\nuse "std/result"\nuse "std/timers"\n' + body, encoding='utf-8')
            for engine in ('auto', 'ast'):
                r = subprocess.run([str(d), str(source), '--engine', engine], capture_output=True, text=True, encoding='utf-8', timeout=30, env=dict(os.environ, PATH='', DEV_CC='missing'))
                if error:
                    assert r.returncode != 0 and error in r.stderr, (body, r.stdout, r.stderr)
                else:
                    assert r.returncode == 0 and r.stdout == expected, (body, r.stdout, r.stderr)
                count += 1

        run('''fn main() {
let q=sync.channel<i64>(2)
print(q.capacity())
print(q.len())
print(q.trySend(1))
print(q.trySend(2))
print(q.trySend(3))
match q.receive() { Item(value) => { print(value) } Empty => { print("empty") } Closed => { print("closed") } }
print(q.sendTimeout(3,0))
print(q.close())
print(q.close())
print(q.isClosed())
print(q.send(4))
for i in 0..3 {
match q.tryReceive() { Item(value) => { print(value) } Empty => { print("empty") } Closed => { print("closed") } }
}
}
main()
''', '2\n0\ntrue\ntrue\nfalse\n1\ntrue\ntrue\nfalse\ntrue\nfalse\n2\n3\nclosed\n')
        run('''fn main() {
let q=sync.channel<str>(1)
match q.tryReceive() { Item(value) => { print(value) } Empty => { print("empty") } Closed => { print("closed") } }
match q.receiveTimeout(2) { Item(value) => { print(value) } Empty => { print("timeout") } Closed => { print("closed") } }
q.send("ไทย")
print(q.sendTimeout("full",2))
q.close()
match q.receive() { Item(value) => { print(value) } Empty => { print("empty") } Closed => { print("closed") } }
}
main()
''', 'empty\ntimeout\nfalse\nไทย\n')
        run('''fn main() {
let q=sync.channel<i64>(1)
let a=spawn(fn() i64 { for i in 0..1000 { if !q.send(i) { return -1 } }; return 1000 })
let b=spawn(fn() i64 { for i in 0..1000 { if !q.send(i) { return -1 } }; return 1000 })
let total=0
for i in 0..2000 {
match q.receive() { Item(value) => { total+=value } Empty => {} Closed => {} }
}
q.close()
print(total)
print(await(a))
print(await(b))
}
main()
''', '999000\n1000\n1000\n')
        run('''fn main() {
let q=sync.channel<i64>(1)
let waiting=sync.channel<bool>(1)
let a=spawn(fn() bool {
waiting.send(true)
match q.receive() { Item(value) => { return false } Empty => { return false } Closed => { return true } }
return false
})
waiting.receive()
q.close()
print(await(a))
let full=sync.channel<i64>(1)
full.send(1)
let b=spawn(fn() bool { return full.send(2) })
full.close()
print(await(b))
}
main()
''', 'true\nfalse\n')
        run('''struct Item { name str, data Vec<i64> }
fn main() {
let q=sync.channel<Item>(1)
let data=Vec<i64>()
data.push(42)
let worker=spawn(fn() bool { return q.send(Item("worker",data)) })
match q.receive() { Item(item) => { print(item.name); item.data.push(43); print(item.data.len()) } Empty => {} Closed => {} }
print(await(worker))
print(data.len())
let numbers=sync.channel<Vec<i64>>(1)
numbers.send(data)
data.push(99)
match numbers.receive() { Item(item) => { print(item.len()); print(item[0]) } Empty => {} Closed => {} }
q.close()
numbers.close()
}
main()
''', 'worker\n2\ntrue\n1\n1\n42\n')
        run('''fn main() {
let q=sync.channel<Ref<i64>>(1)
q.send(ref(42))
match q.receive() { Item(value) => { print(deref(value)) } Empty => {} Closed => {} }
let m=sync.mutex(0)
let a=spawn(fn() { for i in 0..500 { m.update(fn(n i64) i64 { return n+1 }) } })
let b=spawn(fn() { for i in 0..500 { m.update(fn(n i64) i64 { return n+1 }) } })
let c=spawn(fn() { for i in 0..500 { m.update(fn(n i64) i64 { return n+1 }) } })
let e=spawn(fn() { for i in 0..500 { m.update(fn(n i64) i64 { return n+1 }) } })
await(a)
await(b)
await(c)
await(e)
print(m.get())
print(m.update(fn(n i64) i64 { return n+42 }))
m.set(5)
print(m.get())
m.close()
print(result.isErr(result.run(fn() { m.get() })))
}
main()
''', '42\n2000\n2042\n5\ntrue\n')
        run('''fn main() {
let m=sync.mutex(42)
let bad=result.run(fn() { m.update(fn(n i64) i64 { result.raise("failed"); return n+1 }) })
print(result.isErr(bad))
print(m.get())
let nested=result.run(fn() { m.update(fn(n i64) i64 { return m.get() }) })
print(result.isErr(nested))
print(m.get())
let closed=result.run(fn() { m.update(fn(n i64) i64 { m.close(); return n }) })
print(result.isErr(closed))
print(m.updateTimeout(fn(n i64) i64 { return n+1 },0))
m.close()
}
main()
''', 'true\n42\ntrue\n42\ntrue\n43\n')
        run('''fn main() {
let m=sync.mutex(0)
let started=sync.channel<bool>(1)
let release=sync.channel<bool>(1)
let worker=spawn(fn() i64 { return m.update(fn(n i64) i64 { started.send(true); release.receive(); return n+1 }) })
started.receive()
print(result.isErr(result.run(fn() { m.updateTimeout(fn(n i64) i64 { return n+1 },2) })))
release.send(true)
print(await(worker))
print(m.get())
m.close()
started.close()
release.close()
}
main()
''', 'true\n1\n1\n')
        run('''fn main() {
let data=Vec<i64>()
data.push(1)
let m=sync.mutex(data)
let copy=m.get()
copy.push(2)
print(m.get().len())
m.update(fn(values Vec<i64>) Vec<i64> { values.push(42); return values })
print(m.get().len())
print(data.len())
m.close()
}
main()
''', '1\n2\n1\n')
        run('''fn main() {
let token=sync.token()
print(token.isCancelled())
print(token.wait(0))
print(token.wait(2))
let worker=spawn(fn() bool { return token.wait(5000) })
print(token.cancel())
print(token.cancel())
print(await(worker))
print(token.isCancelled())
print(token.wait(0))
print(result.isErr(result.run(fn() { token.check() })))
let other=sync.token()
print(result.isOk(result.run(fn() { other.check() })))
}
main()
''', 'false\nfalse\nfalse\ntrue\nfalse\ntrue\ntrue\ntrue\ntrue\ntrue\n')
        run('''fn main() {
let token=sync.token()
let started=sync.channel<bool>(1)
let worker=spawn(fn() i64 {
started.send(true)
while !token.wait(1) { }
return 42
})
started.receive()
token.cancel()
print(await(worker))
started.close()
}
main()
''', '42\n')
        for body, error in [
            ('sync.channel<i64>(0)', 'capacity'),
            ('sync.channel<i64>(65537)', '65536'),
            ('sync.channel<i64>(-1)', 'negative'),
            ('let q=sync.channel<i64>(1); q.receiveTimeout(-1)', 'negative'),
            ('let q=sync.channel<i64>(1); q.sendTimeout(1,300001)', '300000'),
            ('sync.token().wait(300001)', '300000'),
            ('sync.Channel<i64>().send(1)', 'must be created'),
            ('sync.Mutex<i64>().get()', 'must be created'),
            ('sync.CancelToken().check()', 'must be created'),
            ('sync.mutex(fn() {})', 'payload must be data'),
            ('let q=sync.channel<sync.CancelToken>(1); q.send(sync.token())', 'payload must be data'),
            ('let q=sync.channel<fn() void>(1); q.send(fn() {})', 'payload must be data'),
            ('let q=sync.channel<i64>(1); q.send("wrong")', 'type'),
            ('let m=sync.mutex(1); m.update(fn(x str) str { return x })', 'type'),
        ]:
            run(body, error=error)
        run('''fn main() {
let retained=Vec<sync.CancelToken>()
for i in 0..64 { retained.push(sync.token()) }
print(result.isErr(result.run(fn() { sync.token() })))
retained=Vec<sync.CancelToken>()
print(sync.token().isCancelled())
for i in 0..100 { let token=sync.token(); token.cancel() }
print("released")
}
main()
''', 'true\nfalse\nreleased\n')
        run('''use "std/strings"
fn main() {
let q=sync.channel<str>(3)
let value=strings.repeat("x",3145728)
print(q.trySend(value))
print(q.trySend(value))
print(q.trySend(value))
q.receive()
print(q.trySend(value))
q.close()
print(result.isErr(result.run(fn() { sync.mutex(strings.repeat("x",8388608)) })))
}
main()
''', 'true\ntrue\nfalse\ntrue\ntrue\n')
        run('''enum Link { End, Next(Ref<Link>) }
fn main() {
let value=Link.End()
for i in 0..70 { value=Link.Next(ref(value)) }
print(result.isErr(result.run(fn() { sync.mutex(value) })))
let m=sync.mutex(1)
print(m.close())
print(m.close())
}
main()
''', 'true\ntrue\nfalse\n')
        run('''use "std/json"
fn main() {
let q=sync.channel<json.Value>(1)
let worker=spawn(fn() { q.send(json.parse("{\\"name\\":\\"Dev\\"}")) })
match q.receive() { Item(value) => { print(json.string(json.get(value,"name"))) } Empty => {} Closed => {} }
await(worker)
q.close()
let entries=Map<str,i64>()
entries.set("answer",42)
let m=sync.mutex(entries)
let updated=m.update(fn(values Map<str,i64>) Map<str,i64> { values.set("answer",43); return values })
print(updated.get("answer"))
print(entries.get("answer"))
m.close()
}
main()
''', 'Dev\n43\n42\n')
        run('''fn main() {
let q=sync.channel<i64>(1)
let total=sync.mutex(0)
let a=spawn(fn() { while true {
match q.receive() { Item(value) => { total.update(fn(n i64) i64 { return n+value }) } Empty => {} Closed => { return } }
} })
let b=spawn(fn() { while true {
match q.receive() { Item(value) => { total.update(fn(n i64) i64 { return n+value }) } Empty => {} Closed => { return } }
} })
for i in 0..1000 { q.send(i) }
q.close()
await(a)
await(b)
print(total.get())
total.close()
}
main()
''', '499500\n')
        run('''fn main() {
let token=sync.token()
let started=sync.channel<bool>(1)
let waiting=spawn(fn() bool { started.send(true); return token.wait(5000) })
started.receive()
timers.sleep(20)
print(ready(waiting))
token.cancel()
print(await(waiting))
started.close()
let m=sync.mutex(42)
let entered=sync.channel<bool>(1)
let release=sync.channel<bool>(1)
let update=spawn(fn() i64 { return m.update(fn(n i64) i64 { entered.send(true); release.receive(); return n+1 }) })
entered.receive()
let closer=spawn(fn() bool { return m.close() })
timers.sleep(10)
print(ready(closer))
release.send(true)
print(await(update))
print(await(closer))
print(m.close())
print(result.isErr(result.run(fn() { m.get() })))
entered.close()
release.close()
}
main()
''', 'false\ntrue\nfalse\n43\ntrue\nfalse\ntrue\n')
        run('''fn main() {
let m=sync.mutex(42)
let failed=spawn(fn() i64 { return m.update(fn(n i64) i64 { result.raise("worker failed"); return n+1 }) })
print(result.isErr(result.attempt(fn() i64 { return await(failed) })))
print(m.get())
print(m.update(fn(n i64) i64 { return n+1 }))
m.close()
}
main()
''', 'true\n42\n43\n')
        (source.parent / 'worker.dev').write_text('''use "std/sync"
async fn add(counter sync.Mutex<i64>, token sync.CancelToken) i64 {
token.check()
return counter.update(fn(n i64) i64 { return n+1 })
}
''', encoding='utf-8')
        run('''use "worker"
fn main() {
let counter=sync.mutex(41)
let token=sync.token()
print(await(worker.add(counter,token)))
print(counter.get())
token.cancel()
print(result.isErr(result.attempt(fn() i64 { return await(worker.add(counter,token)) })))
print(counter.get())
counter.close()
}
main()
''', '42\n42\ntrue\n42\n')
    print(f'PASS: {count} sync executions across auto/AST, real producers, contended mutex and cancellation')


if __name__ == '__main__':
    main()
