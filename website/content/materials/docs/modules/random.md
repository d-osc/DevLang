# std/random

สุ่มแบบทำซ้ำได้ด้วย seed สำหรับ simulation และการทดสอบ

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/random"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `random.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn seed(value i64) void
fn float() f64
fn int(min i64, max i64) i64
fn bool() bool
fn bytes(size i64) Vec<u8>
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/random"

fn main() {
    random.seed(42)
    let value = random.int(0, 10)
    print(value >= 0 && value < 10)
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/random/main.dev
d examples/modules-api/random/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime adds `std/math`, `std/random`, `std/datetime`,
`std/test`, `std/log`, and expands `std/strings`. Import with `use "std/math"`.
These APIs run without a C compiler. They are not yet provided by native stdlib
or existing release installers. Shared signatures let the editor check calls.
Arguments are required; there are no optional/variadic arguments.

## Random

| API | Meaning |
| --- | --- |
| `seed(value i64)` | Reset the interpreter's SplitMix64 state |
| `float() f64` | Uniform 53-bit fraction in `[0,1)` |
| `int(min i64,max i64) i64` | Uniform integer, minimum included, maximum excluded |
| `bool() bool` | Random boolean |
| `bytes(size i64) Vec<u8>` | PRNG bytes, up to 8 MiB |

Without an explicit seed, the OS supplies the initial seed. Equal seeds and equal
call sequences produce repeatable results. Each task interpreter has separate
state. Integer generation uses rejection sampling to avoid modulo bias.
These APIs are **not cryptographically secure**; do not use them for passwords,
tokens, keys or security nonces. Use std/crypto.secureBytes for cryptographic random bytes.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../basic-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/random/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
