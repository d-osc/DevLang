# std/math

คำนวณ f64, ราก, logarithm, ตรีโกณมิติ และปัดเศษ

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/math"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `math.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn pi() f64
fn e() f64
fn tau() f64
fn abs(x f64) f64
fn sqrt(x f64) f64
fn cbrt(x f64) f64
fn pow(x f64, y f64) f64
fn exp(x f64) f64
fn log(x f64) f64
fn log2(x f64) f64
fn log10(x f64) f64
fn sin(x f64) f64
fn cos(x f64) f64
fn tan(x f64) f64
fn asin(x f64) f64
fn acos(x f64) f64
fn atan(x f64) f64
fn atan2(y f64, x f64) f64
fn floor(x f64) f64
fn ceil(x f64) f64
fn round(x f64) f64
fn trunc(x f64) f64
fn min(x f64, y f64) f64
fn max(x f64, y f64) f64
fn clamp(x f64, low f64, high f64) f64
fn hypot(x f64, y f64) f64
fn isFinite(x f64) bool
fn isNaN(x f64) bool
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/math"

fn main() {
    print(math.sqrt(9.0))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/math/main.dev
d examples/modules-api/math/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime adds `std/math`, `std/random`, `std/datetime`,
`std/test`, `std/log`, and expands `std/strings`. Import with `use "std/math"`.
These APIs run without a C compiler. They are not yet provided by native stdlib
or existing release installers. Shared signatures let the editor check calls.
Arguments are required; there are no optional/variadic arguments.

## Math

All numeric arguments/results use **f64**; use `9.0` or an explicit `as f64` cast.
Angles are radians. Invalid domains/nonfinite results raise runtime errors.

| API | Meaning |
| --- | --- |
| `pi()`, `e()`, `tau()` | Constants as functions |
| `abs(x)`, `sqrt(x)`, `cbrt(x)` | Absolute value and roots |
| `pow(x,y)`, `exp(x)` | Powers/exponential |
| `log(x)`, `log2(x)`, `log10(x)` | Natural, base-2 and base-10 logarithms |
| `sin(x)`, `cos(x)`, `tan(x)` | Trigonometry |
| `asin(x)`, `acos(x)`, `atan(x)`, `atan2(y,x)` | Inverse trigonometry |
| `floor(x)`, `ceil(x)`, `round(x)`, `trunc(x)` | Rounding; `round` ties away from zero |
| `min(x,y)`, `max(x,y)`, `clamp(x,low,high)` | Bounds; clamp rejects low > high |
| `hypot(x,y)` | Hypotenuse |
| `isFinite(x)`, `isNaN(x)` | f64 predicates returning bool |

This is floating-point math, not arbitrary-precision arithmetic. Platform math
implementations can differ slightly; use `test.near` for approximate comparisons.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../basic-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/math/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
