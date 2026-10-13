# Interpreter API

See [the per-module API index](modules/index.md) for all 42 builtins and their
individual signatures, examples and runtime/native contracts.

`std/dns` provides OS host resolution; `std/cli` parses schema-defined program
arguments. See [DNS and CLI contracts](system-libs.md).

Data libraries `std/regex`, `std/encoding`, `std/crypto`, `std/compression`,
`std/archive`, `std/uuid` are also available. See [API contracts](data-libs.md).

Foundational `std/math`, `std/random`, `std/datetime`, `std/test`, `std/log` and
expanded `std/strings` are available in the current runtime. See
[Foundational library APIs](basic-libs.md) for contracts and examples.

`std/don` reads and writes Dev Object Notation (`.don`), a JSON-compatible format
with comments, bare keys and newline separators. See [DON](don.md).

Node-style core modules are also available under `std/`: `net`, `path`, `os`,
`stream`, `url`, `module`, `process`, `events`, `buffer`, `dgram`.
See [Core runtime APIs and contracts](node-core.md).

Filesystem and HTTP/HTTPS clients are available through `std/fs` and `std/http`
in the current source runtime. See [Filesystem and HTTP](fs-http.md) for APIs,
examples, errors and limits. These modules currently support runtime execution only.

JSON is available in the current source runtime through `use "std/json"`.
See [JSON API and contracts](json.md) and `examples/json/main.dev` for parsing,
serialization, typed access, objects/arrays and file I/O. Build the updated runtime
from source; the existing v0.4.0 release binaries do not contain this addition.

Run `devrun FILE.dev [-- arguments]` or `devrun -e 'statements'`.
`--eval` is an alias for `-e`; `--timings` prints load/execution times to stderr.
`--engine auto` (default) uses numeric plans where supported; `--engine ast`
selects the reference AST interpreter. With `d`, put this option after the source filename.
`-h`/`--help` and `-v`/`-V`/`--version` show help and version.
Runtime errors include source path, line and column. File-level statements execute
in order; declarations never invoke `main` automatically. Write `main()` explicitly
to call it. `main` defaults to return type `i32`, with a fallthrough value of zero.
Other non-void functions must return a value. A file-level `return` sets the process
exit code; `return main()` propagates the function's result. A bare `main()` ignores
that result, like any expression statement. Imported files contain declarations only.

Use `use "std/io"` (or `as alias`). Built-in modules need no files or libraries:

| Module | Functions |
| --- | --- |
| `std/io` | `write(str) bool`, `writeln(str) bool`, `read_line() str`, `read_file(path str) str`, `write_file(path str, text str) bool` |
| `std/strings` | `len(str) usize`, `concat(str, str) str`, `equal(str, str) bool` |
| `std/time` | `now_ns() u64`, `now_ms() u64`, `sleep_ms(integer) bool` |
| `std/args` | `len() usize`, `get(integer) str` |

I/O failures produce runtime errors. Text files and strings use UTF-8. String
length and indexing count bytes; indexing produces `u8`. `read_line` removes
the final newline. Arguments include only values following `--`, without the
source filename. Time measures elapsed monotonic time since runtime startup.

`print(value)` writes a line; string `+` concatenates. Fixed-width integer
arithmetic wraps, division by zero and invalid shifts produce errors. Locals
and parameters enforce declared types. Explicit numeric casts use `as`.
Float-to-integer casts truncate toward zero and reject NaN, infinity and
out-of-range truncated values, matching native execution. Integer casts retain
wrapping behavior.
Boolean operators short-circuit. Array indices are checked. Unexecuted code is
not type-checked: use `devc check` for native programs, noting that runtime
intrinsics and the native stdlib have different APIs.

The runtime evaluates Dev source without C translation or native code generation.
Only automatically prepared C dependencies use a native-library cache. Networking
modules and thread-backed tasks are supported; there is no JIT or full JavaScript compatibility.

## Native C FFI

`extern fn` uses the platform's default C ABI. `--ffi-lib PATH` loads an existing
shared library (.dll on Windows, .so on Linux), and is repeatable. Put the option
after the entry filename when using `d`. Paths resolve from the working directory.
Explicit libraries are searched in command-line order, followed by system CRTs.
If a symbol remains unresolved, the runtime discovers shared libraries beside
the source files containing `extern fn` declarations. This search is nonrecursive,
uses only library extensions for the current OS, and sorts paths deterministically.
Each canonical library path is attempted once. Imported modules contribute their
own directories; unrelated working directories are not searched. Library loading
can run native initialization code. A missing symbol reports adjacent load errors.
Standard symbols such as `puts`, `malloc` and `free` need no explicit library.
Function pointers and call interfaces are cached and the loaded libraries remain
alive throughout execution.

If no library supplies the symbol, adjacent C sources containing that identifier
trigger automatic preparation through sibling `devc native-build`. This compiles
all `.c` files in that declaration module's directory into a shared library;
Dev source remains interpreted. The first run needs Clang/GCC or `DEV_CC`.
The compiler helper stays in `compiler/`; the interpreter has no compiler-crate
dependency. Cache keys include platform, directory, declared symbols, backend
selection, C contents and recursively collected local `.h` contents. Cached
library bytes are checked before reuse, and warm runs need no C backend.
Preparation failures preserve previous successful cache entries.
Cache files are in `.dev-cache/native/`. External system-header/toolchain changes
require removing this directory. Only adjacent C files are compiled, with up to
512 tracked files and 8 MiB per file; custom include/link flags are not supported
by this automatic path. Use a prebuilt library for more complex native projects.
Explicit and adjacent prebuilt libraries take precedence, so remove stale
prebuilt libraries when switching to C source preparation.

```dev
extern fn puts(message: str): i32
extern fn device_add(a: i32, b: i32): i32

fn main() {
    puts("Calling C directly")
    print(device_add(19, 23))
}

main()
```

```powershell
target\release\d.exe examples\ffi\main.dev
```

This command prepares the C dependency automatically on the first run and
reuses it thereafter. Existing compatible shared libraries need no preparation. Supported ABI types:
`i8/i16/i32/i64`, `u8/u16/u32/u64`, `isize/usize`, `bool` (C `_Bool`), `f32/f64`,
`str`, pointers, C-compatible structs, payload-free enums and `void` returns. Variadic calls apply C default argument promotions; scalar callbacks with captured values use libffi closures. Payload unions, top-level by-value arrays and custom calling conventions require C wrappers.

`str` arguments are temporary, read-only NUL-terminated UTF-8 buffers valid only
for the duration of the call. Embedded NUL bytes are rejected. Native code must
not retain or free them. A returned `str` must point to readable, NUL-terminated
UTF-8 data and is copied before argument buffers are released. A null or invalid
UTF-8 string result is an error. Use `*u8` for nullable string pointers.

Live foreign pointers support unsafe scalar/struct/array reads and writes,
field mutation, pointer arithmetic and numeric/bool volatile access. Null,
alignment and address overflow are checked in runtime access; allocation bounds
and foreign-memory lifetimes are the caller's responsibility. Address-of
interpreter locals remains unsupported. Use the allocating library's free function.

Native library architecture, signatures, calling convention and pointer validity
must match declarations; an incorrect native contract can crash the process.

System CRT streams are flushed after native calls to preserve output order with
runtime I/O. A DLL with its own statically linked CRT must flush its private
streams itself when mixing output with the interpreter.

Missing libraries/symbols, unsupported types, conflicting extern signatures,
argument count/type errors and invalid string results produce runtime errors.

## Numeric execution plans

Functions consisting of supported numeric/bool operations, numeric arrays,
control flow and calls are lowered once during module load to typed register
instructions. Entry scripts can use the same path. Plans stay in memory and
never invoke `devc`, emit C, generate native code or create cache files.
Constants and variable names are resolved outside compute loops. An unsupported
or inconsistent function uses the AST interpreter; fallback functions may call
planned numeric functions. Error checks and source locations remain active.
Array values still copy at assignment/call boundaries, while indexed reads and
writes access individual elements directly. Workspaces are reused across calls;
recursive invocations take independent frames, with at most one cached frame
per function. No SIMD, automatic parallel loops or native JIT is provided. Explicit `spawn`, `Task<T>`, `await(task)`, nonblocking `ready(task)` and `async fn` use OS threads with independent interpreter frames and copy-on-write snapshots.

Use `examples/runtime/compute.dev` for a 100-million-iteration source example.
Measured before/after results and reproduction commands are in
[runtime-compute.md](runtime-compute.md).

## Shared language features

[std/sync](sync.md) provides bounded channels, atomic mutex updates and cooperative
cancellation shared across workers. These primitives use OS threads and blocking
waits; they do not change async functions into coroutines.

Runtime libraries include [error recovery, timers and child processes](control-libs.md),
[SQLite, CSV, TOML and YAML](storage-libs.md), and [TLS/WS/WSS](secure-network.md).
Use `std/result` to recover runtime callback errors. These additions are separate
from the native stdlib; networking remains synchronous within callbacks.

Range `for`, nominal value structs, payload enums with exhaustive `match` and inferred/explicit generics use
the shared frontend described in [language.md](language.md). Type definitions
and specialization arguments are validated during load. Supported numerical
generic functions and range loops retain numeric plans; struct/enum-heavy
functions use AST fallback. No compiler executable is required for these pure
Dev features. Field and array-element mutation checks indices and declared
member types, and struct copies do not alias mutable nested arrays.

Collections, function values, closures, traits, const generics and thread-backed tasks are documented in [advanced.md](advanced.md). Pure Dev execution continues to require no compiler.

`then(task, next)` returns a continuation task without waiting in its caller; its worker still waits on an OS thread. `callback_context(f)` exposes a managed C userdata pair, with userdata last. Captured values are released with the owner; runtime weak owners diagnose expired context invocations. See [advanced.md](advanced.md) for trampoline storage lifetime and unsafe C contracts.

## Prototype cycle collection / การเก็บวงจร object

Source runtime collects unreachable prototype cycles through arrays, maps, references and closures. Automatic passes run at block boundaries after 256 new prototype allocations. `GC.collect(): usize` counts cleared prototype states, `GC.tracked()` counts registered live states and `GC.collections()` counts passes. Concurrent collection is best effort and may return zero when busy. Native prototype allocations still live until exit; cycles hidden in foreign resources or Task internals remain conservative roots.

Runtime เก็บวงจร object ที่ไม่มีผู้ใช้งานได้แล้ว หรือเรียก GC.collect() เองได้ GC ใช้เฉพาะ source runtime ส่วน native ยังใช้ arena จนโปรแกรมจบ

## Filesystem promise scheduling / การจัดคิว filesystem

`std/fs/promises` runs blocking disk operations on 1–4 shared workers, with 64 queued jobs and a 64 MiB pending argument budget. Overflow returns a failed Task; await pending tasks before scheduling more. Repeated await returns the cached result. Returned file contents remain owned by callers. Ordinary async functions/spawn still use OS threads; this is not coroutine or nonblocking network I/O.

fs/promises จำกัดจำนวน worker และขนาดคิว หากเต็ม Task จะคืน error ตอน await จึงควรรองานเดิมก่อนส่งเพิ่ม

[Concurrent cycle collection example / ตัวอย่าง GC หลาย threads](#/examples/prototype-gc)

## Interactive runtime debugger

```sh
d run examples/hello.dev --debug-runtime
# Direct runtime executable:
devrun examples/hello.dev --debug-runtime
```

Execution pauses before the first executable statement. Debugger messages and
prompts use stderr; program output stays on stdout. Commands:

| Command | Behavior |
| --- | --- |
| `step` / `s` | Execute one statement, entering source functions |
| `next` / `n` | Continue to the next statement at the same or lower call depth |
| `finish` | Continue until the current source function returns |
| `continue` / `c` | Run until a breakpoint or program completion |
| `break 5` | Break before statements on line 5 of the current file |
| `break /path/lib.dev:5` | Break in another loaded source file; Windows paths work too |
| `breakpoints` | List breakpoints |
| `clear` | Remove all breakpoints |
| `locals` | Show visible local variables, respecting scope shadowing |
| `stack` | Show source function names and their last statement locations |
| `quit` / `q` | Stop execution with exit code 0 |
| `help` | Show available commands |

The debugger always uses the AST interpreter, including when `--engine auto`
is supplied. Stops happen before statements and every loop condition check;
several statements on one line can produce several stops. Scalar/string values
are shown directly (long strings are truncated); collections and objects show
a type summary without evaluating getters. Breakpoint files must already be
loaded; a line without an executable statement will not stop.

This debugger covers the main interpreter thread. Native FFI, numeric plans,
worker tasks and VS Code/DAP integration are outside its scope. Debugger commands
share stdin with the program, so interactive programs reading stdin should use
separate test inputs with care. EOF ends the debugging session with exit code 0.
Program errors retain their normal error exit code.

ภาษาไทย: ใช้ `d run FILE.dev --debug-runtime` เพื่อหยุดก่อนคำสั่ง แล้วใช้
`step` เข้าไปในฟังก์ชัน, `next` ข้ามการเรียกฟังก์ชัน, `finish` รอจนฟังก์ชันคืนค่า,
`locals` ดูตัวแปร และ `stack` ดูลำดับการเรียก ใช้ `break LINE` หรือ
`break FILE:LINE` ตั้ง breakpoint และ `continue` รันต่อ ระบบนี้ใช้ AST
และตรวจเฉพาะเธรดหลัก ยังไม่ใช่ debugger ผ่าน VS Code และใช้ stdin ร่วมกับโปรแกรม

## Long-running HTTP server qualification

HTTP handler errors are isolated per request: an unsent response becomes 500,
its handle is removed, and later requests continue. Errors after `end` are logged
without a second response. A missing `end` also produces 500. Use stderr logging
with external rotation/retention limits. `server.close()` lets the current callback
finish and exits when no other listeners/timers remain.

```sh
python scripts/soak_http_server.py --bin-dir target/release --requests 20000 --concurrency 8 --report out/http-soak.json
# Extended run: minimum request count plus minimum duration for EACH engine:
python scripts/soak_http_server.py --bin-dir target/release --requests 20000 --duration-seconds 3600 --report out/http-hour.json
```

The test issues real local HTTP requests, injects handler/header failures, allocates
cyclic objects per request, checks a subsequent healthy response, samples process
RSS after warm-up, and checks callback-driven shutdown. It currently supports
Windows and Linux RSS sampling. The default RSS-growth ceiling is 32 MiB;
`--max-growth-mib` sets an explicit workload budget. Sampled RSS is not peak
allocation tracking. The load uses small bodies and bounded client concurrency;
it does not qualify large uploads, TLS, public-client attacks or arbitrary code.

For deployment, bind DevLang to `127.0.0.1`, keep the backend inaccessible to
external clients, and put a proxy in front. The template
[`deploy/http/nginx.conf`](../deploy/http/nginx.conf) buffers request bodies,
limits body size and active requests and configures read/send timeouts. Its
loopback port 8080 expects a separate TLS edge. See [proxy buffering](https://nginx.org/en/docs/http/ngx_http_proxy_module.html)
and [request limits/timeouts](https://nginx.org/en/docs/http/ngx_http_core_module.html).
Timeouts apply between reads/writes rather than providing a total application
execution deadline. An infinite or blocking handler cannot be interrupted by the
proxy; use external health checks and process supervision to restart a stalled
worker. Application state/durable writes must tolerate restarts.

The interpreter serves callbacks serially. Direct public exposure of its initial
HTTP transport is not qualified: ingress connection/header deadlines and transport
queue limits are not configurable in DevLang. Native compilation does not provide
this HTTP server. Production approval still requires application-specific
hours-long load/soak results, authentication and durable-state recovery checks.

ภาษาไทย: handler ที่ผิดพลาดไม่ทำให้ server ทั้งตัวหยุดแล้ว และมีการทดสอบ HTTP
พร้อมวัด RSS แต่ production ต้อง bind loopback ผ่าน reverse proxy มี health check
และ supervisor รวมถึงทดสอบต่อเนื่องตามงานจริงก่อนเปิดรับผู้ใช้ภายนอก

### HTTP ingress regression

## Live ingress verification

```sh
python scripts/smoke_http_ingress.py --bin-dir target/release --nginx nginx
```

The test starts the shipped profile on temporary loopback ports against an actual
DevLang interpreter, checks handler-error recovery, rejects bodies above 64 KiB
and oversized headers, checks slow body/header deadlines, and fills all 16 active
request slots to verify 503 load shedding. It then releases those slots and checks
recovery. Denied and partially uploaded requests must never appear in the backend
request log. Each test owns its processes and shuts down its own NGINX instance.
CI runs this on Linux; local Windows validation also uses the actual NGINX binary.

A timeout may close the connection without an HTTP error page. Body timeouts are
inactivity deadlines: continuously trickling data can keep a request alive.
This test does not qualify a TLS edge, enforce a total upload duration or test all
request-smuggling variants. Keep public ingress protection and application
qualification separate from these local transport regressions.

ภาษาไทย: มีการทดสอบ NGINX กับ DevLang จริงแล้ว รวม body/header เกินขนาด,
การส่งค้าง, เพดาน active requests และการกลับมารับงานหลัง overload แต่ยังต้อง
ทดสอบ TLS edge และ app จริงแยกต่างหาก timeout ของ body เป็น inactivity timeout
ไม่ใช่เพดานเวลารวมของการ upload

## HTTP worker supervisor

Python 3 on Windows/Linux can supervise a source worker without changing the
language/runtime separation:

```sh
python scripts/serve_http.py --runtime target/release/devrun --file examples/production-http/main.dev --health http://127.0.0.1:3000/healthz --max-rss-mib 256 --log out/http-worker.log
```

On Windows use `target/release/devrun.exe`. Use an absolute runtime path if the
executable is elsewhere. The worker's working directory is the source directory.
Keep the backend loopback-only and define a small `/healthz` route returning HTTP
200 with a finite body (at most 4096 bytes). Probes do not follow redirects.

Defaults: 15-second startup grace, 2-second polling interval, 1-second health
socket timeout, restart after 3 failed probes, and at most 5 restarts in a rolling
60-second window with capped exponential backoff. Unexpected worker exits,
including exit 0, are restarted: stop the supervisor to stop the service.
Exhausting the budget returns exit 1 for monitoring. Ctrl+C/SIGTERM stops the
worker; Windows also supports CTRL_BREAK. `--run-seconds` limits a qualification
run and exits 0 on normal stop. A supervisor launch/configuration error exits
nonzero instead of silently retrying a broken installation.

Worker stdout/stderr are drained in bounded chunks and rotated into one log with
three backups (`--log-bytes`, default 1 MiB per file; a final chunk may overshoot
by about 4 KiB). Supervisor events also appear on stdout. Monitor these events and
retain logs appropriately. A sampled RSS budget triggers restart, but it is not
a hard OS memory cap and can miss short allocation spikes; use OS resource limits
as well. Worker restarts interrupt in-flight requests and lose in-memory state.
Store durable data externally and use idempotent retry policies where applicable.

The health route must represent worker readiness. Long legitimate callbacks may
fail a short health budget because handlers are serial; tune budgets from measured
workloads. The supervisor does not repair application/database faults or manage
the external NGINX/TLS edge. Run it under a service manager for host boot/recovery.

```sh
python scripts/smoke_http_supervisor.py --bin-dir target/release
```

This regression creates a real DevLang worker, hangs its callback, closes its
listener and verifies recovery, checks restart exhaustion/RSS budgets, validates
log rotation and clean shutdown. CI executes it on Windows/Linux.

ภาษาไทย: supervisor มี health check, RSS budget, restart/backoff แบบจำกัดจำนวน
และ log rotation แต่การ restart ทำให้ request ที่กำลังทำงานถูกตัดและ state
ใน RAM หาย จึงต้องจัดการข้อมูลถาวรและ retry ของ app ให้เหมาะสม

The bundled `examples/production-http/main.dev` is a minimal loopback starter with
GET `/healthz`, GET `/`, 404 and 405 responses. Its health route checks listener
responsiveness only; extend readiness checks for application dependencies. It is
not a business application with authentication or durable storage.
