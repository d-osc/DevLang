# Managed values and advanced features

All examples work with `d FILE` and `d build FILE` unless stated otherwise.
Run `examples/features/advanced.dev` for a complete executable example.

Interpreter-only cross-worker channels, mutex callbacks and cooperative
cancellation are available through [std/sync](sync.md). Captured sync handles
share their resource, while ordinary managed data keeps value semantics.

## Safety and lifetime

`unsafe { ... }` is required for raw-pointer address-of/dereference/casts,
pointer arithmetic/indexing, volatile access, extern calls and C callback creation.
An unsafe caller does not grant permissions to a called function's body.
Fixed arrays, Vec, slices and string indexing check bounds in both modes.
Integer division/remainder check zero; shifts check the count. Floating division
keeps IEEE behavior. Signed minimum divided by -1 wraps to the minimum; its
remainder is zero. Checks stay active in optimized native builds.

Native raw dereference and volatile access check null/alignment; runtime foreign
access also checks address overflow. Neither engine can establish the allocation
size or lifetime of an arbitrary foreign pointer. `unsafe` is an explicit contract,
not a borrow checker, memory sandbox or protection against an incorrect C ABI.
Clang/GCC builds use `-fwrapv` for signed addition/subtraction/multiplication.
Use explicit widths and avoid relying on overflow when crossing the C ABI. Address-of interpreter
locals remains native-only; runtime raw reads/writes work on live foreign memory.

Ref, collections, closure environments and task handles use reference counting.
Native keep/drop visitors cover arguments, returns, locals, nested fields,
containers and temporaries, including early return/break/continue paths.
Last-owner cleanup frees managed storage. Native `str` and raw pointers remain
borrowed C memory; wrapping them in Ref does not extend their external lifetime.
Managed allocation requires hosted mode. Raw pointers into managed/native data
must never outlive their owners. Extern/export managed values are rejected.

## Collections

```dev
let values = Vec<i64>(19, 23)
let snapshot = values.slice(0, 2)
values[0] = 99
print(snapshot[0] + snapshot[1]) // 42
values.push(7)
print(values.pop())
values.clear()

let scores = Map<str, i64>()
scores.set("answer", 42)
print(scores.get("answer"))
print(scores.contains("answer"))
print(scores.remove("answer"))
```

Vec supports indexed reads/writes, `push`, `pop`, `len`, `clear` and
`slice(start, end)` with exclusive end. Pop on an empty Vec is an error.
Slice is a read-only shared snapshot with indexing, `len` and further slicing.
Vec/Map copies share storage until mutation; mutations copy shared storage.
Recursive layouts can use `Vec<Node>` and `Map<i64, Node>` as well as Ref.
Writable collection methods require a local/field/array/Vec-element target;
bind a returned temporary to a variable before mutating it.

Map supports `set`, `get`, `contains`, `remove`, `len`, `clear`.
Missing-key get is an error, remove reports whether an entry existed.
Keys are numeric/bool/str or payload-free enums; values are non-array values.
Map uses open addressing, a load factor of at most 1/2 and backward-shift
deletion in both engines. Lookup/update/remove are expected O(1); collisions
can make them O(n). Insertion is amortized O(1), with O(n) growth rehashes.
The hash is deterministic, not resistant to adversarial keys. Entry order is
unspecified, including diagnostic display. Floating +0 and -0 are the same key;
NaN keys never compare equal and cannot be retrieved by another NaN. Vec push is
amortized O(1) when storage is unique. A COW mutation costs O(n) when shared.
Native string keys/values borrow their C storage and must remain alive; key bytes
must also remain unchanged while registered in a Map. Runtime
strings own their bytes. Native and runtime allocation failures differ: native
managed allocation aborts, runtime returns a diagnostic where allocation permits.

## Functions, methods and closures

```dev
struct Point { x i64; y i64 }
fn Point.sum(self Point) i64 { return self.x + self.y }
print(Point(19,23).sum())

fn make(offset i64) fn(i64) i64 {
    return fn(value i64) i64 { return offset + value }
}
let add = make(19)
print(add(23))
```

Function values have type `fn(T1, T2) R`; an empty parameter list is `fn() R`.
Concrete Dev functions can be assigned, passed, returned and stored in collections.
Generic functions need a concrete wrapper before becoming a value.
Closures capture used outer variables by value at creation. Captured managed
values retain their storage. Mutating a captured local changes the invocation's
copy; it does not update an outer variable or persist into later invocations.
Fixed arrays must be captured through a Vec or another supported value.
Closures do not inherit the enclosing unsafe permission.

Methods use `fn Type.method(self Type, ...) R`; the receiver is an ordinary
value parameter. A receiver copy can be changed and returned. Use an explicit
unsafe raw-pointer parameter/function for foreign mutable-memory operations.
There is no implicit mutable-self borrowing, overload dispatch or inheritance.
Imported receiver methods require the defining module to be imported.

## Generic inference, constraints and const arguments

```dev
fn identity<T>(value T) T { return value }
print(identity(42))
let typed i32 = identity(42)
let values Vec<i64> = Vec()

trait HasValue { fn value(self Self) i64 }
struct Point { x i64 }
fn Point.value(self Point) i64 { return self.x }
fn read<T:HasValue>(p T) i64 { return p.value() }
print(read(Point(42)))

fn sum<T:Number>(a T, b T) T { return a + b }
struct Buffer<T, const N> { data [T;N] }
let b = Buffer<i64,2>([19,23])
```

Arguments are inferred from values, constructor payloads and expected types.
Ambiguous/conflicting cases require explicit arguments, for example
`Result<i64>.Err("bad")`. Explicit instantiation remains supported.
`Number`, `Integer`, `Equatable` are built-in constraints; multiple bounds use `+`.
User traits statically require matching methods on the concrete nominal type.
There are no trait objects, associated types, default methods or blanket impls.
An unused generic body's errors can remain deferred until specialization.

Const generics are nonnegative integer arguments. Array lengths must be
1..1000000. A const parameter can be used as an array length and numeric value.
Const expressions, default arguments and type-level arithmetic are not supported.
Fixed arrays still cannot be passed/returned directly as function values;
wrap them in a struct or use Vec/raw pointers.

## Matching

```dev
enum Inner { Empty, Value(i64) }
enum Outer { Some(Inner), None }
match Outer.Some(Inner.Value(42)) {
    Some(Value(n)) if n > 0 => { print(n) }
    _ => { print(0) }
}
```

Nested enum patterns, guards and a final unguarded `_` arm are supported.
Coverage requires unguarded irrefutable arms for every variant or a wildcard.
The checker does not combine multiple refutable nested patterns into complete
coverage; add a fallback. Matching remains a statement and does not yet provide
literal/struct/slice patterns or expression-valued arms.

## Threads and async

```dev
async fn answer(n i64) i64 { return n + 1 }
let first = answer(41)
let second = spawn(fn() i64 { return 19 + 23 })
print(await(first))
print(ready(first)) // true
print(await(second))
```

`spawn(fn() T)` returns `Task<T>`. `await(task)` joins and returns a value copy;
repeated waits are supported. `async fn` starts its body on an OS thread and
returns Task. `ready(task)` checks completion without waiting; after it returns
true, await retrieves the result. Dropping the last handle detaches unfinished
work instead of waiting. The worker retains captured values and native task
storage until completion. Detached work does not keep the process alive; await
required work before exiting, and await runtime tasks to observe their errors.
Runtime workers use independent interpreter/numeric-plan frames and native
bindings. Native shared reference counts are atomic throughout a threaded program.
Captures remain snapshots; data races in unsafe foreign memory are the caller's
responsibility. Await runtime tasks to observe worker errors.

This is **thread-backed async**, not an event loop or suspendable coroutine system.
Await blocks the waiting thread. There is no scheduler pool, cancellation, channel,
async I/O, user mutex API or automatic parallel-loop transformation yet. Creating
many tiny tasks has thread/interpreter setup overhead. Native tasks require a C11
compiler with atomics and Windows/pthread support; TinyCC fast builds are not
promised for threaded code. Foreign callback execution currently uses an independent
AST interpreter frame, so high-frequency callbacks have additional overhead.

### Task continuations

`then(task, next)` returns `Task<R>` immediately. For a `Task<T>`, next has type
`fn(T) R`; for `Task<void>`, it has type `fn() R`. The arguments are evaluated
once, at registration. Captures, the input task and managed results remain alive
until the worker completes. A task error prevents the continuation from running
in runtime and is reported by awaiting the returned task. Continuations can be
chained, and a void result is supported.

```dev
let result = then(spawn(fn() i64 { return 19 }),
                  fn(n i64) Ref<i64> { return ref(n + 23) })
print(deref(await(result))) // 42
```

The caller of `then` does not wait for its input. Each continuation still creates
an OS thread which waits for that input: this is not a coroutine scheduler, thread
pool or nonblocking `await`. Use coarse tasks rather than thousands of tiny stages.
Await required work before process exit; dropping handles detaches the workers.

## C ABI

```dev
fn increment(x i64) i64 { return x + 1 }
extern fn invoke(cb callback(i64) i64, x i64) i64
extern fn printf(format str, ...) i32
unsafe {
    print(invoke(callback(increment), 41))
    printf("value=%d\n", 42 as i32)
}
```

Callback signatures use `callback(T1, T2) R`; `callback(function_value)`
creates the C pointer. Function values and closures with captured values are supported for C scalar
signatures, with a non-str scalar or void result. Captures retain managed storage.
Runtime callback pointers remain valid for the engine lifetime; creation from the
same function/environment reuses its libffi trampoline. Native plain functions
use ordinary C pointers. Other native function values use portable static C
trampolines: up to 64 unique function/environment pairs per signature per originating module, with a
diagnostic on exhaustion. Their environments remain retained until process exit;
creating a pointer from the same retained function/environment reuses its slot.
Slots are never assigned to a different environment. Native callback programs require C11 atomics; capturing callbacks require hosted mode.
Use a C userdata adapter for unbounded callback registration or early reclamation. C must not
invoke a callback after engine shutdown. Runtime callback errors are caught at the C boundary. Synchronous callback
errors are reported when the native call on the same thread returns; callbacks
invoked asynchronously by a foreign thread need a caller-provided error channel.

### Callbacks with userdata and managed lifetime

For a C API accepting a separate userdata pointer, `callback_context(function)`
returns `ContextCallback<fn(T1, T2) R>`, a managed record with `.call`, `.data` and
`.owner`. Its C signature is `callback(T1, T2, *void) R`: **userdata is last**.
Pass `.call` and `.data` separately to C; keep the record (or a copy) alive until
C unregisters the callback and all invocations finish. The managed owner is a
`Ref` to the function value. Copies and collections retain it; last-owner cleanup
releases the captured environment. Do not replace or mix the pair's fields.

```dev
extern fn invoke(cb callback(i64,*void) i64, value i64, data *void) i64
let offset = 19
unsafe {
    let cb = callback_context(fn(n i64) i64 { return offset + n })
    print(invoke(cb.call, 23, cb.data))
}
```

Native emits one ordinary C bridge per signature per module; userdata selects the
owned function. This path has no 64-instance trampoline limit and needs no libffi
in the native program. Environments are reclaimed when their owners are released.
The original `callback(fn)` API retains its bounded trampoline behavior for APIs
without userdata. For C APIs with userdata in another position, use a C adapter.

Runtime uses libffi trampolines with weak owners. An invocation after last-owner
cleanup reports `context callback owner has expired`; wrong userdata is also an
error. Runtime retains trampoline code and weak control blocks until engine
shutdown, while releasing captured managed values earlier. Native raw userdata
still follows the unsafe C lifetime contract and must not be used after cleanup.
The managed record itself cannot be passed by value through C ABI.

Run `examples/features/continuations.dev` for a source-only executable example.

Variadic declarations require extern and at least one fixed parameter. Extra
arguments must be C scalars: bool/i8/u8/i16/u16 promote to i32 and f32 to f64.
Other widths remain unchanged; a format string must match those widths.
Native struct ABI supports C-compatible fields, padding and scalar/array nesting;
payload-free enums use int32. Payload unions, managed aggregates, top-level arrays,
packed structs, bitfields and custom calling conventions require a C wrapper.
Runtime aggregate marshaling is limited to 1 MiB, alignment 8 and array fields
up to 1024 elements. Wrong declarations or expired pointers can crash either mode.

Use `scripts/smoke_features.py`, `scripts/smoke_safety.py` and
`scripts/smoke_reference_native.py --sanitize` for differential/lifetime validation.
