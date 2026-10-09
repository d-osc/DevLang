# Advanced DevLang contracts

## Recursive references

```dev
enum List<T> { Nil, Cons(T, Ref<List<T>>) }
let list = List<i64>.Cons(42, ref(List<i64>.Nil()))
match list { Nil => {}; Cons(n, tail) => { print(n) } }
```

Ref stores an immutable value copy; deref returns a copy. It has no null, manual free, mutable dereference or raw-pointer cast. Recursive types need Ref, pointer or managed collection edges; direct by-value cycles have infinite size and are rejected. Native managed allocations require hosted mode. Reference counting releases last-owned storage; it is not a borrow checker. A Ref containing a borrowed str/pointer does not own the pointed-to C memory.

## Generic constraints

```dev
trait HasValue { fn value(self Self) i64 }
struct Point { x i64 }
fn Point.value(self Point) i64 { return self.x }
fn read<T:HasValue>(p T) i64 { return p.value() }
struct Buffer<T, const N> { data [T;N] }
print(read(Point(42)))
let buffer = Buffer<i64,2>([19,23])
print(buffer.data[0] + buffer.data[1])
```

Inference uses arguments, payloads and expected types. Explicit arguments resolve ambiguity. Number, Integer and Equatable are built-in constraints; combine bounds with +. User traits statically require matching methods. No trait objects, associated types, default methods or blanket impls. Generic functions cannot be extern/export; write concrete wrappers. Const arguments are nonnegative integers; array lengths 1..1000000. No const expressions, defaults or type arithmetic.

## Threaded tasks

```dev
async fn compute(n i64) i64 { return n * 2 }
let job = compute(21)
print(await(job))
let chained = then(spawn(fn() i64 { return 19 }), fn(n i64) i64 { return n + 23 })
print(await(chained))
```

Spawn and async create OS threads. Await blocks and returns a copy; repeated waits work. Ready polls completion. Then evaluates arguments once and returns a task immediately; its worker thread waits for input. Task<void> uses fn() R continuations. No coroutine scheduler, pool, async I/O, cancellation, channels or mutex API. Dropping last handle detaches; detached work does not keep the process alive. Await required work/errors before exit. Native task programs need C11 atomics and Windows/pthread support; TinyCC fast mode is not promised.

## Unsafe and foreign pointers

Raw address-of/dereference/casts/indexing/arithmetic, volatile access, extern calls and callback creation need lexical unsafe. Each function/closure body has independent permissions. Fixed-array/Vec/Slice/string bounds, integer zero division/remainder and shift counts are checked in both modes. Native raw dereference checks null/alignment; arbitrary allocation extent and lifetime are not proven. Address-of interpreter locals is native-only. Runtime can access live foreign memory. Native str/pointers are borrowed. Volatile accesses are not synchronization barriers or hardware drivers.

## C signatures and callbacks

```dev
fn make(offset i64) fn(i64) i64 { return fn(n i64) i64 { return offset + n } }
unsafe {
    let cb = callback_context(make(19))
    print(cb.call(23, cb.data))
}
```

Extern declarations must match actual C ABI. Scalar/C-compatible structs, variadic calls and C scalar callbacks are supported. Managed aggregates, payload unions, top-level by-value arrays, packed/bitfield layouts and custom conventions need wrappers. Variadic declarations require a fixed parameter; small integers/bool promote to i32, f32 to f64. Format widths must match.

Callback type is callback(T1,T2) R. Callback results are non-str scalar or void. Native raw capturing callback(function) uses at most 64 unique function/environment pairs per signature/module and retains captures until exit. Runtime trampolines remain until engine shutdown.

Callback_context returns ContextCallback<fn(T) R> with call/data/owner. Userdata is LAST in the C signature. Keep the managed record alive until C unregisters and all calls finish; do not mix fields. Native context bridges avoid the 64-instance limit and release environments with owners. Runtime diagnoses expired owners but keeps trampoline code/weak control blocks until shutdown. Native expired userdata is unsafe. Adapt C APIs whose userdata position differs.

Source-only FFI auto-preparation needs sibling devc + C compiler the first time; loaded/cached shared libraries can run without that compiler. Use --ffi-lib for explicit runtime libraries; --link for native builds. Stdlib modules and archive must be supplied explicitly; they are not automatically injected by d.
