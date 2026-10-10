# std/io

อ่าน/เขียนข้อความบน console และไฟล์ UTF-8

[กลับหน้ารวม module](index.md) · **โหมด:** Source runtime builtin; native stdlib มีชื่อเดียวกันแต่ API/ownership ต่างกัน ดู [native counterpart](native-io.md)

## การ import และเรียกใช้งาน

```dev
use "std/io"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `io.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn write(text str) bool
fn writeln(text str) bool
fn read_line() str
fn read_file(path str) str
fn write_file(path str, text str) bool
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/io"

fn main() {
    io.writeln("Hello DevLang")
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/io/main.dev
d examples/modules-api/io/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

ข้อความใช้ UTF-8; read_line ตัด newline ท้ายบรรทัดออก ความล้มเหลวของ I/O เป็น runtime error; เส้นทาง relative อิง working directory ขณะรัน read_line คืน str โดยไม่ต้องเตรียม buffer ต่างจาก native std/io

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../runtime.md)
- [Source ตัวอย่าง](../../examples/modules-api/io/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
