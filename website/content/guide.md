# [intro] รู้จัก DevLang

DevLang คือภาษาเล็กสำหรับเขียนโปรแกรมทั่วไป งานคำนวณ และเชื่อมต่อ C ใช้ไฟล์ `.dev` และคำสั่ง `d` มี syntax สั้น แต่ยังระบุชนิดข้อมูลและขอบเขตการทำงานชัดเจน

## สองวิธีในการรันโค้ด

| วิธี | คำสั่ง | เหมาะกับ |
| --- | --- | --- |
| Source runtime | `d main.dev` | ลองโค้ด สคริปต์ และพัฒนาโดยไม่ build โปรแกรม Dev |
| Native compiler | `d build main.dev --release` | สร้าง executable และงานที่ต้องการ native code |

Runtime อ่านและประมวลผล source โดยตรง มี numeric execution plans ในหน่วยความจำและ AST fallback ไม่มี JIT การ build แปลง Dev เป็น C11 แล้วให้ Clang/GCC หรือ TinyCC สร้าง machine code

Pure Dev ใน runtime ไม่ต้องมี compiler ส่วน FFI ที่มี C source ต้องเตรียม native dependency ในครั้งแรก จึงยังต้องมี `devc` และ C backend

## โปรแกรมแรก

```dev
fn main() {
    print("Hello Dev")
    print(19 + 23)
}

main()
```

การประกาศ `fn main()` **ไม่ได้เรียกฟังก์ชันเอง** ต้องมี `main()` ด้านท้าย ฟังก์ชันนี้พิมพ์ Hello Dev และ 42 อย่างละบรรทัด

DevLang ยังเป็นภาษาในช่วงพัฒนา ไม่ควรสมมติว่าเร็วกว่า C/Rust/Go ทุกงาน หรือมีความปลอดภัยเท่าภาษาที่มี borrow checker

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Hello Dev](#/examples/hello) — ประกาศ main และเรียกใช้งานอย่างชัดเจน

# [install] ติดตั้งและเริ่มต้น

## Windows

ดาวน์โหลดตัวติดตั้ง `.exe` จากหน้า Download เปิดไฟล์ แล้วกด **Install** ตัวติดตั้งรวม `d`, `devc`, `devrun`, stdlib และ examples พร้อมเพิ่ม user PATH ติดตั้งโดยไม่ต้องใช้สิทธิ์ administrator

โฟลเดอร์เริ่มต้นคือ `%LOCALAPPDATA%\Programs\DevLang` เปิด terminal ใหม่หลังติดตั้ง

```sh
d --help
d --version
d --eval 'print(40 + 2)'
```

ใน PowerShell ใช้ single quotes สำหรับ argument ของ `--eval` ได้ เก็บ source เป็น UTF-8 และตั้งชื่อ `main.dev`

## Linux

รัน offline installer ด้วย Python 3.9 ขึ้นไป ชุด binary Linux x86_64 ต้องการ glibc 2.39 ขึ้นไป

```sh
python3 devlang-setup-v0.4.0-linux-x86_64.py
export PATH="$HOME/.local/bin:$PATH"
d --help
```

ติดตั้งใน `~/.local/share/devlang` และสร้าง symlink ใน `~/.local/bin` หากต้องการใช้ถาวร ให้เพิ่ม PATH ในการตั้งค่า shell ของคุณ

## รันและ build

```sh
d main.dev
d run main.dev --timings
d build main.dev --release --output out/app
```

Windows native builds ต้องมี Clang/GCC และ target toolchain ส่วน Linux bundle มี TinyCC สำหรับ basic fast builds งาน threads/callbacks ขั้นสูงต้องใช้ backend ที่รองรับ C11 atomics

ถอนการติดตั้ง Windows ผ่าน Installed apps ได้ Linux ให้รัน installer เดิมพร้อม `--uninstall` และ directory options เดิม

# [layout] รูปแบบไฟล์และ entry point

## Statement และ block

Statement จบด้วย newline หรือ semicolon ที่ใส่เพิ่มได้ Block ใช้ `{ }` ส่วน newline ภายใน `()` และ `[]` ถือเป็น whitespace Comment ใช้ `#` หรือ `//`

```dev
# comment แบบแรก
// comment แบบที่สอง
let a = 19
let b = 23; print(a + b)
```

ชื่อใช้ตัวอักษร ASCII ตัวเลขและ `_` แต่ห้ามขึ้นต้นด้วยตัวเลข String รองรับ UTF-8 เช่นข้อความภาษาไทย

## main และ exit status

```dev
fn main() i32 {
    print("เรียก main อย่างชัดเจน")
    return 0
}
return main()
```

`main()` แบบ statement ทิ้งค่าที่คืนมา แต่ `return main()` ระดับไฟล์นำค่าที่คืนมาเป็น exit status ฟังก์ชันชื่อ main ที่ไม่ระบุ return type ใช้ `i32` และเมื่อถึงท้ายฟังก์ชันจะคืน 0 ส่วนฟังก์ชันทั่วไปที่ไม่ระบุ return type ใช้ `void`

Entry file เขียน executable statements ได้ตามลำดับ แต่ไฟล์ที่ import ต้องมี declarations เท่านั้น ตัวแปรระดับไฟล์อยู่ใน script scope ฟังก์ชันไม่ capture ตัวแปรเหล่านี้ ให้ส่งผ่าน parameter

# [variables] ตัวแปรและ scope

## ประกาศด้วย let

```dev
let answer = 40
answer += 2
let small i32 = 42
let explicit: i32 = 42
print(answer)
print(small + explicit)
```

`let` แก้ค่าได้ ไม่ได้หมายถึง immutable ทุกตัวแปรต้องมี initializer ถ้าไม่ระบุ type จะอนุมานจากค่า ใช้ `name Type` หรือ `name: Type` ก็ได้

## Scope และ shadowing

```dev
let value = 19
if true {
    let value = 42
    print(value)
}
print(value)
```

ผลคือ 42 แล้ว 19 ชื่อซ้ำใน scope เดียวกันเป็น error แต่ scope ด้านใน shadow ชื่อด้านนอกได้ Assignment เช่น `+=`, `-=`, `*=`, `/=`, `%=` ใช้กับชนิดที่รองรับ

## ตัวอย่างที่รันและดาวน์โหลดได้

- [ตัวแปร ชนิดข้อมูล และ cast](#/examples/variables) — let, mutation, shadowing, typed literals, as และ comments

เรื่องเงื่อนไข: [if / else if / else](#/examples/conditions) และ [&& / || / and / or](#/examples/logical)

# [types] ชนิดข้อมูลและการแปลงค่า

| Type | การใช้งาน |
| --- | --- |
| `i8 i16 i32 i64` | จำนวนเต็ม signed ตามขนาด bit |
| `u8 u16 u32 u64` | จำนวนเต็ม unsigned |
| `isize usize` | จำนวนเต็มตามความกว้าง pointer ของ target |
| `f32 f64` | floating point |
| `bool` | `true` / `false` |
| `str` | UTF-8 string; native ใช้ C string pointer |
| `[T; N]` | fixed array |
| `*T` | raw pointer ต้องใช้ unsafe ในการเข้าถึง |
| `Ref<T>` | immutable managed reference |
| `Vec<T>`, `Slice<T>`, `Map<K,V>` | managed collections |
| `fn(T) R`, `Task<T>` | function value และงานบน thread |
| `void` | ไม่มีค่าที่คืนมา |

## Literal และ contextual type

Integer literal ที่ไม่มี context เป็น `i64` ส่วน float เป็น `f64` รองรับเลขฐานสิบ ฐานสิบหก `_` คั่นหลัก และ exponent สำหรับ float Context จาก type annotation, parameter หรือ return ช่วยกำหนด type ของ literal

```dev
let hex u32 = 0xFF
let count = 1_000
let ratio = 1.5e2
let narrow i32 = 42
let wide = narrow as i64
print(hex)
print(count)
print(ratio)
print(wide)
```

Numeric types ไม่ผสมกันโดยอัตโนมัติ ใช้ `as` แปลงอย่างชัดเจน Literal ที่เกินช่วง type เป็น error การ cast ลดความกว้างอาจตัดค่าตามกฎ backend อย่าสมมติว่าจะตรวจ overflow ทุกกรณี

# [operators] Operators และเงื่อนไข

## กลุ่ม operators

| กลุ่ม | Operators |
| --- | --- |
| คำนวณ | `+ - * / %` |
| เปรียบเทียบ | `== != < <= > >=` |
| Boolean | `and or not`, `&& || !` |
| Bitwise | `& | ^ ~ << >>` |
| Conversion | `as Type` |

คูณ/หารมาก่อนบวก/ลบ จากนั้นเปรียบเทียบ แล้ว boolean ใช้วงเล็บเมื่อต้องการให้ลำดับชัดเจน `%` และ bitwise ใช้ integer เงื่อนไขต้องเป็น `bool` ไม่ถือว่าเลข 0/1 เป็น boolean

```dev
let score = 42
if score > 0 and score < 100 {
    print("อยู่ในช่วง")
} else if score == 100 {
    print("เต็ม")
} else {
    print("นอกช่วง")
}
```

`and/or` short-circuit ไม่ประมวลผล operand ที่ไม่จำเป็น ระวัง C evaluation order ใน native: ถ้าหลาย function calls มี side effects และต้องเรียงลำดับ ให้แยกเป็น statements

## ตัวอย่างที่รันและดาวน์โหลดได้

- [if / else if / else](#/examples/conditions) — ทุกแขนงของเงื่อนไข รวม nested if และ early return
- [&& / || / and / or / ! / not](#/examples/logical) — เปรียบเทียบสองรูปแบบ จัดกลุ่ม และพิสูจน์ short-circuit ด้วยฟังก์ชันที่พิมพ์ข้อความ
- [คำนวณ เปรียบเทียบ และ bitwise](#/examples/operators) — Arithmetic, comparisons, shifts, bitwise, compound assignment และ sizeof

# [loops] while, for, break และ continue

## while

ใช้เมื่อจำนวนรอบขึ้นกับเงื่อนไข ตรวจ condition ก่อนเข้ารอบทุกครั้ง

```dev
let i = 0
while i < 3 {
    print(i)
    i += 1
}
```

## Range for

```dev
let total = 0
for i in 0..10 {
    if i == 3 { continue }
    if i == 8 { break }
    total += i
}
print(total)
```

`0..10` รวม 0 แต่ไม่รวม 10 Bounds ต้องเป็น `i64` และประเมินค่าแต่ละครั้งเพียงครั้งเดียวก่อนเริ่ม loop ช่วงว่างหรือย้อนกลับทำงาน 0 รอบ เปลี่ยนค่า iteration variable ไม่เปลี่ยน internal counter

`continue` ข้ามส่วนที่เหลือในรอบนั้น `break` ออกจาก loop ชั้นปัจจุบัน รองรับ nested loops และ return ภายใน loop ยังไม่มี inclusive range, custom step หรือ `for item in collection` ให้ใช้ index กับ `.len()`

## ตัวอย่างที่รันและดาวน์โหลดได้

- [for / while / break / continue](#/examples/loops) — End-exclusive range, nested loops และวน Vec ด้วย index

# [functions] Functions และ return

## ประกาศและเรียก

```dev
fn add(a i64, b i64) i64 {
    return a + b
}
fn greet(message str) {
    print(message)
}
greet("Hello")
print(add(19, 23))
```

Parameter ต้องระบุ type Return type เขียนหลัง `)` ไม่ต้องมี `->` แต่ spelling แบบมี `:` และ `->` ก็ยอมรับ ฟังก์ชันที่คืนค่าต้อง return ครบทุก path; checker ไม่ถือว่า loop จะ return แน่นอน

## Recursion

```dev
fn factorial(n i64) i64 {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}
print(factorial(5))
```

รองรับ forward declaration และ recursion แต่ runtime มี call-depth limit อย่าใช้ recursion ลึกโดยไม่จำเป็น ยังไม่มี default parameters หรือ overload dispatch

## ตัวอย่างที่รันและดาวน์โหลดได้

- [ฟังก์ชัน recursion และ exit status](#/examples/functions) — Typed parameters, return, factorial, optional :/-> และ return main()

# [strings-arrays] Strings และ fixed arrays

## String อ่านเป็น bytes

```dev
let text = "Dev ภาษาไทย"
print(text)
print(text[0])
let values [i64; 3] = [19, 23, 7]
values[2] = 0
print(values[0] + values[1])
print(sizeof(values))
```

`str[index]` คืน UTF-8 byte (`u8`) ไม่ใช่ Unicode character และแก้ byte ของ string ไม่ได้ String equality ด้วย `==` ไม่รองรับ ให้ใช้ C `strcmp` ผ่าน FFI เมื่อจำเป็น หรือออกแบบข้อมูลเป็นชนิดอื่น

Runtime เป็นเจ้าของ string bytes ส่วน native `str` ยืม C storage ต้องรักษา lifetime ถ้าใช้ค่าจาก library โดยเฉพาะ pointer ที่คืนจาก buffer ชั่วคราว `\0` ทำให้การพิมพ์แบบ C หยุดก่อนข้อความท้าย

## ข้อจำกัดของ array

Fixed array ใช้ literal initializer; native ไม่รองรับ array ทั้งก้อนเป็น parameter/return/value copy ไม่รองรับ nested local arrays ถ้าต้องส่งเป็นค่าให้ห่อด้วย struct หรือใช้ `Vec<T>` ถ้าจะใช้ pointer + length ต้องมี unsafe และรักษา bounds/lifetime เอง

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Arrays และ UTF-8 bytes](#/examples/arrays) — Fixed arrays, index mutation, sizeof, escapes และ string byte indexing

# [modules] Modules และ export

## แยกหลายไฟล์

ไฟล์ `math.dev`:

```dev-module
fn add(a i64, b i64) i64 { return a + b }
```

ไฟล์ `main.dev`:

```dev-module
use "math.dev" as math
fn main() { print(math.add(19, 23)) }
main()
```

รัน `d main.dev` ได้โดยไม่ build Dev program ก่อน `use math` จะหา `math.dev` ข้างไฟล์ที่ import ส่วน `use "../lib/math.dev" as math` ระบุ path/alias ได้ Path canonical เป็น module identity ไฟล์เดียวโหลดครั้งเดียว รองรับ cyclic imports ของ declarations

## Export ไม่ได้แปลว่า public ใน Dev

ฟังก์ชันของ module เรียกได้จาก direct importer อยู่แล้ว ไม่ต้อง `export` Imports ไม่ re-export ต่อโดยอัตโนมัติ ชื่อฟังก์ชัน Dev ถูกแยก C symbol ตาม module

```dev
export fn device_add(a i32, b i32) i32 { return a + b }
print(device_add(19, 23))
```

`export fn` รักษาชื่อ C symbol ที่ประกาศ เพื่อให้ C/linker เรียกได้ ชื่อ export ต้องไม่ชนกันทั้งโปรแกรม ใช้เมื่อต้องเปิด C API ไม่ต้องใส่ทุก function

## ตัวอย่างที่รันและดาวน์โหลดได้

- [หลายไฟล์ หนึ่งโปรแกรม](#/examples/modules) — use แบบชื่อ module, relative path และ alias พร้อมไฟล์ประกอบ
- [export และ C symbol](#/examples/exports) — export fn ใช้รักษาชื่อ C symbol และเรียกจาก Dev ได้ตามปกติ

# [structs] Structs และ methods

## ข้อมูลแบบมี fields

```dev
struct Point { x i64; y i64 }
fn Point.sum(self Point) i64 { return self.x + self.y }
let p = Point(19, 23)
print(p.sum())
p.x += 1
print(p.x)
```

Constructor เป็น positional ตามลำดับ fields ต้องส่งครบ ยังไม่มี named-field constructor Struct มี nominal identity: type คนละ module แม้ fields เหมือนกันก็เป็นคนละ type รองรับ nested struct, enums และ array fields

Struct assignment/parameter/return เป็น value semantics Managed fields คง reference count ของ storage ที่ใช้ร่วมกัน

## Receiver ของ method

`fn Type.method(self Type, ...) R` รับ receiver เป็น **value copy** การแก้ self ไม่แก้ object ต้นทางโดยอัตโนมัติ ให้คืนค่าที่แก้แล้วและ assign กลับ ไม่มี implicit mutable-self borrowing, inheritance หรือ method overloading

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Structs & generics](#/examples/features) — หลายชนิดข้อมูลร่วมกับ imported module
- [Structs และ methods](#/examples/structs) — Positional constructors, nested structs, value copies และ methods ที่คืนค่าใหม่

# [enums-match] Enums, payload และ match

## Enum ไม่มี payload

```dev
enum Color { Red, Green, Blue }
let color = Color.Green
print(color)
print(color == Color.Green)
print(color as i32)
```

Variants เริ่มเลขจาก 0 รองรับ equality และ explicit cast เป็น integer ไม่รองรับ integer-to-enum/custom discriminants

## Enum มี payload

```dev
enum Result<T> { Ok(T), Err(str) }
let result = Result<i64>.Ok(42)
match result {
    Ok(value) => { print(value) }
    Err(message) => { print(message) }
}
```

ถ้า enum มี payload ใน variant ใด จะไม่รองรับ equality/integer cast ให้อ่านผ่าน match `match` เป็น statement ไม่ใช่ expression ทุก variant ต้องมี unguarded irrefutable arm หรือมี wildcard `_` สุดท้าย

## Guard และ nested pattern

```dev
enum Inner { Empty, Value(i64) }
enum Outer { Some(Inner), None }
match Outer.Some(Inner.Value(42)) {
    Some(Value(n)) if n > 0 => { print(n) }
    _ => { print(0) }
}
```

Guard ใช้ `if` หลัง pattern Pattern bindings อยู่ใน arm scope และเป็น value copies Nested pattern ที่ match ไม่ครบต้องมี fallback ยังไม่มี literal/struct/slice patterns

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Enums และ payload](#/examples/enums) — Payload-free equality/cast และ generic Ok/Err แบบ exhaustive match
- [Match: guard, wildcard, nested](#/examples/matching) — Nested destructuring พร้อม guard และ fallback สำหรับทุกกรณี

# [references] Ref และ recursive data

## Immutable managed reference

```dev
let owner = ref(42)
let copy = owner
print(deref(copy))
```

`ref(value)` เก็บ value copy ใน managed storage และคืน `Ref<T>` ส่วน `deref` คืน value copy ไม่มี null Ref, writable dereference, manual free, pointer arithmetic หรือ Ref-to-pointer cast Last-owner cleanup คืน managed storage

## Recursive layout

```dev
enum List<T> { Nil, Cons(T, Ref<List<T>>) }
let list = List<i64>.Cons(42, ref(List<i64>.Nil()))
match list {
    Nil => {}
    Cons(value, next) => { print(value) }
}
```

Recursive type ต้องผ่าน `Ref<T>`, raw pointer หรือ managed collection เพราะ layout แบบ `struct Node { next Node }` มีขนาดไม่สิ้นสุด จึงเป็น error รองรับ mutual recursion ที่มี indirect edge

Reference counting ไม่ใช่ tracing GC หรือ borrow checker การห่อ native `str`/foreign pointer ใน Ref ไม่ได้ยืด lifetime ของ C memory นั้น

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Recursive list](#/examples/payload) — Payload enums, Ref และ recursive list
- [Ref และ recursive struct](#/examples/references) — Immutable snapshot, deref และ struct/enum ที่ recursive ผ่าน Ref

# [collections] Vec, Slice และ Map

## Vec และ snapshot

```dev
let values = Vec<i64>(19, 23)
let snapshot = values.slice(0, 2)
values[0] = 99
print(snapshot[0] + snapshot[1])
values.push(7)
print(values.pop())
print(values.len())
```

ผล snapshot ยังเป็น 42 เพราะ Vec/Slice ใช้ copy-on-write Shared storage ถูก copy เมื่อแก้ค่า Slice เป็น read-only snapshot `.slice(start,end)` ไม่รวม end Vec รองรับ `push/pop/len/clear` และ indexed read/write `pop` บน Vec ว่างเป็น error

## Map

```dev
let scores = Map<str, i64>()
scores.set("answer", 42)
if scores.contains("answer") { print(scores.get("answer")) }
print(scores.remove("answer"))
```

Methods คือ `set/get/contains/remove/len/clear` ถ้า get key ที่ไม่มีจะเกิด error Keys เป็น numeric/bool/str หรือ payload-free enum Values เป็น non-array values ไม่มี collection iterator ในปัจจุบัน

Hash lookup/update/remove คาดหวัง O(1) แต่ collision อาจเป็น O(n) และ mutation ของ shared COW storage ต้อง copy O(n) Hash deterministic และไม่ป้องกัน adversarial keys Native string keys ต้องคง bytes และ lifetime ตลอดที่อยู่ใน Map

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Managed collections](#/examples/advanced) — รวม COW, traits, closures, match และ async
- [Vec ทุก method และ COW](#/examples/vector) — push, pop, len, clear, index read/write และสำเนาที่ไม่เปลี่ยนตาม
- [Slice และ snapshot](#/examples/slices) — slice(start,end), len, nested slice และผลหลังแก้ Vec ต้นทาง
- [Map ทุก method และ COW](#/examples/maps) — set, get, contains, remove, len, clear และ snapshot

# [generics] Generics และ inference

## Function และ type arguments

```dev
fn identity<T>(value T) T { return value }
struct Box<T> { value T }
print(identity(42))
print(identity<str>("Dev"))
let boxed = Box<i64>(42)
print(boxed.value)
```

อนุมาน type จาก arguments, payload และ expected type ได้ เช่น `let n i32 = identity(42)` แต่กรณีกำกวมต้องระบุเอง เช่น `Result<i64>.Err("bad")` รองรับ imported generic ด้วย `module.function<T>(...)`

Frontend specialize แต่ละ combination Native ได้ concrete C type/function ส่วน runtime ใช้ concrete function ภายใน engine โดยไม่เรียก native compilerสำหรับ pure Dev

## Constraints

```dev
fn sum<T:Number>(a T, b T) T { return a + b }
trait HasValue { fn value(self Self) i64 }
struct Point { x i64 }
fn Point.value(self Point) i64 { return self.x }
fn read<T:HasValue>(p T) i64 { return p.value() }
print(sum(19, 23))
print(read(Point(42)))
```

Built-in bounds คือ `Number`, `Integer`, `Equatable` รวมหลาย bounds ด้วย `+` User traits ตรวจ method signatures ของ concrete type แบบ static ไม่มี trait objects, associated types, default methods หรือ blanket impls

## Const generics

```dev
struct Buffer<T, const N> { data [T; N] }
let buffer = Buffer<i64, 2>([19, 23])
print(buffer.data[0] + buffer.data[1])
```

Const argument เป็น nonnegative integer ใช้เป็น array length/numeric value ได้ Array length ต้อง 1..1,000,000 ยังไม่มี const expressions, type-level arithmetic หรือ default generic arguments Generic function ไม่เป็น extern/export ต้องสร้าง concrete wrapper

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Generics, inference, constraints, const](#/examples/generics) — Function/struct generics, Number, Equatable และ Buffer<T,const N>
- [User traits และ generic bounds](#/examples/traits) — กำหนด method contract แล้วเรียกผ่าน T:HasValue

# [closures] Function values และ closures

```dev
fn make(offset i64) fn(i64) i64 {
    return fn(value i64) i64 { return offset + value }
}
let add = make(19)
print(add(23))
```

Function type คือ `fn(T1, T2) R` และ `fn() R` ฟังก์ชัน concrete ใช้เป็น value, parameter, return หรือเก็บใน collection ได้ Generic function ต้องห่อให้เป็น concrete ก่อนใช้เป็น value

Closure capture ตัวแปรที่ใช้จาก outer scope **โดย value ณ ตอนสร้าง** Managed values retain storage การแก้ capture local เปลี่ยนแค่ copy ของ invocation ไม่แก้ outer และไม่ค้างไป invocation ถัดไป

Fixed arrays ให้ capture ผ่าน struct/Vec แทน Closure/function body ต้องมี unsafe ของตัวเอง ไม่ได้สิทธิ์ unsafe จากตำแหน่งที่สร้างหรือเรียก

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Function values และ closures](#/examples/closures) — ส่งฟังก์ชันเป็น parameter, คืน closure และ capture-by-value ที่ไม่สะสมการแก้ไข

# [tasks] Threads, async และ continuations

## เริ่มงาน

```dev
async fn answer(n i64) i64 { return n + 1 }
let job = answer(41)
let other = spawn(fn() i64 { return 19 + 23 })
print(await(job))
print(ready(job))
print(await(other))
```

`async fn` และ `spawn(fn() T)` สร้างงานบน OS thread และคืน `Task<T>` `await(task)` block thread ที่รอจนได้ value copy และ await ซ้ำได้ `ready(task)` ตรวจเสร็จโดยไม่ block

## ต่อขั้นตอนด้วย then

```dev
let job = spawn(fn() i64 { return 19 })
let next = then(job, fn(n i64) Ref<i64> { return ref(n + 23) })
print(deref(await(next)))
```

`then` คืน `Task<R>` ทันทีและประเมิน arguments ครั้งเดียว แต่ worker อีก thread ยังรอ input อยู่ ไม่ใช่ coroutine scheduler/thread pool หรือ async I/O สำหรับ Task<void> continuation ใช้ `fn() R`

## Lifetime และข้อจำกัด

Await งานที่จำเป็นก่อน process จบ Dropping last handle จะ detach งาน แต่ detached threads ไม่ทำให้โปรแกรมอยู่ต่อ Runtime task errors ต้อง observe ด้วย await ไม่มี cancellation, channels, user mutex API หรือ automatic parallel loops ใช้งานเป็น coarse tasks เพื่อลด thread setup overhead

## ตัวอย่างที่รันและดาวน์โหลดได้

- [async / spawn / await / ready / then](#/examples/tasks) — รันงานบน thread, รอซ้ำ, ต่อ continuation และ Task<void>

# [safety] unsafe และความปลอดภัย

## เมื่อใดต้องใช้ unsafe

Raw address-of/dereference/casts/arithmetic/indexing, volatile access, extern calls และการสร้าง C callbacks ต้องอยู่ใน lexical `unsafe { ... }` ฟังก์ชันที่เรียกยังต้องมี unsafe ใน body ของตัวเอง

```dev
fn increment(n i64) i64 { return n + 1 }
unsafe {
    let cb = callback(increment)
    print(cb(41))
}
```

## สิ่งที่ตรวจ

ทั้ง runtime/native ตรวจ fixed-array/Vec/Slice/string bounds, integer division/remainder by zero และ shift count Native raw dereference/volatile access ตรวจ null/alignment Runtime ตรวจ address overflow เพิ่มด้วย Checks ยังอยู่ใน optimized native build

## สิ่งที่ยังเป็นหน้าที่ผู้เขียน

ไม่มี borrow checker และพิสูจน์ allocation bounds/lifetime ของ arbitrary foreign pointer ไม่ได้ ต้องรักษา C ABI, buffer sizes, ownership และ synchronization เอง Unsafe ไม่ใช่ sandbox ผิด ABI หรือใช้ pointer หมดอายุทำให้ process crash ได้

Native managed storage ใช้ reference counting แต่ native `str`/raw pointers เป็น borrowed C memory การ copy struct/Ref ที่มี pointer ไม่ได้เป็นเจ้าของ memory นั้น

## ตัวอย่างที่รันและดาวน์โหลดได้

- [C allocation, pointers และ volatile](#/examples/foreign_memory) — malloc/free, null guard, indexing, pointer arithmetic และ volatile บน allocation ที่ยังมีชีวิต
- [ตรวจขอบเขตและค่าก่อนใช้งาน](#/examples/safe_checks) — ป้องกัน index ผิด, หารศูนย์ และ Map key ที่ไม่มี

# [ffi] เชื่อมต่อ C และ native libraries

## Declaration และ ABI

```dev-ffi
extern fn puts(message str) i32
unsafe { puts("Calling C directly") }
```

`extern fn` บอก signature ของ C symbol ต้องตรงชนิด ขนาด return และ calling convention จริง Runtime เรียกผ่าน libffi ส่วน native linker เชื่อม implementations ตอน build

## โหลด library และ C source

```sh
d examples/ffi/main.dev
d main.dev --ffi-lib ./device.dll
d build main.dev --link device.c --release
```

Runtime ค้นหา shared libraries ข้าง module ที่ประกาศ extern และเตรียม C sources ที่พบด้วย sibling `devc` โดยอัตโนมัติ ครั้งแรกต้องมี C backend; cache อุ่นไม่จำเป็นต้อง build ซ้ำ `--ffi-lib` ใช้เลือก DLL/.so ชัดเจน

## Variadic และ struct ABI

```dev-ffi
extern fn printf(format str, ...) i32
unsafe { printf("value=%d\n", 42 as i32) }
```

Variadic ต้องมี fixed parameter อย่างน้อยหนึ่งตัว Extra bool/small integers promote เป็น i32, f32 เป็น f64 ต้องให้ format string ตรง widths รองรับ C-compatible scalar/struct ABI ไม่รองรับ managed aggregates, payload unions, packed structs, bitfields หรือ custom convention ให้ใช้ C wrapper

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Variadic C calls](#/examples/variadic) — printf พร้อม format และชนิด argument ที่ตรงกับ C ABI
- [C source FFI แบบรันทันที](#/examples/ffi) — เรียก puts และ device_add จาก C source ข้างไฟล์ Dev
- [Struct ABI และส่ง callback ให้ C](#/examples/aggregate) — รับส่ง C-compatible Pair แบบ by-value และให้ C เรียก scalar callback

# [callbacks] C callbacks และ userdata

## Raw callback

```dev
fn increment(value i64) i64 { return value + 1 }
unsafe {
    let pointer = callback(increment)
    print(pointer(41))
}
```

ใช้ type `callback(T1,T2) R` สำหรับ C function pointer รองรับ C scalar arguments และ non-str scalar/void return Native capturing callbacks จำกัด 64 unique function/environment pairs ต่อ signature ต่อ originating module และ retain capture จน process จบ

## Managed context

```dev
fn make(offset i64) fn(i64) i64 {
    return fn(value i64) i64 { return offset + value }
}
unsafe {
    let cb = callback_context(make(19))
    print(cb.call(23, cb.data))
}
```

`ContextCallback<fn(T) R>` มี `.call`, `.data`, `.owner` และ C signature วาง userdata **ตัวสุดท้าย** ส่ง call/data แยกให้ C API เก็บ record/owner ไว้จน unregister และทุก invocation เสร็จ ห้ามสลับ fields จากคนละ pair

Native path นี้ไม่จำกัด 64 instances เพราะ userdata เลือก environment และคืน managed captures เมื่อ last owner หาย Runtime เก็บ trampoline code/weak control blocks จน engine จบ แต่ปล่อย captures ได้ก่อน Runtime diagnose owner หมดอายุ แต่ native userdata ยังต้องรักษา unsafe lifetime contract

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Tasks & callbacks](#/examples/continuations) — then, ready และ callback_context ที่เก็บ ownership
- [Raw / captured / context callbacks](#/examples/callbacks) — callback(fn), capturing callback และ call/data/owner ใน unsafe

# [hardware] Raw pointers และ hardware

## Native memory access

```dev-native
struct Node { value i64; next *Node }
let first = Node(42, null)
unsafe {
    let second = Node(0, &first)
    print((*second.next).value)
}
```

Address-of Dev locals เป็น native-only Runtime เข้าถึง live foreign pointers จาก native library ได้ ใช้ `Ref<T>` เมื่อข้อมูลอยู่ใน pure Dev และต้องใช้ได้ทั้งสอง modes

`volatile_load(pointer)` และ `volatile_store(pointer,value)` เป็น volatile numeric/bool C access ไม่ใช่ atomic, CPU fence, DMA cache maintenance หรือ platform driver อย่าอ่าน hardware address จาก process ทั่วไปโดยไม่มีสิทธิ์/driver ที่ถูกต้อง

## Freestanding

```sh
d build firmware.dev --freestanding --lib --cc clang
```

Target ต้องเตรียม startup, linker script, memory/compiler support routines และ drivers เอง `print` และ managed allocation ใช้ hosted environment ไม่ใช่ทุก feature จะเหมาะกับ firmware ตัวอย่าง ARM ถูกตรวจรูปแบบ ELF แต่ไม่ได้ยืนยันบนอุปกรณ์จริง

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Native: address-of และ volatile register](#/examples/native-memory) — ใช้ &local, raw pointers, pointer arithmetic และ volatile กับ local memory

# [stdlib] Standard library แบบแยกส่วน

Stdlib เป็น native C library แยกจาก source runtime มี bindings ใน `stdlib/modules` และ archive `libdevruntime.a` Namespace `std` เป็นชื่อที่ผู้ใช้กำหนด ไม่ใช่ keyword ของภาษา

## Import และ link

```dev-module
use "std/io"
fn main() {
    io.writeln("Hello from stdlib")
}
main()
```

```sh
d build examples/stdlib/main.dev --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a
```

คำสั่งนี้ใช้จาก repository/installation root ที่มี paths เหล่านี้ เมื่อเปลี่ยน cwd ต้องเปลี่ยน paths ให้ตรง คำสั่ง `d` ใน PATH ไม่ได้ทำให้ stdlib namespace/link อัตโนมัติ

## Modules ที่มี

| Module | งาน |
| --- | --- |
| `memory` | allocation, resize, copy, fill, free |
| `strings` | owned string handles, append, len, view, free |
| `io` | console, files, binary read/write, close |
| `time` | monotonic clock และ sleep |

Bindings มี unsafe อยู่ใน function ที่เรียก C ผู้ใช้ยังต้องตรวจ null/byte counts และคืน opaque handles ตาม API ระวัง borrowed string view หลัง free/resize อ่าน contracts ฉบับเต็มใน Stdlib reference

## ตัวอย่างที่รันและดาวน์โหลดได้

- [Runtime: console, strings, time, args](#/examples/runtime_io) — write/writeln, concat/equal/len, monotonic time, sleep และ arguments หลัง --
- [Runtime: อ่านและเขียนไฟล์](#/examples/runtime_files) — write_file/read_file สำหรับ UTF-8 และตรวจข้อความที่อ่านกลับ
- [Runtime: รับข้อมูลจาก keyboard](#/examples/runtime_input) — read_line รับหนึ่งบรรทัดแล้วทักทายผู้ใช้
- [Native stdlib: string ownership และ I/O](#/examples/native-stdlib) — สร้าง/append/view/free string, I/O, time และจัดการ allocation failure

# [cli] CLI และ options

## Runtime selectors

```sh
d main.dev
d run main.dev
d --run main.dev
d -r main.dev
```

## Compiler selectors

```sh
d build main.dev
d --build main.dev
d -b main.dev
d compiler main.dev
d --compiler main.dev
d -c main.dev
```

Selectors อยู่ argument แรก Compiler options ใส่หลัง source filename Runtime ปฏิเสธ compiler-only options อย่าสับสน `d run` ที่ใช้ source runtime กับ `devc run` ที่ compile แล้วรัน

## Options สำคัญ

| Option | ผล |
| --- | --- |
| `--help`, `--version` | ช่วยเหลือ/เวอร์ชัน component |
| `--cwd DIR`, `-C DIR` | working directory |
| `--eval CODE`, `-e CODE` | รัน inline source |
| `--engine auto\|ast` | numeric plans + fallback หรือ AST |
| `--timings` | เวลาไป stderr |
| `--ffi-lib PATH` | โหลด shared library ใน runtime |
| `--output PATH`, `-o PATH` | output build/emission |
| `--release`, `--debug` | C backend O3/O0 |
| `--native` | optimize ตาม CPU ปัจจุบัน อาจไม่ portable |
| `--fast` | TinyCC basic unoptimized development builds |
| `--module-dir NAME=DIR`, `--link PATH` | namespace และ native dependencies |
| `-- ARGUMENTS` | arguments ของโปรแกรม |

`d check main.dev` ตรวจ frontend โดยไม่ compile native; `d emit main.dev` สร้าง C source การ check ผ่านไม่ได้ยืนยันว่า symbol/linker หรือ foreign ABI ถูกต้อง

# [limitations] ข้อจำกัดและแนวทางเลือกใช้

## เลือก runtime เมื่อใด

เหมาะกับลองโค้ด สคริปต์ และ workflow ที่ไม่อยาก build pure Dev Numeric plans ช่วยงานที่รองรับ แต่ไม่มี JIT และไม่อ้างว่าจะเท่า native งาน recursion ลึก/foreign callbacks มี interpreter overhead

## เลือก native เมื่อใด

เหมาะกับ executable, native tooling, C integration และคำนวณที่ต้องการ optimized machine code ให้ benchmark workload จริงบน hardware/config เดียวกัน เปรียบเทียบเวลา build และเวลารันแยกกัน

## สิ่งที่ยังไม่มี

Source ล่าสุดมี package manager แบบ path/Git, formatter, LSP เบื้องต้น และ native debugger integration แล้ว ดู [เครื่องมือพัฒนา](#/docs/tooling) รองรับ workspace และ SemVer Git tags แล้ว ส่วน registry/global version solver, workspace type checking และ interpreter debugger ยังไม่มี เช่นเดียวกับ coroutine event loop, async I/O, trait objects, literal/struct/slice patterns, inclusive/custom-step ranges และ iterator for collections

Native `str` เป็น borrowed storage และ arbitrary foreign pointers ไม่ได้รับ lifetime proof Shared collection mutation อาจ copy O(n) Capturing raw C callbacks มี slot/lifetime limits ให้เลือก managed userdata API เมื่อเหมาะสม

Compiler/runtime มี implementation limits เช่น module/file sizes และ generic specialization depth ภาษาอยู่ในช่วงพัฒนา ตรวจ source/docs และทดสอบ path จริงก่อนใช้งาน production

# [ai] ใช้ DevLang กับ AI

## Skill ที่ดาวน์โหลดได้

หน้า AI skill มี ZIP แบบ self-contained พร้อม `SKILL.md`, reference ที่จำเป็น และตัวอย่างจริง ใช้กับ Codex หรือ AI coding agent ที่อ่าน skill folders ได้ AI tools อื่นสามารถอ่าน SKILL.md และ references เป็น context ได้

แตก ZIP แล้ววางโฟลเดอร์ `devlang` ใน `~/.codex/skills/` ของเครื่องที่ใช้ Codex จากนั้นเริ่ม session ใหม่เพื่อให้ค้นพบ skill

## ตัวอย่าง prompt

```text
ใช้ $devlang สร้างโปรแกรม DevLang คำนวณผลรวมด้วย Vec
ให้รันด้วย source runtime และตรวจ native build ด้วย
```

Skill เน้น syntax จริง, explicit main invocation, การแยก runtime/compiler, value/COW semantics, unsafe C lifetimes และตรวจโค้ดก่อนสรุป ไม่ใช่ compiler และไม่ได้เพิ่ม feature ที่ภาษาไม่รองรับ

มีไฟล์ `llms.txt` และคู่มือ Markdown ให้ AI อ่านโดยไม่ต้อง scrape หน้าจอ เว็บนี้แสดงตัวอย่างพร้อมผลลัพธ์ที่ตรวจไว้ **ไม่ได้รัน DevLang ใน browser** ให้ดาวน์โหลดไฟล์และรันด้วย `d` ในเครื่อง

# [json] JSON: อ่าน สร้าง และแก้ไขข้อมูล

`use "std/json"` ให้ source runtime อ่านและสร้าง JSON ได้โดยไม่ต้องใช้ C library รองรับ object, array, string, number, bool และ null รวมข้อมูลซ้อนกัน

**API ใหม่จาก source ล่าสุด**: ต้อง build ด้วย `cargo build --release -p dev-runtime -p dev-cli` ตัวติดตั้ง v0.4.0 เดิมยังไม่มี API นี้ Native compiler ยังไม่รองรับ module นี้

## Object literal และ user.name

```dev-runtime
use "std/json"
fn main() {
    let user = {
        name: "Dev",
        age: 18,
        address: { city: "Bangkok" },
        tags: ["developer", 42, true, null],
    }
    let original = user
    user.age += 1
    user.address.city = "Chiang Mai"
    user.active = true
    print(user.name)
    print(user["age"])
    print(json.stringify(user))
    print(original.age)
}
main()
```

สร้าง object ได้โดยไม่ต้อง import; `use "std/json"` จำเป็นเมื่อเรียก JSON API หรือระบุ `json.Value` รองรับ key แบบ `name:` และ `"display-name":` โดย key ที่มีเครื่องหมายใช้ `user["display-name"]` รองรับ `{}`, comma ท้ายรายการ และ object/array ซ้อนกัน array ภายใน literal หรือที่กำหนดให้ JSON member ใช้ชนิดผสมและ `[]` ได้ ส่วน array ปกติใช้กฎเดิม key ซ้ำใน literal เป็น error

อ่านผ่าน dot/bracket จะได้ scalar ของ Dev โดยตรง เช่น str, bool, i64/u64, f64 และ `null` ส่วน object/array ยังคงเป็น JSON ตัวเลขที่เกินช่วงเก็บเป็น JSON เพื่อรักษาความแม่นยำ `json.get`/`json.at` ยังคงคืน JSON เสมอ การเข้าถึงแบบนี้ใช้กับผล `json.parse` ได้ด้วย

`user.newKey = value` เพิ่ม key หรือเปลี่ยนชนิดได้ `+=` ต้องมีค่า scalar ที่เหมาะสมอยู่แล้ว parent ซ้อนกันต้องมีอยู่ และ index ต้องอยู่ในขอบเขต index expression ประเมินครั้งเดียว สำเนา `original` ไม่เปลี่ยนตาม การแก้ค่าคัดลอกต้นไม้ JSON จึงมีต้นทุนตามขนาดข้อมูล การกำหนดค่าต้องมี local JSON เป็น root; ถ้าเก็บใน struct/Vec ให้ดึงออกมาแก้แล้วใส่กลับ

## Parse และอ่านค่า

```dev-runtime
use "std/json"
fn main() {
    let data = json.parse("{\"name\":\"Dev\",\"age\":18}")
    print(json.string(json.get(data, "name")))
    print(json.int(json.get(data, "age")))
    data = json.set(data, "age", 21)
    print(json.stringify(data))
}
main()
```

รันด้วย `./target/release/d.exe main.dev` บน Windows หรือ `./target/release/d main.dev` บน Linux ฟังก์ชันอ่านชนิดข้อมูลจะตรวจชนิดจริง ไม่แปลง string เป็น number ให้อัตโนมัติ

## Object, array และข้อมูลซ้อนกัน

| งาน | API |
| --- | --- |
| ตรวจข้อความก่อน parse | `json.valid(text)` |
| สร้าง object / array / null | `json.object()`, `json.array()`, `json.null_value()` |
| อ่าน member / ตรวจว่ามี key | `json.get(value,key)`, `json.has(value,key)` |
| อ่าน array / จำนวนสมาชิก | `json.at(value,index)`, `json.len(value)` |
| ตรวจชนิด / null | `json.kind(value)`, `json.is_null(value)` |
| อ่าน scalar | `json.string`, `json.bool`, `json.int`, `json.uint`, `json.float` |
| เพิ่มหรือแก้ member / ลบ member | `json.set(value,key,item)`, `json.remove(value,key)` |
| เพิ่ม array element | `json.push(value,item)` |
| ดึงชื่อ keys เป็น Vec | `json.keys(value)` |
| แปลงข้อมูล Dev เป็น JSON | `json.value(value)` |
| สร้างข้อความ compact / จัดย่อหน้า | `json.stringify(value)`, `json.pretty(value)` |

ค่าที่คืนจาก parse/get/at เป็น `json.Value` ใช้เป็น parameter, return type และ element ของ Vec ได้ การแก้ไขคืนค่าใหม่เสมอ ต้องเขียน `data = json.set(...)` หรือ `items = json.push(...)` สำเนาเดิมไม่เปลี่ยน หากแก้ child object ต้องนำ child ที่แก้แล้ว set กลับเข้า parent

`value`, `stringify`, `pretty`, `set` และ `push` รับ scalar, array/Vec/Slice, struct, Ref และ Map ที่ใช้ str key ได้ ไม่มีการแปลง JSON กลับเป็น struct อัตโนมัติ ให้อ่านและตรวจ fields ก่อนสร้าง struct เอง

## อ่านและเขียนไฟล์

ใช้ `io.read_file(path)` แล้ว `json.parse(text)` สำหรับอ่าน และ `io.write_file(path,json.pretty(data))` สำหรับเขียน JSON การเขียนไฟล์อาจเขียนทับไฟล์เดิม

JSON ที่ผิดรูปแบบ, key ที่ไม่มี, index เกินขอบเขต และชนิดข้อมูลผิดเป็น runtime error ที่ระบุ source location ไม่มี try/catch ใช้ `valid`, `has`, `kind`, `len` ตรวจตามความต้องการก่อนเข้าถึง

Unicode/escapes รองรับ UTF-8 และ surrogate pairs ตัวเลข JSON ที่ใหญ่มากคงความแม่นยำใน parse/stringify ส่วน `int/uint/float` จำกัดตาม i64/u64/f64 ตามลำดับ ไม่ serialize NaN/Infinity การซ้อนข้อมูลมีขีดจำกัด และการแก้ไข/ดึง subtree อาจต้อง copy ข้อมูล

## ตัวอย่างที่รันและดาวน์โหลดได้

- [JSON แบบครบชุด](#/examples/json) — nested data, struct, null, keys, immutable update และ file round-trip
- [JSON API reference](#/docs/json-reference) — signatures, conversion rules, ownership และข้อจำกัด

# [tooling] เครื่องมือพัฒนา: packages, formatter, LSP และ debugger

เครื่องมือเหล่านี้อยู่ใน source ล่าสุด ต้อง `cargo build --release` และใช้ `d`, `devrun`, `devc` ที่ build ชุดเดียวกัน ตัวติดตั้ง v0.4.0 เดิมยังไม่มี

## สร้าง project และจัดการ dependencies

```sh
d new hello
cd hello
d run
d fmt --check
d build --release -o out/hello
```

`package.don` (หรือ `dev.toml` เดิม) กำหนด package name, entry (ปกติ `src/main.dev`) และ modules directory (ปกติ `src`) เมื่อไม่ระบุไฟล์ `d run/build/check` จะใช้ entry นี้

```toml
[package]
name = "hello"
entry = "src/main.dev"
modules = "src"

[dependencies.math]
path = "../math"
```

Dependency ต้องมี `package.don` หรือ `dev.toml` ของตัวเอง และไฟล์ที่ import มีเฉพาะ declarations ใช้ `use "math/lib"` แล้วเรียก `lib.answer()` ได้ทั้ง runtime/native ไม่ต้อง copy library เข้า source

```sh
d pkg add math --path ../math
# ตัวอย่าง URL ต้องเปลี่ยนเป็น repository library ของคุณ:
d pkg add math --git https://github.com/your-org/math.git --tag v1.0.0
d pkg install
d pkg install --locked
d pkg update
d pkg list
d pkg remove math
```

Commit manifest และ `package-lock.don`; ignore `.dev/` และ `out/` Lockfile เก็บ commit ของ Git และ SHA-256 ของ package files `install` รักษา Git commit เดิมและบันทึก local edits ที่ตั้งใจ `update` resolve Git refs ใหม่ `--locked` ติดตั้ง checkout ที่ขาดด้วย commit เดิมและตรวจเนื้อหาโดยไม่แก้ lock การรัน/build ตรวจ lock แต่ไม่ดาวน์โหลดอัตโนมัติ Git ใช้ HTTPS/file URL และต้องมี Git ติดตั้ง รองรับ transitive dependencies แบบ namespace เดียว ถ้าชื่อชนกันคนละ source จะเป็น error รองรับ workspace และเลือก Git tag ตาม SemVer แล้ว ดู [Workspace และ versions](#/docs/packages) ยังไม่มี registry หรือ publish command

## Formatter

```sh
d fmt
d fmt src examples
d fmt --check src
d fmt --stdout src/main.dev
```

ใช้ indentation 4 spaces, LF และ newline ท้ายไฟล์ รักษา token, ข้อความ, comment และตำแหน่งขึ้นบรรทัดเดิม ไม่ขยาย body บรรทัดเดียวเป็นหลายบรรทัด ตรวจทุก input ก่อนเขียนไฟล์ `--check` ไม่เขียนและคืน exit 1 ถ้าต้องจัดรูปแบบ `--stdout` ต้องมีไฟล์เดียว ค่าเริ่มต้นคือ modules directory ใน project

## Language server และ VS Code

`d lsp --stdio` ตรวจ syntax และ semantic ตามกฎ compiler เช่น ชนิดตัวแปร อาร์กิวเมนต์ return เงื่อนไข ชื่อที่ไม่มี และการประกาศซ้ำ ตรวจข้อความที่ยังไม่บันทึกและไฟล์ import ที่เปิดอยู่ พร้อมอัปเดตไฟล์ที่เกี่ยวข้อง มี unused-variable hints, keyword/type completion และชื่อ function/type ในไฟล์ปัจจุบัน, hover signatures, Go to Definition, Outline และ Format Document ใช้ full document sync และ UTF-16 positions ตาม LSP เก็บข้อผิดพลาด statement ได้หลายข้อ (สูงสุด 100) แต่ parser/การ resolve declaration และ generics ยังหยุดที่ข้อแรก ไฟล์ runtime JSON ยังตรวจเฉพาะ parser/unused ยังไม่มี persistent workspace index, cross-file navigation, local variable definitions หรือ rename

ติดตั้ง [DevLang จาก VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=n-devs.devlang-language) หรือใช้ `code --install-extension n-devs.devlang-language` ตั้งค่า `devlang.executablePath` ให้ชี้ไป `d` ใหม่ เปิด trusted project และ `.dev` Extension source อยู่ใน `editors/vscode` ถ้าจะสร้าง VSIX เอง ใช้ Node.js 22 ขึ้นไป รัน `npm ci`, `npm run check`, `npm run package` ในโฟลเดอร์นั้น แล้วใช้ **Extensions: Install from VSIX**

## Debugger integration

```sh
d debug src/main.dev
d debug src/main.dev --debugger gdb
d debug src/main.dev --no-launch
d debug --vscode
```

สร้าง native `out/debug/app[.exe]` ด้วย O0, DWARF symbols และ frame pointers รองรับ breakpoint ที่บรรทัด `.dev` ผ่าน source mapping ชื่อตัวแปร/function ใน debugger ยังเป็น generated C names และอาจเห็น helper frames Windows Clang ที่ target MSVC ต้องมี LLD เพื่อรักษา DWARF section names; LLDB บาง distribution ต้องมี Python DLL รุ่นที่ตรงกันบน PATH

ติดตั้ง CodeLLDB และ DevLang extension สำหรับ VS Code แล้ว `d debug --vscode` เพื่อสร้าง tasks/launch (ไม่เขียนทับไฟล์เดิม) เปิด entry `.dev` และกด F5 LLDB/GDB เป็น debugger ภายนอก ฟีเจอร์นี้รองรับ native เท่านั้น ยังไม่รองรับ interpreter/JSON runtime debugging

ดู [reference เครื่องมือ](#/docs/tooling-reference) สำหรับข้อจำกัดและการทดสอบ และ [ตัวอย่าง project](https://github.com/d-osc/DevLang/tree/main/examples/tooling)

# [fs-http] Filesystem และ HTTP

Runtime ปัจจุบันอ่านเขียนไฟล์และเรียก HTTP/HTTPS ได้โดยไม่ต้องใช้ C compiler
API ใหม่นี้ยังไม่รองรับ native build ส่วน HTTP client ทำงานแบบ blocking
มี HTTP server แบบ `http.createServer` แล้ว โดยใช้ callback ที่ระบุชนิด `http.IncomingMessage` และ `http.ServerResponse`

```dev-runtime
use "std/fs"
fn main() {
    fs.write_text("hello.txt", "Hello DevLang")
    print(fs.read_text("hello.txt"))
    fs.remove_file("hello.txt")
}
main()
```

ใช้ `http.get(url)` เพื่อรับ response แล้วอ่าน `status`, `ok`, `body`, `bytes` และ `headers`
ใช้ `http.request(method, url, headers, body, timeout_ms)` สำหรับ headers และ timeout
ตัวอย่างออนไลน์: `d examples/http/main.dev -- https://example.com/`

ชื่อ API แบบ Node ใช้ `fs.readFileSync(path, "utf8")`, `writeFileSync(path, text)` และ `mkdirSync(path, recursive)`
ส่วน `use "std/fs/promises"` คืน `Task` เช่น `await(promises.readFile(path, "utf8"))`
Task ใช้ worker thread และ await รอแบบ blocking ยังไม่ใช่ Promise/event loop ของ JavaScript

เริ่ม server ด้วย `d examples/http-server/main.dev -- serve` แล้วเปิด `http://localhost:3000`
`server.listen(3000)` คืนค่าหลัง bind และ runtime รับ request หลังโค้ดหลักจบ
callback ใช้ `res.setHeader`, `writeHead`, `write`, `end` โดยต้อง `end(text)` ก่อน return
ใช้ Ctrl+C เพื่อหยุด ดูข้อจำกัดและ API ทุกตัวในคู่มือเต็มด้านล่าง

ดู [คู่มือ API เต็ม](../../docs/fs-http.md) สำหรับทุกฟังก์ชัน ชนิดข้อมูล ข้อผิดพลาด และข้อจำกัด

# [node-core] Core modules แบบ Node

Runtime ปัจจุบันเพิ่มโมดูลพื้นฐานทั้ง 10 ตัวแล้ว ใช้ `use "std/ชื่อโมดูล"` และรันด้วย `d file.dev` ได้โดยไม่ต้องใช้ C compiler
โมดูลเหล่านี้ยังไม่รองรับ native build และเป็น API แบบ typed ของ DevLang ไม่ใช่ Node compatibility เต็มรูปแบบ

| โมดูล | การใช้งาน |
| --- | --- |
| net | TCP client/server, data/end callbacks, timeout และปิด socket |
| path | join, resolve, normalize, parse/format และแยกชื่อไฟล์ |
| os | platform, arch, hostname, home/temp, memory และ uptime |
| stream | อ่าน/เขียนไฟล์เป็นช่วง และ pipe ไฟล์ใหญ่โดยไม่โหลดทั้งหมด |
| url | WHATWG URL, file URL, IDN และ URLSearchParams |
| module | ตรวจ builtin, resolve path, entry และรายการโมดูลที่โหลด |
| process | cwd, argv, env, pid และเวลาของ runtime |
| events | EventEmitter แบบ synchronous, on/once/off/emit |
| buffer | UTF-8/hex/base64, binary read/write พร้อมตรวจ bounds |
| dgram | UDP4/UDP6, send/recv และ message callback |

```dev-runtime
use "std/path"
use "std/url"
use "std/buffer"
use "std/events"

fn main() {
    print(path.basename("folder/main.dev"))
    print(url.parse("https://example.com/docs").hostname)
    let bytes = buffer.from("DevLang", "utf8")
    print(bytes.toString("hex"))
    bytes.close()
    let emitter = events.createEmitter()
    emitter.once("ready", fn(value str) { print(value) })
    emitter.emit("ready", "ready")
    emitter.close()
}
main()
```

ลอง `d examples/core-modules/main.dev`, `d examples/tcp/main.dev` และ `d examples/udp/main.dev`
ตัวอย่าง TCP/UDP ใช้ loopback และ port ว่างจากระบบ พร้อมปิดทรัพยากรเองหลังส่งข้อมูลเสร็จ
TCP เป็น byte stream ต้องจัดการหลาย chunk ไม่ควรถือว่า callback หนึ่งครั้งคือข้อความหนึ่งชุด

callback รับข้อมูลของ TCP/UDP ทำงานหลังโค้ดหลักจบ ส่วน `read`/`recv` และ file streams เป็น blocking
handle ต้องใช้และปิดใน thread ที่สร้างมัน และ API ไม่มี optional arguments, Promise หรือ timer แบบ JavaScript
อ่าน [signatures และข้อจำกัดครบทุกตัว](../../docs/node-core.md) ก่อนใช้ API ที่ชื่อคล้าย Node

# [don] Dev Object Notation (DON)

DON เป็นรูปแบบข้อมูลคู่กับ DevLang ใช้นามสกุล `.don` และใช้ `package.don` เป็น manifest ของโปรเจกต์ใหม่
รองรับข้อมูลแบบ JSON พร้อม comments, key ไม่ต้องใส่ quotes, `:` หรือ `=`, newline แทน comma และ multiline strings

```don
# package.don
package: {
  name: 'my_app'
  entry: 'src/main.dev'
  modules: 'src'
}
dependencies: {
  math: { path: '../math' }
}
```

ข้อความต้องใส่ single/double quotes, ตัวเลขใช้ `_` คั่นหลักได้ เช่น `1_000`
ชั้นนอกละ `{}` ได้ แต่ nested object ต้องมี braces ไม่รองรับ key ซ้ำหรือการรัน expression

```dev-runtime
use "std/don"
fn main() {
    let data = don.parse("name: 'Dev'\nport = 3_000")
    print(data.name)
    data.port = 8080
    print(data.port)
    print(don.toJSON(data))
}
main()
```

ใช้ `don.stringify(data)` แปลงกลับเป็น DON และ `don.fromJSON(text)` อ่าน JSON แบบเข้มงวด

อ้างอิงค่าจาก root ได้ด้วย `@version` หรือ `@server.port` รองรับ forward references และคงชนิดข้อมูลเดิม

```don
version: 'v1.0.0'
package: { name: 'my_app', entry: 'src/main.dev', modules: 'src' }
dependencies: {
  utils: {
    git: 'https://github.com/example/utils.git'
    tag: @version
  }
}
```

`'@version'` เป็นข้อความธรรมดา ส่วน `tag: version` ยังใช้ไม่ได้ ต้องมี `@`
key ที่ไม่มีอยู่และ reference วนกันจะเกิด error เมื่อ serialize หรือแก้ dependencies จะเขียนค่าที่ resolve แล้ว
field `version` ชั้นนอกเป็น metadata ของ package และจะตรวจเมื่อ dependency ระบุ SemVer requirement ดู [Workspace และ versions](#/docs/packages)

# [basic-libs] ไลบรารีพื้นฐาน: Math, Random, Strings, Datetime, Test และ Log

ชุดนี้ใช้ผ่าน source runtime และมี signatures ให้ Language Server ตรวจชนิดข้อมูล

| โมดูล | ใช้งาน |
| --- | --- |
| `std/math` | sqrt, pow, log, trigonometry, rounding, clamp |
| `std/random` | seed, float, int, bool และ bytes |
| `std/strings` | trim, split/join, replace, casing และ Unicode character access |
| `std/datetime` | Unix milliseconds, RFC3339, calendar และ UTC offsets |
| `std/test` | assertions, named cases และรายงานผล |
| `std/log` | ระดับ log พร้อมเวลา ส่งออก stderr หรือไฟล์ |

```dev-runtime
use "std/math"
use "std/random"
use "std/strings"
use "std/datetime"
use "std/test"
use "std/log"
fn main() {
    print(math.sqrt(9.0))
    print(strings.toUpperCase(strings.trim("  Dev  ")))
    random.seed(42)
    let first = random.int(1, 100)
    random.seed(42)
    print(first == random.int(1, 100))
    print(datetime.iso(datetime.utc(2024, 2, 29, 0, 0, 0, 0)))
    test.case("square root", fn() {
        test.near(math.sqrt(9.0), 3.0, 0.000001, "sqrt")
    })
    let report = test.run()
    print(report.summary)
    log.info("test run finished")
    if report.failed > 0 { return 1 }
    return 0
}
return main()
```

Math ใช้ `f64` และ radians, `random.int(min,max)` ไม่รวม max และ PRNG ไม่เหมาะกับงาน security
`strings.len/indexOf` ใช้ UTF-8 bytes; `charLength/substring/charAt` ใช้ Unicode scalars ไม่ใช่ grapheme clusters
Datetime เป็นเวลาปฏิทิน Unix milliseconds ต่างจาก monotonic `std/time`; offset กรุงเทพคือ 420 นาที
ยังไม่มี timezone แบบ IANA หรือ DST, `test.run()` ต้องส่งต่อ exit status เอง และ log เป็น synchronous

ลอง `d examples/basic-libs/main.dev` และอ่าน [API กับข้อจำกัด](../../docs/basic-libs.md)
ใช้ `std/fs` อ่านเขียนไฟล์ และ `d don check|fmt|to-json|from-json FILE` ตรวจหรือแปลงข้อมูล
คำสั่ง format/convert ส่งออก stdout ไม่แก้ไฟล์ต้นฉบับ แต่ output ไม่เก็บ comments เดิม

`d new` สร้าง `package.don`; lock ใช้ `package-lock.don` ในรูปแบบ DON รองรับการอ่าน `dev.lock` แบบ TOML เดิม และ install ที่สำเร็จจะสร้าง lock ชื่อใหม่โดยเก็บไฟล์เก่าไว้ ถ้ามีทั้งสองไฟล์จะใช้ `package-lock.don` ก่อน
โปรเจกต์ `dev.toml` เดิมยังใช้ได้ API `std/don` รองรับ runtime ยังไม่มี native build
อ่าน [คู่มือ DON และ API](../../docs/don.md) และลอง `d examples/don/main.dev`

# [data-libs] Regex, Encoding, Crypto, Compression, Archive และ UUID

ใช้ผ่าน source runtime ด้วย `use "std/ชื่อโมดูล"` และมี signatures ให้ Language Server ตรวจชนิดข้อมูล

| โมดูล | ใช้งาน |
| --- | --- |
| `std/regex` | ค้นหา จับกลุ่ม แยก และแทนที่ข้อความ |
| `std/encoding` | แปลงข้อความและ bytes เช่น UTF-8, UTF-16, Shift JIS |
| `std/crypto` | SHA-256/512, HMAC, OS random และ AES-256-GCM |
| `std/compression` | gzip, zlib และ raw DEFLATE |
| `std/archive` | สร้าง อ่าน และดูรายชื่อไฟล์ ZIP/TAR ในหน่วยความจำ |
| `std/uuid` | สร้าง UUID v4 ตรวจ และแปลงรูปแบบ |

```dev-runtime
use "std/regex"
use "std/encoding"
use "std/crypto"
use "std/compression"
use "std/archive"
use "std/uuid"

fn main() {
    let pattern = regex.compile("[0-9]+", "")
    print(pattern.find("port=3000").text)
    print(pattern.replaceAll("a1 b2", "#"))
    pattern.close()
    let text = "DevLang 🚀"
    let utf16 = encoding.encode(text, "utf-16le")
    print(encoding.decode(utf16, "utf-16le"))
    print(crypto.hashText("sha256", "abc"))
    let data = encoding.encode(text, "utf-8")
    let key = crypto.secureBytes(32)
    let aad = encoding.encode("example", "utf-8")
    let packet = crypto.encrypt(key, data, aad)
    print(encoding.decode(crypto.decrypt(key, packet, aad), "utf-8") == text)
    let packed = compression.gzip(data, 6)
    print(encoding.decode(compression.gunzip(packed), "utf-8"))
    let entries = Vec<archive.Entry>()
    entries.push(archive.Entry("hello.txt", data))
    let zip = archive.writeZIP(entries)
    print(archive.listZIP(zip)[0])
    print(encoding.decode(archive.readZIP(zip, "hello.txt"), "utf-8"))
    let tar = archive.writeTAR(entries)
    print(archive.listTAR(tar)[0])
    let id = uuid.v4()
    print(uuid.isValid(id) && uuid.version(id) == 4)
}
main()
```

Regex รองรับ flags `i`, `m`, `s`, `U`; offsets เป็น UTF-8 bytes และไม่รองรับ look-around/backreferences
Encoding ตรวจข้อมูลเสียและตัวอักษรที่ charset เก็บไม่ได้ โดย UTF-16 ไม่เติม BOM อัตโนมัติ
AES-GCM ใช้ key 32 bytes และ nonce สุ่มใหม่ทุกครั้ง; ต้องเก็บ key แยกจากข้อมูล และใช้ AAD เดิมตอนถอดรหัส
`random` ใช้จำลองข้อมูล ส่วน `crypto.secureBytes` ใช้ randomness จากระบบสำหรับงาน security
Crypto ยังไม่มี password hashing, KDF หรือ signatures และ API นี้ไม่ใช่การรับรองความปลอดภัยของแอป
Compression และ Archive จำกัดข้อมูล 8 MiB; Archive รับส่ง bytes และไม่แตกไฟล์ลงดิสก์อัตโนมัติ
UUID สร้างได้เฉพาะ v4; ยังไม่มี v7 และโมดูลชุดนี้ยังไม่รองรับ native build

ลอง `d examples/data-libs/main.dev` และอ่าน [API พร้อมข้อจำกัด](../../docs/data-libs.md)

# [system-libs] DNS และ CLI options

`std/dns` หา IP ผ่าน resolver ของระบบ และ `std/cli` อ่าน options ของโปรแกรมด้วย schema
ทั้งสองโมดูลใช้ source runtime และมี signatures ให้ Language Server ตรวจชนิดข้อมูล

```dev-runtime
use "std/dns"
use "std/cli"

fn main() {
    let definitions = Vec<cli.Option>()
    definitions.push(cli.Option("verbose", "v", false))
    definitions.push(cli.Option("port", "p", true))
    let arguments = Vec<str>()
    arguments.push("-vp3000")
    arguments.push("serve")
    arguments.push("--")
    arguments.push("--literal")
    let parsed = cli.parse(arguments, definitions)
    print(parsed.flags.contains("verbose"))
    print(parsed.values.get("port"))
    print(parsed.positionals[0])
    print(parsed.positionals[1])
    let address = dns.lookupOne("127.0.0.1", 4)
    print(address.address)
    print(address.family)
    print(dns.lookup("::1", 6)[0])
    print(dns.isIP("not an IP"))
}
main()
```

ใช้ `dns.lookup("localhost", 0)` เพื่ออ่าน IP ทั้งหมด หรือ `lookupOne(host, 4)` เพื่อหา IPv4 แรก
family 0 เลือกได้ทั้งสองแบบ, 4 คือ IPv4 และ 6 คือ IPv6; `isIP` คืน 0/4/6 โดยไม่ติดต่อเครือข่าย
DNS เป็น blocking OS lookup รวม hosts file ยังไม่มี MX/TXT/PTR, custom server หรือ timeout ของตัวเอง

`cli.Option("port", "p", true)` กำหนด option ที่รับค่า; false คือ flag
รองรับ `--port 3000`, `--port=3000`, `-p3000`, `-p=3000` และ grouped flags เช่น `-vp3000`
อ่านผลจาก `parsed.values`, `parsed.flags` และ `parsed.positionals`; ใช้ Map `.contains()` ก่อน `.get()`
Unknown/duplicate options และค่าที่ขาดจะเกิด error; flag ไม่เติมค่า false เมื่อไม่ได้ส่ง
Value option กิน token ถัดไปตามข้อความเดิม แม้ขึ้นต้นด้วย `-` ส่วน `--` จบการอ่าน options

ในโปรแกรมจริงใช้ `cli.parse(cli.args(), definitions)` และส่ง arguments ด้วย `d main.dev -- -vp3000 serve`
`cli.args()` ไม่มี executable/source path; help/version และการแปลงค่าเลขต้องกำหนดเอง
ชุดนี้ยังไม่รองรับ native build; CLI จำกัด 128 definitions, 4096 arguments และข้อความ 8 MiB

ลอง `d examples/system-libs/main.dev` และอ่าน [API กับตัวอย่าง CLI](../../docs/system-libs.md)

# [control-libs] Result, Timers และ Child Process

ใช้ result.attempt จับ runtime error แล้วทำงานต่อได้; callback ของ timer รับ Timer handle
child_process.spawn เรียก executable โดยตรง มี timeout และจำกัด stdout/stderr อ่าน API ก่อนเรียกโปรแกรมภายนอก

```dev-runtime
use "std/result"
use "std/fs"
use "std/timers"
use "std/child_process"

fn main() {
    let outcome = result.attempt(fn() str {
        return fs.read_text("missing-example-file.txt")
    })
    match outcome {
        Ok(text) => { print(text) }
        Err(message) => { print("read failed; program continues") }
    }
    print(result.unwrapOr(outcome, "fallback"))
    let arithmetic = result.attempt(fn() i64 { return 6 * 7 })
    print(result.unwrap(arithmetic))
    let options = child_process.options()
    print(options.timeoutMs)
    timers.setInterval(fn(timer timers.Timer) {
        print(timer.ticks())
        if timer.ticks() >= 3 { timer.cancel() }
    }, 1)
    print("main finished; timers run next")
}
main()
```

อ่าน [API และข้อจำกัด](../../docs/control-libs.md)

# [storage-libs] SQLite, CSV, TOML และ YAML

SQLite ใช้ parameter binding และผลลัพธ์ typed enum; transaction ต้อง COMMIT/ROLLBACK เอง
CSV เป็นข้อมูล string ส่วน YAML/TOML ใช้ JSON model และมีข้อจำกัดด้านชนิดข้อมูล ขนาด และ aliases

```dev-runtime
use "std/sqlite"
use "std/csv"
use "std/yaml"
use "std/toml"
use "std/json"

fn main() {
    let database = sqlite.open(":memory:")
    database.execute("CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT)", Vec<sqlite.Value>())
    let params = Vec<sqlite.Value>()
    params.push(sqlite.Value.Text("DevLang"))
    print(database.execute("INSERT INTO users(name) VALUES (?)", params))
    let data = database.query("SELECT name FROM users ORDER BY id", Vec<sqlite.Value>())
    match data.rows[0][0] {
        Null => {}
        Integer(value) => {}
        Real(value) => {}
        Text(value) => { print(value) }
        Blob(value) => {}
    }
    database.close()
    let table = csv.parse("name,age\nDev,18\n", ",", true)
    print(table.headers[0])
    print(table.rows[0][0])
    let config = yaml.parse("name: Dev\nenabled: true\n")
    print(json.string(json.get(config, "name")))
    let manifest = toml.parse("name = 'DevLang'\nversion = 1\n")
    print(json.int(json.get(manifest, "version")))
    print(yaml.valid(yaml.stringify(manifest)))
}
main()
```

อ่าน [API และข้อจำกัด](../../docs/storage-libs.md)

# [secure-network] TLS และ WebSocket

TLS ตรวจ certificate และ hostname ตามปกติ; caFile เพิ่ม CA ที่เชื่อถือได้
WebSocket รองรับ ws/wss, text/binary, ping/pong และ close แต่ I/O ยัง blocking
ตัวอย่างนี้ตรวจการตั้งค่าโดยไม่ใช้เครือข่าย อ่านคู่มือสำหรับ client/server จริง

```dev-runtime
use "std/tls"
use "std/websocket"
use "std/result"

fn main() {
    let options = tls.options()
    print(options.timeoutMs)
    let outcome = result.run(fn() {
        websocket.connect("https://invalid.example", 1000, "")
    })
    print(result.isErr(outcome))
    print("Use ws:// or wss://; TLS verifies certificates")
}
main()
```

อ่าน [API และข้อจำกัด](../../docs/secure-network.md)

# [sync] Channels, Mutex และ Cancellation

std/sync แชร์ resource ข้าม spawn ได้ ส่วน payload ปกติยังเป็นสำเนาข้อมูล
Channel มีคิวจำกัดขนาดและคืน Item/Empty/Closed; ปิดคิวเพื่อปลุก worker ที่กำลังรอ
Mutex.update ทำการเปลี่ยนข้อมูลเป็นหนึ่ง operation และปล่อย lock แม้ callback error
CancelToken เป็น cooperative cancellation; worker ต้องตรวจ token เอง และ await ก่อนจบโปรแกรม

```dev-runtime
use "std/sync"
use "std/result"

fn main() {
    let messages = sync.channel<i64>(2)
    let total = sync.mutex(0)
    let worker = spawn(fn() {
        for i in 1..6 { messages.send(i) }
        messages.close()
    })
    while true {
        match messages.receive() {
            Item(value) => { total.update(fn(n i64) i64 { return n + value }) }
            Empty => {}
            Closed => { break }
        }
    }
    await(worker)
    print(total.get())
    let failed = result.run(fn() {
        total.update(fn(n i64) i64 { result.raise("failed update"); return n })
    })
    print(result.isErr(failed))
    print(total.get())
    total.close()

    let token = sync.token()
    let cancellable = spawn(fn() bool { return token.wait(5000) })
    token.cancel()
    print(await(cancellable))
}
main()
```

ชุดนี้ใช้ runtime เท่านั้นและยังไม่ใช่ coroutine async I/O
อ่าน [API, timeout และข้อจำกัด](../../docs/sync.md)

# [packages] Workspace และ dependency versions

Source build ล่าสุดรองรับหลาย package ใน repository เดียว และ dependency version แบบ SemVer สำหรับ path, workspace และ Git tags ดูสัญญาการใช้งานทั้งหมดใน [Packages reference](#/docs/packages-reference)

## Workspace

สร้าง `package.don` ที่ root และ manifest ของแต่ละ member เช่นตัวอย่าง `examples/workspace`:

```don
version: '0.1.0'
package: { name: 'monorepo' }
workspace: { members: ['apps/app', 'libs/math'] }
```

ใน `apps/app/package.don` อ้างอิง library ด้วยชื่อ package:

```don
version: '0.1.0'
package: { name: 'app', entry: 'src/main.dev', modules: 'src' }
dependencies: {
  math: { workspace: true, version: '^1.0' }
}
```

Library `libs/math/package.don` ต้องมี `package.name: 'math'` และ `version` ที่ตรงเงื่อนไข จากนั้น import ด้วย `use "math/lib"` แต่ละ member มี `package-lock.don` และ cache ของตัวเอง

```sh
d -C examples/workspace pkg workspace
d -C examples/workspace pkg install --workspace
d -C examples/workspace run --package app
d -C examples/workspace build --package app --release -o out/app.exe
d -C examples/workspace pkg install --workspace --locked
```

`--package NAME` เปลี่ยน directory ไปยัง member ก่อนทำคำสั่ง จึงคิด path ของไฟล์, output และ I/O แบบ relative จาก member นั้น ใช้ได้กับ run/build/check/fmt/pkg ต้องประกาศ member ด้วย path ที่แน่นอนภายใน root ไม่รองรับ glob, symlink หรือ nested workspace

## เลือก version

```sh
d pkg add math --workspace --version '^1'
d pkg add math --path ../math --version '~1.2'
# เปลี่ยน URL ตัวอย่างเป็น repository ของคุณ:
d pkg add math --git https://github.com/your-org/math.git --version '^1.2'
d pkg update
```

`^1.2` รับ stable version ตั้งแต่ 1.2.0 และต่ำกว่า 2.0.0; `~1.2` จำกัดใน 1.2.x; `=1.2.3` เลือก version เดียว ใช้ comma เช่น `>=1.2, <2` ได้ แต่ยังไม่รองรับ npm `||` หรือ hyphen ranges Prerelease ต้องระบุเงื่อนไขที่อนุญาตไว้ชัดเจน

Git จะเลือก tag สูงสุดที่ตรงเงื่อนไข เช่น `v1.2.3` และตรวจ version ใน manifest ด้วย `install` รักษา commit เดิม ส่วน `update` เลือก tag ใหม่ `--locked` ตรวจ source/requirements/content และกู้ checkout ที่ขาดโดยใช้ commit เดิม ระบุ `tag` พร้อม `version` ไม่ได้

หากต้องการเลือก branch โดยตรง ใช้ `branch: 'main'` ใน Git dependency หรือ `d pkg add math --git URL --branch main` รองรับชื่อเช่น `feature/new-api` เลือก remote branch แม้มี tag ชื่อเดียวกัน `branch` ใช้พร้อม `tag` หรือ dependency `version` ไม่ได้ `install` ล็อก commit เดิม และ `pkg update` เลือก commit ล่าสุดของ branch

Dependency จาก archive ใช้ `url: 'https://YOUR_HOST/math.zip'` และ `sha256: 'HASH_64_HEX_CHARACTERS'` รองรับ `.zip`, `.tar`, `.tar.gz`, `.tgz` และ HTTP(S)/file URLs ต้องเปลี่ยนเป็น URL และ checksum จริง `version` ใช้ตรวจ manifest ภายใน archive ได้ แต่ `tag`/`branch` ใช้กับ URL ไม่ได้

```don
package: { name: 'archive_app' }
dependencies: {
    math: {
        url: 'https://YOUR_HOST/math-1.2.0.zip'
        sha256: 'REPLACE_WITH_64_CHARACTER_SHA256'
        version: '=1.2.0'
    }
}
```

CLI ใช้ `d pkg add math --url URL --sha256 HASH` ไฟล์ archive ต้องมี manifest ที่ root หรือ wrapper directory เดียว `install --locked` ตรวจเนื้อหา cache และกู้ไฟล์ที่ขาดจากดาวน์โหลดที่ checksum ตรงกันได้ ตัวอย่างรันจริงด้วย HTTP localhost อยู่ใน `examples/archive-url/verify.py`

ยังเป็น namespace เดียว: constraints ที่ใช้ version ที่เลือกเดียวกันได้จะแชร์ package แต่ไม่มี backtracking หรือหลาย version ของชื่อเดียวกัน และยังไม่มี public registry/publish ถ้า add/remove ติดตั้งไม่สำเร็จจะคืน manifest เดิม การติดตั้ง `--workspace` ไม่เป็น transaction ทั้งกลุ่ม

# [module-api-intro] คู่มือแต่ละ Module

เปิด [Module API](#/docs/module-api) เพื่อดูหน้าของ builtin ครบ 42 runtime modules และ native C stdlib 4 bindings แต่ละหน้ามี import, signatures และ types, ตัวอย่างพร้อมคำสั่ง, errors, resource ownership และข้อจำกัดของโหมด

ค้นหาด้วยชื่อ เช่น `std/fs`, `std/http`, `std/json`, `std/sync` หรือเลือกกลุ่ม Module API ใน sidebar ตัวอย่างของแต่ละหน้ามี ZIP และผลลัพธ์ที่รันตรวจไว้ใน [Examples](#/examples?category=Module%20API)

ชื่อ `std/io`, `std/strings`, `std/time` มีทั้ง runtime และ native API แต่ signature ไม่เหมือนกัน เลือกหน้าที่ระบุ Native เมื่อต้อง build/link stdlib ส่วน `std/memory` เป็น native binding; source runtime ใช้ managed Ref/Vec/Map หรือ C FFI ตามงาน

# [don-editor] DON ใน VS Code

Extension `n-devs.devlang-language` รุ่น 0.2.0 รองรับไฟล์ `.don` แยกจาก `.dev` ทั้ง syntax colors, file icon, Format Document และ language server ต้องตั้ง `devlang.executablePath` ให้ชี้ไปที่ d source build ล่าสุด

## Format Document

กด Shift+Alt+F หรือเลือก Format Document โดยใช้ DevLang เป็น default formatter ของภาษา Dev Object Notation formatter รับข้อความที่ยังไม่บันทึก เก็บ comments, `@references`, ลำดับ keys และเนื้อหา quoted/multiline strings ไว้ ถ้าไฟล์ผิดจะไม่แก้ไขข้อความ

```don
// เวอร์ชันเดียวกัน
version: 'v1.0.0'
dependencies: {
    utils: { git: 'https://github.com/example/utils.git', tag: @version }
}
```

```sh
d don fmt-source package.don
```

คำสั่งนี้พิมพ์ source ที่จัดรูปแบบแล้ว ใช้ indentation 4 spaces และ LF นอก literal tokens ส่วน `d don fmt` เดิมเป็นการ serialize ข้อมูล: จะ resolve references และทิ้ง comments

## Diagnostics และ navigation

LSP ตรวจ syntax, duplicate keys, escapes, missing references และ reference cycles ตั้งแต่ข้อความที่ยังไม่บันทึก พร้อม Outline แบบ nested, hover ชนิด value, completion สำหรับ true/false/null และ reference paths และ Go to Definition ของ reference ในไฟล์เดียว ใช้กับ Untitled ที่เลือก language เป็น Dev Object Notation ได้ด้วย

ยังไม่มี schema validation, references ข้ามไฟล์, rename หรือ expression evaluation และ parser แจ้ง error แรกต่อการตรวจแต่ละครั้ง

## File icon และสี code

แยกสี keys, strings, numbers, comments และ `@root.path` ออกจากกันโดยไม่เปลี่ยนธีม editor เมื่อใช้ Material Icon Theme จะเพิ่ม icon `.don` ให้อัตโนมัติ โดยเก็บ association ที่ผู้ใช้ตั้งเองไว้ สำหรับธีมอื่นใช้ language icon ตามที่ธีมรองรับ หรือเลือก DevLang File Icons
