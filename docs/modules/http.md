# std/http

HTTP/HTTPS client และ HTTP server callbacks

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/http"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `http.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct IncomingMessage { method str, url str, headers Map<str,str>, body str, bytes Vec<u8> }
struct ServerResponse { id i64, statusCode i64 }
struct Server { id i64 }
fn createServer(handler fn(IncomingMessage,ServerResponse) void) Server
fn Server.listen(self Server, port i64) void
fn Server.listenOn(self Server, port i64, host str) void
fn Server.close(self Server) void
fn Server.on(self Server, event str, handler fn(IncomingMessage,ServerResponse) void) void
fn ServerResponse.writeHead(self ServerResponse, status i64, headers Map<str,str>) void
fn ServerResponse.setHeader(self ServerResponse, name str, value str) void
fn ServerResponse.write(self ServerResponse, text str) void
fn ServerResponse.end(self ServerResponse, text str) void
struct Response { status i64, body str, bytes Vec<u8>, headers Map<str,str>, ok bool }
fn get(url str) Response
fn head(url str) Response
fn post(url str, body str) Response
fn put(url str, body str) Response
fn patch(url str, body str) Response
fn delete(url str) Response
fn request(method str, url str, headers Map<str,str>, body str, timeout_ms i64) Response
fn request_bytes(method str, url str, headers Map<str,str>, body Vec<u8>, timeout_ms i64) Response
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้ bind loopback port ที่ OS เลือก แล้วปิด handle ทันที ทดสอบการสร้าง resource โดยไม่ต้องมี server ภายนอก; ไม่ได้ทดสอบรับส่งข้อมูล

```dev
use "std/http"

fn main() {
    let server = http.createServer(fn(req http.IncomingMessage, res http.ServerResponse) {
        res.end("Hello")
    })
    server.listenOn(0, "127.0.0.1")
    server.close()
    print("server closed")
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/http/main.dev
d examples/modules-api/http/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime supports `use "std/fs"` and `use "std/http"`.
Run with `d FILE.dev` or `d run FILE.dev`. Both auto and AST engines support
these modules without a C compiler. These modules are not yet available to
`d build`; native file operations remain available through `std/io` and C FFI.
Older release installers do not include these new runtime modules.

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
produce runtime errors. Use `std/result` callbacks to recover these errors; see [control-libs.md](../control-libs.md).

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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../fs-http.md)
- [Source ตัวอย่าง](../../examples/modules-api/http/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
