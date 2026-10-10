# std/test

assertions และชุดทดสอบแบบ callback

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/test"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `test.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
struct Report { total i64, passed i64, failed i64, summary str }
fn expect(condition bool, message str) void
fn equal<T>(actual T, expected T, message str) void
fn near(actual f64, expected f64, tolerance f64, message str) void
fn case(name str, body fn() void) void
fn run() Report
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/test"

fn main() {
    test.case("addition", fn() { test.equal(19 + 23, 42, "sum") })
    let report = test.run()
    print(report.passed)
    if report.failed > 0 { return 1 }
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/test/main.dev
d examples/modules-api/test/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime adds `std/math`, `std/random`, `std/datetime`,
`std/test`, `std/log`, and expands `std/strings`. Import with `use "std/math"`.
These APIs run without a C compiler. They are not yet provided by native stdlib
or existing release installers. Shared signatures let the editor check calls.
Arguments are required; there are no optional/variadic arguments.

## Tests

| API | Meaning |
| --- | --- |
| `expect(condition bool,message str)` | Raise an assertion error when false |
| `equal<T>(actual T,expected T,message str)` | Exact structural comparison of supported JSON-encodable Dev values |
| `near(actual f64,expected f64,tolerance f64,message str)` | Absolute error tolerance; finite values/nonnegative tolerance required |
| `case(name str,body fn() void)` | Register a named callback |
| `run() Report` | Run callbacks in registration order; catch errors and continue |

`Report` contains total/passed/failed i64 and summary str. Running consumes the
queue. At most 1024 uniquely named cases are accepted. Nested runs and registering
cases during a run fail. Summary rows are truncated after 4096 UTF-8 bytes.
Callbacks follow DevLang's usual capture-by-value rules. `equal` excludes function
values/tasks/non-null raw pointers and does not compare underlying handle resources.
There are no mocks, fixture hooks, directory discovery or async test scheduler yet.
The runner **does not change the process exit code**; propagate a failure explicitly:

```dev-runtime
use "std/test"
use "std/math"
fn main() {
    test.case("sqrt", fn() {
        test.near(math.sqrt(9.0), 3.0, 0.000001, "square root")
    })
    let report = test.run()
    print(report.summary)
    if report.failed > 0 { return 1 }
    return 0
}
return main()
```

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../basic-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/test/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
