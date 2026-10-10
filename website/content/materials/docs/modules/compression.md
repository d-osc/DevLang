# std/compression

บีบอัด bytes ด้วย gzip, zlib และ deflate

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/compression"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `compression.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn gzip(data Vec<u8>, level i64) Vec<u8>
fn gunzip(data Vec<u8>) Vec<u8>
fn zlib(data Vec<u8>, level i64) Vec<u8>
fn unzlib(data Vec<u8>) Vec<u8>
fn deflate(data Vec<u8>, level i64) Vec<u8>
fn inflate(data Vec<u8>) Vec<u8>
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/compression"
use "std/encoding"

fn main() {
    let bytes = encoding.encode("Hello", "utf8")
    let packed = compression.gzip(bytes, 6)
    print(encoding.decode(compression.gunzip(packed), "utf8"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/compression/main.dev
d examples/modules-api/compression/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

Six additional runtime modules: `std/regex`, `std/encoding`, `std/crypto`,
`std/compression`, `std/archive`, `std/uuid`. They require the current source build;
native stdlib and existing installers do not provide them. Arguments are required
and signatures are shared with editor checking. Binary APIs use `Vec<u8>`;
`encoding.encode` or `buffer.toBytes` bridges text/Buffer to binary input.

## Compression

| API | Contract |
| --- | --- |
| `gzip(bytes,level)` / `gunzip(bytes)` | GZIP; decode supports concatenated members |
| `zlib(bytes,level)` / `unzlib(bytes)` | ZLIB-wrapped DEFLATE |
| `deflate(bytes,level)` / `inflate(bytes)` | Raw DEFLATE |

All outputs are Vec<u8>. Level is 0..9. Input, encoded output and decoded output
are limited to 8 MiB. Decoders enforce output limits while reading, reject
truncated/malformed streams and trailing invalid data. GZIP/ZLIB checksums are
validated; raw DEFLATE has no checksum. These APIs are one-shot, not streaming;
there is no Brotli/Zstd module yet.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../data-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/compression/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
