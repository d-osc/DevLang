# std/result

จับ runtime errors และจัดการ Result<T>

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/result"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `result.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
enum Result<T> { Ok(T), Err(str) }
fn attempt<T>(body fn() T) Result<T>
fn run(body fn() void) Result<bool>
fn isOk<T>(value Result<T>) bool
fn isErr<T>(value Result<T>) bool
fn unwrap<T>(value Result<T>) T
fn unwrapOr<T>(value Result<T>, fallback T) T
fn message<T>(value Result<T>) str
fn raise(message str) void
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/result"

fn main() {
    let outcome = result.attempt(fn() i64 { result.raise("failed")
        return 0 })
    print(result.isErr(outcome))
    print(result.unwrapOr(outcome, 42))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/result/main.dev
d examples/modules-api/result/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

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

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../control-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/result/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
