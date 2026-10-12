# Dev Language Cheat Sheet

สรุป syntax ที่ใช้บ่อยใน DevLang v0.5.1 สำหรับเปิดดูระหว่างเขียนโค้ด ตัวอย่างแต่ละ block รันแยกกันได้ ยกเว้นส่วน modules และ FFI ที่ระบุเป็น template อ่าน [คู่มือทั้งหมด](#/docs/intro) หรือ [ตัวอย่างพร้อมไฟล์ดาวน์โหลด](#/examples) เมื่อต้องการรายละเอียด

การเขียน `): ReturnType` และ formatter ที่ใช้ colon เป็นรูปแบบมาตรฐานอยู่ใน source ล่าสุดหลัง release v0.5.1

## เริ่มเขียนและรัน

```dev
fn main() {
    print("Hello Dev")
    print(19 + 23)
}
main()
```

ต้องเรียก `main()` เอง การประกาศฟังก์ชันไม่ทำให้มันรัน ใช้ `return main()` ระดับไฟล์เมื่อต้องการส่งค่าคืนเป็น exit status

| งาน | คำสั่ง |
| --- | --- |
| รัน source | `d main.dev` หรือ `d run main.dev` |
| ตรวจด้วย native frontend | `d check main.dev` |
| สร้าง native executable | `d build main.dev --release --output out/app` |
| จัดรูปแบบ | `d fmt main.dev` |
| สร้างโปรเจกต์ | `d new myapp` |
| ติดตั้ง dependencies | `d pkg install` |
| ติดตั้งตาม lockfile | `d pkg ci` |

Pure Dev runtime ไม่ต้อง build โปรแกรมก่อน Native build ต้องมี C backend ส่วน API เฉพาะ runtime อาจไม่ผ่าน `d check` อ่าน [CLI](#/docs/cli) และ [เครื่องมือพัฒนา](#/docs/tooling)

## ตัวแปร ชนิดข้อมูล และ comments

```dev
# comment
// comment อีกแบบ
let count = 10                // อนุมานเป็น i64
let price: f64 = 12.5
let enabled: bool = true
let name: str = "Dev"
let byte: u8 = 255
count += 2
let wide = byte as i64
print(count + wide)
```

`let` แก้ค่าได้และต้องมีค่าเริ่มต้น ชนิดตัวเลขต่างกันต้องแปลงด้วย `as` Statement จบด้วย newline หรือ `;` เพิ่มได้ Block ใช้ `{ }`

| ชนิด | ใช้กับ |
| --- | --- |
| `i8 i16 i32 i64` | จำนวนเต็มมีเครื่องหมาย |
| `u8 u16 u32 u64` | จำนวนเต็มไม่มีเครื่องหมาย |
| `isize usize` | จำนวนเต็มตามขนาด pointer |
| `f32 f64` | ทศนิยม |
| `bool`, `str` | boolean, UTF-8 string |
| `[T; N]`, `Vec<T>`, `Slice<T>` | array ขนาดคงที่, vector, snapshot อ่านอย่างเดียว |
| `Map<K,V>`, `Ref<T>`, `Task<T>` | map, managed reference, ผลลัพธ์จาก worker |
| `*T`, `void` | raw pointer, ฟังก์ชันที่ไม่คืนค่า |

ดู [ตัวแปร](#/docs/variables) และ [ชนิดข้อมูล](#/docs/types)

## Operators และ if / else

```dev
let age = 20
let active = true
if age >= 18 && active {
    print("allowed")
} else if age >= 18 || active {
    print("review")
} else {
    print("denied")
}
print(not false and true)
```

| กลุ่ม | Operators |
| --- | --- |
| คำนวณ | `+ - * / %` |
| เปรียบเทียบ | `== != < <= > >=` |
| boolean | `&&` / `and`, `\|\|` / `or`, `!` / `not` |
| bitwise | `& \| ^ ~ << >>` |
| กำหนดค่า | `= += -= *= /= %= &= \|= ^=` |

เงื่อนไขต้องเป็น `bool` Boolean AND/OR เป็น short-circuit ใช้ `()` เพื่อจัดกลุ่มตามต้องการ ไม่มี truthiness แบบ JavaScript และไม่ใช้ `==` เทียบเนื้อหา `str` ดู [Operators](#/docs/operators)

## for / while / break / continue

```dev
let total = 0
for i in 0..5 {               // 0, 1, 2, 3, 4
    if i == 2 { continue }
    total += i
}
let n = 3
while n > 0 {
    print(n)
    n -= 1
    if n == 1 { break }
}
print(total)
```

`for` ใช้ range เพิ่มขึ้นและไม่รวมปลายทาง Bounds เป็น `i64` ยังไม่มี `for item in collection`, inclusive range หรือ custom step ถ้าต้องการวน vector ให้ใช้ index กับ `while` และ `.len()` ดู [Loops](#/docs/loops)

## ฟังก์ชันและ generics

```dev
fn add(a: i64, b: i64): i64 { return a + b }
fn identity<T>(value: T): T { return value }
fn sum<T: Number>(a: T, b: T): T { return a + b }
print(add(19, 23))
print(identity<i64>(42))
print(sum<f64>(1.5, 2.5))
```

Parameter ต้องระบุชนิด Return type อยู่หลัง `)` ใส่ `:` และ `->` เป็นรูปแบบทางเลือกได้ ฟังก์ชันทั่วไปที่ไม่ระบุ return type ใช้ `void` ส่วน `main` ใช้ `i32` และคืน 0 เมื่อถึงท้ายฟังก์ชัน ดู [Functions](#/docs/functions) และ [Generics](#/docs/generics)

## Structs และ methods

```dev
struct Point { x: i64; y: i64 }
fn Point.sum(self: Point): i64 { return self.x + self.y }
let point = Point(19, 23)
point.x += 1
print(point.sum())
```

Constructor ใช้ตำแหน่งตามลำดับ field ต้องส่งครบ ไม่มี named-field constructor Method รับ `self` แบบ value การแก้ self ไม่แก้ตัวแปรต้นทางอัตโนมัติ Native ให้ print แต่ละ field แทนการ print struct ทั้งตัว ดู [Structs](#/docs/structs)

## Enums และ match

```dev
enum Result<T> { Ok(T), Err(str) }
let result = Result<i64>.Ok(42)
match result {
    Ok(value) if value > 0 => { print(value) }
    Err(message) => { print(message) }
    _ => { print(0) }
}
```

`match` เป็น statement ต้องครอบคลุมทุก variant ด้วย arm ที่ไม่มี guard หรือปิดท้าย `_` รองรับ payload, guard และ nested enum patterns Payload enum ไม่รองรับ equality/integer cast ดู [Enums และ match](#/docs/enums-match)

## Arrays, Vec, Slice และ Map

```dev
let fixed: [i64; 3] = [10, 20, 30]
fixed[1] = 42
print(fixed[1])
let values = Vec<i64>(19, 23)
values.push(7)
let snapshot = values.slice(0, 2)
values[0] = 99
print(snapshot[0] + snapshot[1]) // 42
print(values.pop())            // 7
let scores = Map<str, i64>()
scores.set("answer", 42)
if scores.contains("answer") { print(scores.get("answer")) }
```

Vec/Map มี `.len()` และ `.clear()` Map มี `.remove(key)` Slice เป็น read-only snapshot ช่วง slice ไม่รวมปลายทาง Vec/Map ใช้ copy-on-write `pop` เมื่อว่างและ `get` เมื่อไม่มี key เป็น error Native fixed array ส่ง/คืน/คัดลอกทั้ง array โดยตรงไม่ได้ ดู [Arrays](#/docs/strings-arrays) และ [Collections](#/docs/collections)

## Ref และ recursive data

```dev
enum List<T> { Nil, Cons(T, Ref<List<T>>) }
let tail = ref(List<i64>.Nil())
let list = List<i64>.Cons(42, tail)
match list {
    Nil => {}
    Cons(value, next) => { print(value); print(deref(next)) }
}
```

`ref(value)` สร้าง immutable managed reference ส่วน `deref` คืนสำเนาค่า ใช้ Ref/collections/raw pointer ตัดวงจร recursive layout การมี field ชนิดตัวเองแบบ value ตรง ๆ ไม่ได้ ดู [References](#/docs/references)

## Function values และ closures

```dev
fn make(offset: i64): fn(i64): i64 {
    return fn(value: i64): i64 { return offset + value }
}
let add = make(19)
print(add(23))
```

Closure capture ค่า ณ เวลาสร้าง การแก้ captured local ไม่สะสมข้ามการเรียกและไม่แก้ตัวแปรภายนอก ดู [Closures](#/docs/closures)

## Modules และ export

Template สองไฟล์: `math.dev` มี declarations เท่านั้น ส่วน `main.dev` เป็น entry point

```dev-syntax
// math.dev
fn add(a i64, b i64) i64 { return a + b }
```

```dev-syntax
// main.dev
use "math.dev" as math
fn main() { print(math.add(19, 23)) }
main()
```

ใช้ `use math` สำหรับไฟล์ข้างกัน หรือ `use "package/lib" as lib` สำหรับ module จาก dependency ฟังก์ชันที่ import เรียกได้โดยไม่ต้อง `export` ส่วน `export fn` ใช้รักษาชื่อ symbol สำหรับ C ABI ดู [Modules](#/docs/modules) และ [Package tools](#/docs/package-features)

## JSON object — runtime เท่านั้น

```dev-runtime
use "std/json" as json
let user = { name: "Dev", age: 18, active: true }
print(user.name)
print(json.stringify(user))
```

รองรับ object literal และ dot access ใน source runtime ตัวอย่างนี้ใช้ API JSON เฉพาะ runtime ดู [JSON](#/docs/json)

## Threads และ async

```dev
async fn answer(n: i64): i64 { return n + 1 }
let task = answer(41)
print(await(task))
let worker = spawn(fn(): i64 { return 19 + 23 })
print(await(worker))
```

`async fn` และ `spawn` ใช้ OS threads ส่วน `await` รอแบบ blocking และ `ready` ตรวจว่าทำงานเสร็จหรือยัง ยังไม่ใช่ coroutine event loop Native ต้องใช้ C11 backend สำหรับงาน threads ดู [Tasks](#/docs/tasks) และ [Sync](#/docs/sync)

## unsafe, FFI และ hardware

Template สำหรับ C symbol ที่ต้องมีอยู่ใน native library:

```dev-syntax
extern fn device_add(a i32, b i32) i32
fn add_device(a i32, b i32) i32 {
    unsafe { return device_add(a, b) }
}
```

| เครื่องมือ | ใช้กับ |
| --- | --- |
| `unsafe { ... }` | extern calls, raw pointers, casts/address-of และ volatile access |
| `sizeof(value)` | ขนาดชนิดข้อมูลเป็น `usize` |
| `volatile_load(pointer)` | อ่าน register/memory แบบ volatile |
| `volatile_store(pointer, value)` | เขียน register/memory แบบ volatile |
| `d build main.dev --link device.c` | build และ link C implementation |
| `d run main.dev --ffi-lib PATH` | โหลด DLL/shared library สำหรับ runtime FFI |

Signature และอายุ pointer ต้องตรงกับ C จริง `unsafe` ไม่ตรวจอายุ/ขนาด allocation ของ foreign pointer และ volatile ไม่ใช่ atomic หรือ memory fence Pure Dev recursive data ควรใช้ `Ref` ดู [Safety](#/docs/safety), [FFI](#/docs/ffi) และ [Hardware](#/docs/hardware)

## package.don แบบย่อ

```don
version: '1.0.0'
package: { name: 'myapp', entry: 'src/main.dev', modules: 'src' }
dependencies: { math: { path: '../math' } }
scripts: { start: ['d', 'run'] }
```

วาง `package.don` ที่ root ของโปรเจกต์ แล้วรัน `d pkg install` และ `d task start` ไฟล์ config ใช้ DON ไม่ใช่ syntax ของ `.dev` ดู [DON](#/docs/don), [Packages](#/docs/packages) และ [Package Store](#/packages)
