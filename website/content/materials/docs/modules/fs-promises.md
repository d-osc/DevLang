# std/fs/promises

filesystem operations บน worker threads ที่คืน Task<T>

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/fs/promises"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `promises.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn readFile(path str, encoding str) Task<str>
fn writeFile(path str, data str) Task<bool>
fn appendFile(path str, data str) Task<bool>
fn mkdir(path str, recursive bool) Task<bool>
fn readdir(path str) Task<Vec<str>>
fn unlink(path str) Task<bool>
fn rename(from str, to str) Task<bool>
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/fs/promises"

fn main() {
    await(promises.writeFile("hello.txt", "Hello"))
    print(await(promises.readFile("hello.txt", "utf8")))
    await(promises.unlink("hello.txt"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/fs-promises/main.dev
d examples/modules-api/fs-promises/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

แต่ละคำสั่งเริ่ม worker thread และคืน Task<T> เรียก await(task) เพื่อรับผลหรือ error การ await บล็อก thread ผู้เรียก ไม่ใช่ JavaScript Promise หรือ coroutine event loop ใช้ encoding utf8/utf-8 กับ readFile; paths อิง process working directory

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
- [Source ตัวอย่าง](../../examples/modules-api/fs-promises/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
