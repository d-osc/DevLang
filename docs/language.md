# Dev Lang specification

Compiler v0.4 supports generic external namespaces through `--module-dir NAME=DIR` and explicit libraries through `--link`. The runtime is a separate C library with ordinary Dev bindings. See [runtime.md](runtime.md) for contracts and [optimization.md](optimization.md) for historical v0.2 measurements.

## Layout and syntax

A program consists of `.dev` files. Imports and function declarations appear at module scope. Statements end at a newline or an optional semicolon. Braces delimit blocks. Newlines inside `()` and `[]` are whitespace. Comments begin with `#` or `//`. Names use ASCII letters, digits and `_`, and cannot start with a digit; strings contain UTF-8.

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
```

Parameter types are required; the return type defaults to `void`, except a defined `main` defaults to `i32`. A hosted entry point must be `fn main()` with no parameters and return `i32`. Reaching the end of the entry `main` returns zero. Other value-returning functions must return on every path; this check conservatively does not treat a loop as guaranteed to terminate or return.

`let` variables are mutable. Local variables can shadow an outer scope, but duplicate declarations in the same scope are errors. Variables must have an initializer. There are no global variables in v0.1. A colon between a name and its type, and `->` before a return type, are accepted as optional spelling; neither is required.

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

`break` and `continue` are valid only inside a `while`. Function calls support forward declaration and recursion. Calls resolve to a local function or `module.function`; first-class functions are not implemented.

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
```

The C ABI follows the selected backend's target, calling convention and type definitions. FFI signatures must match the actual C declaration. Struct ABI, variadic calls, callbacks and calling-convention attributes are not provided. Use C wrapper functions for unsupported APIs. CLI `--link file.c`, `.o`, `.a`, or the backend's native object/archive formats provide additional implementations. `--ldflag -lm`, for example, passes a library flag on toolchains that support it.

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
