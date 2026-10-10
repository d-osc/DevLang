# std/url

อ่าน/ประกอบ URL และ query parameters

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/url"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `url.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct URL { href str, protocol str, host str, hostname str, port str, pathname str, search str, hash str, origin str, username str, password str }
struct URLSearchParams { id i64 }
fn parse(input str) URL
fn resolve(base str, relative str) URL
fn format(value URL) str
fn URL.toString(self URL) str
fn pathToFileURL(path str) str
fn fileURLToPath(input str) str
fn domainToASCII(domain str) str
fn searchParams(query str) URLSearchParams
fn URLSearchParams.append(self URLSearchParams, name str, value str) void
fn URLSearchParams.set(self URLSearchParams, name str, value str) void
fn URLSearchParams.get(self URLSearchParams, name str) str
fn URLSearchParams.getAll(self URLSearchParams, name str) Vec<str>
fn URLSearchParams.has(self URLSearchParams, name str) bool
fn URLSearchParams.delete(self URLSearchParams, name str) void
fn URLSearchParams.toString(self URLSearchParams) str
fn URLSearchParams.close(self URLSearchParams) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/url"

fn main() {
    let params = url.searchParams("name=Dev")
    params.set("page", "1")
    print(params.get("name"))
    params.close()
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/url/main.dev
d examples/modules-api/url/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime supports `std/net`, `std/path`, `std/os`, `std/stream`,
`std/url`, `std/module`, `std/process`, `std/events`, `std/buffer`, `std/dgram`.
Run with `d FILE.dev` or `d run FILE.dev` without a C compiler. Both auto/AST
engines and editor signature checking are supported. These modules are
runtime-only and require an updated source build; older installers may lack them.

These are typed APIs inspired by Node, not JavaScript/Node compatibility.
Arguments, encodings and timeouts are explicit. Node properties such as
os.platform and module.builtinModules are functions here. Failures produce
source-located runtime errors. Recover runtime callback errors with `std/result`; JavaScript Promises are not implemented.

## Resource ownership and callbacks

Buffers, emitters, query parameters, streams and sockets have runtime-owned
handles. Close them explicitly. Handles belong to their interpreter thread;
capturing them in another task produces an error. Create resources inside the
worker that uses them. At most 256 core handles exist per interpreter, separate
from HTTP handles. Do not change handle IDs or snapshot metadata.

TCP/UDP and HTTP callbacks are serviced after main statements finish successfully.
Listeners and sockets with receive callbacks keep the runtime alive; close them
to finish. Callbacks run serially and blocking work delays other callbacks.
A blocking read in main from a server in the same runtime prevents the server
from servicing it: use callbacks in this case. Writes are synchronous and do not
expose Node backpressure, timers, error events or Promise scheduling.

## url

`parse(input str)` and `resolve(base str, relative str)` return url.URL using
WHATWG parsing. Fields (all str): href, protocol, host, hostname, port, pathname,
search, hash, origin, username, password. Protocol includes `:`, nonempty
search/hash include `?`/`#`, absent port is empty. Fields are record snapshots,
not live URL setters. `URL.toString()` returns canonical href.
`format(value url.URL) str` rebuilds a hierarchical URL from protocol, hostname,
port, pathname, search, hash and credentials, using href as its initial parser
base; it ignores snapshot host/origin. Opaque URLs may not support format.

`pathToFileURL(path str) str` accepts absolute/cwd-relative paths;
`fileURLToPath(input str) str` returns a local path. Spaces, Unicode and fragment
characters are escaped. `domainToASCII(domain str) str` performs IDN/punycode
normalization and rejects URL syntax.

`searchParams(query str)` returns url.URLSearchParams:

| Method | Result |
| --- | --- |
| `append(name str, value str)`, `set(name str, value str)` | `void` |
| `get(name str)` | `str`, first value; missing is empty, use has to distinguish |
| `getAll(name str)` | `Vec<str>` |
| `has(name str)` | `bool` |
| `delete(name str)` | `void`, deletes all matching pairs |
| `toString()` | `str`, form encoding, spaces become + |
| `close()` | `void` |

Order/repeated keys are preserved; set replaces first and removes duplicates.
Limits: 4096 pairs, 8 MiB decoded key/value storage. No iterators, sorting,
live URL-search binding or domainToUnicode.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../node-core.md)
- [Source ตัวอย่าง](../../examples/modules-api/url/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
