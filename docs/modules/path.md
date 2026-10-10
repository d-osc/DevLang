# std/path

จัดรูปแบบและประกอบ path แบบ lexical

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/path"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `path.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
struct Parts { root str, dir str, base str, ext str, name str }
fn join(base str, child str) str
fn joinMany(parts Vec<str>) str
fn resolve(base str, child str) str
fn normalize(path str) str
fn dirname(path str) str
fn basename(path str) str
fn extname(path str) str
fn isAbsolute(path str) bool
fn relative(from str, to str) str
fn parse(path str) Parts
fn format(parts Parts) str
fn sep() str
fn delimiter() str
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/path"

fn main() {
    print(path.basename(path.join("examples", "hello.dev")))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/path/main.dev
d examples/modules-api/path/main.dev --engine ast
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

## path

Lexical operations use host separators without filesystem access or symlink
resolution. They do not sandbox paths. No posix/win32 namespaces or varargs.
Windows drive-relative/case edge cases can differ from Node.

| Function | Result |
| --- | --- |
| `join(base str, child str)` | `str`, join and normalize; leading child separator does not discard base |
| `joinMany(parts Vec<str>)` | `str` |
| `resolve(base str, child str)` | `str`, absolute/cwd-relative; absolute child replaces base |
| `normalize(path str)` | `str`, empty becomes `.` |
| `dirname(path str)`, `basename(path str)`, `extname(path str)` | `str` |
| `isAbsolute(path str)` | `bool` |
| `relative(from str, to str)` | `str`, equal paths produce empty text |
| `parse(path str)` | `path.Parts { root str, dir str, base str, ext str, name str }` |
| `format(parts path.Parts)` | `str`, dir/base take precedence over root/name/ext |
| `sep()`, `delimiter()` | `str`, path separator / PATH delimiter |

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../node-core.md)
- [Source ตัวอย่าง](../../examples/modules-api/path/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
