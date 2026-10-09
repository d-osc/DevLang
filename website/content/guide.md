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

ยังไม่มี package manager, debugger/LSP integration, coroutine event loop, async I/O, channels/cancellation, trait objects, literal/struct/slice patterns, inclusive/custom-step ranges หรือ iterator for collections

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
