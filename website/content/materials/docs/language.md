# Dev Lang specification

Compiler v0.4 supports generic external namespaces through `--module-dir NAME=DIR` and explicit libraries through `--link`. The source runtime interprets Dev directly. The separate native C library has ordinary Dev bindings. See [runtime.md](runtime.md) and [stdlib.md](stdlib.md) for contracts and [optimization.md](optimization.md) for historical v0.2 measurements.

## Layout and syntax

A program consists of `.dev` files. Imports, function declarations and executable statements appear at entry-file scope. Imported files contain declarations only. Statements end at a newline or an optional semicolon. Braces delimit blocks. Newlines inside `()` and `[]` are whitespace. Comments begin with `#` or `//`. Names use ASCII letters, digits and `_`, and cannot start with a digit; strings contain UTF-8.

```dev
use math
use "../common/device.dev" as device

fn add(a: i32, b: i32): i32 {
    return a + b
}

fn main() {
    let value = 42
    let register: u32 = 0x4000_0000
    value += 1
    print(value)
}

main()
```

Parameter types are required; the return type defaults to `void`, except a defined `main` defaults to `i32`. Entry-file statements execute in order; defining `main` never invokes it. Write `main()` explicitly, or use top-level statements directly. Reaching the end of an i32 `main` returns zero. A file-level `return main()` forwards its result as the exit status. Other value-returning functions must return on every path; this check conservatively does not treat a loop as guaranteed to terminate or return.

`let` variables are mutable. Local variables can shadow an outer scope, but duplicate declarations in the same scope are errors. Variables must have an initializer. File-level variables belong to the script scope and cannot be captured by functions; pass them as parameters. Use `name: Type` for explicit annotations and `fn name(arg: Type): ReturnType` for functions. The formatter normalizes declarations to this spelling. Legacy colon-free annotations and `->` return types remain accepted for compatibility.

Colon return annotations and canonical formatting are available in current source after release v0.5.1.


## Const bindings

`const` creates a binding whose value cannot be changed after initialization.
The initializer can run at runtime; it does not need to be a compile-time value.

```dev
fn main() {
    const user = { name: "Dev", age: 10 }
    const {name, age} = user
    print(name)
    print(age)
    const limit: i32 = 100
    print(limit)
}
main()
```

Assignments to a const binding, its fields or indexed elements are errors.
Mutating Vec/Map methods through that binding are also rejected. Destructuring
supports renamed bindings: `const {name: userName} = user`. A `let` copy can be
changed independently. Inner scopes may shadow the binding with a new `let`.
This is binding immutability, not a guarantee that resources behind raw pointers
or external handles never change. Available in current source builds after v0.5.1.


## Types

| Type | Meaning / C representation |
| --- | --- |
| `i8 i16 i32 i64` | Fixed-width signed integers / `intN_t` |
| `u8 u16 u32 u64` | Fixed-width unsigned integers / `uintN_t` |
| `isize usize` | Pointer-sized integers / `intptr_t`, `uintptr_t` |
| `f32 f64` | `float`, `double` |
| `bool` | `bool` / `_Bool` |
| `str` | Read-only NUL-terminated UTF-8 C string / `const char *` |
| `*T` | Raw pointer to `T`; `*void` is an opaque C pointer |
| `[T; N]` | Fixed-size, one-dimensional local array |
| `void` | No value; function returns and opaque pointer pointee only |

Without context, integer literals use `i64` and floating literals use `f64`. A typed initializer, function argument, return, or compatible arithmetic operand supplies literal context. Integer literals support decimal, hexadecimal `0x...`, and `_` separators; floats support decimal and exponent notation. Out-of-range literals are rejected, including the signed minimum special case.

Numeric types do not implicitly mix. Use `as` for an explicit conversion:

Float-to-integer casts truncate toward zero. The truncated value must fit the
destination integer type; NaN, infinity and out-of-range values produce
`float-to-integer cast out of range` with a source location. These checks apply
in both source engines and optimized native builds. Integer-to-integer casts
keep their existing wrapping behavior.
The float-cast checks are available in current source builds; the published
Included in release v0.5.1.

```dev
let x: i32 = 42
let wide = x as i64
let real = wide as f64
```

Arrays infer their element type from the first element when no type is specified. Every element must match. Arrays are initialized by literals and cannot be copied, returned by value, passed by value, nested, or addressed as a whole. Pass `&array[0]` and an explicit length to functions instead.

```dev
fn first(data: *u8): u8 {
    unsafe { return data[0] }
}

fn main() {
    let bytes: [u8; 3] = [10, 20, 30]
    bytes[1] = 42
    unsafe { print(first(&bytes[0])) }
}

main()
```

`str[index]` reads a UTF-8 **byte** as `u8`, not a Unicode character. String bytes cannot be assigned. String equality is deliberately undefined; call C `strcmp` for content comparison. Strings and `null` follow C lifetime and NUL-termination rules; embedded `\0` truncates ordinary C string output.

### String concatenation

Both source runtime and hosted native builds support `str + str` and `+=`.
Native concatenation is available in source after v0.5.1. It uses the same storage
policy as interpolation: resulting strings remain alive until process exit.
Operands must both be strings; numbers require interpolation or explicit conversion.

```dev
let text: str = "Hello " + "Dev"
text += "!"
print(text)
```

### String interpolation

Use `${expression}` inside a double-quoted string to insert strings, integers,
floats or booleans. Expressions are evaluated once, from left to right.
Escape the dollar with `\${...}` to keep a literal placeholder. Empty,
unclosed and invalid expressions are diagnosed at the source location.

```dev
let name: str = "Dev"
let count: i64 = 41
print("Hello ${name}!")
print("answer = ${count + 1}, ready = ${true}")
print("literal: \${name}")
```

Available in source after v0.5.1, in the runtime and hosted native builds.
Native interpolation uses a C11 backend and retains allocated strings until
process exit, when the OS reclaims their storage; repeated interpolation can
increase RAM usage. Native floats use `%.17g`, as native print does, whereas
runtime floats use Rust display. Structs, collections and pointers are not
implicitly converted to text. Freestanding interpolation is rejected.


## Ternary operator / เลือกค่าด้วยเงื่อนไข

`condition ? valueIfTrue : valueIfFalse` เลือกประเมินเพียงฝั่งเดียว
เงื่อนไขต้องเป็น `bool` และผลลัพธ์สองฝั่งต้องมีชนิดเดียวกัน
The condition must be bool and both branches must have the same type.
Only the selected branch is evaluated. `?:` has lower precedence than `||`
and associates to the right; use parentheses to make nested choices clearer.

```dev
fn main() {
    let age = 18
    const label = age >= 18 ? "adult" : "child"
    print(label)
    let score = 85
    print(score >= 90 ? "A" : score >= 80 ? "B" : "C")
    print(true ? 42 : 1 / 0) // The division is not evaluated.
}
main()
```

Supports runtime and hosted native builds from current source after v0.5.1.
Native ternary branches must produce a value; fixed arrays are copied by value.
See [ตัวอย่างเต็ม / Full example](#/examples/ternary).


## Switch statement / เลือกทำงานตามค่า

```dev
fn main() {
    let status = 200
    switch status {
        case 200, 201: { print("success") }
        case 404: { print("not found") }
        default: { print("other status") }
    }
}
main()
```

`switch` ประเมินค่าที่เลือกครั้งเดียว แล้วทำงาน case แรกที่ค่าตรงกัน
ใช้ comma ระบุหลายค่าต่อ case ได้ แต่ละ case ต้องมี `{ ... }`
`default` ไม่บังคับ และถ้ามีต้องอยู่ท้ายสุด ไม่มี fallthrough จึงไม่ต้องใส่ `break`
Switch evaluates its subject once and runs the first matching case. Case values
use ordinary equality rules and are checked in source order. String cases compare
contents in both engines, including native switch. Standalone native str equality
remains unchanged. Case bodies require
braces. Default is optional, may appear once, and must be last. There is no
fallthrough. `break` and `continue` keep their existing enclosing-loop meaning;
they are not needed to end a case. Use `match` for enum payload patterns.

Available in current source builds after v0.5.1, in runtime and native modes.
See [ตัวอย่างเต็ม / Full example](#/examples/switch).

## Expressions and control flow

Operators, from low to high precedence:

| Level | Operators |
| --- | --- |
| 0 | `?:` (right-associative) |
| 1 | `or`, `||` |
| 2 | `and`, `&&` |
| 3–5 | `|`, `^`, `&` |
| 6 | `==`, `!=` |
| 7 | `<`, `<=`, `>`, `>=` |
| 8 | `<<`, `>>` |
| 9 | `+`, `-` |
| 10 | `*`, `/`, `%` |
| 11 | `as T` |
| 12 | unary `+ - ! not ~ & *`, function call, indexing |

Logical operators and conditions require `bool`. `and/or` short-circuit. Arithmetic requires matching numeric types except for contextual literals; bitwise operators, shifts and modulo require integers. Assignment supports `= += -= *= /= %= &= |= ^=`.

```dev
let i = 0
while i < 10 {
    i += 1
    if i == 3 { continue }
    if i == 8 { break }
    print(i)
}

if i == 8 {
    print(true)
} else if i == 9 {
    print(false)
} else {
    print("done")
}
```

`break` and `continue` are valid inside `while` and `for` loops. Function calls support forward declaration and recursion. Calls resolve to a local function or `module.function`; function values and closures use `fn(T) R` types (see [advanced features](advanced.md)).

## Modules and linkage

`use math` resolves `math.dev` relative to the importing file. `use "lib/math.dev" as math` accepts an explicit relative or absolute path and alias. A canonical file path identifies a module, so several imports of one file load it once. Cycles are allowed because signatures are collected before checking bodies.

Functions are visible to direct importers. Imports are not transitively re-exported. Dev functions get module-specific C symbols, preventing unrelated modules' same-name functions from colliding. `export fn` keeps its declared name as a C symbol; export names must be unique across the program. Imported `extern fn` declarations for one C symbol must agree on their types.

```dev
extern fn malloc(bytes: usize): *void
extern fn free(memory: *void)

export fn add(a: i32, b: i32): i32 {
    return a + b
}

fn main() {
    unsafe {
        let data = malloc(64) as *u8
        if data == null { return 1 }
        data[0] = 42
        print(data[0])
        free(data as *void)
    }
}

main()
```

The C ABI follows the selected backend's target, calling convention and type definitions. FFI signatures must match the actual C declaration. C scalar/struct ABI, variadic calls and scalar callbacks with captured values are supported. Payload unions and custom calling-convention attributes require C wrappers. Native capturing callbacks retain environments until exit and have a 64-slot limit per signature per module; see advanced.md. Use C wrapper functions for unsupported APIs. CLI `--link file.c`, `.o`, `.a`, or the backend's native object/archive formats provide additional implementations. `--ldflag -lm`, for example, passes a library flag on toolchains that support it.

## Builtins and hardware

| Builtin | Behavior |
| --- | --- |
| `print(value)` | Prints one scalar followed by a newline; returns `void`; hosted only |
| `sizeof(value)` | Size in bytes of the value's type, or full array; returns `usize`; does not evaluate the value |
| `volatile_load(pointer)` | Reads one numeric/bool value through a C volatile-qualified pointer |
| `volatile_store(pointer, value)` | Writes one numeric/bool value through a C volatile-qualified pointer; returns `void` |

`null` is contextual to a pointer or `str`, and otherwise has type `*void`. Pointer casts accept another pointer or an integer; direct `str` → mutable pointer casts are rejected. Pointer arithmetic supports `pointer + integer`, `pointer - integer`, and matching pointer subtraction. Dereferencing or performing arithmetic on `*void` requires a cast to an element pointer.

`--freestanding --lib` builds objects with `-ffreestanding -fno-builtin` and archives them. Hosted `print` is unavailable. It introduces no language VM, GC or automatic allocation. Depending on code and target, the C backend can still require compiler support routines or basic memory routines; the platform must supply those, its startup code, and its linker script. See [Clang's freestanding documentation](https://clang.llvm.org/docs/CommandGuide/clang.html).

Cross-compiling changes emitted C pointer widths through the backend. Literal range checking for `usize/isize` currently uses the compiler host width; keep cross-target pointer-sized constants within the target range. The ARM example uses no out-of-range address constants and has been verified as ELF32 ARM output, but not on a physical device.

## Runtime semantics and current limits

Clang/GCC backend flags include `-fwrapv` for signed addition/subtraction/multiplication wrapping, including `-O3` release builds. The optional `--fast` TinyCC development backend emits unoptimized machine arithmetic; wrapping, casts, pointers and volatile access are covered by native regression scenarios. Small integer operation results are cast back to the declared width; narrowing casts otherwise follow C behavior. Invalid shifts and integer division/remainder by zero are checked in both modes. Signed minimum divided by `-1` wraps to the minimum; its remainder is zero. Fixed-array, Vec, Slice and string bounds are checked. Raw native dereference/volatile access checks null/alignment; arbitrary foreign allocation bounds and expired pointers remain the caller's responsibility.

Ordinary named calls and Dev function values evaluate arguments from left to right, as do array literals and spread parts. Binary operands and native callback invocation still follow C evaluation rules; split state-changing operands into statements if their order matters. `and/or` retain C's short-circuit behavior. Volatile accesses are observable C accesses, not atomic synchronization, CPU fences, DMA cache maintenance or a substitute for platform drivers.

Compiler limits currently include 512 modules, 8 MiB per source file, and 1..1,000,000 elements per explicitly sized array. Deeply nested source is not hardened against exhausting the parser stack. The language is an initial implementation, with no ownership/lifetime system or production safety guarantees.

## Range for

```dev
for i in 0..10 {
    if i == 3 { continue }
    print(i)
}
```

The range is ascending and excludes the end. Bounds must be `i64` (default
integer literals); evaluate each once, before introducing the iteration
variable. Empty/reversed ranges execute zero times. The iteration variable is
local to each body execution; changing or shadowing it does not change the
internal counter. `break`, `continue`, nested loops and `return` work in both
runtime engines and native code. The parser lowers ranges to scoped statements
and while loops, retaining the numeric execution path. Array iteration, custom
steps and inclusive ranges are not yet implemented.

## Structs and enums

```dev
struct Point {
    x: i64
    y: i64
}

let p = Point(19, 23)
p.x += 1
print(p.x + p.y)

enum Color { Red, Green, Blue }
let color: Color = Color.Green
print(color)
print(color == Color.Green)
print(color as i32)
```

Struct constructors are positional, in declaration order, with all fields
required. Dot access supports nested fields and fields of array elements.
Struct assignment, arguments and returns use value semantics. Runtime strings
are owned; native `str` fields retain the existing C pointer/lifetime contract.
Structs may contain other structs, enums and fixed-size arrays.
Recursive layouts through `*T` or `Ref<T>` are supported, including mutual
recursion. Direct by-value cycles are rejected because their size is infinite.
Methods use `fn Type.method(self Type, ...) R` (see advanced.md). Inheritance and named-field constructors are not supported.
Native whole-struct printing is unsupported;
print individual fields. Nominal identity includes the declaring module and
generic arguments: unrelated types with identical fields are distinct.

Payload-free enums have variants numbered from zero, support `==`/`!=`, print
the variant name and explicitly cast to integers. Enums with any payload
variant do not support equality or integer casts; use `match`. Integer-to-enum
casts and custom discriminants are unsupported. All enums print the variant
name; read their payload through `match`.

```dev
enum Result<T> { Ok(T), Err(str) }
let result = Result<i64>.Ok(42)
match result {
    Ok(value) => { print(value) }
    Err(message) => { print(message) }
}
```

`Result.Ok<i64>(42)` is also accepted. Payload-free generic constructors use
parentheses, for example `List<i64>.Nil()`. Patterns use unqualified variant
names, bind every payload position, and allow `_` to ignore one position.
Every variant must be covered by an unguarded irrefutable arm, or a final unguarded `_` arm. The subject is evaluated once; arm
bindings have their own scope and hold value copies. `return`, `break` and
`continue` retain the enclosing function/loop meaning. Match is a statement;
guards (`Variant(x) if condition`), final `_` wildcard arms and nested enum patterns are supported. Refutable nested patterns require a fallback.
Payload types can be scalars, structs, enums and references; wrap fixed arrays
in a struct. Extern C-compatible structs and payload-free enums work by value. Managed references/collections/function values and payload unions require raw-pointer or scalar wrappers.

## Recursive types and immutable references

```dev
enum List<T> {
    Nil
    Cons(T, Ref<List<T>>)
}
let tail = ref(List<i64>.Nil())
let list = List<i64>.Cons(42, tail)
match list {
    Nil => {}
    Cons(value, next) => {
        print(value)
        match deref(next) {
            Nil => { print("end") }
            Cons(_, _) => {}
        }
    }
}
```

`ref(value)` copies a non-array, non-void value into immutable managed storage;
its type is `Ref<T>`. `deref(reference)` returns a value copy. References can
escape functions, be copied and passed across modules. There is no null Ref,
manual free, writable dereference, pointer arithmetic, or Ref-to-pointer cast.
Managed references, including those inside aggregates, cannot be passed or
returned through extern/export C signatures.
Both execution modes use reference counting; locals, returns, container elements and
closure captures keep their values alive and release storage when the last owner
is dropped. This is not a borrow checker or tracing garbage collector. Immutable
references and copy-on-write containers support recursive data without ordinary
mutable reference cycles. Native managed allocation requires hosted mode and
aborts on allocation failure. Native `str` and raw pointers remain borrowed C
memory: copying an enclosing managed value does not take ownership of that memory.

Raw pointer access/casts, address-of, volatile memory operations, C callbacks and
extern calls require lexical `unsafe { ... }`. Function bodies need their own
unsafe blocks. Both modes check fixed-array/vector/slice/string bounds, integer
division by zero and shift counts. Native raw dereference/volatile access also
checks null/alignment; raw allocation bounds, expired pointers, foreign signatures
and synchronization of external memory remain the unsafe caller's responsibility.

Raw recursive layouts are also accepted:

```dev
struct Node { value: i64; next: *Node }
let first = Node(1, null)
unsafe {
    let second = Node(2, &first)
    print((*second.next).value)
} // address-of interpreter locals remains native-only
```

Runtime accepts the same layouts and directly reads/writes live foreign pointers in unsafe blocks, including struct fields and volatile scalar memory. Use `Ref<T>` for pure Dev
recursive data in either execution mode. Neither `struct Node { next Node }`
nor a mutual by-value cycle is accepted.

## Generics

```dev
fn identity<T>(value: T): T {
    return value
}
print(identity<i64>(42))
print(identity<str>("Dev"))

struct Box<T> { value: T }
let boxed = Box<Point>(p)
print(boxed.value.x)
```

Function, struct and enum type arguments can be inferred from values/expected types; explicit arguments remain supported, including in type annotations
such as `let b Box<i64> = Box<i64>(42)`. Nested arguments such as
`Box<Box<i64>>` work. Multiple parameters, forward references, recursive
functions and imported generics are supported. Use `module.Type` and
`module.function<i64>(...)` across imports.

The shared frontend specializes each used function/type combination; native
code receives concrete C types and functions, while the runtime interprets
concrete functions and can plan numerical specializations. Pure Dev generics
never invoke a native backend during runtime execution. Generic bodies must
support the selected types. Static method traits, `Number`/`Integer`/`Equatable` constraints and nonnegative integer const generics are supported; default arguments are not implemented. See [advanced.md](advanced.md). Generic
functions cannot be `extern` or `export`; expose a concrete wrapper instead.
Existing native array parameter/return restrictions still apply. Limits are
1024 generic function instances and 64 levels of nominal type nesting.

See `examples/features/main.dev` and run
`python scripts/smoke_features.py --bin-dir target/release` for both execution paths.

`ContextCallback<fn(T) R>` provides a managed C callback/userdata pair; `callback_context(f)` creates it. `then(task, next)` schedules a task continuation without waiting in the caller. See [advanced features](advanced.md) for ABI, lifetime and thread limits.

### Explicit boolean casts

Use `as` to convert explicitly between `bool` and numeric types. `false` becomes
`0`, and `true` becomes `1`. Numeric zero (including `-0.0`) becomes `false`;
other values, including floating-point NaN and infinity, become `true`.
Conditions still require `bool`; numeric values are not converted implicitly.

```dev
fn main() {
    let enabled = 3 as bool
    let count = enabled as i64
    if enabled { print(count) }
}
main()
```

This runtime parity fix is included in release v0.5.1.

Raw pointers also support an explicit `as bool` inside `unsafe`: a null pointer
becomes `false`, and any non-null address becomes `true`. This only checks the
address; it does not prove that the pointer is alive, readable or safe to dereference.

```dev
fn has_address(p: *i64): bool {
    unsafe { return p as bool }
}
```

This pointer cast parity fix is included in release v0.5.1.

### Native string address casts

Converting an integer or raw pointer to native `str`, or extracting an integer
address from `str`, requires a lexical `unsafe` block. These casts do not copy
text or validate that the address contains a live, NUL-terminated string.
`text as str` with an existing string remains safe. Direct casts from `str` to a
mutable pointer remain rejected, including inside `unsafe`.

The source runtime owns its string values and does not implement string address
casts, even inside `unsafe`; use the supported FFI string interface instead.
These additional native safety checks are included in release v0.5.1.

### Callback address casts

C callback casts require a lexical `unsafe` block in both source and native
execution. A callback may be converted to another callback signature, a raw
pointer, an integer address or `bool`. Only callbacks, raw pointers and integer
addresses may be converted to a callback; floating-point, boolean and string
sources are rejected. Null callbacks convert to `false`.

```dev
fn add(n: i64): i64 { return n + 1 }
unsafe {
    let cb = callback(add)
    let address = cb as usize
    let restored = address as callback(i64): i64
    print(restored(41))
}
```

The caller must keep the callback owner alive and preserve its actual C signature.
A cast does not validate an address or make an incompatible signature callable.
These callback cast checks and runtime conversions are included in release v0.5.1.


## Object methods and this

```dev
fn main() {
    let user = {
        name: "Dev"
        greet: fn(): str {
            return "Hello ${this.name}"
        }
    }
    print(user.greet())
    user.name = "Alice"
    print(user.greet())
}
main()
```

Object literals with `fn` fields create an anonymous typed record. Methods receive
an implicit `this` containing a read-only snapshot of the object used for that
call. Methods can call other methods through `this.method()`. Parameter and
return annotations use ordinary function syntax. Both runtime and native builds
support this form in current source after v0.5.1.

`this` อ่านค่าของ object ที่เรียก method ณ เวลานั้น และเรียก method อื่นได้
แต่ยังไม่รองรับการแก้ object เดิมผ่าน `this` หรือการ capture ตัวแปรนอก method
ให้เก็บค่าที่ต้องใช้เป็น field แล้วอ่าน `this.field` แทน

Data fields have fixed inferred types; adding new fields or replacing methods
is not supported on method objects. Fixed array data fields are supported.
Detached method values are not supported yet. `json.stringify` serializes data
fields only. A `.don` data document cannot contain functions or `this`.
See [ตัวอย่างเต็ม / Full example](#/examples/object-methods).

## Computed data field access on method objects

```dev
fn main() {
    let person = {
        name: "Dev"
        age: 18
        greet: fn(): str { return "Hello ${this.name}" }
    }
    let x = "name"
    print(person[x])
    person[x] = "Alice"
    print(person.greet())
    x = "age"
    person[x] += 1
    print(person[x])
}
main()
```

ใช้ string เลือก data field ได้ทั้งอ่านและแก้ค่า แม้ object จะมี methods
Key expressions are evaluated once. Missing data fields and incompatible writes
report an error. Field types remain fixed; new fields cannot be inserted.
`const` and read-only `this` reject writes. Methods still use `person.greet()`;
computed method calls and detached methods are not supported.
Writes currently target a local object directly (`person[key] = value`);
writes through computed chains or temporary objects are not supported yet.
Available in current source after v0.5.1 in both runtime and hosted native builds.
See [ตัวอย่างเต็ม / Full example](#/examples/object-keys).

## Array tools / การใช้งาน array เพิ่มเติม

```dev
fn make(): [i64; 3] { return [1, 2, 3] }
fn main() {
    let values = make()
    let [first, second, ...rest] = values
    print(first)
    print(rest[0])
    let combined = [0, ...values, ...rest, 4]
    for item in combined { print(item) }
    let middle = values[1..3]
    print(middle[0])
    let zeros: [i32; 4] = [0; 4]
    print(zeros.len())
}
main()
```

- `let [a, b] = values` และ `const [a, b] = values`: แยกสมาชิกตาม index
- `[...values, extra]`: รวม Array, Vec และ Slice เป็น `Vec<T>` ชนิดเดียวกัน
- `let [first, ...rest] = values`: rest ต้องอยู่ท้ายสุด และได้ Vec สำเนาที่แก้ได้
- `for item in values`: วน Array, Vec, Slice หรือ DON array บน snapshot ที่ประเมินครั้งเดียว
- `values[start..end]`: Slice snapshot แบบไม่รวม end ต้องระบุทั้งสอง bounds
- `[value; count]`: fixed array ซ้ำค่า ประเมิน value ครั้งเดียว count เป็น integer literal 1..1000000
- Fixed arrays รองรับ copy, assignment, parameters, return, closures และ data fields ของ object methods ใน native

Spread creates a mutable Vec with one element type; rest bindings are Vec snapshots.
Range slicing creates a read-only Slice. Foreach snapshots the input and evaluates
it once; modifying the original collection does not change the iteration set.
Break and continue retain their normal loop behavior. Fixed arrays pass and return
by value in ordinary Dev functions and function values; native uses an internal
aggregate carrier. C FFI array aggregates remain unsupported. Local fixed arrays
remain one-dimensional; use Vec<Vec<T>> or nested DON arrays for matrices.
Destructuring holes/defaults, open-ended slices and runtime repeat counts are not
supported yet. These expression forms belong to `.dev`; `.don` remains a data format.
Available in current source after v0.5.1 in runtime and native modes.
See [ตัวอย่างเต็ม / Full example](#/examples/array-tools).

## Array search

```dev
fn main() {
    const values = [1, 2, 3, 2]
    print(values.includes(2))      // true
    print(values.indexOf(2))       // 1
    print(values.lastIndexOf(2))   // 3
    print(values.indexOf(9))       // -1
}
main()
```

`includes(value)` returns bool. `indexOf(value)` returns the first matching
index and `lastIndexOf(value)` the last, as i64; both return -1 when absent.
Indices start at zero and are relative to the receiver, including Slice.
Supported in source runtime and native for homogeneous Array, Vec and Slice
of numeric, bool or str values. String comparison uses contents. The needle
must have the same element type. Search is read-only, evaluates the receiver
and needle once in that order, and takes O(n) time. No fromIndex argument,
record equality or heterogeneous DON-array search yet. Predicate index searches are documented below.

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

## Prototype objects

`Object({...})` creates a shared mutable object. `Object.create(proto)` inherits properties, and `Object.construct(fn, ...args)` invokes a constructor with an explicit first `this: json.Value` parameter. Function prototypes can be updated at runtime. This API is supported in the source runtime and hosted native compiler. Plain DON literals and anonymous record methods keep their existing value behavior. See [Object Reference](object-reference.md) for all methods, descriptors and limitations.
