# std/datetime

แปลง Unix milliseconds, RFC3339 และปฏิทิน

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/datetime"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `datetime.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
struct Parts { year i64, month i64, day i64, hour i64, minute i64, second i64, millisecond i64, weekday i64, offsetMinutes i64 }
fn now() i64
fn parse(text str) i64
fn iso(timestamp i64) str
fn format(timestamp i64, pattern str) str
fn formatOffset(timestamp i64, offsetMinutes i64, pattern str) str
fn parts(timestamp i64, offsetMinutes i64) Parts
fn utc(year i64, month i64, day i64, hour i64, minute i64, second i64, millisecond i64) i64
fn addMilliseconds(timestamp i64, duration i64) i64
fn isLeapYear(year i64) bool
fn daysInMonth(year i64, month i64) i64
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/datetime"

fn main() {
    print(datetime.iso(datetime.parse("2026-01-01T00:00:00Z")))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/datetime/main.dev
d examples/modules-api/datetime/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

The current source runtime adds `std/math`, `std/random`, `std/datetime`,
`std/test`, `std/log`, and expands `std/strings`. Import with `use "std/math"`.
These APIs run without a C compiler. They are not yet provided by native stdlib
or existing release installers. Shared signatures let the editor check calls.
Arguments are required; there are no optional/variadic arguments.

## Datetime

Timestamps are **signed i64 Unix milliseconds**, distinct from the monotonic
elapsed time in `std/time`. Supported calendar range is bounded by Chrono.

| API | Meaning |
| --- | --- |
| `now() i64` | Current system clock |
| `parse(text) i64` | RFC3339 with explicit Z/offset; fractions truncated to milliseconds |
| `iso(timestamp) str` | UTC RFC3339, with three fractional digits |
| `format(timestamp,pattern) str` | UTC strftime-style format, e.g. `%F %T` |
| `formatOffset(timestamp,offsetMinutes,pattern) str` | Format in a fixed UTC offset |
| `parts(timestamp,offsetMinutes) Parts` | Calendar fields in a fixed UTC offset |
| `utc(year,month,day,hour,minute,second,millisecond) i64` | Construct validated UTC timestamp |
| `addMilliseconds(timestamp,duration) i64` | Checked signed-duration addition |
| `isLeapYear(year) bool`, `daysInMonth(year,month) i64` | Gregorian calendar helpers |

`Parts` fields: year, month, day, hour, minute, second, millisecond, weekday,
offsetMinutes (all i64). Month/day start at 1; weekday is Monday=1 to Sunday=7.
Fixed offsets must be -1439..1439 minutes; Bangkok uses 420. Invalid dates,
leap seconds, unsupported format directives and out-of-range dates raise errors.
This release does not include IANA timezone rules, DST or localized formatting.
System clock corrections can move `now()` backwards; use `std/time` for durations.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../basic-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/datetime/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
