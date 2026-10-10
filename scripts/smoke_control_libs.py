"""Validate recovery, timer lifetime and bounded subprocesses in both engines."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True)
    args = parser.parse_args()
    d = Path(args.bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-control-') as temporary:
        root = Path(temporary)
        source = root / 'main.dev'
        env = dict(os.environ, PATH='', DEV_CC='missing-compiler')

        def run(code, expected=None, error=None, check=None, arguments=(), exit_code=0):
            nonlocal count
            source.write_text(code, encoding='utf-8')
            for engine in ('auto', 'ast'):
                command = [str(d), str(source), '--engine', engine]
                if arguments:
                    command += ['--', *arguments]
                started = time.monotonic()
                result = subprocess.run(command, cwd=root, env=env, capture_output=True,
                                        text=True, encoding='utf-8', timeout=25)
                if error:
                    assert result.returncode != 0 and error in result.stderr, (code, result.stdout, result.stderr)
                else:
                    assert result.returncode == exit_code, (code, result.stdout, result.stderr)
                    if expected is not None:
                        assert result.stdout == expected, (code, result.stdout, result.stderr)
                if check:
                    check(result, time.monotonic() - started)
                count += 1

        run('''use "std/result"
use "std/fs"
use "std/strings"
let good=result.attempt(fn() i64 { return 42 })
print(result.isOk(good))
print(result.isErr(good))
print(result.unwrap(good))
print(result.message(good) == "")
let bad=result.attempt(fn() str { return fs.read_text("missing") })
print(result.isErr(bad))
print(strings.contains(result.message(bad),"main.dev:"))
print(result.unwrapOr(bad,"fallback"))
match bad { Ok(text) => { print(text) } Err(message) => { print("caught") } }
let done=result.run(fn() { print("void body") })
print(result.unwrap(done))
let raised=result.run(fn() { result.raise("custom failure") })
print(strings.contains(result.message(raised),"custom failure"))
print("continued")
''', 'true\nfalse\n42\ntrue\ntrue\ntrue\nfallback\ncaught\nvoid body\ntrue\ntrue\ncontinued\n')
        run('''use "std/result"
use "std/strings"
fn divide(n i64) i64 { return 12 / n }
fn recurse() i64 { return recurse() }
fn apply<T>(body fn() T) T { return body() }
for i in 0..160 {
    let outcome=result.attempt(fn() i64 { return divide(0) })
    if not result.isErr(outcome) { result.raise("missed divide error") }
}
let overflow=result.attempt(recurse)
print(result.isErr(overflow))
print(strings.contains(result.message(overflow),"recursion limit"))
print(apply(fn() i64 { return divide(3) }))
print(result.unwrap(result.attempt(fn() i64 { return 7 })))
''', 'true\ntrue\n4\n7\n')
        run('''use "std/result"
let value=result.attempt(fn() Vec<i64> { let data=Vec<i64>(); data.push(5); return data })
let data=result.unwrap(value)
print(data[0])
let failure=result.attempt(fn() i64 { let data=Vec<i64>(); return data[0] })
print(result.isErr(failure))
let nested=result.attempt(fn() result.Result<i64> { return result.attempt(fn() i64 { return 8 }) })
print(result.unwrap(result.unwrap(nested)))
''', '5\ntrue\n8\n')
        run('use "std/result"\nresult.raise("uncaught failure")', error='uncaught failure')
        run('''use "std/result"
let failure=result.run(fn() { result.raise("unwrap failure") })
result.unwrap(failure)
''', error='unwrap failure')
        run('''use "std/result"
use "std/fs/promises"
let task=promises.readFile("missing-task", "utf8")
let outcome=result.attempt(fn() str { return await(task) })
print(result.isErr(outcome))
print("after task")
''', 'true\nafter task\n')
        run('''use "std/timers"
print("before")
timers.setTimeout(fn(timer timers.Timer) { print("timeout"); print(timer.ticks()) },0)
print("after")
''', 'before\nafter\ntimeout\n1\n')
        run('''use "std/timers"
let cancelled=timers.setTimeout(fn(timer timers.Timer) { print("BAD") },0)
print(timers.clearTimeout(cancelled))
print(timers.clearInterval(cancelled))
let interval=timers.setInterval(fn(timer timers.Timer) {
    print(timer.ticks())
    if timer.ticks() == 3 { print(timer.cancel()) }
},1)
print(interval.hasRef())
timers.run()
print(interval.cancel())
print("done")
''', 'true\nfalse\ntrue\n1\n2\n3\ntrue\nfalse\ndone\n')
        run('''use "std/timers"
let timer=timers.setTimeout(fn(timer timers.Timer) { print("BAD") },1000)
timer.unref()
print(timer.hasRef())
''', 'false\n', check=lambda r, duration: (_ for _ in ()).throw(AssertionError(duration)) if duration > 0.8 else None)
        run('''use "std/timers"
let timer=timers.setImmediate(fn(timer timers.Timer) { print("immediate") })
timer.unref()
timer.ref()
print(timer.hasRef())
timers.setTimeout(fn(timer timers.Timer) { print("later") },10)
''', 'true\nimmediate\nlater\n')
        run('''use "std/timers"
use "std/result"
let timer=timers.setTimeout(fn(timer timers.Timer) { result.raise("callback failure") },0)
let outcome=result.run(fn() { timers.run() })
print(result.isErr(outcome))
print(timer.cancel())
timers.setImmediate(fn(timer timers.Timer) { print("recovered") })
''', 'true\nfalse\nrecovered\n')
        run('''use "std/timers"
use "std/result"
timers.setImmediate(fn(timer timers.Timer) {
    let outcome=result.run(fn() { timers.run() })
    print(result.isErr(outcome))
})
''', 'true\n')
        run('''use "std/timers"
use "std/result"
for i in 0..64 { timers.setTimeout(fn(timer timers.Timer) {},0) }
let capacity=result.attempt(fn() timers.Timer { return timers.setImmediate(fn(timer timers.Timer) {}) })
print(result.isErr(capacity))
timers.run()
timers.setImmediate(fn(timer timers.Timer) { print("capacity recovered") })
''', 'true\ncapacity recovered\n')
        for expression, error in [
            ('timers.setTimeout(fn(timer timers.Timer) {},-1)', 'negative'),
            ('timers.setTimeout(fn(timer timers.Timer) {},300001)', 'exceeds'),
            ('timers.setInterval(fn(timer timers.Timer) {},0)', 'at least 1'),
            ('timers.sleep(-1)', 'negative')]:
            run('use "std/timers"\n' + expression, error=error)
        run('use "std/timers"\ntimers.setImmediate(fn(timer timers.Timer) { print("BAD") })\nreturn 7', '', exit_code=7)

        # Real subprocesses use an explicit Python executable even with PATH empty.
        program = root / 'child.py'
        program.write_text('''import os, subprocess, sys, time
mode=sys.argv[1]
if mode == 'echo':
    data=sys.stdin.buffer.read()
    sys.stdout.buffer.write(data)
    sys.stderr.buffer.write(b'\\x00\\xffstderr')
elif mode == 'args':
    print(sys.argv[2])
elif mode == 'cwd':
    print(os.getcwd())
elif mode == 'nonzero':
    print('failed',file=sys.stderr)
    sys.exit(7)
elif mode == 'flood':
    sys.stdout.buffer.write(b'A'*262144); sys.stdout.flush()
    sys.stderr.buffer.write(b'B'*262144); sys.stderr.flush()
elif mode == 'sleep':
    time.sleep(10)
elif mode == 'descendant':
    subprocess.Popen([sys.executable,'-c','import time; time.sleep(10)'])
elif mode == 'marker':
    open(sys.argv[2],'w').write(str(os.getpid()))
    time.sleep(10)
''', encoding='utf-8')
        prefix = '''use "std/child_process"
use "std/encoding"
use "std/result"
use "std/timers"
let arguments=Vec<str>()
arguments.push(PROGRAM)
let options=child_process.options()
'''.replace('PROGRAM', json.dumps(str(program)))
        executable = json.dumps(sys.executable)
        call = f'child_process.execFileSync({executable},arguments,options)'
        spawn = f'child_process.spawn({executable},arguments,options)'
        run(prefix + f'''arguments.push("echo")
options.input=encoding.encode("ไทย 🚀", "utf8")
let output={call}
print(output.success)
print(output.code)
print(output.timedOut)
print(output.killed)
print(encoding.decode(output.stdout,"utf8"))
print(output.stderr[1])
print(output.pid > 0)
''', 'true\n0\nfalse\nfalse\nไทย 🚀\n255\ntrue\n')
        literal = ';$(echo BAD)&"literal"'
        run(prefix + 'arguments.push("args")\narguments.push(' + json.dumps(literal) + f')\nlet output={call}\nprint(encoding.decode(output.stdout,"utf8"))', literal + '\n\n')
        run(prefix + f'''arguments.push("cwd")
options.cwd={json.dumps(str(root))}
let output={call}
print(encoding.decode(output.stdout,"utf8") == {json.dumps(str(root) + os.linesep)})
''', 'true\n')
        run(prefix + f'''arguments.push("nonzero")
let output={call}
print(output.success)
print(output.code)
print(output.timedOut)
''', 'false\n7\nfalse\n')
        run(prefix + f'''arguments.push("flood")
let output={call}
print(output.stdout.len())
print(output.stderr.len())
''', '262144\n262144\n')
        run(prefix + f'''arguments.push("echo")
options.input=encoding.encode("large input", "utf8")
let child={spawn}
print(child.pid() > 0)
let output=child.wait()
print(output.success)
print(encoding.decode(output.stdout,"utf8"))
let closed=result.run(fn() {{ child.ready() }})
print(result.isErr(closed))
''', 'true\ntrue\nlarge input\ntrue\n')
        for mode in ('sleep', 'descendant'):
            run(prefix + f'''arguments.push("{mode}")
options.timeoutMs=150
let output={call}
print(output.success)
print(output.timedOut)
''', 'false\ntrue\n', check=lambda r, duration: (_ for _ in ()).throw(AssertionError(duration)) if duration > 3 else None)
        run(prefix + f'''arguments.push("sleep")
let child={spawn}
print(child.ready())
print(child.kill())
print(child.kill())
let output=child.wait()
print(output.success)
print(output.killed)
print(output.timedOut)
''', 'false\ntrue\nfalse\nfalse\ntrue\nfalse\n')
        run(prefix + f'''arguments.push("sleep")
options.timeoutMs=100
let child={spawn}
timers.sleep(250)
print(child.ready())
print(child.wait().timedOut)
''', 'true\ntrue\n')
        run(prefix + f'''arguments.push("flood")
options.maxBuffer=1000
let outcome=result.attempt(fn() child_process.Output {{ return {call} }})
print(result.isErr(outcome))
print("after overflow")
''', 'true\nafter overflow\n')
        run(prefix + 'child_process.execFileSync("definitely-missing-executable",arguments,options)', error='child spawn failed')
        for field, value, error in [('timeoutMs', '0', 'timeoutMs'), ('timeoutMs', '300001', 'exceeds'), ('maxBuffer', '0', 'maxBuffer'), ('maxBuffer', '8388609', 'exceeds')]:
            run(prefix + f'options.{field}={value}\n{call}', error=error)
        run(prefix + f'options.cwd="missing-cwd"\n{call}', error='child spawn failed')
        run(prefix + f'arguments.push("nul\\0argument")\n{call}', error='NUL')
        if os.name == 'nt':
            run(prefix + 'child_process.spawn("test.cmd",arguments,options)', error='explicit shell')
        marker = root / 'pid.txt'

        def verify_cleanup(result, duration):
            assert duration < 3, duration
            pid = int(marker.read_text())
            if os.name == 'nt':
                import ctypes
                kernel = ctypes.WinDLL('kernel32', use_last_error=True)
                kernel.OpenProcess.restype = ctypes.c_void_p
                handle = kernel.OpenProcess(0x1000 | 0x100000, False, pid)
                if handle:
                    code = ctypes.c_ulong()
                    assert kernel.GetExitCodeProcess(ctypes.c_void_p(handle), ctypes.byref(code)) and code.value != 259, code.value
                    kernel.CloseHandle(ctypes.c_void_p(handle))
            else:
                try:
                    os.kill(pid, 0)
                except ProcessLookupError:
                    return
                raise AssertionError(f'child {pid} remains alive')

        run(prefix + f'''arguments.push("marker")
arguments.push({json.dumps(str(marker))})
let child={spawn}
timers.sleep(250)
''', '', check=verify_cleanup)
    print(f'PASS: {count} control-library executions across auto/AST; real subprocess timeout, pipes and cleanup verified')


if __name__ == '__main__':
    main()
