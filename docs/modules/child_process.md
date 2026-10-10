# std/child_process

เริ่มและจัดการโปรแกรมภายนอกพร้อม timeout และ output

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/child_process"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `child_process.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

Handle fields เช่น id เป็น metadata ของ runtime อย่าสร้างหรือเปลี่ยนเอง ใช้ constructor function และปิด resource ตามสัญญา API

```text
struct Options { cwd str, timeoutMs i64, maxBuffer i64, input Vec<u8> }
struct Output { pid i64, code i64, success bool, timedOut bool, killed bool, stdout Vec<u8>, stderr Vec<u8> }
struct Child { id i64 }
fn options() Options
fn spawn(file str, args Vec<str>, options Options) Child
fn execFileSync(file str, args Vec<str>, options Options) Output
fn Child.pid(self Child) i64
fn Child.ready(self Child) bool
fn Child.kill(self Child) bool
fn Child.wait(self Child) Output
```

## ตัวอย่างเริ่มต้น

ตัวอย่างเริ่ม child process ของ runtime executable ปัจจุบันด้วย -e แล้วเก็บ output โดยส่ง arguments แยก ไม่ผ่าน shell

```dev
use "std/child_process"
use "std/process"
use "std/encoding"

fn main() {
    let args = Vec<str>()
    args.push("-e")
    args.push("print(42)")
    let output = child_process.execFileSync(process.execPath(), args, child_process.options())
    print(output.success)
    print(encoding.decode(output.stdout, "utf8"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/child_process/main.dev
d examples/modules-api/child_process/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

These modules run in `d file.dev` / `d run file.dev`, without a native compiler. Native builds do not yet implement these APIs.

## std/child_process

`options()` returns `Options(cwd, timeoutMs, maxBuffer, input)` with defaults `""`, 30000 ms, 1048576 bytes, and an empty `Vec<u8>`. `spawn(file, args Vec<str>, options)` returns `Child`; `execFileSync` waits and returns `Output` immediately. `Child.pid()`, `ready()`, `kill()` and `wait()` inspect, cancel and collect a process. `wait()` consumes its handle even when it reports an error.

`Output` contains `pid`, `code`, `success`, `timedOut`, `killed`, `stdout Vec<u8>` and `stderr Vec<u8>`. Nonzero exit status is an ordinary output, not an exception. A timeout kills the process group/job and sets `timedOut`; explicit cancellation sets `killed`. Decode output with `std/encoding`.

Programs are launched directly, with separate arguments and inherited environment. No shell is implicitly invoked; Windows `.bat`/`.cmd` files are rejected. Set `cwd` to change the working directory. Timeout is 1–300000 ms, input is capped at 8 MiB, and `maxBuffer` is a combined stdout/stderr cap of 1 byte–8 MiB. Overflow is a catchable error. Readers drain both pipes concurrently, and a background manager enforces deadlines even while Dev code is busy. At most 32 child handles may exist per interpreter.

Interpreter teardown cancels and joins outstanding children. Ordinary descendants belong to the managed group/job; a detached or breakaway descendant that escapes it and retains pipe handles can prevent cleanup from completing. There is currently no streaming stdio, environment override, shell `exec`, IPC, or signal-selection API.

See [the executable example](../../examples/control-libs/main.dev) and `scripts/smoke_control_libs.py` for process invocation and recovery tests.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../control-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/child_process/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
