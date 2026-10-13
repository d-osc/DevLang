# Array Methods

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

ยังไม่มี splice, flat/flatMap, callback array arguments, หรือ find ที่คืน
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
