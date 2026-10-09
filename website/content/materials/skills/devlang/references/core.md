# Core syntax and contracts

## JSON (current source runtime)

`use "std/json"` supplies managed `json.Value` values. This module is runtime-only;
build the latest repository runtime, because v0.4.0 installers do not include it.
Use parse/valid, get/has, at/len/keys, kind/is_null, string/bool/int/uint/float,
object/array/null_value, set/remove/push, value/stringify/pretty. Updates return a
new value: `data = json.set(data, "age", 21)`. Copies keep their previous content.
Missing keys, bad indices/types and malformed JSON stop execution; no try/catch.
Stringify accepts scalars, finite floats, structs, Ref, array/Vec/Slice and Map
with string keys. JSON-to-struct conversion is explicit. Parsed number text can
retain arbitrary precision; scalar conversion enforces i64/u64/f64 ranges.
See repository `docs/json.md` and `examples/json/main.dev` for full contracts.

## Script and modules

```dev
fn add(a i64, b i64) i64 { return a + b }
fn main() { print(add(19, 23)) }
main()
```

Ordinary functions default to void; main defaults to i32 and falls through with zero. File-level `return main()` propagates exit status. File-level variables cannot be captured by declared functions; pass them as arguments. Imported files must contain declarations only.

`use math` resolves math.dev beside the importing file. `use "lib/math.dev" as math` uses an explicit path. Call `math.add(...)`. Direct importers see functions without export. Imports are not transitively re-exported. `export fn` preserves a globally unique C symbol. `--module-dir NAME=DIR` maps external namespaces explicitly.

## Scalars, loops and arrays

Types: i8/i16/i32/i64, u8/u16/u32/u64, isize/usize, f32/f64, bool, str, *T and void. Integer literals default to i64; floats to f64. Typed contexts constrain literals. Numeric mixing needs explicit `as`. Conditions require bool; and/or short-circuit. Both # and // introduce comments. Strings use UTF-8; indexing returns a byte, not a Unicode character. String equality is unsupported; native strings are borrowed NUL-terminated C memory.

```dev
let values [i64; 3] = [19, 23, 7]
let total = 0
for i in 0..3 { total += values[i] }
print(total)
```

Range bounds are i64, evaluated once, ascending and end-exclusive. Changing the iteration variable does not change the internal counter. Break/continue work with while/for. Native fixed arrays cannot be copied, passed or returned directly; wrap them in structs or use Vec. Local nested arrays are unsupported.

## Value types

```dev
struct Point { x i64; y i64 }
fn Point.sum(self Point) i64 { return self.x + self.y }
let p = Point(19, 23)
print(p.sum())

enum Result<T> { Ok(T), Err(str) }
match Result<i64>.Ok(42) {
    Ok(n) => { print(n) }
    Err(message) => { print(message) }
}
```

Struct constructors are positional, complete and ordered. Methods receive a value copy. Nominal identity includes module and generic arguments. Structs can hold fixed arrays; native whole-struct printing is unsupported. Payload-free enums support equality and casts to integer. Payload enums do not; use match. Match is a statement. Guards and nested enum patterns are supported, but coverage needs unguarded irrefutable variant arms or a final wildcard. Literal, struct and slice patterns are unsupported.

## Managed collections

```dev
let values = Vec<i64>(19, 23)
let snapshot = values.slice(0, 2)
values[0] = 99
print(snapshot[0] + snapshot[1])
let scores = Map<str, i64>()
scores.set("answer", 42)
if scores.contains("answer") { print(scores.get("answer")) }
```

Vec: index read/write, push(value), pop(), len(), clear(), slice(start,end). Slice: read-only index, len(), slice(start,end); end is exclusive. Map: set(key,value), get(key), contains(key), remove(key), len(), clear(). Get missing keys and pop empty vectors are errors. There is no iterator API. Writable methods need a local/field/array/Vec-element target, not an unbound temporary.

Vec/Map copies share COW storage; mutations preserve earlier copies/slices. Unique Vec push and Map insertion are amortized O(1); shared mutations can copy O(n). Map keys are numeric/bool/str or payload-free enums. Hashing is deterministic, not collision-attack resistant. Native string keys must remain live and byte-stable.

## Functions as values

```dev
fn make(offset i64) fn(i64) i64 {
    return fn(value i64) i64 { return offset + value }
}
print(make(19)(23))
```

Function value type is fn(T1,T2) R. Captures are snapshots; modifying a capture does not update the outer value or persist across invocations. Managed captures retain storage. Wrap arrays in a supported value. Generic functions need concrete wrappers before becoming values.
