# Filesystem and HTTP client

The current source runtime supports `use "std/fs"` and `use "std/http"`.
Run with `d FILE.dev` or `d run FILE.dev`. Both auto and AST engines support
these modules without a C compiler. These modules are not yet available to
`d build`; native file operations remain available through `std/io` and C FFI.
Older release installers do not include these new runtime modules.

## Filesystem

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
produce runtime errors. The language currently has no catch API for these errors.

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

Requests block the calling thread. This is a client API; HTTP servers, streaming,
multipart upload and an async I/O event loop are not implemented.
See `examples/filesystem/main.dev`, `examples/http/main.dev` and
`python scripts/smoke_fs_http.py --bin-dir target/release` for runnable coverage.
