# Dev Lang specification

Compiler v0.4 supports generic external namespaces through `--module-dir NAME=DIR` and explicit libraries through `--link`. The source runtime interprets Dev directly. The separate native C library has ordinary Dev bindings. See [runtime.md](runtime.md) and [stdlib.md](stdlib.md) for contracts and [optimization.md](optimization.md) for historical v0.2 measurements.

## Layout and syntax

A program consists of `.dev` files. Imports, function declarations and executable statements appear at entry-file scope. Imported files contain declarations only. Statements end at a newline or an optional semicolon. Braces delimit blocks. Newlines inside `()` and `[]` are whitespace. Comments begin with `#` or `//`. Names use ASCII letters, digits and `_`, and cannot start with a digit; strings contain UTF-8.

```dev
use math
use "../common/device.dev" as device

fn add(a i32, b i32) i32 {
    return a + b
}

fn main() {
    let value = 42
    let register u32 = 0x4000_0000
    value += 1
    print(value)
}

main()
```

Parameter types are required; the return type defaults to `void`, except a defined `main` defaults to `i32`. Entry-file statements execute in order; defining `main` never invokes it. Write `main()` explicitly, or use top-level statements directly. Reaching the end of an i32 `main` returns zero. A file-level `return main()` forwards its result as the exit status. Other value-returning functions must return on every path; this check conservatively does not treat a loop as guaranteed to terminate or return.

`let` variables are mutable. Local variables can shadow an outer scope, but duplicate declarations in the same scope are errors. Variables must have an initializer. File-level variables belong to the script scope and cannot be captured by functions; pass them as parameters. A colon between a name and its type, and `->` before a return type, are accepted as optional spelling; neither is required.

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

```dev
let x i32 = 42
let wide = x as i64
let real = wide as f64
```

Arrays infer their element type from the first element when no type is specified. Every element must match. Arrays are initialized by literals and cannot be copied, returned by value, passed by value, nested, or addressed as a whole. Pass `&array[0]` and an explicit length to functions instead.

```dev
fn first(data *u8) u8 {
    return data[0]
}

fn main() {
    let bytes [u8; 3] = [10, 20, 30]
    bytes[1] = 42
    print(first(&bytes[0]))
}

main()
```

`str[index]` reads a UTF-8 **byte** as `u8`, not a Unicode character. String bytes cannot be assigned. String equality is deliberately undefined; call C `strcmp` for content comparison. Strings and `null` follow C lifetime and NUL-termination rules; embedded `\0` truncates ordinary C string output.

## Expressions and control flow

Operators, from low to high precedence:

| Level | Operators |
| --- | --- |
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
extern fn malloc(bytes usize) *void
extern fn free(memory *void)

export fn add(a i32, b i32) i32 {
    return a + b
}

fn main() {
    let data = malloc(64) as *u8
    if data == null { return 1 }
    data[0] = 42
    print(data[0])
    free(data as *void)
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

Clang/GCC backend flags include `-fwrapv` for signed addition/subtraction/multiplication wrapping, including `-O3` release builds. The optional `--fast` TinyCC development backend emits unoptimized machine arithmetic; wrapping, casts, pointers and volatile access are covered by native regression scenarios. Small integer operation results are cast back to the declared width; narrowing casts otherwise follow C behavior. Invalid shifts, division by zero, signed minimum divided by `-1`, invalid pointer arithmetic, misalignment, out-of-bounds access and use-after-free are not checked at runtime. Obvious nonnegative integer-literal array bounds are checked during compilation.

Evaluation order for calls and binary operands follows C. An expression with several state-changing calls or writes must be split into statements if their order matters. `and/or` retain C's short-circuit behavior. Volatile accesses are observable C accesses, not atomic synchronization, CPU fences, DMA cache maintenance or a substitute for platform drivers.

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
    x i64
    y i64
}

let p = Point(19, 23)
p.x += 1
print(p.x + p.y)

enum Color { Red, Green, Blue }
let color Color = Color.Green
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
Methods, inheritance and named-field constructors are not yet supported.
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
struct Node { value i64; next *Node }
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
fn identity<T>(value T) T {
    return value
}
print(identity<i64>(42))
print(identity<str>("Dev"))

struct Box<T> { value T }
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
