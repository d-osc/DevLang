# std/fs

อ่าน/เขียนไฟล์และจัดการ directories

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/fs"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `fs.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn readFileSync(path str, encoding str) str
fn writeFileSync(path str, data str) bool
fn appendFileSync(path str, data str) bool
fn existsSync(path str) bool
fn mkdirSync(path str, recursive bool) bool
fn readdirSync(path str) Vec<str>
fn unlinkSync(path str) bool
fn rmdirSync(path str) bool
fn copyFileSync(from str, to str) bool
fn renameSync(from str, to str) bool
fn read_text(path str) str
fn write_text(path str, text str) bool
fn append_text(path str, text str) bool
fn read_bytes(path str) Vec<u8>
fn write_bytes(path str, bytes Vec<u8>) bool
fn exists(path str) bool
fn is_file(path str) bool
fn is_dir(path str) bool
fn create_dir(path str) bool
fn create_dirs(path str) bool
fn remove_file(path str) bool
fn remove_dir(path str) bool
fn read_dir(path str) Vec<str>
fn copy(from str, to str) bool
fn rename(from str, to str) bool
fn size(path str) i64
fn current_dir() str
fn join(base str, child str) str
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/fs"

fn main() {
    fs.writeFileSync("hello.txt", "Hello")
    print(fs.readFileSync("hello.txt", "utf8"))
    fs.unlinkSync("hello.txt")
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/fs/main.dev
d examples/modules-api/fs/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../fs-http.md)
- [Source ตัวอย่าง](../../examples/modules-api/fs/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
