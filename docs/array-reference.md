# Array Reference / คู่มืออ้างอิง array

## Array constructor / การสร้างด้วย constructor

```dev
use "std/json"
fn main() {
    let empty = new Array()
    empty.push("Dev")
    let slots = new Array(3)
    print(json.stringify(slots)) // [null,null,null]
    slots[0] = 42
    let mixed = new Array("Dev", 42, true, null)
    print(json.stringify(mixed))
    print(mixed.length)
    print(Array.isArray(mixed))
}
main()
```

`new Array()` creates an empty `Vec<json.Value>`. A single numeric argument is a length: it must be an integer from 0 to 1,000,000. Each slot contains null. A single non-number is one element; multiple arguments become elements in order. Use `[3]` or `Vec<i64>(3)` for a single numeric element.

`new Array()` สร้าง `Vec<json.Value>` ว่าง หากส่งตัวเลขตัวเดียวจะหมายถึงความยาว ตั้งแต่ 0 ถึง 1,000,000 และต้องไม่มีเศษ แต่ละช่องเป็น null หากส่งค่าที่ไม่ใช่ตัวเลขตัวเดียวจะได้สมาชิกหนึ่งตัว ส่วนหลาย argument จะเรียงเป็นสมาชิกตามลำดับ ใช้ `[3]` หรือ `Vec<i64>(3)` หากต้องการเลข 3 เพียงสมาชิกเดียว

The constructor is available in both the source runtime and hosted native compiler. Dynamic array insertion (`push`, `unshift`, `splice`, `toSpliced` and index assignment) boxes values into json.Value. Callbacks use `json.Value` parameters and JSON accessors such as `json.int(value)`. Typed array methods retain their type requirements: default numeric sort/search and scalar toString/join require suitable typed elements. Arrays are dense; missing entries are null, not JavaScript holes or undefined. These arrays keep DevLang Vec copy-on-write semantics.

รองรับทั้ง runtime และ native สมาชิกเป็น json.Value จึงเขียน callback โดยระบุ parameter ชนิด json.Value และอ่านค่าผ่าน json.int เป็นต้น method ที่กำหนดชนิดสมาชิกไว้ยังคงข้อกำหนดเดิม ไม่มีช่องว่างแบบ JavaScript holes หรือ undefined และใช้ copy-on-write เช่นเดียวกับ Vec ของ DevLang


A reference for DevLang collection syntax and supported methods, verified against
the source runtime and hosted native compiler. Use a current source build after
v0.5.1. DevLang collections do not implicitly coerce element types.

คู่มือรวม syntax และเมธอดที่มีจริงใน DevLang ใช้ได้ทั้ง runtime และ hosted native
จาก source ล่าสุดหลัง v0.5.1 ข้อมูลใน Array, Vec และ Slice ต้องมีชนิดเดียวกัน

## Collection types / เลือกชนิดข้อมูล

| Type | Syntax | Size and mutation / ความยาวและการแก้ไข |
| --- | --- | --- |
| Fixed array | `[T; N]`, `[1, 2, 3]` | Fixed length; writable elements when binding is let / ความยาวคงที่ |
| Vec | `Vec<T>(...)`, `[...values, extra]` | Growable, copy-on-write; let allows mutation / เพิ่มลบสมาชิกได้ |
| Slice | `Slice<T>`, `values[start..end]` | Read-only snapshot; end exclusive / อ่านได้อย่างเดียว |
| DON data array | `{items: [1, 'Dev', true, null]}` | Heterogeneous data; the typed methods below do not apply / ข้อมูลหลายชนิด |

`let` is mutable; `const` prevents mutation through that binding. Fixed arrays copy
and pass/return by value. Vec copies and Slice share storage until Vec mutation
creates a separate copy; existing snapshots retain their values.

## Syntax quick reference / รูปแบบการเขียน

| Syntax | Meaning / การใช้งาน |
| --- | --- |
| `let xs = [1, 2, 3]` | Infer homogeneous element type; integer literals default to i64 |
| `let xs: [i32; 3] = [1, 2, 3]` | Explicit element type and fixed length |
| `let xs = [value; 3]` | Repeat value; evaluate it once; positive integer literal count |
| `let xs = Vec<i64>()` | Empty growable collection |
| `let xs = Vec<i64>(1, 2, 3)` | Initialized growable collection |
| `xs[0]`, `xs[0] = value` | Zero-based read or checked writable assignment |
| `xs.at(-1)` | Read from the end; negative indexing applies to at, not brackets |
| `xs[1..3]` | Read-only Slice containing positions 1 and 2 |
| `[...xs, extra]` | New Vec combining compatible typed collections and elements |
| `let [first, second, ...rest] = xs` | Destructure; optional final rest is a Vec snapshot |
| `for value in xs { print(value) }` | Iterate a snapshot; break and continue supported |
| `for i in 0..3 { print(xs[i]) }` | Numeric range; end exclusive |

```dev
fn main() {
    const fixed: [i64; 3] = [10, 20, 30]
    let repeated = [7; 3]
    let vector = Vec<i64>(1, 2)
    vector.push(3)
    let view = fixed[1..3]
    let [first, second, ...rest] = fixed
    let merged = [...fixed, 40]
    print(first)
    print(second)
    print(rest.at(0))
    print(repeated.at(-1))
    print(view.at(0))
    print(merged.len())
    for value in vector { print(value) }
}
main()
```

Repeat counts must be positive integer literals (up to 1,000,000). Array
patterns currently have no holes/defaults; rest must be last. Slices require both
bounds. Spread/rest use typed Array, Vec or Slice, not heterogeneous DON arrays.
`xs[-1]` is an error; use `xs.at(-1)`. Out-of-range reads are errors, not undefined.

[Download the syntax example / ตัวอย่าง syntax](#/examples/array-reference)

## Array Methods / เมธอดของ Array, Vec และ Slice

These methods work in the source runtime and hosted native compiler from current
source after v0.5.1. They operate on homogeneous typed collections. DON data arrays
with mixed types are not supported by these methods.

รองรับทั้ง runtime และ native สำหรับ Array, Vec และ Slice ที่มีชนิดข้อมูลเดียวกัน
เมธอดที่สร้างผลลัพธ์ใหม่จะไม่เปลี่ยนข้อมูลเดิม ส่วนเมธอดแก้ข้อมูลใช้กับ Vec ที่เขียนได้เท่านั้น

| Method | Receiver | Result / การทำงาน |
| --- | --- | --- |
| `len()` | Array, Vec, Slice | usize length / จำนวนสมาชิก |
| `at(index)` | Array, Vec, Slice | T; i64 index, negative counts from the end / ติดลบนับจากท้าย |
| `slice(start, end)` | Array, Vec, Slice | Read-only Slice snapshot; end exclusive |
| `includes(value)` | Array, Vec, Slice | bool; numeric, bool or str equality |
| `indexOf(value)`, `lastIndexOf(value)` | Array, Vec, Slice | i64 index, or -1 |
| `toVec()` | Array, Vec, Slice | New Vec containing visible elements / สำเนาเป็น Vec |
| `concat(other)` | Array, Vec, Slice | New Vec; other is Array, Vec or Slice of the same type |
| `toReversed()` | Array, Vec, Slice | New Vec in reverse order / กลับลำดับโดยไม่แก้ต้นฉบับ |
| `sort(comparator?)` | Writable Vec | Stable ascending order or custom comparison; returns void |
| `toSorted(comparator?)` | Array, Vec, Slice | New stably sorted Vec; original preserved |
| `join(separator)` | Array, Vec, Slice of str | str; explicit str separator / รวมข้อความ |
| `map(callback)` | Array, Vec, Slice | Vec of callback results / แปลงสมาชิก |
| `filter(predicate)` | Array, Vec, Slice | Vec containing matching elements / กรองสมาชิก |
| `reduce(callback, initial)` | Array, Vec, Slice | Accumulator value / รวมผลโดยเริ่มจาก initial |
| `forEach(callback)` | Array, Vec, Slice | void; visit each element / ทำงานกับแต่ละสมาชิก |
| `some(predicate)`, `every(predicate)` | Array, Vec, Slice | bool; short-circuit / หยุดเมื่อทราบผล |
| `findIndex(predicate)`, `findLastIndex(predicate)` | Array, Vec, Slice | i64 index, or -1; last searches backwards |
| `push(value)`, `unshift(value)` | Writable Vec | void; append or prepend / เพิ่มท้ายหรือหน้า |
| `pop()`, `shift()` | Writable Vec | T; remove last or first / นำท้ายหรือหน้าออก |
| `reverse()` | Writable Vec | void; reverse in place, preserving other snapshots |
| `clear()` | Writable Vec | void; remove all elements |

```dev
fn main() {
    const values = [1, 2, 3, 4]
    let doubled = values.map(fn(n: i64): i64 { return n * 2 })
    let even = doubled.filter(fn(n: i64): bool { return n % 4 == 0 })
    print(even.reduce(fn(sum: i64, n: i64): i64 { return sum + n }, 0))
    print(values.at(-1))
    print(values.findIndex(fn(n: i64): bool { return n > 2 }))
    let queue = values.toVec()
    queue.unshift(0)
    print(queue.shift())
    queue.reverse()
    print(queue.at(0))
    print(["Hello", "Dev"].join(" "))
}
main()
```

Callbacks are Dev functions or closures with one typed element parameter and an optional index: i64.
`map` returns a non-void value; `filter`, `some`, `every` and index predicates return
bool; `forEach` returns void. `reduce` takes a callback with two parameters and an optional third index: i64:
`fn(acc: A, item: T): A`, and an explicit initial value of A. Mapping or reducing
cannot produce fixed arrays; use Vec for collection results. Captures follow the
ordinary Dev snapshot rules. Array methods evaluate receiver, callback and initial
value once, left to right, and iterate a snapshot. Slice indices are relative to
its visible range. `findLastIndex` invokes predicates from the end and stops at the
first match. Empty `some` is false, empty `every` true, empty index searches -1;
empty reduce returns initial without calling its callback.

Callback ใช้ฟังก์ชันหรือ closure ที่ระบุชนิด parameter ให้ตรงกับสมาชิกได้
`reduce` ต้องมีค่าเริ่มต้นเสมอ และ predicate ต้องคืน bool เมื่อใช้ Slice ตำแหน่งเริ่มที่ 0
ของ Slice นั้น `some`/`every` และการค้นหาด้วย predicate หยุดทันทีเมื่อได้คำตอบ

`at` out of range and `pop`/`shift` on an empty Vec are located runtime errors;
there is no undefined result. Const bindings allow read-only methods, including
forEach, but reject mutating methods. `join` supports str elements only, requires
an explicit separator and runs in O(total output bytes). Native generated strings
remain allocated until process exit, as with string interpolation.
`shift`/`unshift`/`reverse`, conversion, concat, map, filter and searches are O(n);
unique Vec push/pop are amortized O(1). Fixed-array receivers currently convert
through a temporary Vec, including at; do not assume zero-allocation access.

ยังไม่มี flatMap, callback array arguments, หรือ find ที่คืน
optional value; ใช้ findIndex แล้วตรวจ -1 ก่อนเข้าถึงสมาชิก ไม่มี JavaScript coercion
หรือ undefined และไม่ใช่ API ของไฟล์ .don โดยตรง

[Full runnable example / ตัวอย่างเต็ม](#/examples/array-methods)


## Array Sort / เรียงลำดับสมาชิก

`sort()` sorts a writable Vec and returns void. `toSorted()` accepts Array, Vec
or Slice and returns a new Vec; it preserves its receiver. Both support an
optional comparator `fn(a: T, b: T): i64`. A negative result puts a first, zero
means equal, and a positive result puts b first. Capturing closures are supported.

```dev
fn main() {
    const values = [10, 2, 1]
    let ascending = values.toSorted()
    print(ascending.at(0)) // 1; numeric ordering, not string ordering
    let descending = values.toSorted(fn(a: i64, b: i64): i64 {
        return a < b ? 1 : (a > b ? -1 : 0)
    })
    print(descending.at(0)) // 10
    let mutable = values.toVec()
    mutable.sort()
    print(mutable.at(0)) // 1
    print(["zebra", "apple"].toSorted().join(",")) // apple,zebra
}
main()
```

Default ordering is ascending numeric order or case-sensitive UTF-8 byte order
for str. Floats place NaNs after all numbers; NaNs compare equal, and signed zeros
compare equal. Other element types, including bool, records and Ref, need a
comparator. Return i64, not bool. Use comparisons as above rather than subtraction
when signed integer subtraction might overflow. Comparators should define a
consistent ordering and avoid side effects.

Both implementations use stable bottom-up merge sort: equal elements retain
their original order, O(n log n) comparisons and O(n) additional storage. Sort
works on a separate buffer, preserving earlier Vec copies and Slice snapshots.
Empty and single-element inputs are valid. Comparator evaluation happens once;
its function is invoked for comparisons. There is no locale-aware collation,
JavaScript type coercion or heterogeneous DON-array sorting. Native str has its
ordinary C-string semantics. These methods are available in current source after
v0.5.1, in source runtime and hosted native modes.

`sort()` เรียง Vec ที่แก้ไขได้และคืน void ส่วน `toSorted()` สร้าง Vec ใหม่จาก Array,
Vec หรือ Slice โดยไม่แก้ต้นฉบับ ค่าเริ่มต้นเรียงตัวเลขจากน้อยไปมาก และ string ตาม byte
UTF-8 แบบแยกตัวพิมพ์ใหญ่เล็ก ถ้าต้องการเรียงตามเงื่อนไขอื่นให้ส่ง comparator ที่คืน i64:
ติดลบให้ a มาก่อน, ศูนย์เท่ากัน, บวกให้ b มาก่อน ข้อมูลที่เท่ากันจะคงลำดับเดิม
ใช้ได้ทั้ง runtime/native แต่ยังไม่รองรับ DON array ที่มีหลายชนิด

[Full runnable example / ตัวอย่างเต็ม](#/examples/array-sort)


## Callback index / ตำแหน่งสมาชิกใน callback

Array, Vec and Slice iteration callbacks accept an optional final `index: i64`.
Indices start at zero, relative to the receiver (including Slice), and refer to
original input positions, not the filtered output. Existing one-parameter
callbacks remain valid. `reduce` accepts index as its optional third parameter;
sort comparators retain two parameters and do not receive an index.

```dev
fn main() {
    const values = [10, 20, 30]
    values.forEach(fn(value: i64, index: i64) { print("${index}: ${value}") })
    let mapped = values.map(fn(value: i64, index: i64): i64 { return value + index })
    print(mapped.at(2)) // 32
    print(values.reduce(fn(acc: i64, value: i64, index: i64): i64 {
        return acc + value + index
    }, 0)) // 63
}
main()
```

ใช้ index ได้ใน map, filter, forEach, some, every, findIndex และ findLastIndex
โดยเป็น parameter ตัวที่สอง ส่วน reduce เป็นตัวที่สาม ชนิดต้องเป็น i64 และเริ่มที่ 0
เมื่อใช้ Slice จะนับจากจุดเริ่มของ Slice นั้น ไม่จำเป็นต้องใส่ index หากไม่ได้ใช้
findLastIndex เรียก callback จากท้าย แต่ index ยังเป็นตำแหน่งเดิมของสมาชิก

[Full runnable example / ตัวอย่างเต็ม](#/examples/array-index)

## Runnable examples / ตัวอย่างที่รันได้

- [Array tools: copy, pass/return, repeat, spread and destructuring](#/examples/array-tools)
- [Search: includes, indexOf, lastIndexOf](#/examples/array-search)
- [Methods: map, filter, reduce, queue operations and join](#/examples/array-methods)
- [Callback index: position-aware iteration](#/examples/array-index)
- [Sort: stable ordering and custom comparators](#/examples/array-sort)

## Unsupported operations / ข้อจำกัดปัจจุบัน

No flatMap, sort by locale, callback array argument or optional-value
find/findLast. Use findIndex/findLastIndex, test for -1, then access the element.
Nested local fixed arrays are not supported in native; use Vec<Vec<T>> instead.
Fixed Array and Slice use read-only methods; length-changing operations and
in-place reverse/sort require writable Vec. Native strings created by join and
interpolation remain allocated until program exit.

ยังไม่มี flatMap, find ที่คืน optional value หรือการส่ง array เป็น
parameter ของ callback และเมธอดชุดนี้ยังไม่ใช้กับ DON array ที่มีหลายชนิด


## Basic Array Methods / เมธอดพื้นฐาน

| Method | Available receiver | Parameters and return |
| --- | --- | --- |
| `length` / `length()` / `len()` | Array, Vec, Slice | Read-only usize element count |
| `toString()` | Array, Vec, Slice | Comma-separated str; numeric/bool/str elements |
| `at(index)` | Array, Vec, Slice | i64 index, negative from end; T or bounds error |
| `join(separator)` | str Array, Vec, Slice | Explicit str separator; str result |
| `pop()` / `shift()` | Writable Vec | Remove last/first; return T; empty is an error |
| `push(value)` / `unshift(value)` | Writable Vec | Add last/first; return void |
| `Array.isArray(value)` / `xs.isArray()` | Any value | bool; true for Array, Vec, Slice and DON arrays |
| `delete(index)` | Writable Vec | Remove one element, shift following elements; return T |
| `concat(other)` | Array, Vec, Slice | Compatible typed collection; return new Vec |
| `copyWithin(target, start, end?)` | Writable Vec | i64 indices; overwrite from snapshot, preserve length; void |
| `flat(depth?)` | Array, Vec, Slice | Literal depth 0..32 (default 1); return new Vec |
| `slice(start, end)` | Array, Vec, Slice | Checked nonnegative bounds; end-exclusive Slice |
| `splice(start, deleteCount?, ...items)` | Writable Vec | i64 start/count, same-type items; return removed Vec |
| `toSpliced(start, deleteCount?, ...items)` | Array, Vec, Slice | Return edited copy as Vec; preserve input |

```dev
fn main() {
    const values = [1, 2, 3, 4]
    print(values.length) // 4
    print(values.toString()) // 1,2,3,4
    print(Array.isArray(values)) // true
    let xs = values.toVec()
    let removed = xs.splice(1, 2, 9, 8)
    print(removed.toString()) // 2,3
    print(xs.toString()) // 1,9,8,4
    print(xs.delete(-1)) // 4
    xs.copyWithin(1, 0, 2)
    print(xs.toString()) // 1,1,9
    print(values.toSpliced(1, 1, 99).toString()) // 1,99,3,4
    let nested = Vec<Vec<i64>>(Vec<i64>(1, 2), Vec<i64>(3, 4))
    print(nested.flat().toString()) // 1,2,3,4
}
main()
```

For splice/toSpliced, a negative start counts from the end and is clamped to the
collection bounds. Negative deleteCount becomes zero; counts beyond the end are
clamped. Omitted deleteCount removes the remaining suffix. Each replacement must
match the element type. The removed Vec is a separate snapshot.
copyWithin also normalizes negative indices and clips ranges; omitted end means
length, end is exclusive, and end before start copies nothing. Overlap reads the
original input, preserving existing Vec/Slice aliases. These operations take O(n)
time and O(n) extra storage in the current implementation.

DevLang uses dense typed collections: delete removes and shifts, it does not leave
an undefined hole. Mutating methods require writable Vec, including copyWithin.
Use toVec for a writable copy of Array/Slice. length is read-only; assign elements
or use Vec methods to change contents. splice/toSpliced require at least start.
flat's depth is a compile-time decimal integer literal so its result type is known;
use Vec<Vec<T>> for nested native collections. flat(0) copies without flattening,
and depth beyond the nesting level is harmless. Nested local fixed arrays remain
unsupported in native. toString supports numeric, bool and str elements, without
aggregate coercion; floating output follows each backend's scalar formatting.
Native generated strings remain allocated until exit, as with interpolation.

รองรับครบตามรายการนี้ โดย length อ่านได้อย่างเดียวและคืน usize, delete ลบแล้วเลื่อน
สมาชิกโดยไม่สร้างช่องว่าง, splice คืนสมาชิกที่ลบ, toSpliced คืนสำเนาที่แก้แล้ว และ
copyWithin คัดลอกจากข้อมูลเดิมแม้ช่วงทับกัน เมธอดที่แก้ข้อมูลใช้กับ Vec ที่เขียนได้
flat ใช้กับชุดข้อมูลชนิดเดียวกันและระบุ depth เป็นตัวเลขคงที่ 0 ถึง 32 เท่านั้น
Array.isArray ตรวจ DON array ได้ แต่เมธอดแก้ข้อมูลชุดนี้ยังไม่รองรับ DON array หลายชนิด

[Full example / ตัวอย่างเต็ม](#/examples/array-basic)
