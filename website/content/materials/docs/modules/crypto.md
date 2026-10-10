# std/crypto

hash, HMAC, secure random และ authenticated encryption

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/crypto"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `crypto.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
fn hash(algorithm str, data Vec<u8>) Vec<u8>
fn hashText(algorithm str, text str) str
fn hmac(algorithm str, key Vec<u8>, data Vec<u8>) Vec<u8>
fn verifyHmac(algorithm str, key Vec<u8>, data Vec<u8>, tag Vec<u8>) bool
fn secureBytes(size i64) Vec<u8>
fn timingSafeEqual(a Vec<u8>, b Vec<u8>) bool
fn encrypt(key Vec<u8>, data Vec<u8>, associatedData Vec<u8>) Vec<u8>
fn decrypt(key Vec<u8>, packet Vec<u8>, associatedData Vec<u8>) Vec<u8>
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/crypto"

fn main() {
    print(crypto.hashText("sha256", "abc"))
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/crypto/main.dev
d examples/modules-api/crypto/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

Six additional runtime modules: `std/regex`, `std/encoding`, `std/crypto`,
`std/compression`, `std/archive`, `std/uuid`. They require the current source build;
native stdlib and existing installers do not provide them. Arguments are required
and signatures are shared with editor checking. Binary APIs use `Vec<u8>`;
`encoding.encode` or `buffer.toBytes` bridges text/Buffer to binary input.

## Crypto

| API | Contract |
| --- | --- |
| `hash(algorithm,bytes) Vec<u8>` | SHA-256 or SHA-512 digest |
| `hashText(algorithm,text) str` | UTF-8 digest as lowercase hex |
| `hmac(algorithm,key,data) Vec<u8>` | HMAC SHA-256/SHA-512 |
| `verifyHmac(algorithm,key,data,tag) bool` | Verify tag; wrong tag/length returns false |
| `secureBytes(size) Vec<u8>` | OS-provided cryptographic random bytes |
| `timingSafeEqual(a,b) bool` | Equal-length content comparison using subtle; differing lengths return false |
| `encrypt(key,data,associatedData) Vec<u8>` | AES-256-GCM with fresh random nonce |
| `decrypt(key,packet,associatedData) Vec<u8>` | Authenticate before returning plaintext |

Algorithm names are exactly `sha256` / `sha512`. AES keys must contain 32 bytes;
generate with `secureBytes(32)`. This is distinct from the noncryptographic
`std/random` PRNG. AES-GCM uses the RustCrypto implementation; algorithms are not
implemented from scratch in DevLang.

```dev-runtime
use "std/crypto"
use "std/encoding"
let key = crypto.secureBytes(32)
let data = encoding.encode("DevLang", "utf8")
let aad = encoding.encode("example-v1", "utf8")
let packet = crypto.encrypt(key, data, aad)
print(encoding.decode(crypto.decrypt(key, packet, aad), "utf8"))
```

Packet format: **12-byte nonce + ciphertext + 16-byte authentication tag**.
Encrypt generates the nonce through the OS; callers cannot specify it. Decrypt
requires the same key and associated data. A modified packet/key/AAD raises an
authentication error. Associated data is authenticated but not included in the
packet, so its storage/protocol is the application's responsibility. Empty AAD
can be supplied as `Vec<u8>()`.

The complete encrypted packet is capped at 8 MiB (plaintext maximum 8 MiB minus
28 bytes). Other binary inputs/outputs are capped at 8 MiB. Lengths are visible;
timingSafeEqual does not make surrounding interpreted code constant-time.
Managed values/copies are not automatically zeroized. No password hashing/KDF,
RSA/ECC signatures, key storage or certificate APIs are implemented in this set;
raw passwords are not AES keys and SHA digests are not password-storage APIs.
No system-level security certification is claimed by these runtime tests.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../data-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/crypto/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
