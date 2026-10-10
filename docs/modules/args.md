# std/args

อ่าน arguments ของโปรแกรมที่ส่งหลัง --

[กลับหน้ารวม module](index.md) · **โหมด:** Source runtime builtin; ไม่มี native stdlib binding สำหรับ std/args

## การ import และเรียกใช้งาน

```dev
use "std/args"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `args.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn len() usize
fn get(index integer) str
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/args"

fn main() {
    print(args.len())
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/args/main.dev
d examples/modules-api/args/main.dev --engine ast
```

ส่ง arguments เช่น `d examples/modules-api/args/main.dev -- hello world` จะพิมพ์จำนวน 2

## พฤติกรรม, errors และข้อจำกัด

นับเฉพาะ arguments หลัง -- ไม่รวมชื่อ executable หรือ source file get ใช้ index เริ่มจาก 0; index ติดลบหรือเกินจำนวนเป็น error ตรวจ len ก่อน get หรือใช้ std/cli เมื่อต้องการ parse flags

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../runtime.md)
- [Source ตัวอย่าง](../../examples/modules-api/args/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
