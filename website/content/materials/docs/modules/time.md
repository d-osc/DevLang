# std/time

วัดเวลาที่ผ่านไปด้วยนาฬิกา monotonic และพัก thread

[กลับหน้ารวม module](index.md) · **โหมด:** Source runtime builtin; native stdlib มีชื่อเดียวกันแต่ API/ownership ต่างกัน ดู [native counterpart](native-time.md)

## การ import และเรียกใช้งาน

```dev
use "std/time"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `time.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn now_ns() u64
fn now_ms() u64
fn sleep_ms(milliseconds integer) bool
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/time"

fn main() {
    let start = time.now_ns()
    time.sleep_ms(1)
    print(time.now_ns() >= start)
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/time/main.dev
d examples/modules-api/time/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

นับเวลาที่ผ่านไปตั้งแต่ runtime เริ่มด้วย monotonic clock ไม่ใช่ Unix timestamp ใช้ std/datetime สำหรับวันที่จริง sleep_ms บล็อก thread ปัจจุบันและคืน true; ค่าติดลบเป็น error

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../runtime.md)
- [Source ตัวอย่าง](../../examples/modules-api/time/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
