# std/encoding

แปลงข้อความและ bytes ตาม encoding

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/encoding"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `encoding.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn encode(text str, charset str) Vec<u8>
fn decode(data Vec<u8>, charset str) str
fn transcode(data Vec<u8>, from str, to str) Vec<u8>
fn supported(charset str) bool
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/encoding"

fn main() {
    let bytes = encoding.encode("Dev 🚀", "utf-16le")
    print(encoding.decode(bytes, "utf-16le"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/encoding/main.dev
d examples/modules-api/encoding/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

Six additional runtime modules: `std/regex`, `std/encoding`, `std/crypto`,
`std/compression`, `std/archive`, `std/uuid`. They require the current source build;
native stdlib and existing installers do not provide them. Arguments are required
and signatures are shared with editor checking. Binary APIs use `Vec<u8>`;
`encoding.encode` or `buffer.toBytes` bridges text/Buffer to binary input.

## Encoding

| API | Contract |
| --- | --- |
| `encode(text,charset) Vec<u8>` | Strict text encoding; unrepresentable text fails |
| `decode(bytes,charset) str` | Strict decoding; malformed sequences fail |
| `transcode(bytes,from,to) Vec<u8>` | Decode then encode |
| `supported(charset) bool` | Check recognized charset label |

UTF-8, UTF-16LE/BE, Windows-1252, Shift_JIS, GBK, Big5 and other encoding_rs labels
are supported. UTF-16 encoding writes no BOM; decoding preserves an explicit BOM
as U+FEFF, requires even byte length and rejects invalid surrogates. Other decoding
also preserves BOMs. Labels follow WHATWG mappings: `latin1`/`iso-8859-1` map to
Windows-1252, not a strict ISO-8859-1 implementation. Outputs/inputs are capped at
8 MiB. There is no silent replacement of malformed/unrepresentable text.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../data-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/encoding/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
