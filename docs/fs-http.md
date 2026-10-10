# Filesystem and HTTP client

The current source runtime supports `use "std/fs"` and `use "std/http"`.
Run with `d FILE.dev` or `d run FILE.dev`. Both auto and AST engines support
these modules without a C compiler. These modules are not yet available to
`d build`; native file operations remain available through `std/io` and C FFI.
Older release installers do not include these new runtime modules.

## Node-style filesystem API

These APIs follow Node's naming, with explicit typed arguments in DevLang.
They are a subset, not a Node.js compatibility runtime.

| Synchronous API | Arguments / result |
| --- | --- |
| `fs.readFileSync(path str, encoding str)` | `str`; encoding must be `utf8` or `utf-8` |
| `fs.writeFileSync(path str, data str)` | `bool`, create/truncate |
| `fs.appendFileSync(path str, data str)` | `bool`, create/append |
| `fs.existsSync(path str)` | `bool` |
| `fs.mkdirSync(path str, recursive bool)` | `bool`; pass true for parent directories |
| `fs.readdirSync(path str)` | `Vec<str>`, sorted names |
| `fs.unlinkSync(path str)`, `fs.rmdirSync(path str)` | `bool`; directory must be empty |
| `fs.copyFileSync(from str, to str)`, `fs.renameSync(from str, to str)` | `bool` |

Import `std/fs/promises` to use `promises.readFile(path, encoding)`,
`writeFile(path, data)`, `appendFile(path, data)`, `mkdir(path, recursive)`,
`readdir(path)`, `unlink(path)` and `rename(from, to)`. Each starts a worker
thread and returns `Task<T>`. Use `await(task)` to observe the result or error.
Awaiting blocks the calling thread; this is not a JavaScript Promise/event loop.
There are no optional arguments, callback filesystem APIs, Buffer, streaming,
file descriptors or file watchers in this subset. Binary APIs remain
`fs.read_bytes` and `fs.write_bytes` with `Vec<u8>`.

```dev
use "std/fs"
use "std/fs/promises"
fn main() {
    fs.writeFileSync("hello.txt", "Hello")
    print(fs.readFileSync("hello.txt", "utf8"))
    print(await(promises.readFile("hello.txt", "utf8")))
    await(promises.unlink("hello.txt"))
}
main()
```

## Original filesystem API

Paths and text use UTF-8. Relative paths resolve against the process working
directory, not the source file directory. Operations have normal OS permissions.

| Function | Result |
| --- | --- |
| `read_text(path str)` | `str`, requires valid UTF-8 |
| `write_text(path str, text str)` | `bool`, creates or truncates |
| `append_text(path str, text str)` | `bool`, creates or appends |
| `read_bytes(path str)` | `Vec<u8>` |
| `write_bytes(path str, bytes Vec<u8>)` | `bool` |
| `exists(path str)`, `is_file(path str)`, `is_dir(path str)` | `bool` |
| `create_dir(path str)`, `create_dirs(path str)` | `bool`, single or recursive creation |
| `remove_file(path str)`, `remove_dir(path str)` | `bool`, directory must be empty |
| `read_dir(path str)` | `Vec<str>`, sorted entry names |
| `copy(from str, to str)`, `rename(from str, to str)` | `bool`, OS overwrite semantics |
| `size(path str)` | `i64`, bytes |
| `current_dir()` | `str` |
| `join(base str, child str)` | `str`, OS path joining |

```dev
use "std/fs"
fn main() {
    fs.write_text("hello.txt", "Hello DevLang")
    print(fs.read_text("hello.txt"))
    fs.remove_file("hello.txt")
}
main()
```

Buffered reads and writes have an 8 MiB limit. I/O failures produce located
runtime errors; successful mutations return true. There is no recursive delete.
An absolute child in `join` may replace the base; joining does not sandbox paths.

## HTTP/HTTPS client

```dev
use "std/http"
fn main() {
    let response = http.get("https://example.com/")
    print(response.status)
    print(response.ok)
    print(response.body)
}
main()
```

| Function | Arguments |
| --- | --- |
| `get`, `head`, `delete` | `(url str)` |
| `post`, `put`, `patch` | `(url str, body str)` |
| `request` | `(method str, url str, headers Map<str,str>, body str, timeout_ms i64)` |
| `request_bytes` | `(method str, url str, headers Map<str,str>, body Vec<u8>, timeout_ms i64)` |

All return `http.Response`: `status i64`, `ok bool`, `body str`,
`bytes Vec<u8>` and `headers Map<str,str>`. `ok` means status 200 through 299.
HTTP 4xx/5xx return a response; connection, TLS, timeout and body-limit failures
produce runtime errors. Use `std/result` callbacks to recover these errors; see [control-libs.md](control-libs.md).

Headers use lowercase keys in responses; repeated values are joined by newline.
`body` decodes UTF-8 with replacement for invalid bytes; use `bytes` for exact
binary data. HEAD has an empty body. Request methods include GET, HEAD, POST,
PUT, PATCH, DELETE and OPTIONS. Explicit timeouts range from 1 to 300000 ms.
Default timeout is 10000 ms; at most five redirects are followed.
HTTPS verifies server certificates. Gzip responses are decoded automatically.
Request and decoded response bodies have an 8 MiB limit.

```dev
use "std/http"
fn main() {
    let headers = Map<str,str>()
    headers.set("Content-Type", "application/json")
    let response = http.request("POST", "http://localhost:8080/users",
        headers, "{\"name\":\"Dev\"}", 5000)
    print(response.status)
    print(response.body)
}
main()
```

Client requests block the calling thread. The client `get`/`request` APIs retain
their original signatures; Node's ClientRequest/event callbacks are not implemented.
Streaming, multipart upload and an async I/O event loop are not implemented.

## Node-style HTTP server

```dev
use "std/http"
fn main() {
    let server = http.createServer(fn(req http.IncomingMessage, res http.ServerResponse) {
        res.setHeader("Content-Type", "text/plain; charset=utf-8")
        if req.url == "/" {
            res.end("Hello DevLang!\n")
        } else {
            res.statusCode = 404
            res.end("Not found\n")
        }
    })
    server.listen(3000)
    print("Listening on http://localhost:3000")
}
main()
```

`http.createServer(handler fn(http.IncomingMessage,http.ServerResponse) void)`
returns `http.Server`. `server.listen(port i64)` binds all IPv4 interfaces;
`server.listenOn(port i64, host str)` selects a bind address. A port of zero
allows the OS to select a port, though no address-query API exists yet.
`listen` returns after binding; the runtime then services listening servers after
the file's statements finish successfully. It stays alive until all listeners are
closed or the process is stopped. `server.close()` removes the listener and lets
the active callback finish. `server.on("request", handler)` replaces the handler;
other events are not implemented. Handles belong to the interpreter thread that
created them; do not pass them into spawned tasks.

IncomingMessage exposes `method str`, `url str` (path plus query),
`headers Map<str,str>` (lowercase keys), `body str` (lossy UTF-8), and
`bytes Vec<u8>`. The complete request body is buffered before callback invocation.

ServerResponse supports `setHeader(name str, value str)`,
`writeHead(status i64, headers Map<str,str>)`, `write(text str)` and `end(text str)`.
Use `end("")` to finish without extra text. `write` buffers until `end`; this is
not a streaming response. `res.statusCode = 404` sets the status on the next
response-method call. Status codes must be 200 through 599. Since DevLang uses
value receivers, `writeHead` does not update the local `res.statusCode` field;
prefer `writeHead` for explicit status/headers. Response framing headers are
managed by the runtime; explicit Content-Length/Transfer-Encoding are rejected.
Invalid header names/values and writes after `end` produce errors.

Handlers must call `end` before returning. Missing `end` produces HTTP 500.
A handler error produces HTTP 500 if the response has not been sent, then aborts
the runtime with a located error. Bodies are limited to 8 MiB, and at most 64
server handles may exist in an interpreter. Callbacks are served serially, so
blocking work or slow body reads delay other requests. This initial HTTP/1.x
server does not provide configurable read/header timeouts, HTTPS serving,
WebSockets, HTTP/2 or a general Node event loop; use a reverse proxy when needed.

See `examples/node-io/main.dev` and
`d examples/http-server/main.dev -- serve` (port 3000), with real server coverage
in `python scripts/smoke_node_io.py --bin-dir target/release`.
See `examples/filesystem/main.dev`, `examples/http/main.dev` and
`python scripts/smoke_fs_http.py --bin-dir target/release` for runnable coverage.
