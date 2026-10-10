# Error recovery, timers and child processes

These modules run in `d file.dev` / `d run file.dev`, without a native compiler. Native builds do not yet implement these APIs.

## std/result

`Result<T>` has `Ok(T)` and `Err(str)` payloads. `attempt<T>(fn() T)` catches runtime errors from the callback; `run(fn() void)` returns `Result<bool>` with `Ok(true)` on success. Inspect with `isOk`, `isErr`, `message`, pattern matching, or `unwrapOr(value, fallback)`. `unwrap` returns the payload or raises the stored error. `raise(message)` raises a catchable error.

```dev
use "std/result"
fn main() {
    let outcome = result.attempt(fn() i64 { return 6 * 7 })
    match outcome {
        Ok(value) => { print(value) }
        Err(message) => { print(message) }
    }
}
main()
```

Fallback arguments are evaluated eagerly. Recovery does not undo side effects or roll back database transactions. Source loading, parsing/type checking, process crashes, Rust panics and foreign memory faults are outside this mechanism.

## std/timers

`setTimeout(fn(Timer) void, delayMs)`, `setInterval(fn(Timer) void, delayMs)` and `setImmediate(fn(Timer) void)` return a `Timer`. Unlike Node's timer callback, the callback receives its handle. `Timer.ticks()` counts invocations; `cancel()` returns whether an active timer was removed. `clearTimeout` and `clearInterval` also cancel. `ref`, `unref` and `hasRef` control whether the timer keeps the event pump alive.

Timers run after a successful entry call, or when `timers.run()` explicitly pumps timers and network servers. Delays use a monotonic clock. Timeouts allow 0–300000 ms; intervals allow 1–300000 ms. There are at most 64 active timers per interpreter. Interval delays start after each callback finishes; missed intervals do not accumulate. Callbacks execute sequentially on the interpreter thread. A failed callback removes that timer and propagates its error. Nested event pumps are rejected.

```dev
use "std/timers"
fn main() {
    timers.setInterval(fn(t timers.Timer) {
        print(t.ticks())
        if t.ticks() >= 3 { t.cancel() }
    }, 1)
}
main()
```

Unreferenced timers may run while another referenced timer/server keeps the pump alive. Worker tasks need an explicit pump. `sleep(ms)` blocks the current thread; these APIs are not coroutine-based async I/O. Closures capture values, rather than sharing mutable local counters automatically.

## std/child_process

`options()` returns `Options(cwd, timeoutMs, maxBuffer, input)` with defaults `""`, 30000 ms, 1048576 bytes, and an empty `Vec<u8>`. `spawn(file, args Vec<str>, options)` returns `Child`; `execFileSync` waits and returns `Output` immediately. `Child.pid()`, `ready()`, `kill()` and `wait()` inspect, cancel and collect a process. `wait()` consumes its handle even when it reports an error.

`Output` contains `pid`, `code`, `success`, `timedOut`, `killed`, `stdout Vec<u8>` and `stderr Vec<u8>`. Nonzero exit status is an ordinary output, not an exception. A timeout kills the process group/job and sets `timedOut`; explicit cancellation sets `killed`. Decode output with `std/encoding`.

Programs are launched directly, with separate arguments and inherited environment. No shell is implicitly invoked; Windows `.bat`/`.cmd` files are rejected. Set `cwd` to change the working directory. Timeout is 1–300000 ms, input is capped at 8 MiB, and `maxBuffer` is a combined stdout/stderr cap of 1 byte–8 MiB. Overflow is a catchable error. Readers drain both pipes concurrently, and a background manager enforces deadlines even while Dev code is busy. At most 32 child handles may exist per interpreter.

Interpreter teardown cancels and joins outstanding children. Ordinary descendants belong to the managed group/job; a detached or breakaway descendant that escapes it and retains pipe handles can prevent cleanup from completing. There is currently no streaming stdio, environment override, shell `exec`, IPC, or signal-selection API.

See [the executable example](../examples/control-libs/main.dev) and `scripts/smoke_control_libs.py` for process invocation and recovery tests.
