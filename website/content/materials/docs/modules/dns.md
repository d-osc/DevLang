# std/dns

resolve host ด้วย DNS resolver ของระบบปฏิบัติการ

[กลับหน้ารวม module](index.md) · **โหมด:** ใช้ d run หรือ d FILE.dev ได้โดยไม่ต้องมี C compiler; native build ใช้ API นี้ไม่ได้

## การ import และเรียกใช้งาน

```dev
use "std/dns"
```

เรียก function ด้วย alias ของชื่อส่วนท้าย เช่น `dns.FUNCTION(...)` หรือใช้ `as alias` เพื่อกำหนดชื่อเอง Methods เรียกผ่าน instance; ต้องส่ง arguments ครบตาม signature

## ชนิดข้อมูลและ signatures

รายการนี้แสดงชื่อ, parameter และ return type โดยไม่มี function bodies ใช้เป็น API reference; methods ที่มี self รับ instance ผ่านการเรียก instance.method(...)

```text
struct Address { address str, family i64 }
fn lookup(host str, family i64) Vec<str>
fn lookupOne(host str, family i64) Address
fn isIP(address str) i64
fn isIPv4(address str) bool
fn isIPv6(address str) bool
```

## ตัวอย่างเริ่มต้น

ตัวอย่างนี้รันในเครื่องได้ ไม่ต้องมีบริการภายนอก

```dev
use "std/dns"

fn main() {
    print(dns.lookupOne("127.0.0.1", 4).address)
    return 0
}
return main()
```

ใช้ source build ล่าสุด; binary ของ installer เก่าอาจไม่มี module นี้:

```sh
d examples/modules-api/dns/main.dev
d examples/modules-api/dns/main.dev --engine ast
```

## พฤติกรรม, errors และข้อจำกัด

`std/dns` and `std/cli` are built into the updated **source runtime**. Their
signatures are shared with the Language Server. Native builds do not implement
these modules. They need no C compiler or external executable.

## DNS / host resolution

```dev
use "std/dns"
fn main() {
    let addresses = dns.lookup("localhost", 0)
    for i in 0..addresses.len() as i64 {
        print(addresses[i as usize])
    }
    let first = dns.lookupOne("127.0.0.1", 4)
    print(first.address)
    print(first.family)
}
main()
```

| API | Result |
| --- | --- |
| `lookup(host str, family i64)` | `Vec<str>` of canonical IP addresses |
| `lookupOne(host str, family i64)` | `Address { address str, family i64 }`, first matching result |
| `isIP(address str)` | `i64`: 4 for IPv4, 6 for IPv6, 0 for invalid input |
| `isIPv4(address str)` | `bool` |
| `isIPv6(address str)` | `bool` |

`family` must be 0 (either), 4 or 6. Duplicate IPs are removed while preserving
system resolver order. `lookup` returns an empty vector if resolution succeeds
but the selected family has no results; `lookupOne` reports a located error.
Resolver failures also report located runtime errors, not empty success.

Numeric IP literals bypass name resolution and are normalized; `::1` is valid,
`[::1]` and `127.0.0.1:80` are not IP inputs. Scoped IPv6 addresses are not
supported. Hostnames must use ASCII letters/digits/hyphens, labels of 1..63
bytes, and at most 253 bytes overall. Labels cannot start/end with a hyphen;
a trailing root dot is allowed. For international domains use
`url.domainToASCII(name)` before lookup. IP classifiers simply return false/0
for invalid input without attempting a lookup.

This is **OS host resolution**, including hosts-file and OS resolver policy,
not a raw DNS-record client. It has no MX/TXT/SRV/PTR, TTL, reverse lookup,
custom DNS server, DNSSEC validation, cancellation or configurable timeout.
Lookups block until the system resolver returns; its ordering/cache/timeouts
are controlled by the OS. At most 256 matching unique IPs are returned; this
does not limit the resolver's internal allocations. No network socket remains
open after the call.

Use the returned address with `net.connect(port,address,timeout_ms)` or UDP
APIs. A connect timeout does not bound a separate DNS lookup. For concurrent
work, call lookup inside an existing `async fn` and await its Task; that uses an
OS thread and does not add a cancellable resolver or event loop.

## แหล่งอ้างอิงและการตรวจ

- [สัญญา API ฉบับรวม](../system-libs.md)
- [Source ตัวอย่าง](../../examples/modules-api/dns/main.dev)
- [Shared runtime signatures](../../syntax/src/intrinsics.rs) และ [runtime implementation](../../runtime/src/engine.rs)

สร้างด้วย `python scripts/build_module_docs.py`; ตรวจความสดใหม่ด้วย `--check` และรันตัวอย่างด้วย `--d target/release/d.exe` ตัวอย่าง network เริ่มต้นไม่ได้พิสูจน์ TLS handshake หรือรับส่งข้อมูล; ดู smoke suites ในคู่มือฉบับรวม
